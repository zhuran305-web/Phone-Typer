use crate::config::Config;
use std::net::UdpSocket;
use std::path::{Component, Path, PathBuf};

pub fn get_lan_ip() -> String {
    let sock = UdpSocket::bind("0.0.0.0:0");
    match sock {
        Ok(s) => match s.connect("8.8.8.8:80").and_then(|_| s.local_addr()) {
            Ok(addr) if !addr.ip().is_loopback() => addr.ip().to_string(),
            _ => "127.0.0.1".to_string(),
        },
        Err(_) => "127.0.0.1".to_string(),
    }
}

pub fn safe_join(web_root: &Path, rel: &str) -> Option<PathBuf> {
    if rel.is_empty() {
        return None;
    }
    let p = Path::new(rel);
    if p.is_absolute() {
        return None;
    }
    let mut depth: i32 = 0;
    for comp in p.components() {
        match comp {
            Component::Normal(_) => depth += 1,
            Component::ParentDir => {
                depth -= 1;
                if depth < 0 {
                    return None;
                }
            }
            Component::CurDir => {}
            _ => return None,
        }
    }
    if depth == 0 {
        return None;
    }
    let joined = web_root.join(p);
    let canon = normalized(&joined);
    let root = normalized(web_root);
    if canon.starts_with(&root) && canon != root {
        Some(joined)
    } else {
        None
    }
}

fn normalized(p: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for comp in p.components() {
        match comp {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            other => out.push(other.as_os_str()),
        }
    }
    out
}

pub fn content_type_for(path: &str) -> &'static str {
    let lower = path.to_ascii_lowercase();
    if lower.ends_with(".html") || lower.ends_with(".htm") {
        "text/html; charset=utf-8"
    } else if lower.ends_with(".json") {
        "application/json; charset=utf-8"
    } else if lower.ends_with(".png") {
        "image/png"
    } else if lower.ends_with(".ico") {
        "image/x-icon"
    } else if lower.ends_with(".js") {
        "application/javascript; charset=utf-8"
    } else if lower.ends_with(".css") {
        "text/css; charset=utf-8"
    } else if lower.ends_with(".svg") {
        "image/svg+xml"
    } else {
        "application/octet-stream"
    }
}

pub fn access_url(cfg: &Config) -> String {
    format!("http://{}:{}", get_lan_ip(), cfg.http_port)
}

pub fn pair_uri(cfg: &Config) -> String {
    format!(
        "ptyper://connect?ip={}&http={}&ws={}&pin={}",
        get_lan_ip(),
        cfg.http_port,
        cfg.ws_port,
        cfg.pin
    )
}

pub fn port_in_use(port: u16) -> bool {
    std::net::TcpListener::bind(("0.0.0.0", port)).is_err()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safe_join_rejects_traversal() {
        let root = Path::new("web");
        assert!(safe_join(root, "../config.json").is_none());
        assert!(safe_join(root, "..\\config.json").is_none() || safe_join(root, "..\\config.json").is_none());
        assert!(safe_join(root, "a/../../b.html").is_none());
        assert!(safe_join(root, "/etc/passwd").is_none());
        assert!(safe_join(root, "").is_none());
        assert!(safe_join(root, ".").is_none());
    }

    #[test]
    fn test_safe_join_accepts_normal() {
        let root = Path::new("web");
        assert!(safe_join(root, "index.html").is_some());
        assert!(safe_join(root, "icons/icon-256.png").is_some());
        assert!(safe_join(root, "a/b/../c.html").is_some());
    }

    #[test]
    fn test_pair_uri_format_and_order() {
        let cfg = Config::default();
        let uri = pair_uri(&cfg);
        assert!(uri.starts_with("ptyper://connect?"), "格式 A 前缀: {uri}");
        let q = uri.split('?').nth(1).unwrap();
        let params: Vec<&str> = q.split('&').collect();
        assert_eq!(params[0].split('=').next().unwrap(), "ip");
        assert_eq!(params[1].split('=').next().unwrap(), "http");
        assert_eq!(params[2].split('=').next().unwrap(), "ws");
        assert_eq!(params[3].split('=').next().unwrap(), "pin");
        assert_eq!(params[3], "pin=1234");
    }

    #[test]
    fn test_pair_uri_carries_current_pin() {
        let cfg = Config {
            pin: "9x9y".to_string(),
            ..Config::default()
        };
        let uri = pair_uri(&cfg);
        assert!(uri.ends_with("pin=9x9y"));
    }

    #[test]
    fn test_get_lan_ip_returns_something() {
        let ip = get_lan_ip();
        assert!(!ip.is_empty());
    }
}
