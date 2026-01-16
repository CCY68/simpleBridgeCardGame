mod admin;
mod ai;
mod game;
mod lobby;
mod net;
mod protocol;

use admin::{
    spawn_admin_server, AdminConfig, AdminEvent, AdminResponse, GameLogger, PlayerInfo, RoomInfo,
};
use ai::{AiStrategy, SmartStrategy};
use game::{CardData, GameEngine, PlayError, PlayResult, TrickResolution};
use lobby::{HandshakeResult, Room, RoomManager, RoomState, process_hello};
use log::{error, info, warn};
use net::{
    ClientSender, ConnectionId, GameEvent, create_event_channel, create_heartbeat_tracker,
    next_connection_id, spawn_handler, spawn_heartbeat_server,
};
use protocol::{ClientMessage, ErrorCode, RejectReason, RoomId, ServerMessage};
use std::collections::HashMap;
use std::env;
use std::net::SocketAddr;
use std::sync::mpsc;
use std::thread;

const DEFAULT_PORT: u16 = 8888;
const DEFAULT_UDP_PORT_OFFSET: u16 = 1; // UDP port = TCP port + 1
const DEFAULT_ADMIN_PORT_OFFSET: u16 = 2; // Admin port = TCP port + 2
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

    // 啟動 Admin Server
    let admin_port = port + DEFAULT_ADMIN_PORT_OFFSET;
    let (admin_tx, admin_rx) = mpsc::channel();
    let logger = GameLogger::new();

    let admin_config = AdminConfig {
        port: admin_port,
        ..AdminConfig::default()
    };

    match spawn_admin_server(admin_config, admin_tx, logger.clone()) {
        Ok(_) => {
            info!("[SERVER] Admin server started on port {}", admin_port);
        }
        Err(e) => {
            warn!(
                "[SERVER] Failed to start Admin server: {} (continuing without admin)",
                e
            );
        }
    }

    let config = ServerConfig::default();
    game_loop(event_rx, admin_rx, logger, config);
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

fn game_loop(
    event_rx: net::EventReceiver,
    admin_rx: mpsc::Receiver<AdminEvent>,
    logger: GameLogger,
    config: ServerConfig,
) {
    let mut state = ServerState::new();

    info!("[GAME] Game loop started");

    loop {
        // 先處理 admin 事件 (non-blocking)
        while let Ok(admin_event) = admin_rx.try_recv() {
            handle_admin_event(admin_event, &mut state, &logger);
        }

        // 處理 game 事件 (blocking with timeout)
        match event_rx.recv_timeout(std::time::Duration::from_millis(100)) {
            Ok(event) => match event {
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

                    // 取得該連線所在的房間
                    let room_id = state.conn_to_room.remove(&conn_id);

                    if let Some(room_id) = room_id {
                        // 處理 Bridge Mode 遊戲重啟
                        handle_bridge_mode_disconnect(conn_id, &room_id, &mut state, &logger);
                    } else if let Some(player) = state.room_manager.handle_disconnect(conn_id) {
                        info!(
                            "[GAME] Player '{}' ({}) disconnected from room",
                            player.nickname, player.player_id
                        );
                        logger.player_leave(&player.player_id, &player.nickname, "unknown");
                    }

                    info!(
                        "[GAME] Client #{} disconnected (total: {})",
                        conn_id,
                        state.clients.len()
                    );
                }

                GameEvent::Message { conn_id, message } => {
                    handle_message(conn_id, &message, &mut state, &logger, &config);
                }
            },
            Err(mpsc::RecvTimeoutError::Timeout) => {
                // Timeout, 繼續迴圈 (處理 admin 事件)
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                info!("[GAME] Event channel closed, shutting down");
                break;
            }
        }
    }

    info!("[GAME] Game loop ended");
}

