use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::process::{Child, Command};
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;

use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use tauri::{Emitter, Manager};
use tokio::sync::{mpsc, oneshot, Mutex};
use tokio_tungstenite::tungstenite::Message;

use crate::chrome_app::find_chrome;

// ── CDP WebSocket 类型 ──────────────────────────────────────────

type WsSink = futures_util::stream::SplitSink<
    tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
    Message,
>;

// ── CdpClient（从 ks_scroll_scraper demo 移植） ────────────────

pub struct CdpClient {
    sink: Arc<Mutex<WsSink>>,
    pending: Arc<Mutex<HashMap<i64, oneshot::Sender<Value>>>>,
    next_id: AtomicI64,
}

impl CdpClient {
    pub async fn connect(port: u16) -> Result<(Self, mpsc::UnboundedReceiver<Value>), String> {
        let url = format!("http://127.0.0.1:{}/json", port);
        let tabs: Vec<Value> = reqwest::Client::new()
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("获取 CDP 标签页列表失败: {}", e))?
            .json()
            .await
            .map_err(|e| format!("解析 CDP 标签页列表失败: {}", e))?;

        let ws_url = tabs
            .iter()
            .find(|t| t["type"].as_str() == Some("page"))
            .and_then(|t| t["webSocketDebuggerUrl"].as_str())
            .ok_or("未找到可用的 Chrome 标签页")?
            .to_string();

        let (ws, _) = tokio_tungstenite::connect_async(&ws_url)
            .await
            .map_err(|e| format!("WebSocket 连接失败: {}", e))?;

        let (sink, stream) = ws.split();
        let sink = Arc::new(Mutex::new(sink));
        let pending: Arc<Mutex<HashMap<i64, oneshot::Sender<Value>>>> =
            Arc::new(Mutex::new(HashMap::new()));

        let (event_tx, event_rx) = mpsc::unbounded_channel::<Value>();

        let pending_clone = pending.clone();
        tokio::spawn(async move {
            let mut stream = stream;
            while let Some(Ok(msg)) = stream.next().await {
                if let Message::Text(text) = msg {
                    if let Ok(json) = serde_json::from_str::<Value>(&text) {
                        if let Some(id) = json.get("id").and_then(|v| v.as_i64()) {
                            let mut map = pending_clone.lock().await;
                            if let Some(tx) = map.remove(&id) {
                                let _ = tx.send(json);
                            }
                        } else if json.get("method").is_some() {
                            let _ = event_tx.send(json);
                        }
                    }
                }
            }
        });

        Ok((
            CdpClient {
                sink,
                pending,
                next_id: AtomicI64::new(1),
            },
            event_rx,
        ))
    }

    pub async fn send(&self, method: &str, params: Value) -> Result<Value, String> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let msg = serde_json::json!({ "id": id, "method": method, "params": params });
        let (tx, rx) = oneshot::channel();
        self.pending.lock().await.insert(id, tx);
        self.sink
            .lock()
            .await
            .send(Message::Text(msg.to_string()))
            .await
            .map_err(|e| format!("发送 CDP 命令失败: {}", e))?;
        let resp = tokio::time::timeout(std::time::Duration::from_secs(30), rx)
            .await
            .map_err(|_| format!("CDP 命令超时: {}", method))?
            .map_err(|_| format!("CDP 响应通道关闭: {}", method))?;
        if let Some(err) = resp.get("error") {
            return Err(format!("CDP 错误: {}", err));
        }
        Ok(resp)
    }

    pub async fn send_no_params(&self, method: &str) -> Result<Value, String> {
        self.send(method, serde_json::json!({})).await
    }

    pub async fn navigate_and_wait(&self, url: &str, max_secs: u64) -> Result<(), String> {
        self.send("Page.navigate", serde_json::json!({ "url": url }))
            .await?;
        let start = tokio::time::Instant::now();
        let deadline = start + std::time::Duration::from_secs(max_secs);
        let min_wait = std::time::Duration::from_millis(1500);
        loop {
            if tokio::time::Instant::now() >= deadline {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(400)).await;
            if let Ok(r) = self
                .send(
                    "Runtime.evaluate",
                    serde_json::json!({"expression": "document.readyState"}),
                )
                .await
            {
                let state = r
                    .pointer("/result/result/value")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                if state == "complete" && start.elapsed() >= min_wait {
                    break;
                }
            }
        }
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        Ok(())
    }

    pub async fn eval(&self, expression: &str) -> Result<String, String> {
        let r = self
            .send(
                "Runtime.evaluate",
                serde_json::json!({"expression": expression, "awaitPromise": true}),
            )
            .await?;
        let val = r.pointer("/result/result/value");
        let s = match val {
            Some(Value::String(s)) => s.clone(),
            Some(Value::Bool(b)) => b.to_string(),
            Some(Value::Number(n)) => n.to_string(),
            Some(v) => v.to_string(),
            None => String::new(),
        };
        Ok(s)
    }
}

// ── ChromeSession（全局单例） ───────────────────────────────────

pub struct ChromeSession {
    pub child: Option<Child>,
    pub port: u16,
    pub cdp: Option<Arc<CdpClient>>,
    pub event_rx: Option<Arc<Mutex<mpsc::UnboundedReceiver<Value>>>>,
    #[cfg(target_os = "windows")]
    pub hwnd: Option<isize>,
    pub user_data_dir: Option<PathBuf>,
}

pub struct ChromeSessionState(pub Mutex<ChromeSession>);

impl ChromeSession {
    pub fn new() -> Self {
        ChromeSession {
            child: None,
            port: CDP_PORT,
            cdp: None,
            event_rx: None,
            #[cfg(target_os = "windows")]
            hwnd: None,
            user_data_dir: None,
        }
    }

