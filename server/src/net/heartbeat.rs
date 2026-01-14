//! UDP Heartbeat Server
//!
//! 處理 HB_PING/HB_PONG 訊息，用於監控連線品質 (RTT/loss rate)。

use crate::protocol::{HeartbeatPing, HeartbeatPong};
use log::{debug, error, info, warn};
use std::collections::HashMap;
use std::io;
use std::net::{SocketAddr, UdpSocket};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Client heartbeat 狀態
#[derive(Debug, Clone)]
pub struct ClientHeartbeatState {
    /// Client 地址
    #[allow(dead_code)]
    pub addr: SocketAddr,
    /// 最後收到 heartbeat 的時間
    pub last_heartbeat: Instant,
    /// 收到的 ping 次數
    pub ping_count: u64,
    /// 最後的 sequence number
    pub last_seq: u64,
}

/// Heartbeat 追蹤器 (thread-safe)
pub type HeartbeatTracker = Arc<Mutex<HashMap<SocketAddr, ClientHeartbeatState>>>;

/// 建立 heartbeat 追蹤器
pub fn create_heartbeat_tracker() -> HeartbeatTracker {
    Arc::new(Mutex::new(HashMap::new()))
}

/// 取得當前 Unix timestamp (毫秒)
fn current_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// 建立 UDP socket 並綁定到指定 port
pub fn create_udp_socket(port: u16) -> io::Result<UdpSocket> {
    let addr: SocketAddr = format!("0.0.0.0:{}", port).parse().unwrap();
    let socket = UdpSocket::bind(addr)?;

    // 設定非阻塞模式的 timeout (用於檢查 stale clients)
    socket.set_read_timeout(Some(Duration::from_secs(1)))?;

    info!("[HEARTBEAT] UDP socket bound to {}", addr);
    Ok(socket)
}

/// 處理單個 HB_PING 訊息
fn handle_ping(
    socket: &UdpSocket,
    buf: &[u8],
    src_addr: SocketAddr,
    tracker: &HeartbeatTracker,
) -> io::Result<()> {
    // 解析 HB_PING
    let ping: HeartbeatPing = match serde_json::from_slice(buf) {
        Ok(p) => p,
        Err(e) => {
            debug!("[HEARTBEAT] Failed to parse ping from {}: {}", src_addr, e);
            return Ok(());
        }
    };

    // 驗證訊息類型
    if ping.msg_type != "HB_PING" {
        debug!(
            "[HEARTBEAT] Unexpected message type from {}: {}",
            src_addr, ping.msg_type
        );
        return Ok(());
    }

    // 更新追蹤器
    {
        let mut tracker = tracker.lock().unwrap();
        let state = tracker.entry(src_addr).or_insert_with(|| ClientHeartbeatState {
            addr: src_addr,
            last_heartbeat: Instant::now(),
            ping_count: 0,
            last_seq: 0,
        });
        state.last_heartbeat = Instant::now();
        state.ping_count += 1;
        state.last_seq = ping.seq;
    }

    // 產生 HB_PONG
    let pong = HeartbeatPong::from_ping(&ping, current_time_ms());
    let pong_json = serde_json::to_string(&pong)?;

    // 發送回覆
    socket.send_to(pong_json.as_bytes(), src_addr)?;

    debug!(
        "[HEARTBEAT] PING seq={} from {} -> PONG",
        ping.seq, src_addr
    );

    Ok(())
}

/// Stale client 檢測結果
pub struct StaleCheckResult {
    /// 被標記為 stale 的 client 地址
    pub stale_clients: Vec<SocketAddr>,
}

/// 檢查並移除 stale clients
pub fn check_stale_clients(tracker: &HeartbeatTracker, threshold_secs: u64) -> StaleCheckResult {
    let threshold = Duration::from_secs(threshold_secs);
    let now = Instant::now();
    let mut stale_clients = Vec::new();

    let mut tracker = tracker.lock().unwrap();
    tracker.retain(|addr, state| {
        let elapsed = now.duration_since(state.last_heartbeat);
        if elapsed > threshold {
            warn!(
                "[HEARTBEAT] Client {} is stale (no heartbeat for {:.1}s)",
                addr,
                elapsed.as_secs_f32()
            );
            stale_clients.push(*addr);
            false // 移除
        } else {
            true // 保留
        }
    });

    StaleCheckResult { stale_clients }
}

