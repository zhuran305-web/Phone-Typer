use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex as StdMutex};
use tokio::sync::mpsc;
use tracing::warn;

pub type ConnId = usize;

pub struct WsHub {
    next_id: AtomicUsize,
    connections: StdMutex<HashMap<ConnId, mpsc::UnboundedSender<String>>>,
}

impl WsHub {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            next_id: AtomicUsize::new(0),
            connections: StdMutex::new(HashMap::new()),
        })
    }

    pub fn register(&self, tx: mpsc::UnboundedSender<String>) -> ConnId {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        self.connections.lock().unwrap().insert(id, tx);
        id
    }

    pub fn unregister(&self, id: ConnId) {
        self.connections.lock().unwrap().remove(&id);
    }

    #[allow(dead_code)]
    pub fn connection_count(&self) -> usize {
        self.connections.lock().unwrap().len()
    }

    pub fn broadcast_clip(&self, text: &str) -> usize {
        let msg = serde_json::json!({"type": "clip", "text": text}).to_string();
        let conns = self.connections.lock().unwrap();
        let mut count = 0;
        for (id, tx) in conns.iter() {
            match tx.send(msg.clone()) {
                Ok(()) => count += 1,
                Err(_) => warn!(conn_id = id, "推送剪贴板失败：连接已关闭"),
            }
        }
        count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_register_unregister() {
        let hub = WsHub::new();
        let (tx, _rx) = mpsc::unbounded_channel::<String>();
        let id = hub.register(tx);
        assert_eq!(hub.connection_count(), 1);
        hub.unregister(id);
        assert_eq!(hub.connection_count(), 0);
    }

    #[tokio::test]
    async fn test_broadcast_clip_sends_to_all() {
        let hub = WsHub::new();
        let (tx1, mut rx1) = mpsc::unbounded_channel::<String>();
        let (tx2, mut rx2) = mpsc::unbounded_channel::<String>();
        hub.register(tx1);
        hub.register(tx2);
        let count = hub.broadcast_clip("hello");
        assert_eq!(count, 2);
        let m1 = rx1.recv().await.unwrap();
        let m2 = rx2.recv().await.unwrap();
        assert!(m1.contains("clip") && m1.contains("hello"));
        assert!(m2.contains("clip") && m2.contains("hello"));
    }

    #[tokio::test]
    async fn test_broadcast_clip_skips_closed() {
        let hub = WsHub::new();
        let (tx, rx) = mpsc::unbounded_channel::<String>();
        hub.register(tx);
        drop(rx);
        let count = hub.broadcast_clip("hello");
        assert_eq!(count, 0);
    }
}
