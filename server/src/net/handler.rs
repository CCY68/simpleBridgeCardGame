use super::connection::ConnectionId;
use super::event::{ClientReceiver, ClientSender, EventSender, GameEvent, create_client_channel};
use crate::protocol::{Codec, ErrorCode};
use log::{info, warn};
use std::io;
use std::net::TcpStream;
use std::sync::mpsc::TryRecvError;
use std::thread;
use std::time::Duration;

/// Connection handler - 處理單一連線的讀寫
pub struct ConnectionHandler {
    conn_id: ConnectionId,
    codec: Codec,
    event_tx: EventSender,
    client_rx: ClientReceiver,
}

impl ConnectionHandler {
    /// 建立新的 connection handler
    pub fn new(
        conn_id: ConnectionId,
        stream: TcpStream,
        event_tx: EventSender,
    ) -> io::Result<(Self, ClientSender)> {
        // 設定 non-blocking 或 timeout 以便檢查 client_rx
        stream.set_read_timeout(Some(Duration::from_millis(100)))?;

        let codec = Codec::new(stream)?;
        let (client_tx, client_rx) = create_client_channel();

        let handler = Self {
            conn_id,
            codec,
            event_tx,
            client_rx,
        };

        Ok((handler, client_tx))
    }

    /// 執行 handler 主迴圈
    pub fn run(mut self) {
        // 通知 game loop 連線已建立
        // Note: sender 會在建立時由 spawn 函數發送

        loop {
            // 嘗試讀取 client 訊息
            match self.codec.read_message() {
                Ok(Some(msg)) => {
                    info!("[HANDLER] Connection #{} received: {:?}", self.conn_id, msg);

                    // 發送事件到 game loop
                    if self
                        .event_tx
                        .send(GameEvent::Message {
                            conn_id: self.conn_id,
                            message: msg,
                        })
                        .is_err()
                    {
                        warn!("[HANDLER] Connection #{} event channel closed", self.conn_id);
                        break;
                    }
                }
                Ok(None) => {
                    // EOF - 連線關閉
                    info!("[HANDLER] Connection #{} EOF", self.conn_id);
                    break;
                }
                Err(e) => {
                    // 區分 timeout 和其他錯誤
                    if e.kind() == io::ErrorKind::WouldBlock || e.kind() == io::ErrorKind::TimedOut
                    {
                        // Timeout - 繼續檢查 client_rx
                    } else {
                        warn!("[HANDLER] Connection #{} read error: {}", self.conn_id, e);
                        let _ = self
                            .codec
                            .send_error(ErrorCode::ProtocolError, e.to_string());
                        break;
                    }
                }
            }

            // 檢查是否有要發送給 client 的訊息
            loop {
                match self.client_rx.try_recv() {
                    Ok(msg) => {
                        if let Err(e) = self.codec.send_message(&msg) {
                            warn!("[HANDLER] Connection #{} send error: {}", self.conn_id, e);
                            break;
                        }
                    }
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => {
                        info!("[HANDLER] Connection #{} client channel closed", self.conn_id);
                        return;
                    }
                }
            }
        }

        // 通知 game loop 連線已斷開
        let _ = self.event_tx.send(GameEvent::Disconnected {
            conn_id: self.conn_id,
        });
    }
}

/// 在新執行緒中啟動 connection handler
pub fn spawn_handler(
    conn_id: ConnectionId,
    stream: TcpStream,
    event_tx: EventSender,
) -> io::Result<ClientSender> {
    let (handler, client_tx) = ConnectionHandler::new(conn_id, stream, event_tx.clone())?;

    // 發送 Connected 事件
    let sender_for_event = client_tx.clone();
    if event_tx
        .send(GameEvent::Connected {
            conn_id,
            sender: sender_for_event,
        })
        .is_err()
    {
        return Err(io::Error::new(
            io::ErrorKind::BrokenPipe,
            "Event channel closed",
        ));
    }

    // 啟動 handler 執行緒
    thread::spawn(move || {
        handler.run();
    });

    Ok(client_tx)
}
