use std::process::Command;
use std::sync::atomic::{AtomicI64, Ordering};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tauri::{Emitter, Manager};
use tokio_tungstenite::tungstenite::Message;

use crate::chrome_app::{find_chrome, LoginSession, LoginSessionManager, calc_chrome_window_size, get_main_hwnd};
use crate::database::DbState;

// ── 数据结构 ──────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PlatformAccount {
    pub id: i64,
    pub platform: String,
    pub name: String,
    pub avatar: String,
    pub cookies: Option<String>,
    pub status: String,
    pub remark: String,
    pub created_at: String,
    pub updated_at: String,
}

// ── 数据库操作（不变） ───────────────────────────────────────────

#[tauri::command]
pub fn list_download_accounts(state: tauri::State<'_, DbState>) -> Result<Vec<PlatformAccount>, String> {
    state.with_conn(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, platform, name, avatar, cookies, status, remark, created_at, updated_at
             FROM platform_accounts
             WHERE platform IN ('douyin', 'kuaishou', 'bilibili', 'migu')
             ORDER BY id DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(PlatformAccount {
                id: row.get(0)?,
                platform: row.get(1)?,
                name: row.get(2)?,
                avatar: row.get(3)?,
                cookies: row.get(4)?,
                status: row.get(5)?,
                remark: row.get(6)?,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>()
    })
}

#[tauri::command]
pub fn upsert_download_account(
    platform: String,
    name: String,
    cookies: String,
    avatar: Option<String>,
    remark: Option<String>,
    state: tauri::State<'_, DbState>,
) -> Result<i64, String> {
    state.with_conn(|conn| {
        let existing: Option<i64> = conn
            .query_row(
                "SELECT id FROM platform_accounts WHERE platform = ?1 AND name = ?2",
                rusqlite::params![platform, name],
                |row| row.get(0),
            )
            .ok();

        if let Some(id) = existing {
            conn.execute(
                "UPDATE platform_accounts SET cookies = ?1, avatar = ?2, status = 'active',
                 updated_at = datetime('now','localtime') WHERE id = ?3",
                rusqlite::params![cookies, avatar.unwrap_or_default(), id],
            )?;
            Ok(id)
        } else {
            conn.execute(
                "INSERT INTO platform_accounts (platform, name, avatar, cookies, status, remark)
                 VALUES (?1, ?2, ?3, ?4, 'active', ?5)",
                rusqlite::params![
                    platform,
                    name,
                    avatar.unwrap_or_default(),
                    cookies,
                    remark.unwrap_or_default()
                ],
            )?;
            Ok(conn.last_insert_rowid())
        }
    })
}

#[tauri::command]
pub fn update_platform_account(
    id: i64,
    updates: serde_json::Value,
    state: tauri::State<'_, DbState>,
) -> Result<(), String> {
    state.with_conn(|conn| {
        let obj = updates.as_object().unwrap_or(&serde_json::Map::new()).clone();
        let allowed = ["name", "avatar", "cookies", "status", "remark"];
        let mut sets = Vec::new();
        let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        for key in &allowed {
            if let Some(val) = obj.get(*key) {
                if let Some(s) = val.as_str() {
                    sets.push(format!("{} = ?", key));
                    params.push(Box::new(s.to_string()));
                }
            }
        }

        if sets.is_empty() {
            return Ok(());
        }

        sets.push("updated_at = datetime('now','localtime')".to_string());
        let sql = format!("UPDATE platform_accounts SET {} WHERE id = ?", sets.join(", "));
        params.push(Box::new(id));

        let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        conn.execute(&sql, param_refs.as_slice())?;
        Ok(())
    })
}

#[tauri::command]
pub fn delete_platform_account(id: i64, state: tauri::State<'_, DbState>) -> Result<(), String> {
    state.with_conn(|conn| {
        conn.execute("DELETE FROM platform_accounts WHERE id = ?1", rusqlite::params![id])?;
        Ok(())
    })
}

// ── Cookie 状态检测（不变，使用 HTTP 检测，不依赖浏览器） ────────

#[tauri::command]
pub async fn check_download_cookie_status(cookies: String, platform: String) -> Result<bool, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .redirect(reqwest::redirect::Policy::limited(3))
        .build()
        .map_err(|e| format!("HTTP客户端创建失败: {}", e))?;

    let ua = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36";

    match platform.as_str() {
        "douyin" => {
            let has_session = cookies.contains("sessionid_ss=") || cookies.contains("sessionid=");
            if !has_session {
                return Ok(false);
            }
            let resp = client
                .get("https://www.douyin.com/user/self")
                .header("Cookie", &cookies)
                .header("User-Agent", ua)
                .header("Referer", "https://www.douyin.com/")
                .send()
                .await
                .map_err(|e| format!("请求失败: {}", e))?;

            if !resp.status().is_success() {
                return Ok(false);
            }
            let body = resp.text().await.unwrap_or_default();
            let is_invalid = body.contains("用户未登录")
                || body.contains("\"statusCode\":8")
                || body.contains("\"statusCode\": 8");
            Ok(!is_invalid && body.len() > 1000)
        }
        "kuaishou" => {
            let has_user = cookies.contains("userId=");
            let has_session = cookies.contains("kuaishou.live.web_st=")
                || cookies.contains("kuaishou.server.webday7_st=");
            Ok(has_user && has_session)
        }
        "bilibili" => {
            let resp = client
                .get("https://api.bilibili.com/x/web-interface/nav")
                .header("Cookie", &cookies)
                .header("User-Agent", ua)
                .send()
                .await
                .map_err(|e| format!("请求失败: {}", e))?;

            if !resp.status().is_success() {
                return Ok(false);
            }
            let body = resp.text().await.unwrap_or_default();
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body) {
                let is_login = json
                    .get("data")
                    .and_then(|d| d.get("isLogin"))
                    .and_then(|l| l.as_bool())
                    .unwrap_or(false);
                Ok(is_login)
            } else {
                Ok(false)
            }
        }
        "migu" => {
            Ok(cookies.contains("checked_token=") || cookies.contains("UserInfo="))
        }
        _ => Ok(false),
    }
}

