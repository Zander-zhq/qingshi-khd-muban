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

// ── 快手单视频 (纯 API, GraphQL visionVideoDetail) ────────────────

const KS_VIDEO_DETAIL_QUERY: &str = r#"query visionVideoDetail($photoId: String, $page: String) {
  visionVideoDetail(photoId: $photoId, page: $page) {
    status
    type
    author {
      id
      name
      following
      headerUrl
    }
    photo {
      id
      duration
      caption
      likeCount
      realLikeCount
      coverUrl
      photoUrl
      photoH265Url
      liked
      timestamp
      expTag
      viewCount
      videoRatio
      stereoType
      musicBlocked
      riskTagContent
      riskTagUrl
      manifest {
        mediaType
        businessType
        version
        adaptationSet {
          id
          duration
          representation {
            id
            defaultSelect
            backupUrl
            codecs
            url
            height
            width
            avgBitrate
            maxBitrate
            m3u8Slice
            qualityType
            qualityLabel
            frameRate
            featureP2sp
            hidden
            disableAdaptive
          }
        }
      }
      manifestH265
      videoResource
    }
    tags {
      type
      name
    }
    commentLimit {
      canAddComment
    }
    llsid
    danmakuSwitch
  }
}"#;

#[tauri::command]
pub async fn api_parse_kuaishou_video(
    app: tauri::AppHandle,
    photo_id: String,
    cookies: String,
) -> Result<String, String> {
    eprintln!("[快手单视频] photoId={}", photo_id);

    let _ = app.emit("cdp-parse-progress", serde_json::json!({
        "message": "正在解析快手视频...",
    }));

    let client = build_ks_client()
        .map_err(|e| format!("HTTP客户端创建失败: {}", e))?;

    let query = serde_json::json!({
        "operationName": "visionVideoDetail",
        "variables": { "photoId": photo_id, "page": "detail" },
        "query": KS_VIDEO_DETAIL_QUERY
    });

    let data = ks_graphql_post(&client, &query, &cookies).await?;

    let detail = data.get("visionVideoDetail")
        .ok_or("GraphQL返回无visionVideoDetail字段")?;

    let status = detail.get("status").and_then(|s| s.as_i64()).unwrap_or(0);
    if status != 1 {
        return Err(format!("视频不可用 (status={})", status));
    }

    let author = detail.get("author").cloned().unwrap_or(serde_json::json!({}));
    let mut photo = detail.get("photo").cloned().unwrap_or(serde_json::json!({}));
    let author_id = author.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();

    if let Some(photo_obj) = photo.as_object_mut() {
        if let Some(vr) = photo_obj.remove("videoResource") {
            let inner = if vr.is_string() {
                serde_json::from_str::<serde_json::Value>(vr.as_str().unwrap_or("{}"))
                    .unwrap_or(vr)
            } else {
                vr
            };
            photo_obj.insert(
                "videoResource".to_string(),
                serde_json::json!({ "type": "json", "json": inner }),
            );
        }
    }

    let mut dc = serde_json::Map::new();
    dc.insert(format!("VisionVideoDetailAuthor:{}", author_id), author);
    dc.insert(format!("VisionVideoDetailPhoto:{}", photo_id), photo.clone());

    if !author_id.is_empty() {
        if let Some(profile) = ks_fetch_profile(&client, &author_id, &cookies).await {
            dc.insert("__kuaishou_profile__".to_string(), profile);
        }
    }

    if let Some(cover_url) = photo.get("coverUrl").and_then(|u| u.as_str()) {
        if let Ok(reqwest_client) = build_http_client() {
            if let Some((w, h)) = fetch_jpeg_dimensions(&reqwest_client, cover_url).await {
                dc.insert("__cover_width__".to_string(), serde_json::json!(w));
                dc.insert("__cover_height__".to_string(), serde_json::json!(h));
            }
        }
    }

    let apollo_state = serde_json::json!({ "defaultClient": dc });

    eprintln!("[快手单视频] 解析成功, photoId={}", photo_id);

    Ok(apollo_state.to_string())
}

// ── 快手主页 (纯 API, REST /rest/v/profile/feed) ──────────────────