    pub fn is_alive(&mut self) -> bool {
        if let Some(ref mut child) = self.child {
            match child.try_wait() {
                Ok(Some(_)) => {
                    self.child = None;
                    self.cdp = None;
                    self.event_rx = None;
                    false
                }
                Ok(None) => true,
                Err(_) => {
                    self.child = None;
                    self.cdp = None;
                    self.event_rx = None;
                    false
                }
            }
        } else {
            false
        }
    }

    pub fn kill(&mut self) {
        if let Some(ref mut child) = self.child {
            kill_process_tree(child.id());
            let _ = child.kill();
            let _ = child.wait();
        }
        self.child = None;
        self.cdp = None;
        self.event_rx = None;
        #[cfg(target_os = "windows")]
        {
            self.hwnd = None;
        }
    }
}

fn kill_process_tree(pid: u32) {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        let _ = Command::new("taskkill")
            .args(&["/PID", &pid.to_string(), "/T", "/F"])
            .creation_flags(0x08000000)
            .output();
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = pid;
    }
}

// ── Chrome 启动 ────────────────────────────────────────────────

const CDP_PORT: u16 = 9224;


pub async fn launch_and_connect(
    app: &tauri::AppHandle,
    session: &mut ChromeSession,
    show_window: bool,
) -> Result<(), String> {
    let chrome_path = find_chrome().ok_or(
        "未检测到 Google Chrome 浏览器，请先安装 Chrome。\n下载地址：https://www.google.cn/chrome/",
    )?;

    if session.is_alive() {
        session.kill();
    }

    // 先杀掉 chrome_app.rs 管理的旧 Chrome 实例，避免端口/实例冲突
    {
        let old_state = app.state::<crate::chrome_app::ChromeProcess>();
        if let Ok(mut guard) = old_state.0.lock() {
            if let Some(ref mut child) = *guard {
                match child.try_wait() {
                    Ok(Some(_)) => {}
                    Ok(None) => {
                        kill_process_tree(child.id());
                        let _ = child.kill();
                        let _ = child.wait();
                    }
                    Err(_) => {}
                }
            }
            *guard = None;
        };
    }

    tokio::time::sleep(std::time::Duration::from_millis(300)).await;

    let port = CDP_PORT;
    let app_data = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("获取数据目录失败: {}", e))?;
    let user_data = app_data.join("chrome_cdp_data");
    std::fs::create_dir_all(&user_data)
        .map_err(|e| format!("创建 Chrome 数据目录失败: {}", e))?;

    // 清理 Chrome 会话恢复文件，防止 Chrome 恢复旧标签页而忽略 --app
    let default_dir = user_data.join("Default");
    if default_dir.exists() {
        let _ = std::fs::remove_file(default_dir.join("Last Session"));
        let _ = std::fs::remove_file(default_dir.join("Last Tabs"));
        let _ = std::fs::remove_file(default_dir.join("Current Session"));
        let _ = std::fs::remove_file(default_dir.join("Current Tabs"));
    }

    let (chrome_w, chrome_h) = crate::chrome_app::calc_chrome_window_size(app);
    let main_hwnd = crate::chrome_app::get_main_hwnd(app);

    let mut cmd = Command::new(&chrome_path);
    cmd.arg("--app=data:text/html,<html></html>")
        .arg(format!("--remote-debugging-port={}", port))
        .arg(format!(
            "--user-data-dir={}",
            user_data.to_string_lossy()
        ))
        .arg(format!("--window-size={},{}", chrome_w, chrome_h))
        .arg("--no-first-run")
        .arg("--no-default-browser-check")
        .arg("--disable-extensions")
        .arg("--disable-popup-blocking")
        .arg("--disable-infobars")
        .arg("--disable-session-crashed-bubble")
        .arg("--restore-last-session=false")
        .arg("--no-restore-state");

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW for the launcher process
    }

    let child = cmd
        .spawn()
        .map_err(|e| format!("启动 Chrome 失败: {}", e))?;

    let chrome_pid = child.id();
    eprintln!("[CDP] Chrome 已启动, PID={}, port={}", chrome_pid, port);

    session.child = Some(child);
    session.port = port;
    session.user_data_dir = Some(user_data);

    // Connect CDP with retries
    let mut cdp_result = None;
    for attempt in 1..=10 {
        let wait = if attempt == 1 { 1500 } else { 600 };
        tokio::time::sleep(std::time::Duration::from_millis(wait)).await;
        match CdpClient::connect(port).await {
            Ok(pair) => {
                cdp_result = Some(pair);
                eprintln!("[CDP] 连接成功 (第{}次尝试)", attempt);
                break;
            }
            Err(e) if attempt < 10 => {
                eprintln!("[CDP] 连接尝试{}/10 失败: {}", attempt, e);
            }
            Err(e) => {
                session.kill();
                return Err(format!("CDP 连接失败: {}", e));
            }
        }
    }

    let (cdp, event_rx) = cdp_result.ok_or("CDP 连接超时")?;
    let _ = cdp.send_no_params("Network.enable").await;
    let _ = cdp.send_no_params("Page.enable").await;

    let cdp = Arc::new(cdp);
    session.cdp = Some(cdp.clone());
    session.event_rx = Some(Arc::new(Mutex::new(event_rx)));

    // 居中 + 窗口管理
    crate::chrome_app::center_chrome_async(chrome_pid, main_hwnd).await;

    #[cfg(target_os = "windows")]
    {
        if let Some(hwnd) = find_window_by_pid(chrome_pid) {
            session.hwnd = Some(hwnd as isize);
            if !show_window && !is_dev_mode() {
                hide_chrome_window(hwnd as isize);
            }
        }
    }

    Ok(())
}

