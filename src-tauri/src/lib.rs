use tauri::Manager;
use tauri::Emitter;
use tauri::AppHandle;
use tauri::menu::{MenuBuilder, MenuItemBuilder, CheckMenuItemBuilder, CheckMenuItem, SubmenuBuilder};
use tauri::tray::TrayIconBuilder;
use tauri::image::Image;
use std::collections::hash_map::DefaultHasher;
use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};
use hmac::{Hmac, Mac};
use sha2::{Sha256, digest::Digest};
use rand::Rng;
use std::time::{SystemTime, UNIX_EPOCH};
use aes_gcm::{Aes256Gcm, KeyInit, aead::Aead, Nonce};
use base64::Engine;
use std::fs;
use std::path::PathBuf;
use std::io::{BufRead, BufReader};
use std::sync::atomic::{AtomicBool, Ordering};

mod app_config;
mod cdp_parse;
mod chrome_app;
mod database;
mod download_accounts;
mod download_parse;
use app_config::{APP_ID, APP_KEY};

type HmacSha256 = Hmac<Sha256>;

/* ─── 退出时登出会话 ─── */

#[derive(Clone)]
struct SessionInfo {
    token: String,
    device_id: String,
    instance_id: String,
    app_id: String,
    api_base_url: String,
}

struct SessionState(std::sync::Mutex<Option<SessionInfo>>);

#[tauri::command]
fn register_session(app: AppHandle, token: String, device_id: String, instance_id: String, api_base_url: String) {
    if let Some(state) = app.try_state::<SessionState>() {
        if let Ok(mut session) = state.0.lock() {
            *session = Some(SessionInfo {
                token, device_id, instance_id,
                app_id: APP_ID.to_string(),
                api_base_url,
            });
        }
    }
}

#[tauri::command]
fn clear_session(app: AppHandle) {
    if let Some(state) = app.try_state::<SessionState>() {
        if let Ok(mut session) = state.0.lock() {
            *session = None;
        }
    }
}

fn send_logout_on_exit(session: &SessionInfo) {
    let body = serde_json::json!({
        "app_id": &session.app_id,
        "token": &session.token,
        "device_id": &session.device_id,
        "instance_id": &session.instance_id,
    });

    let obj = body.as_object().unwrap();
    let mut sorted: BTreeMap<&str, String> = BTreeMap::new();
    for (k, v) in obj {
        if k == "sign" { continue; }
        let val_str = match v {
            serde_json::Value::String(s) => s.clone(),
            serde_json::Value::Number(n) => n.to_string(),
            _ => v.to_string(),
        };
        sorted.insert(k.as_str(), val_str);
    }

    let params: Vec<String> = sorted.iter().map(|(k, v)| format!("{}={}", k, v)).collect();
    let param_str = params.join("&");
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .to_string();
    let nonce = generate_nonce();
    let sign_string = format!("{}&timestamp={}&nonce={}", param_str, timestamp, nonce);

    let Ok(mut mac) = <HmacSha256 as Mac>::new_from_slice(APP_KEY.as_bytes()) else { return };
    mac.update(sign_string.as_bytes());
    let sign = hex::encode(mac.finalize().into_bytes());

    let url = format!("{}/client/user/logout", session.api_base_url);
    let Ok(client) = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .build() else { return };

    let _ = client.post(&url)
        .header("Content-Type", "application/json")
        .header("X-App-Id", &session.app_id)
        .header("X-Timestamp", &timestamp)
        .header("X-Nonce", &nonce)
        .header("X-Sign", &sign)
        .json(&body)
        .send();
}

struct TrayChecks {
    autostart_on: CheckMenuItem<tauri::Wry>,
    autostart_off: CheckMenuItem<tauri::Wry>,
    close_exit: CheckMenuItem<tauri::Wry>,
    close_minimize: CheckMenuItem<tauri::Wry>,
}

