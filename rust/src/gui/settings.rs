use super::{clear_settings_hwnd, current_pin, res, save_pin, SETTINGS_HWND};
use crate::autostart;
use tracing::{error, info};
use windows::core::{HSTRING, PCWSTR};
use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::Graphics::Gdi::{GetStockObject, DEFAULT_GUI_FONT};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Controls::{BST_CHECKED, BST_UNCHECKED, EM_SETSEL};
use windows::Win32::UI::Input::KeyboardAndMouse::SetFocus;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, GetDlgItem, GetWindowTextW, LoadCursorW,
    MessageBoxW, RegisterClassExW, SendMessageW, SetWindowLongPtrW, ShowWindow, BM_GETCHECK,
    BM_SETCHECK, BS_AUTOCHECKBOX, BS_DEFPUSHBUTTON, ES_AUTOHSCROLL, GWLP_USERDATA, HMENU,
    IDC_ARROW, MB_ICONERROR, MB_ICONINFORMATION, MB_ICONWARNING, MESSAGEBOX_STYLE,
    SW_SHOWNORMAL, WM_CLOSE, WM_COMMAND, WM_DESTROY, WM_GETTEXTLENGTH, WM_SETFONT,
    WINDOW_EX_STYLE, WINDOW_STYLE, WNDCLASSEXW, WS_BORDER, WS_CAPTION, WS_CHILD,
    WS_OVERLAPPED, WS_SYSMENU, WS_TABSTOP, WS_VISIBLE,
};

const CLASS_NAME: &str = "PhoneTyperSettingsWin";
const ID_PIN_EDIT: i32 = 3001;
const ID_AUTOSTART: i32 = 3002;
const ID_SAVE: i32 = 3003;

pub fn show_or_create_settings() {
    let snapshot = *SETTINGS_HWND.lock().unwrap();
    if let Some(raw) = snapshot {
        if raw != 0 {
            let hwnd = HWND(raw as *mut core::ffi::c_void);
            res::bring_to_front(hwnd);
            return;
        }
    }
    create_settings_window();
}

fn create_settings_window() {
    unsafe {
        let hmodule = GetModuleHandleW(None).unwrap_or_default();
        let hinstance = HINSTANCE(hmodule.0);
        let class_name = HSTRING::from(CLASS_NAME);

        let hicon = res::load_window_hicon();
        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            lpfnWndProc: Some(settings_wndproc),
            hInstance: hinstance,
            hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
            hIcon: hicon.unwrap_or_default(),
            hIconSm: hicon.unwrap_or_default(),
            lpszClassName: PCWSTR::from_raw(class_name.as_ptr()),
            ..Default::default()
        };
        RegisterClassExW(&wc);

        let (sw, sh) = (360i32, 240i32);
        let title = HSTRING::from("设置");
        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE(0),
            PCWSTR::from_raw(class_name.as_ptr()),
            PCWSTR::from_raw(title.as_ptr()),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU,
            100,
            100,
            sw,
            sh,
            None,
            None,
            Some(hinstance),
            None,
        )
        .ok();

        let Some(hwnd) = hwnd else {
            clear_settings_hwnd();
            return;
        };

        SetWindowLongPtrW(hwnd, GWLP_USERDATA, 1);

        let font = GetStockObject(DEFAULT_GUI_FONT);

        let mk = |class: &str,
                      text: &str,
                      style: WINDOW_STYLE,
                      x: i32,
                      y: i32,
                      w: i32,
                      h: i32,
                      id: i32|
         -> HWND {
            let cls = HSTRING::from(class);
            let txt = HSTRING::from(text);
            let ctrl = CreateWindowExW(
                WINDOW_EX_STYLE(0),
                PCWSTR::from_raw(cls.as_ptr()),
                PCWSTR::from_raw(txt.as_ptr()),
                WS_CHILD | WS_VISIBLE | style,
                x,
                y,
                w,
                h,
                Some(hwnd),
                Some(HMENU(id as isize as *mut core::ffi::c_void)),
                Some(hinstance),
                None,
            )
            .unwrap_or_default();
            SendMessageW(ctrl, WM_SETFONT, Some(WPARAM(font.0 as usize)), Some(LPARAM(1)));
            ctrl
        };

        mk(
            "STATIC",
            "访问 PIN（手机连接时输入）",
            WINDOW_STYLE(0),
            20,
            18,
            320,
            18,
            0,
        );
        let pin = current_pin();
        let edit = mk(
            "EDIT",
            &pin,
            WINDOW_STYLE(ES_AUTOHSCROLL as u32 | WS_BORDER.0 | WS_TABSTOP.0),
            20,
            40,
            320,
            26,
            ID_PIN_EDIT,
        );

        let autostart_on = autostart::is_autostart_enabled();
        let check = mk(
            "BUTTON",
            "开机自动启动",
            WINDOW_STYLE(WS_TABSTOP.0 | BS_AUTOCHECKBOX as u32),
            20,
            80,
            320,
            24,
            ID_AUTOSTART,
        );
        SendMessageW(
            check,
            BM_SETCHECK,
            Some(WPARAM(if autostart_on {
                BST_CHECKED.0 as usize
            } else {
                BST_UNCHECKED.0 as usize
            })),
            Some(LPARAM(0)),
        );

        mk(
            "BUTTON",
            "保存",
            WINDOW_STYLE(WS_TABSTOP.0 | BS_DEFPUSHBUTTON as u32),
            20,
            120,
            320,
            34,
            ID_SAVE,
        );

        *SETTINGS_HWND.lock().unwrap() = Some(hwnd.0 as isize);

        let _ = ShowWindow(hwnd, SW_SHOWNORMAL);
        let len = SendMessageW(edit, WM_GETTEXTLENGTH, Some(WPARAM(0)), Some(LPARAM(0))).0;
        SendMessageW(edit, EM_SETSEL, Some(WPARAM(len as usize)), Some(LPARAM(len)));
        let _ = SetFocus(Some(edit));
    }
}

