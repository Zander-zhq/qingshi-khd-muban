use std::collections::HashMap;
use std::path::PathBuf;
use std::process::{Child, Command};
use std::sync::Mutex;
use tauri::Manager;

pub struct ChromeProcess(pub Mutex<Option<Child>>);

pub struct LoginSession {
    pub child: Child,
    pub port: u16,
}

pub struct LoginSessionManager(pub Mutex<HashMap<String, LoginSession>>);

// ── Chrome 查找 ──────────────────────────────────────────────────

pub fn find_chrome() -> Option<PathBuf> {
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

// ── Chrome 窗口尺寸计算（公共） ──────────────────────────────────

/// 根据主窗口尺寸计算 Chrome 弹出窗口的逻辑尺寸（宽-360, 高-290, 最小 800x600）
pub fn calc_chrome_window_size(app: &tauri::AppHandle) -> (i32, i32) {
    let main_win = app.get_webview_window("main");
    if let Some(ref win) = main_win {
        let phys_size = win
            .outer_size()
            .unwrap_or(tauri::PhysicalSize {
                width: 1200,
                height: 800,
            });
        let scale = win.scale_factor().unwrap_or(1.0);
        let main_logical_w = (phys_size.width as f64 / scale).round() as i32;
        let main_logical_h = (phys_size.height as f64 / scale).round() as i32;
        let cw = (main_logical_w - 360).max(800);
        let ch = (main_logical_h - 290).max(600);
        (cw, ch)
    } else {
        (800, 600)
    }
}

/// 获取主窗口的 HWND（仅 Windows），用于 Win32 窗口定位
pub fn get_main_hwnd(app: &tauri::AppHandle) -> isize {
    #[cfg(target_os = "windows")]
    {
        app.get_webview_window("main")
            .and_then(|w| w.hwnd().ok())
            .map(|h| h.0 as isize)
            .unwrap_or(0)
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = app;
        0
    }
}

// ── Win32 窗口操作（公共） ───────────────────────────────────────

#[cfg(target_os = "windows")]
pub mod win32_window {
    use windows_sys::Win32::Foundation::{BOOL, HWND, LPARAM, RECT};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindowRect, GetWindowThreadProcessId, IsWindowVisible, SetWindowPos,
        SWP_NOSIZE, SWP_NOZORDER,
    };

    struct FindWindowData {
        target_pid: u32,
        found_hwnd: HWND,
    }

    unsafe extern "system" fn enum_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let data = &mut *(lparam as *mut FindWindowData);
        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, &mut pid);
        if pid == data.target_pid && IsWindowVisible(hwnd) != 0 {
            data.found_hwnd = hwnd;
            return 0;
        }
        1
    }

    pub fn find_window_by_pid(pid: u32) -> Option<HWND> {
        let mut data = FindWindowData {
            target_pid: pid,
            found_hwnd: std::ptr::null_mut(),
        };
        unsafe {
            EnumWindows(Some(enum_callback), &mut data as *mut _ as LPARAM);
        }
        if !data.found_hwnd.is_null() {
            Some(data.found_hwnd)
        } else {
            None
        }
    }

    /// 将 Chrome 窗口居中到主窗口上（物理像素坐标，无需 DPI 换算）
    pub fn center_chrome_over_main(chrome_pid: u32, main_hwnd: isize) -> bool {
        let main_hwnd_ptr = main_hwnd as HWND;
        let mut main_rect = RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        unsafe {
            if GetWindowRect(main_hwnd_ptr, &mut main_rect) == 0 {
                return false;
            }
        }

        let chrome_hwnd = match find_window_by_pid(chrome_pid) {
            Some(h) => h,
            None => return false,
        };

        let mut chrome_rect = RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        unsafe {
            if GetWindowRect(chrome_hwnd, &mut chrome_rect) == 0 {
                return false;
            }
        }

        let main_w = main_rect.right - main_rect.left;
        let main_h = main_rect.bottom - main_rect.top;
        let chrome_w = chrome_rect.right - chrome_rect.left;
        let chrome_h = chrome_rect.bottom - chrome_rect.top;

        let x = main_rect.left + (main_w - chrome_w) / 2;
        let y = main_rect.top + (main_h - chrome_h) / 2;

        unsafe { SetWindowPos(chrome_hwnd, std::ptr::null_mut(), x, y, 0, 0, SWP_NOZORDER | SWP_NOSIZE) != 0 }
    }
}