fn is_dev_mode() -> bool {
    if let Ok(exe) = std::env::current_exe() {
        let path_str = exe.to_string_lossy().to_lowercase();
        path_str.contains("target\\debug") || path_str.contains("target/debug")
    } else {
        false
    }
}

// ── Windows 窗口管理 ───────────────────────────────────────────

#[cfg(target_os = "windows")]
fn find_window_by_pid(pid: u32) -> Option<windows_sys::Win32::Foundation::HWND> {
    use windows_sys::Win32::Foundation::{BOOL, HWND, LPARAM};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindowThreadProcessId, IsWindowVisible,
    };

    struct FindData {
        target_pid: u32,
        found: HWND,
    }

    unsafe extern "system" fn callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let data = &mut *(lparam as *mut FindData);
        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, &mut pid);
        if pid == data.target_pid && IsWindowVisible(hwnd) != 0 {
            data.found = hwnd;
            return 0;
        }
        1
    }

    let mut data = FindData {
        target_pid: pid,
        found: std::ptr::null_mut(),
    };
    unsafe {
        EnumWindows(Some(callback), &mut data as *mut _ as LPARAM);
    }
    if !data.found.is_null() {
        Some(data.found)
    } else {
        None
    }
}

#[cfg(target_os = "windows")]
pub fn hide_chrome_window(hwnd: isize) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_HIDE};
    unsafe {
        ShowWindow(hwnd as windows_sys::Win32::Foundation::HWND, SW_HIDE);
    }
    eprintln!("[CDP] Chrome 窗口已隐藏");
}

#[cfg(target_os = "windows")]
pub fn show_chrome_window(hwnd: isize) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_SHOW, SetForegroundWindow};
    unsafe {
        ShowWindow(hwnd as windows_sys::Win32::Foundation::HWND, SW_SHOW);
        SetForegroundWindow(hwnd as windows_sys::Win32::Foundation::HWND);
    }
    eprintln!("[CDP] Chrome 窗口已显示");
}

// ── 网络拦截核心 ───────────────────────────────────────────────

pub struct InterceptResult {
    pub url: String,
    pub body: String,
}

/// Navigate to a URL and intercept network responses matching any of the given URL patterns.
/// Returns all matched responses collected within `timeout_secs`.
pub async fn navigate_and_intercept(
    cdp: &Arc<CdpClient>,
    event_rx: &Arc<Mutex<mpsc::UnboundedReceiver<Value>>>,
    url: &str,
    url_patterns: &[&str],
    timeout_secs: u64,
) -> Result<Vec<InterceptResult>, String> {
    let pending_requests: Arc<Mutex<HashMap<String, String>>> =
        Arc::new(Mutex::new(HashMap::new()));
    let results: Arc<Mutex<Vec<InterceptResult>>> = Arc::new(Mutex::new(Vec::new()));

    cdp.navigate_and_wait(url, timeout_secs.min(15)).await?;

    collect_intercepted(cdp, event_rx, url_patterns, &pending_requests, &results, timeout_secs).await?;

    let out = results.lock().await.drain(..).collect();
    Ok(out)
}

