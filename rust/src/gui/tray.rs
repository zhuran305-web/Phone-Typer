use crate::gui::res;
use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{MouseButton, TrayIcon, TrayIconBuilder, TrayIconEvent};
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{PostMessageW, WM_APP};

pub const WM_APP_MENU: u32 = WM_APP + 1;

const ID_SHOW_MAIN: &str = "show_main";
const ID_COPY_ADDR: &str = "copy_addr";
const ID_SEND_CLIP: &str = "send_clip";
const ID_SETTINGS: &str = "settings";
const ID_RESTART: &str = "restart";
const ID_EXIT: &str = "exit";

#[repr(usize)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TrayAction {
    ShowMain = 0,
    CopyAddr = 1,
    SendClip = 2,
    Settings = 3,
    Restart = 4,
    Exit = 5,
}

impl TrayAction {
    fn from_menu_id(id: &str) -> Option<Self> {
        match id {
            ID_SHOW_MAIN => Some(Self::ShowMain),
            ID_COPY_ADDR => Some(Self::CopyAddr),
            ID_SEND_CLIP => Some(Self::SendClip),
            ID_SETTINGS => Some(Self::Settings),
            ID_RESTART => Some(Self::Restart),
            ID_EXIT => Some(Self::Exit),
            _ => None,
        }
    }
}

pub struct TrayManager {
    _tray: TrayIcon,
}

impl TrayManager {
    pub fn create(hwnd: HWND) -> Option<Self> {
        let icon = res::load_tray_icon()?;

        let show_main = MenuItem::with_id(ID_SHOW_MAIN, "显示主界面", true, None);
        let copy_addr = MenuItem::with_id(ID_COPY_ADDR, "复制访问地址", true, None);
        let send_clip = MenuItem::with_id(ID_SEND_CLIP, "发送剪贴板到手机", true, None);
        let settings = MenuItem::with_id(ID_SETTINGS, "设置", true, None);
        let restart = MenuItem::with_id(ID_RESTART, "重启服务", true, None);
        let exit = MenuItem::with_id(ID_EXIT, "退出", true, None);

        let menu = Menu::new();
        menu.append(&show_main).ok()?;
        menu.append(&copy_addr).ok()?;
        menu.append(&send_clip).ok()?;
        menu.append(&settings).ok()?;
        menu.append(&restart).ok()?;
        menu.append(&PredefinedMenuItem::separator()).ok()?;
        menu.append(&exit).ok()?;

        let tooltip = format!("Phone-Typer v{}", crate::version::APP_VERSION);

        let tray = TrayIconBuilder::new()
            .with_icon(icon)
            .with_tooltip(tooltip)
            .with_menu(Box::new(menu))
            .build()
            .ok()?;

        start_menu_event_bridge(hwnd);
        start_icon_event_bridge(hwnd);

        Some(Self { _tray: tray })
    }
}

fn start_menu_event_bridge(hwnd: HWND) {
    let hwnd_raw = hwnd.0 as isize;
    std::thread::Builder::new()
        .name("pt-menu-events".into())
        .spawn(move || {
            let hwnd = HWND(hwnd_raw as *mut core::ffi::c_void);
            let receiver = MenuEvent::receiver();
            while let Ok(event) = receiver.recv() {
                let id = event.id.0.clone();
                if let Some(action) = TrayAction::from_menu_id(&id) {
                    unsafe {
                        let _ = PostMessageW(
                            Some(hwnd),
                            WM_APP_MENU,
                            WPARAM(action as usize),
                            LPARAM(0),
                        );
                    }
                }
            }
        })
        .expect("spawn menu event bridge");
}

fn start_icon_event_bridge(hwnd: HWND) {
    let hwnd_raw = hwnd.0 as isize;
    std::thread::Builder::new()
        .name("pt-tray-events".into())
        .spawn(move || {
            let hwnd = HWND(hwnd_raw as *mut core::ffi::c_void);
            let receiver = TrayIconEvent::receiver();
            while let Ok(event) = receiver.recv() {
                if let TrayIconEvent::Click {
                    button: MouseButton::Left,
                    ..
                } = event
                {
                    unsafe {
                        let _ = PostMessageW(
                            Some(hwnd),
                            WM_APP_MENU,
                            WPARAM(TrayAction::ShowMain as usize),
                            LPARAM(0),
                        );
                    }
                }
            }
        })
        .expect("spawn tray icon event bridge");
}
