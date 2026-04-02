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
use sha2::Sha256;
use rand::Rng;
use std::time::{SystemTime, UNIX_EPOCH};

const APP_ID: &str = "1001";
const APP_KEY: &str = "fb6837e15f113ca32d0a838272f3f659";

type HmacSha256 = Hmac<Sha256>;

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
        if let Ok(output) = std::process::Command::new("wmic")
            .args(["csproduct", "get", "UUID"])
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

    let mut mac = HmacSha256::new_from_slice(APP_KEY.as_bytes())
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, None))
        .plugin(tauri_plugin_http::init())
        .invoke_handler(tauri::generate_handler![
            get_device_id,
            get_app_credentials,
            compute_sign,
            prepare_window,
            reveal_window,
            exit_app,
            sync_tray_checks
        ])
        .setup(|app| {
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

            let icon = Image::from_path("icons/icon.png")
                .unwrap_or_else(|_| Image::from_bytes(include_bytes!("../icons/icon.png")).expect("内置图标加载失败"));

            let _tray = TrayIconBuilder::new()
                .icon(icon)
                .tooltip("青拾")
                .menu(&tray_menu)
                .on_menu_event(|app: &AppHandle, event| {
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
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
