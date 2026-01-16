//! Admin Server - 管理介面 TCP Server
//!
//! 在獨立 Port (8890) 監聽管理連線

use super::commands::{
    format_logs, format_players, format_result, format_rooms, format_status, help_message,
    parse_command, AdminEvent, AdminResponse, ParsedCommand,
};
use super::logger::GameLogger;
use log::{error, info, warn};
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

/// Admin Server 設定
pub struct AdminConfig {
    /// 認證 Token
    pub auth_token: String,
    /// 監聽 Port
    pub port: u16,
}

impl Default for AdminConfig {
    fn default() -> Self {
        Self {
            auth_token: std::env::var("ADMIN_AUTH_TOKEN")
                .unwrap_or_else(|_| "admin".to_string()),
            port: 8890,
        }
    }
}

/// Admin 連線狀態
struct AdminSession {
    stream: TcpStream,
    authenticated: bool,
    peer_addr: String,
}

impl AdminSession {
    fn new(stream: TcpStream) -> Self {
        let peer_addr = stream
            .peer_addr()
            .map(|a| a.to_string())
            .unwrap_or_else(|_| "unknown".to_string());

        Self {
            stream,
            authenticated: false,
            peer_addr,
        }
    }

    fn send(&mut self, message: &str) {
        let msg = if message.ends_with('\n') {
            message.to_string()
        } else {
            format!("{}\n", message)
        };

        if let Err(e) = self.stream.write_all(msg.as_bytes()) {
            warn!("[ADMIN] Failed to send to {}: {}", self.peer_addr, e);
        }
        let _ = self.stream.flush();
    }

    fn send_prompt(&mut self) {
        let prompt = if self.authenticated {
            "admin> "
        } else {
            "auth> "
        };
        let _ = self.stream.write_all(prompt.as_bytes());
        let _ = self.stream.flush();
    }
}

/// 啟動 Admin Server
pub fn spawn_admin_server(
    config: AdminConfig,
    event_tx: mpsc::Sender<AdminEvent>,
    logger: GameLogger,
) -> std::io::Result<()> {
    let addr = format!("0.0.0.0:{}", config.port);
    let listener = TcpListener::bind(&addr)?;

    info!("[ADMIN] Admin server listening on {}", addr);

    thread::spawn(move || {
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let event_tx = event_tx.clone();
                    let logger = logger.clone();
                    let auth_token = config.auth_token.clone();

                    thread::spawn(move || {
                        handle_admin_connection(stream, auth_token, event_tx, logger);
                    });
                }
                Err(e) => {
                    error!("[ADMIN] Accept error: {}", e);
                }
            }
        }
    });

    Ok(())
}

