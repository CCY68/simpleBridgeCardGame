use std::collections::HashMap;
use std::net::{SocketAddr, TcpStream};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

/// 連線 ID 類型
pub type ConnectionId = u64;

/// 全域連線 ID 計數器
static CONNECTION_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

/// 產生新的連線 ID
pub fn next_connection_id() -> ConnectionId {
    CONNECTION_ID_COUNTER.fetch_add(1, Ordering::SeqCst)
}

/// 單一連線的資訊
pub struct ConnectionInfo {
    pub id: ConnectionId,
    pub peer_addr: SocketAddr,
    pub stream: TcpStream,
}

impl ConnectionInfo {
    pub fn new(stream: TcpStream) -> std::io::Result<Self> {
        let peer_addr = stream.peer_addr()?;
        let id = next_connection_id();
        Ok(Self {
            id,
            peer_addr,
            stream,
        })
    }
}

/// 連線註冊表 - 管理所有活躍連線
pub struct ConnectionRegistry {
    connections: HashMap<ConnectionId, SocketAddr>,
}

impl ConnectionRegistry {
    pub fn new() -> Self {
        Self {
            connections: HashMap::new(),
        }
    }

    /// 註冊新連線
    pub fn register(&mut self, id: ConnectionId, addr: SocketAddr) {
        self.connections.insert(id, addr);
    }

    /// 移除連線
    pub fn unregister(&mut self, id: ConnectionId) -> Option<SocketAddr> {
        self.connections.remove(&id)
    }

    /// 取得連線數量
    pub fn count(&self) -> usize {
        self.connections.len()
    }

    /// 檢查連線是否存在
    pub fn contains(&self, id: ConnectionId) -> bool {
        self.connections.contains_key(&id)
    }
}

impl Default for ConnectionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// 執行緒安全的連線註冊表
pub type SharedRegistry = Arc<Mutex<ConnectionRegistry>>;

/// 建立共享的連線註冊表
pub fn create_shared_registry() -> SharedRegistry {
    Arc::new(Mutex::new(ConnectionRegistry::new()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_id_generation() {
        let id1 = next_connection_id();
        let id2 = next_connection_id();
        assert!(id2 > id1);
    }

    #[test]
    fn test_registry_operations() {
        let mut registry = ConnectionRegistry::new();
        let addr: SocketAddr = "127.0.0.1:12345".parse().unwrap();

        registry.register(1, addr);
        assert_eq!(registry.count(), 1);
        assert!(registry.contains(1));

        let removed = registry.unregister(1);
        assert_eq!(removed, Some(addr));
        assert_eq!(registry.count(), 0);
    }
}
