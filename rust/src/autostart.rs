use std::env;
use winreg::enums::{HKEY_CURRENT_USER, KEY_READ, KEY_SET_VALUE, KEY_QUERY_VALUE};
use winreg::RegKey;

pub const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
pub const APP_NAME: &str = "PhoneTyperRust";

pub fn set_autostart(enabled: bool) -> std::io::Result<()> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key = hkcu.open_subkey_with_flags(RUN_KEY, KEY_SET_VALUE)?;
    if enabled {
        let exe = env::current_exe()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        let value = format!("\"{exe}\" --autostart");
        key.set_value(APP_NAME, &value)
    } else {
        key.delete_value(APP_NAME).or_else(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                Ok(())
            } else {
                Err(e)
            }
        })
    }
}

pub fn is_autostart_enabled() -> bool {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key = match hkcu.open_subkey_with_flags(RUN_KEY, KEY_READ | KEY_QUERY_VALUE) {
        Ok(k) => k,
        Err(_) => return false,
    };
    key.get_value::<String, _>(APP_NAME).is_ok()
}