// ── CDP 辅助函数 ─────────────────────────────────────────────────

type WsStream = tokio_tungstenite::WebSocketStream<
    tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
>;

/// 发送 CDP 命令并等待响应（自动跳过中间的事件消息）
async fn cdp_call(
    ws: &mut WsStream,
    msg_id: &mut i64,
    method: &str,
    params: serde_json::Value,
) -> Result<serde_json::Value, String> {
    *msg_id += 1;
    let cmd = serde_json::json!({"id": *msg_id, "method": method, "params": params});
    ws.send(Message::Text(cmd.to_string()))
        .await
        .map_err(|e| format!("CDP 发送失败: {}", e))?;

    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(15);
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return Err(format!("CDP 命令超时: {}", method));
        }

        let msg = tokio::time::timeout(remaining, ws.next())
            .await
            .map_err(|_| format!("CDP 命令超时: {}", method))?
            .ok_or("WebSocket 已关闭")?
            .map_err(|e| format!("WebSocket 错误: {}", e))?;

        if let Message::Text(text) = msg {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                if json.get("id").and_then(|v| v.as_i64()) == Some(*msg_id) {
                    return Ok(json);
                }
            }
        }
    }
}

/// 连接到 Chrome CDP 调试端口
async fn cdp_connect(port: u16) -> Result<WsStream, String> {
    let url = format!("http://127.0.0.1:{}/json", port);
    let tabs: Vec<serde_json::Value> = reqwest::Client::new()
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("CDP 连接失败: {}", e))?
        .json()
        .await
        .map_err(|e| format!("CDP 解析失败: {}", e))?;

    let ws_url = tabs
        .iter()
        .find(|t| t["type"].as_str() == Some("page"))
        .and_then(|t| t["webSocketDebuggerUrl"].as_str())
        .ok_or("未找到 Chrome 页面标签")?
        .to_string();

    let (ws, _) = tokio_tungstenite::connect_async(&ws_url)
        .await
        .map_err(|e| format!("WebSocket 连接失败: {}", e))?;
    Ok(ws)
}