/// Drain event_rx, matching responseReceived → loadingFinished → getResponseBody.
/// Once the first result is collected, sets a short grace period (2s) to collect more,
/// then returns immediately instead of waiting the full timeout.
async fn collect_intercepted(
    cdp: &Arc<CdpClient>,
    event_rx: &Arc<Mutex<mpsc::UnboundedReceiver<Value>>>,
    url_patterns: &[&str],
    pending_requests: &Arc<Mutex<HashMap<String, String>>>,
    results: &Arc<Mutex<Vec<InterceptResult>>>,
    timeout_secs: u64,
) -> Result<(), String> {
    let deadline =
        tokio::time::Instant::now() + std::time::Duration::from_secs(timeout_secs);
    let mut first_result_at: Option<tokio::time::Instant> = None;
    let grace_period = std::time::Duration::from_secs(2);

    loop {
        // 如果已经有结果了，用 grace_period 作为剩余等待时间
        let effective_deadline = match first_result_at {
            Some(t) => (t + grace_period).min(deadline),
            None => deadline,
        };
        let remaining = effective_deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            break;
        }

        let event = {
            let mut rx = event_rx.lock().await;
            tokio::time::timeout(remaining, rx.recv()).await
        };

        match event {
            Ok(Some(ev)) => {
                let method = ev.get("method").and_then(|m| m.as_str()).unwrap_or("");
                let params = ev.get("params").cloned().unwrap_or(serde_json::json!({}));

                match method {
                    "Network.responseReceived" => {
                        let resp_url = params
                            .pointer("/response/url")
                            .and_then(|u| u.as_str())
                            .unwrap_or("");
                        let request_id = params
                            .get("requestId")
                            .and_then(|r| r.as_str())
                            .unwrap_or("");
                        if url_patterns.iter().any(|p| resp_url.contains(p)) {
                            pending_requests
                                .lock()
                                .await
                                .insert(request_id.to_string(), resp_url.to_string());
                        }
                    }
                    "Network.loadingFinished" => {
                        let request_id = params
                            .get("requestId")
                            .and_then(|r| r.as_str())
                            .unwrap_or("")
                            .to_string();
                        let matched_url = {
                            pending_requests.lock().await.remove(&request_id)
                        };
                        if let Some(url) = matched_url {
                            if let Ok(resp) = cdp
                                .send(
                                    "Network.getResponseBody",
                                    serde_json::json!({"requestId": request_id}),
                                )
                                .await
                            {
                                let body = resp
                                    .pointer("/result/body")
                                    .and_then(|b| b.as_str())
                                    .unwrap_or("")
                                    .to_string();
                                if !body.is_empty() {
                                    results.lock().await.push(InterceptResult { url, body });
                                    if first_result_at.is_none() {
                                        first_result_at = Some(tokio::time::Instant::now());
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            Ok(None) => break,
            Err(_) => break, // timeout
        }
    }

    Ok(())
}

fn random_delay_ms(min: u64, max: u64) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    std::time::SystemTime::now().hash(&mut h);
    std::thread::current().id().hash(&mut h);
    min + (h.finish() % (max - min + 1))
}

// ── Cookie 注入 ────────────────────────────────────────────────

pub async fn inject_cookies(
    cdp: &Arc<CdpClient>,
    cookies_str: &str,
    domain: &str,
) -> Result<(), String> {
    if cookies_str.is_empty() {
        return Ok(());
    }
    let mut cdp_cookies = Vec::new();
    for part in cookies_str.split(';') {
        let part = part.trim();
        if let Some(idx) = part.find('=') {
            let name = part[..idx].trim();
            let value = part[idx + 1..].trim();
            if !name.is_empty() {
                cdp_cookies.push(serde_json::json!({
                    "name": name, "value": value,
                    "domain": domain, "path": "/",
                }));
            }
        }
    }
    if !cdp_cookies.is_empty() {
        cdp.send(
            "Network.setCookies",
            serde_json::json!({ "cookies": cdp_cookies }),
        )
        .await?;
        eprintln!("[CDP] 注入 {} 个 Cookie (domain={})", cdp_cookies.len(), domain);
    }
    Ok(())
}

/// 平台对应的 cookie 域名和用于验证登录态的关键 cookie 名
fn platform_cookie_info(platform: &str) -> Option<(&'static str, &'static str)> {
    match platform {
        "douyin" => Some((".douyin.com", "sessionid")),
        "kuaishou" => Some((".kuaishou.com", "userId")),
        "bilibili" => Some((".bilibili.com", "DedeUserID")),
        _ => None,
    }
}

/// 通过 CDP Network.getCookies 检查 Chrome 中是否已有平台的关键 auth cookie
async fn has_auth_cookie(
    cdp: &Arc<CdpClient>,
    platform: &str,
) -> Result<bool, String> {
    let (domain, key_cookie) = match platform_cookie_info(platform) {
        Some(info) => info,
        None => return Ok(false),
    };

    let url = match platform {
        "douyin" => "https://www.douyin.com",
        "kuaishou" => "https://www.kuaishou.com",
        "bilibili" => "https://www.bilibili.com",
        _ => return Ok(false),
    };

    let resp = cdp
        .send("Network.getCookies", serde_json::json!({ "urls": [url] }))
        .await?;

    if let Some(cookies) = resp.pointer("/result/cookies").and_then(|v| v.as_array()) {
        for c in cookies {
            let name = c.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let c_domain = c.get("domain").and_then(|v| v.as_str()).unwrap_or("");
            let value = c.get("value").and_then(|v| v.as_str()).unwrap_or("");
            if name == key_cookie && c_domain.contains(domain.trim_start_matches('.')) && !value.is_empty() {
                eprintln!("[CDP] 平台 {} 已有有效 cookie: {}={}...", platform, name, &value[..value.len().min(8)]);
                return Ok(true);
            }
        }
    }
    Ok(false)
}

/// 智能 Cookie 管理：
/// 1. 检查 Chrome 是否已有该平台的登录态 → 有就跳过
/// 2. 没有 → 从数据库取所有 active 账号，逐个注入并验证
/// 3. 验证通过 → 使用该账号
/// 4. 验证失败 → 标记 expired，试下一个
/// 5. 全部失败 → 通知前端
async fn ensure_platform_cookies(
    app: &tauri::AppHandle,
    cdp: &Arc<CdpClient>,
    platform: &str,
) -> Result<(), String> {
    let (domain, _key_cookie) = match platform_cookie_info(platform) {
        Some(info) => info,
        None => return Ok(()),
    };

    // 1) 检查 Chrome 是否已经有该平台的有效 auth cookie
    if has_auth_cookie(cdp, platform).await.unwrap_or(false) {
        return Ok(());
    }

    eprintln!("[CDP] 平台 {} 无有效 cookie，从数据库获取账号...", platform);

    // 2) 从数据库获取所有 active 账号
    let db = app.state::<crate::database::DbState>();
    let accounts: Vec<(i64, String, Option<String>)> = db.with_conn(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, name, cookies FROM platform_accounts WHERE platform = ?1 AND status = 'active' ORDER BY updated_at DESC",
        )?;
        let rows = stmt.query_map(rusqlite::params![platform], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?, row.get::<_, Option<String>>(2)?))
        })?;
        Ok(rows.filter_map(|r| r.ok()).collect::<Vec<_>>())
    })?;

    if accounts.is_empty() {
        let platform_name = match platform {
            "douyin" => "抖音",
            "kuaishou" => "快手",
            "bilibili" => "B站",
            _ => platform,
        };
        let msg = format!("{}未登录账号，请先在「账号登记」中登录", platform_name);
        let _ = app.emit("cdp-parse-progress", serde_json::json!({"message": &msg}));
        return Err(msg);
    }

    // 3) 逐个尝试注入并验证
    for (account_id, account_name, cookies_opt) in &accounts {
        let cookies = match cookies_opt {
            Some(ck) if !ck.is_empty() => ck.as_str(),
            _ => continue,
        };

        eprintln!("[CDP] 尝试账号: {} (id={})", account_name, account_id);

        // 先清除旧的该域名 cookies
        let _ = cdp.send(
            "Network.deleteCookies",
            serde_json::json!({ "name": "*", "domain": domain }),
        ).await;

        // 注入这个账号的 cookies
        inject_cookies(cdp, cookies, domain).await?;

        // 验证注入后是否有关键 auth cookie
        if has_auth_cookie(cdp, platform).await.unwrap_or(false) {
            eprintln!("[CDP] 账号 {} 的 cookies 有效", account_name);
            return Ok(());
        }

        // 这个账号的 cookies 无效，标记为 expired
        eprintln!("[CDP] 账号 {} 的 cookies 无效，标记为 expired", account_name);
        let account_id_owned = *account_id;
        let _ = db.with_conn(move |conn| {
            conn.execute(
                "UPDATE platform_accounts SET status = 'expired' WHERE id = ?1",
                rusqlite::params![account_id_owned],
            )?;
            Ok(())
        });
    }

    // 4) 全部失败
    let platform_name = match platform {
        "douyin" => "抖音",
        "kuaishou" => "快手",
        "bilibili" => "B站",
        _ => platform,
    };
    let msg = format!("{}所有账号的登录已过期，请重新登录", platform_name);
    let _ = app.emit("cdp-parse-progress", serde_json::json!({"message": &msg}));
    let _ = app.emit("cdp-cookies-expired", serde_json::json!({
        "platform": platform,
        "message": &msg,
    }));
    Err(msg)
}

