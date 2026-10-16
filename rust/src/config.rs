use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use tracing::warn;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_http_port")]
    pub http_port: u16,
    #[serde(default = "default_ws_port")]
    pub ws_port: u16,
    #[serde(default = "default_pin")]
    pub pin: String,
    #[serde(default = "default_restore_clipboard")]
    pub restore_clipboard: bool,
    #[serde(default = "default_paste_delay_ms")]
    pub paste_delay_ms: u64,
    #[serde(default)]
    pub auto_enter: bool,
}

fn default_http_port() -> u16 {
    8766
}
fn default_ws_port() -> u16 {
    8767
}
fn default_pin() -> String {
    "1234".to_string()
}
fn default_restore_clipboard() -> bool {
    true
}
fn default_paste_delay_ms() -> u64 {
    60
}

impl Default for Config {
    fn default() -> Self {
        Self {
            http_port: default_http_port(),
            ws_port: default_ws_port(),
            pin: default_pin(),
            restore_clipboard: default_restore_clipboard(),
            paste_delay_ms: default_paste_delay_ms(),
            auto_enter: false,
        }
    }
}

pub type SharedConfig = Arc<RwLock<Config>>;

pub fn config_path() -> PathBuf {
    let mut dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."));
    dir.push("config.json");
    dir
}

pub fn load_config() -> Config {
    let path = config_path();
    match fs::read_to_string(&path) {
        Ok(text) => match serde_json::from_str::<Config>(&text) {
            Ok(cfg) => cfg,
            Err(e) => {
                warn!(error = %e, path = %path.display(), "config.json 损坏，使用默认配置启动");
                Config::default()
            }
        },
        Err(_) => {
            warn!(path = %path.display(), "config.json 不存在，使用默认配置启动");
            Config::default()
        }
    }
}

pub fn save_config(cfg: &Config) -> std::io::Result<()> {
    let path = config_path();
    let mut merged = load_existing_raw();
    let obj = serde_json::to_value(cfg).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    if let (serde_json::Value::Object(dst), serde_json::Value::Object(src)) = (&mut merged, &obj) {
        for (k, v) in src {
            dst.insert(k.clone(), v.clone());
        }
    }
    let text = serde_json::to_string_pretty(&merged)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    fs::write(&path, text)
}

fn load_existing_raw() -> serde_json::Value {
    let path = config_path();
    fs::read_to_string(&path)
        .ok()
        .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
        .unwrap_or_else(|| serde_json::Value::Object(serde_json::Map::new()))
}

pub fn shared_config() -> SharedConfig {
    Arc::new(RwLock::new(load_config()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_values() {
        let cfg = Config::default();
        assert_eq!(cfg.http_port, 8766);
        assert_eq!(cfg.ws_port, 8767);
        assert_eq!(cfg.pin, "1234");
        assert!(cfg.restore_clipboard);
        assert_eq!(cfg.paste_delay_ms, 60);
        assert!(!cfg.auto_enter);
    }

    #[test]
    fn test_corrupted_json_falls_back() {
        let cfg: Config = serde_json::from_str("{not valid json").unwrap_or_default();
        assert_eq!(cfg.http_port, 8766);
        assert_eq!(cfg.pin, "1234");
    }

    #[test]
    fn test_merge_write_keeps_unknown_keys() {
        let existing = r#"{"http_port":9000,"ws_port":9001,"pin":"9999","unknown_key":"keepme"}"#;
        let mut base: serde_json::Value = serde_json::from_str(existing).unwrap();
        let cfg = Config {
            pin: "abcd".to_string(),
            ..Config::default()
        };
        let obj = serde_json::to_value(&cfg).unwrap();
        if let (serde_json::Value::Object(dst), serde_json::Value::Object(src)) =
            (&mut base, &obj)
        {
            for (k, v) in src {
                dst.insert(k.clone(), v.clone());
            }
        }
        assert_eq!(base["http_port"], 8766);
        assert_eq!(base["ws_port"], 8767);
        assert_eq!(base["pin"], "abcd");
        assert_eq!(base["unknown_key"], "keepme");
    }

    #[test]
    fn test_partial_json_uses_defaults() {
        let cfg: Config = serde_json::from_str(r#"{"pin":"5678"}"#).unwrap();
        assert_eq!(cfg.pin, "5678");
        assert_eq!(cfg.http_port, 8766);
        assert_eq!(cfg.ws_port, 8767);
    }
}