/// 连接 CDP（带重试）
async fn cdp_connect_retry(port: u16, max_attempts: u32) -> Result<WsStream, String> {
    let mut last_err = String::new();
    for i in 1..=max_attempts {
        let wait = if i == 1 { 1500 } else { 600 };
        tokio::time::sleep(std::time::Duration::from_millis(wait)).await;
        eprintln!("[CDP] 尝试连接 port={}, 第{}/{}次", port, i, max_attempts);
        match cdp_connect(port).await {
            Ok(ws) => {
                eprintln!("[CDP] 连接成功 port={}, 第{}次尝试", port, i);
                return Ok(ws);
            }
            Err(e) => {
                eprintln!("[CDP] 连接失败 port={}: {}", port, e);
                last_err = e;
            }
        }
    }
    eprintln!("[CDP] 连接最终失败 port={}: {}", port, last_err);
    Err(last_err)
}

// ── 端口分配 ─────────────────────────────────────────────────────

static NEXT_PORT_OFFSET: AtomicI64 = AtomicI64::new(0);

fn allocate_login_port() -> u16 {
    let offset = NEXT_PORT_OFFSET.fetch_add(1, Ordering::Relaxed) % 100;
    9300 + offset as u16
}

// ── 登录检测脚本（改用 window 变量回传，不再使用 location.href） ──

