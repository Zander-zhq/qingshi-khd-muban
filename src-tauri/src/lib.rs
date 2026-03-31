use tauri::Manager;
use tauri::AppHandle;
use tauri::menu::{MenuBuilder, MenuItemBuilder};
use tauri::tray::TrayIconBuilder;
use tauri::image::Image;
use tauri::webview::WebviewWindowBuilder;
use std::collections::hash_map::DefaultHasher;
use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use rand::Rng;
use std::time::{SystemTime, UNIX_EPOCH};

const APP_ID: &str = "1004";
const APP_KEY: &str = "7cdba759d52ba9524374d161d47e8bf6";

type HmacSha256 = Hmac<Sha256>;

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

// #region agent log
fn dbg_log(loc: &str, msg: &str, data: &str) {
    use std::io::Write;
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(r"e:\qingshi\client_shipinxiazai\debug-23ef92.log") {
        let ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis();
        let _ = writeln!(f, r#"{{"sessionId":"23ef92","location":"{}","message":"{}","data":{},"timestamp":{}}}"#, loc, msg, data, ts);
    }
}
// #endregion

#[tauri::command]
fn activate_main_window(app: AppHandle) -> Result<(), String> {
    // #region agent log
    dbg_log("lib.rs:activate_main_window:entry", "activate_main_window called", "{}");
    // #endregion

    if let Some(login) = app.get_webview_window("login") {
        let _ = login.hide();
    }

    if let Some(main) = app.get_webview_window("main") {
        // #region agent log
        dbg_log("lib.rs:activate_main_window:existing", "main window already exists, showing", "{}");
        // #endregion
        main.show().map_err(|e| format!("显示主窗口失败: {}", e))?;
        main.set_focus().map_err(|e| format!("聚焦主窗口失败: {}", e))?;
        return Ok(());
    }

    // #region agent log
    dbg_log("lib.rs:activate_main_window:creating", "creating new main window via run_on_main_thread", "{}");
    // #endregion

    let app_handle = app.clone();
    app.run_on_main_thread(move || {
        // #region agent log
        dbg_log("lib.rs:activate_main_window:on_main_thread", "now on main thread, building window", "{}");
        // #endregion

        let build_result = WebviewWindowBuilder::new(
            &app_handle,
            "main",
            tauri::WebviewUrl::App("/main/dashboard".into()),
        )
        .title("青拾")
        .inner_size(1440.0, 900.0)
        .min_inner_size(1200.0, 760.0)
        .center()
        .decorations(false)
        .shadow(true)
        .resizable(true)
        .visible(false)
        .build();

        match build_result {
            Ok(main) => {
                // #region agent log
                dbg_log("lib.rs:activate_main_window:build_ok", "window build succeeded on main thread", "{}");
                // #endregion

                #[cfg(target_os = "windows")]
                apply_rounded_corners(&main);

                let handle = main.clone();
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_millis(800));
                    // #region agent log
                    dbg_log("lib.rs:activate_main_window:thread", "thread show/focus", "{}");
                    // #endregion
                    let _ = handle.show();
                    let _ = handle.set_focus();
                });
            }
            Err(e) => {
                // #region agent log
                dbg_log("lib.rs:activate_main_window:build_err", "window build FAILED", &format!(r#"{{"error":"{}"}}"#, e));
                // #endregion
                if let Some(login) = app_handle.get_webview_window("login") {
                    let _ = login.show();
                    let _ = login.set_focus();
                }
            }
        }
    }).map_err(|e| format!("dispatch to main thread failed: {}", e))?;

    Ok(())
}

#[tauri::command]
fn activate_login_window(app: AppHandle) -> Result<(), String> {
    if let Some(login) = app.get_webview_window("login") {
        login.show().map_err(|e| format!("显示登录窗口失败: {}", e))?;
        login.set_focus().map_err(|e| format!("聚焦登录窗口失败: {}", e))?;
    }

    if let Some(main) = app.get_webview_window("main") {
        let _ = main.close();
    }

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
        .invoke_handler(tauri::generate_handler![
            get_device_id,
            get_app_credentials,
            compute_sign,
            activate_main_window,
            activate_login_window,
            exit_app
        ])
        .setup(|app| {
            if let Some(window) = app.get_webview_window("login") {
                #[cfg(target_os = "windows")]
                apply_rounded_corners(&window);
            }

            let show_item = MenuItemBuilder::with_id("show", "显示窗口").build(app)?;
            let exit_item = MenuItemBuilder::with_id("exit", "退出程序").build(app)?;
            let tray_menu = MenuBuilder::new(app)
                .item(&show_item)
                .separator()
                .item(&exit_item)
                .build()?;

            let icon = Image::from_path("icons/icon.png")
                .unwrap_or_else(|_| Image::from_bytes(include_bytes!("../icons/icon.png")).expect("内置图标加载失败"));

            fn show_active_window(app: &AppHandle) {
                if let Some(main) = app.get_webview_window("main") {
                    let _ = main.show();
                    let _ = main.set_focus();
                } else if let Some(login) = app.get_webview_window("login") {
                    let _ = login.show();
                    let _ = login.set_focus();
                }
            }

            let _tray = TrayIconBuilder::new()
                .icon(icon)
                .tooltip("青拾")
                .menu(&tray_menu)
                .on_menu_event(|app: &AppHandle, event| {
                    match event.id().as_ref() {
                        "show" => show_active_window(app),
                        "exit" => app.exit(0),
                        _ => {}
                    }
                })
                .on_tray_icon_event(|tray: &tauri::tray::TrayIcon, event| {
                    if let tauri::tray::TrayIconEvent::DoubleClick { .. } = event {
                        show_active_window(tray.app_handle());
                    }
                })
                .build(app)?;

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
