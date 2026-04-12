use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::process::{Child, Command};
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;

use clap::Parser;
use futures_util::{SinkExt, StreamExt};
use rand::Rng;
use serde::Serialize;
use tokio::sync::{mpsc, Mutex};
use tokio_tungstenite::tungstenite::Message;

// ── CLI 参数 ──────────────────────────────────────────────────────

#[derive(Parser)]
#[command(name = "ks_scraper", about = "快手主页视频采集（Chrome CDP 方案）")]
struct Args {
    #[arg(short, long)]
    url: String,

    #[arg(short, long, default_value = "")]
    cookies: String,

    #[arg(long, default_value_t = true)]
    auto_db: bool,

    #[arg(short, long, default_value_t = 9222)]
    port: u16,

    #[arg(short, long, default_value = "kuaishou_videos.csv")]
    output: String,

    #[arg(long, default_value_t = 8)]
    max_empty: usize,

    #[arg(long, default_value_t = 600)]
    timeout: u64,
}

// ── 从数据库读取 Cookie ───────────────────────────────────────────

fn read_cookies_from_db() -> Result<Vec<(String, String)>, String> {
    let db_path = dirs::data_dir()
        .ok_or("无法获取 AppData 目录")?
        .join("com.qingshi.app")
        .join("app_data.db");

    if !db_path.exists() {
        return Err(format!("数据库文件不存在: {}", db_path.display()));
    }

    let conn = rusqlite::Connection::open_with_flags(
        &db_path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .map_err(|e| format!("打开数据库失败: {}", e))?;

    let mut stmt = conn
        .prepare(
            "SELECT name, cookies FROM platform_accounts
             WHERE platform = 'kuaishou' AND cookies IS NOT NULL AND cookies != ''
             ORDER BY updated_at DESC",
        )
        .map_err(|e| format!("查询失败: {}", e))?;

    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
            ))
        })
        .map_err(|e| format!("读取失败: {}", e))?;

    let mut results = Vec::new();
    for row in rows {
        if let Ok((name, cookies)) = row {
            results.push((name, cookies));
        }
    }
    Ok(results)
}

// ── 视频数据结构 ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
struct VideoItem {
    id: String,
    caption: String,
    cover_url: String,
    photo_url: String,
    duration: f64,
    like_count: i64,
    view_count: i64,
    timestamp: i64,
    publish_time: String,
}

// ── Chrome 进程管理 ───────────────────────────────────────────────

fn find_chrome() -> Option<PathBuf> {
    let candidates = [
        std::env::var("PROGRAMFILES")
            .ok()
            .map(|p| PathBuf::from(p).join("Google\\Chrome\\Application\\chrome.exe")),
        std::env::var("PROGRAMFILES(X86)")
            .ok()
            .map(|p| PathBuf::from(p).join("Google\\Chrome\\Application\\chrome.exe")),
        std::env::var("LOCALAPPDATA")
            .ok()
            .map(|p| PathBuf::from(p).join("Google\\Chrome\\Application\\chrome.exe")),
        Some(PathBuf::from(
            "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe",
        )),
    ];
    for c in candidates.into_iter().flatten() {
        if c.exists() {
            return Some(c);
        }
    }
    None
}

fn launch_chrome(port: u16) -> Result<Child, String> {
    let chrome = find_chrome().ok_or("未找到 Chrome，请确认已安装 Google Chrome")?;
    let user_data = std::env::temp_dir().join("ks_scraper_chrome_data");
    std::fs::create_dir_all(&user_data)
        .map_err(|e| format!("创建临时目录失败: {}", e))?;

    let child = Command::new(&chrome)
        .arg("--app=about:blank")
        .arg(format!("--remote-debugging-port={}", port))
        .arg(format!("--user-data-dir={}", user_data.to_string_lossy()))
        .arg("--window-size=1200,900")
        .arg("--no-first-run")
        .arg("--no-default-browser-check")
        .arg("--disable-extensions")
        .arg("--disable-popup-blocking")
        .spawn()
        .map_err(|e| format!("启动 Chrome 失败: {}", e))?;

    Ok(child)
}

