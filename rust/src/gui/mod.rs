pub mod main_win;
pub mod res;
pub mod settings;
pub mod tray;

use crate::config::{save_config, SharedConfig};
use crate::hub::WsHub;
use crate::net::access_url;
use crate::server::ServiceRunner;
use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use tracing::{info, warn};
use windows::core::{HSTRING, PCWSTR};
use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, MessageBoxW,
    RegisterClassExW, HWND_MESSAGE, MB_ICONINFORMATION, MB_ICONWARNING,
    MESSAGEBOX_STYLE, MSG, WINDOW_EX_STYLE, WINDOW_STYLE, WNDCLASSEXW,
};

pub struct AppContext {
    pub cfg: SharedConfig,
    pub hub: Arc<WsHub>,
    pub runner: ServiceRunner,
}

static CTX: OnceLock<Arc<AppContext>> = OnceLock::new();
thread_local! {
    static TRAY: RefCell<Option<tray::TrayManager>> = const { RefCell::new(None) };
}
static MAIN_HWND: Mutex<Option<isize>> = Mutex::new(None);
pub(super) static SETTINGS_HWND: Mutex<Option<isize>> = Mutex::new(None);
static CLIP_SENDING: AtomicBool = AtomicBool::new(false);

pub fn run_gui(ctx: Arc<AppContext>, show_main: bool) -> ! {
    let _ = CTX.set(ctx);

    let hub_hwnd = create_hub_window();
    crate::instance::start_show_main_listener(hub_hwnd);

    match tray::TrayManager::create(hub_hwnd) {
        Some(t) => TRAY.with(|cell| *cell.borrow_mut() = Some(t)),
        None => warn!("托盘创建失败，仅后台运行"),
    }

    if show_main {
        action_show_main();
    }

    info!("GUI 消息循环启动");

    unsafe {
        let mut msg = MSG::default();
        loop {
            let r = GetMessageW(&mut msg, None, 0, 0);
            if r.0 <= 0 {
                break;
            }
            let _ = DispatchMessageW(&msg);
        }
    }
    std::process::exit(0);
}

fn create_hub_window() -> HWND {
    unsafe {
        let hmodule = GetModuleHandleW(None).expect("GetModuleHandleW");
        let hinstance = HINSTANCE(hmodule.0);
        let class_name = HSTRING::from("PhoneTyperHubWin");
        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            lpfnWndProc: Some(hub_wndproc),
            hInstance: hinstance,
            lpszClassName: PCWSTR::from_raw(class_name.as_ptr()),
            ..Default::default()
        };
        RegisterClassExW(&wc);
        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            PCWSTR::from_raw(class_name.as_ptr()),
            PCWSTR::null(),
            WINDOW_STYLE(0),
            0,
            0,
            0,
            0,
            Some(HWND_MESSAGE),
            None,
            Some(hinstance),
            None,
        )
        .expect("create hub window")
    }
}