#[tauri::command]
fn sync_tray_checks(app: AppHandle, autostart: bool, close_mode: String) -> Result<(), String> {
    if let Some(state) = app.try_state::<TrayChecks>() {
        let _ = state.autostart_on.set_checked(autostart);
        let _ = state.autostart_off.set_checked(!autostart);
        let _ = state.close_exit.set_checked(close_mode == "exit");
        let _ = state.close_minimize.set_checked(close_mode == "minimize");
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn apply_rounded_corners(window: &tauri::WebviewWindow) {
    use windows_sys::Win32::Graphics::Dwm::DwmSetWindowAttribute;

    const DWMWA_WINDOW_CORNER_PREFERENCE: u32 = 33;
    const DWMWCP_ROUND: u32 = 2;

    if let Ok(hwnd) = window.hwnd() {
        unsafe {
            DwmSetWindowAttribute(
                hwnd.0 as *mut std::ffi::c_void,
                DWMWA_WINDOW_CORNER_PREFERENCE,
                &DWMWCP_ROUND as *const u32 as *const std::ffi::c_void,
                std::mem::size_of::<u32>() as u32,
            );
        }
    }
}

#[tauri::command]
fn prepare_window(app: AppHandle, width: f64, height: f64, min_width: f64, min_height: f64, resizable: bool) -> Result<(), String> {
    let window = app.get_webview_window("main").ok_or("窗口不存在")?;
    window.hide().map_err(|e| format!("hide 失败: {}", e))?;
    window.set_size(tauri::LogicalSize::new(width, height)).map_err(|e| format!("setSize 失败: {}", e))?;
    window.set_min_size(Some(tauri::Size::Logical(tauri::LogicalSize { width: min_width, height: min_height }))).map_err(|e| format!("setMinSize 失败: {}", e))?;
    window.set_resizable(resizable).map_err(|e| format!("setResizable 失败: {}", e))?;
    window.center().map_err(|e| format!("center 失败: {}", e))?;
    Ok(())
}

#[tauri::command]
fn reveal_window(app: AppHandle) -> Result<(), String> {
    let window = app.get_webview_window("main").ok_or("窗口不存在")?;
    window.show().map_err(|e| format!("show 失败: {}", e))?;
    window.set_focus().map_err(|e| format!("setFocus 失败: {}", e))?;
    Ok(())
}

#[tauri::command]
fn exit_app(app: AppHandle) {
    if let Some(state) = app.try_state::<chrome_app::ChromeProcess>() {
        if let Ok(mut guard) = state.0.lock() {
            if let Some(ref mut child) = *guard {
                let _ = child.kill();
                let _ = child.wait();
            }
            *guard = None;
        }
    }
    if let Some(state) = app.try_state::<chrome_app::LoginSessionManager>() {
        if let Ok(mut sessions) = state.0.lock() {
            for (_, session) in sessions.iter_mut() {
                let _ = session.child.kill();
                let _ = session.child.wait();
            }
            sessions.clear();
        }
    }
    app.exit(0);
}

#[tauri::command]
fn get_device_id() -> String {
    let mut parts: Vec<String> = Vec::new();

    if let Ok(name) = hostname::get() {
        parts.push(name.to_string_lossy().to_string());
    }

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        if let Ok(output) = std::process::Command::new("wmic")
            .args(["csproduct", "get", "UUID"])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
        {
            let text = String::from_utf8_lossy(&output.stdout);
            for line in text.lines() {
                let trimmed = line.trim();
                if !trimmed.is_empty() && trimmed != "UUID" {
                    parts.push(trimmed.to_string());
                    break;
                }
            }
        }
    }

    let raw = parts.join("|");
    let mut hasher = DefaultHasher::new();
    raw.hash(&mut hasher);
    format!("{:016X}", hasher.finish())
}

#[tauri::command]
fn get_app_credentials() -> (String, String) {
    (APP_ID.to_string(), APP_KEY.to_string())
}

fn generate_nonce() -> String {
    let mut rng = rand::thread_rng();
    (0..24).map(|_| {
        let idx = rng.gen_range(0..36);
        if idx < 10 { (b'0' + idx) as char } else { (b'a' + idx - 10) as char }
    }).collect()
}

#[derive(serde::Serialize)]
pub struct SignResult {
    pub app_id: String,
    pub timestamp: String,
    pub nonce: String,
    pub sign: String,
}

#[tauri::command]
fn compute_sign(body_json: String) -> Result<SignResult, String> {
    let body: serde_json::Value = serde_json::from_str(&body_json)
        .map_err(|e| format!("JSON 解析失败: {}", e))?;

    let obj = body.as_object().ok_or("请求体不是 JSON 对象")?;

    let mut sorted: BTreeMap<&str, String> = BTreeMap::new();
    for (k, v) in obj {
        if k == "sign" { continue; }
        if v.is_null() { continue; }
        let val_str = match v {
            serde_json::Value::String(s) => s.clone(),
            serde_json::Value::Number(n) => n.to_string(),
            serde_json::Value::Bool(b) => b.to_string(),
            _ => v.to_string(),
        };
        sorted.insert(k.as_str(), val_str);
    }

    let params: Vec<String> = sorted.iter().map(|(k, v)| format!("{}={}", k, v)).collect();
    let param_str = params.join("&");

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("时间错误: {}", e))?
        .as_secs()
        .to_string();

    let nonce = generate_nonce();

    let sign_string = format!("{}&timestamp={}&nonce={}", param_str, timestamp, nonce);

    let mut mac = <HmacSha256 as Mac>::new_from_slice(APP_KEY.as_bytes())
        .map_err(|e| format!("HMAC 初始化失败: {}", e))?;
    mac.update(sign_string.as_bytes());
    let sign = hex::encode(mac.finalize().into_bytes());

    Ok(SignResult {
        app_id: APP_ID.to_string(),
        timestamp,
        nonce,
        sign,
    })
}

/* ─── 品牌加密配置文件 ─── */

fn get_config_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_data_dir().map_err(|e| format!("获取数据目录失败: {}", e))?;
    fs::create_dir_all(&dir).map_err(|e| format!("创建目录失败: {}", e))?;
    Ok(dir.join("brand_config.enc"))
}

fn derive_key() -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(APP_KEY.as_bytes());
    let result = hasher.finalize();
    let mut key = [0u8; 32];
    key.copy_from_slice(&result);
    key
}

fn decrypt_config_bytes(encrypted_b64: &str) -> Result<String, String> {
    let raw = base64::engine::general_purpose::STANDARD
        .decode(encrypted_b64)
        .map_err(|e| format!("base64 解码失败: {}", e))?;
    if raw.len() < 12 + 16 {
        return Err("密文数据太短".to_string());
    }
    let (nonce_bytes, ciphertext) = raw.split_at(12);
    let nonce = Nonce::from_slice(nonce_bytes);
    let key = derive_key();
    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|e| format!("创建解密器失败: {}", e))?;
    let plaintext = cipher.decrypt(nonce, ciphertext)
        .map_err(|_| "解密失败：密钥不匹配或数据已损坏".to_string())?;
    String::from_utf8(plaintext).map_err(|e| format!("UTF-8 解码失败: {}", e))
}

