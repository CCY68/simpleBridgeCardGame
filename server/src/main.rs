mod game;
mod lobby;
mod net;
mod protocol;

use game::{GameEngine, GamePhase, PlayError, PlayResult, TrickResolution};
use lobby::{HandshakeResult, RoomManager, RoomState, process_hello};
use log::{error, info, warn};
use net::{
    ClientSender, ConnectionId, GameEvent, create_event_channel, create_heartbeat_tracker,
    next_connection_id, spawn_handler, spawn_heartbeat_server,
};
use protocol::{ClientMessage, ErrorCode, RejectReason, RoomId, ServerMessage};
use std::collections::HashMap;
use std::env;
use std::net::SocketAddr;
use std::thread;

const DEFAULT_PORT: u16 = 8888;
const DEFAULT_UDP_PORT_OFFSET: u16 = 1; // UDP port = TCP port + 1
const STALE_THRESHOLD_SECS: u64 = 10; // Client stale 閾值 (秒)

/// 伺服器設定
struct ServerConfig {
    ai_auth_token: Option<String>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            ai_auth_token: env::var("AI_AUTH_TOKEN").ok(),
        }
    }
}

/// 伺服器狀態
struct ServerState {
    /// 所有連線的 sender
    clients: HashMap<ConnectionId, ClientSender>,
    /// 房間管理器
    room_manager: RoomManager,
    /// 遊戲引擎 (room_id -> engine)
    games: HashMap<RoomId, GameEngine>,
    /// 連線到房間的對應 (conn_id -> room_id)
    conn_to_room: HashMap<ConnectionId, RoomId>,
}

impl ServerState {
    fn new() -> Self {
        Self {
            clients: HashMap::new(),
            room_manager: RoomManager::new(),
            games: HashMap::new(),
            conn_to_room: HashMap::new(),
        }
    }
}

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let port = parse_port_from_args().unwrap_or(DEFAULT_PORT);
    let addr: SocketAddr = format!("0.0.0.0:{}", port).parse().expect("Invalid address");

    let listener = match net::create_tcp_listener(addr) {
        Ok(l) => l,
        Err(e) => {
            error!("[SERVER] Failed to create listener: {}", e);
            std::process::exit(1);
        }
    };

    let local_addr = listener.local_addr().expect("Failed to get local address");
    info!("[SERVER] Listening on {}", local_addr);

    // 啟動 UDP Heartbeat Server
    let udp_port = port + DEFAULT_UDP_PORT_OFFSET;
    let heartbeat_tracker = create_heartbeat_tracker();
    match spawn_heartbeat_server(udp_port, heartbeat_tracker.clone(), STALE_THRESHOLD_SECS) {
        Ok(_) => {
            info!("[SERVER] UDP Heartbeat server started on port {}", udp_port);
        }
        Err(e) => {
            warn!(
                "[SERVER] Failed to start UDP Heartbeat server: {} (continuing without heartbeat)",
                e
            );
        }
    }

    let (event_tx, event_rx) = create_event_channel();

    let accept_tx = event_tx.clone();
    thread::spawn(move || {
        accept_loop(listener, accept_tx);
    });

    let config = ServerConfig::default();
    game_loop(event_rx, config);
}

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

                if let Err(e) = spawn_handler(conn_id, stream, event_tx.clone()) {
                    warn!("[ACCEPT] Failed to spawn handler for #{}: {}", conn_id, e);
                }
            }
            Err(e) => {
                error!("[ACCEPT] Accept error: {}", e);
            }
        }
    }
}

fn game_loop(event_rx: net::EventReceiver, config: ServerConfig) {
    let mut state = ServerState::new();

    info!("[GAME] Game loop started");

    for event in event_rx {
        match event {
            GameEvent::Connected { conn_id, sender } => {
                info!(
                    "[GAME] Client #{} connected (total: {})",
                    conn_id,
                    state.clients.len() + 1
                );
                state.clients.insert(conn_id, sender);
            }

            GameEvent::Disconnected { conn_id } => {
                state.clients.remove(&conn_id);
                state.conn_to_room.remove(&conn_id);

                if let Some(player) = state.room_manager.handle_disconnect(conn_id) {
                    info!(
                        "[GAME] Player '{}' ({}) disconnected from room",
                        player.nickname, player.player_id
                    );
                }
                info!(
                    "[GAME] Client #{} disconnected (total: {})",
                    conn_id,
                    state.clients.len()
                );
            }

            GameEvent::Message { conn_id, message } => {
                handle_message(conn_id, &message, &mut state, &config);
            }
        }
    }

    info!("[GAME] Game loop ended");
}

fn handle_message(
    conn_id: ConnectionId,
    msg: &ClientMessage,
    state: &mut ServerState,
    config: &ServerConfig,
) {
    match msg {
        ClientMessage::Ping => {
            info!("[GAME] #{} PING -> PONG", conn_id);
            send_to(&state.clients, conn_id, &ServerMessage::Pong);
        }

        ClientMessage::Hello {
            role,
            nickname,
            proto,
            auth,
        } => {
            handle_hello(conn_id, role, nickname, *proto, auth, state, config);
        }

        ClientMessage::Play { card } => {
            handle_play(conn_id, card, state);
        }
    }
}

