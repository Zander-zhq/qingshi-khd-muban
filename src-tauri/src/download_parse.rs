use std::collections::{BTreeMap, HashSet};

use serde::Serialize;
use tauri::{Emitter, Manager};

// ── 错误类型 ──────────────────────────────────────────────────────

#[derive(Debug)]
pub enum ParseError {
    Internal(String),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::Internal(msg) => write!(f, "{}", msg),
        }
    }
}

impl Serialize for ParseError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        #[derive(Serialize)]
        struct ErrorPayload {
            code: &'static str,
            message: String,
        }
        let payload = ErrorPayload {
            code: "PARSE_ERROR",
            message: self.to_string(),
        };
        payload.serialize(serializer)
    }
}

pub type ParseResult<T> = Result<T, ParseError>;

// ── 常量 ──────────────────────────────────────────────────────────

const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";

// ── 辅助函数 ──────────────────────────────────────────────────────

fn random_delay_ms(min: u64, max: u64) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    std::time::SystemTime::now().hash(&mut h);
    std::thread::current().id().hash(&mut h);
    let r = h.finish();
    min + (r % (max - min + 1))
}

async fn human_like_sleep(page: usize) {
    let base = random_delay_ms(2500, 5000);
    let extra = if page % 5 == 0 { random_delay_ms(5000, 10000) } else { 0 };
    tokio::time::sleep(std::time::Duration::from_millis(base + extra)).await;
}

fn build_http_client() -> ParseResult<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .redirect(reqwest::redirect::Policy::limited(10))
        .gzip(true)
        .brotli(true)
        .deflate(true)
        .build()
        .map_err(|e| ParseError::Internal(format!("HTTP客户端创建失败: {}", e)))
}

// ── 短链解析 ──────────────────────────────────────────────────────

#[tauri::command]
pub async fn resolve_video_url(url: String, _platform: String) -> ParseResult<String> {
    use std::sync::{Arc, Mutex as StdMutex};

    // 用于捕获重定向链中的合集 URL
    let collection_url: Arc<StdMutex<Option<String>>> = Arc::new(StdMutex::new(None));
    let collection_url_clone = collection_url.clone();

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .redirect(reqwest::redirect::Policy::custom(move |attempt| {
            let u = attempt.url().to_string();
            // 抖音合集：中间重定向经过 /collection/ 或 /mix/detail/ 时记录
            if u.contains("/collection/") || u.contains("/mix/detail/") {
                if let Some(id) = extract_mix_id_from_url(&u) {
                    let col_url = format!("https://www.douyin.com/collection/{}", id);
                    *collection_url_clone.lock().unwrap() = Some(col_url);
                }
            }
            if attempt.previous().len() >= 10 { attempt.stop() } else { attempt.follow() }
        }))
        .gzip(true).brotli(true).deflate(true)
        .build()
        .map_err(|e| ParseError::Internal(format!("HTTP客户端创建失败: {}", e)))?;

    let resp = client
        .get(&url)
        .header("User-Agent", UA)
        .send()
        .await
        .map_err(|e| ParseError::Internal(format!("短链解析失败: {}", e)))?;

    // 如果重定向链中发现了合集 URL，优先返回
    if let Some(col_url) = collection_url.lock().unwrap().take() {
        return Ok(col_url);
    }

    let mut resolved = resp.url().to_string();

    if resolved.contains("kuaishou.com") && resolved.contains("/profile/") {
        resolved = resolved.replace("www.kuaishou.com", "live.kuaishou.com");
    } else if resolved.contains("live.kuaishou.com") && !resolved.contains("/profile/") {
        resolved = resolved.replace("live.kuaishou.com", "www.kuaishou.com");
    }

    Ok(resolved)
}

fn extract_mix_id_from_url(url: &str) -> Option<String> {
    // /collection/{id} 或 /mix/detail/{id}
    if let Some(pos) = url.find("/collection/") {
        let after = &url[pos + 12..];
        let id: String = after.chars().take_while(|c| c.is_ascii_digit()).collect();
        if !id.is_empty() { return Some(id); }
    }
    if let Some(pos) = url.find("/mix/detail/") {
        let after = &url[pos + 12..];
        let id: String = after.chars().take_while(|c| c.is_ascii_digit()).collect();
        if !id.is_empty() { return Some(id); }
    }
    // object_id= 参数
    if let Some(pos) = url.find("object_id=") {
        let after = &url[pos + 10..];
        let id: String = after.chars().take_while(|c| c.is_ascii_digit()).collect();
        if !id.is_empty() { return Some(id); }
    }
    None
}

// ── 抖音单视频 (纯 API, 通过 iesdouyin 移动端分享页) ─────────────

const MOBILE_UA: &str = "Mozilla/5.0 (iPhone; CPU iPhone OS 16_6 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/16.6 Mobile/15E148 Safari/604.1";

#[tauri::command]
pub async fn api_parse_douyin_video(
    app: tauri::AppHandle,
    video_id: String,
) -> Result<String, String> {
    let share_url = format!("https://www.iesdouyin.com/share/video/{}", video_id);
    eprintln!("[抖音单视频API] 请求分享页: {}", share_url);

    let _ = app.emit("cdp-parse-progress", serde_json::json!({
        "message": "正在通过API解析抖音视频...",
    }));

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .redirect(reqwest::redirect::Policy::limited(10))
        .gzip(true).brotli(true).deflate(true)
        .build()
        .map_err(|e| format!("HTTP客户端创建失败: {}", e))?;

    let resp = client
        .get(&share_url)
        .header("User-Agent", MOBILE_UA)
        .header("Referer", "https://www.douyin.com/")
        .send()
        .await
        .map_err(|e| format!("请求分享页失败: {}", e))?;

    let status = resp.status();
    let html = resp.text().await.map_err(|e| format!("读取响应失败: {}", e))?;

    eprintln!("[抖音单视频API] HTTP {}, 响应长度={}", status, html.len());

    if html.is_empty() {
        return Err("分享页返回空数据".into());
    }

    let router_data = extract_router_data(&html)
        .ok_or("分享页中未找到 _ROUTER_DATA 数据")?;

    let json: serde_json::Value = serde_json::from_str(&router_data)
        .map_err(|e| format!("_ROUTER_DATA JSON解析失败: {}", e))?;

    let item_list = json
        .pointer("/loaderData/video_(id)~1page/videoInfoRes/item_list")
        .and_then(|v| v.as_array())
        .ok_or("未找到 item_list 数据")?;

    if item_list.is_empty() {
        return Err("item_list 为空，可能视频已被删除".into());
    }

    let detail = &item_list[0];

    let _ = app.emit("cdp-parse-chunk", serde_json::json!({
        "platform": "douyin",
        "type": "video",
        "items": [detail],
    }));

    let _ = app.emit("cdp-parse-done", serde_json::json!({
        "platform": "douyin",
        "type": "video",
    }));

    eprintln!("[抖音单视频API] 解析成功, aweme_id={}", detail.get("aweme_id").and_then(|v| v.as_str()).unwrap_or("?"));

    Ok(serde_json::to_string(detail).unwrap_or_default())
}

fn extract_router_data(html: &str) -> Option<String> {
    let marker = "_ROUTER_DATA";
    let pos = html.find(marker)?;
    let after_marker = &html[pos..];
    let eq_pos = after_marker.find('=')?;
    let after_eq = &after_marker[eq_pos + 1..];
    let after_eq = after_eq.trim_start();

    let script_end = after_eq.find("</script>")?;
    let json_str = after_eq[..script_end].trim().trim_end_matches(';');

    let decoded = json_str.replace("\\u002F", "/");
    Some(decoded)
}


// ── 抖音主页解析 (纯 HTTP, 无需 Chrome) ──────────────────────────

#[tauri::command]
pub async fn api_parse_douyin_homepage(
    app: tauri::AppHandle,
    sec_uid: String,
    cookies: String,
) -> Result<String, String> {
    use rquest_util::Emulation;
    let client = rquest::Client::builder()
        .emulation(Emulation::Chrome131)
        .timeout(std::time::Duration::from_secs(15))
        .redirect(rquest::redirect::Policy::limited(10))
        .gzip(true)
        .brotli(true)
        .deflate(true)
        .build()
        .map_err(|e| format!("HTTP客户端创建失败: {}", e))?;

    let mut all_items: Vec<serde_json::Value> = Vec::new();
    let mut seen_ids: HashSet<String> = HashSet::new();
    let mut max_cursor: i64 = 0;
    let mut page: usize = 0;
    let mut empty_pages: usize = 0;

    eprintln!("[抖音主页API] 开始解析, sec_uid={}", sec_uid);

    let _ = app.emit("cdp-parse-progress", serde_json::json!({
        "message": "正在加载抖音主页...",
    }));

    loop {
        page += 1;
        if page > 200 { break; }

        let api_url = format!(
            "https://www.douyin.com/aweme/v1/web/aweme/post/\
             ?sec_user_id={}&max_cursor={}&count=18\
             &device_platform=webapp&aid=6383\
             &version_code=290100&version_name=29.1.0\
             &platform=PC&publish_video_strategy_type=2",
            urlencoding::encode(&sec_uid),
            max_cursor
        );

        eprintln!("[抖音主页API] 第{}页, max_cursor={}", page, max_cursor);

        let resp = client
            .get(&api_url)
            .header("User-Agent", UA)
            .header("Referer", "https://www.douyin.com/")
            .header("Cookie", &cookies)
            .send()
            .await
            .map_err(|e| { eprintln!("[抖音主页API] HTTP请求失败: {}", e); format!("HTTP请求失败: {}", e) })?;

        let status = resp.status();
        let text = resp
            .text()
            .await
            .map_err(|e| format!("读取响应失败: {}", e))?;

        eprintln!("[抖音主页API] 第{}页: HTTP {}, 响应长度={}", page, status, text.len());

        if text.is_empty() {
            empty_pages += 1;
            if empty_pages >= 3 { break; }
            let delay = random_delay_ms(500, 1000);
            tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
            continue;
        }

        let data: serde_json::Value = serde_json::from_str(&text)
            .map_err(|e| format!("JSON解析失败: {}", e))?;

        let items = data.get("aweme_list").and_then(|v| v.as_array());
        let mut new_items: Vec<serde_json::Value> = Vec::new();

        if let Some(list) = items {
            for item in list {
                let id_opt = item.get("aweme_id").and_then(|v|
                    v.as_str().map(|s| s.to_string()).or_else(|| v.as_i64().map(|n| n.to_string()))
                );
                if let Some(id) = id_opt {
                    if seen_ids.insert(id) {
                        new_items.push(item.clone());
                    }
                }
            }
        }

        if new_items.is_empty() {
            empty_pages += 1;
            if empty_pages >= 3 { break; }
        } else {
            empty_pages = 0;
            all_items.extend(new_items.clone());
            let _ = app.emit("cdp-parse-chunk", serde_json::json!({
                "platform": "douyin",
                "type": "homepage",
                "items": &new_items,
            }));
            let _ = app.emit("cdp-parse-progress", serde_json::json!({
                "message": format!("已加载 {} 个作品（第{}页）", all_items.len(), page),
            }));
        }

        let has_more = data.get("has_more")
            .and_then(|v| v.as_i64().or_else(|| v.as_bool().map(|b| b as i64)))
            .unwrap_or(0);
        let next_cursor = data.get("max_cursor")
            .and_then(|v| v.as_i64().or_else(|| v.as_str().and_then(|s| s.parse().ok())))
            .unwrap_or(0);

        if has_more == 0 || next_cursor == 0 { break; }
        max_cursor = next_cursor;

        let delay = random_delay_ms(300, 800);
        tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
    }

    let _ = app.emit("cdp-parse-done", serde_json::json!({
        "platform": "douyin",
        "type": "homepage",
        "total": all_items.len(),
    }));

    Ok(serde_json::json!({"total": all_items.len()}).to_string())
}

