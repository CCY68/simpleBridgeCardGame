//! Admin Commands - 管理指令處理
//!
//! 處理來自管理介面的指令

use super::logger::{EventType, GameLogger};
use std::sync::mpsc;

/// Admin 事件 (傳送給 Game Loop)
#[derive(Debug, Clone)]
pub enum AdminEvent {
    /// 請求伺服器狀態
    GetStatus { reply_tx: mpsc::Sender<AdminResponse> },
    /// 請求房間列表
    GetRooms { reply_tx: mpsc::Sender<AdminResponse> },
    /// 請求玩家列表
    GetPlayers { reply_tx: mpsc::Sender<AdminResponse> },
    /// 踢除玩家
    KickPlayer {
        player_id: String,
        reply_tx: mpsc::Sender<AdminResponse>,
    },
    /// 重設房間
    ResetRoom {
        room_id: Option<String>,
        reply_tx: mpsc::Sender<AdminResponse>,
    },
}

/// Admin 回應
#[derive(Debug, Clone)]
pub enum AdminResponse {
    /// 狀態資訊
    Status {
        total_connections: usize,
        total_rooms: usize,
        games_in_progress: usize,
    },
    /// 房間列表
    Rooms(Vec<RoomInfo>),
    /// 玩家列表
    Players(Vec<PlayerInfo>),
    /// 操作成功
    Ok(String),
    /// 操作失敗
    Error(String),
}

/// 房間資訊
#[derive(Debug, Clone)]
pub struct RoomInfo {
    pub id: String,
    pub state: String,
    pub player_count: usize,
    pub human_count: usize,
}

/// 玩家資訊
#[derive(Debug, Clone)]
pub struct PlayerInfo {
    pub player_id: String,
    pub nickname: String,
    pub room_id: String,
    pub role: String,
    pub is_ai: bool,
}

/// 指令解析結果
pub enum ParsedCommand {
    Help,
    Auth(String),
    Status,
    Rooms,
    Players,
    Logs(usize, Option<EventType>),
    Kick(String),
    Reset(Option<String>),
    Quit,
    Unknown(String),
}

/// 解析指令
pub fn parse_command(input: &str) -> ParsedCommand {
    let input = input.trim();
    let parts: Vec<&str> = input.split_whitespace().collect();

    if parts.is_empty() {
        return ParsedCommand::Unknown(String::new());
    }

    let cmd = parts[0].to_uppercase();

    match cmd.as_str() {
        "HELP" | "?" => ParsedCommand::Help,
        "AUTH" => {
            if parts.len() < 2 {
                ParsedCommand::Unknown("AUTH requires a token".to_string())
            } else {
                ParsedCommand::Auth(parts[1].to_string())
            }
        }
        "STATUS" => ParsedCommand::Status,
        "ROOMS" => ParsedCommand::Rooms,
        "PLAYERS" => ParsedCommand::Players,
        "LOGS" => {
            let count = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(20);
            let event_type = parts.get(2).and_then(|s| EventType::from_str(s));
            ParsedCommand::Logs(count, event_type)
        }
        "KICK" => {
            if parts.len() < 2 {
                ParsedCommand::Unknown("KICK requires a player_id".to_string())
            } else {
                ParsedCommand::Kick(parts[1].to_string())
            }
        }
        "RESET" => {
            let room_id = parts.get(1).map(|s| s.to_string());
            ParsedCommand::Reset(room_id)
        }
        "QUIT" | "EXIT" | "BYE" => ParsedCommand::Quit,
        _ => ParsedCommand::Unknown(format!("Unknown command: {}", cmd)),
    }
}

/// 產生 HELP 訊息
pub fn help_message() -> String {
    r#"
=== CardArena Admin Console ===

Commands:
  AUTH <token>       Authenticate with admin token (required first)
  HELP               Show this help message
  STATUS             Show server status
  ROOMS              List all rooms
  PLAYERS            List all players
  LOGS [n] [type]    Show recent n logs (default: 20)
                     Types: PLAYER_JOIN, PLAYER_LEAVE, GAME_START,
                            GAME_END, PLAY, TRICK_RESULT, ADMIN, ERROR
  KICK <player_id>   Kick a player (e.g., KICK P1)
  RESET [room_id]    Reset a room (e.g., RESET R001)
  QUIT               Disconnect from admin console

Examples:
  AUTH my_secret_token
  LOGS 50
  LOGS 10 PLAY
  KICK P1
  RESET R001
"#
    .to_string()
}