#[tauri::command]
fn read_brand_config(app: AppHandle) -> Result<Option<String>, String> {
    let path = get_config_path(&app)?;
    if path.exists() {
        let encrypted_b64 = fs::read_to_string(&path)
            .map_err(|e| format!("读取配置文件失败: {}", e))?;
        let json = decrypt_config_bytes(encrypted_b64.trim())?;
        return Ok(Some(json));
    }

    if let Ok(res_dir) = app.path().resource_dir() {
        let res_path = res_dir.join("resources").join("brand_config.enc");
        if res_path.exists() {
            let encrypted_b64 = fs::read_to_string(&res_path)
                .map_err(|e| format!("读取内置配置失败: {}", e))?;
            let json = decrypt_config_bytes(encrypted_b64.trim())?;
            let _ = fs::write(&path, encrypted_b64.trim());
            return Ok(Some(json));
        }
    }

    Ok(None)
}

#[tauri::command]
fn save_brand_config(app: AppHandle, encrypted_base64: String) -> Result<(), String> {
    let path = get_config_path(&app)?;
    fs::write(&path, encrypted_base64.trim())
        .map_err(|e| format!("写入配置文件失败: {}", e))
}

#[tauri::command]
fn decrypt_brand_config(encrypted_base64: String) -> Result<String, String> {
    decrypt_config_bytes(encrypted_base64.trim())
}

/* ─── 托盘图标 ─── */

#[tauri::command]
fn update_tray(app: AppHandle, tooltip: String, icon_data: String) -> Result<(), String> {
    if let Some(tray) = app.tray_by_id("main") {
        if !tooltip.is_empty() {
            let _ = tray.set_tooltip(Some(&tooltip));
        }
    }

    if icon_data.is_empty() { return Ok(()); }

    let raw = if icon_data.contains(',') {
        icon_data.split(',').nth(1).unwrap_or("")
    } else {
        &icon_data
    };
    let png_bytes = base64::engine::general_purpose::STANDARD
        .decode(raw)
        .map_err(|e| format!("base64 解码失败: {}", e))?;

    let icon_path = app.path().app_data_dir()
        .map_err(|e| format!("获取数据目录失败: {}", e))?
        .join("tray_icon.png");
    fs::write(&icon_path, &png_bytes)
        .map_err(|e| format!("写入图标失败: {}", e))?;

    let icon = Image::from_path(&icon_path)
        .map_err(|e| format!("加载图标失败: {}", e))?;

    if let Some(tray) = app.tray_by_id("main") {
        let _ = tray.set_icon(Some(icon));
    }

    if let Some(window) = app.get_webview_window("main") {
        let icon2 = Image::from_path(&icon_path)
            .map_err(|e| format!("加载窗口图标失败: {}", e))?;
        let _ = window.set_icon(icon2);
    }

    Ok(())
}

/* ─── 品牌打包构建 ─── */

static BUILD_RUNNING: AtomicBool = AtomicBool::new(false);

#[tauri::command]
fn is_build_running() -> bool {
    BUILD_RUNNING.load(Ordering::SeqCst)
}

#[tauri::command]
fn start_brand_build(app: AppHandle, brand_name: String, product_name: String, logo_data: String, current_version: Option<String>) -> Result<(), String> {
    if BUILD_RUNNING.load(Ordering::SeqCst) {
        return Err("已有构建任务正在运行".to_string());
    }

    let tauri_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let project_root = tauri_dir.parent()
        .ok_or("无法确定项目根目录")?
        .to_path_buf();

    let config_path = get_config_path(&app)?;
    if !config_path.exists() {
        return Err("品牌配置文件不存在，请先保存品牌设置".to_string());
    }

    let build_dir = project_root.join(".brand-build");
    fs::create_dir_all(&build_dir)
        .map_err(|e| format!("创建构建目录失败: {}", e))?;
    fs::copy(&config_path, build_dir.join("brand_config.enc"))
        .map_err(|e| format!("复制品牌配置失败: {}", e))?;

    BUILD_RUNNING.store(true, Ordering::SeqCst);
    let _ = app.emit("build-status", "building");

    std::thread::spawn(move || {
        let ver_ref = current_version.as_deref();
        let result = execute_build(&app, &tauri_dir, &project_root, &brand_name, &product_name, &logo_data, true, ver_ref);

        let payload = match result {
            Ok(path) => serde_json::json!({ "success": true, "output_path": path }),
            Err(e) => serde_json::json!({ "success": false, "error": e }),
        };
        let _ = app.emit("build-complete", payload);
        BUILD_RUNNING.store(false, Ordering::SeqCst);
    });

    Ok(())
}

fn decode_data_uri(data_uri: &str) -> Result<Vec<u8>, String> {
    let raw = if data_uri.contains(',') {
        data_uri.split(',').nth(1).unwrap_or("")
    } else {
        data_uri
    };
    base64::engine::general_purpose::STANDARD
        .decode(raw)
        .map_err(|e| format!("base64 解码失败: {}", e))
}