fn handle_hello(
    conn_id: ConnectionId,
    role: &protocol::Role,
    nickname: &str,
    proto: u32,
    auth: &Option<String>,
    state: &mut ServerState,
    config: &ServerConfig,
) {
    info!("[GAME] #{} HELLO from {:?} '{}'", conn_id, role, nickname);

    let room = state.room_manager.get_or_create_waiting_room();

    if room.is_full() {
        send_to(
            &state.clients,
            conn_id,
            &ServerMessage::Error {
                code: ErrorCode::RoomFull,
                message: "Room is full".to_string(),
            },
        );
        return;
    }

    let result = process_hello(
        role,
        nickname,
        proto,
        auth,
        room.get_nicknames(),
        room.next_slot(),
        &room.id,
        config.ai_auth_token.as_deref(),
    );

    match result {
        HandshakeResult::Success(welcome_msg) => {
            if let ServerMessage::Welcome {
                player_id,
                nickname: final_nickname,
                room: room_id,
            } = &welcome_msg
            {
                let room_id_clone = room_id.clone();
                let player_count;
                let wait_msg;
                let conn_ids;
                let can_start;
                let seed;
                let players_data;

                {
                    room.add_player(conn_id, player_id, final_nickname, *role);
                    player_count = room.players.len();
                    wait_msg = room.room_wait_message();
                    conn_ids = room.conn_ids();
                    can_start = room.can_start();
                    seed = room.seed;

                    // 收集玩家資料用於建立 GameEngine
                    players_data = room
                        .players
                        .iter()
                        .map(|p| (p.conn_id, p.player_id.clone(), p.team.unwrap_or(protocol::Team::Human)))
                        .collect::<Vec<_>>();
                }

                state.room_manager.associate_conn(conn_id, &room_id_clone);
                state.conn_to_room.insert(conn_id, room_id_clone.clone());

                send_to(&state.clients, conn_id, &welcome_msg);

                info!(
                    "[LOBBY] Player '{}' ({}) joined room {} ({}/4 players)",
                    final_nickname, player_id, room_id, player_count
                );

                for &cid in &conn_ids {
                    send_to(&state.clients, cid, &wait_msg);
                }

                if can_start {
                    if let Some(room) = state.room_manager.get_room_mut(&room_id_clone) {
                        room.state = RoomState::Playing;
                        room.assign_teams();

                        let start_msg = room.room_start_message();
                        let conn_ids = room.conn_ids();

                        // 收集更新後的玩家資料 (含 team)
                        let players_with_teams: Vec<_> = room
                            .players
                            .iter()
                            .map(|p| (p.conn_id, p.player_id.clone(), p.team.unwrap_or(protocol::Team::Human)))
                            .collect();

                        info!("[LOBBY] Room {} starting game with seed {}", room_id_clone, seed);

                        for &cid in &conn_ids {
                            send_to(&state.clients, cid, &start_msg);
                        }

                        // 建立 GameEngine 並發牌
                        start_game(&room_id_clone, seed, players_with_teams, state);
                    }
                }
            }
        }
        HandshakeResult::Error(error_msg) => {
            warn!("[LOBBY] Handshake failed for #{}: {:?}", conn_id, error_msg);
            send_to(&state.clients, conn_id, &error_msg);
        }
    }
}

fn start_game(
    room_id: &str,
    seed: u64,
    players: Vec<(ConnectionId, String, protocol::Team)>,
    state: &mut ServerState,
) {
    info!("[ENGINE] Creating game engine for room {}", room_id);

    let mut engine = GameEngine::new(seed, players);

    // 發牌
    let deal_messages = engine.deal();
    for (conn_id, msg) in deal_messages {
        send_to(&state.clients, conn_id, &msg);
        info!("[ENGINE] Sent DEAL to connection #{}", conn_id);
    }

    // 發送 YOUR_TURN 給第一位玩家
    if let Some(current_idx) = engine.current_player_idx() {
        let your_turn_msg = engine.your_turn_message(current_idx);
        let current_conn_id = engine.players[current_idx].conn_id;
        send_to(&state.clients, current_conn_id, &your_turn_msg);
        info!(
            "[ENGINE] Sent YOUR_TURN to {} (trick {})",
            engine.players[current_idx].player_id, engine.current_trick
        );
    }

    state.games.insert(room_id.to_string(), engine);
}

