use crate::net::ConnectionId;
use crate::protocol::{ClientMessage, ServerMessage};
use std::sync::mpsc;

/// 從 connection handler 傳送到 game loop 的事件
#[derive(Debug)]
pub enum GameEvent {
    /// 新連線建立
    Connected {
        conn_id: ConnectionId,
        sender: ClientSender,
    },

    /// 連線斷開
    Disconnected { conn_id: ConnectionId },

    /// 收到客戶端訊息
    Message {
        conn_id: ConnectionId,
        message: ClientMessage,
    },
}

/// 用於發送訊息給特定 client 的 sender
pub type ClientSender = mpsc::Sender<ServerMessage>;

/// 用於接收訊息的 receiver (connection handler 持有)
pub type ClientReceiver = mpsc::Receiver<ServerMessage>;

/// 建立 client 的訊息通道
pub fn create_client_channel() -> (ClientSender, ClientReceiver) {
    mpsc::channel()
}

/// Game event sender (connection handler 持有，發送事件到 game loop)
pub type EventSender = mpsc::Sender<GameEvent>;

/// Game event receiver (game loop 持有)
pub type EventReceiver = mpsc::Receiver<GameEvent>;

/// 建立 game event 通道
pub fn create_event_channel() -> (EventSender, EventReceiver) {
    mpsc::channel()
}