/// 启动后异步等待 Chrome 窗口出现并居中到主窗口上
#[cfg(target_os = "windows")]
pub async fn center_chrome_async(chrome_pid: u32, main_hwnd: isize) {
    for attempt in 1..=10 {
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        if win32_window::center_chrome_over_main(chrome_pid, main_hwnd) {
            eprintln!("[Chrome] Win32 居中成功 (第{}次尝试)", attempt);
            return;
        }
    }
    eprintln!("[Chrome] Win32 居中失败，已尝试10次");
}

#[cfg(not(target_os = "windows"))]
pub async fn center_chrome_async(_chrome_pid: u32, _main_hwnd: isize) {}

#[tauri::command]
pub fn check_chrome_installed() -> Result<String, String> {
    find_chrome()
        .map(|p| p.to_string_lossy().to_string())
        .ok_or_else(|| "未检测到 Google Chrome 浏览器，请先安装 Chrome 后再使用此功能。\n下载地址：https://www.google.cn/chrome/".into())
}

#[tauri::command]
pub fn launch_chrome_app(
    app: tauri::AppHandle,
    url: String,
    width: Option<u32>,
    height: Option<u32>,
    port: Option<u16>,
) -> Result<u16, String> {
    let chrome_path = find_chrome().ok_or("未找到 Chrome 浏览器，请确认已安装 Google Chrome")?;

    let state = app.state::<ChromeProcess>();
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;

    if let Some(ref mut child) = *guard {
        match child.try_wait() {
            Ok(Some(_)) => {}
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
            }
            Err(_) => {}
        }
    }

    let debug_port = port.unwrap_or(9222);
    let w = width.unwrap_or(1200);
    let h = height.unwrap_or(800);

    let user_data = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("获取数据目录失败: {}", e))?
        .join("chrome_app_data");

    std::fs::create_dir_all(&user_data)
        .map_err(|e| format!("创建 Chrome 数据目录失败: {}", e))?;

    let child = Command::new(&chrome_path)
        .arg(format!("--app={}", url))
        .arg(format!("--remote-debugging-port={}", debug_port))
        .arg(format!("--user-data-dir={}", user_data.to_string_lossy()))
        .arg(format!("--window-size={},{}", w, h))
        .arg("--no-first-run")
        .arg("--no-default-browser-check")
        .arg("--disable-extensions")
        .arg("--disable-popup-blocking")
        .spawn()
        .map_err(|e| format!("启动 Chrome 失败: {}", e))?;

    *guard = Some(child);
    Ok(debug_port)
}

#[tauri::command]
pub fn kill_chrome_app(app: tauri::AppHandle) -> Result<(), String> {
    let state = app.state::<ChromeProcess>();
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;

    if let Some(ref mut child) = *guard {
        let _ = child.kill();
        let _ = child.wait();
    }
    *guard = None;
    Ok(())
}

#[tauri::command]
pub fn is_chrome_running(app: tauri::AppHandle) -> Result<bool, String> {
    let state = app.state::<ChromeProcess>();
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;

    if let Some(ref mut child) = *guard {
        match child.try_wait() {
            Ok(Some(_)) => {
                *guard = None;
                Ok(false)
            }
            Ok(None) => Ok(true),
            Err(_) => {
                *guard = None;
                Ok(false)
            }
        }
    } else {
        Ok(false)
    }
}
