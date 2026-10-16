mod autostart;
mod clipboard;
mod config;
mod diff;
mod gui;
mod hub;
mod instance;
mod keys;
mod logging_setup;
mod net;
mod qr;
mod server;
mod smoke;
mod version;

use crate::config::SharedConfig;
use crate::hub::WsHub;
use crate::keys::{spawn_typist, TypistConfig};
use crate::server::ServiceRunner;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

fn free_console() {
    use windows::Win32::System::Console::FreeConsole;
    unsafe {
        let _ = FreeConsole();
    }
}

fn main() {
    instance::enable_dpi_awareness();

    let args: Vec<String> = std::env::args().skip(1).collect();
    let mode = args.first().map(|s| s.as_str()).unwrap_or("").to_string();

    let console_output = mode == "--console" || mode == "--smoke";
    if !console_output {
        free_console();
    }
    logging_setup::init_logging(console_output);

    let shared = config::shared_config();

    match mode.as_str() {
        "--smoke" => std::process::exit(smoke::run_smoke(shared)),
        "--console" => run_console(shared),
        _ => run_gui_mode(shared, mode == "--autostart"),
    }
}

fn run_gui_mode(shared: SharedConfig, autostart: bool) {
    if !instance::acquire_single_instance() {
        if !autostart {
            if instance::signal_existing_instance() {
                tracing::info!("已唤起运行中的实例");
            } else {
                tracing::warn!("唤起已有实例失败");
            }
        }
        std::process::exit(0);
    }

    let hub = WsHub::new();
    let typist_tx = spawn_typist(typist_from(&shared, false));

    let runner = ServiceRunner::new();
    let ctx = Arc::new(gui::AppContext {
        cfg: shared.clone(),
        hub: hub.clone(),
        runner,
    });

    ctx.runner.start(server::StartConfig {
        shared_cfg: shared.clone(),
        hub,
        typist_tx,
        smoke: false,
    });

    gui::run_gui(ctx, !autostart);
}

fn run_console(shared: SharedConfig) {
    let (http_port, ws_port, url, pin) = {
        let c = shared.read().unwrap();
        (c.http_port, c.ws_port, net::access_url(&c), c.pin.clone())
    };

    println!("========================================");
    println!("Phone-Typer v{}", version::APP_VERSION);
    println!("访问地址: {url}");
    println!("WS 端口:  {ws_port}");
    println!("PIN:      {pin}");
    println!("Ctrl+C 停止");
    println!("========================================");
    println!();
    qr::print_ascii(&url);
    println!();

    let hub = WsHub::new();
    let typist_tx = spawn_typist(typist_from(&shared, false));

    let shutdown = Arc::new(AtomicBool::new(false));

    let http_cfg = shared.clone();
    let http_shutdown = shutdown.clone();
    std::thread::spawn(move || {
        if let Err(e) = server::start_http_server(http_port, http_cfg, http_shutdown) {
            eprintln!("[FATAL] HTTP 端口 {} 被占用：{}", http_port, e);
            eprintln!("请修改 exe 旁 config.json 后重试");
            std::process::exit(1);
        }
    });

    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("build runtime");
    rt.block_on(async {
        if let Err(e) =
            server::start_ws_server(ws_port, shared.clone(), hub, typist_tx, false, shutdown).await
        {
            eprintln!("[FATAL] WS 端口 {} 被占用：{}", ws_port, e);
            eprintln!("请修改 exe 旁 config.json 后重试");
            std::process::exit(1);
        }
    });
}

fn typist_from(shared: &SharedConfig, smoke: bool) -> TypistConfig {
    let c = shared.read().unwrap();
    TypistConfig {
        paste_delay_ms: c.paste_delay_ms,
        auto_enter: c.auto_enter,
        restore_clipboard: c.restore_clipboard,
        smoke,
    }
}