fn build_login_script(platform: &str) -> String {
    format!(
        r#"
(function() {{
    var PLATFORM = "{platform}";
    var DETECTED = false;
    var REDIRECTING = false;

    function hasCookie(name) {{
        return document.cookie.indexOf(name + '=') !== -1;
    }}

    function isLoggedIn() {{
        if (PLATFORM === 'douyin') {{
            return hasCookie('sessionid_ss') || hasCookie('sessionid')
                || (hasCookie('login_time') && hasCookie('passport_csrf_token'));
        }} else if (PLATFORM === 'kuaishou') {{
            return hasCookie('userId');
        }} else if (PLATFORM === 'bilibili') {{
            return hasCookie('DedeUserID');
        }} else if (PLATFORM === 'migu') {{
            return hasCookie('checked_token') || hasCookie('UserInfo');
        }}
        return false;
    }}

    function doCallback(name, avatar) {{
        if (REDIRECTING) return;
        REDIRECTING = true;
        var cookies = document.cookie || '';
        window.__LOGIN_CALLBACK_DATA = JSON.stringify({{
            cookies: cookies,
            platform: PLATFORM,
            name: name || '',
            avatar: avatar || ''
        }});
    }}

    if (PLATFORM === 'douyin' && window.location.pathname.indexOf('/user/self') !== -1) {{
        var INVALID_NAMES = ['未登录', '登录', '用户', '抖音', ''];
        var profileRetry = 0;
        var profileTimer = setInterval(function() {{
            if (REDIRECTING) {{ clearInterval(profileTimer); return; }}
            profileRetry++;
            if (profileRetry > 30) {{
                clearInterval(profileTimer);
                return;
            }}
            var h1 = document.querySelector('h1');
            if (h1 && h1.textContent && h1.textContent.trim()) {{
                var name = h1.textContent.trim();
                if (INVALID_NAMES.indexOf(name) !== -1 || name.length > 50) return;
                clearInterval(profileTimer);
                var avatarImg = document.querySelector('img[src*="aweme"][src*="avatar"]')
                    || document.querySelector('img[alt*="头像"]');
                doCallback(name, avatarImg ? avatarImg.src : '');
            }}
        }}, 1000);
        return;
    }}

    var LAST_COOKIE = '';

    if (PLATFORM === 'kuaishou' && window.location.hostname === 'live.kuaishou.com') {{
        var params = new URLSearchParams(window.location.search);
        var ksName = params.get('__ks_name') || '';
        var ksAvatar = params.get('__ks_avatar') || '';
        if (ksName) {{
            setTimeout(function() {{ doCallback(ksName, ksAvatar); }}, 2500);
            return;
        }}
    }}

    function verifyLogin() {{
        var currentCookie = document.cookie || '';
        if (PLATFORM === 'kuaishou') {{
            if (!hasCookie('userId')) return;
            if (currentCookie === LAST_COOKIE && DETECTED) return;
            LAST_COOKIE = currentCookie;
            DETECTED = true;
            fetch('https://www.kuaishou.com/graphql', {{
                method: 'POST', credentials: 'include',
                headers: {{ 'Content-Type': 'application/json' }},
                body: JSON.stringify({{ operationName: 'userInfoQuery', query: 'query userInfoQuery {{ userInfo {{ id eid name avatar }} }}', variables: {{}} }})
            }})
            .then(function(r) {{ return r.json(); }})
            .then(function(d) {{
                var info = d && d.data && d.data.userInfo;
                if (info && info.name) {{
                    window.location.href = 'https://live.kuaishou.com/?__ks_name='
                        + encodeURIComponent(info.name) + '&__ks_avatar=' + encodeURIComponent(info.avatar || '');
                }} else {{ DETECTED = false; }}
            }})
            .catch(function() {{ DETECTED = false; }});
        }} else if (PLATFORM === 'bilibili') {{
            if (!hasCookie('DedeUserID')) return;
            if (currentCookie === LAST_COOKIE && DETECTED) return;
            LAST_COOKIE = currentCookie;
            DETECTED = true;
            fetch('https://api.bilibili.com/x/web-interface/nav', {{ credentials: 'include' }})
            .then(function(r) {{ return r.json(); }})
            .then(function(d) {{
                if (d && d.code === 0 && d.data && d.data.isLogin && d.data.uname) {{
                    doCallback(d.data.uname, d.data.face || '');
                }} else {{ DETECTED = false; }}
            }})
            .catch(function() {{ DETECTED = false; }});
        }} else if (PLATFORM === 'migu') {{
            if (!hasCookie('checked_token') && !hasCookie('UserInfo')) return;
            if (currentCookie === LAST_COOKIE && DETECTED) return;
            LAST_COOKIE = currentCookie;
            DETECTED = true;
            try {{
                var uiMatch = document.cookie.match(/userInfo=([^;]+)/);
                if (uiMatch) {{
                    var info = JSON.parse(decodeURIComponent(uiMatch[1]));
                    if (info && info.sname) {{
                        doCallback(info.sname, info.picture || '');
                        return;
                    }}
                }}
            }} catch(e) {{}}
            var uidMatch = document.cookie.match(/UserInfo=([^|]+)/);
            doCallback(uidMatch ? uidMatch[1] : 'migu_user', '');
        }}
    }}

    setInterval(function() {{
        if (REDIRECTING) return;
        if (PLATFORM === 'douyin') {{
            if (!DETECTED && isLoggedIn()) {{
                DETECTED = true;
                window.location.href = 'https://www.douyin.com/user/self';
            }}
        }} else {{
            verifyLogin();
        }}
    }}, 2000);
}})();
"#,
        platform = platform
    )
}

// ── 辅助：强制终止进程树（Windows 上用 taskkill /T /F） ─────────

fn kill_process_tree(pid: u32) {
    #[cfg(target_os = "windows")]
    {
        eprintln!("[登录Chrome] 正在终止进程树 PID={}", pid);
        let output = Command::new("taskkill")
            .args(&["/PID", &pid.to_string(), "/T", "/F"])
            .creation_flags(0x08000000) // CREATE_NO_WINDOW
            .output();
        match output {
            Ok(o) => {
                let stdout = String::from_utf8_lossy(&o.stdout);
                let stderr = String::from_utf8_lossy(&o.stderr);
                eprintln!("[登录Chrome] taskkill 结果: status={}, stdout={}, stderr={}",
                    o.status, stdout.trim(), stderr.trim());
            }
            Err(e) => eprintln!("[登录Chrome] taskkill 执行失败: {}", e),
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = pid;
    }
}

/// 清理旧的登录 Chrome 数据目录（保留最新的，删除超过1小时的旧目录）
fn cleanup_old_login_dirs(base_dir: &std::path::Path) {
    if let Ok(entries) = std::fs::read_dir(base_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str.starts_with("chrome_login_") && entry.path().is_dir() {
                if let Ok(meta) = entry.metadata() {
                    if let Ok(modified) = meta.modified() {
                        if modified.elapsed().map_or(false, |d| d.as_secs() > 3600) {
                            eprintln!("[登录Chrome] 清理旧目录: {:?}", entry.path());
                            let _ = std::fs::remove_dir_all(entry.path());
                        }
                    }
                }
            }
        }
    }
}