// ── CDP 客户端 ────────────────────────────────────────────────────

type WsSink = futures_util::stream::SplitSink<
    tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
    Message,
>;

struct CdpClient {
    sink: Arc<Mutex<WsSink>>,
    pending: Arc<Mutex<HashMap<i64, tokio::sync::oneshot::Sender<serde_json::Value>>>>,
    next_id: AtomicI64,
}

impl CdpClient {
    async fn connect(port: u16) -> Result<(Self, mpsc::UnboundedReceiver<serde_json::Value>), String> {
        let url = format!("http://127.0.0.1:{}/json", port);
        let tabs: Vec<serde_json::Value> = reqwest::Client::new()
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
        let pending: Arc<Mutex<HashMap<i64, tokio::sync::oneshot::Sender<serde_json::Value>>>> =
            Arc::new(Mutex::new(HashMap::new()));

        let (event_tx, event_rx) = mpsc::unbounded_channel::<serde_json::Value>();

        let pending_clone = pending.clone();
        tokio::spawn(async move {
            let mut stream = stream;
            while let Some(Ok(msg)) = stream.next().await {
                if let Message::Text(text) = msg {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
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
            CdpClient { sink, pending, next_id: AtomicI64::new(1) },
            event_rx,
        ))
    }

    async fn send(&self, method: &str, params: serde_json::Value) -> Result<serde_json::Value, String> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let msg = serde_json::json!({ "id": id, "method": method, "params": params });
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.pending.lock().await.insert(id, tx);
        self.sink.lock().await
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

    async fn send_no_params(&self, method: &str) -> Result<serde_json::Value, String> {
        self.send(method, serde_json::json!({})).await
    }

    /// 导航并等待页面 load 完成（动态等待，最多 max_secs 秒）
    async fn navigate_and_wait(&self, url: &str, event_rx: &Arc<Mutex<mpsc::UnboundedReceiver<serde_json::Value>>>, max_secs: u64) -> Result<(), String> {
        self.send("Page.navigate", serde_json::json!({ "url": url })).await?;
        let start = tokio::time::Instant::now();
        let deadline = start + std::time::Duration::from_secs(max_secs);
        let min_wait = std::time::Duration::from_millis(1500);
        loop {
            if tokio::time::Instant::now() >= deadline { break; }
            tokio::time::sleep(std::time::Duration::from_millis(400)).await;
            if let Ok(r) = self.send("Runtime.evaluate", serde_json::json!({"expression": "document.readyState"})).await {
                let state = r.pointer("/result/result/value").and_then(|v| v.as_str()).unwrap_or("");
                if state == "complete" && start.elapsed() >= min_wait { break; }
            }
        }
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        let _ = event_rx;
        Ok(())
    }

    /// 快速执行 JS 并返回结果字符串（自动处理 boolean/number/string）
    async fn eval(&self, expression: &str) -> Result<String, String> {
        let r = self.send("Runtime.evaluate", serde_json::json!({"expression": expression, "awaitPromise": true})).await?;
        let val = r.pointer("/result/result/value");
        let s = match val {
            Some(serde_json::Value::String(s)) => s.clone(),
            Some(serde_json::Value::Bool(b)) => b.to_string(),
            Some(serde_json::Value::Number(n)) => n.to_string(),
            Some(v) => v.to_string(),
            None => String::new(),
        };
        Ok(s)
    }
}

// ── Cookie 注入 ───────────────────────────────────────────────────

async fn inject_cookies(cdp: &CdpClient, cookies_str: &str) -> Result<(), String> {
    if cookies_str.is_empty() { return Ok(()); }
    let mut cdp_cookies = Vec::new();
    for part in cookies_str.split(';') {
        let part = part.trim();
        if let Some(idx) = part.find('=') {
            let name = part[..idx].trim();
            let value = part[idx + 1..].trim();
            if !name.is_empty() {
                cdp_cookies.push(serde_json::json!({
                    "name": name, "value": value,
                    "domain": ".kuaishou.com", "path": "/",
                }));
            }
        }
    }
    if !cdp_cookies.is_empty() {
        cdp.send("Network.setCookies", serde_json::json!({ "cookies": cdp_cookies })).await?;
        println!("[Cookie] 注入 {} 个", cdp_cookies.len());
    }
    Ok(())
}

// ── 响应解析 ──────────────────────────────────────────────────────

fn make_video_item(photo: &serde_json::Value) -> Option<VideoItem> {
    let id = photo
        .get("id")
        .or_else(|| photo.get("photo_id"))
        .and_then(|v| v.as_str().map(|s| s.to_string()).or_else(|| v.as_i64().map(|n| n.to_string())))
        .unwrap_or_default();
    if id.is_empty() { return None; }

    let ts = photo.get("timestamp").and_then(|v| v.as_i64()).unwrap_or(0);
    let publish_time = if ts > 0 {
        let ts_secs = if ts > 9_999_999_999 { ts / 1000 } else { ts };
        chrono::DateTime::from_timestamp(ts_secs, 0)
            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_default()
    } else { String::new() };

    Some(VideoItem {
        id,
        caption: photo.get("caption").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        cover_url: photo.get("coverUrl").or_else(|| photo.get("headUrl")).and_then(|v| v.as_str()).unwrap_or("").to_string(),
        photo_url: photo.get("photoUrl").or_else(|| photo.get("playUrl")).and_then(|v| v.as_str()).unwrap_or("").to_string(),
        duration: photo.get("duration").and_then(|v| v.as_f64()).unwrap_or(0.0),
        like_count: photo.get("realLikeCount").or_else(|| photo.get("likeCount")).and_then(|v| v.as_i64()).unwrap_or(0),
        view_count: photo.get("viewCount").and_then(|v| v.as_i64()).unwrap_or(0),
        timestamp: ts,
        publish_time,
    })
}

fn parse_video_response(body: &str) -> (Vec<VideoItem>, &'static str) {
    let json: serde_json::Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(_) => return (Vec::new(), "parse_error"),
    };
    if let Some(feeds) = json.pointer("/data/visionProfilePhotoList/feeds").and_then(|f| f.as_array()) {
        return (feeds.iter().filter_map(|f| f.get("photo").and_then(make_video_item)).collect(), "graphql");
    }
    if let Some(list) = json.pointer("/data/list").and_then(|l| l.as_array()) {
        return (list.iter().filter_map(make_video_item).collect(), "rest_list");
    }
    if let Some(feeds) = json.pointer("/data/feeds").and_then(|f| f.as_array()) {
        return (feeds.iter().filter_map(make_video_item).collect(), "rest_feeds");
    }
    if let Some(feeds) = json.get("feeds").and_then(|f| f.as_array()) {
        return (feeds.iter().filter_map(|f| {
            if let Some(photo) = f.get("photo") { return make_video_item(photo); }
            make_video_item(f)
        }).collect(), "rest_top_feeds");
    }
    (Vec::new(), "unknown")
}

// ── CSV 输出 ──────────────────────────────────────────────────────

fn write_csv(items: &[VideoItem], path: &str) -> Result<(), String> {
    let mut wtr = csv::Writer::from_path(path).map_err(|e| format!("创建 CSV 失败: {}", e))?;
    for item in items { wtr.serialize(item).map_err(|e| format!("写入失败: {}", e))?; }
    wtr.flush().map_err(|e| format!("刷新失败: {}", e))?;
    Ok(())
}

// ── 搜索解析自定义 ID → 3x 格式 ─────────────────────────────────

async fn resolve_custom_id(
    cdp: &CdpClient,
    raw_id: &str,
    event_rx: &Arc<Mutex<mpsc::UnboundedReceiver<serde_json::Value>>>,
) -> String {
    let search_url = format!("https://www.kuaishou.com/search/{}?source=SEARCH", raw_id);
    println!("[ID] 搜索: {}", search_url);

    if cdp.navigate_and_wait(&search_url, event_rx, 8).await.is_err() {
        println!("[ID] 搜索页加载失败");
        return raw_id.to_string();
    }

    // 等待「用户」tab 出现并点击（轮询，最多 6s）
    let mut tab_clicked = false;
    for _ in 0..12 {
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        if let Ok(val) = cdp.eval(r#"(function(){
            const els = document.querySelectorAll('button, div, span, a');
            for (const el of els) {
                if (el.textContent.trim() === '用户' && el.offsetParent !== null) {
                    el.click(); return 'ok';
                }
            }
            return 'not_found';
        })()"#).await {
            if val == "ok" { tab_clicked = true; break; }
        }
    }
    if !tab_clicked {
        println!("[ID] 未找到「用户」标签");
    }

    // 等待用户搜索结果加载（轮询检测「快手号」文本出现，最多 8s）
    let mut found = false;
    for _ in 0..16 {
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        if let Ok(val) = cdp.eval("document.body.innerText.includes('快手号')").await {
            if val == "true" { found = true; break; }
        }
    }

    if !found {
        println!("[ID] 搜索结果未加载，使用原始 ID");
        return raw_id.to_string();
    }

    // 提取「快手号：3x...」
    let extract_js = r#"(function(){
        const body = document.body.innerText;
        const re = /快手号[：:]\s*(3x[a-z0-9]+)/gi;
        const ids = [];
        let m;
        while ((m = re.exec(body)) !== null) { ids.push(m[1]); }
        return JSON.stringify(ids);
    })()"#;

    match cdp.send("Runtime.evaluate", serde_json::json!({"expression": extract_js})).await {
        Ok(r) => {
            let val = r.pointer("/result/result/value").and_then(|v| v.as_str()).unwrap_or("[]");
            let ids: Vec<String> = serde_json::from_str(val).unwrap_or_default();
            if let Some(first) = ids.first() {
                println!("[ID] {} -> {} (共{}个结果)", raw_id, first, ids.len());
                first.clone()
            } else {
                println!("[ID] 搜索页未找到快手号");
                raw_id.to_string()
            }
        }
        Err(_) => raw_id.to_string(),
    }
}