fn handle_message(
    conn_id: ConnectionId,
    msg: &ClientMessage,
    state: &mut ServerState,
    logger: &GameLogger,
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
            handle_hello(conn_id, role, nickname, *proto, auth, state, logger, config);
        }

        ClientMessage::Play { card } => {
            handle_play(conn_id, card, state, logger);
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
    logger: &GameLogger,
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

                {
                    room.add_player(conn_id, player_id, final_nickname, *role);
                    player_count = room.players.len();
                    wait_msg = room.room_wait_message();
                    conn_ids = room.conn_ids();
                    can_start = room.can_start();
                    seed = room.seed;
                }

                state.room_manager.associate_conn(conn_id, &room_id_clone);
                state.conn_to_room.insert(conn_id, room_id_clone.clone());

                send_to(&state.clients, conn_id, &welcome_msg);

                info!(
                    "[LOBBY] Player '{}' ({}) joined room {} ({}/4 players)",
                    final_nickname, player_id, room_id, player_count
                );
                logger.player_join(player_id, final_nickname, room_id);

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
                        logger.game_start(&room_id_clone, seed);

                        for &cid in &conn_ids {
                            send_to(&state.clients, cid, &start_msg);
                        }

                        // 建立 GameEngine 並發牌
                        start_game(&room_id_clone, seed, players_with_teams, state, logger);
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
    logger: &GameLogger,
) {
    info!("[ENGINE] Creating game engine for room {}", room_id);

    let mut engine = GameEngine::new(seed, players);

    // 發牌 (只發給真人玩家)
    let deal_messages = engine.deal();
    for (conn_id, msg) in deal_messages {
        if !Room::is_virtual_conn(conn_id) {
            send_to(&state.clients, conn_id, &msg);
            info!("[ENGINE] Sent DEAL to connection #{}", conn_id);
        }
    }

    state.games.insert(room_id.to_string(), engine);

    // 處理回合 (如果第一位是 AI，自動出牌；否則發 YOUR_TURN 給 Human)
    process_ai_turns(room_id, state, logger);
}