// ── 登录态检测 ─────────────────────────────────────────────────

pub async fn check_login_status(
    cdp: &Arc<CdpClient>,
    platform: &str,
) -> Result<bool, String> {
    has_auth_cookie(cdp, platform).await
}

// ── Tauri 命令 ─────────────────────────────────────────────────

#[tauri::command]
pub async fn cdp_ensure_chrome(app: tauri::AppHandle) -> Result<bool, String> {
    let state = app.state::<ChromeSessionState>();
    let mut session = state.0.lock().await;
    if session.is_alive() && session.cdp.is_some() {
        return Ok(true);
    }
    launch_and_connect(&app, &mut session, false).await?;
    Ok(true)
}

#[tauri::command]
pub async fn cdp_check_login(app: tauri::AppHandle, platform: String) -> Result<bool, String> {
    let state = app.state::<ChromeSessionState>();
    let session = state.0.lock().await;
    let cdp = session.cdp.as_ref().ok_or("Chrome 未启动")?;
    check_login_status(cdp, &platform).await
}

#[tauri::command]
pub async fn cdp_show_chrome(app: tauri::AppHandle) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let state = app.state::<ChromeSessionState>();
        let session = state.0.lock().await;
        if let Some(hwnd) = session.hwnd {
            show_chrome_window(hwnd);
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn cdp_hide_chrome(app: tauri::AppHandle) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let state = app.state::<ChromeSessionState>();
        let session = state.0.lock().await;
        if let Some(hwnd) = session.hwnd {
            hide_chrome_window(hwnd);
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn cdp_kill_chrome(app: tauri::AppHandle) -> Result<(), String> {
    let state = app.state::<ChromeSessionState>();
    let mut session = state.0.lock().await;
    session.kill();
    Ok(())
}

// ── 快手 CDP 解析 ──────────────────────────────────────────────

#[tauri::command]
pub async fn cdp_parse_kuaishou_video(
    app: tauri::AppHandle,
    page_url: String,
) -> Result<String, String> {
    let state = app.state::<ChromeSessionState>();
    let mut session = state.0.lock().await;

    if !session.is_alive() || session.cdp.is_none() {
        launch_and_connect(&app, &mut session, false).await?;
    }

    let cdp = session.cdp.as_ref().ok_or("Chrome 未启动")?.clone();
    drop(session);

    ensure_platform_cookies(&app, &cdp, "kuaishou").await?;
    let _ = app.emit("cdp-parse-progress", serde_json::json!({"message": "正在打开快手视频页面..."}));

    cdp.navigate_and_wait(&page_url, 15).await?;

    tokio::time::sleep(std::time::Duration::from_millis(2000)).await;

    let apollo_js = r#"(function(){
        try {
            var state = window.__APOLLO_STATE__;
            if (!state) return JSON.stringify({error: "未找到 __APOLLO_STATE__"});
            return JSON.stringify(state);
        } catch(e) {
            return JSON.stringify({error: e.message});
        }
    })()"#;

    let raw = cdp.eval(apollo_js).await?;
    if raw.is_empty() {
        return Err("快手页面返回空数据".into());
    }

    let _ = app.emit("cdp-parse-done", serde_json::json!({"platform": "kuaishou", "type": "video"}));

    Ok(raw)
}