/// 格式化狀態回應
pub fn format_status(response: &AdminResponse) -> String {
    match response {
        AdminResponse::Status {
            total_connections,
            total_rooms,
            games_in_progress,
        } => {
            format!(
                r#"
=== Server Status ===
Total Connections: {}
Total Rooms: {}
Games In Progress: {}
"#,
                total_connections, total_rooms, games_in_progress
            )
        }
        _ => "Invalid response".to_string(),
    }
}

/// 格式化房間列表
pub fn format_rooms(response: &AdminResponse) -> String {
    match response {
        AdminResponse::Rooms(rooms) => {
            if rooms.is_empty() {
                return "No rooms found.".to_string();
            }

            let mut output = String::from("\n=== Rooms ===\n");
            output.push_str(&format!(
                "{:<8} {:<12} {:<10} {:<10}\n",
                "ID", "State", "Players", "Humans"
            ));
            output.push_str(&"-".repeat(42));
            output.push('\n');

            for room in rooms {
                output.push_str(&format!(
                    "{:<8} {:<12} {:<10} {:<10}\n",
                    room.id, room.state, room.player_count, room.human_count
                ));
            }
            output
        }
        _ => "Invalid response".to_string(),
    }
}

/// 格式化玩家列表
pub fn format_players(response: &AdminResponse) -> String {
    match response {
        AdminResponse::Players(players) => {
            if players.is_empty() {
                return "No players found.".to_string();
            }

            let mut output = String::from("\n=== Players ===\n");
            output.push_str(&format!(
                "{:<8} {:<16} {:<8} {:<8} {:<6}\n",
                "ID", "Nickname", "Room", "Role", "AI"
            ));
            output.push_str(&"-".repeat(50));
            output.push('\n');

            for player in players {
                output.push_str(&format!(
                    "{:<8} {:<16} {:<8} {:<8} {:<6}\n",
                    player.player_id,
                    player.nickname,
                    player.room_id,
                    player.role,
                    if player.is_ai { "Yes" } else { "No" }
                ));
            }
            output
        }
        _ => "Invalid response".to_string(),
    }
}

/// 格式化日誌
pub fn format_logs(logger: &GameLogger, count: usize, event_type: Option<EventType>) -> String {
    let entries = match event_type {
        Some(et) => logger.get_recent_by_type(count, et),
        None => logger.get_recent(count),
    };

    if entries.is_empty() {
        return "No logs found.".to_string();
    }

    let mut output = String::from("\n=== Logs ===\n");
    for entry in entries {
        output.push_str(&entry.format());
        output.push('\n');
    }
    output
}

/// 格式化操作結果
pub fn format_result(response: &AdminResponse) -> String {
    match response {
        AdminResponse::Ok(msg) => format!("OK: {}", msg),
        AdminResponse::Error(msg) => format!("ERROR: {}", msg),
        _ => "Invalid response".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_help() {
        match parse_command("HELP") {
            ParsedCommand::Help => {}
            _ => panic!("Expected Help"),
        }

        match parse_command("?") {
            ParsedCommand::Help => {}
            _ => panic!("Expected Help"),
        }
    }

    #[test]
    fn test_parse_auth() {
        match parse_command("AUTH my_token") {
            ParsedCommand::Auth(token) => assert_eq!(token, "my_token"),
            _ => panic!("Expected Auth"),
        }
    }

    #[test]
    fn test_parse_logs() {
        match parse_command("LOGS") {
            ParsedCommand::Logs(n, None) => assert_eq!(n, 20),
            _ => panic!("Expected Logs"),
        }

        match parse_command("LOGS 50") {
            ParsedCommand::Logs(n, None) => assert_eq!(n, 50),
            _ => panic!("Expected Logs"),
        }

        match parse_command("LOGS 10 PLAY") {
            ParsedCommand::Logs(n, Some(EventType::Play)) => assert_eq!(n, 10),
            _ => panic!("Expected Logs with type"),
        }
    }

    #[test]
    fn test_parse_kick() {
        match parse_command("KICK P1") {
            ParsedCommand::Kick(id) => assert_eq!(id, "P1"),
            _ => panic!("Expected Kick"),
        }
    }

    #[test]
    fn test_parse_reset() {
        match parse_command("RESET") {
            ParsedCommand::Reset(None) => {}
            _ => panic!("Expected Reset without room"),
        }

        match parse_command("RESET R001") {
            ParsedCommand::Reset(Some(id)) => assert_eq!(id, "R001"),
            _ => panic!("Expected Reset with room"),
        }
    }
}