// ── 抖音合集解析 (纯 HTTP, 无需 Chrome) ──────────────────────────

#[tauri::command]
pub async fn api_parse_douyin_collection(
    app: tauri::AppHandle,
    mix_id: String,
    cookies: String,
) -> Result<String, String> {
    use rquest_util::Emulation;
    let client = rquest::Client::builder()
        .emulation(Emulation::Chrome131)
        .timeout(std::time::Duration::from_secs(15))
        .redirect(rquest::redirect::Policy::limited(10))
        .gzip(true)
        .brotli(true)
        .deflate(true)
        .build()
        .map_err(|e| format!("HTTP客户端创建失败: {}", e))?;

    let mut all_items: Vec<serde_json::Value> = Vec::new();
    let mut seen_ids: HashSet<String> = HashSet::new();
    let mut cursor: i64 = 0;
    let mut page: usize = 0;
    let mut empty_pages: usize = 0;

    eprintln!("[抖音合集] 开始解析, mix_id={}", mix_id);

    let _ = app.emit("cdp-parse-progress", serde_json::json!({
        "message": "正在加载抖音合集...",
    }));

    loop {
        page += 1;
        if page > 200 { break; }

        let api_url = format!(
            "https://www.douyin.com/aweme/v1/web/mix/aweme/\
             ?mix_id={}&cursor={}&count=20\
             &device_platform=webapp&aid=6383\
             &version_code=170400&platform=PC",
            urlencoding::encode(&mix_id),
            cursor
        );

        eprintln!("[抖音合集] 第{}页, cursor={}", page, cursor);

        let resp = client
            .get(&api_url)
            .header("User-Agent", UA)
            .header("Referer", "https://www.douyin.com/")
            .header("Cookie", &cookies)
            .send()
            .await
            .map_err(|e| { eprintln!("[抖音合集] HTTP请求失败: {}", e); format!("HTTP请求失败: {}", e) })?;

        let status = resp.status();
        let text = resp
            .text()
            .await
            .map_err(|e| format!("读取响应失败: {}", e))?;

        eprintln!("[抖音合集] 第{}页: HTTP {}, 响应长度={}", page, status, text.len());

        // 临时日志：显示每页的 aweme_list 数量
        if let Ok(tmp) = serde_json::from_str::<serde_json::Value>(&text) {
            let count = tmp.get("aweme_list").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0);
            let has_more = tmp.get("has_more").and_then(|v| v.as_i64()).unwrap_or(-1);
            let next_cur = tmp.get("cursor").and_then(|v| v.as_i64()).unwrap_or(-1);
            eprintln!("[抖音合集] 第{}页: aweme_list={}, has_more={}, cursor={}", page, count, has_more, next_cur);
        }

        if text.is_empty() {
            empty_pages += 1;
            if empty_pages >= 3 { break; }
            let delay = random_delay_ms(500, 1000);
            tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
            continue;
        }

        let data: serde_json::Value = serde_json::from_str(&text)
            .map_err(|e| format!("JSON解析失败: {}", e))?;

        let items = data.get("aweme_list").and_then(|v| v.as_array());
        let mut new_items: Vec<serde_json::Value> = Vec::new();

        if let Some(list) = items {
            for item in list {
                let id_opt = item.get("aweme_id").and_then(|v|
                    v.as_str().map(|s| s.to_string()).or_else(|| v.as_i64().map(|n| n.to_string()))
                );
                if let Some(id) = id_opt {
                    if seen_ids.insert(id) {
                        new_items.push(item.clone());
                    }
                }
            }
        }

        if new_items.is_empty() {
            empty_pages += 1;
            if empty_pages >= 3 { break; }
        } else {
            empty_pages = 0;
            all_items.extend(new_items.clone());
            let _ = app.emit("cdp-parse-chunk", serde_json::json!({
                "platform": "douyin",
                "type": "collection",
                "items": &new_items,
            }));
            let _ = app.emit("cdp-parse-progress", serde_json::json!({
                "message": format!("已加载合集 {} 个视频（第{}页）", all_items.len(), page),
            }));
        }

        let has_more = data.get("has_more")
            .and_then(|v| v.as_i64().or_else(|| v.as_bool().map(|b| b as i64)))
            .unwrap_or(0);
        let next_cursor = data.get("cursor")
            .and_then(|v| v.as_i64().or_else(|| v.as_str().and_then(|s| s.parse().ok())))
            .unwrap_or(0);

        if has_more == 0 { break; }
        cursor = next_cursor;

        let delay = random_delay_ms(300, 800);
        tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
    }

    let _ = app.emit("cdp-parse-done", serde_json::json!({
        "platform": "douyin",
        "type": "collection",
        "total": all_items.len(),
    }));

    Ok(serde_json::json!({"total": all_items.len()}).to_string())
}

// ── 抖音合集查找（通过用户 sec_uid + video_id 查找 mix_id）────────

#[tauri::command]
pub async fn api_find_douyin_mix_id(
    sec_uid: String,
    video_id: String,
    cookies: String,
) -> Result<String, String> {
    use rquest_util::Emulation;
    let client = rquest::Client::builder()
        .emulation(Emulation::Chrome131)
        .timeout(std::time::Duration::from_secs(15))
        .redirect(rquest::redirect::Policy::limited(10))
        .gzip(true).brotli(true).deflate(true)
        .build()
        .map_err(|e| format!("HTTP客户端创建失败: {}", e))?;

    // 1. 获取用户所有合集列表
    let list_url = format!(
        "https://www.douyin.com/aweme/v1/web/mix/list/\
         ?sec_user_id={}&req_from=channel_pc_web&cursor=0&count=50\
         &list_scene=3&device_platform=webapp&aid=6383\
         &version_code=170400&platform=PC",
        urlencoding::encode(&sec_uid)
    );

    let resp = client.get(&list_url)
        .header("User-Agent", UA)
        .header("Referer", "https://www.douyin.com/")
        .header("Cookie", &cookies)
        .send().await
        .map_err(|e| format!("获取合集列表失败: {}", e))?;

    let data: serde_json::Value = resp.json().await
        .map_err(|e| format!("解析合集列表失败: {}", e))?;

    let mix_infos = data.get("mix_infos").and_then(|v| v.as_array())
        .ok_or("该用户没有合集")?;

    // 2. 逐个合集查找包含目标视频的合集
    for mix in mix_infos {
        let mix_id = mix.get("mix_id").and_then(|v| v.as_str()).unwrap_or("");
        let mix_name = mix.get("mix_name").and_then(|v| v.as_str()).unwrap_or("");
        if mix_id.is_empty() { continue; }

        let aweme_url = format!(
            "https://www.douyin.com/aweme/v1/web/mix/aweme/\
             ?mix_id={}&cursor=0&count=20\
             &device_platform=webapp&aid=6383\
             &version_code=170400&platform=PC",
            urlencoding::encode(mix_id)
        );

        let resp = client.get(&aweme_url)
            .header("User-Agent", UA)
            .header("Referer", "https://www.douyin.com/")
            .header("Cookie", &cookies)
            .send().await;

        if let Ok(resp) = resp {
            if let Ok(d) = resp.json::<serde_json::Value>().await {
                if let Some(list) = d.get("aweme_list").and_then(|v| v.as_array()) {
                    let found = list.iter().any(|item| {
                        item.get("aweme_id").and_then(|v| v.as_str()) == Some(&video_id)
                    });
                    if found {
                        eprintln!("[抖音合集查找] 在「{}」(mix_id={}) 中找到视频 {}", mix_name, mix_id, video_id);
                        return Ok(serde_json::json!({
                            "mix_id": mix_id,
                            "mix_name": mix_name,
                        }).to_string());
                    }
                }
            }
        }
    }

    Err(format!("在该用户的 {} 个合集中未找到视频 {}", mix_infos.len(), video_id))
}

// ── 快手 rquest 客户端 (TLS 指纹模拟) ─────────────────────────────

fn build_ks_client() -> Result<rquest::Client, String> {
    use rquest_util::Emulation;
    rquest::Client::builder()
        .emulation(Emulation::Chrome131)
        .timeout(std::time::Duration::from_secs(30))
        .redirect(rquest::redirect::Policy::limited(10))
        .gzip(true)
        .brotli(true)
        .deflate(true)
        .build()
        .map_err(|e| format!("快手HTTP客户端创建失败: {}", e))
}