fn handle_play(conn_id: ConnectionId, card: &str, state: &mut ServerState, logger: &GameLogger) {
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
            let reason = match e {
                PlayError::NotYourTurn => RejectReason::NotYourTurn,
                PlayError::NotInHand => RejectReason::NotInHand,
                PlayError::NotLegal => RejectReason::NotLegal,
                PlayError::InvalidCard => RejectReason::NotInHand,
                PlayError::NotInGame => RejectReason::NotYourTurn,
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

    let player_id = engine.players[player_idx].player_id.clone();
    let current_trick = engine.current_trick;

    info!(
        "[ENGINE] {} plays {} (trick {})",
        player_id, card, current_trick
    );
    logger.play(&player_id, card, current_trick);

    // 執行出牌
    let play_result = engine.play_card(player_idx, card_data);

    match play_result {
        PlayResult::Continue(broadcast_msg, _next_idx) => {
            // 廣播出牌給真人玩家
            broadcast_to_humans(&room_id, &broadcast_msg, state);

            // 處理下一位玩家回合 (可能是 AI 自動出牌)
            process_ai_turns(&room_id, state, logger);
        }

        PlayResult::TrickComplete(broadcast_msg) => {
            // 廣播出牌
            broadcast_to_humans(&room_id, &broadcast_msg, state);

            // 結算 trick
            let engine = state.games.get_mut(&room_id).unwrap();
            let resolution = engine.resolve_trick();

            match resolution {
                TrickResolution::NextTrick(result_msg, next_idx) => {
                    // 廣播 TRICK_RESULT
                    broadcast_to_humans(&room_id, &result_msg, state);

                    let engine = state.games.get(&room_id).unwrap();
                    let winner_id = engine.players[next_idx].player_id.clone();
                    let trick_num = engine.current_trick - 1;

                    info!(
                        "[ENGINE] Trick {} complete, winner: {}, score: HUMAN={} AI={}",
                        trick_num,
                        winner_id,
                        engine.score.human,
                        engine.score.ai
                    );
                    logger.trick_result(&winner_id, trick_num);

                    // 處理下一位玩家回合 (可能是 AI 自動出牌)
                    process_ai_turns(&room_id, state, logger);
                }

                TrickResolution::GameOver(result_msg) => {
                    // 廣播最後一個 TRICK_RESULT
                    broadcast_to_humans(&room_id, &result_msg, state);

                    // 廣播 GAME_OVER
                    let engine = state.games.get(&room_id).unwrap();
                    let game_over_msg = engine.game_over_message();
                    broadcast_to_humans(&room_id, &game_over_msg, state);

                    let human_score = engine.score.human;
                    let ai_score = engine.score.ai;

                    info!(
                        "[ENGINE] Game over! Final score: HUMAN={} AI={}, Winner: {:?}",
                        human_score,
                        ai_score,
                        if human_score > ai_score {
                            "HUMAN"
                        } else {
                            "AI"
                        }
                    );
                    logger.game_end(&room_id, human_score, ai_score);

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

/// 檢查並處理 AI 玩家的回合
/// 如果當前玩家是 AI，自動選擇並出牌，直到輪到 Human 或遊戲結束
fn process_ai_turns(room_id: &str, state: &mut ServerState, logger: &GameLogger) {
    let strategy = SmartStrategy::default();

    loop {
        let engine = match state.games.get_mut(room_id) {
            Some(e) => e,
            None => return,
        };

        // 取得當前玩家
        let current_idx = match engine.current_player_idx() {
            Some(idx) => idx,
            None => return, // 遊戲已結束或尚未開始
        };

        let current_conn_id = engine.players[current_idx].conn_id;

        // 檢查是否為 AI (虛擬連線)
        if !Room::is_virtual_conn(current_conn_id) {
            // Human 玩家，發送 YOUR_TURN 並結束 AI 處理迴圈
            let your_turn_msg = engine.your_turn_message(current_idx);
            send_to(&state.clients, current_conn_id, &your_turn_msg);
            info!(
                "[ENGINE] YOUR_TURN -> {} (trick {})",
                engine.players[current_idx].player_id, engine.current_trick
            );
            return;
        }

        // AI 玩家，自動出牌
        let player_id = engine.players[current_idx].player_id.clone();
        let hand = engine.players[current_idx].hand.clone();
        let legal_moves = engine.get_legal_moves(current_idx);
        let table: Vec<(usize, CardData)> = engine.table.clone();
        let is_leader = table.is_empty();
        let current_trick = engine.current_trick;

        // 使用策略選擇牌
        let chosen_card = strategy.choose_card(&hand, &legal_moves, &table, is_leader);
        let card_str = chosen_card.to_protocol_string();

        info!(
            "[AI] {} chooses {} (trick {}, is_leader={})",
            player_id, card_str, current_trick, is_leader
        );
        logger.play(&player_id, &card_str, current_trick);

        // 執行出牌
        let play_result = engine.play_card(current_idx, chosen_card);

        match play_result {
            PlayResult::Continue(broadcast_msg, _next_idx) => {
                // 廣播出牌給所有真人玩家
                broadcast_to_humans(room_id, &broadcast_msg, state);
                // 繼續迴圈處理下一位玩家
            }

            PlayResult::TrickComplete(broadcast_msg) => {
                // 廣播出牌
                broadcast_to_humans(room_id, &broadcast_msg, state);

                // 結算 trick
                let engine = state.games.get_mut(room_id).unwrap();
                let resolution = engine.resolve_trick();

                match resolution {
                    TrickResolution::NextTrick(result_msg, next_idx) => {
                        // 廣播 TRICK_RESULT
                        broadcast_to_humans(room_id, &result_msg, state);

                        let engine = state.games.get(room_id).unwrap();
                        let winner_id = engine.players[next_idx].player_id.clone();
                        let trick_num = engine.current_trick - 1;

                        info!(
                            "[ENGINE] Trick {} complete, winner: {}, score: HUMAN={} AI={}",
                            trick_num,
                            winner_id,
                            engine.score.human,
                            engine.score.ai
                        );
                        logger.trick_result(&winner_id, trick_num);
                        // 繼續迴圈處理下一位玩家
                    }

                    TrickResolution::GameOver(result_msg) => {
                        // 廣播最後一個 TRICK_RESULT
                        broadcast_to_humans(room_id, &result_msg, state);

                        // 廣播 GAME_OVER
                        let engine = state.games.get(room_id).unwrap();
                        let game_over_msg = engine.game_over_message();
                        broadcast_to_humans(room_id, &game_over_msg, state);

                        let human_score = engine.score.human;
                        let ai_score = engine.score.ai;

                        info!(
                            "[ENGINE] Game over! Final score: HUMAN={} AI={}, Winner: {:?}",
                            human_score,
                            ai_score,
                            if human_score > ai_score {
                                "HUMAN"
                            } else {
                                "AI"
                            }
                        );
                        logger.game_end(room_id, human_score, ai_score);
                        return;
                    }
                }
            }
        }
    }
}

/// 廣播訊息給房間內的所有真人玩家
fn broadcast_to_humans(room_id: &str, msg: &ServerMessage, state: &ServerState) {
    if let Some(engine) = state.games.get(room_id) {
        for player in &engine.players {
            if !Room::is_virtual_conn(player.conn_id) {
                send_to(&state.clients, player.conn_id, msg);
            }
        }
    }
}

/// 處理 Bridge Mode 下的玩家斷線
/// 如果遊戲進行中有玩家斷線，重置遊戲等待新玩家
fn handle_bridge_mode_disconnect(
    conn_id: ConnectionId,
    room_id: &str,
    state: &mut ServerState,
    logger: &GameLogger,
) {
    // 取得房間資訊
    let room = match state.room_manager.get_room_mut(room_id) {
        Some(r) => r,
        None => return,
    };

    let player = room.remove_player(conn_id);
    let is_bridge_mode = room.bridge_mode;
    let was_playing = room.state == RoomState::Playing;

    if let Some(ref p) = player {
        info!(
            "[GAME] Player '{}' ({}) disconnected from room {}",
            p.nickname, p.player_id, room_id
        );
        logger.player_leave(&p.player_id, &p.nickname, room_id);
    }

    // Bridge Mode 且遊戲進行中，需要重置
    if is_bridge_mode && was_playing {
        info!("[BRIDGE] Human disconnected during game, resetting room {}", room_id);

        // 移除遊戲引擎
        state.games.remove(room_id);

        // 重置房間
        let room = state.room_manager.get_room_mut(room_id).unwrap();
        let removed_humans = room.reset_for_bridge_mode();

        // 清除被移除 Human 的 conn_to_room 對應
        for human_conn in &removed_humans {
            state.conn_to_room.remove(human_conn);
        }

        // 通知剩餘的 Human (如果有的話)
        // 注意：斷線的玩家已經不在了，另一個 Human 可能還連線中
        let wait_msg = room.room_wait_message();
        for player in &room.players {
            if !Room::is_virtual_conn(player.conn_id) {
                // 發送遊戲重置通知
                let reset_msg = ServerMessage::Error {
                    code: protocol::ErrorCode::ProtocolError,
                    message: "Game reset due to player disconnect. Waiting for players...".to_string(),
                };
                send_to(&state.clients, player.conn_id, &reset_msg);
                send_to(&state.clients, player.conn_id, &wait_msg);
            }
        }

        info!(
            "[BRIDGE] Room {} reset, waiting for {} more human(s)",
            room_id,
            room.players_needed()
        );
    }
}

/// 處理 Admin 事件
fn handle_admin_event(event: AdminEvent, state: &mut ServerState, logger: &GameLogger) {
    match event {
        AdminEvent::GetStatus { reply_tx } => {
            let total_connections = state.clients.len();
            let total_rooms = state.room_manager.rooms_count();
            let games_in_progress = state.games.len();

            let _ = reply_tx.send(AdminResponse::Status {
                total_connections,
                total_rooms,
                games_in_progress,
            });
        }

        AdminEvent::GetRooms { reply_tx } => {
            let rooms = state.room_manager.get_all_rooms_info();
            let room_infos: Vec<RoomInfo> = rooms
                .iter()
                .map(|(id, room_state, player_count, human_count)| RoomInfo {
                    id: id.clone(),
                    state: room_state.clone(),
                    player_count: *player_count,
                    human_count: *human_count,
                })
                .collect();

            let _ = reply_tx.send(AdminResponse::Rooms(room_infos));
        }

        AdminEvent::GetPlayers { reply_tx } => {
            let players = state.room_manager.get_all_players_info();
            let player_infos: Vec<PlayerInfo> = players
                .iter()
                .map(|(player_id, nickname, room_id, role, is_ai)| PlayerInfo {
                    player_id: player_id.clone(),
                    nickname: nickname.clone(),
                    room_id: room_id.clone(),
                    role: role.clone(),
                    is_ai: *is_ai,
                })
                .collect();

            let _ = reply_tx.send(AdminResponse::Players(player_infos));
        }

        AdminEvent::KickPlayer { player_id, reply_tx } => {
            // 找到玩家的連線 ID
            if let Some((conn_id, room_id)) = state.room_manager.find_player_conn(&player_id) {
                if Room::is_virtual_conn(conn_id) {
                    let _ = reply_tx.send(AdminResponse::Error("Cannot kick AI player".to_string()));
                    return;
                }

                // 關閉該玩家的連線 (會觸發 Disconnected 事件)
                if let Some(sender) = state.clients.get(&conn_id) {
                    // 發送斷線訊息
                    let _ = sender.send(ServerMessage::Error {
                        code: ErrorCode::ProtocolError,
                        message: "You have been kicked by admin".to_string(),
                    });
                }

                // 移除連線
                state.clients.remove(&conn_id);
                state.conn_to_room.remove(&conn_id);

                // 處理房間
                if let Some(room) = state.room_manager.get_room_mut(&room_id) {
                    let was_playing = room.state == RoomState::Playing;

                    if let Some(player) = room.remove_player(conn_id) {
                        logger.admin_action("KICK", &format!("Kicked {} from {}", player.nickname, room_id));

                        if room.bridge_mode && was_playing {
                            // 重置房間
                            state.games.remove(&room_id);
                            room.reset_for_bridge_mode();
                        }
                    }
                }

                let _ = reply_tx.send(AdminResponse::Ok(format!(
                    "Player {} kicked from {}",
                    player_id, room_id
                )));
            } else {
                let _ = reply_tx.send(AdminResponse::Error(format!(
                    "Player {} not found",
                    player_id
                )));
            }
        }

        AdminEvent::ResetRoom { room_id, reply_tx } => {
            match room_id {
                Some(rid) => {
                    if let Some(room) = state.room_manager.get_room_mut(&rid) {
                        if room.state != RoomState::Playing {
                            let _ = reply_tx.send(AdminResponse::Error(format!(
                                "Room {} is not in playing state",
                                rid
                            )));
                            return;
                        }

                        // 移除遊戲引擎
                        state.games.remove(&rid);

                        // 通知玩家
                        for player in &room.players {
                            if !Room::is_virtual_conn(player.conn_id) {
                                send_to(
                                    &state.clients,
                                    player.conn_id,
                                    &ServerMessage::Error {
                                        code: ErrorCode::ProtocolError,
                                        message: "Game reset by admin".to_string(),
                                    },
                                );
                            }
                        }

                        // 重置房間
                        if room.bridge_mode {
                            room.reset_for_bridge_mode();
                        } else {
                            room.state = RoomState::Waiting;
                        }

                        logger.admin_action("RESET", &format!("Reset room {}", rid));
                        let _ = reply_tx.send(AdminResponse::Ok(format!("Room {} reset successfully", rid)));
                    } else {
                        let _ = reply_tx.send(AdminResponse::Error(format!("Room {} not found", rid)));
                    }
                }
                None => {
                    // Reset all rooms
                    let room_ids: Vec<String> = state.games.keys().cloned().collect();
                    let mut reset_count = 0;

                    for rid in room_ids {
                        state.games.remove(&rid);
                        if let Some(room) = state.room_manager.get_room_mut(&rid) {
                            for player in &room.players {
                                if !Room::is_virtual_conn(player.conn_id) {
                                    send_to(
                                        &state.clients,
                                        player.conn_id,
                                        &ServerMessage::Error {
                                            code: ErrorCode::ProtocolError,
                                            message: "Game reset by admin".to_string(),
                                        },
                                    );
                                }
                            }

                            if room.bridge_mode {
                                room.reset_for_bridge_mode();
                            } else {
                                room.state = RoomState::Waiting;
                            }
                            reset_count += 1;
                        }
                    }

                    logger.admin_action("RESET", &format!("Reset {} rooms", reset_count));
                    let _ = reply_tx.send(AdminResponse::Ok(format!("{} rooms reset", reset_count)));
                }
            }
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