/// 處理單一管理連線
fn handle_admin_connection(
    stream: TcpStream,
    auth_token: String,
    event_tx: mpsc::Sender<AdminEvent>,
    logger: GameLogger,
) {
    let mut session = AdminSession::new(stream.try_clone().unwrap());
    info!("[ADMIN] New admin connection from {}", session.peer_addr);

    // 設定讀取 timeout
    let _ = stream.set_read_timeout(Some(Duration::from_secs(300))); // 5 分鐘

    // 發送歡迎訊息
    session.send("=== CardArena Admin Console ===");
    session.send("Type 'AUTH <token>' to authenticate, or 'HELP' for commands.");
    session.send_prompt();

    let reader = BufReader::new(stream);

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                info!("[ADMIN] {} disconnected: {}", session.peer_addr, e);
                break;
            }
        };

        let command = parse_command(&line);

        match command {
            ParsedCommand::Help => {
                session.send(&help_message());
            }

            ParsedCommand::Auth(token) => {
                if token == auth_token {
                    session.authenticated = true;
                    session.send("OK: Authentication successful");
                    logger.admin_action("AUTH", &format!("Admin logged in from {}", session.peer_addr));
                } else {
                    session.send("ERROR: Invalid token");
                    warn!("[ADMIN] Failed auth attempt from {}", session.peer_addr);
                }
            }

            ParsedCommand::Status => {
                if !session.authenticated {
                    session.send("ERROR: Not authenticated. Use AUTH <token> first.");
                } else {
                    match send_and_receive(&event_tx, |reply_tx| AdminEvent::GetStatus { reply_tx }) {
                        Some(response) => session.send(&format_status(&response)),
                        None => session.send("ERROR: Failed to get status"),
                    }
                }
            }

            ParsedCommand::Rooms => {
                if !session.authenticated {
                    session.send("ERROR: Not authenticated. Use AUTH <token> first.");
                } else {
                    match send_and_receive(&event_tx, |reply_tx| AdminEvent::GetRooms { reply_tx }) {
                        Some(response) => session.send(&format_rooms(&response)),
                        None => session.send("ERROR: Failed to get rooms"),
                    }
                }
            }

            ParsedCommand::Players => {
                if !session.authenticated {
                    session.send("ERROR: Not authenticated. Use AUTH <token> first.");
                } else {
                    match send_and_receive(&event_tx, |reply_tx| AdminEvent::GetPlayers { reply_tx }) {
                        Some(response) => session.send(&format_players(&response)),
                        None => session.send("ERROR: Failed to get players"),
                    }
                }
            }

            ParsedCommand::Logs(count, event_type) => {
                if !session.authenticated {
                    session.send("ERROR: Not authenticated. Use AUTH <token> first.");
                } else {
                    session.send(&format_logs(&logger, count, event_type));
                }
            }

            ParsedCommand::Kick(player_id) => {
                if !session.authenticated {
                    session.send("ERROR: Not authenticated. Use AUTH <token> first.");
                } else {
                    match send_and_receive(&event_tx, |reply_tx| AdminEvent::KickPlayer {
                        player_id: player_id.clone(),
                        reply_tx,
                    }) {
                        Some(response) => {
                            session.send(&format_result(&response));
                            if matches!(response, AdminResponse::Ok(_)) {
                                logger.admin_action("KICK", &format!("Kicked player {}", player_id));
                            }
                        }
                        None => session.send("ERROR: Failed to kick player"),
                    }
                }
            }

            ParsedCommand::Reset(room_id) => {
                if !session.authenticated {
                    session.send("ERROR: Not authenticated. Use AUTH <token> first.");
                } else {
                    match send_and_receive(&event_tx, |reply_tx| AdminEvent::ResetRoom {
                        room_id: room_id.clone(),
                        reply_tx,
                    }) {
                        Some(response) => {
                            session.send(&format_result(&response));
                            if matches!(response, AdminResponse::Ok(_)) {
                                let target = room_id.unwrap_or_else(|| "all".to_string());
                                logger.admin_action("RESET", &format!("Reset room {}", target));
                            }
                        }
                        None => session.send("ERROR: Failed to reset room"),
                    }
                }
            }

            ParsedCommand::Quit => {
                session.send("Goodbye!");
                info!("[ADMIN] {} logged out", session.peer_addr);
                break;
            }

            ParsedCommand::Unknown(msg) => {
                if msg.is_empty() {
                    // 空行，不顯示錯誤
                } else {
                    session.send(&format!("ERROR: {}", msg));
                }
            }
        }

        session.send_prompt();
    }

    info!("[ADMIN] Connection closed: {}", session.peer_addr);
}

/// 發送事件並等待回應
fn send_and_receive<F>(event_tx: &mpsc::Sender<AdminEvent>, make_event: F) -> Option<AdminResponse>
where
    F: FnOnce(mpsc::Sender<AdminResponse>) -> AdminEvent,
{
    let (reply_tx, reply_rx) = mpsc::channel();
    let event = make_event(reply_tx);

    if event_tx.send(event).is_err() {
        return None;
    }

    // 等待回應 (最多 5 秒)
    reply_rx.recv_timeout(Duration::from_secs(5)).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_admin_config_default() {
        let config = AdminConfig::default();
        assert_eq!(config.port, 8890);
    }
}
