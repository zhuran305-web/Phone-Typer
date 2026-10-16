use crate::server::http::WEB_EMBEDDED;
use std::path::{Path, PathBuf};
use tracing::warn;
use tray_icon::Icon;
use windows::core::{HSTRING, PCWSTR};
use windows::Win32::Foundation::{HINSTANCE, HWND};
use windows::Win32::UI::Input::KeyboardAndMouse::SetFocus;
use windows::Win32::UI::WindowsAndMessaging::{
    BringWindowToTop, IsIconic, LoadIconW, LoadImageW, SetWindowPos, ShowWindow, HICON,
    IDI_APPLICATION, IMAGE_ICON, LR_DEFAULTSIZE, LR_LOADFROMFILE, SW_RESTORE, SWP_NOMOVE,
    SWP_NOSIZE, SWP_SHOWWINDOW,
};

pub fn exe_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
}

pub fn load_tray_icon() -> Option<Icon> {
    let path = exe_dir().join("web").join("icons").join("icon-256.png");
    if let Some(icon) = load_png_icon(&path) {
        return Some(icon);
    }
    if let Some(file) = WEB_EMBEDDED.get_file("icons/icon-256.png") {
        if let Some(icon) = png_bytes_to_icon(file.contents()) {
            return Some(icon);
        }
    }
    warn!("托盘图标三级加载均失败，使用代码生成占位图标");
    make_placeholder_icon()
}

fn load_png_icon(path: &Path) -> Option<Icon> {
    let bytes = std::fs::read(path).ok()?;
    png_bytes_to_icon(&bytes)
}

fn png_bytes_to_icon(bytes: &[u8]) -> Option<Icon> {
    let img = image::load_from_memory_with_format(bytes, image::ImageFormat::Png).ok()?;
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    Icon::from_rgba(rgba.into_raw(), w, h).ok()
}

fn make_placeholder_icon() -> Option<Icon> {
    let size = 32u32;
    let mut rgba = Vec::with_capacity((size * size * 4) as usize);
    for _ in 0..(size * size) {
        rgba.extend_from_slice(&[0x6A, 0x5A, 0xE8, 0xFF]);
    }
    Icon::from_rgba(rgba, size, size).ok()
}

pub fn load_window_hicon() -> Option<HICON> {
    let path = exe_dir().join("web").join("icons").join("icon.ico");
    if let Some(hicon) = load_ico_hicon(&path) {
        return Some(hicon);
    }
    if let Some(file) = WEB_EMBEDDED.get_file("icons/icon.ico") {
        if let Some(hicon) = ico_bytes_to_hicon(file.contents()) {
            return Some(hicon);
        }
    }
    warn!("窗口图标加载失败，使用系统默认图标");
    unsafe { LoadIconW(None, IDI_APPLICATION).ok() }
}

fn load_ico_hicon(path: &Path) -> Option<HICON> {
    let path_str = HSTRING::from(path);
    unsafe {
        let handle = LoadImageW(
            None,
            PCWSTR::from_raw(path_str.as_ptr()),
            IMAGE_ICON,
            0,
            0,
            LR_LOADFROMFILE | LR_DEFAULTSIZE,
        )
        .ok()?;
        Some(HICON(handle.0))
    }
}

fn ico_bytes_to_hicon(bytes: &[u8]) -> Option<HICON> {
    let tmp = std::env::temp_dir().join("phone-typer-rust-icon.ico");
    std::fs::write(&tmp, bytes).ok()?;
    let hicon = load_ico_hicon(&tmp);
    let _ = std::fs::remove_file(&tmp);
    hicon
}

pub fn bring_to_front(hwnd: HWND) {
    unsafe {
        if IsIconic(hwnd).as_bool() {
            let _ = ShowWindow(hwnd, SW_RESTORE);
        }
        let _ = BringWindowToTop(hwnd);
        let topmost = HWND(-1isize as *mut core::ffi::c_void);
        let _ = SetWindowPos(
            hwnd,
            Some(topmost),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW,
        );
        let _ = SetFocus(Some(hwnd));
    }
}

#[allow(unused)]
fn _unused(_: HINSTANCE) {}