#[tauri::command]
pub async fn cdp_parse_kuaishou_homepage(
    app: tauri::AppHandle,
    user_id: String,
) -> Result<String, String> {
    let state = app.state::<ChromeSessionState>();
    let mut session = state.0.lock().await;

    if !session.is_alive() || session.cdp.is_none() {
        launch_and_connect(&app, &mut session, false).await?;
    }

    let cdp = session.cdp.as_ref().ok_or("Chrome 未启动")?.clone();
    let event_rx = session.event_rx.as_ref().ok_or("事件通道未就绪")?.clone();
    drop(session);

    ensure_platform_cookies(&app, &cdp, "kuaishou").await?;
    let profile_url = format!("https://www.kuaishou.com/profile/{}", user_id);
    let _ = app.emit("cdp-parse-progress", serde_json::json!({"message": "正在打开快手主页..."}));

    let initial = navigate_and_intercept(
        &cdp,
        &event_rx,
        &profile_url,
        &["/graphql"],
        15,
    )
    .await?;

    let mut all_items: Vec<Value> = Vec::new();
    let mut seen_ids: HashSet<String> = HashSet::new();

    for res in &initial {
        parse_kuaishou_graphql_response(&res.body, &mut all_items, &mut seen_ids);
    }

    if !all_items.is_empty() {
        let _ = app.emit("cdp-parse-chunk", serde_json::json!({
            "platform": "kuaishou",
            "type": "homepage",
            "items": &all_items,
        }));
    }
    let _ = app.emit("cdp-parse-progress", serde_json::json!({
        "message": format!("已加载 {} 个视频（第1页）", all_items.len()),
    }));

    // fetch() 翻页
    let mut pcursor = String::new();
    let mut page = 1;
    loop {
        if page > 200 { break; }
        page += 1;

        let fetch_js = format!(
            r#"(async()=>{{try{{const r=await fetch('/rest/v/profile/feed',{{method:'POST',headers:{{'Content-Type':'application/json'}},body:JSON.stringify({{user_id:'{user_id}',pcursor:'{pcursor}',page:'profile'}})}});const d=await r.json();return JSON.stringify(d);}}catch(e){{return JSON.stringify({{ok:false,error:e.message}});}}}})();"#,
            user_id = user_id,
            pcursor = pcursor,
        );
        let raw = cdp.eval(&fetch_js).await?;
        if raw.is_empty() { break; }

        let json: Value = match serde_json::from_str(&raw) {
            Ok(v) => v,
            Err(_) => break,
        };

        if json.get("error").is_some() { break; }
        let result = json.get("result").and_then(|v| v.as_i64()).unwrap_or(-1);
        if result != 1 { break; }

        let new_pcursor = json.get("pcursor").and_then(|v| v.as_str()).unwrap_or("").to_string();

        let mut new_items = Vec::new();
        if let Some(feeds) = json.get("feeds").and_then(|f| f.as_array()) {
            for feed in feeds {
                let photo = feed.get("photo").unwrap_or(feed);
                let id = photo.get("id")
                    .and_then(|v| v.as_str().map(|s| s.to_string()).or_else(|| v.as_i64().map(|n| n.to_string())))
                    .unwrap_or_default();
                if !id.is_empty() && seen_ids.insert(id) {
                    new_items.push(photo.clone());
                }
            }
        }

        if !new_items.is_empty() {
            all_items.extend(new_items.clone());
            let _ = app.emit("cdp-parse-chunk", serde_json::json!({
                "platform": "kuaishou", "type": "homepage", "items": &new_items,
            }));
            let _ = app.emit("cdp-parse-progress", serde_json::json!({
                "message": format!("已加载 {} 个视频（第{}页）", all_items.len(), page),
            }));
        }

        if new_pcursor.is_empty() || new_pcursor == pcursor || new_pcursor == "no_more" {
            break;
        }
        pcursor = new_pcursor;

        let delay = random_delay_ms(800, 1500);
        tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
    }

    let _ = app.emit("cdp-parse-done", serde_json::json!({"platform": "kuaishou", "type": "homepage", "total": all_items.len()}));

    Ok(serde_json::json!({"total": all_items.len()}).to_string())
}

fn parse_kuaishou_graphql_response(body: &str, items: &mut Vec<Value>, seen: &mut HashSet<String>) {
    if let Ok(json) = serde_json::from_str::<Value>(body) {
        if let Some(feeds) = json
            .pointer("/data/visionProfilePhotoList/feeds")
            .and_then(|f| f.as_array())
        {
            for feed in feeds {
                if let Some(photo) = feed.get("photo") {
                    let id = photo
                        .get("id")
                        .and_then(|v| v.as_str().map(|s| s.to_string()).or_else(|| v.as_i64().map(|n| n.to_string())))
                        .unwrap_or_default();
                    if !id.is_empty() && seen.insert(id) {
                        items.push(photo.clone());
                    }
                }
            }
        }
    }
}

// ── B站 CDP 解析 ──────────────────────────────────────────────

#[tauri::command]
pub async fn cdp_parse_bilibili_video(
    app: tauri::AppHandle,
    bvid: String,
) -> Result<String, String> {
    let state = app.state::<ChromeSessionState>();
    let mut session = state.0.lock().await;

    if !session.is_alive() || session.cdp.is_none() {
        launch_and_connect(&app, &mut session, false).await?;
    }

    let cdp = session.cdp.as_ref().ok_or("Chrome 未启动")?.clone();
    let event_rx = session.event_rx.as_ref().ok_or("事件通道未就绪")?.clone();
    drop(session);

    ensure_platform_cookies(&app, &cdp, "bilibili").await?;
    let video_url = format!("https://www.bilibili.com/video/{}", bvid);
    let _ = app.emit("cdp-parse-progress", serde_json::json!({"message": "正在打开B站视频页面..."}));

    let results = navigate_and_intercept(
        &cdp,
        &event_rx,
        &video_url,
        &["/x/web-interface/view", "/x/player/playurl"],
        15,
    )
    .await?;

    let mut view_data: Option<Value> = None;
    let mut playurl_data: Option<Value> = None;

    for res in &results {
        if let Ok(json) = serde_json::from_str::<Value>(&res.body) {
            if json.get("data").is_some() {
                if res.url.contains("/x/web-interface/view") {
                    view_data = json.get("data").cloned();
                } else if res.url.contains("/x/player/playurl") {
                    playurl_data = json.get("data").cloned();
                }
            }
        }
    }

    let view = view_data.ok_or("未拦截到B站视频详情数据")?;

    let mut result = view.clone();
    if let Some(playurl) = playurl_data {
        result.as_object_mut().map(|obj| {
            obj.insert("__playurl__".to_string(), playurl);
        });
    }

    let _ = app.emit("cdp-parse-chunk", serde_json::json!({
        "platform": "bilibili",
        "type": "video",
        "items": [&result],
    }));
    let _ = app.emit("cdp-parse-done", serde_json::json!({"platform": "bilibili", "type": "video"}));

    Ok(serde_json::to_string(&result).unwrap_or_default())
}

