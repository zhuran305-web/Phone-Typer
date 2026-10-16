use super::{action_send_clipboard, action_show_settings, AppContext};
use crate::net::{access_url, pair_uri};
use crate::qr;
use crate::version::APP_VERSION;
use std::sync::Arc;
use windows::core::{HSTRING, PCWSTR};
use windows::Win32::Foundation::{COLORREF, HINSTANCE, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, CreateCompatibleBitmap, CreateCompatibleDC, CreateSolidBrush, DeleteDC,
    DeleteObject, DrawTextW, EndPaint, FillRect, GetStockObject, InvalidateRect,
    SelectObject, StretchBlt, UpdateWindow, DEFAULT_GUI_FONT, DT_CENTER, DT_SINGLELINE,
    DT_VCENTER, HDC, HGDIOBJ, PAINTSTRUCT, SRCCOPY,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Controls::{
    NMHDR, TCIF_TEXT, TCITEMW, TCN_SELCHANGE, TCM_GETCURSEL, TCM_INSERTITEMW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, GetClientRect, GetSystemMetrics,
    GetWindowLongPtrW, LoadCursorW, RegisterClassExW, SendMessageW, SetForegroundWindow,
    SetWindowLongPtrW, ShowWindow, CS_HREDRAW, CS_VREDRAW, ES_AUTOHSCROLL, ES_READONLY,
    GWLP_USERDATA, HMENU, IDC_ARROW, SM_CXSCREEN, SM_CYSCREEN, SW_SHOWNORMAL, WM_CLOSE,
    WM_COMMAND, WM_DESTROY, WM_ERASEBKGND, WM_NOTIFY, WM_PAINT, WM_SETFONT, WINDOW_EX_STYLE,
    WINDOW_STYLE, WNDCLASSEXW, WS_BORDER, WS_CAPTION, WS_CHILD, WS_MINIMIZEBOX,
    WS_OVERLAPPED, WS_SYSMENU, WS_TABSTOP, WS_VISIBLE,
};

const CLASS_NAME: &str = "PhoneTyperMainWin";
const ID_TAB: i32 = 2001;
const ID_COPY_CLIP: i32 = 2002;
const ID_OPEN_SETTINGS: i32 = 2003;
const QR_CANVAS_CLASS: &str = "PhoneTyperQrCanvas";

const QR_AREA: RECT = RECT {
    left: 40,
    top: 144,
    right: 380,
    bottom: 376,
};

pub struct MainWindowState {
    pub tab: usize,
    pub qr_web: Option<qr::QrMatrix>,
    pub qr_app: Option<qr::QrMatrix>,
    pub qr_canvas: HWND,
}

pub fn create_main_window(ctx: &Arc<AppContext>) -> Option<HWND> {
    unsafe {
        let hmodule = GetModuleHandleW(None).ok()?;
        let hinstance = HINSTANCE(hmodule.0);
        let class_name = HSTRING::from(CLASS_NAME);

        let hicon = super::res::load_window_hicon();
        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(main_wndproc),
            hInstance: hinstance,
            hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
            hIcon: hicon.unwrap_or_default(),
            hIconSm: hicon.unwrap_or_default(),
            lpszClassName: PCWSTR::from_raw(class_name.as_ptr()),
            ..Default::default()
        };
        RegisterClassExW(&wc);

        let (qr_web, qr_app, url, info) = {
            let cfg = ctx.cfg.read().unwrap();
            (
                qr::make_matrix(&access_url(&cfg)),
                qr::make_matrix(&pair_uri(&cfg)),
                access_url(&cfg),
                format!("HTTP {} · WS {} · PIN {}", cfg.http_port, cfg.ws_port, cfg.pin),
            )
        };

        let (sw, sh) = (420i32, 580i32);
        let sx = GetSystemMetrics(SM_CXSCREEN);
        let sy = GetSystemMetrics(SM_CYSCREEN);
        let x = (sx - sw) / 2;
        let y = (sy - sh) / 2;

        let title = HSTRING::from(format!("Phone-Typer v{}", APP_VERSION));
        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE(0),
            PCWSTR::from_raw(class_name.as_ptr()),
            PCWSTR::from_raw(title.as_ptr()),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX,
            x,
            y,
            sw,
            sh,
            None,
            None,
            Some(hinstance),
            None,
        )
        .ok()?;

        let state = Box::new(MainWindowState {
            tab: 0,
            qr_web,
            qr_app,
            qr_canvas: HWND::default(),
        });
        let state_ptr = Box::into_raw(state);
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, state_ptr as isize);

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
            &format!("Phone-Typer 服务运行中（v{}）", APP_VERSION),
            WINDOW_STYLE(0),
            20,
            18,
            380,
            22,
            0,
        );
        mk("STATIC", "手机访问地址", WINDOW_STYLE(0), 20, 52, 380, 18, 0);
        mk(
            "EDIT",
            &url,
            WINDOW_STYLE(ES_READONLY as u32 | WS_BORDER.0 | ES_AUTOHSCROLL as u32),
            20,
            72,
            380,
            26,
            0,
        );

        let tab = mk("SysTabControl32", "", WS_TABSTOP, 20, 108, 380, 282, ID_TAB);
        let mut item = TCITEMW {
            mask: TCIF_TEXT,
            ..Default::default()
        };
        let t1 = HSTRING::from("网页版");
        item.pszText = windows::core::PWSTR(t1.as_ptr() as *mut u16);
        SendMessageW(
            tab,
            TCM_INSERTITEMW,
            Some(WPARAM(0)),
            Some(LPARAM(&item as *const TCITEMW as isize)),
        );
        let t2 = HSTRING::from("App 版");
        item.pszText = windows::core::PWSTR(t2.as_ptr() as *mut u16);
        SendMessageW(
            tab,
            TCM_INSERTITEMW,
            Some(WPARAM(1)),
            Some(LPARAM(&item as *const TCITEMW as isize)),
        );

        let qr_class_name = HSTRING::from(QR_CANVAS_CLASS);
        let qr_wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            lpfnWndProc: Some(qr_canvas_wndproc),
            hInstance: hinstance,
            hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
            lpszClassName: PCWSTR::from_raw(qr_class_name.as_ptr()),
            ..Default::default()
        };
        RegisterClassExW(&qr_wc);

        let qr_canvas = CreateWindowExW(
            WINDOW_EX_STYLE(0),
            PCWSTR::from_raw(qr_class_name.as_ptr()),
            PCWSTR::null(),
            WS_CHILD | WS_VISIBLE,
            QR_AREA.left,
            QR_AREA.top,
            QR_AREA.right - QR_AREA.left,
            QR_AREA.bottom - QR_AREA.top,
            Some(hwnd),
            None,
            Some(hinstance),
            None,
        )
        .unwrap_or_default();
        SetWindowLongPtrW(qr_canvas, GWLP_USERDATA, state_ptr as isize);
        (*state_ptr).qr_canvas = qr_canvas;

        mk("STATIC", &info, WINDOW_STYLE(0), 20, 400, 380, 20, 0);
        mk(
            "BUTTON",
            "复制剪贴板到手机",
            WS_TABSTOP,
            20,
            430,
            185,
            34,
            ID_COPY_CLIP,
        );
        mk(
            "BUTTON",
            "打开设置",
            WS_TABSTOP,
            215,
            430,
            185,
            34,
            ID_OPEN_SETTINGS,
        );
        mk(
            "STATIC",
            "手机与电脑处于同一局域网，手机浏览器打开上方地址，输入 PIN 后即可同步输入；App 版扫码需安装文台 App。",
            WINDOW_STYLE(0),
            20,
            476,
            380,
            90,
            0,
        );

        let _ = ShowWindow(hwnd, SW_SHOWNORMAL);
        let _ = SetForegroundWindow(hwnd);
        let _ = UpdateWindow(hwnd);
        Some(hwnd)
    }
}

