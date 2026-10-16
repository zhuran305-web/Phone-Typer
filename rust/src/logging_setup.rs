use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};

pub const MAX_LOG_SIZE: u64 = 512 * 1024;
pub const MAX_BACKUPS: u32 = 2;

pub struct RotatingFile {
    path: PathBuf,
    file: Option<File>,
}

impl RotatingFile {
    pub fn new(path: PathBuf) -> Self {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        Self { path, file: None }
    }

    fn ensure_open(&mut self) -> io::Result<&mut File> {
        if self.file.is_none() {
            self.file = Some(
                OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&self.path)?,
            );
        }
        Ok(self.file.as_mut().unwrap())
    }

    fn rotate(&mut self) -> io::Result<()> {
        self.file = None;
        let newest = self.path.with_extension(format!(
            "{}.{}",
            self.path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("log"),
            MAX_BACKUPS
        ));
        let _ = fs::remove_file(&newest);
        for n in (1..MAX_BACKUPS).rev() {
            let from = self.path.with_extension(format!(
                "{}.{}",
                self.path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("log"),
                n
            ));
            let to = self.path.with_extension(format!(
                "{}.{}",
                self.path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("log"),
                n + 1
            ));
            let _ = fs::rename(&from, &to);
        }
        let first = self.path.with_extension(format!(
            "{}.1",
            self.path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("log")
        ));
        let _ = fs::rename(&self.path, &first);
        self.file = Some(
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.path)?,
        );
        Ok(())
    }
}

impl Write for RotatingFile {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let need_rotate = {
            let len = match self.file.as_ref() {
                Some(f) => f.metadata().map(|m| m.len()).unwrap_or(MAX_LOG_SIZE),
                None => MAX_LOG_SIZE,
            };
            len + buf.len() as u64 > MAX_LOG_SIZE
        };
        if need_rotate {
            let _ = self.rotate();
        }
        self.ensure_open()?.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        if let Some(f) = self.file.as_mut() {
            f.flush()?;
        }
        Ok(())
    }
}

#[derive(Clone)]
pub struct RotatingMaker(Arc<Mutex<RotatingFile>>);

pub struct GuardWriter<'a>(std::sync::MutexGuard<'a, RotatingFile>);

impl Write for GuardWriter<'_> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for RotatingMaker {
    type Writer = GuardWriter<'a>;

    fn make_writer(&'a self) -> Self::Writer {
        GuardWriter(self.0.lock().expect("log writer poisoned"))
    }
}

pub fn log_dir() -> PathBuf {
    let mut dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."));
    dir.push("logs");
    dir
}

pub fn log_file_path() -> PathBuf {
    let mut dir = log_dir();
    dir.push("phone-typer.log");
    dir
}

static ROTATING: OnceLock<Arc<Mutex<RotatingFile>>> = OnceLock::new();

pub fn init_logging(console: bool) {
    let rotating = ROTATING.get_or_init(|| Arc::new(Mutex::new(RotatingFile::new(log_file_path()))));

    use tracing_subscriber::prelude::*;
    let maker = RotatingMaker(rotating.clone());
    let file_layer = tracing_subscriber::fmt::layer()
        .with_ansi(false)
        .with_writer(maker);
    if console {
        let stdout_layer = tracing_subscriber::fmt::layer().with_writer(io::stdout);
        tracing_subscriber::registry()
            .with(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
            .with(file_layer)
            .with(stdout_layer)
            .init();
    } else {
        tracing_subscriber::registry()
            .with(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
            .with(file_layer)
            .init();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rotation_triggers_and_keeps_two_backups() {
        let dir = std::env::temp_dir().join(format!("pt-log-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let path = dir.join("test.log");
        let mut rf = RotatingFile::new(path.clone());
        let chunk = "x".repeat(64 * 1024);
        for _ in 0..10 {
            rf.write_all(chunk.as_bytes()).unwrap();
        }
        rf.flush().unwrap();
        assert!(path.exists(), "当前日志文件应存在");
        assert!(path.metadata().unwrap().len() <= MAX_LOG_SIZE + 64 * 1024);
        let backups: Vec<_> = (1..=MAX_BACKUPS)
            .map(|n| dir.join(format!("test.log.{n}")))
            .filter(|p| p.exists())
            .collect();
        assert!(!backups.is_empty(), "至少存在一个备份文件");
        for n in 1..=MAX_BACKUPS {
            assert!(
                !dir.join(format!("test.log.{}", MAX_BACKUPS + n)).exists(),
                "备份不应超过 2 个"
            );
        }
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_write_continues_after_rotation() {
        let dir = std::env::temp_dir().join(format!("pt-log-test2-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let path = dir.join("t.log");
        let mut rf = RotatingFile::new(path.clone());
        let chunk = "y".repeat(128 * 1024);
        for _ in 0..6 {
            rf.write_all(chunk.as_bytes()).unwrap();
        }
        assert!(rf.write_all(b"final-line").is_ok(), "轮转后写入不中断");
        rf.flush().unwrap();
        let _ = fs::remove_dir_all(&dir);
    }
}