#[tauri::command]
pub async fn api_parse_kuaishou_homepage(
    app: tauri::AppHandle,
    user_id: String,
    cookies: String,
) -> Result<String, String> {
    let client = build_ks_client()
        .map_err(|e| format!("HTTP客户端创建失败: {}", e))?;

    let mut all_count: usize = 0;
    let mut seen_ids: HashSet<String> = HashSet::new();
    let mut pcursor = String::new();
    let mut page: usize = 0;
    let mut empty_pages: usize = 0;

    eprintln!("[快手主页] 开始解析, user_id={}", user_id);

    let _ = app.emit("cdp-parse-progress", serde_json::json!({
        "message": "正在加载快手主页...",
    }));

    loop {
        page += 1;
        if page > 500 { break; }

        let body = serde_json::json!({
            "user_id": user_id,
            "pcursor": pcursor,
            "page": "profile"
        });

        eprintln!("[快手主页] 第{}页, pcursor={}", page, if pcursor.is_empty() { "(初始)" } else { &pcursor });

        let resp = client
            .post("https://www.kuaishou.com/rest/v/profile/feed")
            .header("User-Agent", UA)
            .header("Referer", format!("https://www.kuaishou.com/profile/{}", user_id))
            .header("Origin", "https://www.kuaishou.com")
            .header("Accept", "application/json, text/plain, */*")
            .header("Accept-Language", "zh-CN,zh;q=0.9,en;q=0.8")
            .header("Content-Type", "application/json")
            .header("Cookie", &cookies)
            .header("sec-ch-ua", r#""Chromium";v="131", "Not_A Brand";v="24""#)
            .header("sec-ch-ua-mobile", "?0")
            .header("sec-ch-ua-platform", r#""Windows""#)
            .header("sec-fetch-dest", "empty")
            .header("sec-fetch-mode", "cors")
            .header("sec-fetch-site", "same-origin")
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                eprintln!("[快手主页] HTTP请求失败: {}", e);
                format!("HTTP请求失败: {}", e)
            })?;

        let text = resp.text().await
            .map_err(|e| format!("读取响应失败: {}", e))?;

        if text.is_empty() {
            empty_pages += 1;
            if empty_pages >= 3 { break; }
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            continue;
        }

        let data: serde_json::Value = serde_json::from_str(&text)
            .map_err(|e| format!("JSON解析失败: {}", e))?;

        let result = data.get("result").and_then(|v| v.as_i64()).unwrap_or(0);
        if result != 1 {
            let err_msg = data.get("error_msg").and_then(|v| v.as_str()).unwrap_or("未知错误");
            return Err(format!("快手返回错误 (result={}): {}", result, err_msg));
        }

        let feeds = data.get("feeds").and_then(|v| v.as_array());
        let mut new_items: Vec<serde_json::Value> = Vec::new();

        if let Some(list) = feeds {
            for item in list {
                let id_opt = item
                    .get("photo").and_then(|p| p.get("id"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
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
            all_count += new_items.len();
            let _ = app.emit("cdp-parse-chunk", serde_json::json!({
                "platform": "kuaishou",
                "type": "homepage",
                "items": &new_items,
            }));
            let _ = app.emit("cdp-parse-progress", serde_json::json!({
                "message": format!("已加载 {} 个作品（第{}页）", all_count, page),
            }));
        }

        let next_cursor = data.get("pcursor")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        if next_cursor.is_empty() || next_cursor == "no_more" { break; }
        pcursor = next_cursor;

        let delay = random_delay_ms(300, 800);
        tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
    }

    let _ = app.emit("cdp-parse-done", serde_json::json!({
        "platform": "kuaishou",
        "type": "homepage",
        "total": all_count,
    }));

    eprintln!("[快手主页] 解析完成, 共{}个作品", all_count);

    Ok(serde_json::json!({"total": all_count}).to_string())
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

// ── 咪咕单视频 (API元信息 + CDP拦截m3u8) ──────────────────────────

#[tauri::command]
pub async fn api_parse_migu_video(
    app: tauri::AppHandle,
    content_id: String,
    cookies: String,
) -> Result<String, String> {
    use crate::cdp_parse::{ChromeSessionState, inject_cookies, navigate_and_intercept};

    eprintln!("[咪咕单视频] contentId={}", content_id);

    let _ = app.emit("cdp-parse-progress", serde_json::json!({
        "message": "正在解析咪咕视频...",
    }));

    let client = build_http_client()
        .map_err(|e| e.to_string())?;

    // 1. 纯 API 获取视频元信息
    let play_url = format!(
        "https://v2-sc.miguvideo.com/program/v3/cont/playing-info/{}",
        content_id
    );
    let play_resp = client.get(&play_url)
        .header("User-Agent", UA)
        .send().await
        .map_err(|e| format!("获取播放信息失败: {}", e))?;
    let play_data: serde_json::Value = play_resp.json().await
        .map_err(|e| format!("解析播放信息失败: {}", e))?;
    let play = play_data.get("body").unwrap_or(&play_data);

    let content_url = format!(
        "https://v3-sc.miguvideo.com/program/v4/cont/content-info/{}/1",
        content_id
    );
    let content_resp = client.get(&content_url)
        .header("User-Agent", UA)
        .send().await
        .map_err(|e| format!("获取内容信息失败: {}", e))?;
    let content_data: serde_json::Value = content_resp.json().await
        .map_err(|e| format!("解析内容信息失败: {}", e))?;
    let content = content_data
        .pointer("/body/data")
        .cloned()
        .unwrap_or(serde_json::json!({}));

    // 获取社交数据（点赞、评论等）
    let social_resp = client.post("https://webapi.miguvideo.com/gateway/private/social/short-video/find-status-and-info")
        .header("User-Agent", UA)
        .header("Content-Type", "application/json")
        .header("Cookie", &cookies)
        .json(&serde_json::json!({
            "requests": [{
                "contentId": content_id,
                "contentType": 1,
                "clientType": 1,
                "type": "4",
                "videoPlatForm": "0005",
                "mId": play.get("assetID").and_then(|v| v.as_str()).unwrap_or("")
            }]
        }))
        .send().await.ok();

    let mut like_count: i64 = 0;
    let mut comment_count: i64 = 0;
    if let Some(resp) = social_resp {
        if let Ok(social) = resp.json::<serde_json::Value>().await {
            if let Some(arr) = social.pointer("/body/responses").and_then(|v| v.as_array()) {
                if let Some(first) = arr.first() {
                    like_count = first.get("likeNum").and_then(|v| v.as_i64())
                        .or_else(|| first.get("likeNum").and_then(|v| v.as_str()).and_then(|s| s.parse().ok()))
                        .unwrap_or(0);
                    comment_count = first.get("commentNum").and_then(|v| v.as_i64())
                        .or_else(|| first.get("commentNum").and_then(|v| v.as_str()).and_then(|s| s.parse().ok()))
                        .unwrap_or(0);
                }
            }
        }
    }

    let _ = app.emit("cdp-parse-progress", serde_json::json!({
        "message": "正在获取播放地址（需要几秒钟）...",
    }));

    // 2. CDP 获取 m3u8 播放地址
    let state = app.state::<ChromeSessionState>();
    let mut session = state.0.lock().await;

    if !session.is_alive() || session.cdp.is_none() {
        crate::cdp_parse::launch_and_connect(&app, &mut session, false).await?;
    }

    let cdp = session.cdp.as_ref().ok_or("Chrome 未启动")?.clone();
    let event_rx = session.event_rx.as_ref().ok_or("事件通道未就绪")?.clone();
    drop(session);

    inject_cookies(&cdp, &cookies, ".miguvideo.com").await?;

    let page_url = format!("https://www.miguvideo.com/p/vertical/{}", content_id);
    let results = navigate_and_intercept(
        &cdp, &event_rx, &page_url,
        &[".m3u8"],
        20,
    ).await?;

    // 找到含 #EXTINF 的实际 m3u8（不是 GSLB 跳转）
    let mut m3u8_url = String::new();
    for res in &results {
        if res.body.contains("#EXTINF") {
            m3u8_url = res.url.clone();
            break;
        }
        if res.body.starts_with("http") {
            // GSLB 返回的是重定向 URL，也记录下来作为备选
            let redirected = res.body.trim().to_string();
            if m3u8_url.is_empty() {
                m3u8_url = redirected;
            }
        }
    }

    if m3u8_url.is_empty() {
        return Err("未能拦截到咪咕视频播放地址".into());
    }

    eprintln!("[咪咕单视频] m3u8_url={}", &m3u8_url[..m3u8_url.len().min(120)]);

    // 3. 构造返回数据
    let result = serde_json::json!({
        "contentId": content_id,
        "play": play,
        "content": content,
        "m3u8_url": m3u8_url,
        "likeCount": like_count,
        "commentCount": comment_count,
    });

    eprintln!("[咪咕单视频] 解析成功, contentId={}", content_id);

    Ok(result.to_string())
}

// ── 央视CCTV单视频 (纯API) ────────────────────────────────────────

#[tauri::command]
pub async fn api_parse_cctv_video(
    _app: tauri::AppHandle,
    page_url: String,
) -> Result<String, String> {
    eprintln!("[央视] url={}", page_url);

    let client = build_http_client().map_err(|e| e.to_string())?;

    // 1. 请求页面 HTML，提取 guid
    let resp = client.get(&page_url)
        .header("User-Agent", UA)
        .send().await
        .map_err(|e| format!("请求央视页面失败: {}", e))?;
    let html = resp.text().await
        .map_err(|e| format!("读取央视页面失败: {}", e))?;

    let guid = {
        let re = regex::Regex::new(r#"guid\s*=\s*["']([a-f0-9]+)["']"#)
            .map_err(|e| format!("正则编译失败: {}", e))?;
        re.captures(&html)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string())
            .ok_or("页面中未找到视频ID (guid)")?
    };

    eprintln!("[央视] guid={}", guid);

    // 2. 调用视频信息 API
    let tsp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let api_url = format!(
        "https://vdn.apps.cntv.cn/api/getHttpVideoInfo.do?pid={}&client=flash&im=0&tsp={}&vn=2049&vc=AF7FF58FF097A48CFF8495C5D6BCE1B1&uid=&wlan=",
        guid, tsp
    );
    let info_resp = client.get(&api_url)
        .header("User-Agent", UA)
        .send().await
        .map_err(|e| format!("获取视频信息失败: {}", e))?;
    let info: serde_json::Value = info_resp.json().await
        .map_err(|e| format!("解析视频信息失败: {}", e))?;

    let title = info.get("title").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let hls_url = info.get("hls_url").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let image = info.get("image").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let duration_str = info.pointer("/video/totalLength").and_then(|v| v.as_str()).unwrap_or("0");
    let duration_secs: f64 = duration_str.parse().unwrap_or(0.0);
    let column = info.get("column").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let publish_time = info.get("f_pgmtime").and_then(|v| v.as_str()).unwrap_or("").to_string();

    if hls_url.is_empty() {
        return Err("未获取到视频播放地址".into());
    }

    // 3. 请求主 m3u8 获取最高画质
    let main_m3u8 = client.get(&hls_url)
        .header("User-Agent", UA)
        .send().await
        .map_err(|e| format!("请求m3u8失败: {}", e))?
        .text().await
        .map_err(|e| format!("读取m3u8失败: {}", e))?;

    let base_url = hls_url.rsplitn(2, '/').nth(1).unwrap_or("");

    let mut best_url = hls_url.clone();
    let mut best_bw: u64 = 0;
    let mut best_res = String::new();

    for line in main_m3u8.lines() {
        if line.starts_with("#EXT-X-STREAM-INF") {
            let bw = line.split("BANDWIDTH=").nth(1)
                .and_then(|s| s.split(',').next())
                .and_then(|s| s.trim().parse::<u64>().ok())
                .unwrap_or(0);
            let res = line.split("RESOLUTION=").nth(1)
                .and_then(|s| s.split(',').next())
                .unwrap_or("").trim().to_string();
            if bw > best_bw {
                best_bw = bw;
                best_res = res;
            }
        } else if best_bw > 0 && !line.starts_with('#') && !line.trim().is_empty() {
            let path = line.trim();
            best_url = if path.starts_with("http") {
                path.to_string()
            } else if path.starts_with('/') {
                if let Some(origin) = hls_url.find("://").and_then(|i| hls_url[i+3..].find('/').map(|j| &hls_url[..i+3+j])) {
                    format!("{}{}", origin, path)
                } else {
                    format!("{}{}", base_url, path)
                }
            } else {
                format!("{}/{}", base_url, path)
            };
            best_bw = 0;
        }
    }

    let (vw, vh) = if best_res.contains('x') {
        let parts: Vec<&str> = best_res.split('x').collect();
        (parts[0].parse::<u64>().unwrap_or(0), parts[1].parse::<u64>().unwrap_or(0))
    } else { (0, 0) };

    let result = serde_json::json!({
        "title": title,
        "guid": guid,
        "m3u8_url": best_url,
        "cover_url": image,
        "duration": duration_secs,
        "column": column,
        "publish_time": publish_time,
        "video_width": vw,
        "video_height": vh,
    });

    eprintln!("[央视] 解析成功, title={}, 最高画质={}", title, best_res);

    Ok(result.to_string())
}

// ── 央视CCTV栏目列表 (纯API) ──────────────────────────────────────

#[tauri::command]
pub async fn api_parse_cctv_column(
    app: tauri::AppHandle,
    page_url: String,
) -> Result<String, String> {
    eprintln!("[央视栏目] url={}", page_url);

    let _ = app.emit("cdp-parse-progress", serde_json::json!({
        "message": "正在加载央视栏目...",
    }));

    let client = build_http_client().map_err(|e| e.to_string())?;

    // 1. 请求栏目页面 HTML，提取 columnId (TOPC...)
    let resp = client.get(&page_url)
        .header("User-Agent", UA)
        .send().await
        .map_err(|e| format!("请求栏目页面失败: {}", e))?;
    let html = resp.text().await
        .map_err(|e| format!("读取栏目页面失败: {}", e))?;

    let column_id = {
        let re = regex::Regex::new(r"TOPC\d+")
            .map_err(|e| format!("正则编译失败: {}", e))?;
        re.find(&html)
            .map(|m| m.as_str().to_string())
            .ok_or("页面中未找到栏目ID (TOPC...)")?
    };

    eprintln!("[央视栏目] columnId={}", column_id);

    // 2. 获取栏目信息
    let col_info_url = format!(
        "https://api.cntv.cn/lanmu/columnInfoByColumnId?id={}&serviceId=tvcctv",
        column_id
    );
    let col_resp = client.get(&col_info_url)
        .header("User-Agent", UA)
        .send().await.ok();
    let mut column_name = String::new();
    if let Some(resp) = col_resp {
        if let Ok(text) = resp.text().await {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                column_name = json.pointer("/data/column_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("").to_string();
            }
        }
    }

    eprintln!("[央视栏目] 栏目名: {}", column_name);

    // 3. 分页获取所有视频
    let mut all_count: usize = 0;
    let mut page: usize = 1;
    let page_size = 20;

    loop {
        if page > 500 { break; }

        let list_url = format!(
            "https://api.cntv.cn/NewVideo/getVideoListByColumn?id={}&n={}&sort=desc&p={}&d=&mode=0&serviceId=tvcctv",
            column_id, page_size, page
        );

        let resp = client.get(&list_url)
            .header("User-Agent", UA)
            .send().await
            .map_err(|e| format!("获取视频列表失败: {}", e))?;
        let text = resp.text().await
            .map_err(|e| format!("读取视频列表失败: {}", e))?;

        // JSONP 格式去包装
        let json_str = if let Some(start) = text.find('{') {
            let end = text.rfind('}').unwrap_or(text.len() - 1);
            &text[start..=end]
        } else {
            &text
        };

        let data: serde_json::Value = serde_json::from_str(json_str)
            .map_err(|e| format!("解析视频列表失败: {}", e))?;

        let total = data.pointer("/data/total").and_then(|v| v.as_i64()).unwrap_or(0) as usize;
        let list = data.pointer("/data/list").and_then(|v| v.as_array());

        if let Some(items) = list {
            if items.is_empty() { break; }

            // 为每个视频获取 m3u8 地址
            let mut enriched: Vec<serde_json::Value> = Vec::new();
            for item in items {
                let guid = item.get("guid").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let mut enriched_item = item.clone();

                if !guid.is_empty() {
                    let tsp = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();
                    let api_url = format!(
                        "https://vdn.apps.cntv.cn/api/getHttpVideoInfo.do?pid={}&client=flash&im=0&tsp={}&vn=2049&vc=AF7FF58FF097A48CFF8495C5D6BCE1B1&uid=&wlan=",
                        guid, tsp
                    );
                    if let Ok(info_resp) = client.get(&api_url).header("User-Agent", UA).send().await {
                        if let Ok(info) = info_resp.json::<serde_json::Value>().await {
                            let hls_url = info.get("hls_url").and_then(|v| v.as_str()).unwrap_or("");
                            if !hls_url.is_empty() {
                                // 获取最高画质 m3u8
                                if let Ok(main_resp) = client.get(hls_url).header("User-Agent", UA).send().await {
                                    if let Ok(main_text) = main_resp.text().await {
                                        let base = hls_url.rsplitn(2, '/').nth(1).unwrap_or("");
                                        let origin = hls_url.find("://")
                                            .and_then(|i| hls_url[i+3..].find('/').map(|j| &hls_url[..i+3+j]))
                                            .unwrap_or("");
                                        let mut best_bw: u64 = 0;
                                        let mut best_m3u8 = hls_url.to_string();
                                        for line in main_text.lines() {
                                            if line.starts_with("#EXT-X-STREAM-INF") {
                                                let bw = line.split("BANDWIDTH=").nth(1)
                                                    .and_then(|s| s.split(',').next())
                                                    .and_then(|s| s.trim().parse::<u64>().ok())
                                                    .unwrap_or(0);
                                                if bw > best_bw { best_bw = bw; }
                                            } else if best_bw > 0 && !line.starts_with('#') && !line.trim().is_empty() {
                                                let path = line.trim();
                                                best_m3u8 = if path.starts_with("http") { path.to_string() }
                                                    else if path.starts_with('/') { format!("{}{}", origin, path) }
                                                    else { format!("{}/{}", base, path) };
                                                best_bw = 0;
                                            }
                                        }
                                        if let Some(obj) = enriched_item.as_object_mut() {
                                            obj.insert("m3u8_url".to_string(), serde_json::json!(best_m3u8));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                if let Some(obj) = enriched_item.as_object_mut() {
                    obj.insert("column_name".to_string(), serde_json::json!(column_name));
                }
                enriched.push(enriched_item);
            }

            all_count += enriched.len();

            let _ = app.emit("cdp-parse-chunk", serde_json::json!({
                "platform": "cctv",
                "type": "homepage",
                "items": &enriched,
            }));
            let _ = app.emit("cdp-parse-progress", serde_json::json!({
                "message": format!("已加载 {}/{} 个节目（第{}页）", all_count, total, page),
            }));
        } else {
            break;
        }

        if all_count >= total { break; }
        page += 1;

        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    }

    if all_count == 0 {
        return Err("该栏目未找到视频数据".into());
    }

    let _ = app.emit("cdp-parse-done", serde_json::json!({
        "platform": "cctv",
        "type": "homepage",
        "total": all_count,
    }));

    eprintln!("[央视栏目] 解析完成, 共{}个节目", all_count);

    Ok(serde_json::json!({"total": all_count}).to_string())
}

// ── 小红书单视频 (纯API, SSR数据提取) ─────────────────────────────

#[tauri::command]
pub async fn api_parse_xiaohongshu_video(
    app: tauri::AppHandle,
    note_url: String,
    cookies: String,
) -> Result<String, String> {
    eprintln!("[小红书] url={}", note_url);

    let _ = app.emit("cdp-parse-progress", serde_json::json!({
        "message": "正在解析小红薯视频...",
    }));

    let client = build_http_client().map_err(|e| e.to_string())?;

    let resp = client.get(&note_url)
        .header("User-Agent", UA)
        .header("Cookie", &cookies)
        .header("Referer", "https://www.xiaohongshu.com/")
        .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8")
        .header("Accept-Language", "zh-CN,zh;q=0.9,en;q=0.8")
        .send()
        .await
        .map_err(|e| format!("请求小红书页面失败: {}", e))?;

    let html = resp.text().await
        .map_err(|e| format!("读取小红书页面失败: {}", e))?;

    if html.contains("请输入验证码") || html.contains("captcha") {
        return Err("小红书触发了验证码，请稍后再试".into());
    }
    if html.contains("/login") && !html.contains("noteDetailMap") {
        return Err("小红书需要登录，请先在「账号登记」中登录小红薯账号".into());
    }

    // 提取 __INITIAL_STATE__ JSON
    let marker = "__INITIAL_STATE__=";
    let start = html.find(marker)
        .ok_or("页面中未找到视频数据 (__INITIAL_STATE__)")?;
    let json_start = start + marker.len();
    let rest = &html[json_start..];

    // 找到 </script> 结束位置
    let script_end = rest.find("</script>")
        .ok_or("未找到 __INITIAL_STATE__ 结束标记")?;
    let json_str = rest[..script_end].trim().trim_end_matches(';');

    // 替换 undefined 为 null
    let json_str = json_str.replace("undefined", "null");

    let state: serde_json::Value = serde_json::from_str(&json_str)
        .map_err(|e| format!("解析小红书数据失败: {}", e))?;

    // 遍历找 noteDetailMap
    let mut note_data: Option<serde_json::Value> = None;
    if let Some(obj) = state.as_object() {
        for (_key, val) in obj {
            if let Some(map) = val.get("noteDetailMap").and_then(|m| m.as_object()) {
                if let Some(first) = map.values().next() {
                    note_data = first.get("note").cloned().or_else(|| Some(first.clone()));
                    break;
                }
            }
        }
    }

    let note = note_data.ok_or("未找到笔记详情数据")?;

    let note_type = note.get("type").and_then(|v| v.as_str()).unwrap_or("");
    let title = note.get("title").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let desc = note.get("desc").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let note_id = note.get("noteId").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let time = note.get("time").and_then(|v| v.as_i64()).unwrap_or(0);

    let user = note.get("user").cloned().unwrap_or(serde_json::json!({}));
    let interact = note.get("interactInfo").cloned().unwrap_or(serde_json::json!({}));

    // 视频流
    let mut video_url = String::new();
    let mut video_size: i64 = 0;
    let mut duration: i64 = 0;
    let mut vw: i64 = 0;
    let mut vh: i64 = 0;

    if note_type == "video" {
        let streams = note.pointer("/video/media/stream/h264")
            .and_then(|v| v.as_array());
        if let Some(list) = streams {
            if let Some(best) = list.first() {
                video_url = best.get("masterUrl").and_then(|v| v.as_str()).unwrap_or("").to_string();
                video_size = best.get("size").and_then(|v| v.as_i64()).unwrap_or(0);
                duration = best.get("duration").and_then(|v| v.as_i64()).unwrap_or(0);
                vw = best.get("width").and_then(|v| v.as_i64()).unwrap_or(0);
                vh = best.get("height").and_then(|v| v.as_i64()).unwrap_or(0);
            }
        }
    }

    // 图片列表（图文笔记）
    let images: Vec<String> = note.get("imageList")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|img| {
            img.get("urlDefault").or_else(|| img.get("url"))
                .and_then(|u| u.as_str())
                .map(|s| s.to_string())
        }).collect())
        .unwrap_or_default();

    let result = serde_json::json!({
        "noteId": note_id,
        "title": title,
        "desc": desc,
        "type": note_type,
        "video_url": video_url,
        "video_size": video_size,
        "duration": duration,
        "video_width": vw,
        "video_height": vh,
        "images": images,
        "user": user,
        "interactInfo": interact,
        "time": time,
    });

    eprintln!("[小红书] 解析成功, noteId={}, type={}", note_id, note_type);

    Ok(result.to_string())
}

// ── 小红书主页 (CDP滚动+API拦截+并行详情补全) ────────────────────

async fn xhs_fetch_note_details(
    client: &reqwest::Client,
    note_items: &[(String, String)], // (noteId, xsec_token)
    cookies: &str,
    author_name: &str,
    author_avatar: &str,
    user_id: &str,
) -> Vec<serde_json::Value> {
    let sem = std::sync::Arc::new(tokio::sync::Semaphore::new(2));
    let client = std::sync::Arc::new(client.clone());
    let cookies = cookies.to_string();

    let mut handles = Vec::new();
    for (_idx, (nid, token)) in note_items.iter().enumerate() {
        let client = client.clone();
        let cookies = cookies.clone();
        let sem = sem.clone();
        let nid = nid.clone();
        let token = token.clone();
        let a_name = author_name.to_string();
        let a_avatar = author_avatar.to_string();
        let uid = user_id.to_string();

        let handle = tokio::spawn(async move {
            let _permit = sem.acquire().await;
            tokio::time::sleep(std::time::Duration::from_millis(300)).await;

            // 带 xsec_token 请求详情页
            let note_url = if token.is_empty() {
                format!("https://www.xiaohongshu.com/explore/{}", nid)
            } else {
                format!("https://www.xiaohongshu.com/explore/{}?xsec_token={}&xsec_source=pc_user", nid, urlencoding::encode(&token))
            };
            eprintln!("[小红书详情] 请求 nid={} hasToken={}", nid, !token.is_empty());

            let mut title = String::new();
            let mut desc = String::new();
            let mut duration: i64 = 0;
            let mut time: i64 = 0;
            let mut video_url = String::new();
            let mut cover = String::new();
            let mut vw: i64 = 0;
            let mut vh: i64 = 0;
            let mut note_type = String::from("video");
            let mut liked = String::new();
            let mut collected = String::new();
            let mut comment = String::new();
            let mut shared = String::new();
            let mut images: Vec<String> = Vec::new();

            if let Ok(resp) = client.get(&note_url)
                .header("User-Agent", UA)
                .header("Cookie", &cookies)
                .header("Referer", "https://www.xiaohongshu.com/")
                .send().await
            {
                let status = resp.status();
                if let Ok(html) = resp.text().await {
                    let has_state = html.contains("__INITIAL_STATE__");
                    let has_note_map = html.contains("noteDetailMap");
                    eprintln!("[小红书详情] nid={} status={} len={} hasState={} hasNoteMap={} url前60={}", nid, status, html.len(), has_state, has_note_map, &note_url[..note_url.len().min(90)]);
                    let marker = "__INITIAL_STATE__=";
                    if let Some(start) = html.find(marker) {
                        let rest = &html[start + marker.len()..];
                        if let Some(end) = rest.find("</script>") {
                            let json_str = rest[..end].trim().trim_end_matches(';').replace("undefined", "null");
                            match serde_json::from_str::<serde_json::Value>(&json_str) {
                                Err(e) => eprintln!("[小红书详情] nid={} JSON解析失败: {}", nid, e),
                                Ok(state) => {
                                    let mut found_map = false;
                                    if let Some(obj) = state.as_object() {
                                        for (k, v) in obj {
                                            if let Some(map) = v.get("noteDetailMap").and_then(|m| m.as_object()) {
                                                found_map = true;
                                                let map_keys: Vec<&String> = map.keys().collect();
                                                let has_null_key = map.contains_key("null");
                                                eprintln!("[小红书详情] nid={} noteDetailMap keys={:?} hasNullKey={} (在state.{}下)", nid, map_keys, has_null_key, k);
                                                // 如果只有 "null" key，说明 xsec_token 缺失或无效
                                                if has_null_key && map.len() == 1 {
                                                    eprintln!("[小红书详情] ⚠️ nid={} noteDetailMap只有null键，xsec_token可能无效!", nid);
                                                }
                                                // 优先用 noteId 精确查找，否则取第一个非 null 的值
                                                let detail_val = map.get(&nid)
                                                    .or_else(|| map.values().find(|v| v.get("note").is_some()))
                                                    .or_else(|| map.values().next());
                                                if let Some(first) = detail_val {
                                                    let note = first.get("note").unwrap_or(first);
                                                    let note_keys: Vec<&String> = note.as_object().map(|o| o.keys().collect()).unwrap_or_default();
                                                    title = note.get("title").and_then(|t| t.as_str()).unwrap_or("").to_string();
                                                    desc = note.get("desc").and_then(|d| d.as_str()).unwrap_or("").to_string();
                                                    time = note.get("time").and_then(|t| t.as_i64()).unwrap_or(0);
                                                    note_type = note.get("type").and_then(|t| t.as_str()).unwrap_or("video").to_string();
                                                    let has_video = note.pointer("/video/media/stream/h264/0").is_some();
                                                    eprintln!("[小红书详情] nid={} title={} time={} type={} hasVideo={} noteKeys={:?}", nid, &title[..title.len().min(30)], time, note_type, has_video, &note_keys[..note_keys.len().min(10)]);

                                                    if let Some(interact) = note.get("interactInfo") {
                                                        liked = interact.get("likedCount").and_then(|l| l.as_str()).unwrap_or("0").to_string();
                                                        collected = interact.get("collectedCount").and_then(|c| c.as_str()).unwrap_or("0").to_string();
                                                        comment = interact.get("commentCount").and_then(|c| c.as_str()).unwrap_or("0").to_string();
                                                        shared = interact.get("shareCount").and_then(|s| s.as_str()).unwrap_or("0").to_string();
                                                    }

                                                    if let Some(h264) = note.pointer("/video/media/stream/h264/0") {
                                                        duration = h264.get("duration").and_then(|d| d.as_i64()).unwrap_or(0);
                                                        video_url = h264.get("masterUrl").and_then(|u| u.as_str()).unwrap_or("").to_string();
                                                        vw = h264.get("width").and_then(|w| w.as_i64()).unwrap_or(0);
                                                        vh = h264.get("height").and_then(|h| h.as_i64()).unwrap_or(0);
                                                    }

                                                    if let Some(img_list) = note.get("imageList").and_then(|i| i.as_array()) {
                                                        for img in img_list {
                                                            if let Some(url) = img.get("urlDefault").or_else(|| img.get("url")).and_then(|u| u.as_str()) {
                                                                images.push(url.to_string());
                                                                if cover.is_empty() { cover = url.to_string(); }
                                                            }
                                                        }
                                                    }
                                                }
                                                break;
                                            }
                                        }
                                    }
                                    if !found_map {
                                        let state_keys: Vec<&String> = state.as_object().map(|o| o.keys().collect()).unwrap_or_default();
                                        eprintln!("[小红书详情] nid={} 未找到noteDetailMap! stateKeys={:?}", nid, state_keys);
                                    }
                                }
                            }
                        }
                    }
                }
            } else {
                eprintln!("[小红书详情] nid={} HTTP请求失败!", nid);
            }

            serde_json::json!({
                "noteId": nid,
                "title": title,
                "desc": desc,
                "type": note_type,
                "duration": duration,
                "time": time,
                "video_url": video_url,
                "cover": cover,
                "images": images,
                "video_width": vw,
                "video_height": vh,
                "interactInfo": {
                    "likedCount": liked,
                    "collectedCount": collected,
                    "commentCount": comment,
                    "shareCount": shared,
                },
                "user": { "nickname": a_name, "avatar": a_avatar, "userId": uid },
                "__author_name__": a_name,
                "__author_avatar__": a_avatar,
                "__user_id__": uid,
            })
        });
        handles.push(handle);
    }

    let mut results = Vec::new();
    for h in handles {
        if let Ok(v) = h.await { results.push(v); }
    }
    results
}

/// 从列表数据（DOM提取或API user_posted响应）构建标准化的笔记对象
/// 不再逐条HTTP请求详情页（会被限流），视频URL在下载时通过CDP解析
fn build_xhs_list_item(
    note: &serde_json::Value,
    author_name: &str,
    author_avatar: &str,
    user_id: &str,
) -> serde_json::Value {
    let note_id = note.get("note_id").or(note.get("noteId"))
        .and_then(|v| v.as_str()).unwrap_or("").to_string();
    let title = note.get("display_title").or(note.get("title"))
        .and_then(|v| v.as_str()).unwrap_or("").to_string();

    // 判断是否视频
    let is_video = note.get("is_video").and_then(|v| v.as_bool()).unwrap_or(false)
        || note.get("type").and_then(|v| v.as_str()).unwrap_or("") == "video";
    let note_type = if is_video { "video" } else { "normal" };

    // 封面：DOM格式为 cover_url 字符串，API格式为 cover 对象
    let cover = note.get("cover_url").and_then(|v| v.as_str())
        .or_else(|| note.get("cover").and_then(|v| v.as_str()))
        .or_else(|| note.pointer("/cover/url_default").and_then(|v| v.as_str()))
        .or_else(|| note.pointer("/cover/url").and_then(|v| v.as_str()))
        .unwrap_or("").to_string();

    // 点赞数：DOM格式为 liked_count，API格式为 interact_info.liked_count
    let liked = note.get("liked_count").and_then(|v| v.as_str())
        .or_else(|| note.pointer("/interact_info/liked_count").and_then(|v| v.as_str()))
        .unwrap_or("0").to_string();

    let token = note.get("xsec_token").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let cover_w = note.pointer("/cover/width").and_then(|v| v.as_i64()).unwrap_or(0);
    let cover_h = note.pointer("/cover/height").and_then(|v| v.as_i64()).unwrap_or(0);

    serde_json::json!({
        "noteId": note_id,
        "title": title,
        "type": note_type,
        "cover": cover,
        "video_url": "",
        "duration": 0,
        "time": 0,
        "video_width": cover_w,
        "video_height": cover_h,
        "xsec_token": token,
        "interactInfo": { "likedCount": liked },
        "__author_name__": author_name,
        "__author_avatar__": author_avatar,
        "__user_id__": user_id,
    })
}

#[tauri::command]
pub async fn api_parse_xiaohongshu_homepage(
    app: tauri::AppHandle,
    user_id: String,
    cookies: String,
) -> Result<String, String> {
    use crate::cdp_parse::{ChromeSessionState, inject_cookies};

    eprintln!("[小红书主页] userId={}", user_id);

    let _ = app.emit("cdp-parse-progress", serde_json::json!({
        "message": "正在加载小红薯主页...",
    }));

    let state = app.state::<ChromeSessionState>();
    let mut session = state.0.lock().await;
    if !session.is_alive() || session.cdp.is_none() {
        crate::cdp_parse::launch_and_connect(&app, &mut session, false).await?;
    }
    let cdp = session.cdp.as_ref().ok_or("Chrome 未启动")?.clone();
    let event_rx = session.event_rx.as_ref().ok_or("事件通道未就绪")?.clone();
    drop(session);

    inject_cookies(&cdp, &cookies, ".xiaohongshu.com").await?;

    let page_url = format!("https://www.xiaohongshu.com/user/profile/{}", user_id);
    cdp.navigate_and_wait(&page_url, 15).await?;
    tokio::time::sleep(std::time::Duration::from_millis(2000)).await;

    // 提取用户信息
    let user_js = r#"(function(){
        var n = document.querySelector('.user-name'); var a = document.querySelector('.avatar-wrapper img');
        return JSON.stringify({name: n ? n.textContent.trim() : '', avatar: a ? a.src : ''});
    })()"#;
    let user_raw = cdp.eval(user_js).await.unwrap_or_default();
    let user_info: serde_json::Value = serde_json::from_str(&user_raw).unwrap_or(serde_json::json!({}));
    let author_name = user_info.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let author_avatar = user_info.get("avatar").and_then(|v| v.as_str()).unwrap_or("").to_string();

    // ── Phase 1: 滚动虚拟列表收集全部初始笔记 ────────────
    // 小红书用了虚拟滚动，DOM只渲染视口内约12条，但SSR实际加载了约30条
    // 通过小幅滚动让虚拟列表把所有初始item渲染一遍，逐批收集
    let dom_js = r#"(function(){
        var items = [];
        var sections = document.querySelectorAll('section.note-item');
        for (var i = 0; i < sections.length; i++) {
            var s = sections[i];
            var links = s.querySelectorAll('a');
            var noteId = '', token = '';
            for (var j = 0; j < links.length; j++) {
                var href = links[j].href || '';
                var m = href.match(/\/([a-f0-9]{24})/);
                if (m && !noteId) noteId = m[1];
                var t = href.match(/xsec_token=([^&]+)/);
                if (t && !token) token = decodeURIComponent(t[1]);
            }
            if (!noteId) continue;
            var title = '';
            var titleEl = s.querySelector('.title span, .footer .title');
            if (titleEl) title = titleEl.textContent.trim();
            var cover = '';
            var coverEl = s.querySelector('a.cover img, img');
            if (coverEl) cover = coverEl.src || '';
            var likes = '0';
            var likesEl = s.querySelector('.like-wrapper .count, .count');
            if (likesEl) likes = likesEl.textContent.trim();
            var isVideo = !!s.querySelector('.play-icon, svg.play-icon, .video-icon, [class*="play"]');
            items.push({note_id: noteId, xsec_token: token, display_title: title, cover_url: cover, liked_count: likes, is_video: isVideo});
        }
        return JSON.stringify(items);
    })()"#;

    let mut seen_ids: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut initial_items: Vec<serde_json::Value> = Vec::new();
    let mut all_count: usize = 0;

    // 首次提取
    let raw = cdp.eval(dom_js).await.unwrap_or_default();
    let batch: Vec<serde_json::Value> = serde_json::from_str(&raw).unwrap_or_default();
    for item in &batch {
        let nid = item.get("note_id").and_then(|v| v.as_str()).unwrap_or("");
        if !nid.is_empty() && seen_ids.insert(nid.to_string()) {
            initial_items.push(item.clone());
        }
    }
    eprintln!("[小红书主页] DOM首次提取: {}条", initial_items.len());

    // 小幅滚动收集更多初始item（虚拟滚动会渲染不同批次）
    let mut no_new_scroll = 0;
    for scroll_round in 0..20 {
        let _ = cdp.send("Input.dispatchMouseEvent", serde_json::json!({
            "type": "mouseWheel", "x": 400, "y": 400, "deltaX": 0, "deltaY": 500
        })).await;
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        let raw = cdp.eval(dom_js).await.unwrap_or_default();
        let batch: Vec<serde_json::Value> = serde_json::from_str(&raw).unwrap_or_default();
        let prev = seen_ids.len();
        for item in &batch {
            let nid = item.get("note_id").and_then(|v| v.as_str()).unwrap_or("");
            if !nid.is_empty() && seen_ids.insert(nid.to_string()) {
                initial_items.push(item.clone());
            }
        }
        if seen_ids.len() == prev {
            no_new_scroll += 1;
            if no_new_scroll >= 3 { break; }
        } else {
            no_new_scroll = 0;
            eprintln!("[小红书主页] 滚动第{}次: 累计{}条", scroll_round + 1, seen_ids.len());
        }
    }
    eprintln!("[小红书主页] 初始收集完成: {}条 (作者: {})", initial_items.len(), author_name);

    // 将初始item构建为标准格式并推送
    if !initial_items.is_empty() {
        let items_json: Vec<serde_json::Value> = initial_items.iter().map(|item| {
            build_xhs_list_item(item, &author_name, &author_avatar, &user_id)
        }).collect();
        all_count += items_json.len();

        let _ = app.emit("cdp-parse-chunk", serde_json::json!({
            "platform": "xiaohongshu", "type": "homepage", "items": &items_json,
        }));
        let _ = app.emit("cdp-parse-progress", serde_json::json!({
            "message": format!("已加载 {} 个作品（首屏）", all_count),
        }));
    }

    // ── Phase 2: 滚动触发 user_posted API，直接从API响应提取完整笔记 ────
    let mut no_new_rounds = 0;
    let mut api_page = 0;

    for _round in 0..200 {
        for _ in 0..5 {
            let _ = cdp.send("Input.dispatchMouseEvent", serde_json::json!({
                "type": "mouseWheel", "x": 400, "y": 400, "deltaX": 0, "deltaY": 800
            })).await;
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
        tokio::time::sleep(std::time::Duration::from_millis(800)).await;

        // 从 user_posted API 响应中提取完整笔记对象
        let mut new_notes: Vec<serde_json::Value> = Vec::new();
        {
            let mut rx = event_rx.lock().await;
            let mut pending: Vec<String> = Vec::new();
            loop {
                match tokio::time::timeout(std::time::Duration::from_millis(300), rx.recv()).await {
                    Ok(Some(ev)) => {
                        let method = ev.get("method").and_then(|m| m.as_str()).unwrap_or("");
                        let params = ev.get("params").cloned().unwrap_or(serde_json::json!({}));
                        if method == "Network.responseReceived" {
                            let url = params.pointer("/response/url").and_then(|u| u.as_str()).unwrap_or("");
                            let req_id = params.get("requestId").and_then(|r| r.as_str()).unwrap_or("").to_string();
                            if url.contains("user_posted") { pending.push(req_id); }
                        } else if method == "Network.loadingFinished" {
                            let req_id = params.get("requestId").and_then(|r| r.as_str()).unwrap_or("").to_string();
                            if pending.contains(&req_id) {
                                if let Ok(resp) = cdp.send("Network.getResponseBody", serde_json::json!({"requestId": req_id})).await {
                                    let body = resp.pointer("/result/body").and_then(|b| b.as_str()).unwrap_or("");
                                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(body) {
                                        if let Some(notes) = json.pointer("/data/notes").and_then(|v| v.as_array()) {
                                            eprintln!("[小红书主页] API user_posted 返回 {} 条笔记", notes.len());
                                            for note in notes {
                                                let nid = note.get("note_id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                                if !nid.is_empty() && seen_ids.insert(nid) {
                                                    new_notes.push(note.clone());
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    _ => break,
                }
            }
        }

        if !new_notes.is_empty() {
            no_new_rounds = 0;
            api_page += 1;

            let items_json: Vec<serde_json::Value> = new_notes.iter().map(|note| {
                build_xhs_list_item(note, &author_name, &author_avatar, &user_id)
            }).collect();
            eprintln!("[小红书主页] 第{}页API: 新增{}条", api_page, items_json.len());
            all_count += items_json.len();

            let _ = app.emit("cdp-parse-chunk", serde_json::json!({
                "platform": "xiaohongshu", "type": "homepage", "items": &items_json,
            }));
            let _ = app.emit("cdp-parse-progress", serde_json::json!({
                "message": format!("已加载 {} 个作品（第{}页）", all_count, api_page),
            }));
        } else {
            no_new_rounds += 1;
            if no_new_rounds >= 5 { break; }
        }
    }

    if all_count == 0 {
        return Err("该用户主页未找到作品数据".into());
    }

    let _ = app.emit("cdp-parse-done", serde_json::json!({
        "platform": "xiaohongshu", "type": "homepage", "total": all_count,
    }));

    eprintln!("[小红书主页] 解析完成, 共{}个作品", all_count);

    Ok(serde_json::json!({"total": all_count}).to_string())
}

// ── 央视频 yangshipin 单视频 (CDP获取mp4直链) ─────────────────────

#[tauri::command]
pub async fn api_parse_yangshipin_video(
    app: tauri::AppHandle,
    vid: String,
) -> Result<String, String> {
    use crate::cdp_parse::ChromeSessionState;

    eprintln!("[央视频] vid={}", vid);

    let _ = app.emit("cdp-parse-progress", serde_json::json!({
        "message": "正在解析央视频视频...",
    }));

    // 1. CDP 打开页面获取 mp4 直链
    let state = app.state::<ChromeSessionState>();
    let mut session = state.0.lock().await;

    if !session.is_alive() || session.cdp.is_none() {
        crate::cdp_parse::launch_and_connect(&app, &mut session, false).await?;
    }

    let cdp = session.cdp.as_ref().ok_or("Chrome 未启动")?.clone();
    drop(session);

    let page_url = format!("https://yangshipin.cn/video/home?vid={}", vid);
    cdp.navigate_and_wait(&page_url, 15).await?;

    // 等待视频加载
    tokio::time::sleep(std::time::Duration::from_millis(5000)).await;

    // 从 <video> 标签获取 mp4 URL 和页面信息
    let video_js = r#"(function(){
        try {
            var v = document.querySelector('video');
            var mp4 = v ? (v.src || v.currentSrc || '') : '';
            var title = document.title || '';
            // 尝试获取更多信息
            var duration = v ? v.duration : 0;
            var poster = v ? v.poster : '';
            return JSON.stringify({
                mp4_url: mp4,
                title: title,
                duration: duration || 0,
                poster: poster
            });
        } catch(e) {
            return JSON.stringify({error: e.message});
        }
    })()"#;

    let raw = cdp.eval(video_js).await?;
    let info: serde_json::Value = serde_json::from_str(&raw)
        .map_err(|_| "解析视频信息失败")?;

    let mp4_url = info.get("mp4_url").and_then(|v| v.as_str()).unwrap_or("").to_string();
    if mp4_url.is_empty() || !mp4_url.contains(".mp4") {
        return Err("未能获取到央视频视频播放地址".into());
    }

    let title = info.get("title").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let title = title.replace(" - 央视频", "").replace(" - 有品质的视频社交媒体", "").trim().to_string();
    let duration = info.get("duration").and_then(|v| v.as_f64()).unwrap_or(0.0);

    let result = serde_json::json!({
        "vid": vid,
        "title": title,
        "mp4_url": mp4_url,
        "duration": duration,
    });

    eprintln!("[央视频] 解析成功, title={}", title);

    Ok(result.to_string())
}

// ── 咪咕主页 (纯API，不需要CDP) ───────────────────────────────────

#[tauri::command]
pub async fn api_parse_migu_homepage(
    app: tauri::AppHandle,
    author_id: String,
    _cookies: String,
) -> Result<String, String> {
    eprintln!("[咪咕主页] authorId={}", author_id);

    let _ = app.emit("cdp-parse-progress", serde_json::json!({
        "message": "正在加载咪咕主页...",
    }));

    let client = build_http_client()
        .map_err(|e| e.to_string())?;

    // 1. 纯 API 获取用户信息
    let user_resp = client.get(&format!(
        "https://v4-sc.miguvideo.com/user/staticcache/queryUserInfo/{}", author_id
    )).header("User-Agent", UA).send().await
        .map_err(|e| format!("获取用户信息失败: {}", e))?;
    let user_data: serde_json::Value = user_resp.json().await
        .map_err(|e| format!("解析用户信息失败: {}", e))?;
    let user_info = user_data.pointer("/body/data").cloned().unwrap_or(serde_json::json!({}));
    let author_name = user_info.get("sname").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let author_avatar = user_info.get("picture").and_then(|v| v.as_str()).unwrap_or("").to_string();

    eprintln!("[咪咕主页] 作者: {}", author_name);

    // 2. 纯 API 分页获取所有视频列表，逐页推送
    let mut all_count: usize = 0;
    let mut page: usize = 1;

    loop {
        if page > 500 { break; }
        let list_url = format!(
            "https://program-sc.miguvideo.com/private/social/staticcache/getVideoDynamicList/{}/{}/20",
            author_id, page
        );
        let resp = client.get(&list_url)
            .header("User-Agent", UA)
            .send().await
            .map_err(|e| format!("获取视频列表失败: {}", e))?;
        let data: serde_json::Value = resp.json().await
            .map_err(|e| format!("解析视频列表失败: {}", e))?;

        let body = data.get("body").unwrap_or(&data);
        let items = body.get("data").and_then(|v| v.as_array());

        if let Some(list) = items {
            if list.is_empty() { break; }

            let enriched: Vec<serde_json::Value> = list.iter().map(|item| {
                let mut v = item.clone();
                if let Some(obj) = v.as_object_mut() {
                    obj.insert("author_name".to_string(), serde_json::json!(author_name));
                    obj.insert("author_avatar".to_string(), serde_json::json!(author_avatar));
                    obj.insert("author_id".to_string(), serde_json::json!(author_id));
                }
                v
            }).collect();

            all_count += enriched.len();

            let _ = app.emit("cdp-parse-chunk", serde_json::json!({
                "platform": "migu",
                "type": "homepage",
                "items": &enriched,
            }));
            let _ = app.emit("cdp-parse-progress", serde_json::json!({
                "message": format!("已加载 {} 个作品（第{}页）", all_count, page),
            }));
        } else {
            break;
        }

        let has_more = body.get("hasMoreData").and_then(|v| v.as_bool()).unwrap_or(false);
        if !has_more { break; }

        let next_page = body.get("pageNum").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
        page = if next_page > page { next_page } else { page + 1 };

        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }

    if all_count == 0 {
        return Err("该用户主页未找到视频数据".into());
    }

    let _ = app.emit("cdp-parse-done", serde_json::json!({
        "platform": "migu",
        "type": "homepage",
        "total": all_count,
    }));

    eprintln!("[咪咕主页] 解析完成, 共{}个作品", all_count);

    Ok(serde_json::json!({"total": all_count}).to_string())
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
) -> (String, Vec<String>, i64, String, i64) {
    let url = format!(
        "https://api.bilibili.com/x/player/playurl?bvid={}&cid={}&qn=116&fnval=0&fourk=1",
        urlencoding::encode(bvid), cid
    );
    let play = bili_api_get(client, &url, cookies).await.ok();

    let mut video_url = String::new();
    let mut backup_urls: Vec<String> = Vec::new();
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
                if let Some(bu) = first.get("backup_url").and_then(|b| b.as_array()) {
                    for u in bu {
                        if let Some(s) = u.as_str() {
                            if !s.is_empty() { backup_urls.push(s.to_string()); }
                        }
                    }
                }
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

    (video_url, backup_urls, video_size, video_codec, video_bitrate)
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

    let (video_url, backup_urls, video_size, video_codec, video_bitrate) =
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
        "video_url_fallbacks": backup_urls,
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

        let vlist_len = vlist.len();
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
                            let (url, fallbacks, size, codec, bitrate) =
                                bili_fetch_playurl(&client, &item_bvid, cid, &cookies).await;
                            obj.insert("__video_url__".into(), serde_json::json!(url));
                            obj.insert("__video_url_fallbacks__".into(), serde_json::json!(fallbacks));
                            obj.insert("__video_size__".into(), serde_json::json!(size));
                            obj.insert("__video_codec__".into(), serde_json::json!(codec));
                            obj.insert("__video_bitrate__".into(), serde_json::json!(bitrate));
                        }
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                }
            }
            all_videos.push(v);
        }

        if !all_videos.is_empty() {
            let page_start = all_videos.len() - vlist_len;
            let page_items: Vec<_> = all_videos[page_start..].to_vec();
            let _ = app.emit("cdp-parse-chunk", serde_json::json!({
                "platform": "bilibili",
                "type": "homepage",
                "items": &page_items,
            }));
            let _ = app.emit("cdp-parse-progress", serde_json::json!({
                "message": format!("已加载 {} 个作品（第{}页）", all_videos.len(), page),
            }));
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

    let _ = app.emit("cdp-parse-done", serde_json::json!({
        "platform": "bilibili",
        "type": "homepage",
        "total": all_videos.len(),
    }));

    Ok(serde_json::json!({"total": all_videos.len()}).to_string())
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
    if url.contains("bilivideo.com") || url.contains("bilibili.com")
        || url.contains("hdslb.com") || url.contains("akamaized.net")
        || url.contains("upgcxcode") {
        "https://www.bilibili.com/"
    } else if url.contains("douyinvod.com") || url.contains("douyin.com") || url.contains("byteicdn.com") || url.contains("byteimg.com") {
        "https://www.douyin.com/"
    } else if url.contains("kuaishou") || url.contains("ksnvse.com") || url.contains("ksyun.com") || url.contains("yximgs.com") || url.contains("ndcimgs.com") || url.contains("oskwai.com") {
        "https://www.kuaishou.com/"
    } else if url.contains("miguvideo.com") || url.contains("cmvideo.cn") || url.contains("migu") {
        "https://www.miguvideo.com/"
    } else if url.contains("cntv.cn") || url.contains("cctv.com") || url.contains("lxdns.com") {
        "https://tv.cctv.com/"
    } else if url.contains("yangshipin.cn") || url.contains("ysp.cctv.cn") {
        "https://yangshipin.cn/"
    } else if url.contains("xiaohongshu.com") || url.contains("xhscdn.com") || url.contains("xhslink.com") {
        "https://www.xiaohongshu.com/"
    } else {
        ""
    }
}

fn find_ffmpeg(app: Option<&tauri::AppHandle>) -> Option<std::path::PathBuf> {
    // 1. Tauri resource_dir (打包后资源的正确路径)
    if let Some(app) = app {
        use tauri::Manager;
        if let Ok(res_dir) = app.path().resource_dir() {
            let p = res_dir.join("resources").join("ffmpeg.exe");
            if p.exists() { return Some(p); }
            let p = res_dir.join("resources").join("ffmpeg");
            if p.exists() { return Some(p); }
        }
    }
    // 2. exe 同级目录
    if let Some(exe_dir) = std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.to_path_buf())) {
        for candidate in &[
            exe_dir.join("resources").join("ffmpeg.exe"),
            exe_dir.join("ffmpeg.exe"),
            exe_dir.join("resources").join("ffmpeg"),
            exe_dir.join("ffmpeg"),
        ] {
            if candidate.exists() { return Some(candidate.clone()); }
        }
    }
    // 3. 系统 PATH
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        if let Ok(output) = std::process::Command::new("ffmpeg")
            .arg("-version")
            .creation_flags(0x08000000)
            .output()
        {
            if output.status.success() {
                return Some(std::path::PathBuf::from("ffmpeg"));
            }
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        if let Ok(output) = std::process::Command::new("ffmpeg").arg("-version").output() {
            if output.status.success() {
                return Some(std::path::PathBuf::from("ffmpeg"));
            }
        }
    }
    None
}

async fn download_m3u8_as_mp4(
    client: &reqwest::Client,
    m3u8_url: &str,
    save_path: &std::path::Path,
    app: Option<&tauri::AppHandle>,
    task_id: Option<&str>,
) -> Result<u64, (String, bool)> {
    let ffmpeg = find_ffmpeg(app)
        .ok_or_else(|| ("未找到 ffmpeg，请安装 ffmpeg 或将 ffmpeg.exe 放到应用目录".to_string(), false))?;

    // 获取 m3u8 内容
    let m3u8_text = client.get(m3u8_url)
        .header("User-Agent", UA)
        .send().await
        .map_err(|e| (format!("请求m3u8失败: {}", e), true))?
        .text().await
        .map_err(|e| (format!("读取m3u8失败: {}", e), true))?;

    // 如果 m3u8 内容是重定向 URL（GSLB），再请求一次
    let (final_m3u8_text, base_url) = if m3u8_text.trim().starts_with("http") {
        let real_url = m3u8_text.trim();
        let real_text = client.get(real_url)
            .header("User-Agent", UA)
            .send().await
            .map_err(|e| (format!("请求实际m3u8失败: {}", e), true))?
            .text().await
            .map_err(|e| (format!("读取实际m3u8失败: {}", e), true))?;
        let base = real_url.rsplitn(2, '/').nth(1).unwrap_or("").to_string();
        (real_text, base)
    } else {
        let base = m3u8_url.rsplitn(2, '/').nth(1).unwrap_or("").to_string();
        (m3u8_text, base)
    };

    // 解析 TS 分片
    let origin = {
        let m3u8_for_origin = if base_url.starts_with("http") { base_url.as_str() } else { m3u8_url };
        m3u8_for_origin.find("://")
            .and_then(|i| m3u8_for_origin[i+3..].find('/').map(|j| m3u8_for_origin[..i+3+j].to_string()))
            .unwrap_or_default()
    };

    let ts_urls: Vec<String> = final_m3u8_text
        .lines()
        .filter(|line| !line.starts_with('#') && !line.trim().is_empty())
        .map(|line| {
            let l = line.trim();
            if l.starts_with("http") { l.to_string() }
            else if l.starts_with('/') { format!("{}{}", origin, l) }
            else { format!("{}/{}", base_url, l) }
        })
        .collect();

    if ts_urls.is_empty() {
        return Err(("m3u8中未找到TS分片".to_string(), false));
    }

    eprintln!("[m3u8下载] 共{}个TS分片", ts_urls.len());

    if let (Some(a), Some(tid)) = (app, task_id) {
        let _ = a.emit("download-file-progress", serde_json::json!({
            "task_id": tid, "downloaded": 0, "total": ts_urls.len() as u64,
        }));
    }

    // 创建临时目录
    let tmp_dir = save_path.parent().unwrap_or(std::path::Path::new("."))
        .join(format!(".migu_tmp_{}", std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis()));
    tokio::fs::create_dir_all(&tmp_dir).await
        .map_err(|e| (format!("创建临时目录失败: {}", e), false))?;

    // 下载所有 TS 分片
    let mut ts_files: Vec<std::path::PathBuf> = Vec::new();
    let mut total_size: u64 = 0;

    for (i, ts_url) in ts_urls.iter().enumerate() {
        let ts_path = tmp_dir.join(format!("seg_{:04}.ts", i));
        let resp = client.get(ts_url)
            .header("User-Agent", UA)
            .send().await
            .map_err(|e| (format!("下载TS分片{}失败: {}", i, e), true))?;

        if !resp.status().is_success() {
            let _ = tokio::fs::remove_dir_all(&tmp_dir).await;
            return Err((format!("TS分片{}返回HTTP {}", i, resp.status()), true));
        }

        let bytes = resp.bytes().await
            .map_err(|e| (format!("读取TS分片{}失败: {}", i, e), true))?;
        total_size += bytes.len() as u64;
        tokio::fs::write(&ts_path, &bytes).await
            .map_err(|e| (format!("写入TS分片{}失败: {}", i, e), false))?;
        ts_files.push(ts_path);

        if let (Some(a), Some(tid)) = (app, task_id) {
            let _ = a.emit("download-file-progress", serde_json::json!({
                "task_id": tid,
                "downloaded": (i + 1) as u64,
                "total": ts_urls.len() as u64,
            }));
        }
    }

    // 生成 ffmpeg concat 文件列表
    let list_path = tmp_dir.join("filelist.txt");
    let list_content: String = ts_files.iter()
        .map(|p| format!("file '{}'", p.to_string_lossy().replace('\\', "/")))
        .collect::<Vec<_>>()
        .join("\n");
    tokio::fs::write(&list_path, &list_content).await
        .map_err(|e| (format!("写入合并列表失败: {}", e), false))?;

    // 用 ffmpeg 合并为 mp4
    if let Some(parent) = save_path.parent() {
        let _ = tokio::fs::create_dir_all(parent).await;
    }

    let ffmpeg_clone = ffmpeg.clone();
    let list_str = list_path.to_string_lossy().to_string();
    let save_str = save_path.to_string_lossy().to_string();
    let output = tokio::task::spawn_blocking(move || {
        std::process::Command::new(&ffmpeg_clone)
            .arg("-y")
            .arg("-f").arg("concat")
            .arg("-safe").arg("0")
            .arg("-i").arg(&list_str)
            .arg("-c").arg("copy")
            .arg(&save_str)
            .output()
    })
    .await
    .map_err(|e| (format!("ffmpeg任务失败: {}", e), false))?
    .map_err(|e| (format!("执行ffmpeg失败: {}", e), false))?;

    // 清理临时目录
    let _ = tokio::fs::remove_dir_all(&tmp_dir).await;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err((format!("ffmpeg合并失败: {}", stderr.chars().take(200).collect::<String>()), false));
    }

    // 获取最终文件大小
    let final_size = tokio::fs::metadata(save_path).await
        .map(|m| m.len())
        .unwrap_or(total_size);

    eprintln!("[m3u8下载] 合并完成, 大小={}KB", final_size / 1024);

    Ok(final_size)
}

async fn resolve_migu_m3u8(app: &tauri::AppHandle, content_id: &str) -> Result<String, String> {
    use crate::cdp_parse::{ChromeSessionState, inject_cookies, navigate_and_intercept};

    let state = app.state::<ChromeSessionState>();
    let mut session = state.0.lock().await;

    if !session.is_alive() || session.cdp.is_none() {
        crate::cdp_parse::launch_and_connect(app, &mut session, false).await?;
    }

    let cdp = session.cdp.as_ref().ok_or("Chrome 未启动")?.clone();
    let event_rx = session.event_rx.as_ref().ok_or("事件通道未就绪")?.clone();
    drop(session);

    // 注入 Cookie（从数据库获取）
    let migu_cookies: Option<String> = app.try_state::<crate::database::DbState>().and_then(|db| {
        let conn = db.0.lock().ok()?;
        let result = conn.prepare("SELECT cookies FROM platform_accounts WHERE platform='migu' AND status='active' LIMIT 1")
            .ok()?.query_row([], |row| row.get::<_, String>(0)).ok();
        result
    });
    if let Some(cookies) = migu_cookies {
        let _ = inject_cookies(&cdp, &cookies, ".miguvideo.com").await;
    }

    let page_url = format!("https://www.miguvideo.com/p/vertical/{}", content_id);
    let results = navigate_and_intercept(
        &cdp, &event_rx, &page_url, &[".m3u8"], 20,
    ).await?;

    for res in &results {
        if res.body.contains("#EXTINF") { return Ok(res.url.clone()); }
        if res.body.starts_with("http") { return Ok(res.body.trim().to_string()); }
    }

    Err("未能拦截到m3u8地址".into())
}

async fn download_single_file(
    client: &reqwest::Client,
    url: &str,
    save_path: &std::path::Path,
    app: Option<&tauri::AppHandle>,
    task_id: Option<&str>,
) -> Result<u64, (String, bool)> {
    // 咪咕视频：按需通过 CDP 获取 m3u8 播放地址
    if url.starts_with("migu://resolve/") {
        let content_id = &url["migu://resolve/".len()..];
        let a = app.ok_or_else(|| ("咪咕视频需要app句柄".to_string(), false))?;

        let m3u8_url = resolve_migu_m3u8(a, content_id).await
            .map_err(|e| (format!("获取咪咕播放地址失败: {}", e), true))?;

        if m3u8_url.is_empty() {
            return Err(("未能获取到咪咕视频播放地址".to_string(), false));
        }
        return download_m3u8_as_mp4(client, &m3u8_url, save_path, app, task_id).await;
    }

    // 小红书：按需通过 CDP 获取 mp4 地址（HTTP方式会被限流）
    if url.starts_with("xhs://resolve/") {
        let rest = &url["xhs://resolve/".len()..];
        let (note_id, xsec_token) = if let Some(q) = rest.find('?') {
            let nid = &rest[..q];
            let params = &rest[q + 1..];
            let token = params.split('&')
                .find(|p| p.starts_with("xsec_token="))
                .map(|p| urlencoding::decode(&p["xsec_token=".len()..]).unwrap_or_default().to_string())
                .unwrap_or_default();
            (nid.to_string(), token)
        } else {
            (rest.to_string(), String::new())
        };
        let a = app.ok_or_else(|| ("小红书需要app句柄".to_string(), false))?;

        // 用 CDP 导航到笔记页面提取视频URL（比HTTP更可靠，不会被限流）
        let state = a.state::<crate::cdp_parse::ChromeSessionState>();
        let mut session = state.0.lock().await;
        if !session.is_alive() || session.cdp.is_none() {
            crate::cdp_parse::launch_and_connect(&a, &mut session, false).await
                .map_err(|e| (format!("启动Chrome失败: {}", e), true))?;
        }
        let cdp = session.cdp.as_ref().ok_or_else(|| ("Chrome未连接".to_string(), true))?.clone();
        drop(session);

        let xhs_cookies: Option<String> = a.try_state::<crate::database::DbState>().and_then(|db| {
            let conn = db.0.lock().ok()?;
            let result = conn.prepare("SELECT cookies FROM platform_accounts WHERE platform='xiaohongshu' AND status='active' LIMIT 1")
                .ok()?.query_row([], |row| row.get::<_, String>(0)).ok();
            result
        });
        if let Some(ck) = &xhs_cookies {
            let _ = crate::cdp_parse::inject_cookies(&cdp, ck, ".xiaohongshu.com").await;
        }

        let note_url = if xsec_token.is_empty() {
            format!("https://www.xiaohongshu.com/explore/{}", note_id)
        } else {
            format!("https://www.xiaohongshu.com/explore/{}?xsec_token={}&xsec_source=pc_user", note_id, urlencoding::encode(&xsec_token))
        };
        eprintln!("[小红书下载] CDP导航解析 nid={}", note_id);
        cdp.navigate_and_wait(&note_url, 10).await
            .map_err(|e| (format!("导航失败: {}", e), true))?;
        tokio::time::sleep(std::time::Duration::from_millis(2000)).await;

        // 从页面 __INITIAL_STATE__ 提取视频URL
        let extract_js = r#"(function(){
            try {
                var state = JSON.parse(JSON.stringify(window.__INITIAL_STATE__));
                for (var k in state) {
                    var map = state[k] && state[k].noteDetailMap;
                    if (!map) continue;
                    for (var nk in map) {
                        var note = map[nk].note || map[nk];
                        var h264 = note && note.video && note.video.media && note.video.media.stream && note.video.media.stream.h264;
                        if (h264 && h264[0] && h264[0].masterUrl) return h264[0].masterUrl;
                    }
                }
            } catch(e){}
            return '';
        })()"#;
        let mp4_url = cdp.eval(extract_js).await.unwrap_or_default();
        if !mp4_url.is_empty() {
            let real_url = if mp4_url.starts_with("http://") { mp4_url.replacen("http://", "https://", 1) } else { mp4_url };
            eprintln!("[小红书下载] CDP解析成功 nid={} url前60={}", note_id, &real_url[..real_url.len().min(60)]);
            // 直接下载（不递归调用download_single_file，避免async递归问题）
            let referer = "https://www.xiaohongshu.com/";
            let resp = client.get(&real_url)
                .header("User-Agent", UA)
                .header("Referer", referer)
                .header("Accept", "*/*")
                .send().await
                .map_err(|e| (format!("请求失败: {}", e), true))?;
            if !resp.status().is_success() {
                return Err((format!("HTTP {}", resp.status()), true));
            }
            let content_len = resp.content_length();
            let tmp_path = { let mut p = save_path.as_os_str().to_owned(); p.push(".downloading"); std::path::PathBuf::from(p) };
            if let Some(parent) = save_path.parent() { let _ = tokio::fs::create_dir_all(parent).await; }
            let mut file = tokio::fs::File::create(&tmp_path).await.map_err(|e| (format!("创建文件失败: {}", e), false))?;
            use futures_util::StreamExt;
            use tokio::io::AsyncWriteExt;
            let mut stream = resp.bytes_stream();
            let mut written: u64 = 0;
            let mut last_emit: u64 = 0;
            while let Some(chunk) = stream.next().await {
                let chunk = chunk.map_err(|e| (format!("读取中断: {}", e), true))?;
                file.write_all(&chunk).await.map_err(|e| (format!("写入失败: {}", e), false))?;
                written += chunk.len() as u64;
                if let (Some(a), Some(tid)) = (app, task_id) {
                    if written - last_emit >= 256 * 1024 {
                        last_emit = written;
                        let _ = a.emit("download-file-progress", serde_json::json!({"task_id": tid, "downloaded": written, "total": content_len.unwrap_or(0)}));
                    }
                }
            }
            file.flush().await.map_err(|e| (format!("刷新失败: {}", e), false))?;
            drop(file);
            if written == 0 { let _ = tokio::fs::remove_file(&tmp_path).await; return Err(("下载内容为空".into(), true)); }
            tokio::fs::rename(&tmp_path, save_path).await.map_err(|e| (format!("保存失败: {}", e), false))?;
            return Ok(written);
        }
        return Err(("小红书视频URL解析失败".to_string(), true));
    }

    if url.contains(".m3u8") {
        return download_m3u8_as_mp4(client, url, save_path, app, task_id).await;
    }
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
    let mut last_emit: u64 = 0;
    const EMIT_INTERVAL: u64 = 256 * 1024;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| (format!("读取数据中断: {}", e), true))?;
        file.write_all(&chunk).await
            .map_err(|e| (format!("写入失败: {}", e), false))?;
        written += chunk.len() as u64;
        if let (Some(a), Some(tid)) = (app, task_id) {
            if written - last_emit >= EMIT_INTERVAL {
                last_emit = written;
                let _ = a.emit("download-file-progress", serde_json::json!({
                    "task_id": tid,
                    "downloaded": written,
                    "total": content_len.unwrap_or(0),
                }));
            }
        }
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
                    let _ = app.emit("batch-download-progress", json!({
                        "task_id": task_id, "status": "skipped",
                        "completed": done, "total": total,
                        "bytes": total_bytes.load(Ordering::Relaxed)
                    }));
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

                    match download_single_file(&client, dl_url, &save_path, Some(&app), Some(&task_id)).await {
                        Ok(size) => {
                            let bytes = total_bytes.fetch_add(size, Ordering::Relaxed) + size;
                            let done = completed.fetch_add(1, Ordering::Relaxed) + 1;
                            let _ = app.emit("batch-download-progress", json!({
                                "task_id": task_id, "status": "done",
                                "completed": done, "total": total, "bytes": bytes
                            }));
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
            let _ = app.emit("batch-download-progress", json!({
                "task_id": task_id, "status": "error",
                "completed": done, "total": total, "error": last_error,
                "bytes": total_bytes.load(Ordering::Relaxed)
            }));
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