// ── 打开登录窗口（Chrome --app + CDP 监控） ─────────────────────

#[tauri::command]
pub async fn open_download_login(
    app: tauri::AppHandle,
    platform: String,
    cookies: Option<String>,
) -> Result<String, String> {
    eprintln!("[登录Chrome] ===== 开始打开登录窗口 =====");
    eprintln!("[登录Chrome] 平台: {}, 有历史Cookie: {}", platform, cookies.as_ref().map_or(false, |c| !c.is_empty()));

    let chrome_path = find_chrome().ok_or(
        "未检测到 Google Chrome 浏览器，请先安装 Chrome。\n下载地址：https://www.google.cn/chrome/"
    )?;
    eprintln!("[登录Chrome] Chrome 路径: {:?}", chrome_path);

    let has_cookies = cookies.as_ref().map_or(false, |c| !c.is_empty());
    let (login_url, cookie_domain) = match platform.as_str() {
        "douyin" => ("https://www.douyin.com", ".douyin.com"),
        "kuaishou" => ("https://www.kuaishou.com", ".kuaishou.com"),
        "bilibili" => ("https://www.bilibili.com", ".bilibili.com"),
        "migu" => ("https://www.miguvideo.com", ".miguvideo.com"),
        _ => return Err("不支持的平台".into()),
    };

    let port = allocate_login_port();
    let label = format!("download_login_{}", chrono::Utc::now().timestamp_millis());
    eprintln!("[登录Chrome] 分配端口: {}, label: {}", port, label);

    // ── 第1步：关闭旧的登录会话（杀掉进程树） ──
    {
        let state = app.state::<LoginSessionManager>();
        let mut sessions = state.0.lock().map_err(|e| e.to_string())?;
        let old_keys: Vec<String> = sessions.keys().cloned().collect();
        eprintln!("[登录Chrome] 清理旧会话: {} 个", old_keys.len());
        for key in old_keys {
            if let Some(mut s) = sessions.remove(&key) {
                let pid = s.child.id();
                eprintln!("[登录Chrome] 终止旧会话: key={}, PID={}", key, pid);
                kill_process_tree(pid);
                let _ = s.child.kill();
                let _ = s.child.wait();
            }
        }
    }

    // 短暂等待进程退出
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;

    // ── 第2步：使用唯一的 user-data-dir（彻底避免 Chrome 单例冲突） ──
    let app_data = app.path().app_data_dir()
        .map_err(|e| format!("获取数据目录失败: {}", e))?;
    let session_id = chrono::Utc::now().timestamp_millis();
    let user_data = app_data.join(format!("chrome_login_{}", session_id));
    let _ = std::fs::create_dir_all(&user_data);
    eprintln!("[登录Chrome] user-data-dir: {:?}", user_data);

    // 后台清理超过1小时的旧登录目录
    let cleanup_base = app_data.clone();
    std::thread::spawn(move || cleanup_old_login_dirs(&cleanup_base));

    // ── 第3步：计算 Chrome 窗口尺寸 + 获取主窗口 HWND ──
    let (chrome_w, chrome_h) = calc_chrome_window_size(&app);
    let main_hwnd = get_main_hwnd(&app);
    eprintln!("[登录Chrome] Chrome尺寸: {}x{}, main_hwnd={}", chrome_w, chrome_h, main_hwnd);

    // ── 第4步：启动 Chrome（不指定位置，由 Win32 API 精确居中） ──
    eprintln!("[登录Chrome] 启动 Chrome: --app={} --window-size={},{} --port={}",
        login_url, chrome_w, chrome_h, port);

    let child = Command::new(&chrome_path)
        .arg(format!("--app={}", login_url))
        .arg(format!("--remote-debugging-port={}", port))
        .arg(format!("--user-data-dir={}", user_data.to_string_lossy()))
        .arg(format!("--window-size={},{}", chrome_w, chrome_h))
        .arg("--no-first-run")
        .arg("--no-default-browser-check")
        .arg("--disable-extensions")
        .arg("--disable-popup-blocking")
        .arg("--disable-infobars")
        .spawn()
        .map_err(|e| format!("启动 Chrome 失败: {}", e))?;

    let chrome_pid = child.id();
    eprintln!("[登录Chrome] Chrome 已启动, PID={}", chrome_pid);

    // 保存会话
    {
        let state = app.state::<LoginSessionManager>();
        let mut sessions = state.0.lock().map_err(|e| e.to_string())?;
        sessions.insert(label.clone(), LoginSession { child, port });
    }

    // 后台异步监控登录
    let app_clone = app.clone();
    let label_clone = label.clone();
    let cookies_to_inject = cookies.unwrap_or_default();
    let platform_clone = platform.clone();
    let cookie_domain_owned = cookie_domain.to_string();

    tokio::spawn(async move {
        if let Err(e) = run_login_monitor(
            &app_clone,
            port,
            login_url,
            &platform_clone,
            &label_clone,
            has_cookies,
            &cookies_to_inject,
            &cookie_domain_owned,
            chrome_pid,
            main_hwnd,
        ).await {
            eprintln!("[登录监控] 异常: {}", e);
        }
    });

    Ok(label)
}

