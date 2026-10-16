pub mod http;
mod ws;

pub use http::start_http_server;
pub use ws::start_ws_server;

use crate::config::SharedConfig;
use crate::hub::WsHub;
use crate::keys::KeyCommand;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{mpsc as std_mpsc, Arc, Mutex};
use std::time::Duration;
use tracing::{error, info, warn};

#[derive(Debug, Clone, PartialEq)]
pub enum ServiceState {
    Stopped,
    Running,
    Restarting,
}

#[derive(Clone)]
pub struct StartConfig {
    pub shared_cfg: SharedConfig,
    pub hub: Arc<WsHub>,
    pub typist_tx: std_mpsc::Sender<KeyCommand>,
    pub smoke: bool,
}

pub struct ServiceRunner {
    state: Arc<Mutex<ServiceState>>,
    attempts: Arc<AtomicU32>,
    shutdown: Arc<AtomicBool>,
    handle: Arc<Mutex<Option<std::thread::JoinHandle<()>>>>,
    config: Arc<Mutex<Option<StartConfig>>>,
}

impl ServiceRunner {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(ServiceState::Stopped)),
            attempts: Arc::new(AtomicU32::new(0)),
            shutdown: Arc::new(AtomicBool::new(false)),
            handle: Arc::new(Mutex::new(None)),
            config: Arc::new(Mutex::new(None)),
        }
    }

    pub fn start(&self, cfg: StartConfig) {
        let mut handle = self.handle.lock().unwrap();
        if let Some(h) = handle.as_ref() {
            if !h.is_finished() {
                return;
            }
        }

        *self.config.lock().unwrap() = Some(cfg.clone());
        self.shutdown.store(false, Ordering::Relaxed);

        let state = self.state.clone();
        let attempts = self.attempts.clone();
        let shutdown = self.shutdown.clone();

        let h = std::thread::Builder::new()
            .name("pt-service".into())
            .spawn(move || service_loop(cfg, state, attempts, shutdown))
            .expect("spawn service thread");

        *handle = Some(h);
    }

    pub fn stop(&self) {
        self.shutdown.store(true, Ordering::Relaxed);
    }

    pub fn restart(&self) {
        self.attempts.store(0, Ordering::Relaxed);
        self.stop();
        {
            let mut handle = self.handle.lock().unwrap();
            if let Some(h) = handle.take() {
                let _ = h.join();
            }
        }
        if let Some(cfg) = self.config.lock().unwrap().clone() {
            self.start(cfg);
        }
    }

    #[allow(dead_code)]
    pub fn state(&self) -> ServiceState {
        self.state.lock().unwrap().clone()
    }
}

pub fn backoff_delay(attempts: u32) -> u64 {
    if attempts == 0 {
        return 0;
    }
    std::cmp::min(2u64.pow(attempts - 1), 16)
}

pub fn fatal_dialog(text: &str) {
    use windows::core::{HSTRING, PCWSTR};
    use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR};
    unsafe {
        let t = HSTRING::from(text);
        let title = HSTRING::from("Phone-Typer");
        let _ = MessageBoxW(
            None,
            PCWSTR::from_raw(t.as_ptr()),
            PCWSTR::from_raw(title.as_ptr()),
            MB_ICONERROR,
        );
    }
}

fn service_loop(
    cfg: StartConfig,
    state: Arc<Mutex<ServiceState>>,
    attempts: Arc<AtomicU32>,
    shutdown: Arc<AtomicBool>,
) {
    let (http_port, ws_port) = {
        let c = cfg.shared_cfg.read().unwrap();
        (c.http_port, c.ws_port)
    };

    let http_cfg = cfg.shared_cfg.clone();
    let http_shutdown = shutdown.clone();
    let _http_handle = std::thread::Builder::new()
        .name("pt-http".into())
        .spawn(move || {
            if let Err(e) = start_http_server(http_port, http_cfg, http_shutdown) {
                error!(error = %e, "HTTP 端口 {} 绑定失败", http_port);
                eprintln!("[FATAL] HTTP 端口 {} 被占用：{}", http_port, e);
                fatal_dialog(&format!(
                    "端口 {} 被占用…请修改 exe 旁 config.json 后重试",
                    http_port
                ));
                std::process::exit(1);
            }
        })
        .expect("spawn http thread");

    loop {
        if shutdown.load(Ordering::Relaxed) {
            *state.lock().unwrap() = ServiceState::Stopped;
            break;
        }

        *state.lock().unwrap() = ServiceState::Running;

        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("build tokio runtime");

        let ws_cfg = cfg.shared_cfg.clone();
        let ws_hub = cfg.hub.clone();
        let ws_tx = cfg.typist_tx.clone();
        let ws_shutdown = shutdown.clone();
        let ws_smoke = cfg.smoke;

        let ws_result = runtime.block_on(async {
            start_ws_server(ws_port, ws_cfg, ws_hub, ws_tx, ws_smoke, ws_shutdown).await
        });

        drop(runtime);

        if shutdown.load(Ordering::Relaxed) {
            *state.lock().unwrap() = ServiceState::Stopped;
            break;
        }

        match ws_result {
            Ok(()) => {
                *state.lock().unwrap() = ServiceState::Stopped;
                info!("服务正常停止");
                break;
            }
            Err(e) => {
                let n = attempts.fetch_add(1, Ordering::Relaxed) + 1;
                if n > 5 {
                    *state.lock().unwrap() = ServiceState::Stopped;
                    error!("服务重启超限（5 次），转 STOPPED；可从托盘菜单手动重启");
                    break;
                }
                let delay = backoff_delay(n);
                *state.lock().unwrap() = ServiceState::Restarting;
                warn!(error = %e, "服务异常 —— {}s 后第 {} 次重启", delay, n);
                std::thread::sleep(Duration::from_secs(delay));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backoff_sequence() {
        assert_eq!(backoff_delay(1), 1);
        assert_eq!(backoff_delay(2), 2);
        assert_eq!(backoff_delay(3), 4);
        assert_eq!(backoff_delay(4), 8);
        assert_eq!(backoff_delay(5), 16);
        assert_eq!(backoff_delay(6), 16);
        assert_eq!(backoff_delay(7), 16);
    }

    #[test]
    fn test_backoff_zero() {
        assert_eq!(backoff_delay(0), 0);
    }

    #[test]
    fn test_runner_initial_state() {
        let runner = ServiceRunner::new();
        assert_eq!(runner.state(), ServiceState::Stopped);
    }
}
