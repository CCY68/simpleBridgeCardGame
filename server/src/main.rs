mod game;
mod lobby;
mod net;
mod protocol;

use lobby::{HandshakeResult, RoomManager, RoomState, process_hello};
use log::{error, info, warn};
use net::{
    ClientSender, ConnectionId, GameEvent, create_event_channel, next_connection_id, spawn_handler,
};
use protocol::{ClientMessage, ErrorCode, ServerMessage};
use std::collections::HashMap;
use std::env;
use std::net::SocketAddr;
use std::thread;

const DEFAULT_PORT: u16 = 8888;

/// 伺服器設定
struct ServerConfig {
    ai_auth_token: Option<String>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            // 開發模式：不需要 AI token
            // 正式環境可設定環境變數 AI_AUTH_TOKEN
            ai_auth_token: env::var("AI_AUTH_TOKEN").ok(),
        }
    }
}

fn main() {
    // 初始化 logger
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    // 解析命令列參數
    let port = parse_port_from_args().unwrap_or(DEFAULT_PORT);
    let addr: SocketAddr = format!("0.0.0.0:{}", port).parse().expect("Invalid address");

    // 建立 TCP listener
    let listener = match net::create_tcp_listener(addr) {
        Ok(l) => l,
        Err(e) => {
            error!("[SERVER] Failed to create listener: {}", e);
            std::process::exit(1);
        }
    };

    let local_addr = listener.local_addr().expect("Failed to get local address");
    info!("[SERVER] Listening on {}", local_addr);

    // 建立 event channel
    let (event_tx, event_rx) = create_event_channel();

    // 啟動 accept loop thread
    let accept_tx = event_tx.clone();
    thread::spawn(move || {
        accept_loop(listener, accept_tx);
    });

    // 載入設定
    let config = ServerConfig::default();

    // Main game loop
    game_loop(event_rx, config);
}

/// Accept loop - 接受新連線並建立 handler
fn accept_loop(listener: std::net::TcpListener, event_tx: net::EventSender) {
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let peer_addr = stream
                    .peer_addr()
                    .map(|a| a.to_string())
                    .unwrap_or_else(|_| "unknown".to_string());
                let conn_id = next_connection_id();

                info!("[ACCEPT] New connection #{} from {}", conn_id, peer_addr);

                // 啟動 connection handler
                if let Err(e) = spawn_handler(conn_id, stream, event_tx.clone()) {
                    warn!(
                        "[ACCEPT] Failed to spawn handler for #{}: {}",
                        conn_id, e
                    );
                }
            }
            Err(e) => {
                error!("[ACCEPT] Accept error: {}", e);
            }
        }
    }
}

/// Game loop - 處理所有 game events
fn game_loop(event_rx: net::EventReceiver, config: ServerConfig) {
    // 儲存所有連線的 sender
    let mut clients: HashMap<ConnectionId, ClientSender> = HashMap::new();
    // 房間管理器
    let mut room_manager = RoomManager::new();

    info!("[GAME] Game loop started");

    for event in event_rx {
        match event {
            GameEvent::Connected { conn_id, sender } => {
                info!(
                    "[GAME] Client #{} connected (total: {})",
                    conn_id,
                    clients.len() + 1
                );
                clients.insert(conn_id, sender);
            }

            GameEvent::Disconnected { conn_id } => {
                clients.remove(&conn_id);
                // 從房間移除
                if let Some(player) = room_manager.handle_disconnect(conn_id) {
                    info!(
                        "[GAME] Player '{}' ({}) disconnected from room",
                        player.nickname, player.player_id
                    );
                }
                info!(
                    "[GAME] Client #{} disconnected (total: {})",
                    conn_id,
                    clients.len()
                );
            }

            GameEvent::Message { conn_id, message } => {
                handle_game_message(conn_id, &message, &mut clients, &mut room_manager, &config);
            }
        }
    }

    info!("[GAME] Game loop ended");
}