#[tauri::command]
pub async fn cdp_parse_bilibili_homepage(
    app: tauri::AppHandle,
    mid: String,
) -> Result<String, String> {
    let state = app.state::<ChromeSessionState>();
    let mut session = state.0.lock().await;

    if !session.is_alive() || session.cdp.is_none() {
        launch_and_connect(&app, &mut session, false).await?;
    }

    let cdp = session.cdp.as_ref().ok_or("Chrome 未启动")?.clone();
    let event_rx = session.event_rx.as_ref().ok_or("事件通道未就绪")?.clone();
    drop(session);

    ensure_platform_cookies(&app, &cdp, "bilibili").await?;
    let space_url = format!("https://space.bilibili.com/{}/video", mid);
    let _ = app.emit("cdp-parse-progress", serde_json::json!({"message": "正在打开B站空间页面..."}));

    let initial = navigate_and_intercept(
        &cdp,
        &event_rx,
        &space_url,
        &["/x/space/wbi/arc/search"],
        15,
    )
    .await?;

    let mut all_items: Vec<Value> = Vec::new();
    let mut seen_ids: HashSet<String> = HashSet::new();
    let mut total_count: i64 = 0;

    for res in &initial {
        let tc = parse_bilibili_space_response_v2(&res.body, &mut all_items, &mut seen_ids);
        if tc > 0 { total_count = tc; }
    }

    if !all_items.is_empty() {
        let _ = app.emit("cdp-parse-chunk", serde_json::json!({
            "platform": "bilibili",
            "type": "homepage",
            "items": &all_items,
        }));
    }
    let _ = app.emit("cdp-parse-progress", serde_json::json!({
        "message": format!("已加载 {} 个视频（第1页）", all_items.len()),
    }));

    // fetch() 翻页，B站用 pn 页码 + ps 每页数量
    let ps = 30;
    let max_pages = if total_count > 0 { ((total_count + ps - 1) / ps).min(100) as usize } else { 100 };
    let mut page: usize = 1;
    while page < max_pages {
        page += 1;
        let fetch_js = format!(
            r#"(async()=>{{try{{const r=await fetch('/x/space/wbi/arc/search?mid={mid}&pn={pn}&ps={ps}&order=pubdate&platform=web&web_location=1550101');const d=await r.json();return JSON.stringify(d);}}catch(e){{return JSON.stringify({{error:e.message}});}}}})();"#,
            mid = mid,
            pn = page,
            ps = ps,
        );
        let raw = cdp.eval(&fetch_js).await?;
        if raw.is_empty() || raw.contains("\"error\"") { break; }

        let mut new_items = Vec::new();
        parse_bilibili_space_response_v2(&raw, &mut new_items, &mut seen_ids);

        if new_items.is_empty() { break; }

        all_items.extend(new_items.clone());
        let _ = app.emit("cdp-parse-chunk", serde_json::json!({
            "platform": "bilibili", "type": "homepage", "items": &new_items,
        }));
        let _ = app.emit("cdp-parse-progress", serde_json::json!({
            "message": format!("已加载 {} 个视频（第{}页）", all_items.len(), page),
        }));

        let delay = random_delay_ms(500, 1200);
        tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
    }

    let _ = app.emit("cdp-parse-done", serde_json::json!({"platform": "bilibili", "type": "homepage", "total": all_items.len()}));

    Ok(serde_json::json!({"total": all_items.len()}).to_string())
}

/// Returns total_count from the response (for calculating max pages)
fn parse_bilibili_space_response_v2(body: &str, items: &mut Vec<Value>, seen: &mut HashSet<String>) -> i64 {
    if let Ok(json) = serde_json::from_str::<Value>(body) {
        let total = json.pointer("/data/page/count").and_then(|v| v.as_i64()).unwrap_or(0);
        if let Some(vlist) = json
            .pointer("/data/list/vlist")
            .and_then(|l| l.as_array())
        {
            for item in vlist {
                let bvid = item
                    .get("bvid")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();
                if !bvid.is_empty() && seen.insert(bvid) {
                    items.push(item.clone());
                }
            }
        }
        return total;
    }
    0
}

// ── 登录流程（CDP 版） ────────────────────────────────────────