/// 啟動 UDP heartbeat server (在獨立 thread 中運行)
pub fn spawn_heartbeat_server(
    port: u16,
    tracker: HeartbeatTracker,
    stale_threshold_secs: u64,
) -> io::Result<thread::JoinHandle<()>> {
    let socket = create_udp_socket(port)?;

    let handle = thread::spawn(move || {
        info!("[HEARTBEAT] Heartbeat server started on port {}", port);

        let mut buf = [0u8; 1024];
        let mut last_stale_check = Instant::now();
        let stale_check_interval = Duration::from_secs(5);

        loop {
            // 嘗試接收 UDP 封包
            match socket.recv_from(&mut buf) {
                Ok((len, src_addr)) => {
                    if let Err(e) = handle_ping(&socket, &buf[..len], src_addr, &tracker) {
                        error!("[HEARTBEAT] Error handling ping from {}: {}", src_addr, e);
                    }
                }
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                    // Timeout，繼續檢查 stale clients
                }
                Err(ref e) if e.kind() == io::ErrorKind::TimedOut => {
                    // Timeout，繼續檢查 stale clients
                }
                Err(e) => {
                    error!("[HEARTBEAT] recv_from error: {}", e);
                }
            }

            // 定期檢查 stale clients
            if last_stale_check.elapsed() > stale_check_interval {
                let result = check_stale_clients(&tracker, stale_threshold_secs);
                if !result.stale_clients.is_empty() {
                    info!(
                        "[HEARTBEAT] Removed {} stale client(s)",
                        result.stale_clients.len()
                    );
                }
                last_stale_check = Instant::now();
            }
        }
    });

    Ok(handle)
}

/// 取得所有活躍 clients 的統計
#[allow(dead_code)]
pub fn get_heartbeat_stats(tracker: &HeartbeatTracker) -> Vec<(SocketAddr, u64, Duration)> {
    let tracker = tracker.lock().unwrap();
    tracker
        .iter()
        .map(|(addr, state)| {
            let since_last = Instant::now().duration_since(state.last_heartbeat);
            (*addr, state.ping_count, since_last)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_heartbeat_tracker() {
        let tracker = create_heartbeat_tracker();
        let guard = tracker.lock().unwrap();
        assert!(guard.is_empty());
    }

    #[test]
    fn test_current_time_ms() {
        let t1 = current_time_ms();
        std::thread::sleep(Duration::from_millis(10));
        let t2 = current_time_ms();
        assert!(t2 >= t1);
    }

    #[test]
    fn test_heartbeat_pong_from_ping() {
        let ping = HeartbeatPing {
            msg_type: "HB_PING".to_string(),
            seq: 42,
            t_client_ms: 1000000,
        };
        let server_time = 1000005;
        let pong = HeartbeatPong::from_ping(&ping, server_time);

        assert_eq!(pong.msg_type, "HB_PONG");
        assert_eq!(pong.seq, 42);
        assert_eq!(pong.t_client_ms, 1000000);
        assert_eq!(pong.t_server_ms, 1000005);
    }

    #[test]
    fn test_stale_client_detection() {
        let tracker = create_heartbeat_tracker();
        let addr: SocketAddr = "127.0.0.1:12345".parse().unwrap();

        // 新增一個 client
        {
            let mut guard = tracker.lock().unwrap();
            guard.insert(
                addr,
                ClientHeartbeatState {
                    addr,
                    last_heartbeat: Instant::now() - Duration::from_secs(15),
                    ping_count: 5,
                    last_seq: 5,
                },
            );
        }

        // 10 秒 threshold，應該被標記為 stale
        let result = check_stale_clients(&tracker, 10);
        assert_eq!(result.stale_clients.len(), 1);
        assert_eq!(result.stale_clients[0], addr);

        // 確認已被移除
        let guard = tracker.lock().unwrap();
        assert!(guard.is_empty());
    }

    #[test]
    fn test_heartbeat_ping_serialize() {
        let ping = HeartbeatPing {
            msg_type: "HB_PING".to_string(),
            seq: 1,
            t_client_ms: 123456789,
        };
        let json = serde_json::to_string(&ping).unwrap();
        assert!(json.contains("\"type\":\"HB_PING\""));
        assert!(json.contains("\"seq\":1"));
    }

    #[test]
    fn test_heartbeat_pong_serialize() {
        let pong = HeartbeatPong {
            msg_type: "HB_PONG".to_string(),
            seq: 1,
            t_client_ms: 123456789,
            t_server_ms: 123456790,
        };
        let json = serde_json::to_string(&pong).unwrap();
        assert!(json.contains("\"type\":\"HB_PONG\""));
        assert!(json.contains("\"t_server_ms\":123456790"));
    }
}