fn create_ico_from_png(png_data: &[u8], ico_path: &PathBuf) -> Result<(), String> {
    let img = image::load_from_memory(png_data)
        .map_err(|e| format!("PNG 解码失败: {}", e))?;

    let sizes: &[u32] = &[48, 32, 16];
    let mut bmp_entries: Vec<Vec<u8>> = Vec::new();

    for &size in sizes {
        let resized = img.resize_exact(size, size, image::imageops::FilterType::Lanczos3);
        let rgba = resized.to_rgba8();

        let mut bmp = Vec::new();
        // BITMAPINFOHEADER (40 bytes)
        bmp.extend_from_slice(&40u32.to_le_bytes());
        bmp.extend_from_slice(&(size as i32).to_le_bytes());
        bmp.extend_from_slice(&((size * 2) as i32).to_le_bytes()); // height doubled for ICO
        bmp.extend_from_slice(&1u16.to_le_bytes());
        bmp.extend_from_slice(&32u16.to_le_bytes());
        bmp.extend_from_slice(&0u32.to_le_bytes()); // BI_RGB
        let mask_row_bytes = ((size + 31) / 32) * 4;
        let image_size = size * size * 4 + mask_row_bytes * size;
        bmp.extend_from_slice(&image_size.to_le_bytes());
        bmp.extend_from_slice(&[0u8; 16]); // remaining header fields

        // pixel data: BGRA, bottom-to-top
        for y in (0..size).rev() {
            for x in 0..size {
                let p = rgba.get_pixel(x, y);
                bmp.extend_from_slice(&[p[2], p[1], p[0], p[3]]);
            }
        }

        // AND mask
        for y in (0..size).rev() {
            let mut row = vec![0u8; mask_row_bytes as usize];
            for x in 0..size {
                if rgba.get_pixel(x, y)[3] == 0 {
                    row[(x / 8) as usize] |= 1 << (7 - (x % 8));
                }
            }
            bmp.extend_from_slice(&row);
        }

        bmp_entries.push(bmp);
    }

    let mut ico = Vec::new();
    // ICO header
    ico.extend_from_slice(&0u16.to_le_bytes());
    ico.extend_from_slice(&1u16.to_le_bytes());
    ico.extend_from_slice(&(sizes.len() as u16).to_le_bytes());

    // directory entries
    let header_len = 6 + 16 * sizes.len();
    let mut offset = header_len;
    for (i, &size) in sizes.iter().enumerate() {
        ico.push(size as u8);
        ico.push(size as u8);
        ico.push(0); ico.push(0);
        ico.extend_from_slice(&1u16.to_le_bytes());
        ico.extend_from_slice(&32u16.to_le_bytes());
        ico.extend_from_slice(&(bmp_entries[i].len() as u32).to_le_bytes());
        ico.extend_from_slice(&(offset as u32).to_le_bytes());
        offset += bmp_entries[i].len();
    }

    for entry in &bmp_entries {
        ico.extend_from_slice(entry);
    }

    fs::write(ico_path, ico).map_err(|e| format!("写入 ICO 失败: {}", e))
}