unsafe extern "system" fn settings_wndproc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_COMMAND => {
            let id = (wparam.0 & 0xFFFF) as i32;
            let code = ((wparam.0 >> 16) & 0xFFFF) as u16;
            match (id, code) {
                (ID_SAVE, _) => on_save(hwnd),
                (ID_AUTOSTART, 0) => on_toggle_autostart(hwnd),
                _ => {}
            }
            LRESULT(0)
        }
        WM_CLOSE => {
            let _ = DestroyWindow(hwnd);
            LRESULT(0)
        }
        WM_DESTROY => {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            clear_settings_hwnd();
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

fn on_save(hwnd: HWND) {
    unsafe {
        let edit = GetDlgItem(Some(hwnd), ID_PIN_EDIT).unwrap_or_default();
        let mut buf = [0u16; 256];
        let len = GetWindowTextW(edit, &mut buf);
        let pin = String::from_utf16_lossy(&buf[..len as usize]);

        if pin.trim().is_empty() {
            message_box(hwnd, "PIN 不能为空", MB_ICONWARNING);
            return;
        }

        match save_pin(&pin) {
            Ok(()) => {
                info!("PIN 已更新（立即生效）");
                message_box(hwnd, "已保存（立即生效，无需重启）", MB_ICONINFORMATION);
                let _ = DestroyWindow(hwnd);
            }
            Err(e) => {
                error!(error = %e, "PIN 保存失败");
                message_box(hwnd, "保存失败", MB_ICONERROR);
            }
        }
    }
}

fn on_toggle_autostart(hwnd: HWND) {
    unsafe {
        let check = GetDlgItem(Some(hwnd), ID_AUTOSTART).unwrap_or_default();
        let prev = autostart::is_autostart_enabled();
        let checked = SendMessageW(check, BM_GETCHECK, Some(WPARAM(0)), Some(LPARAM(0))).0 != 0;
        if checked == prev {
            return;
        }
        match autostart::set_autostart(checked) {
            Ok(()) => {
                info!("开机自启状态变更为 {}", if checked { "开启" } else { "关闭" });
            }
            Err(_) => {
                SendMessageW(
                    check,
                    BM_SETCHECK,
                    Some(WPARAM(if prev {
                        BST_CHECKED.0 as usize
                    } else {
                        BST_UNCHECKED.0 as usize
                    })),
                    Some(LPARAM(0)),
                );
                message_box(hwnd, "设置开机自启失败，请检查系统权限", MB_ICONERROR);
            }
        }
    }
}

fn message_box(parent: HWND, text: &str, style: MESSAGEBOX_STYLE) {
    unsafe {
        let t = HSTRING::from(text);
        let title = HSTRING::from("Phone-Typer");
        let _ = MessageBoxW(
            Some(parent),
            PCWSTR::from_raw(t.as_ptr()),
            PCWSTR::from_raw(title.as_ptr()),
            style,
        );
    }
}
