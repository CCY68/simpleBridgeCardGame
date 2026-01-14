use serde::{Deserialize, Serialize};

/// 玩家角色
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Role {
    Human,
    Ai,
}

/// 隊伍
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(rename_all = "UPPERCASE")]
pub enum Team {
    Human,
    Ai,
}

/// 玩家 ID (P1-P4)
pub type PlayerId = String;

/// 房間 ID
pub type RoomId = String;

/// 撲克牌表示 (e.g., "AS", "10H", "KC")
pub type Card = String;

/// 錯誤代碼
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    InvalidHello,
    AuthFailed,
    RoomFull,
    InvalidMove,
    NotYourTurn,
    ProtocolError,
    Timeout,
}

/// 出牌被拒原因
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RejectReason {
    NotInHand,
    NotLegal,
    NotYourTurn,
}

/// 房間中的玩家資訊
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerInfo {
    pub id: PlayerId,
    pub nickname: String,
    pub role: Role,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub team: Option<Team>,
}

/// 桌面上的出牌資訊
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TablePlay {
    pub player_id: PlayerId,
    pub card: Card,
}

/// Trick 歷史記錄
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrickHistory {
    pub trick: u32,
    pub winner: PlayerId,
    pub cards: Vec<Card>,
}

/// 分數
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Score {
    #[serde(rename = "HUMAN")]
    pub human: u32,
    #[serde(rename = "AI")]
    pub ai: u32,
}

/// 客戶端到伺服器的訊息
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    /// 連線握手
    #[serde(rename = "HELLO")]
    Hello {
        role: Role,
        nickname: String,
        proto: u32,
        #[serde(skip_serializing_if = "Option::is_none")]
        auth: Option<String>,
    },

    /// 出牌
    #[serde(rename = "PLAY")]
    Play { card: Card },

    /// Ping (用於測試)
    #[serde(rename = "PING")]
    Ping,
}

/// 伺服器到客戶端的訊息
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServerMessage {
    /// 歡迎訊息
    #[serde(rename = "WELCOME")]
    Welcome {
        player_id: PlayerId,
        nickname: String,
        room: RoomId,
    },

    /// 錯誤訊息
    #[serde(rename = "ERROR")]
    Error { code: ErrorCode, message: String },

    /// 等待其他玩家
    #[serde(rename = "ROOM_WAIT")]
    RoomWait {
        room: RoomId,
        players: Vec<PlayerInfo>,
        need: u32,
    },

    /// 遊戲開始
    #[serde(rename = "ROOM_START")]
    RoomStart {
        room: RoomId,
        players: Vec<PlayerInfo>,
        seed: u64,
    },

    /// 發牌
    #[serde(rename = "DEAL")]
    Deal {
        hand: Vec<Card>,
        total_tricks: u32,
    },

    /// 輪到你出牌
    #[serde(rename = "YOUR_TURN")]
    YourTurn {
        trick: u32,
        table: Vec<TablePlay>,
        legal: Vec<Card>,
        timeout_ms: u32,
    },

    /// 廣播出牌
    #[serde(rename = "PLAY_BROADCAST")]
    PlayBroadcast {
        player_id: PlayerId,
        card: Card,
        trick: u32,
    },

    /// 出牌被拒絕
    #[serde(rename = "PLAY_REJECT")]
    PlayReject { card: Card, reason: RejectReason },

    /// Trick 結果
    #[serde(rename = "TRICK_RESULT")]
    TrickResult {
        trick: u32,
        plays: Vec<TablePlay>,
        winner: PlayerId,
        score: Score,
    },

    /// 遊戲結束
    #[serde(rename = "GAME_OVER")]
    GameOver {
        final_score: Score,
        winner: Team,
        history: Vec<TrickHistory>,
    },

    /// Pong (用於測試)
    #[serde(rename = "PONG")]
    Pong,
}

/// UDP Heartbeat Ping (Client → Server)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatPing {
    #[serde(rename = "type")]
    pub msg_type: String, // "HB_PING"
    pub seq: u64,
    pub t_client_ms: u64,
}

/// UDP Heartbeat Pong (Server → Client)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatPong {
    #[serde(rename = "type")]
    pub msg_type: String, // "HB_PONG"
    pub seq: u64,
    pub t_client_ms: u64,
    pub t_server_ms: u64,
}

impl HeartbeatPong {
    pub fn from_ping(ping: &HeartbeatPing, server_time: u64) -> Self {
        Self {
            msg_type: "HB_PONG".to_string(),
            seq: ping.seq,
            t_client_ms: ping.t_client_ms,
            t_server_ms: server_time,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hello_serialize() {
        let msg = ClientMessage::Hello {
            role: Role::Human,
            nickname: "Alice".to_string(),
            proto: 1,
            auth: None,
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"HELLO\""));
        assert!(json.contains("\"role\":\"HUMAN\""));
    }

    #[test]
    fn test_hello_deserialize() {
        let json = r#"{"type":"HELLO","role":"HUMAN","nickname":"Alice","proto":1}"#;
        let msg: ClientMessage = serde_json::from_str(json).unwrap();
        match msg {
            ClientMessage::Hello {
                role, nickname, proto, ..
            } => {
                assert_eq!(role, Role::Human);
                assert_eq!(nickname, "Alice");
                assert_eq!(proto, 1);
            }
            _ => panic!("Expected Hello message"),
        }
    }

    #[test]
    fn test_welcome_serialize() {
        let msg = ServerMessage::Welcome {
            player_id: "P1".to_string(),
            nickname: "Alice".to_string(),
            room: "R001".to_string(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"WELCOME\""));
        assert!(json.contains("\"player_id\":\"P1\""));
    }

    #[test]
    fn test_error_serialize() {
        let msg = ServerMessage::Error {
            code: ErrorCode::InvalidHello,
            message: "Missing nickname".to_string(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"code\":\"INVALID_HELLO\""));
    }

    #[test]
    fn test_ping_deserialize() {
        let json = r#"{"type":"PING"}"#;
        let msg: ClientMessage = serde_json::from_str(json).unwrap();
        assert!(matches!(msg, ClientMessage::Ping));
    }
}