async fn ks_graphql_post(
    client: &rquest::Client,
    query: &serde_json::Value,
    cookies: &str,
) -> Result<serde_json::Value, String> {
    let resp = client
        .post("https://www.kuaishou.com/graphql")
        .header("User-Agent", UA)
        .header("Referer", "https://www.kuaishou.com/")
        .header("Origin", "https://www.kuaishou.com")
        .header("Accept", "*/*")
        .header("Accept-Language", "zh-CN,zh;q=0.9,en;q=0.8")
        .header("Content-Type", "application/json")
        .header("Cookie", cookies)
        .header("sec-ch-ua", r#""Chromium";v="131", "Not_A Brand";v="24""#)
        .header("sec-ch-ua-mobile", "?0")
        .header("sec-ch-ua-platform", r#""Windows""#)
        .header("sec-fetch-dest", "empty")
        .header("sec-fetch-mode", "cors")
        .header("sec-fetch-site", "same-origin")
        .json(query)
        .send()
        .await
        .map_err(|e| format!("GraphQL请求失败: {}", e))?;

    let json: serde_json::Value = resp.json().await
        .map_err(|e| format!("GraphQL响应解析失败: {}", e))?;

    if let Some(errors) = json.get("errors") {
        return Err(format!("GraphQL错误: {}", errors));
    }
    json.get("data").cloned().ok_or_else(|| "GraphQL返回无data字段".to_string())
}

async fn ks_fetch_profile(
    client: &rquest::Client,
    user_id: &str,
    cookies: &str,
) -> Option<serde_json::Value> {
    let query = serde_json::json!({
        "operationName": "visionProfile",
        "query": "query visionProfile($userId: String) { visionProfile(userId: $userId) { userProfile { ownerCount { fan photo follow } profile { user_name user_id headurl user_text gender } } } }",
        "variables": { "userId": user_id }
    });

    let data = ks_graphql_post(client, &query, cookies).await.ok()?;
    let mut profile = data
        .get("visionProfile")
        .and_then(|p| p.get("userProfile"))
        .cloned()?;

    let profile_url = format!("https://www.kuaishou.com/profile/{}", user_id);
    if let Ok(page_resp) = client
        .get(&profile_url)
        .header("User-Agent", UA)
        .header("Referer", "https://www.kuaishou.com/")
        .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8")
        .header("Accept-Language", "zh-CN,zh;q=0.9,en;q=0.8")
        .header("Cookie", cookies)
        .header("sec-ch-ua", r#""Chromium";v="131", "Not_A Brand";v="24""#)
        .header("sec-ch-ua-mobile", "?0")
        .header("sec-ch-ua-platform", r#""Windows""#)
        .header("sec-fetch-dest", "document")
        .header("sec-fetch-mode", "navigate")
        .header("sec-fetch-site", "same-origin")
        .header("sec-fetch-user", "?1")
        .header("Upgrade-Insecure-Requests", "1")
        .send()
        .await
    {
        if let Ok(html) = page_resp.text().await {
            if let Some(cap_start) = html.find("\"userDefineId\":\"") {
                let val_start = cap_start + "\"userDefineId\":\"".len();
                if let Some(val_end) = html[val_start..].find('"') {
                    let define_id = &html[val_start..val_start + val_end];
                    if let Some(obj) = profile.as_object_mut() {
                        obj.insert("userDefineId".to_string(), serde_json::Value::String(define_id.to_string()));
                    }
                }
            }
        }
    }

    Some(profile)
}

// ── 快手 Cookie 合并 ──────────────────────────────────────────────

fn merge_set_cookies(original: &str, resp_headers: &rquest::header::HeaderMap) -> String {
    let mut map = BTreeMap::<String, String>::new();
    for segment in original.split(';') {
        let s = segment.trim();
        if let Some(idx) = s.find('=') {
            let name = s[..idx].trim().to_string();
            let value = s[idx + 1..].trim().to_string();
            if !name.is_empty() {
                map.insert(name, value);
            }
        }
    }
    for val in resp_headers.get_all(rquest::header::SET_COOKIE).iter() {
        if let Ok(sc) = val.to_str() {
            let first_part = sc.split(';').next().unwrap_or_default().trim();
            if let Some(idx) = first_part.find('=') {
                let name = first_part[..idx].trim().to_string();
                let value = first_part[idx + 1..].trim().to_string();
                if !name.is_empty() {
                    map.insert(name, value);
                }
            }
        }
    }
    map.iter().map(|(k, v)| format!("{}={}", k, v)).collect::<Vec<_>>().join("; ")
}

async fn ks_warmup_live_cookies(
    client: &rquest::Client,
    principal_id: &str,
    cookies: &str,
) -> String {
    let profile_url = format!("https://live.kuaishou.com/profile/{}", principal_id);
    match client
        .get(&profile_url)
        .header("User-Agent", UA)
        .header("Accept", "text/html,application/xhtml+xml,*/*;q=0.8")
        .header("Accept-Language", "zh-CN,zh;q=0.9")
        .header("Cookie", cookies)
        .header("sec-fetch-dest", "document")
        .header("sec-fetch-mode", "navigate")
        .header("sec-fetch-site", "none")
        .send()
        .await
    {
        Ok(resp) => {
            let merged = merge_set_cookies(cookies, resp.headers());
            log::info!("[KS-API] warmup done, cookies {} → {} chars", cookies.len(), merged.len());
            merged
        }
        Err(e) => {
            log::warn!("[KS-API] warmup failed: {}, using original cookies", e);
            cookies.to_string()
        }
    }
}

// ── 快手主页 API (三阶段回退) ─────────────────────────────────────

#[tauri::command]
pub async fn fetch_kuaishou_homepage_api(
    app: tauri::AppHandle,
    principal_id: String,
    cookies_list: Vec<String>,
) -> ParseResult<String> {
    use serde_json::json;

    if cookies_list.is_empty() {
        return Err(ParseError::Internal("快手 Cookie 池为空".into()));
    }

    let max_pages = 2500usize;

    let _ = app.emit("parse-debug-log", format!(
        "[快手] principalId={}, Cookie池={}个", principal_id, cookies_list.len()
    ));

    let ks_client = build_ks_client()
        .map_err(|e| ParseError::Internal(e))?;

    // ═══════════════════════════════════════════════════════════════
    // Phase 1 (首选): live_api 拿列表 + GraphQL visionVideoDetail 补全
    // ═══════════════════════════════════════════════════════════════
    let _ = app.emit("parse-debug-log", "[快手] Phase1: live_api 列表 + GraphQL 补全...".to_string());

    let mut live_cookies: Vec<String> = cookies_list.iter()
        .map(|c| c.replace("kpn=KUAISHOU_VISION", "kpn=GAME_ZONE"))
        .collect();

    let _ = app.emit("parse-debug-log", format!(
        "[快手] Cookie 预热中 ({}个)...", live_cookies.len()
    ));
    for i in 0..live_cookies.len() {
        live_cookies[i] = ks_warmup_live_cookies(&ks_client, &principal_id, &live_cookies[i]).await;
        if i + 1 < live_cookies.len() {
            tokio::time::sleep(std::time::Duration::from_millis(random_delay_ms(500, 1000))).await;
        }
    }
    let _ = app.emit("parse-debug-log", "[快手] Cookie 预热完成".to_string());

    let referer = format!("https://live.kuaishou.com/profile/{}", principal_id);

    let mut all_items: Vec<serde_json::Value> = Vec::new();
    let mut seen_ids = HashSet::new();
    let mut pcursor: Option<String> = None;
    let mut live_cookie_idx: usize = 0;
    let mut enrich_cookie_idx: usize = 0;
    let mut pages: usize = 0;
    let mut consecutive_r2: usize = 0;
    let max_r2 = 3 * live_cookies.len();
    let mut _live_api_ok = false;

    while pages < max_pages {
        let ci = live_cookie_idx % live_cookies.len();
        let cookie = &live_cookies[ci];

        let mut url = format!(
            "https://live.kuaishou.com/live_api/profile/public?principalId={}&count=12&privacy=public&caver=2&hasMore=true",
            urlencoding::encode(&principal_id),
        );
        if let Some(ref cur) = pcursor {
            url.push_str(&format!("&pcursor={}", urlencoding::encode(cur)));
        }

        let resp = ks_client
            .get(&url)
            .header("User-Agent", UA)
            .header("Accept", "application/json, text/plain, */*")
            .header("Accept-Language", "zh-CN,zh;q=0.9")
            .header("Referer", &referer)
            .header("Origin", "https://live.kuaishou.com")
            .header("Cookie", cookie.as_str())
            .header("sec-fetch-dest", "empty")
            .header("sec-fetch-mode", "cors")
            .header("sec-fetch-site", "same-origin")
            .send()
            .await;

        let resp = match resp {
            Ok(r) => r,
            Err(e) => {
                let _ = app.emit("parse-debug-log", format!(
                    "[快手] live_api 请求失败: {}, 切换到Phase2", e
                ));
                break;
            }
        };

        let resp_headers = resp.headers().clone();

        let body: serde_json::Value = match resp.json().await {
            Ok(b) => b,
            Err(e) => {
                let _ = app.emit("parse-debug-log", format!(
                    "[快手] live_api JSON解析失败: {}, 切换到Phase2", e
                ));
                break;
            }
        };

        live_cookies[ci] = merge_set_cookies(&live_cookies[ci], &resp_headers);

        let data_result = body.pointer("/data/result").and_then(|v| v.as_i64()).unwrap_or(-1);
        let list_len = body.pointer("/data/list").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0);
        let _ = app.emit("parse-debug-log", format!(
            "[快手] live_api 第{}页 cookie#{} result={} 返回{}条", pages + 1, ci, data_result, list_len
        ));

        if data_result == 2 {
            consecutive_r2 += 1;
            let _ = app.emit("parse-debug-log", format!(
                "[快手] live_api 被限流! 连续失败{}/{}", consecutive_r2, max_r2
            ));
            if consecutive_r2 >= max_r2 {
                let _ = app.emit("ks-parse-progress", json!({
                    "total": all_items.len(), "error": "所有Cookie均被限流"
                }));
                break;
            }
            let old_idx = live_cookie_idx;
            live_cookie_idx = (live_cookie_idx + 1) % live_cookies.len();
            let _ = app.emit("ks-parse-progress", json!({
                "total": all_items.len(), "cookie_switch": true, "from": old_idx, "to": live_cookie_idx
            }));
            tokio::time::sleep(std::time::Duration::from_millis(random_delay_ms(8000, 15000))).await;
            continue;
        }
        consecutive_r2 = 0;

        let list = match body.pointer("/data/list").and_then(|v| v.as_array()) {
            Some(l) if !l.is_empty() => l,
            _ => break,
        };

        let mut new_items: Vec<serde_json::Value> = Vec::new();
        for item in list {
            let id = item.get("id").and_then(|v| v.as_str()).unwrap_or_default().to_string();
            if id.is_empty() || seen_ids.contains(&id) { continue; }
            let play_url = item.get("playUrl").and_then(|v| v.as_str()).unwrap_or_default();
            if play_url.starts_with("http") {
                seen_ids.insert(id);
                new_items.push(item.clone());
            }
        }

        if !new_items.is_empty() {
            _live_api_ok = true;
            ks_enrich_video_details(
                &app, &ks_client, &mut new_items, &cookies_list, &mut enrich_cookie_idx,
            ).await;
            let _ = app.emit("ks-parse-chunk", json!({ "items": new_items }));
            all_items.extend(new_items.into_iter());
        }

        pages += 1;
        let _ = app.emit("ks-parse-progress", json!({ "total": all_items.len(), "pages": pages }));

        let next = body.pointer("/data/pcursor").and_then(|v| v.as_str()).map(|s| s.to_string());
        match next {
            Some(ref c) if !c.is_empty() && Some(c.clone()) != pcursor => { pcursor = Some(c.clone()); }
            _ => break,
        }

        tokio::time::sleep(std::time::Duration::from_millis(random_delay_ms(100, 300))).await;
    }

    if !all_items.is_empty() {
        let _ = app.emit("parse-debug-log", format!(
            "[快手] 完成: live_api+补全 共{}个视频, {}页", all_items.len(), pages
        ));
        return serde_json::to_string(&all_items)
            .map_err(|e| ParseError::Internal(format!("序列化失败: {}", e)));
    }

    // ═══════════════════════════════════════════════════════════════
    // Phase 2 (备选): GraphQL visionProfilePhotoList 列表
    // ═══════════════════════════════════════════════════════════════
    let _ = app.emit("parse-debug-log", "[快手] Phase1未获取到数据, 尝试Phase2: GraphQL 列表...".to_string());
    let profile_info = ks_fetch_profile(&ks_client, &principal_id, &cookies_list[0]).await;
    let mut gql_cookies = cookies_list.clone();
    let gql_result = ks_fetch_homepage_graphql(
        &app, &ks_client, &principal_id, &mut gql_cookies, max_pages, &profile_info,
    ).await;
    match &gql_result {
        Ok(items) if !items.is_empty() => {
            return serde_json::to_string(items)
                .map_err(|e| ParseError::Internal(format!("序列化失败: {}", e)));
        }
        _ => {}
    }

    // ═══════════════════════════════════════════════════════════════
    // Phase 3 (兜底): WebView 滚动加载
    // ═══════════════════════════════════════════════════════════════
    let _ = app.emit("parse-debug-log", "[快手] Phase2也失败, 尝试Phase3: WebView 兜底...".to_string());
    let ks_page_url = format!("https://www.kuaishou.com/profile/{}", principal_id);
    let webview_result = fetch_kuaishou_homepage(
        app.clone(), ks_page_url, cookies_list[0].clone(),
    ).await;
    match webview_result {
        Ok(data) => {
            if let Ok(items) = serde_json::from_str::<Vec<serde_json::Value>>(&data) {
                if !items.is_empty() { return Ok(data); }
            }
        }
        Err(_) => {}
    }

    Err(ParseError::Internal("快手所有解析方案均未获取到数据".into()))
}

// ── 快手 GraphQL 单视频详情补全 ──────────────────────────────────────

async fn ks_enrich_video_details(
    app: &tauri::AppHandle,
    client: &rquest::Client,
    items: &mut Vec<serde_json::Value>,
    cookies: &[String],
    cookie_idx: &mut usize,
) {
    let mut total_enriched = 0usize;
    let mut total_give_up = 0usize;
    let max_retries_per_video = 3usize;

    for item in items.iter_mut() {
        let id = item.get("id").and_then(|v| v.as_str()).unwrap_or_default().to_string();
        if id.is_empty() { continue; }

        let page_url = format!("https://www.kuaishou.com/short-video/{}", id);
        let mut enriched = false;

        for attempt in 0..max_retries_per_video {
            if attempt > 0 {
                tokio::time::sleep(std::time::Duration::from_millis(
                    random_delay_ms(1000, 2500)
                )).await;
            }

            let cookie_str = if cookies.is_empty() {
                String::new()
            } else {
                let ci = (*cookie_idx + attempt) % cookies.len();
                cookies[ci].clone()
            };

            let mut req = client
                .get(&page_url)
                .header("User-Agent", UA)
                .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8")
                .header("Accept-Language", "zh-CN,zh;q=0.9,en;q=0.8")
                .header("Referer", "https://www.kuaishou.com/")
                .header("sec-ch-ua", r#""Chromium";v="131", "Not_A Brand";v="24""#)
                .header("sec-ch-ua-mobile", "?0")
                .header("sec-ch-ua-platform", r#""Windows""#)
                .header("sec-fetch-dest", "document")
                .header("sec-fetch-mode", "navigate")
                .header("sec-fetch-site", "same-origin")
                .header("Upgrade-Insecure-Requests", "1");

            if !cookie_str.is_empty() {
                req = req.header("Cookie", cookie_str.as_str());
            }

            let resp = req.send().await;

            let html = match resp {
                Ok(r) => match r.text().await {
                    Ok(t) => t,
                    Err(e) => {
                        let _ = app.emit("parse-debug-log", format!(
                            "[快手补全] id={} 第{}次读取失败: {}", id, attempt + 1, e
                        ));
                        continue;
                    }
                },
                Err(e) => {
                    let _ = app.emit("parse-debug-log", format!(
                        "[快手补全] id={} 第{}次请求失败: {}", id, attempt + 1, e
                    ));
                    continue;
                }
            };

            let marker = "window.__APOLLO_STATE__=";
            let apollo = match html.find(marker) {
                Some(start) => {
                    let json_start = start + marker.len();
                    let rest = &html[json_start..];
                    let mut depth = 0i32;
                    let mut end = 0usize;
                    for (i, ch) in rest.char_indices() {
                        match ch {
                            '{' | '[' => depth += 1,
                            '}' | ']' => {
                                depth -= 1;
                                if depth == 0 { end = i + 1; break; }
                            }
                            _ => {}
                        }
                    }
                    if end == 0 { continue; }
                    match serde_json::from_str::<serde_json::Value>(&rest[..end]) {
                        Ok(v) => v,
                        Err(_) => { continue; }
                    }
                }
                None => {
                    if attempt + 1 < max_retries_per_video {
                        let _ = app.emit("parse-debug-log", format!(
                            "[快手补全] id={} 第{}次无数据, 重试中...", id, attempt + 1
                        ));
                    }
                    continue;
                }
            };

            if let Some(dc) = apollo.get("defaultClient").and_then(|c| c.as_object()) {
                let photo = dc.keys()
                    .find(|k| k.starts_with("VisionVideoDetailPhoto:"))
                    .and_then(|k| dc.get(k));

                if let Some(photo) = photo {
                    if let Some(obj) = item.as_object_mut() {
                        if let Some(v) = photo.get("caption") { obj.insert("__caption__".into(), v.clone()); }
                        if let Some(v) = photo.get("duration") { obj.insert("__duration__".into(), v.clone()); }
                        if let Some(v) = photo.get("timestamp") { obj.insert("__timestamp__".into(), v.clone()); }
                        if let Some(v) = photo.get("videoRatio") { obj.insert("__videoRatio__".into(), v.clone()); }
                        if let Some(v) = photo.get("realLikeCount").or_else(|| photo.get("likeCount")) {
                            obj.insert("__likeCount__".into(), v.clone());
                        }
                        if let Some(v) = photo.get("viewCount") { obj.insert("__viewCount__".into(), v.clone()); }
                        if let Some(v) = photo.get("coverUrl") { obj.insert("__coverUrl__".into(), v.clone()); }
                        if let Some(v) = photo.get("photoUrl") { obj.insert("__photoUrl__".into(), v.clone()); }
                    }
                    enriched = true;
                    break;
                }
            }
        }

        if enriched {
            total_enriched += 1;
        } else {
            total_give_up += 1;
            let _ = app.emit("parse-debug-log", format!(
                "[快手补全] id={} {}次尝试均失败, 跳过", id, max_retries_per_video
            ));
        }

        tokio::time::sleep(std::time::Duration::from_millis(random_delay_ms(50, 200))).await;
    }

    let _ = app.emit("parse-debug-log", format!(
        "[快手补全] 完成: 补全{}/{}条, 跳过{}条", total_enriched, items.len(), total_give_up
    ));
}

// ── 快手 GraphQL 主页 ─────────────────────────────────────────────

async fn ks_fetch_homepage_graphql(
    app: &tauri::AppHandle,
    client: &rquest::Client,
    principal_id: &str,
    cookies: &mut Vec<String>,
    max_pages: usize,
    profile_info: &Option<serde_json::Value>,
) -> Result<Vec<serde_json::Value>, String> {
    use serde_json::json;

    let gql = "query visionProfilePhotoList($userId:String,$pcursor:String,$page:String){visionProfilePhotoList(userId:$userId,pcursor:$pcursor,page:$page){result pcursor feeds{photo{...on PhotoEntity{id duration caption coverUrl photoUrl likeCount realLikeCount viewCount timestamp videoRatio}}}}}";

    let mut all_items: Vec<serde_json::Value> = Vec::new();
    let mut seen_ids = HashSet::new();
    let mut pcursor = String::new();
    let mut cookie_idx: usize = 0;
    let mut pages: usize = 0;
    let mut consecutive_fail: usize = 0;
    let max_fail = 3 * cookies.len();

    while pages < max_pages {
        let ci = cookie_idx % cookies.len();
        let cookie = cookies[ci].clone();

        let query = json!({
            "operationName": "visionProfilePhotoList",
            "query": gql,
            "variables": { "userId": principal_id, "pcursor": pcursor, "page": "profile" }
        });

        let result = ks_graphql_post(client, &query, &cookie).await;
        let data = match result {
            Ok(d) => d,
            Err(e) => {
                let _ = app.emit("parse-debug-log", format!("[快手GQL] 第{}页 cookie#{} 请求失败: {}", pages + 1, ci, e));
                consecutive_fail += 1;
                if consecutive_fail >= max_fail { return Err("GraphQL 认证失败".into()); }
                cookie_idx = (cookie_idx + 1) % cookies.len();
                let _ = app.emit("ks-parse-progress", json!({
                    "total": all_items.len(), "cookie_switch": true
                }));
                tokio::time::sleep(std::time::Duration::from_millis(random_delay_ms(8000, 15000))).await;
                continue;
            }
        };

        let list_data = data.get("visionProfilePhotoList");
        let result_code = list_data.and_then(|d| d.get("result")).and_then(|v| v.as_i64()).unwrap_or(-1);

        if result_code != 1 {
            let _ = app.emit("parse-debug-log", format!("[快手GQL] 第{}页 cookie#{} result={} (非1=失败)", pages + 1, ci, result_code));
            consecutive_fail += 1;
            if consecutive_fail >= max_fail { return Err(format!("GraphQL result={}", result_code)); }
            cookie_idx = (cookie_idx + 1) % cookies.len();
            let _ = app.emit("ks-parse-progress", json!({
                "total": all_items.len(), "cookie_switch": true
            }));
            tokio::time::sleep(std::time::Duration::from_millis(random_delay_ms(8000, 15000))).await;
            continue;
        }
        consecutive_fail = 0;

        let feeds = list_data
            .and_then(|d| d.get("feeds"))
            .and_then(|f| f.as_array());
        let feeds = match feeds {
            Some(f) => f,
            None => break,
        };

        if feeds.is_empty() { break; }

        let mut new_items: Vec<serde_json::Value> = Vec::new();
        for feed in feeds {
            if let Some(mut photo) = feed.get("photo").cloned() {
                let id = photo.get("id").and_then(|v| v.as_str()).unwrap_or_default().to_string();
                if id.is_empty() || seen_ids.contains(&id) { continue; }
                if let Some(ref pi) = profile_info {
                    photo.as_object_mut().map(|o| o.insert("__kuaishou_profile__".to_string(), pi.clone()));
                }
                seen_ids.insert(id);
                new_items.push(photo);
            }
        }

        all_items.extend(new_items.iter().cloned());
        pages += 1;
        if !new_items.is_empty() {
            let _ = app.emit("ks-parse-chunk", json!({ "items": new_items }));
        }
        let _ = app.emit("ks-parse-progress", json!({ "total": all_items.len(), "pages": pages }));

        if new_items.is_empty() { break; }

        let next = list_data
            .and_then(|d| d.get("pcursor"))
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();

        if next.is_empty() || next == pcursor { break; }
        pcursor = next;

        human_like_sleep(pages).await;
    }

    Ok(all_items)
}

// ── 快手 live_api 主页 ────────────────────────────────────────────

#[allow(dead_code)]
async fn ks_fetch_homepage_live_api(
    app: &tauri::AppHandle,
    client: &reqwest::Client,
    principal_id: &str,
    cookies: &mut Vec<String>,
    max_pages: usize,
    profile_info: &Option<serde_json::Value>,
) -> ParseResult<Vec<serde_json::Value>> {
    use serde_json::json;

    let referer = format!("https://live.kuaishou.com/profile/{}", principal_id);
    let mut all_items: Vec<serde_json::Value> = Vec::new();
    let mut seen_ids = HashSet::new();
    let mut pcursor: Option<String> = None;
    let mut cookie_idx: usize = 0;
    let mut pages: usize = 0;
    let mut consecutive_r2: usize = 0;
    let max_r2 = 3 * cookies.len();

    while pages < max_pages {
        let ci = cookie_idx % cookies.len();
        let cookie = cookies[ci].clone();

        let mut url = format!(
            "https://live.kuaishou.com/live_api/profile/public?principalId={}&count=12",
            urlencoding::encode(principal_id),
        );
        if let Some(ref cur) = pcursor {
            url.push_str(&format!("&pcursor={}", urlencoding::encode(cur)));
        }

        let resp = client
            .get(&url)
            .header("User-Agent", UA)
            .header("Accept", "application/json, text/plain, */*")
            .header("Accept-Language", "zh-CN,zh;q=0.9")
            .header("Referer", &referer)
            .header("Origin", "https://live.kuaishou.com")
            .header("Cookie", cookie.as_str())
            .header("sec-fetch-dest", "empty")
            .header("sec-fetch-mode", "cors")
            .header("sec-fetch-site", "same-origin")
            .send()
            .await
            .map_err(|e| ParseError::Internal(format!("快手API请求失败: {}", e)))?;

        let body: serde_json::Value = resp.json().await
            .map_err(|e| ParseError::Internal(format!("快手API JSON解析失败: {}", e)))?;

        let data_result = body.pointer("/data/result").and_then(|v| v.as_i64()).unwrap_or(-1);
        let list_len = body.pointer("/data/list").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0);
        let _ = app.emit("parse-debug-log", format!("[快手live] 第{}页 cookie#{} result={} 返回{}条", pages + 1, ci, data_result, list_len));
        if data_result == 2 {
            consecutive_r2 += 1;
            let _ = app.emit("parse-debug-log", format!("[快手live] 被限流! 连续失败{}/{}", consecutive_r2, max_r2));
            if consecutive_r2 >= max_r2 {
                let _ = app.emit("ks-parse-progress", json!({ "total": all_items.len(), "error": "所有Cookie均被限流" }));
                break;
            }
            cookie_idx = (cookie_idx + 1) % cookies.len();
            let _ = app.emit("ks-parse-progress", json!({ "total": all_items.len(), "cookie_switch": true }));
            tokio::time::sleep(std::time::Duration::from_millis(random_delay_ms(8000, 15000))).await;
            continue;
        }
        consecutive_r2 = 0;

        let list = match body.pointer("/data/list").and_then(|v| v.as_array()) {
            Some(l) if !l.is_empty() => l,
            _ => break,
        };

        let mut new_count = 0usize;
        for item in list {
            let id = item.get("id").and_then(|v| v.as_str()).unwrap_or_default().to_string();
            if id.is_empty() || seen_ids.contains(&id) { continue; }
            let play_url = item.get("playUrl").and_then(|v| v.as_str()).unwrap_or_default();
            if play_url.starts_with("http") {
                let mut item_clone = item.clone();
                if let Some(ref pi) = profile_info {
                    item_clone.as_object_mut().map(|o| o.insert("__kuaishou_profile__".to_string(), pi.clone()));
                }
                seen_ids.insert(id);
                all_items.push(item_clone);
                new_count += 1;
            }
        }

        pages += 1;
        let _ = app.emit("ks-parse-progress", json!({ "total": all_items.len(), "pages": pages }));
        if new_count == 0 { break; }

        let next = body.pointer("/data/pcursor").and_then(|v| v.as_str()).map(|s| s.to_string());
        match next {
            Some(ref c) if !c.is_empty() && Some(c.clone()) != pcursor => { pcursor = Some(c.clone()); }
            _ => break,
        }

        human_like_sleep(pages).await;
    }

    Ok(all_items)
}

// ── 快手 WebView 滚动加载主页 ─────────────────────────────────────

#[tauri::command]
pub async fn fetch_kuaishou_homepage(
    app: tauri::AppHandle,
    page_url: String,
    cookies: String,
) -> ParseResult<String> {
    let label = format!("ks_hp_{}", chrono::Utc::now().timestamp_millis());
    let (tx, mut rx) = tokio::sync::mpsc::channel::<String>(16);

    let cookies_escaped = cookies
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "");

    let init_script = format!(
        r#"
(function() {{
    if (window.__KS_SCROLL_STARTED__) return;
    window.__KS_SCROLL_STARTED__ = true;

    var USER_ID = (function() {{
        var m = window.location.pathname.match(/\/profile\/([^/?#]+)/);
        return m ? m[1] : '';
    }})();
    if (!USER_ID) return;

    var RAW_COOKIES = "{cookies_escaped}";

    if (RAW_COOKIES) {{
        RAW_COOKIES.split('; ').forEach(function(c) {{
            if (c.indexOf('=') > 0) {{
                document.cookie = c + '; domain=.kuaishou.com; path=/';
            }}
        }});
    }}

    var allPhotos = [];
    var seenIds = {{}};
    var profileInfo = null;
    var DONE = false;
    var noNewScrolls = 0;
    var scrollCount = 0;

    var SENDING = false;
    var SEND_QUEUE = [];
    function sendSignal(params) {{
        SEND_QUEUE.push(params);
        if (!SENDING) drainQueue();
    }}
    function drainQueue() {{
        if (SEND_QUEUE.length === 0) {{ SENDING = false; return; }}
        SENDING = true;
        var params = SEND_QUEUE.shift();
        window.location.href = 'https://ks-hp-cb.internal/?' + params;
        setTimeout(drainQueue, 200);
    }}

    var _origFetch = window.fetch;
    window.fetch = function() {{
        var args = arguments;
        var url = (typeof args[0] === 'string') ? args[0] : (args[0] && args[0].url) || '';
        var p = _origFetch.apply(this, args);
        if (url.indexOf('/graphql') !== -1) {{
            p.then(function(resp) {{
                resp.clone().json().then(function(json) {{
                    try {{
                        var d = json && json.data;
                        if (!d) return;
                        if (d.visionProfile && d.visionProfile.userProfile) {{
                            profileInfo = d.visionProfile.userProfile;
                            try {{
                                var did = extractUserDefineId();
                                if (did) profileInfo.userDefineId = did;
                            }} catch(e) {{}}
                            console.log('[KS滚动] 捕获用户资料');
                        }}
                        var pl = d.visionProfilePhotoList;
                        if (!pl || !pl.feeds) return;
                        var batch = [];
                        pl.feeds.forEach(function(f) {{
                            var photo = f && f.photo;
                            if (photo && photo.id && !seenIds[photo.id]) {{
                                seenIds[photo.id] = true;
                                if (profileInfo) photo.__kuaishou_profile__ = profileInfo;
                                allPhotos.push(photo);
                                batch.push(photo);
                            }}
                        }});
                        if (batch.length > 0) {{
                            noNewScrolls = 0;
                            console.log('[KS滚动] 拦截到 ' + batch.length + ' 个新视频, 累计 ' + allPhotos.length);
                            sendSignal('status=chunk&data=' + encodeURIComponent(JSON.stringify(batch)));
                        }}
                    }} catch(e) {{}}
                }}).catch(function() {{}});
            }}).catch(function() {{}});
        }}
        return p;
    }};

    function extractUserDefineId() {{
        try {{
            var scripts = document.querySelectorAll('script');
            for (var i = 0; i < scripts.length; i++) {{
                var text = scripts[i].textContent || '';
                var idx = text.indexOf('"userDefineId":"');
                if (idx !== -1) {{
                    var start = idx + '"userDefineId":"'.length;
                    var end = text.indexOf('"', start);
                    if (end !== -1) return text.substring(start, end);
                }}
            }}
        }} catch(e) {{}}
        return '';
    }}

    function doScroll() {{
        if (DONE) return;
        scrollCount++;
        var h = 500 + Math.floor(Math.random() * 500);
        window.scrollBy({{ top: h, behavior: 'smooth' }});
        console.log('[KS滚动] 第' + scrollCount + '次滚动 ' + h + 'px, 累计 ' + allPhotos.length + ' 个视频');

        var prevCount = allPhotos.length;
        setTimeout(function() {{
            if (DONE) return;
            if (allPhotos.length === prevCount) {{
                noNewScrolls++;
            }} else {{
                noNewScrolls = 0;
            }}
            if (noNewScrolls >= 5) {{
                DONE = true;
                console.log('[KS滚动] 连续5次滚动无新数据, 完成! 共 ' + allPhotos.length + ' 个视频');
                sendSignal('status=done&data=' + allPhotos.length);
                return;
            }}
            scheduleNext();
        }}, 2500);
    }}

    function scheduleNext() {{
        var delay = 3000 + Math.floor(Math.random() * 5000);
        if (scrollCount % 8 === 0) delay += 3000 + Math.floor(Math.random() * 5000);
        setTimeout(doScroll, delay);
    }}

    function checkRateLimit() {{
        try {{
            var bodyText = (document.body && document.body.innerText) || '';
            bodyText = bodyText.trim();
            if (bodyText.indexOf('"result"') !== -1) {{
                try {{
                    var obj = JSON.parse(bodyText);
                    if (obj && obj.result === 2) {{
                        console.log('[KS滚动] 检测到限流 result=2, 立即退出');
                        DONE = true;
                        sendSignal('status=warmup&data=result%3D2');
                        return true;
                    }}
                }} catch(e) {{}}
            }}
        }} catch(e) {{}}
        return false;
    }}

    function startWhenReady() {{
        if (document.readyState === 'complete') {{
            if (checkRateLimit()) return;
            window.__COOKIES_READY__ = true;
            console.log('[KS滚动] 页面加载完成, 3秒后开始滚动...');
            setTimeout(function() {{
                if (!DONE && checkRateLimit()) return;
                doScroll();
            }}, 3000);
        }} else {{
            setTimeout(startWhenReady, 500);
        }}
    }}

    startWhenReady();

    setTimeout(function() {{
        if (!DONE) {{
            DONE = true;
            console.log('[KS滚动] 超时, 返回已获取的 ' + allPhotos.length + ' 个视频');
            sendSignal('status=done&data=' + allPhotos.length);
        }}
    }}, 600000);
}})();
"#,
        cookies_escaped = cookies_escaped,
    );

    let _label_for_close = label.clone();
    let _app_for_close = app.clone();
    let app_for_progress = app.clone();

    let chunks: std::sync::Arc<std::sync::Mutex<Vec<String>>> =
        std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let chunks_for_nav = chunks.clone();
    let running_total = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let running_total_nav = running_total.clone();

    let _window = tauri::WebviewWindowBuilder::new(
        &app,
        &label,
        tauri::WebviewUrl::External(
            page_url
                .parse()
                .map_err(|_| ParseError::Internal("URL解析失败".into()))?,
        ),
    )
    .title(format!("快手主页解析: {}", page_url))
    .inner_size(1200.0, 800.0)
    .center()
    .visible(true)
    .skip_taskbar(false)
    .user_agent(UA)
    .initialization_script(&init_script)
    .on_navigation(move |nav_url| {
        if nav_url.host_str() == Some("ks-hp-cb.internal") {
            let status = nav_url
                .query_pairs()
                .find(|(k, _)| k == "status")
                .map(|(_, v)| v.to_string())
                .unwrap_or_default();
            let data = nav_url
                .query_pairs()
                .find(|(k, _)| k == "data")
                .map(|(_, v)| v.to_string())
                .unwrap_or_default();

            if status == "chunk" {
                if let Ok(mut guard) = chunks_for_nav.lock() {
                    let parsed_items = serde_json::from_str::<Vec<serde_json::Value>>(&data)
                        .unwrap_or_default();
                    let chunk_len = parsed_items.len();
                    guard.push(data);
                    let total = running_total_nav.fetch_add(chunk_len, std::sync::atomic::Ordering::Relaxed) + chunk_len;
                    if !parsed_items.is_empty() {
                        let _ = app_for_progress.emit("ks-parse-chunk", serde_json::json!({
                            "items": parsed_items
                        }));
                    }
                    let _ = app_for_progress.emit("ks-parse-progress", serde_json::json!({
                        "total": total,
                    }));
                }
            } else if status == "done" {
                let final_data = if let Ok(guard) = chunks_for_nav.lock() {
                    let combined: Vec<serde_json::Value> = guard
                        .iter()
                        .flat_map(|c| {
                            serde_json::from_str::<Vec<serde_json::Value>>(c)
                                .unwrap_or_default()
                        })
                        .collect();
                    serde_json::json!(combined).to_string()
                } else {
                    "[]".to_string()
                };
                let _ = tx.try_send(final_data);
            } else if status == "warmup" {
                let warmup_json = serde_json::json!({"error": format!("warmup:result={}", data)}).to_string();
                let _ = tx.try_send(warmup_json);
                let _ = app_for_progress.emit("ks-parse-progress", serde_json::json!({
                    "total": 0, "warmup": true,
                }));
            } else if status == "error" {
                let err_json = serde_json::json!({"error": data}).to_string();
                let _ = tx.try_send(err_json);
            }
            return false;
        }
        true
    })
    .build()
    .map_err(|e| ParseError::Internal(format!("无法创建快手解析WebView: {}", e)))?;

    #[cfg(target_os = "windows")]
    {
        let cookies_owned = cookies.clone();
        let window_for_signal = _window.clone();

        let _ = _window.with_webview(move |webview| {
            use webview2_com::Microsoft::Web::WebView2::Win32::*;
            use windows::core::{Interface, PCWSTR};

            unsafe {
                let controller: ICoreWebView2Controller = webview.controller();
                let core: ICoreWebView2 = controller.CoreWebView2().unwrap();
                let core2: ICoreWebView2_2 = core.cast().unwrap();
                let cookie_manager: ICoreWebView2CookieManager =
                    core2.CookieManager().unwrap();

                for part in cookies_owned.split("; ") {
                    let mut kv = part.splitn(2, '=');
                    let name = kv.next().unwrap_or("").trim();
                    let value = kv.next().unwrap_or("").trim();
                    if name.is_empty() {
                        continue;
                    }

                    let name_w: Vec<u16> =
                        name.encode_utf16().chain(std::iter::once(0)).collect();
                    let value_w: Vec<u16> =
                        value.encode_utf16().chain(std::iter::once(0)).collect();
                    let domain_w: Vec<u16> = ".kuaishou.com"
                        .encode_utf16()
                        .chain(std::iter::once(0))
                        .collect();
                    let path_w: Vec<u16> =
                        "/".encode_utf16().chain(std::iter::once(0)).collect();

                    if let Ok(cookie) = cookie_manager.CreateCookie(
                        PCWSTR(name_w.as_ptr()),
                        PCWSTR(value_w.as_ptr()),
                        PCWSTR(domain_w.as_ptr()),
                        PCWSTR(path_w.as_ptr()),
                    ) {
                        let _ = cookie_manager.AddOrUpdateCookie(&cookie);
                    }
                }
            }

            let _ = window_for_signal.eval("window.__COOKIES_READY__ = true;");
        });
    }

    let data = loop {
        let result = tokio::time::timeout(std::time::Duration::from_secs(660), rx.recv()).await;
        match result {
            Ok(Some(data)) => break data,
            Ok(None) => break "[]".to_string(),
            Err(_) => return Err(ParseError::Internal("快手主页解析超时(10min)".into())),
        }
    };

    if let Some(w) = _app_for_close.get_webview_window(&_label_for_close) {
        let _ = w.close();
    }

    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&data) {
        if let Some(err) = json.get("error").and_then(|e| e.as_str()) {
            return Err(ParseError::Internal(err.to_string()));
        }
    }

    Ok(data)
}

// ── 图片尺寸探测 ──────────────────────────────────────────────────

async fn fetch_jpeg_dimensions(client: &reqwest::Client, url: &str) -> Option<(u32, u32)> {
    let resp = client
        .get(url)
        .header("User-Agent", UA)
        .header("Range", "bytes=0-65535")
        .send()
        .await
        .ok()?;

    let bytes = resp.bytes().await.ok()?;
    if bytes.len() < 4 { return None; }

    if bytes.len() > 24 && bytes[0] == 0x89 && bytes[1] == 0x50 {
        let w = u32::from_be_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
        let h = u32::from_be_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]);
        return Some((w, h));
    }

    if bytes[0] == 0xFF && bytes[1] == 0xD8 {
        let mut i = 2usize;
        while i + 9 < bytes.len() {
            if bytes[i] != 0xFF { i += 1; continue; }
            let marker = bytes[i + 1];
            if marker == 0xC0 || marker == 0xC1 || marker == 0xC2 {
                let h = u16::from_be_bytes([bytes[i + 5], bytes[i + 6]]) as u32;
                let w = u16::from_be_bytes([bytes[i + 7], bytes[i + 8]]) as u32;
                return Some((w, h));
            }
            if marker == 0xD9 || marker == 0xDA { break; }
            if i + 3 >= bytes.len() { break; }
            let seg_len = u16::from_be_bytes([bytes[i + 2], bytes[i + 3]]) as usize;
            i += 2 + seg_len;
        }
    }

    None
}

// ── 快手单视频 ────────────────────────────────────────────────────

#[tauri::command]
pub async fn fetch_kuaishou_video(
    page_url: String,
    cookies: String,
) -> ParseResult<String> {
    let ks_client = build_ks_client()
        .map_err(|e| ParseError::Internal(e))?;

    let resp = ks_client
        .get(&page_url)
        .header("User-Agent", UA)
        .header("Referer", "https://www.kuaishou.com/")
        .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8")
        .header("Accept-Language", "zh-CN,zh;q=0.9,en;q=0.8")
        .header("Cookie", &cookies)
        .header("sec-ch-ua", r#""Chromium";v="131", "Not_A Brand";v="24""#)
        .header("sec-ch-ua-mobile", "?0")
        .header("sec-ch-ua-platform", r#""Windows""#)
        .header("sec-fetch-dest", "document")
        .header("sec-fetch-mode", "navigate")
        .header("sec-fetch-site", "none")
        .header("sec-fetch-user", "?1")
        .header("Upgrade-Insecure-Requests", "1")
        .send()
        .await
        .map_err(|e| ParseError::Internal(format!("快手页面请求失败: {}", e)))?;

    let status = resp.status();
    if !status.is_success() {
        return Err(ParseError::Internal(format!("快手页面返回HTTP {}", status)));
    }

    let html = resp.text().await
        .map_err(|e| ParseError::Internal(format!("读取快手页面失败: {}", e)))?;

    let marker = "window.__APOLLO_STATE__=";
    let start = html.find(marker)
        .ok_or_else(|| ParseError::Internal("页面中未找到视频数据，可能被风控拦截".into()))?;

    let json_start = start + marker.len();
    let rest = &html[json_start..];

    let mut depth = 0i32;
    let mut end = 0usize;
    for (i, ch) in rest.char_indices() {
        match ch {
            '{' | '[' => depth += 1,
            '}' | ']' => {
                depth -= 1;
                if depth == 0 {
                    end = i + 1;
                    break;
                }
            }
            _ => {}
        }
    }

    if end == 0 {
        return Err(ParseError::Internal("解析快手视频数据失败: 未找到完整JSON".into()));
    }

    let json_str = &rest[..end];
    let mut apollo: serde_json::Value = serde_json::from_str(json_str)
        .map_err(|e| ParseError::Internal(format!("快手数据JSON解析失败: {}", e)))?;

    if let Some(dc) = apollo.get("defaultClient").and_then(|c| c.as_object()) {
        let author_id = dc.keys()
            .find(|k| k.starts_with("VisionVideoDetailAuthor:"))
            .and_then(|k| dc.get(k))
            .and_then(|a| a.get("id"))
            .and_then(|id| id.as_str())
            .map(|s| s.to_string());

        let cover_url = dc.keys()
            .find(|k| k.starts_with("VisionVideoDetailPhoto:"))
            .and_then(|k| dc.get(k))
            .and_then(|p| p.get("coverUrl"))
            .and_then(|u| u.as_str())
            .map(|s| s.to_string());

        if let Some(uid) = author_id {
            if let Ok(ks_client_inner) = build_ks_client() {
                if let Some(profile) = ks_fetch_profile(&ks_client_inner, &uid, &cookies).await {
                    if let Some(dc_mut) = apollo.get_mut("defaultClient").and_then(|c| c.as_object_mut()) {
                        dc_mut.insert("__kuaishou_profile__".to_string(), profile);
                    }
                }
            }
        }

        if let Some(url) = cover_url {
            let reqwest_client = build_http_client()?;
            if let Some((w, h)) = fetch_jpeg_dimensions(&reqwest_client, &url).await {
                if let Some(dc_mut) = apollo.get_mut("defaultClient").and_then(|c| c.as_object_mut()) {
                    dc_mut.insert("__cover_width__".to_string(), serde_json::json!(w));
                    dc_mut.insert("__cover_height__".to_string(), serde_json::json!(h));
                }
            }
        }
    }

    Ok(apollo.to_string())
}


// ── B站 WBI 签名 ─────────────────────────────────────────────────

const MIXIN_KEY_ENC_TAB: [usize; 64] = [
    46, 47, 18, 2, 53, 8, 23, 32, 15, 50, 10, 31, 58, 3, 45, 35,
    27, 43, 5, 49, 33, 9, 42, 19, 29, 28, 14, 39, 12, 38, 41, 13,
    37, 48, 7, 16, 24, 55, 40, 61, 26, 17, 0, 1, 60, 51, 30, 4,
    22, 25, 54, 21, 56, 59, 6, 63, 57, 62, 11, 36, 20, 34, 44, 52,
];

fn get_mixin_key(img_key: &str, sub_key: &str) -> String {
    let raw: Vec<u8> = format!("{}{}", img_key, sub_key).bytes().collect();
    MIXIN_KEY_ENC_TAB
        .iter()
        .filter_map(|&i| raw.get(i).map(|&b| b as char))
        .take(32)
        .collect()
}

fn wbi_sign(params: &mut BTreeMap<String, String>, img_key: &str, sub_key: &str) {
    let mixin_key = get_mixin_key(img_key, sub_key);
    let wts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    params.insert("wts".into(), wts.to_string());

    for v in params.values_mut() {
        *v = v.chars().filter(|c| !"!'()*".contains(*c)).collect();
    }

    let query: String = params
        .iter()
        .map(|(k, v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v)))
        .collect::<Vec<_>>()
        .join("&");

    let w_rid = format!("{:x}", md5::compute(format!("{}{}", query, mixin_key)));
    params.insert("w_rid".into(), w_rid);
}

async fn fetch_wbi_keys(client: &reqwest::Client, cookies: &str) -> Result<(String, String), String> {
    let resp = client
        .get("https://api.bilibili.com/x/web-interface/nav")
        .header("User-Agent", UA)
        .header("Referer", "https://www.bilibili.com/")
        .header("Cookie", cookies)
        .send()
        .await
        .map_err(|e| format!("获取WBI密钥失败: {}", e))?;

    let json: serde_json::Value = resp.json().await
        .map_err(|e| format!("WBI响应解析失败: {}", e))?;

    let wbi_img = json.get("data")
        .and_then(|d| d.get("wbi_img"))
        .ok_or("WBI密钥数据缺失")?;

    let img_url = wbi_img.get("img_url").and_then(|v| v.as_str()).unwrap_or("");
    let sub_url = wbi_img.get("sub_url").and_then(|v| v.as_str()).unwrap_or("");

    let img_key = img_url.rsplit('/').next().unwrap_or("").split('.').next().unwrap_or("");
    let sub_key = sub_url.rsplit('/').next().unwrap_or("").split('.').next().unwrap_or("");

    if img_key.is_empty() || sub_key.is_empty() {
        return Err("WBI密钥提取失败".into());
    }
    Ok((img_key.to_string(), sub_key.to_string()))
}

// ── B站 API 通用 ──────────────────────────────────────────────────

async fn bili_api_get(
    client: &reqwest::Client,
    url: &str,
    cookies: &str,
) -> Result<serde_json::Value, String> {
    let resp = client
        .get(url)
        .header("User-Agent", UA)
        .header("Referer", "https://www.bilibili.com/")
        .header("Cookie", cookies)
        .send()
        .await
        .map_err(|e| format!("B站API请求失败: {}", e))?;

    let json: serde_json::Value = resp.json().await
        .map_err(|e| format!("B站API响应解析失败: {}", e))?;

    let code = json.get("code").and_then(|c| c.as_i64()).unwrap_or(-1);
    if code != 0 {
        let msg = json.get("message").and_then(|m| m.as_str()).unwrap_or("未知错误");
        return Err(format!("B站API错误(code={}): {}", code, msg));
    }
    json.get("data").cloned().ok_or("B站API返回无data字段".into())
}

async fn bili_fetch_playurl(
    client: &reqwest::Client,
    bvid: &str,
    cid: i64,
    cookies: &str,
) -> (String, i64, String, i64) {
    let url = format!(
        "https://api.bilibili.com/x/player/playurl?bvid={}&cid={}&qn=116&fnval=0&fourk=1",
        urlencoding::encode(bvid), cid
    );
    let play = bili_api_get(client, &url, cookies).await.ok();

    let mut video_url = String::new();
    let mut video_size: i64 = 0;
    let mut video_codec = String::new();
    let mut video_bitrate: i64 = 0;

    if let Some(ref play_data) = play {
        if let Some(durl) = play_data.get("durl").and_then(|d| d.as_array()) {
            if let Some(first) = durl.first() {
                video_url = first.get("url")
                    .and_then(|u| u.as_str())
                    .unwrap_or("")
                    .to_string();
                video_size = first.get("size")
                    .and_then(|s| s.as_i64())
                    .unwrap_or(0);
            }
        }
        let quality = play_data.get("quality").and_then(|q| q.as_i64()).unwrap_or(0);
        video_codec = match quality {
            116 => "H265 1080P60".into(),
            112 => "H264 1080P+".into(),
            80 => "H264 1080P".into(),
            64 => "H264 720P".into(),
            32 => "H264 480P".into(),
            16 => "H264 360P".into(),
            _ => format!("qn={}", quality),
        };
        if video_size > 0 {
            let dur = play_data.get("timelength").and_then(|t| t.as_i64()).unwrap_or(0) / 1000;
            if dur > 0 {
                video_bitrate = video_size * 8 / dur;
            }
        }
    }

    (video_url, video_size, video_codec, video_bitrate)
}

// ── B站单视频 ─────────────────────────────────────────────────────

#[tauri::command]
pub async fn fetch_bilibili_video(
    bvid: String,
    cookies: String,
) -> ParseResult<String> {
    let client = build_http_client()?;

    let view_url = format!(
        "https://api.bilibili.com/x/web-interface/view?bvid={}",
        urlencoding::encode(&bvid)
    );
    let view = bili_api_get(&client, &view_url, &cookies).await
        .map_err(|e| ParseError::Internal(e))?;

    let owner = view.get("owner").cloned().unwrap_or(serde_json::json!({}));
    let stat = view.get("stat").cloned().unwrap_or(serde_json::json!({}));
    let mid = owner.get("mid").and_then(|m| m.as_i64()).unwrap_or(0);
    let cid = view.get("cid").and_then(|c| c.as_i64()).unwrap_or(0);
    let dimension = view.get("dimension").cloned().unwrap_or(serde_json::json!({}));

    let card_url = format!(
        "https://api.bilibili.com/x/web-interface/card?mid={}",
        mid
    );
    let card = bili_api_get(&client, &card_url, &cookies).await.ok();
    let card_info = card.as_ref().and_then(|c| c.get("card"));
    let sign = card_info
        .and_then(|c| c.get("sign"))
        .and_then(|s| s.as_str())
        .unwrap_or("");
    let fans = card.as_ref()
        .and_then(|c| c.get("follower"))
        .and_then(|f| f.as_i64())
        .unwrap_or(0);

    let (video_url, video_size, video_codec, video_bitrate) =
        bili_fetch_playurl(&client, &bvid, cid, &cookies).await;

    let result = serde_json::json!({
        "bvid": bvid,
        "aid": view.get("aid").and_then(|a| a.as_i64()).unwrap_or(0),
        "title": view.get("title").and_then(|t| t.as_str()).unwrap_or(""),
        "desc": view.get("desc").and_then(|d| d.as_str()).unwrap_or(""),
        "pic": view.get("pic").and_then(|p| p.as_str()).unwrap_or(""),
        "duration": view.get("duration").and_then(|d| d.as_i64()).unwrap_or(0),
        "pubdate": view.get("pubdate").and_then(|p| p.as_i64()).unwrap_or(0),
        "width": dimension.get("width").and_then(|w| w.as_i64()).unwrap_or(0),
        "height": dimension.get("height").and_then(|h| h.as_i64()).unwrap_or(0),
        "owner": {
            "mid": mid,
            "name": owner.get("name").and_then(|n| n.as_str()).unwrap_or(""),
            "face": owner.get("face").and_then(|f| f.as_str()).unwrap_or(""),
            "sign": sign,
            "fans": fans,
        },
        "stat": {
            "view": stat.get("view").and_then(|v| v.as_i64()).unwrap_or(0),
            "danmaku": stat.get("danmaku").and_then(|d| d.as_i64()).unwrap_or(0),
            "reply": stat.get("reply").and_then(|r| r.as_i64()).unwrap_or(0),
            "favorite": stat.get("favorite").and_then(|f| f.as_i64()).unwrap_or(0),
            "coin": stat.get("coin").and_then(|c| c.as_i64()).unwrap_or(0),
            "share": stat.get("share").and_then(|s| s.as_i64()).unwrap_or(0),
            "like": stat.get("like").and_then(|l| l.as_i64()).unwrap_or(0),
        },
        "video_url": video_url,
        "video_size": video_size,
        "video_codec": video_codec,
        "video_bitrate": video_bitrate,
    });

    Ok(result.to_string())
}

// ── B站主页 ───────────────────────────────────────────────────────

#[tauri::command]
pub async fn fetch_bilibili_homepage(
    app: tauri::AppHandle,
    mid: String,
    cookies: String,
) -> ParseResult<String> {
    let client = build_http_client()?;

    let (img_key, sub_key) = fetch_wbi_keys(&client, &cookies).await
        .map_err(|e| ParseError::Internal(e))?;

    let card_url = format!("https://api.bilibili.com/x/web-interface/card?mid={}", mid);
    let card = bili_api_get(&client, &card_url, &cookies).await.ok();
    let card_info = card.as_ref().and_then(|c| c.get("card")).cloned()
        .unwrap_or(serde_json::json!({}));
    let fans = card.as_ref()
        .and_then(|c| c.get("follower"))
        .and_then(|f| f.as_i64())
        .unwrap_or(0);

    let mut all_videos: Vec<serde_json::Value> = Vec::new();
    let mut page: i64 = 0;
    let ps = 30;

    loop {
        page += 1;
        let mut params = BTreeMap::new();
        params.insert("mid".into(), mid.clone());
        params.insert("ps".into(), ps.to_string());
        params.insert("pn".into(), page.to_string());
        params.insert("order".into(), "pubdate".into());
        wbi_sign(&mut params, &img_key, &sub_key);

        let query: String = params
            .iter()
            .map(|(k, v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v)))
            .collect::<Vec<_>>()
            .join("&");

        let url = format!("https://api.bilibili.com/x/space/wbi/arc/search?{}", query);
        let data = bili_api_get(&client, &url, &cookies).await
            .map_err(|e| ParseError::Internal(e))?;

        let vlist = data.get("list")
            .and_then(|l| l.get("vlist"))
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        if vlist.is_empty() {
            break;
        }

        for mut v in vlist {
            if let Some(obj) = v.as_object_mut() {
                obj.insert("__card__".into(), card_info.clone());
                obj.insert("__fans__".into(), serde_json::json!(fans));

                let item_bvid = obj.get("bvid").and_then(|b| b.as_str()).unwrap_or("").to_string();
                if !item_bvid.is_empty() {
                    let view_url = format!(
                        "https://api.bilibili.com/x/web-interface/view?bvid={}",
                        urlencoding::encode(&item_bvid)
                    );
                    if let Ok(view_data) = bili_api_get(&client, &view_url, &cookies).await {
                        let cid = view_data.get("cid").and_then(|c| c.as_i64()).unwrap_or(0);
                        let dim = view_data.get("dimension").cloned().unwrap_or(serde_json::json!({}));
                        obj.insert("__width__".into(), dim.get("width").cloned().unwrap_or(serde_json::json!(0)));
                        obj.insert("__height__".into(), dim.get("height").cloned().unwrap_or(serde_json::json!(0)));

                        if let Some(stat) = view_data.get("stat") {
                            obj.insert("__like__".into(), stat.get("like").cloned().unwrap_or(serde_json::json!(0)));
                            obj.insert("__share__".into(), stat.get("share").cloned().unwrap_or(serde_json::json!(0)));
                            obj.insert("__favorite__".into(), stat.get("favorite").cloned().unwrap_or(serde_json::json!(0)));
                            obj.insert("__coin__".into(), stat.get("coin").cloned().unwrap_or(serde_json::json!(0)));
                            obj.insert("__danmaku__".into(), stat.get("danmaku").cloned().unwrap_or(serde_json::json!(0)));
                            obj.insert("__reply__".into(), stat.get("reply").cloned().unwrap_or(serde_json::json!(0)));
                            obj.insert("__view__".into(), stat.get("view").cloned().unwrap_or(serde_json::json!(0)));
                        }

                        if cid > 0 {
                            let (url, size, codec, bitrate) =
                                bili_fetch_playurl(&client, &item_bvid, cid, &cookies).await;
                            obj.insert("__video_url__".into(), serde_json::json!(url));
                            obj.insert("__video_size__".into(), serde_json::json!(size));
                            obj.insert("__video_codec__".into(), serde_json::json!(codec));
                            obj.insert("__video_bitrate__".into(), serde_json::json!(bitrate));
                        }
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                }
            }
            all_videos.push(v);

            if all_videos.len() % 10 == 0 {
                let _ = app.emit("bili-parse-progress", serde_json::json!({
                    "total": all_videos.len(),
                }));
            }
        }

        let total_count = data.get("page")
            .and_then(|p| p.get("count"))
            .and_then(|c| c.as_i64())
            .unwrap_or(0);

        if (page * ps) >= total_count {
            break;
        }

        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    }

    let _ = app.emit("bili-parse-progress", serde_json::json!({
        "total": all_videos.len(),
    }));

    Ok(serde_json::json!(all_videos).to_string())
}

// ── 批量下载 ──────────────────────────────────────────────────────

#[derive(serde::Deserialize)]
pub struct DownloadTask {
    pub url: String,
    pub save_path: String,
    pub task_id: String,
    #[serde(default)]
    pub fallback_urls: Vec<String>,
}

const MAX_DL_RETRIES: u32 = 3;

fn build_download_client() -> ParseResult<reqwest::Client> {
    reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(15))
        .timeout(std::time::Duration::from_secs(300))
        .redirect(reqwest::redirect::Policy::limited(10))
        .gzip(true)
        .brotli(true)
        .deflate(true)
        .build()
        .map_err(|e| ParseError::Internal(format!("下载客户端创建失败: {}", e)))
}

fn derive_referer(url: &str) -> &str {
    if url.contains("bilivideo.com") || url.contains("bilibili.com") {
        "https://www.bilibili.com/"
    } else if url.contains("douyinvod.com") || url.contains("douyin.com") || url.contains("byteicdn.com") || url.contains("byteimg.com") {
        "https://www.douyin.com/"
    } else if url.contains("kuaishou") || url.contains("ksnvse.com") || url.contains("ksyun.com") || url.contains("yximgs.com") {
        "https://www.kuaishou.com/"
    } else {
        ""
    }
}

async fn download_single_file(
    client: &reqwest::Client,
    url: &str,
    save_path: &std::path::Path,
) -> Result<u64, (String, bool)> {
    let referer = derive_referer(url);

    let resp = client
        .get(url)
        .header("User-Agent", UA)
        .header("Referer", referer)
        .header("Accept", "*/*")
        .send()
        .await
        .map_err(|e| (format!("请求失败: {}", e), true))?;

    let status = resp.status();
    if !status.is_success() {
        let code = status.as_u16();
        let retryable = status.is_server_error() || code == 429 || code == 403 || code == 408;
        return Err((format!("HTTP {}", status), retryable));
    }

    let content_len = resp.content_length();
    let tmp_path = {
        let mut p = save_path.as_os_str().to_owned();
        p.push(".downloading");
        std::path::PathBuf::from(p)
    };

    // 确保目标目录存在
    if let Some(parent) = save_path.parent() {
        tokio::fs::create_dir_all(parent).await
            .map_err(|e| (format!("创建目录失败 {:?}: {}", parent, e), false))?;
    }

    let mut file = tokio::fs::File::create(&tmp_path).await
        .map_err(|e| (format!("创建文件失败 {:?}: {}", tmp_path, e), false))?;

    use futures_util::StreamExt;
    use tokio::io::AsyncWriteExt;

    let mut stream = resp.bytes_stream();
    let mut written: u64 = 0;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| (format!("读取数据中断: {}", e), true))?;
        file.write_all(&chunk).await
            .map_err(|e| (format!("写入失败: {}", e), false))?;
        written += chunk.len() as u64;
    }

    file.flush().await.map_err(|e| (format!("刷新失败: {}", e), false))?;
    drop(file);

    if let Some(expected) = content_len {
        if written != expected {
            let _ = tokio::fs::remove_file(&tmp_path).await;
            return Err((
                format!("文件不完整: 预期{}字节, 实际{}字节", expected, written),
                true,
            ));
        }
    }

    if written == 0 {
        let _ = tokio::fs::remove_file(&tmp_path).await;
        return Err(("下载内容为空(0字节)".into(), true));
    }

    tokio::fs::rename(&tmp_path, save_path).await
        .map_err(|e| (format!("保存失败: {} (from {:?} to {:?})", e, tmp_path, save_path), false))?;

    Ok(written)
}

#[tauri::command]
pub async fn batch_download_videos(
    app: tauri::AppHandle,
    tasks: Vec<DownloadTask>,
    concurrent: usize,
) -> ParseResult<String> {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, AtomicU64, Ordering};
    use serde_json::json;

    let total = tasks.len();
    let emit_step = (total / 50).max(1).min(50);
    let completed = Arc::new(AtomicUsize::new(0));
    let failed = Arc::new(AtomicUsize::new(0));
    let skipped = Arc::new(AtomicUsize::new(0));
    let total_bytes = Arc::new(AtomicU64::new(0));
    let sem = Arc::new(tokio::sync::Semaphore::new(concurrent.max(1).min(20)));

    let client = Arc::new(build_download_client()?);

    let mut handles = Vec::with_capacity(total);

    for task in tasks {
        let app = app.clone();
        let client = Arc::clone(&client);
        let sem = Arc::clone(&sem);
        let completed = Arc::clone(&completed);
        let failed = Arc::clone(&failed);
        let skipped = Arc::clone(&skipped);
        let total_bytes = Arc::clone(&total_bytes);

        let handle = tokio::spawn(async move {
            let _permit = sem.acquire().await;
            let task_id = task.task_id.clone();

            let save_path = std::path::PathBuf::from(&task.save_path);
            if let Some(parent) = save_path.parent() {
                let _ = tokio::fs::create_dir_all(parent).await;
            }

            if let Ok(meta) = tokio::fs::metadata(&save_path).await {
                if meta.len() > 0 {
                    let done = completed.fetch_add(1, Ordering::Relaxed) + 1;
                    skipped.fetch_add(1, Ordering::Relaxed);
                    if done % emit_step == 0 || done == total {
                        let _ = app.emit("batch-download-progress", json!({
                            "task_id": task_id, "status": "skipped",
                            "completed": done, "total": total,
                            "bytes": total_bytes.load(Ordering::Relaxed)
                        }));
                    }
                    return;
                }
            }

            // 收集所有可用 URL：主 URL + 备用 URL
            let mut all_urls = vec![task.url.clone()];
            for fb in &task.fallback_urls {
                if !fb.is_empty() && !all_urls.contains(fb) {
                    all_urls.push(fb.clone());
                }
            }

            let mut last_error = String::new();
            for (url_idx, dl_url) in all_urls.iter().enumerate() {
                for attempt in 0..=MAX_DL_RETRIES {
                    if attempt > 0 {
                        let delay_ms = 2000 * 2u64.pow(attempt - 1);
                        tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
                    }

                    match download_single_file(&client, dl_url, &save_path).await {
                        Ok(size) => {
                            let bytes = total_bytes.fetch_add(size, Ordering::Relaxed) + size;
                            let done = completed.fetch_add(1, Ordering::Relaxed) + 1;
                            if done % emit_step == 0 || done == total {
                                let _ = app.emit("batch-download-progress", json!({
                                    "task_id": task_id, "status": "done",
                                    "completed": done, "total": total, "bytes": bytes
                                }));
                            }
                            return;
                        }
                        Err((msg, retryable)) => {
                            last_error = msg.clone();
                            log::warn!("[下载] task={} url#{} 第{}次失败: {} (可重试={})", task_id, url_idx, attempt + 1, msg, retryable);
                            if !retryable { break; }
                        }
                    }
                }
                // 当前 URL 所有重试都失败，尝试下一个备用 URL
                if url_idx + 1 < all_urls.len() {
                    log::info!("[下载] task={} 切换备用URL #{}", task_id, url_idx + 1);
                }
            }

            let done = completed.fetch_add(1, Ordering::Relaxed) + 1;
            failed.fetch_add(1, Ordering::Relaxed);
            if done % emit_step == 0 || done == total {
                let _ = app.emit("batch-download-progress", json!({
                    "task_id": task_id, "status": "error",
                    "completed": done, "total": total, "error": last_error,
                    "bytes": total_bytes.load(Ordering::Relaxed)
                }));
            }
        });
        handles.push(handle);
    }

    for h in handles {
        let _ = h.await;
    }

    let done = completed.load(Ordering::Relaxed);
    let fail = failed.load(Ordering::Relaxed);
    let skip = skipped.load(Ordering::Relaxed);
    let bytes = total_bytes.load(Ordering::Relaxed);
    Ok(json!({
        "total": total,
        "success": done - fail - skip,
        "failed": fail,
        "skipped": skip,
        "bytes": bytes,
    }).to_string())
}