fn execute_build(app: &AppHandle, tauri_dir: &PathBuf, project_root: &PathBuf, brand_name: &str, product_name: &str, logo_data: &str, include_brand_config: bool, version: Option<&str>) -> Result<String, String> {
    let build_dir = project_root.join(".brand-build");
    fs::create_dir_all(&build_dir).map_err(|e| format!("创建构建目录失败: {}", e))?;
    let display_name = if product_name.is_empty() { brand_name } else { product_name };

    let nsis_hooks_str = format!(
        r#"!define MUI_CUSTOMFUNCTION_GUIINIT __CustomGuiInit

Function __CustomGuiInit
  IfFileExists "D:\*.*" 0 __skip_d_drive
    StrCpy $INSTDIR "D:\{name}"
  __skip_d_drive:
FunctionEnd
"#,
        name = display_name
    );
    let mut nsis_hooks_bytes: Vec<u8> = vec![0xEF, 0xBB, 0xBF];
    nsis_hooks_bytes.extend_from_slice(nsis_hooks_str.as_bytes());
    fs::write(build_dir.join("nsis-hooks.nsi"), nsis_hooks_bytes)
        .map_err(|e| format!("写入 NSIS hooks 失败: {}", e))?;

    let mut config_override = serde_json::json!({
        "productName": display_name,
        "mainBinaryName": display_name,
        "bundle": {
            "targets": ["nsis"],
            "windows": {
                "nsis": {
                    "languages": ["SimpChinese"],
                    "displayLanguageSelector": false,
                    "installerHooks": "../.brand-build/nsis-hooks.nsi"
                }
            }
        }
    });
    if include_brand_config {
        config_override["bundle"]["resources"] = serde_json::json!({
            "../.brand-build/brand_config.enc": "resources/brand_config.enc"
        });
    }

    if !include_brand_config {
        let update_icon_png = tauri_dir.join("icons").join("update-icon.png");
        if update_icon_png.exists() {
            let ico_path = build_dir.join("update-icon.ico");
            if let Ok(png_bytes) = fs::read(&update_icon_png) {
                if create_ico_from_png(&png_bytes, &ico_path).is_ok() {
                    config_override["bundle"]["icon"] = serde_json::json!([
                        "../src-tauri/icons/update-icon.png",
                        "../.brand-build/update-icon.ico"
                    ]);
                    config_override["bundle"]["windows"]["nsis"]["installerIcon"] = serde_json::json!("../.brand-build/update-icon.ico");
                    let _ = app.emit("build-log", "使用通用更新图标");
                }
            }
        }
    } else if !logo_data.is_empty() {
        if let Ok(png_bytes) = decode_data_uri(logo_data) {
            let png_path = build_dir.join("brand-icon.png");
            let ico_path = build_dir.join("brand-icon.ico");
            if fs::write(&png_path, &png_bytes).is_ok() && create_ico_from_png(&png_bytes, &ico_path).is_ok() {
                config_override["bundle"]["icon"] = serde_json::json!([
                    "../.brand-build/brand-icon.png",
                    "../.brand-build/brand-icon.ico"
                ]);
                config_override["bundle"]["windows"]["nsis"]["installerIcon"] = serde_json::json!("../.brand-build/brand-icon.ico");
                let _ = app.emit("build-log", "已生成品牌图标文件");
            }
        }
    }

    if config_override["bundle"]["windows"]["nsis"].get("installerIcon").is_none() {
        config_override["bundle"]["windows"]["nsis"]["installerIcon"] = serde_json::json!("icons/icon.ico");
    }

    let config_str = serde_json::to_string(&config_override)
        .map_err(|e| format!("序列化配置失败: {}", e))?;

    let config_file = build_dir.join("tauri-build-config.json");
    fs::write(&config_file, &config_str)
        .map_err(|e| format!("写入构建配置失败: {}", e))?;

    let config_file_str = config_file.to_string_lossy().to_string();
    let _ = app.emit("build-log", format!("构建配置: productName={}, 语言=简体中文", display_name));

    let nsis_dir = tauri_dir.join("target").join("release").join("bundle").join("nsis");
    if nsis_dir.exists() {
        let _ = fs::remove_dir_all(&nsis_dir);
        let _ = app.emit("build-log", "已清理旧 NSIS 构建缓存");
    }

    let output_dir = project_root.join("output");
    if output_dir.exists() {
        if let Ok(entries) = fs::read_dir(&output_dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.extension().map_or(false, |e| e == "exe") {
                    let _ = fs::remove_file(&p);
                }
            }
        }
    }

    let build_result = run_build_process(app, project_root, tauri_dir, &config_file_str, version);

    let final_result = match build_result {
        Ok(exe_path) => {
            let _ = fs::create_dir_all(&output_dir);
            let src = PathBuf::from(&exe_path);
            if src.exists() {
                let out_filename = if let Some(ver) = version {
                    let v = if ver.starts_with('V') || ver.starts_with('v') {
                        ver.to_string()
                    } else {
                        format!("V{}", ver)
                    };
                    format!("{}_{}.exe", display_name, v)
                } else if let Some(f) = src.file_name() {
                    f.to_string_lossy().to_string()
                } else {
                    return Ok(exe_path);
                };
                let dest = output_dir.join(&out_filename);
                fs::copy(&src, &dest)
                    .map_err(|e| format!("复制安装包到 output 目录失败: {}", e))?;
                let _ = app.emit("build-log", format!("已复制到: {}", dest.to_string_lossy()));
                Ok(dest.to_string_lossy().to_string())
            } else { Ok(exe_path) }
        }
        Err(e) => Err(e),
    };

    let _ = fs::remove_dir_all(&build_dir);

    if nsis_dir.exists() {
        let _ = fs::remove_dir_all(&nsis_dir);
    }

    final_result
}

fn run_build_process(app: &AppHandle, project_root: &PathBuf, tauri_dir: &PathBuf, config_file: &str, version: Option<&str>) -> Result<String, String> {
    #[cfg(target_os = "windows")]
    let mut cmd = std::process::Command::new("cmd");
    #[cfg(target_os = "windows")]
    cmd.args(["/c", "npx", "tauri", "build", "--config", config_file]);

    #[cfg(not(target_os = "windows"))]
    let mut cmd = std::process::Command::new("npx");
    #[cfg(not(target_os = "windows"))]
    cmd.args(["tauri", "build", "--config", config_file]);

    cmd.current_dir(project_root)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    if let Some(v) = version {
        let ver = if v.starts_with('V') || v.starts_with('v') {
            v.to_string()
        } else {
            format!("V{}", v)
        };
        cmd.env("VITE_APP_VERSION", &ver);
        let _ = app.emit("build-log", format!("设置构建版本号: {}", ver));
    }

    let mut child = cmd.spawn()
        .map_err(|e| format!("启动构建失败: {}", e))?;

    let app_out = app.clone();
    let stdout = child.stdout.take();
    let t1 = std::thread::spawn(move || {
        if let Some(out) = stdout {
            for line in BufReader::new(out).lines().flatten() {
                let _ = app_out.emit("build-log", &line);
            }
        }
    });

    let app_err = app.clone();
    let stderr = child.stderr.take();
    let t2 = std::thread::spawn(move || {
        if let Some(err) = stderr {
            for line in BufReader::new(err).lines().flatten() {
                let _ = app_err.emit("build-log", &line);
            }
        }
    });

    let _ = t1.join();
    let _ = t2.join();

    let status = child.wait()
        .map_err(|e| format!("等待构建进程失败: {}", e))?;

    if !status.success() {
        return Err(format!("构建失败，退出码: {}", status.code().unwrap_or(-1)));
    }

    let bundle_dir = tauri_dir.join("target").join("release").join("bundle");
    let nsis_dir = bundle_dir.join("nsis");

    if nsis_dir.exists() {
        if let Ok(entries) = fs::read_dir(&nsis_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map_or(false, |ext| ext == "exe") {
                    return Ok(path.to_string_lossy().to_string());
                }
            }
        }
    }

    Ok(bundle_dir.to_string_lossy().to_string())
}