// ── 主流程 ────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let cookies = if !args.cookies.is_empty() {
        args.cookies.clone()
    } else if args.auto_db {
        match read_cookies_from_db() {
            Ok(accounts) if !accounts.is_empty() => {
                println!("[Cookie] 使用: {} ({}字符)", accounts[0].0, accounts[0].1.len());
                accounts[0].1.clone()
            }
            Ok(_) => { eprintln!("[警告] 数据库中无快手账号"); String::new() }
            Err(e) => { eprintln!("[警告] 读取数据库失败: {}", e); String::new() }
        }
    } else { String::new() };

    // 提取原始 ID
    let raw_id = {
        let path_part = args.url.split('?').next().unwrap_or(&args.url);
        path_part.trim_end_matches('/').rsplit('/').next().unwrap_or("").to_string()
    };
    let is_custom_id = !raw_id.starts_with("3x");

    println!("[配置] ID: {} | 输出: {} | 自定义ID: {}", raw_id, args.output, is_custom_id);

    // 1. 启动 Chrome
    let mut chrome = match launch_chrome(args.port) {
        Ok(c) => c,
        Err(e) => { eprintln!("[错误] {}", e); return; }
    };

    // 2. 连接 CDP（动态轮询，500ms 间隔，最多 8 次）
    let (cdp, mut event_rx) = {
        let mut result = None;
        for attempt in 1..=8 {
            tokio::time::sleep(std::time::Duration::from_millis(if attempt == 1 { 1500 } else { 500 })).await;
            match CdpClient::connect(args.port).await {
                Ok(pair) => { result = Some(pair); break; }
                Err(_) if attempt < 8 => {}
                Err(e) => { eprintln!("[错误] CDP 连接失败: {}", e); let _ = chrome.kill(); return; }
            }
        }
        match result {
            Some(pair) => pair,
            None => { eprintln!("[错误] CDP 连接超时"); let _ = chrome.kill(); return; }
        }
    };

    let _ = cdp.send_no_params("Network.enable").await;
    let _ = cdp.send_no_params("Page.enable").await;

    // 3. 事件处理
    let all_videos: Arc<Mutex<Vec<VideoItem>>> = Arc::new(Mutex::new(Vec::new()));
    let seen_ids: Arc<Mutex<HashSet<String>>> = Arc::new(Mutex::new(HashSet::new()));
    let pending_requests: Arc<Mutex<HashMap<String, String>>> = Arc::new(Mutex::new(HashMap::new()));

    let videos_clone = all_videos.clone();
    let seen_clone = seen_ids.clone();
    let pending_clone = pending_requests.clone();
    let cdp_arc = Arc::new(cdp);
    let cdp_for_scroll = cdp_arc.clone();

    let event_task = tokio::spawn(async move {
        while let Some(event) = event_rx.recv().await {
            let method = event.get("method").and_then(|m| m.as_str()).unwrap_or("");
            let params = event.get("params").cloned().unwrap_or(serde_json::json!({}));

            match method {
                "Network.responseReceived" => {
                    let url = params.pointer("/response/url").and_then(|u| u.as_str()).unwrap_or("");
                    let request_id = params.get("requestId").and_then(|r| r.as_str()).unwrap_or("");
                    if url.contains("/graphql") || url.contains("/rest/v/profile/feed") || url.contains("visionProfilePhotoList") {
                        pending_clone.lock().await.insert(request_id.to_string(), url.to_string());
                    }
                }
                "Network.loadingFinished" => {
                    let request_id = params.get("requestId").and_then(|r| r.as_str()).unwrap_or("").to_string();
                    let is_target = { pending_clone.lock().await.remove(&request_id).is_some() };
                    if is_target {
                        if let Ok(resp) = cdp_arc.send("Network.getResponseBody", serde_json::json!({"requestId": request_id})).await {
                            let body = resp.pointer("/result/body").and_then(|b| b.as_str()).unwrap_or("");
                            let (new_items, _) = parse_video_response(body);
                            if !new_items.is_empty() {
                                let mut videos = videos_clone.lock().await;
                                let mut seen = seen_clone.lock().await;
                                for item in new_items {
                                    if seen.insert(item.id.clone()) { videos.push(item); }
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    });

    // 4. 注入 Cookie
    let _ = inject_cookies(&cdp_for_scroll, &cookies).await;

    // 用 Arc 包装 event_rx 的占位（navigate_and_wait 需要签名兼容）
    let dummy_rx: Arc<Mutex<mpsc::UnboundedReceiver<serde_json::Value>>> =
        Arc::new(Mutex::new(mpsc::unbounded_channel().1));

    // 5. ID 解析 + 导航（自定义 ID 直接走搜索，跳过无效的 profile 导航）
    let user_id = if is_custom_id {
        let resolved = resolve_custom_id(&cdp_for_scroll, &raw_id, &dummy_rx).await;
        // 导航到解析后的 profile 页
        let profile_url = format!("https://www.kuaishou.com/profile/{}", resolved);
        println!("[导航] {}", profile_url);
        let _ = cdp_for_scroll.navigate_and_wait(&profile_url, &dummy_rx, 5).await;
        resolved
    } else {
        let profile_url = format!("https://www.kuaishou.com/profile/{}", raw_id);
        println!("[导航] {}", profile_url);
        let _ = cdp_for_scroll.navigate_and_wait(&profile_url, &dummy_rx, 5).await;
        raw_id.clone()
    };

    println!("[翻页] 用户 ID: {}", user_id);

    // 6. JS fetch 翻页采集
    let mut pcursor = String::new();
    let mut page_num = 0usize;
    let start_time = std::time::Instant::now();
    let timeout_dur = std::time::Duration::from_secs(args.timeout);

    loop {
        if start_time.elapsed() > timeout_dur {
            println!("[超时] {}s", args.timeout);
            break;
        }

        page_num += 1;

        let fetch_js = format!(
            r#"(async () => {{
                try {{
                    const resp = await fetch('/rest/v/profile/feed', {{
                        method: 'POST',
                        headers: {{ 'Content-Type': 'application/json' }},
                        body: JSON.stringify({{ user_id: '{}', pcursor: '{}', page: 'profile' }})
                    }});
                    const data = await resp.json();
                    return JSON.stringify({{ ok: true, pcursor: data.pcursor || '', feedCount: (data.feeds || []).length, result: data.result, feeds: data.feeds || [] }});
                }} catch(e) {{
                    return JSON.stringify({{ ok: false, err: e.message }});
                }}
            }})()"#,
            user_id, pcursor
        );

        let resp = cdp_for_scroll.send(
            "Runtime.evaluate",
            serde_json::json!({"expression": fetch_js, "awaitPromise": true}),
        ).await;

        let body_str = match resp {
            Ok(r) => r.pointer("/result/result/value").and_then(|v| v.as_str()).unwrap_or("{}").to_string(),
            Err(e) => { eprintln!("[错误] fetch: {}", e); break; }
        };

        let body_json: serde_json::Value = match serde_json::from_str(&body_str) {
            Ok(v) => v,
            Err(_) => break,
        };

        if body_json.get("ok").and_then(|v| v.as_bool()) != Some(true) { break; }

        let result = body_json.get("result").and_then(|v| v.as_i64()).unwrap_or(-1);
        if result != 1 {
            println!("[停止] result={}", result);
            break;
        }

        let new_pcursor = body_json.get("pcursor").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let _feed_count = body_json.get("feedCount").and_then(|v| v.as_i64()).unwrap_or(0);

        if let Some(feeds) = body_json.get("feeds").and_then(|f| f.as_array()) {
            let mut videos = all_videos.lock().await;
            let mut seen = seen_ids.lock().await;
            let mut added = 0usize;
            for feed in feeds {
                let photo = feed.get("photo").unwrap_or(feed);
                if let Some(item) = make_video_item(photo) {
                    if seen.insert(item.id.clone()) { videos.push(item); added += 1; }
                }
            }
            print!(
                "\r[翻页] 第{}页 +{} 累计{} ",
                page_num, added, videos.len(),
            );
            let _ = std::io::Write::flush(&mut std::io::stdout());
        }

        if new_pcursor.is_empty() || new_pcursor == pcursor || new_pcursor == "no_more" {
            println!("\n[完成] 无更多数据");
            break;
        }
        pcursor = new_pcursor;

        // 翻页间隔：800-1500ms，每 10 页暂停 1.5-2.5s
        let delay_ms = rand::thread_rng().gen_range(800..1500);
        let extra = if page_num % 10 == 0 { rand::thread_rng().gen_range(1500..2500) } else { 0 };
        tokio::time::sleep(std::time::Duration::from_millis(delay_ms + extra)).await;
    }

    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    event_task.abort();

    // 7. 输出
    let videos = all_videos.lock().await;
    println!("\n[结果] {} 个视频", videos.len());

    if !videos.is_empty() {
        match write_csv(&videos, &args.output) {
            Ok(_) => println!("[输出] {}", args.output),
            Err(e) => eprintln!("[错误] {}", e),
        }
    }

    let _ = chrome.kill();
    let _ = chrome.wait();
}
