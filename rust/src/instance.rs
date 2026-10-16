use windows::core::HSTRING;
use windows::core::PWSTR;
use windows::Win32::Foundation::{
    CloseHandle, GetLastError, ERROR_ALREADY_EXISTS, HWND, LPARAM, WAIT_OBJECT_0, WPARAM,
};
use windows::Win32::System::Threading::{
    CreateEventW, CreateMutexW, OpenEventW, SetEvent, WaitForSingleObject, EVENT_MODIFY_STATE,
    INFINITE,
};
use windows::Win32::UI::HiDpi::SetProcessDpiAwareness;
use windows::Win32::UI::WindowsAndMessaging::SetProcessDPIAware;
use windows::Win32::UI::WindowsAndMessaging::{PostMessageW, WM_APP};

pub const MUTEX_NAME: &str = r"Local\PhoneTyperRust-SingleInstance-Mutex-v1";
pub const EVENT_NAME: &str = r"Local\PhoneTyperRust-ShowMain-Event-v1";
pub const WM_APP_RAISE: u32 = WM_APP + 2;

fn pwstr(s: &HSTRING) -> PWSTR {
    PWSTR(s.as_ptr() as *mut u16)
}

pub fn acquire_single_instance() -> bool {
    unsafe {
        let name = HSTRING::from(MUTEX_NAME);
        match CreateMutexW(None, false, pwstr(&name)) {
            Ok(_) => {
                let err = GetLastError();
                if err == ERROR_ALREADY_EXISTS {
                    tracing::info!("已有实例运行中（Mutex 已存在）");
                    return false;
                }
                true
            }
            Err(e) => {
                tracing::warn!(error = %e, "创建单实例 Mutex 失败，降级为不拦截多开");
                true
            }
        }
    }
}

pub fn signal_existing_instance() -> bool {
    unsafe {
        let name = HSTRING::from(EVENT_NAME);
        match OpenEventW(EVENT_MODIFY_STATE, false, pwstr(&name)) {
            Ok(h) => {
                let _ = SetEvent(h);
                let _ = CloseHandle(h);
                true
            }
            Err(_) => false,
        }
    }
}

pub fn start_show_main_listener(hwnd: HWND) {
    let hwnd_raw = hwnd.0 as isize;
    std::thread::Builder::new()
        .name("pt-raise-listener".into())
        .spawn(move || unsafe {
            let hwnd = HWND(hwnd_raw as *mut core::ffi::c_void);
            let name = HSTRING::from(EVENT_NAME);
            match CreateEventW(None, false, false, pwstr(&name)) {
                Ok(h) => loop {
                    match WaitForSingleObject(h, INFINITE) {
                        WAIT_OBJECT_0 => {
                            let _ =
                                PostMessageW(Some(hwnd), WM_APP_RAISE, WPARAM(0), LPARAM(0));
                        }
                        _ => break,
                    }
                },
                Err(e) => tracing::warn!(error = %e, "创建唤起 Event 失败，单实例唤起不可用"),
            }
        })
        .expect("spawn raise listener");
}

pub fn enable_dpi_awareness() {
    use windows::Win32::UI::HiDpi::PROCESS_PER_MONITOR_DPI_AWARE;
    unsafe {
        if SetProcessDpiAwareness(PROCESS_PER_MONITOR_DPI_AWARE).is_err() {
            let _ = SetProcessDPIAware();
        }
    }
}