/// 處理遊戲訊息
fn handle_game_message(
    conn_id: ConnectionId,
    msg: &ClientMessage,
    clients: &mut HashMap<ConnectionId, ClientSender>,
    room_manager: &mut RoomManager,
    config: &ServerConfig,
) {
    match msg {
        ClientMessage::Ping => {
            info!("[GAME] #{} PING -> PONG", conn_id);
            send_to(clients, conn_id, &ServerMessage::Pong);
        }

        ClientMessage::Hello {
            role,
            nickname,
            proto,
            auth,
        } => {
            info!("[GAME] #{} HELLO from {:?} '{}'", conn_id, role, nickname);

            // 取得或建立等待中的房間
            let room = room_manager.get_or_create_waiting_room();

            // 檢查房間是否已滿
            if room.is_full() {
                send_to(
                    clients,
                    conn_id,
                    &ServerMessage::Error {
                        code: ErrorCode::RoomFull,
                        message: "Room is full".to_string(),
                    },
                );
                return;
            }

            // 處理 handshake
            let result = process_hello(
                role,
                nickname,
                *proto,
                auth,
                room.get_nicknames(),
                room.next_slot(),
                &room.id,
                config.ai_auth_token.as_deref(),
            );

            match result {
                HandshakeResult::Success(welcome_msg) => {
                    // 從 welcome_msg 取得 player_id 和 nickname
                    if let ServerMessage::Welcome {
                        player_id,
                        nickname: final_nickname,
                        room: room_id,
                    } = &welcome_msg
                    {
                        // 複製需要的資料，避免借用衝突
                        let room_id_clone = room_id.clone();
                        let player_count;
                        let wait_msg;
                        let conn_ids;
                        let can_start;
                        let seed;

                        // 加入房間 (在這個作用域內完成所有對 room 的操作)
                        {
                            room.add_player(conn_id, player_id, final_nickname, *role);
                            player_count = room.players.len();
                            wait_msg = room.room_wait_message();
                            conn_ids = room.conn_ids();
                            can_start = room.can_start();
                            seed = room.seed;
                        }

                        // 關聯連線到房間
                        room_manager.associate_conn(conn_id, &room_id_clone);

                        // 發送 WELCOME 給新玩家
                        send_to(clients, conn_id, &welcome_msg);

                        info!(
                            "[LOBBY] Player '{}' ({}) joined room {} ({}/4 players)",
                            final_nickname, player_id, room_id, player_count
                        );

                        // 廣播 ROOM_WAIT 給所有房間內的玩家
                        for &cid in &conn_ids {
                            send_to(clients, cid, &wait_msg);
                        }

                        // 檢查是否可以開始遊戲
                        if can_start {
                            // 重新取得 room 的 mutable 引用
                            if let Some(room) = room_manager.get_room_mut(&room_id_clone) {
                                room.state = RoomState::Playing;
                                room.assign_teams();

                                let start_msg = room.room_start_message();
                                let conn_ids = room.conn_ids();

                                info!(
                                    "[LOBBY] Room {} starting game with seed {}",
                                    room_id_clone, seed
                                );

                                for &cid in &conn_ids {
                                    send_to(clients, cid, &start_msg);
                                }

                                // TODO: S3.1 會實作發牌邏輯
                            }
                        }
                    }
                }
                HandshakeResult::Error(error_msg) => {
                    warn!("[LOBBY] Handshake failed for #{}: {:?}", conn_id, error_msg);
                    send_to(clients, conn_id, &error_msg);
                }
            }
        }

        ClientMessage::Play { card } => {
            info!(
                "[GAME] #{} PLAY {} (rejected - game not started)",
                conn_id, card
            );
            // TODO: S3.3 會實作完整的出牌驗證
            send_to(
                clients,
                conn_id,
                &ServerMessage::Error {
                    code: ErrorCode::NotYourTurn,
                    message: "Game not started".to_string(),
                },
            );
        }
    }
}

/// 發送訊息給特定 client
fn send_to(clients: &HashMap<ConnectionId, ClientSender>, conn_id: ConnectionId, msg: &ServerMessage) {
    if let Some(sender) = clients.get(&conn_id) {
        if sender.send(msg.clone()).is_err() {
            warn!("[GAME] Failed to send to #{}", conn_id);
        }
    }
}

/// 廣播訊息給所有 clients
#[allow(dead_code)]
fn broadcast(clients: &HashMap<ConnectionId, ClientSender>, msg: &ServerMessage) {
    for (conn_id, sender) in clients {
        if sender.send(msg.clone()).is_err() {
            warn!("[GAME] Failed to broadcast to #{}", conn_id);
        }
    }
}

/// 從命令列參數解析 port
fn parse_port_from_args() -> Option<u16> {
    let args: Vec<String> = env::args().collect();
    let mut i = 1;
    while i < args.len() {
        if args[i] == "--port" && i + 1 < args.len() {
            return args[i + 1].parse().ok();
        }
        i += 1;
    }
    None
}