fn handle_play(conn_id: ConnectionId, card: &str, state: &mut ServerState) {
    // 找到該連線所屬的遊戲
    let room_id = match state.conn_to_room.get(&conn_id) {
        Some(id) => id.clone(),
        None => {
            warn!("[ENGINE] #{} tried to play but not in any room", conn_id);
            send_to(
                &state.clients,
                conn_id,
                &ServerMessage::Error {
                    code: ErrorCode::ProtocolError,
                    message: "Not in a game".to_string(),
                },
            );
            return;
        }
    };

    let engine = match state.games.get_mut(&room_id) {
        Some(e) => e,
        None => {
            warn!("[ENGINE] #{} tried to play but game not found", conn_id);
            send_to(
                &state.clients,
                conn_id,
                &ServerMessage::Error {
                    code: ErrorCode::NotYourTurn,
                    message: "Game not started".to_string(),
                },
            );
            return;
        }
    };

    // 驗證出牌
    let (player_idx, card_data) = match engine.validate_play(conn_id, card) {
        Ok(result) => result,
        Err(e) => {
            let (code, reason) = match e {
                PlayError::NotYourTurn => (ErrorCode::NotYourTurn, RejectReason::NotYourTurn),
                PlayError::NotInHand => (ErrorCode::InvalidMove, RejectReason::NotInHand),
                PlayError::NotLegal => (ErrorCode::InvalidMove, RejectReason::NotLegal),
                PlayError::InvalidCard => (ErrorCode::InvalidMove, RejectReason::NotInHand),
                PlayError::NotInGame => (ErrorCode::ProtocolError, RejectReason::NotYourTurn),
            };

            info!("[ENGINE] #{} PLAY {} rejected: {:?}", conn_id, card, e);
            send_to(
                &state.clients,
                conn_id,
                &ServerMessage::PlayReject {
                    card: card.to_string(),
                    reason,
                },
            );
            return;
        }
    };

    info!(
        "[ENGINE] {} plays {} (trick {})",
        engine.players[player_idx].player_id, card, engine.current_trick
    );

    // 執行出牌
    let play_result = engine.play_card(player_idx, card_data);

    match play_result {
        PlayResult::Continue(broadcast_msg, next_idx) => {
            // 廣播出牌
            for &cid in &engine.all_conn_ids() {
                send_to(&state.clients, cid, &broadcast_msg);
            }

            // 發送 YOUR_TURN 給下一位玩家
            let your_turn_msg = engine.your_turn_message(next_idx);
            let next_conn_id = engine.players[next_idx].conn_id;
            send_to(&state.clients, next_conn_id, &your_turn_msg);
            info!(
                "[ENGINE] YOUR_TURN -> {} (trick {})",
                engine.players[next_idx].player_id, engine.current_trick
            );
        }

        PlayResult::TrickComplete(broadcast_msg) => {
            // 廣播出牌
            for &cid in &engine.all_conn_ids() {
                send_to(&state.clients, cid, &broadcast_msg);
            }

            // 結算 trick
            let resolution = engine.resolve_trick();

            match resolution {
                TrickResolution::NextTrick(result_msg, next_idx) => {
                    // 廣播 TRICK_RESULT
                    for &cid in &engine.all_conn_ids() {
                        send_to(&state.clients, cid, &result_msg);
                    }

                    info!(
                        "[ENGINE] Trick {} complete, winner: {}, score: HUMAN={} AI={}",
                        engine.current_trick - 1,
                        engine.players[next_idx].player_id,
                        engine.score.human,
                        engine.score.ai
                    );

                    // 發送 YOUR_TURN 給下一 trick 的第一位 (上一 trick 贏家)
                    let your_turn_msg = engine.your_turn_message(next_idx);
                    let next_conn_id = engine.players[next_idx].conn_id;
                    send_to(&state.clients, next_conn_id, &your_turn_msg);
                    info!(
                        "[ENGINE] YOUR_TURN -> {} (trick {})",
                        engine.players[next_idx].player_id, engine.current_trick
                    );
                }

                TrickResolution::GameOver(result_msg) => {
                    // 廣播最後一個 TRICK_RESULT
                    for &cid in &engine.all_conn_ids() {
                        send_to(&state.clients, cid, &result_msg);
                    }

                    // 廣播 GAME_OVER
                    let game_over_msg = engine.game_over_message();
                    for &cid in &engine.all_conn_ids() {
                        send_to(&state.clients, cid, &game_over_msg);
                    }

                    info!(
                        "[ENGINE] Game over! Final score: HUMAN={} AI={}, Winner: {:?}",
                        engine.score.human,
                        engine.score.ai,
                        if engine.score.human > engine.score.ai {
                            "HUMAN"
                        } else {
                            "AI"
                        }
                    );

                    // 移除遊戲 (可選: 保留用於重播)
                    // state.games.remove(&room_id);
                }
            }
        }
    }
}

fn send_to(clients: &HashMap<ConnectionId, ClientSender>, conn_id: ConnectionId, msg: &ServerMessage) {
    if let Some(sender) = clients.get(&conn_id) {
        if sender.send(msg.clone()).is_err() {
            warn!("[GAME] Failed to send to #{}", conn_id);
        }
    }
}

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