unsafe extern "system" fn hub_wndproc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        tray::WM_APP_MENU => {
            let action = match wparam.0 {
                0 => tray::TrayAction::ShowMain,
                1 => tray::TrayAction::CopyAddr,
                2 => tray::TrayAction::SendClip,
                3 => tray::TrayAction::Settings,
                4 => tray::TrayAction::Restart,
                5 => tray::TrayAction::Exit,
                _ => return LRESULT(0),
            };
            handle_tray_action(action);
            LRESULT(0)
        }
        crate::instance::WM_APP_RAISE => {
            action_show_main();
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

fn handle_tray_action(action: tray::TrayAction) {
    match action {
        tray::TrayAction::ShowMain => action_show_main(),
        tray::TrayAction::CopyAddr => action_copy_address(),
        tray::TrayAction::SendClip => action_send_clipboard(),
        tray::TrayAction::Settings => action_show_settings(),
        tray::TrayAction::Restart => action_restart(),
        tray::TrayAction::Exit => action_exit(),
    }
}

pub fn action_show_main() {
    let snapshot = *MAIN_HWND.lock().unwrap();
    if let Some(raw) = snapshot {
        if raw != 0 {
            let hwnd = HWND(raw as *mut core::ffi::c_void);
            res::bring_to_front(hwnd);
            return;
        }
    }
    match CTX.get() {
        Some(ctx) => match main_win::create_main_window(ctx) {
            Some(hwnd) => *MAIN_HWND.lock().unwrap() = Some(hwnd.0 as isize),
            None => warn!("主界面创建失败"),
        },
        None => warn!("GUI 上下文未初始化，无法打开主界面"),
    }
}

pub fn action_show_settings() {
    settings::show_or_create_settings();
}

fn action_copy_address() {
    let Some(ctx) = CTX.get() else { return };
    let url = {
        let cfg = ctx.cfg.read().unwrap();
        access_url(&cfg)
    };
    let mut clip = crate::clipboard::shared();
    match clip.set_text(&url) {
        Ok(()) => info!("访问地址已复制：{url}"),
        Err(e) => warn!(error = %e, "复制访问地址失败"),
    }
}

pub fn action_send_clipboard() {
    let Some(ctx) = CTX.get() else { return };
    if CLIP_SENDING
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return;
    }
    let result = (|| -> Result<(usize, String), String> {
        let mut clip = crate::clipboard::shared();
        let text = clip.get_text().map_err(|e| e.to_string())?;
        if text.trim().is_empty() {
            return Err("EMPTY".to_string());
        }
        let count = ctx.hub.broadcast_clip(&text);
        Ok((count, text))
    })();
    CLIP_SENDING.store(false, Ordering::SeqCst);
    match result {
        Ok((count, text)) if count > 0 => {
            let preview: String = text
                .chars()
                .take(50)
                .collect::<String>()
                .replace('\r', "\\r")
                .replace('\n', "\\n");
            let msg = format!("已推送到 {count} 台手机\n\n预览：{preview}");
            info!("clip 推送完成，{count} 台");
            message_box(&msg, MB_ICONINFORMATION);
        }
        Ok(_) => message_box("剪贴板为空，或没有手机连接", MB_ICONINFORMATION),
        Err(e) if e == "EMPTY" => {
            message_box("剪贴板为空，或没有手机连接", MB_ICONINFORMATION);
        }
        Err(e) => {
            warn!(error = %e, "读取剪贴板失败");
            message_box("读取剪贴板失败", MB_ICONWARNING);
        }
    }
}

fn action_restart() {
    let Some(ctx) = CTX.get() else { return };
    info!("用户触发服务重启");
    ctx.runner.restart();
}

fn action_exit() {
    info!("用户选择退出");
    TRAY.with(|cell| *cell.borrow_mut() = None);
    if let Some(ctx) = CTX.get() {
        ctx.runner.stop();
    }
    std::process::exit(0);
}

pub fn clear_main_hwnd() {
    *MAIN_HWND.lock().unwrap() = None;
}

pub fn clear_settings_hwnd() {
    *SETTINGS_HWND.lock().unwrap() = None;
}

pub fn current_pin() -> String {
    match CTX.get() {
        Some(ctx) => ctx.cfg.read().unwrap().pin.clone(),
        None => crate::config::load_config().pin,
    }
}

pub fn save_pin(pin: &str) -> std::io::Result<()> {
    let Some(ctx) = CTX.get() else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "GUI 上下文未初始化",
        ));
    };
    let new_cfg = {
        let cfg = ctx.cfg.read().unwrap();
        let mut c = cfg.clone();
        c.pin = pin.to_string();
        c
    };
    save_config(&new_cfg)?;
    *ctx.cfg.write().unwrap() = new_cfg;
    info!("PIN 已更新并热生效");
    Ok(())
}

pub fn message_box(text: &str, style: MESSAGEBOX_STYLE) {
    unsafe {
        let t = HSTRING::from(text);
        let title = HSTRING::from("Phone-Typer");
        let _ = MessageBoxW(
            None,
            PCWSTR::from_raw(t.as_ptr()),
            PCWSTR::from_raw(title.as_ptr()),
            style,
        );
    }
}