/* ─── 写入版本号到 brand.ts ─── */

#[tauri::command]
fn update_version_in_source(app: AppHandle, version: String) -> Result<(), String> {
    let tauri_dir = app.path().resolve("", tauri::path::BaseDirectory::Resource)
        .map_err(|e| format!("获取目录失败: {}", e))?
        .parent().unwrap().parent().unwrap().parent().unwrap()
        .join("src-tauri");
    let project_root = tauri_dir.parent().unwrap();
    let src_dir = project_root.join("src").join("brand.ts");

    let content = fs::read_to_string(&src_dir)
        .map_err(|e| format!("读取 brand.ts 失败: {}", e))?;

    let re = regex::Regex::new(r"export const VERSION = '[^']*'")
        .map_err(|e| format!("正则编译失败: {}", e))?;
    let new_content = re.replace(&content, format!("export const VERSION = 'V{}'", version).as_str());

    fs::write(&src_dir, new_content.as_ref())
        .map_err(|e| format!("写入 brand.ts 失败: {}", e))?;

    Ok(())
}

/* ─── 版本打包（不含品牌配置） ─── */

#[tauri::command]
async fn start_version_build(
    app: AppHandle,
    brand_name: String,
    product_name: String,
    logo_data: String,
    version: String,
) -> Result<(), String> {
    if BUILD_RUNNING.load(Ordering::SeqCst) {
        return Err("已有构建任务在运行".into());
    }

    let tauri_dir = app.path().resolve("", tauri::path::BaseDirectory::Resource)
        .map_err(|e| format!("获取 tauri 目录失败: {}", e))?
        .parent().unwrap().parent().unwrap().parent().unwrap()
        .join("src-tauri");
    let project_root = tauri_dir.parent().unwrap().to_path_buf();

    BUILD_RUNNING.store(true, Ordering::SeqCst);
    let _ = app.emit("build-status", "building");

    std::thread::spawn(move || {
        let ver = if version.is_empty() { None } else { Some(version.as_str()) };
        let result = execute_build(&app, &tauri_dir, &project_root, &brand_name, &product_name, &logo_data, false, ver);

        let payload = match result {
            Ok(path) => serde_json::json!({ "success": true, "output_path": path }),
            Err(e) => serde_json::json!({ "success": false, "error": e }),
        };
        let _ = app.emit("build-complete", payload);
        BUILD_RUNNING.store(false, Ordering::SeqCst);
    });

    Ok(())
}

/* ─── 版本更新：下载 + 安装 ─── */

#[derive(Clone, serde::Serialize)]
struct DownloadFileProgress {
    loaded: u64,
    total: Option<u64>,
    percent: f64,
    file_path: String,
}

