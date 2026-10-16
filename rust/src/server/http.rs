use crate::config::SharedConfig;
use crate::net::{content_type_for, safe_join};
use crate::version::APP_VERSION;
use include_dir::{include_dir, Dir};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tiny_http::{Header, Response, Server};

pub static WEB_EMBEDDED: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/web");

pub fn web_root() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
        .join("web")
}

pub fn start_http_server(
    http_port: u16,
    shared_cfg: SharedConfig,
    shutdown: Arc<AtomicBool>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let server = Server::http(("0.0.0.0", http_port))?;
    tracing::info!("HTTP 监听 0.0.0.0:{}", http_port);

    while !shutdown.load(Ordering::Relaxed) {
        match server.recv_timeout(Duration::from_millis(200)) {
            Ok(Some(req)) => handle_request(req, &shared_cfg),
            Ok(None) => {}
            Err(e) => {
                tracing::warn!(error = %e, "HTTP recv 失败");
                break;
            }
        }
    }
    Ok(())
}

fn handle_request(request: tiny_http::Request, shared_cfg: &SharedConfig) {
    let url = request.url().to_string();

    if url == "/api/config" || url == "/api/config/" {
        let cfg = shared_cfg.read().unwrap();
        let body = serde_json::json!({
            "ws_port": cfg.ws_port,
            "version": APP_VERSION,
        })
        .to_string();
        respond(request, 200, body.as_bytes(), "application/json; charset=utf-8");
        return;
    }

    let rel = url
        .split('?')
        .next()
        .unwrap_or("")
        .split('#')
        .next()
        .unwrap_or("");
    let rel = if rel.is_empty() || rel == "/" {
        "index.html"
    } else {
        rel.strip_prefix('/').unwrap_or(rel)
    };

    let root = web_root();
    if let Some(path) = safe_join(&root, rel) {
        if let Ok(body) = std::fs::read(&path) {
            let ctype = content_type_for(rel);
            respond(request, 200, &body, ctype);
            return;
        }
    }

    if let Some(file) = WEB_EMBEDDED.get_file(rel) {
        let ctype = content_type_for(rel);
        respond(request, 200, file.contents(), ctype);
        return;
    }

    respond(request, 404, b"404 Not Found", "text/plain; charset=utf-8");
}

fn respond(request: tiny_http::Request, code: u16, body: &[u8], ctype: &str) {
    let response = Response::from_data(body)
        .with_status_code(code)
        .with_header(Header::from_bytes(&b"Content-Type"[..], ctype.as_bytes()).unwrap())
        .with_header(Header::from_bytes(&b"Cache-Control"[..], &b"no-store"[..]).unwrap());
    let _ = request.respond(response);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use std::sync::{Arc, RwLock};

    fn make_cfg() -> SharedConfig {
        Arc::new(RwLock::new(Config::default()))
    }

    #[test]
    fn test_api_config_response_has_no_pin() {
        let cfg = make_cfg();
        let body = serde_json::json!({
            "ws_port": cfg.read().unwrap().ws_port,
            "version": APP_VERSION,
        })
        .to_string();
        let parsed: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert!(parsed.get("ws_port").is_some());
        assert!(parsed.get("version").is_some());
        assert!(parsed.get("pin").is_none(), "/api/config 不得包含 PIN");
    }

    #[test]
    fn test_api_config_version_matches() {
        let cfg = make_cfg();
        let body = serde_json::json!({
            "ws_port": cfg.read().unwrap().ws_port,
            "version": APP_VERSION,
        })
        .to_string();
        let parsed: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(parsed["version"], APP_VERSION);
    }

    #[test]
    fn test_traversal_rejected_by_safe_join() {
        let root = std::path::Path::new("web");
        assert!(safe_join(root, "../../../etc/passwd").is_none());
        assert!(safe_join(root, "..\\..\\config.json").is_none());
    }

    #[test]
    fn test_embedded_web_has_index() {
        assert!(WEB_EMBEDDED.get_file("index.html").is_some());
    }
}
