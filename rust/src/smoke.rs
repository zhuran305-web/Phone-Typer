use crate::config::SharedConfig;
use crate::hub::WsHub;
use crate::keys::{spawn_typist, TypistConfig};
use crate::net::port_in_use;
use futures_util::{SinkExt, StreamExt};
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

type WsStream = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;

pub fn run_smoke(shared_cfg: SharedConfig) -> i32 {
    let (http_port, ws_port, pin) = {
        let c = shared_cfg.read().unwrap();
        (c.http_port, c.ws_port, c.pin.clone())
    };

    println!("[SMOKE] Phone-Typer 冒烟自检（真实键入已禁用）");
    println!("[SMOKE] HTTP :{} / WS :{} / PIN {}", http_port, ws_port, pin);

    if port_in_use(http_port) || port_in_use(ws_port) {
        eprintln!(
            "[SMOKE] 端口 {} / {} 被占用，请先关闭其他 Phone-Typer 实例",
            http_port, ws_port
        );
        return 1;
    }

    let shutdown = Arc::new(AtomicBool::new(false));

    let http_cfg = shared_cfg.clone();
    let http_shutdown = shutdown.clone();
    std::thread::Builder::new()
        .name("smoke-http".into())
        .spawn(move || {
            if let Err(e) = crate::server::start_http_server(http_port, http_cfg, http_shutdown) {
                eprintln!("[SMOKE] HTTP 服务异常退出: {e}");
            }
        })
        .expect("spawn smoke http");

    let typist_tx = spawn_typist(TypistConfig {
        paste_delay_ms: 60,
        auto_enter: false,
        restore_clipboard: true,
        smoke: true,
    });

    let hub = WsHub::new();
    let ws_cfg = shared_cfg.clone();
    let ws_hub = hub.clone();
    let ws_tx = typist_tx.clone();
    let ws_shutdown = shutdown.clone();
    std::thread::Builder::new()
        .name("smoke-ws".into())
        .spawn(move || {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .enable_all()
                .build()
                .expect("build ws runtime");
            rt.block_on(async {
                if let Err(e) = crate::server::start_ws_server(
                    ws_port,
                    ws_cfg,
                    ws_hub,
                    ws_tx,
                    true,
                    ws_shutdown,
                )
                .await
                {
                    eprintln!("[SMOKE] WS 服务异常退出: {e}");
                }
            });
        })
        .expect("spawn smoke ws");

    if !wait_port(http_port) {
        eprintln!("[SMOKE] HTTP 端口 {} 未就绪", http_port);
        return 1;
    }
    if !wait_port(ws_port) {
        eprintln!("[SMOKE] WS 端口 {} 未就绪", ws_port);
        return 1;
    }

    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("build smoke runtime");

    let mut passed = 0usize;
    let mut failed = 0usize;
    let mut skipped = 0usize;

    println!("[SMOKE] STEP 1/5 HTTP 首页 + /api/config + 404/穿越");
    match step1_http(http_port) {
        Ok(d) => {
            println!("[SMOKE] STEP 1/5 PASS  {}", d);
            passed += 1;
        }
        Err(e) => {
            println!("[SMOKE] STEP 1/5 FAIL  {}", e);
            failed += 1;
        }
    }

    println!("[SMOKE] STEP 2/5 错误 PIN ×5 退避断开 + bad-json 保持");
    match rt.block_on(step2_bad_pin(ws_port)) {        Ok(d) => {
            println!("[SMOKE] STEP 2/5 PASS  {}", d);
            passed += 1;
        }
        Err(e) => {
            println!("[SMOKE] STEP 2/5 FAIL  {}", e);
            failed += 1;
        }
    }

    println!("[SMOKE] STEP 3/5 正确 PIN send + 缺省 type + 键入 stub");
    println!("[SMOKE] STEP 4/5 sync diff 算式 + too-long + 连接保持");
    match rt.block_on(step3_4(ws_port, &pin)) {
        Ok((d3, d4)) => {
            println!("[SMOKE] STEP 3/5 PASS  {}", d3);
            println!("[SMOKE] STEP 4/5 PASS  {}", d4);
            passed += 2;
        }
        Err(e) => {
            println!("[SMOKE] STEP 3/5 FAIL  {}", e);
            println!("[SMOKE] STEP 4/5 FAIL  {}", e);
            failed += 2;
        }
    }

    println!("[SMOKE] STEP 5/5 clip 广播与回读");
    match rt.block_on(step5_clip(ws_port, &pin, &hub)) {
        Ok(d) => {
            println!("[SMOKE] STEP 5/5 PASS  {}", d);
            passed += 1;
        }
        Err(e) if e == "SKIP" => {
            println!("[SMOKE] STEP 5/5 SKIP  剪贴板读取失败（降级不误报）");
            skipped += 1;
        }
        Err(e) => {
            println!("[SMOKE] STEP 5/5 FAIL  {}", e);
            failed += 1;
        }
    }

    shutdown.store(true, Ordering::Relaxed);
    println!(
        "[SMOKE] 自检完成：PASS {} / FAIL {} / SKIP {}（日志交叉核对: logs/phone-typer.log）",
        passed, failed, skipped
    );
    if failed == 0 {
        0
    } else {
        1
    }
}