/// 后台登录监控任务
#[allow(clippy::too_many_arguments)]
async fn run_login_monitor(
    app: &tauri::AppHandle,
    port: u16,
    login_url: &str,
    platform: &str,
    label: &str,
    has_cookies: bool,
    cookies_str: &str,
    cookie_domain: &str,
    chrome_pid: u32,
    main_hwnd: isize,
) -> Result<(), String> {
    crate::chrome_app::center_chrome_async(chrome_pid, main_hwnd).await;

    eprintln!("[登录监控] 开始连接 CDP, port={}, url={}", port, login_url);
    let mut ws = cdp_connect_retry(port, 8).await?;
    eprintln!("[登录监控] CDP 连接成功");
    let mut id: i64 = 0;

    cdp_call(&mut ws, &mut id, "Network.enable", serde_json::json!({})).await?;
    cdp_call(&mut ws, &mut id, "Page.enable", serde_json::json!({})).await?;
    cdp_call(&mut ws, &mut id, "Runtime.enable", serde_json::json!({})).await?;
    eprintln!("[登录监控] CDP 域已启用 (Network, Page, Runtime)");

    if has_cookies && !cookies_str.is_empty() {
        let mut cdp_cookies = Vec::new();
        for part in cookies_str.split(';') {
            let part = part.trim();
            if let Some(idx) = part.find('=') {
                let name = part[..idx].trim();
                let value = part[idx + 1..].trim();
                if !name.is_empty() {
                    cdp_cookies.push(serde_json::json!({
                        "name": name,
                        "value": value,
                        "domain": cookie_domain,
                        "path": "/",
                    }));
                }
            }
        }
        if !cdp_cookies.is_empty() {
            cdp_call(&mut ws, &mut id, "Network.setCookies",
                serde_json::json!({"cookies": cdp_cookies})).await?;
        }
    } else {
        cdp_call(&mut ws, &mut id, "Network.clearBrowserCookies", serde_json::json!({})).await?;
    }

    let init_script = build_login_script(platform);
    cdp_call(&mut ws, &mut id, "Page.addScriptToEvaluateOnNewDocument",
        serde_json::json!({"source": init_script})).await?;
    eprintln!("[登录监控] 登录检测脚本已注入");

    // Chrome 已经通过 --app={login_url} 直接打开了目标页面
    // 如果有 cookies 需要注入，reload 让 cookies 生效；否则只需 reload 让检测脚本生效
    let nav_result = cdp_call(&mut ws, &mut id, "Page.reload",
        serde_json::json!({"ignoreCache": false})).await?;
    eprintln!("[登录监控] 页面已 reload (让注入的脚本和 cookies 生效): {:?}",
        nav_result.get("id"));

    // 轮询检测登录回调数据（window.__LOGIN_CALLBACK_DATA）
    let timeout = tokio::time::Instant::now() + std::time::Duration::from_secs(300);

    loop {
        tokio::time::sleep(std::time::Duration::from_millis(800)).await;

        if tokio::time::Instant::now() > timeout {
            break;
        }

        // 检查 Chrome 是否还在运行
        {
            let state = app.state::<LoginSessionManager>();
            let sessions = state.0.lock().unwrap_or_else(|e| e.into_inner());
            if !sessions.contains_key(label) {
                break;
            }
        }

        let result = cdp_call(&mut ws, &mut id, "Runtime.evaluate", serde_json::json!({
            "expression": "typeof window.__LOGIN_CALLBACK_DATA !== 'undefined' ? window.__LOGIN_CALLBACK_DATA : ''"
        })).await;

        match result {
            Ok(r) => {
                if r.get("error").is_some() {
                    continue;
                }
                let val = r.pointer("/result/result/value")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                if val.is_empty() {
                    continue;
                }

                // 解析回调数据
                if let Ok(data) = serde_json::from_str::<serde_json::Value>(val) {
                    let _ = app.emit("download-login-success", serde_json::json!({
                        "cookies": data.get("cookies").and_then(|v| v.as_str()).unwrap_or(""),
                        "platform": data.get("platform").and_then(|v| v.as_str()).unwrap_or(""),
                        "name": data.get("name").and_then(|v| v.as_str()).unwrap_or(""),
                        "avatar": data.get("avatar").and_then(|v| v.as_str()).unwrap_or(""),
                        "label": label,
                    }));
                    break;
                }
            }
            Err(_) => {
                // WebSocket 断开（Chrome 被关闭）
                break;
            }
        }
    }

    Ok(())
}

