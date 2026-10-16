use std::sync::{Mutex, OnceLock};

#[derive(Debug)]
pub enum ClipError {
    Access(String),
}

impl std::fmt::Display for ClipError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClipError::Access(m) => write!(f, "剪贴板访问失败: {m}"),
        }
    }
}

pub struct Clipboard {
    inner: Option<arboard::Clipboard>,
}

impl Clipboard {
    fn new() -> Self {
        Self { inner: None }
    }

    fn ensure(&mut self) -> Result<&mut arboard::Clipboard, ClipError> {
        if self.inner.is_none() {
            self.inner = Some(
                arboard::Clipboard::new()
                    .map_err(|e| ClipError::Access(e.to_string()))?,
            );
        }
        Ok(self.inner.as_mut().unwrap())
    }

    pub fn get_text(&mut self) -> Result<String, ClipError> {
        self.ensure()?
            .get_text()
            .map_err(|e| ClipError::Access(e.to_string()))
    }

    pub fn set_text(&mut self, text: &str) -> Result<(), ClipError> {
        self.ensure()?
            .set_text(text.to_string())
            .map_err(|e| ClipError::Access(e.to_string()))
    }
}

static SHARED: OnceLock<Mutex<Clipboard>> = OnceLock::new();

pub fn shared() -> std::sync::MutexGuard<'static, Clipboard> {
    SHARED
        .get_or_init(|| Mutex::new(Clipboard::new()))
        .lock()
        .expect("clipboard poisoned")
}
