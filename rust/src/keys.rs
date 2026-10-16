use std::sync::mpsc;
use tracing::info;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, VIRTUAL_KEY, VK_BACK,
    VK_CONTROL, VK_LEFT, VK_RETURN, VK_RIGHT,
};

#[derive(Debug, Clone)]
pub enum KeyCommand {
    Type { text: String },
    Backspace { n: i64 },
    MoveLeft { n: i64 },
    MoveRight { n: i64 },
}

pub struct TypistConfig {
    pub paste_delay_ms: u64,
    pub auto_enter: bool,
    pub restore_clipboard: bool,
    pub smoke: bool,
}

pub fn spawn_typist(cfg: TypistConfig) -> mpsc::Sender<KeyCommand> {
    let (tx, rx) = mpsc::channel::<KeyCommand>();
    std::thread::Builder::new()
        .name("pt-typist".into())
        .spawn(move || typist_loop(&cfg, rx))
        .expect("spawn typist thread");
    tx
}

fn typist_loop(cfg: &TypistConfig, rx: mpsc::Receiver<KeyCommand>) {
    while let Ok(cmd) = rx.recv() {
        match cmd {
            KeyCommand::Type { text } => type_text(cfg, &text),
            KeyCommand::Backspace { n } => tap_key(cfg, VK_BACK, n),
            KeyCommand::MoveLeft { n } => tap_key(cfg, VK_LEFT, n),
            KeyCommand::MoveRight { n } => tap_key(cfg, VK_RIGHT, n),
        }
    }
}

fn normalize_newlines(text: &str) -> String {
    let lf = text.replace("\r\n", "\n").replace('\r', "\n");
    lf.replace('\n', "\r\n")
}

fn preview(text: &str) -> String {
    let first: String = text.chars().take(30).collect();
    first.replace('\r', "\\r").replace('\n', "\\n")
}

fn type_text(cfg: &TypistConfig, raw: &str) {
    let text = normalize_newlines(raw);
    if cfg.smoke {
        info!("[SMOKE] 跳过真实键入 {} 字（预览: {}）", text.chars().count(), preview(&text));
        return;
    }
    let saved = {
        let mut clip = crate::clipboard::shared();
        let old = clip.get_text().ok();
        if clip.set_text(&text).is_err() {
            tracing::warn!("写系统剪贴板失败，本次键入中止");
            return;
        }
        old
    };
    if cfg.paste_delay_ms > 0 {
        std::thread::sleep(std::time::Duration::from_millis(cfg.paste_delay_ms));
    }
    send_key(VK_CONTROL, false);
    send_key(VIRTUAL_KEY(b'V' as u16), false);
    send_key(VIRTUAL_KEY(b'V' as u16), true);
    send_key(VK_CONTROL, true);
    if cfg.auto_enter {
        std::thread::sleep(std::time::Duration::from_millis(50));
        send_key(VK_RETURN, false);
        send_key(VK_RETURN, true);
    }
    if cfg.restore_clipboard {
        std::thread::sleep(std::time::Duration::from_millis(150));
        if let Some(old) = saved {
            let mut clip = crate::clipboard::shared();
            let _ = clip.set_text(&old);
        }
    }
    info!("键入完成（{} 字，预览: {}）", text.chars().count(), preview(&text));
}

fn tap_key(cfg: &TypistConfig, vk: VIRTUAL_KEY, n: i64) {
    let n = n.clamp(0, 10_000);
    if cfg.smoke {
        info!("[SMOKE] 跳过按键 {} 次", n);
        return;
    }
    for _ in 0..n {
        send_key(vk, false);
        send_key(vk, true);
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}

fn send_key(vk: VIRTUAL_KEY, up: bool) {
    let flags = if up { KEYEVENTF_KEYUP } else { KEYBD_EVENT_FLAGS(0) };
    let input = INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: 0,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };
    unsafe {
        SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
    }
}

use windows::Win32::UI::Input::KeyboardAndMouse::KEYBD_EVENT_FLAGS;