unsafe extern "system" fn main_wndproc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_PAINT => {
            let mut ps = PAINTSTRUCT::default();
            let _ = BeginPaint(hwnd, &mut ps);
            let _ = EndPaint(hwnd, &ps);
            LRESULT(0)
        }
        WM_COMMAND => {
            let id = (wparam.0 & 0xFFFF) as i32;
            let code = ((wparam.0 >> 16) & 0xFFFF) as u16;
            if code == 0 {
                match id {
                    ID_COPY_CLIP => action_send_clipboard(),
                    ID_OPEN_SETTINGS => action_show_settings(),
                    _ => {}
                }
            }
            LRESULT(0)
        }
        WM_NOTIFY => {
            if lparam.0 != 0 {
                let nm = *(lparam.0 as *const NMHDR);
                if nm.code == TCN_SELCHANGE {
                    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
                    if ptr != 0 {
                        let state = &mut *(ptr as *mut MainWindowState);
                        state.tab = SendMessageW(
                            nm.hwndFrom,
                            TCM_GETCURSEL,
                            Some(WPARAM(0)),
                            Some(LPARAM(0)),
                        )
                        .0
                        .max(0) as usize;
                        let _ = InvalidateRect(Some(state.qr_canvas), None, true);
                    }
                }
            }
            LRESULT(0)
        }
        WM_CLOSE => {
            let _ = DestroyWindow(hwnd);
            LRESULT(0)
        }
        WM_DESTROY => {
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
            if ptr != 0 {
                drop(Box::from_raw(ptr as *mut MainWindowState));
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            }
            super::clear_main_hwnd();
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

unsafe extern "system" fn qr_canvas_wndproc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_ERASEBKGND => LRESULT(1),
        WM_PAINT => {
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
            let mut ps = PAINTSTRUCT::default();
            let hdc = BeginPaint(hwnd, &mut ps);
            if ptr != 0 {
                let state = &*(ptr as *const MainWindowState);
                let matrix = match state.tab {
                    1 => &state.qr_app,
                    _ => &state.qr_web,
                };
                let mut rect = RECT::default();
                let _ = GetClientRect(hwnd, &mut rect);
                draw_qr(hdc, &rect, matrix);
            }
            let _ = EndPaint(hwnd, &ps);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

unsafe fn draw_qr(hdc: HDC, area: &RECT, matrix: &Option<qr::QrMatrix>) {
    match matrix {
        Some(m) => {
            let size = m.size as i32;
            let tw = area.right - area.left;
            let th = area.bottom - area.top;
            let side = tw.min(th);
            let ox = area.left + (tw - side) / 2;
            let oy = area.top + (th - side) / 2;

            let mem_dc = CreateCompatibleDC(Some(hdc));
            let mem_bmp = CreateCompatibleBitmap(hdc, size, size);
            let old = SelectObject(mem_dc, HGDIOBJ(mem_bmp.0));

            let white = CreateSolidBrush(COLORREF(0x00FF_FFFF));
            let black = CreateSolidBrush(COLORREF(0x0028_1E1E));
            let full = RECT {
                left: 0,
                top: 0,
                right: size,
                bottom: size,
            };
            FillRect(mem_dc, &full, white);
            for y in 0..m.size {
                for x in 0..m.size {
                    if m.dark[y * m.size + x] {
                        let cell = RECT {
                            left: x as i32,
                            top: y as i32,
                            right: x as i32 + 1,
                            bottom: y as i32 + 1,
                        };
                        FillRect(mem_dc, &cell, black);
                    }
                }
            }
            let _ = StretchBlt(
                hdc,
                ox,
                oy,
                side,
                side,
                Some(mem_dc),
                0,
                0,
                size,
                size,
                SRCCOPY,
            );

            let _ = SelectObject(mem_dc, old);
            let _ = DeleteObject(HGDIOBJ(mem_bmp.0));
            let _ = DeleteDC(mem_dc);
            let _ = DeleteObject(HGDIOBJ(white.0));
            let _ = DeleteObject(HGDIOBJ(black.0));
        }
        None => {
            let white = CreateSolidBrush(COLORREF(0x00FF_FFFF));
            FillRect(hdc, area, white);
            let _ = DeleteObject(HGDIOBJ(white.0));
            let mut text_rect = *area;
            let mut label: Vec<u16> = "二维码生成失败".encode_utf16().collect();
            DrawTextW(
                hdc,
                &mut label,
                &mut text_rect,
                DT_CENTER | DT_VCENTER | DT_SINGLELINE,
            );
        }
    }
}