// ── 捕获 Cookie（通过 CDP 获取完整 Cookie，包括 HttpOnly） ──────

#[tauri::command]
pub async fn capture_download_cookies(
    app: tauri::AppHandle,
    label: String,
    url: String,
) -> Result<String, String> {
    let port = {
        let state = app.state::<LoginSessionManager>();
        let sessions = state.0.lock().map_err(|e| e.to_string())?;
        sessions
            .get(&label)
            .map(|s| s.port)
            .ok_or_else(|| format!("会话不存在: {}", label))?
    };

    // 短生命周期 CDP 连接，仅用于获取 Cookie
    let mut ws = cdp_connect(port).await?;
    let mut id: i64 = 100_000; // 避免与监控任务的 ID 冲突

    let resp = cdp_call(&mut ws, &mut id, "Network.getCookies",
        serde_json::json!({"urls": [url]})).await?;

    let cookies = resp
        .pointer("/result/cookies")
        .and_then(|c| c.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|c| {
                    let name = c.get("name")?.as_str()?;
                    let value = c.get("value")?.as_str()?;
                    if name.is_empty() { return None; }
                    Some(format!("{}={}", name, value))
                })
                .collect::<Vec<_>>()
                .join("; ")
        })
        .unwrap_or_default();

    // 关闭临时连接
    let _ = ws.close(None).await;

    Ok(cookies)
}

// ── 关闭登录窗口（终止 Chrome 进程树） ───────────────────────────

#[tauri::command]
pub async fn close_download_webview(app: tauri::AppHandle, label: String) -> Result<(), String> {
    eprintln!("[登录Chrome] 关闭登录窗口: label={}", label);
    let state = app.state::<LoginSessionManager>();
    let mut sessions = state.0.lock().map_err(|e| e.to_string())?;

    if let Some(mut session) = sessions.remove(&label) {
        let pid = session.child.id();
        eprintln!("[登录Chrome] 终止 Chrome 进程树 PID={}", pid);
        kill_process_tree(pid);
        let _ = session.child.kill();
        let _ = session.child.wait();
    }

    Ok(())
}
