use crate::config::SharedConfig;
use crate::diff::{diff_sync, key_commands};
use crate::hub::WsHub;
use crate::keys::KeyCommand;
use futures_util::{SinkExt, StreamExt};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::mpsc as std_mpsc;
use std::time::Duration;
use tokio_tungstenite::tungstenite::Message;
use tracing::{error, info, warn};

const TEXT_LIMIT: usize = 1_000_000;
const PIN_FAIL_LIMIT: u32 = 5;

pub enum MsgOutcome {
    Reply(String),
    ReplyDelayClose(String),
}

pub fn process_ws_message(
    raw: &str,
    shared_cfg: &SharedConfig,
    last_text: &mut String,
    pin_failures: &mut u32,
    typist_tx: &std_mpsc::Sender<KeyCommand>,
    _smoke: bool,
) -> MsgOutcome {
    let msg: serde_json::Value = match serde_json::from_str(raw) {
        Ok(v) => v,
        Err(_) => return MsgOutcome::Reply(json_err("bad-json")),
    };
    let obj = match msg.as_object() {
        Some(o) => o,
        None => return MsgOutcome::Reply(json_err("bad-json")),
    };

    let pin = pin_from_msg(obj);
    let current_pin = { shared_cfg.read().unwrap().pin.clone() };
    if pin != current_pin {
        *pin_failures += 1;
        if *pin_failures >= PIN_FAIL_LIMIT {
            return MsgOutcome::ReplyDelayClose(json_err("bad-pin"));
        }
        return MsgOutcome::Reply(json_err("bad-pin"));
    }
    *pin_failures = 0;

    let text = text_from_msg(obj);
    if text.chars().count() > TEXT_LIMIT {
        return MsgOutcome::Reply(json_err("too-long"));
    }

    let mtype = obj.get("type").and_then(|v| v.as_str()).unwrap_or("send");

    if mtype == "sync" {
        let d = diff_sync(last_text, &text);
        for cmd in key_commands(&d) {
            let _ = typist_tx.send(cmd);
        }
        *last_text = text;
        MsgOutcome::Reply(
            serde_json::json!({
                "ok": true,
                "bs": d.bs,
                "typed": d.insert.chars().count(),
                "s": d.suffix,
            })
            .to_string(),
        )
    } else {
        if !text.is_empty() {
            let _ = typist_tx.send(KeyCommand::Type { text });
        }
        *last_text = String::new();
        MsgOutcome::Reply(r#"{"ok":true}"#.to_string())
    }
}

fn json_err(err: &str) -> String {
    serde_json::json!({"ok": false, "err": err}).to_string()
}

fn pin_from_msg(obj: &serde_json::Map<String, serde_json::Value>) -> String {
    match obj.get("pin") {
        Some(serde_json::Value::String(s)) => s.clone(),
        Some(serde_json::Value::Number(n)) => n.to_string(),
        Some(serde_json::Value::Bool(b)) => b.to_string(),
        Some(serde_json::Value::Null) => String::new(),
        Some(v) => v.to_string(),
        None => String::new(),
    }
}

fn text_from_msg(obj: &serde_json::Map<String, serde_json::Value>) -> String {
    match obj.get("text") {
        Some(serde_json::Value::String(s)) => s.clone(),
        Some(serde_json::Value::Number(n)) => n.to_string(),
        Some(serde_json::Value::Bool(b)) => b.to_string(),
        Some(serde_json::Value::Null) => String::new(),
        Some(v) => v.to_string(),
        None => String::new(),
    }
}

pub async fn start_ws_server(
    ws_port: u16,
    shared_cfg: SharedConfig,
    hub: Arc<WsHub>,
    typist_tx: std_mpsc::Sender<KeyCommand>,
    smoke: bool,
    shutdown: Arc<AtomicBool>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let listener = match tokio::net::TcpListener::bind(("0.0.0.0", ws_port)).await {
        Ok(l) => l,
        Err(e) => {
            error!(error = %e, "WS 端口 {} 绑定失败", ws_port);
            eprintln!("[FATAL] WS 端口 {} 被占用：{}", ws_port, e);
            crate::server::fatal_dialog(&format!(
                "WS 端口 {} 被占用…请修改 exe 旁 config.json 后重试",
                ws_port
            ));
            std::process::exit(1);
        }
    };
    info!("WebSocket 监听 0.0.0.0:{}", ws_port);

    loop {
        if shutdown.load(Ordering::Relaxed) {
            return Ok(());
        }

        let (stream, peer) = tokio::select! {
            result = listener.accept() => result?,
            _ = tokio::time::sleep(Duration::from_millis(200)) => continue,
        };

        let cfg = shared_cfg.clone();
        let hub = hub.clone();
        let tx = typist_tx.clone();
        tokio::spawn(async move {
            handle_connection(stream, peer, cfg, hub, tx, smoke).await;
        });
    }
}

async fn handle_connection(
    stream: tokio::net::TcpStream,
    peer: std::net::SocketAddr,
    shared_cfg: SharedConfig,
    hub: Arc<WsHub>,
    typist_tx: std_mpsc::Sender<KeyCommand>,
    smoke: bool,
) {
    let ws = match tokio_tungstenite::accept_async(stream).await {
        Ok(ws) => ws,
        Err(e) => {
            warn!(error = %e, "WS 握手失败: {}", peer);
            return;
        }
    };

    info!("手机已连接: {}", peer);

    let (ws_sink, mut ws_stream) = ws.split();
    let (out_tx, mut out_rx) = tokio::sync::mpsc::unbounded_channel::<String>();
    let conn_id = hub.register(out_tx.clone());

    let write_task = tokio::spawn(async move {
        let mut ws_sink = ws_sink;
        while let Some(msg) = out_rx.recv().await {
            match tokio::time::timeout(Duration::from_secs(5), ws_sink.send(Message::text(msg)))
                .await
            {
                Ok(Ok(())) => {}
                Ok(Err(_)) => break,
                Err(_) => {
                    tracing::warn!("WS 发送超时（5s），断开该连接");
                    break;
                }
            }
        }
    });

    let mut last_text = String::new();
    let mut pin_failures: u32 = 0;

    while let Some(msg_result) = ws_stream.next().await {
        let text = match msg_result {
            Ok(Message::Text(t)) => t.as_str().to_string(),
            Ok(Message::Close(_)) => break,
            Ok(_) => continue,
            Err(e) => {
                warn!(error = %e, "连接异常: {}", peer);
                break;
            }
        };

        match process_ws_message(
            &text,
            &shared_cfg,
            &mut last_text,
            &mut pin_failures,
            &typist_tx,
            smoke,
        ) {
            MsgOutcome::Reply(resp) => {
                if out_tx.send(resp).is_err() {
                    break;
                }
            }
            MsgOutcome::ReplyDelayClose(resp) => {
                let _ = out_tx.send(resp);
                tokio::time::sleep(Duration::from_secs(1)).await;
                break;
            }
        }
    }

    hub.unregister(conn_id);
    write_task.abort();
    info!("手机断开: {}", peer);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use std::sync::{Arc, RwLock};

    fn make_cfg(pin: &str) -> SharedConfig {
        Arc::new(RwLock::new(Config {
            pin: pin.to_string(),
            ..Config::default()
        }))
    }

    fn make_typist() -> (std_mpsc::Sender<KeyCommand>, std_mpsc::Receiver<KeyCommand>) {
        std_mpsc::channel()
    }

    #[test]
    fn test_bad_json() {
        let cfg = make_cfg("1234");
        let (tx, _rx) = make_typist();
        let mut last_text = String::new();
        let mut pin_failures = 0;
        match process_ws_message("not json", &cfg, &mut last_text, &mut pin_failures, &tx, false) {
            MsgOutcome::Reply(s) => assert!(s.contains("bad-json")),
            _ => panic!("expected Reply"),
        }
    }

    #[test]
    fn test_non_object_json() {
        let cfg = make_cfg("1234");
        let (tx, _rx) = make_typist();
        let mut last_text = String::new();
        let mut pin_failures = 0;
        match process_ws_message("[1,2,3]", &cfg, &mut last_text, &mut pin_failures, &tx, false) {
            MsgOutcome::Reply(s) => assert!(s.contains("bad-json")),
            _ => panic!("expected Reply"),
        }
    }

    #[test]
    fn test_bad_pin_counting_and_limit() {
        let cfg = make_cfg("1234");
        let (tx, _rx) = make_typist();
        let mut last_text = String::new();
        let mut pin_failures = 0;
        for _ in 0..4 {
            match process_ws_message(
                r#"{"pin":"wrong","text":"hi"}"#,
                &cfg,
                &mut last_text,
                &mut pin_failures,
                &tx,
                false,
            ) {
                MsgOutcome::Reply(s) => assert!(s.contains("bad-pin")),
                _ => panic!("expected Reply"),
            }
        }
        assert_eq!(pin_failures, 4);
        match process_ws_message(
            r#"{"pin":"wrong","text":"hi"}"#,
            &cfg,
            &mut last_text,
            &mut pin_failures,
            &tx,
            false,
        ) {
            MsgOutcome::ReplyDelayClose(s) => assert!(s.contains("bad-pin")),
            _ => panic!("expected ReplyDelayClose on 5th failure"),
        }
    }

    #[test]
    fn test_pin_reset_on_success() {
        let cfg = make_cfg("1234");
        let (tx, _rx) = make_typist();
        let mut last_text = String::new();
        let mut pin_failures = 0;
        for _ in 0..2 {
            let _ = process_ws_message(
                r#"{"pin":"wrong"}"#,
                &cfg,
                &mut last_text,
                &mut pin_failures,
                &tx,
                false,
            );
        }
        assert_eq!(pin_failures, 2);
        match process_ws_message(
            r#"{"pin":"1234","text":"hello","type":"send"}"#,
            &cfg,
            &mut last_text,
            &mut pin_failures,
            &tx,
            false,
        ) {
            MsgOutcome::Reply(s) => assert!(s.contains(r#""ok":true"#)),
            _ => panic!("expected Reply"),
        }
        assert_eq!(pin_failures, 0);
    }

    #[test]
    fn test_too_long() {
        let cfg = make_cfg("1234");
        let (tx, _rx) = make_typist();
        let mut last_text = String::new();
        let mut pin_failures = 0;
        let long_text = "x".repeat(TEXT_LIMIT + 1);
        let msg = serde_json::json!({"pin":"1234","text":long_text}).to_string();
        match process_ws_message(&msg, &cfg, &mut last_text, &mut pin_failures, &tx, false) {
            MsgOutcome::Reply(s) => assert!(s.contains("too-long")),
            _ => panic!("expected Reply"),
        }
    }

    #[test]
    fn test_sync_response_fields() {
        let cfg = make_cfg("1234");
        let (tx, _rx) = make_typist();
        let mut last_text = String::new();
        let mut pin_failures = 0;
        match process_ws_message(
            r#"{"pin":"1234","text":"hello","type":"sync"}"#,
            &cfg,
            &mut last_text,
            &mut pin_failures,
            &tx,
            false,
        ) {
            MsgOutcome::Reply(s) => {
                let v: serde_json::Value = serde_json::from_str(&s).unwrap();
                assert_eq!(v["ok"], true);
                assert!(v["bs"].is_number());
                assert!(v["typed"].is_number());
                assert!(v["s"].is_number());
            }
            _ => panic!("expected Reply"),
        }
    }

    #[test]
    fn test_send_clears_baseline() {
        let cfg = make_cfg("1234");
        let (tx, _rx) = make_typist();
        let mut last_text = "previous".to_string();
        let mut pin_failures = 0;
        match process_ws_message(
            r#"{"pin":"1234","text":"hello","type":"send"}"#,
            &cfg,
            &mut last_text,
            &mut pin_failures,
            &tx,
            false,
        ) {
            MsgOutcome::Reply(s) => assert!(s.contains(r#""ok":true"#)),
            _ => panic!("expected Reply"),
        }
        assert!(last_text.is_empty(), "send 应清空基线");
    }

    #[test]
    fn test_pin_numeric_string_compatible() {
        let cfg = make_cfg("1234");
        let (tx, _rx) = make_typist();
        let mut last_text = String::new();
        let mut pin_failures = 0;
        let msg = serde_json::json!({"pin":1234,"text":"hi","type":"send"}).to_string();
        match process_ws_message(&msg, &cfg, &mut last_text, &mut pin_failures, &tx, false) {
            MsgOutcome::Reply(s) => assert!(
                s.contains(r#""ok":true"#),
                "数字 PIN 应与字符串 PIN 等价"
            ),
            _ => panic!("expected Reply"),
        }
    }

    #[test]
    fn test_processing_order_bad_json_before_pin() {
        let cfg = make_cfg("1234");
        let (tx, _rx) = make_typist();
        let mut last_text = String::new();
        let mut pin_failures = 0;
        match process_ws_message(
            r#"not json but has pin"#,
            &cfg,
            &mut last_text,
            &mut pin_failures,
            &tx,
            false,
        ) {
            MsgOutcome::Reply(s) => assert!(s.contains("bad-json"), "bad-json 应先于 PIN 校验"),
            _ => panic!("expected Reply"),
        }
        assert_eq!(pin_failures, 0, "bad-json 不应增加 PIN 失败计数");
    }

    #[test]
    fn test_empty_text_send_allowed() {
        let cfg = make_cfg("1234");
        let (tx, _rx) = make_typist();
        let mut last_text = "baseline".to_string();
        let mut pin_failures = 0;
        match process_ws_message(
            r#"{"pin":"1234","text":"","type":"send"}"#,
            &cfg,
            &mut last_text,
            &mut pin_failures,
            &tx,
            false,
        ) {
            MsgOutcome::Reply(s) => assert!(s.contains(r#""ok":true"#)),
            _ => panic!("expected Reply"),
        }
        assert!(last_text.is_empty(), "空文本 send 仍应清空基线");
    }
}