#[tauri::command]
fn read_file_base64(path: String) -> Result<String, String> {
    let clean = path.strip_prefix(r"\\?\").unwrap_or(&path);
    let bytes = fs::read(clean).map_err(|e| format!("读取文件失败: {}", e))?;
    Ok(base64::engine::general_purpose::STANDARD.encode(&bytes))
}

#[tauri::command]
fn compute_file_sha256(path: String) -> Result<String, String> {
    let clean = path.strip_prefix(r"\\?\").unwrap_or(&path);
    let bytes = fs::read(clean).map_err(|e| format!("读取文件失败: {}", e))?;
    let hash = Sha256::digest(&bytes);
    Ok(hex::encode(hash))
}

#[tauri::command]
async fn download_file_to_dir(
    app: AppHandle,
    url: String,
    save_dir: String,
) -> Result<String, String> {
    use futures_util::StreamExt;

    if url.is_empty() { return Err("下载地址为空".into()); }
    let dir = std::path::Path::new(&save_dir);
    if !dir.exists() {
        fs::create_dir_all(dir).map_err(|e| format!("创建目录失败: {}", e))?;
    }

    let url_path = url.split('?').next().unwrap_or(&url);
    let file_name = url_path.rsplit('/').next().unwrap_or("installer.exe");
    let file_path = dir.join(file_name);
    let file_path_str = file_path.to_string_lossy().to_string();

    let client = reqwest::Client::new();
    let response = client.get(&url).send().await
        .map_err(|e| format!("下载失败: {}", e))?;
    if !response.status().is_success() {
        return Err(format!("下载失败: HTTP {}", response.status()));
    }

    let total = response.content_length();
    let mut stream = response.bytes_stream();
    let mut downloaded: u64 = 0;
    let mut file = fs::File::create(&file_path)
        .map_err(|e| format!("创建文件失败: {}", e))?;

    while let Some(chunk) = stream.next().await {
        let bytes = chunk.map_err(|e| format!("读取数据失败: {}", e))?;
        downloaded += bytes.len() as u64;
        std::io::Write::write_all(&mut file, &bytes)
            .map_err(|e| format!("写入文件失败: {}", e))?;

        let percent = total
            .map(|t| if t > 0 { (downloaded as f64 / t as f64) * 100.0 } else { 0.0 })
            .unwrap_or(0.0);
        let _ = app.emit("download_file_progress", DownloadFileProgress {
            loaded: downloaded, total, percent,
            file_path: file_path_str.clone(),
        });
    }

    let _ = app.emit("download_file_progress", DownloadFileProgress {
        loaded: downloaded, total, percent: 100.0,
        file_path: file_path_str.clone(),
    });

    Ok(file_path_str)
}

#[tauri::command]
fn get_download_dir(_app: AppHandle) -> Result<String, String> {
    let exe = std::env::current_exe()
        .map_err(|e| format!("获取程序路径失败: {}", e))?;
    let install_dir = exe.parent()
        .ok_or_else(|| "无法获取程序所在目录".to_string())?;

    let dir = if install_dir.to_string_lossy().contains("target\\debug")
        || install_dir.to_string_lossy().contains("target/debug")
    {
        PathBuf::from("E:\\00000")
    } else {
        install_dir.join("downloads")
    };

    fs::create_dir_all(&dir).map_err(|e| format!("创建下载目录失败: {}", e))?;
    Ok(dir.to_string_lossy().to_string())
}

#[tauri::command]
async fn run_installer_and_exit(app: AppHandle, installer_path: String) -> Result<(), String> {
    let path = std::path::Path::new(&installer_path);
    if !path.exists() {
        return Err(format!("安装包不存在: {}", installer_path));
    }

    std::process::Command::new(&installer_path)
        .args(["/S", "/R"])
        .spawn()
        .map_err(|e| format!("启动安装包失败: {}", e))?;

    std::thread::sleep(std::time::Duration::from_millis(500));
    app.exit(0);
    Ok(())
}

#[tauri::command]
fn get_setting(key: String, app: AppHandle) -> Result<Option<String>, String> {
    let db = app.state::<database::DbState>();
    db.with_conn(|conn| {
        let mut stmt = conn.prepare("SELECT value FROM app_settings WHERE key = ?1")?;
        let result = stmt.query_row(rusqlite::params![key], |row| row.get::<_, String>(0));
        match result {
            Ok(val) => Ok(Some(val)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    })
}

#[tauri::command]
fn set_setting(key: String, value: String, app: AppHandle) -> Result<(), String> {
    let db = app.state::<database::DbState>();
    db.with_conn(|conn| {
        conn.execute(
            "INSERT INTO app_settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            rusqlite::params![key, value],
        )?;
        Ok(())
    })
}

#[tauri::command]
fn get_all_settings(app: AppHandle) -> Result<std::collections::HashMap<String, String>, String> {
    let db = app.state::<database::DbState>();
    db.with_conn(|conn| {
        let mut stmt = conn.prepare("SELECT key, value FROM app_settings")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        let mut map = std::collections::HashMap::new();
        for row in rows {
            let (k, v) = row?;
            map.insert(k, v);
        }
        Ok(map)
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, None))
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            get_device_id,
            get_app_credentials,
            compute_sign,
            prepare_window,
            reveal_window,
            exit_app,
            sync_tray_checks,
            read_brand_config,
            save_brand_config,
            decrypt_brand_config,
            update_tray,
            is_build_running,
            start_brand_build,
            start_version_build,
            update_version_in_source,
            read_file_base64,
            compute_file_sha256,
            download_file_to_dir,
            get_download_dir,
            run_installer_and_exit,
            download_accounts::list_download_accounts,
            download_accounts::upsert_download_account,
            download_accounts::update_platform_account,
            download_accounts::delete_platform_account,
            download_accounts::check_download_cookie_status,
            download_accounts::open_download_login,
            download_accounts::capture_download_cookies,
            download_accounts::close_download_webview,
            download_parse::resolve_video_url,
            download_parse::batch_download_videos,
            chrome_app::launch_chrome_app,
            chrome_app::kill_chrome_app,
            chrome_app::is_chrome_running,
            chrome_app::check_chrome_installed,
            cdp_parse::cdp_ensure_chrome,
            cdp_parse::cdp_check_login,
            cdp_parse::cdp_show_chrome,
            cdp_parse::cdp_hide_chrome,
            cdp_parse::cdp_kill_chrome,
            download_parse::api_parse_douyin_video,
            download_parse::api_parse_douyin_homepage,
            download_parse::api_parse_douyin_collection,
            download_parse::api_find_douyin_mix_id,
            download_parse::api_parse_kuaishou_video,
            download_parse::api_parse_kuaishou_homepage,
            download_parse::fetch_bilibili_video,
            download_parse::fetch_bilibili_homepage,
            download_parse::api_parse_migu_video,
            download_parse::api_parse_migu_homepage,
            download_parse::api_parse_cctv_video,
            download_parse::api_parse_cctv_column,
            download_parse::api_parse_yangshipin_video,
            download_parse::api_parse_xiaohongshu_video,
            download_parse::api_parse_xiaohongshu_homepage,
            cdp_parse::cdp_open_login,
            cdp_parse::cdp_close_login,
            get_setting,
            set_setting,
            get_all_settings,
            register_session,
            clear_session
        ])
        .setup(|app| {
            let db = database::init_database(&app.handle())
                .expect("数据库初始化失败");
            app.manage(db);

            app.manage(SessionState(std::sync::Mutex::new(None)));
            app.manage(chrome_app::ChromeProcess(std::sync::Mutex::new(None)));
            app.manage(chrome_app::LoginSessionManager(std::sync::Mutex::new(std::collections::HashMap::new())));
            app.manage(cdp_parse::ChromeSessionState(tokio::sync::Mutex::new(cdp_parse::ChromeSession::new())));

            if let Some(window) = app.get_webview_window("main") {
                #[cfg(target_os = "windows")]
                apply_rounded_corners(&window);
            }

            let show_item = MenuItemBuilder::with_id("show", "显示窗口").build(app)?;
            let refresh_item = MenuItemBuilder::with_id("refresh", "刷新界面").build(app)?;
            let clear_cache_item = MenuItemBuilder::with_id("clear_cache", "清理缓存").build(app)?;

            let autostart_on = CheckMenuItemBuilder::with_id("autostart_on", "开启").build(app)?;
            let autostart_off = CheckMenuItemBuilder::with_id("autostart_off", "关闭").checked(true).build(app)?;
            let autostart_sub = SubmenuBuilder::with_id(app, "autostart_sub", "开机自启")
                .item(&autostart_on)
                .item(&autostart_off)
                .build()?;

            let close_exit = CheckMenuItemBuilder::with_id("close_exit", "直接退出").checked(true).build(app)?;
            let close_minimize = CheckMenuItemBuilder::with_id("close_minimize", "最小化到后台").build(app)?;
            let close_sub = SubmenuBuilder::with_id(app, "close_sub", "关闭设置")
                .item(&close_exit)
                .item(&close_minimize)
                .build()?;

            app.manage(TrayChecks {
                autostart_on: autostart_on.clone(),
                autostart_off: autostart_off.clone(),
                close_exit: close_exit.clone(),
                close_minimize: close_minimize.clone(),
            });

            let about_item = MenuItemBuilder::with_id("about", "关于").build(app)?;
            let exit_item = MenuItemBuilder::with_id("exit", "退出程序").build(app)?;

            let tray_menu = MenuBuilder::new(app)
                .item(&show_item)
                .separator()
                .item(&refresh_item)
                .item(&clear_cache_item)
                .separator()
                .item(&autostart_sub)
                .item(&close_sub)
                .separator()
                .item(&about_item)
                .item(&exit_item)
                .build()?;

            let cached_icon_path = app.path().app_data_dir().ok()
                .map(|d| d.join("tray_icon.png"));
            let icon = cached_icon_path.as_ref()
                .and_then(|p| if p.exists() { Image::from_path(p).ok() } else { None })
                .unwrap_or_else(|| {
                    Image::from_path("icons/icon.png")
                        .unwrap_or_else(|_| Image::from_bytes(include_bytes!("../icons/icon.png")).expect("内置图标加载失败"))
                });

            let _tray = TrayIconBuilder::with_id("main")
                .icon(icon)
                .tooltip("应用加载中…")
                .menu(&tray_menu)
                .on_menu_event(|app: &AppHandle, event: tauri::menu::MenuEvent| {
                    match event.id().as_ref() {
                        "show" => {
                            if let Some(w) = app.get_webview_window("main") {
                                let _ = w.show();
                                let _ = w.set_focus();
                            }
                        }
                        "refresh" | "clear_cache" | "about" => {
                            if let Some(w) = app.get_webview_window("main") {
                                let _ = w.show();
                                let _ = w.set_focus();
                            }
                            let _ = app.emit("tray-action", event.id().as_ref());
                        }
                        "autostart_on" | "autostart_off" => {
                            let is_on = event.id().as_ref() == "autostart_on";
                            if let Some(state) = app.try_state::<TrayChecks>() {
                                let _ = state.autostart_on.set_checked(is_on);
                                let _ = state.autostart_off.set_checked(!is_on);
                            }
                            if let Some(w) = app.get_webview_window("main") {
                                let _ = w.show();
                                let _ = w.set_focus();
                            }
                            let _ = app.emit("tray-action", event.id().as_ref());
                        }
                        "close_exit" | "close_minimize" => {
                            let is_exit = event.id().as_ref() == "close_exit";
                            if let Some(state) = app.try_state::<TrayChecks>() {
                                let _ = state.close_exit.set_checked(is_exit);
                                let _ = state.close_minimize.set_checked(!is_exit);
                            }
                            if let Some(w) = app.get_webview_window("main") {
                                let _ = w.show();
                                let _ = w.set_focus();
                            }
                            let _ = app.emit("tray-action", event.id().as_ref());
                        }
                        "exit" => app.exit(0),
                        _ => {}
                    }
                })
                .on_tray_icon_event(|tray: &tauri::tray::TrayIcon, event| {
                    if let tauri::tray::TrayIconEvent::DoubleClick { .. } = event {
                        if let Some(w) = tray.app_handle().get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                })
                .build(app)?;

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            if let tauri::RunEvent::Exit = event {
                if let Some(state) = app.try_state::<SessionState>() {
                    if let Ok(mut session) = state.0.lock() {
                        if let Some(ref info) = session.take() {
                            send_logout_on_exit(info);
                        }
                    }
                }
            }
        });
}