#[tauri::command]
pub async fn cdp_open_login(
    app: tauri::AppHandle,
    platform: String,
    cookies: Option<String>,
) -> Result<String, String> {
    let state = app.state::<ChromeSessionState>();
    let mut session = state.0.lock().await;

    let login_url = match platform.as_str() {
        "douyin" => "https://www.douyin.com",
        "kuaishou" => "https://www.kuaishou.com",
        "bilibili" => "https://www.bilibili.com",
        _ => return Err("不支持的平台".into()),
    };

    let cookie_domain = match platform.as_str() {
        "douyin" => ".douyin.com",
        "kuaishou" => ".kuaishou.com",
        "bilibili" => ".bilibili.com",
        _ => "",
    };

    if !session.is_alive() || session.cdp.is_none() {
        launch_and_connect(&app, &mut session, true).await?;
    } else {
        #[cfg(target_os = "windows")]
        if let Some(hwnd) = session.hwnd {
            show_chrome_window(hwnd);
        }
    }

    let cdp = session.cdp.as_ref().ok_or("Chrome 未启动")?.clone();

    if let Some(ref ck) = cookies {
        if !ck.is_empty() {
            inject_cookies(&cdp, ck, cookie_domain).await?;
        }
    }

    cdp.navigate_and_wait(login_url, 15).await?;

    // Inject login detection script
    let detect_script = build_cdp_login_script(&platform);
    cdp.send(
        "Page.addScriptToEvaluateOnNewDocument",
        serde_json::json!({"source": detect_script}),
    )
    .await?;
    cdp.send("Page.reload", serde_json::json!({"ignoreCache": false}))
        .await?;

    let app_clone = app.clone();
    let platform_clone = platform.clone();

    drop(session);

    tokio::spawn(async move {
        poll_login_status(&app_clone, &platform_clone).await;
    });

    Ok(format!("cdp_login_{}", platform))
}

async fn poll_login_status(app: &tauri::AppHandle, platform: &str) {
    let timeout = tokio::time::Instant::now() + std::time::Duration::from_secs(300);

    loop {
        tokio::time::sleep(std::time::Duration::from_millis(2000)).await;

        if tokio::time::Instant::now() > timeout {
            eprintln!("[CDP登录] 超时");
            break;
        }

        let state = app.state::<ChromeSessionState>();
        let session = state.0.lock().await;
        let cdp = match session.cdp.as_ref() {
            Some(c) => c.clone(),
            None => break,
        };
        drop(session);

        let check_js = match platform {
            "douyin" => {
                r#"(function(){
                    if (document.cookie.includes('sessionid')) {
                        var h1 = document.querySelector('h1');
                        var name = h1 ? h1.textContent.trim() : '';
                        var img = document.querySelector('img[src*="aweme"][src*="avatar"]') || document.querySelector('img[alt*="头像"]');
                        var avatar = img ? img.src : '';
                        return JSON.stringify({logged: true, name: name, avatar: avatar, cookies: document.cookie});
                    }
                    return JSON.stringify({logged: false});
                })()"#
            }
            "kuaishou" => {
                r#"(function(){
                    if (document.cookie.includes('userId')) {
                        return JSON.stringify({logged: true, name: '', avatar: '', cookies: document.cookie});
                    }
                    return JSON.stringify({logged: false});
                })()"#
            }
            "bilibili" => {
                r#"(function(){
                    if (document.cookie.includes('DedeUserID')) {
                        return JSON.stringify({logged: true, name: '', avatar: '', cookies: document.cookie});
                    }
                    return JSON.stringify({logged: false});
                })()"#
            }
            _ => break,
        };

        if let Ok(raw) = cdp.eval(check_js).await {
            if let Ok(data) = serde_json::from_str::<Value>(&raw) {
                if data.get("logged").and_then(|v| v.as_bool()) == Some(true) {
                    eprintln!("[CDP登录] {} 登录成功", platform);

                    let cookies_str = data.get("cookies").and_then(|v| v.as_str()).unwrap_or("");

                    // Get full cookies via CDP (includes HttpOnly)
                    let domain_url = match platform {
                        "douyin" => "https://www.douyin.com",
                        "kuaishou" => "https://www.kuaishou.com",
                        "bilibili" => "https://www.bilibili.com",
                        _ => "",
                    };

                    let full_cookies = if let Ok(resp) = cdp
                        .send(
                            "Network.getCookies",
                            serde_json::json!({"urls": [domain_url]}),
                        )
                        .await
                    {
                        resp.pointer("/result/cookies")
                            .and_then(|c| c.as_array())
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|c| {
                                        let name = c.get("name")?.as_str()?;
                                        let value = c.get("value")?.as_str()?;
                                        if name.is_empty() {
                                            return None;
                                        }
                                        Some(format!("{}={}", name, value))
                                    })
                                    .collect::<Vec<_>>()
                                    .join("; ")
                            })
                            .unwrap_or_else(|| cookies_str.to_string())
                    } else {
                        cookies_str.to_string()
                    };

                    let name = data.get("name").and_then(|v| v.as_str()).unwrap_or("");
                    let avatar = data.get("avatar").and_then(|v| v.as_str()).unwrap_or("");

                    let _ = app.emit(
                        "download-login-success",
                        serde_json::json!({
                            "cookies": full_cookies,
                            "platform": platform,
                            "name": name,
                            "avatar": avatar,
                            "label": format!("cdp_login_{}", platform),
                        }),
                    );

                    // Hide Chrome window after login
                    #[cfg(target_os = "windows")]
                    {
                        let state = app.state::<ChromeSessionState>();
                        let session = state.0.lock().await;
                        if let Some(hwnd) = session.hwnd {
                            if !is_dev_mode() {
                                hide_chrome_window(hwnd);
                            }
                        }
                    }

                    break;
                }
            }
        }
    }
}

fn build_cdp_login_script(platform: &str) -> String {
    match platform {
        "douyin" => {
            r#"
            (function(){
                if (window.location.pathname.indexOf('/user/self') === -1 &&
                    document.cookie.includes('sessionid')) {
                    window.location.href = 'https://www.douyin.com/user/self';
                }
            })();
            "#.to_string()
        }
        _ => String::new(),
    }
}

#[tauri::command]
pub async fn cdp_close_login(app: tauri::AppHandle) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let state = app.state::<ChromeSessionState>();
        let session = state.0.lock().await;
        if let Some(hwnd) = session.hwnd {
            if !is_dev_mode() {
                hide_chrome_window(hwnd);
            }
        }
    }
    Ok(())
}