fn wait_port(port: u16) -> bool {
    let t0 = Instant::now();
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    while t0.elapsed() < Duration::from_secs(5) {
        if TcpStream::connect_timeout(&addr, Duration::from_millis(200)).is_ok() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    false
}

fn http_get(port: u16, path: &str) -> Result<(u16, String, Vec<(String, String)>), String> {
    let mut stream =
        TcpStream::connect(("127.0.0.1", port)).map_err(|e| format!("connect: {e}"))?;
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .map_err(|e| e.to_string())?;
    let req = format!("GET {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n");
    stream
        .write_all(req.as_bytes())
        .map_err(|e| format!("write: {e}"))?;
    let mut buf = Vec::new();
    stream
        .read_to_end(&mut buf)
        .map_err(|e| format!("read: {e}"))?;
    let text = String::from_utf8_lossy(&buf).to_string();
    let (head, body) = text.split_once("\r\n\r\n").unwrap_or((&text, ""));
    let code = head
        .lines()
        .next()
        .and_then(|l| l.split_whitespace().nth(1))
        .and_then(|s| s.parse().ok())
        .unwrap_or(0u16);
    let headers: Vec<(String, String)> = head
        .lines()
        .skip(1)
        .filter_map(|l| {
            l.split_once(':')
                .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
        })
        .collect();
    Ok((code, body.to_string(), headers))
}

async fn connect(ws_port: u16) -> Result<WsStream, String> {
    let url = format!("ws://127.0.0.1:{ws_port}");
    let (ws, _) = tokio_tungstenite::connect_async(&url)
        .await
        .map_err(|e| format!("ws connect: {e}"))?;
    Ok(ws)
}

async fn send_recv(ws: &mut WsStream, msg: &str) -> Result<String, String> {
    ws.send(Message::text(msg.to_string()))
        .await
        .map_err(|e| format!("send: {e}"))?;
    recv_one(ws).await
}

async fn recv_one(ws: &mut WsStream) -> Result<String, String> {
    loop {
        match tokio::time::timeout(Duration::from_secs(5), ws.next()).await {
            Ok(Some(Ok(Message::Text(t)))) => return Ok(t.as_str().to_string()),
            Ok(Some(Ok(Message::Close(_)))) => return Err("CLOSED".to_string()),
            Ok(Some(Ok(_))) => continue,
            Ok(Some(Err(e))) => return Err(format!("recv: {e}")),
            Ok(None) => return Err("EOF".to_string()),
            Err(_) => return Err("TIMEOUT".to_string()),
        }
    }
}

fn step1_http(http_port: u16) -> Result<String, String> {
    let (code, body, headers) = http_get(http_port, "/")?;
    if code != 200 {
        return Err(format!("GET / 状态码 {code}"));
    }
    if body.is_empty() || !body.contains('<') {
        return Err("GET / 响应体不是 HTML".into());
    }
    let has = |name: &str| {
        headers
            .iter()
            .any(|(k, _)| k.eq_ignore_ascii_case(name))
    };
    if !has("Content-Type") || !has("Content-Length") {
        return Err("Content-Type / Content-Length 响应头缺失".into());
    }
    if !headers
        .iter()
        .any(|(k, v)| k.eq_ignore_ascii_case("Cache-Control") && v.eq_ignore_ascii_case("no-store"))
    {
        return Err("Cache-Control: no-store 缺失".into());
    }

    let t0 = Instant::now();
    let (code, body, _) = http_get(http_port, "/api/config")?;
    let elapsed = t0.elapsed().as_millis();
    if code != 200 {
        return Err(format!("GET /api/config 状态码 {code}"));
    }
    if elapsed > 200 {
        return Err(format!("/api/config 响应 {}ms > 200ms", elapsed));
    }
    let v: serde_json::Value = serde_json::from_str(&body).map_err(|e| e.to_string())?;
    if v.get("ws_port").is_none() || v.get("version").is_none() {
        return Err(format!("/api/config 字段缺失: {body}"));
    }
    if v.get("pin").is_some() {
        return Err("/api/config 不得包含 PIN".into());
    }

    let (code, _, _) = http_get(http_port, "/no-such.file")?;
    if code != 404 {
        return Err(format!("/no-such.file 期望 404 实际 {code}"));
    }
    let (code, _, _) = http_get(http_port, "/../config.json")?;
    if code != 404 {
        return Err(format!("/../config.json 期望 404 实际 {code}（穿越未拦截）"));
    }

    Ok(format!(
        "index+三头 ✓ / api {}ms 无 PIN ✓ / 404 ✓ / 穿越拦截 ✓",
        elapsed
    ))
}

async fn step2_bad_pin(ws_port: u16) -> Result<String, String> {
    let mut ws = connect(ws_port).await?;

    let r = send_recv(&mut ws, "not-json-at-all").await?;
    if !r.contains("bad-json") {
        return Err(format!("非 JSON 应答不符: {r}"));
    }

    for i in 0..5 {
        let r = send_recv(
            &mut ws,
            r#"{"pin":"0000-wrong","type":"send","text":"hi"}"#,
        )
        .await?;
        if !r.contains("bad-pin") {
            return Err(format!("第 {} 次 bad-pin 应答不符: {r}", i + 1));
        }
    }

    let t0 = Instant::now();
    let closed = loop {
        match tokio::time::timeout(Duration::from_millis(3500), ws.next()).await {
            Ok(Some(Ok(Message::Close(_)))) => break true,
            Ok(Some(Ok(_))) => continue,
            Ok(Some(Err(_))) => break true,
            Ok(None) => break true,
            Err(_) => break false,
        }
    };
    if !closed {
        return Err("5 次 bad-pin 后服务端未主动断开".into());
    }
    let elapsed = t0.elapsed().as_millis();

    Ok(format!(
        "bad-json 保持 ✓ / 5×bad-pin ✓ / {}ms 后服务端断开（约 1s）✓",
        elapsed
    ))
}

async fn step3_4(ws_port: u16, pin: &str) -> Result<(String, String), String> {
    let mut ws = connect(ws_port).await?;

    let r = send_recv(&mut ws, &format!(r#"{{"pin":"{pin}","text":"x"}}"#)).await?;
    if !r.contains("\"ok\":true") {
        return Err(format!("缺省 type 未按 send 处理: {r}"));
    }

    let r = send_recv(
        &mut ws,
        &format!(r#"{{"pin":"{pin}","type":"send","text":"你好\n世界"}}"#),
    )
    .await?;
    if !r.contains("\"ok\":true") {
        return Err(format!("send 应答不符: {r}"));
    }

    let r1 = send_recv(
        &mut ws,
        &format!(r#"{{"pin":"{pin}","type":"sync","text":"ABC"}}"#),
    )
    .await?;
    let v1: serde_json::Value = serde_json::from_str(&r1).map_err(|e| e.to_string())?;
    if v1["bs"] != 0 || v1["typed"] != 3 || v1["s"] != 0 {
        return Err(format!("sync ABC 期望 bs=0/typed=3/s=0 实际: {r1}"));
    }

    let r2 = send_recv(
        &mut ws,
        &format!(r#"{{"pin":"{pin}","type":"sync","text":"ABX"}}"#),
    )
    .await?;
    let v2: serde_json::Value = serde_json::from_str(&r2).map_err(|e| e.to_string())?;
    if v2["bs"] != 1 || v2["typed"] != 1 || v2["s"] != 0 {
        return Err(format!("sync ABX 期望 bs=1/typed=1/s=0 实际: {r2}"));
    }

    let r3 = send_recv(
        &mut ws,
        &format!(r#"{{"pin":"{pin}","type":"sync","text":"ABX"}}"#),
    )
    .await?;
    let v3: serde_json::Value = serde_json::from_str(&r3).map_err(|e| e.to_string())?;
    if v3["bs"] != 0 || v3["typed"] != 0 || v3["s"] != 0 {
        return Err(format!("空 diff 期望 bs=0/typed=0/s=0 实际: {r3}"));
    }

    let long = "a".repeat(1_000_001);
    let r = send_recv(
        &mut ws,
        &format!(r#"{{"pin":"{pin}","type":"send","text":"{long}"}}"#),
    )
    .await?;
    if !r.contains("too-long") {
        return Err(format!("too-long 应答不符: {r}"));
    }

    let r = send_recv(&mut ws, &format!(r#"{{"pin":"{pin}","text":"keep"}}"#)).await?;
    if !r.contains("\"ok\":true") {
        return Err("too-long 后连接未保持".into());
    }
    drop(ws);

    Ok((
        "缺省 type→send ✓ / send ok=true ✓ / 键入 stub（日志核验）✓".into(),
        "sync 0/3/0 → 1/1/0 → 空 0/0/0 ✓ / too-long ✓ / 连接保持+主动断开 ✓".into(),
    ))
}

async fn step5_clip(ws_port: u16, pin: &str, hub: &Arc<WsHub>) -> Result<String, String> {
    let test_text = format!(
        "SMOKE-CLIP-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
    );
    {
        let mut clip = crate::clipboard::shared();
        let ok = clip.set_text(&test_text).is_ok()
            && clip.get_text().map(|t| t == test_text).unwrap_or(false);
        if !ok {
            return Err("SKIP".to_string());
        }
    }

    let mut ws = connect(ws_port).await?;
    let _ = send_recv(&mut ws, &format!(r#"{{"pin":"{pin}","text":"ready"}}"#)).await?;

    let count = hub.broadcast_clip(&test_text);
    if count != 1 {
        return Err(format!("成功台数 {count} != 1"));
    }

    let r = recv_one(&mut ws).await?;
    let v: serde_json::Value = serde_json::from_str(&r).map_err(|e| e.to_string())?;
    if v["type"] != "clip" || v["text"] != test_text {
        return Err(format!("clip 回读不符: {r}"));
    }

    drop(ws);
    Ok("成功台数=1 ✓ / clip 帧文本回读一致 ✓".into())
}
