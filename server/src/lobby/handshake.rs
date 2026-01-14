use crate::protocol::{ErrorCode, PlayerInfo, Role, ServerMessage};
use std::collections::HashSet;

const MIN_NICKNAME_LEN: usize = 1;
const MAX_NICKNAME_LEN: usize = 16;
const PROTOCOL_VERSION: u32 = 1;

/// Handshake 結果
pub enum HandshakeResult {
    /// 成功，回傳歡迎訊息
    Success(ServerMessage),
    /// 失敗，回傳錯誤訊息
    Error(ServerMessage),
}

/// 驗證並處理 HELLO 訊息
pub fn process_hello(
    role: &Role,
    nickname: &str,
    proto: u32,
    auth: &Option<String>,
    existing_nicknames: &HashSet<String>,
    player_slot: u32,
    room_id: &str,
    ai_auth_token: Option<&str>,
) -> HandshakeResult {
    // 驗證協議版本
    if proto != PROTOCOL_VERSION {
        return HandshakeResult::Error(ServerMessage::Error {
            code: ErrorCode::InvalidHello,
            message: format!(
                "Unsupported protocol version: {}. Expected: {}",
                proto, PROTOCOL_VERSION
            ),
        });
    }

    // 驗證暱稱長度
    let nickname_len = nickname.chars().count();
    if nickname_len < MIN_NICKNAME_LEN || nickname_len > MAX_NICKNAME_LEN {
        return HandshakeResult::Error(ServerMessage::Error {
            code: ErrorCode::InvalidHello,
            message: format!(
                "Nickname must be {}-{} characters, got: {}",
                MIN_NICKNAME_LEN, MAX_NICKNAME_LEN, nickname_len
            ),
        });
    }

    // AI 角色需要驗證 token
    if *role == Role::Ai {
        if let Some(expected_token) = ai_auth_token {
            match auth {
                Some(token) if token == expected_token => {}
                Some(_) => {
                    return HandshakeResult::Error(ServerMessage::Error {
                        code: ErrorCode::AuthFailed,
                        message: "Invalid AI authentication token".to_string(),
                    });
                }
                None => {
                    return HandshakeResult::Error(ServerMessage::Error {
                        code: ErrorCode::AuthFailed,
                        message: "AI client requires authentication token".to_string(),
                    });
                }
            }
        }
        // 如果沒有設定 ai_auth_token，則不需驗證 (開發模式)
    }

    // 處理暱稱重複
    let final_nickname = ensure_unique_nickname(nickname, existing_nicknames);

    // 分配 player_id
    let player_id = format!("P{}", player_slot);

    HandshakeResult::Success(ServerMessage::Welcome {
        player_id,
        nickname: final_nickname,
        room: room_id.to_string(),
    })
}

/// 確保暱稱唯一，若重複則加後綴
fn ensure_unique_nickname(nickname: &str, existing: &HashSet<String>) -> String {
    if !existing.contains(nickname) {
        return nickname.to_string();
    }

    // 加上數字後綴
    for i in 2..=99 {
        let new_name = format!("{}_{}", nickname, i);
        if !existing.contains(&new_name) {
            return new_name;
        }
    }

    // 最後手段：加上隨機字串
    format!("{}_{:04x}", nickname, rand_u16())
}

/// 簡易隨機數產生 (不依賴 rand crate)
fn rand_u16() -> u16 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    (duration.subsec_nanos() & 0xFFFF) as u16
}

/// 建立 PlayerInfo 結構
pub fn create_player_info(player_id: &str, nickname: &str, role: Role) -> PlayerInfo {
    PlayerInfo {
        id: player_id.to_string(),
        nickname: nickname.to_string(),
        role,
        team: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_hello() {
        let existing = HashSet::new();
        let result = process_hello(
            &Role::Human,
            "Alice",
            1,
            &None,
            &existing,
            1,
            "R001",
            None,
        );
        match result {
            HandshakeResult::Success(ServerMessage::Welcome {
                player_id,
                nickname,
                ..
            }) => {
                assert_eq!(player_id, "P1");
                assert_eq!(nickname, "Alice");
            }
            _ => panic!("Expected success"),
        }
    }

    #[test]
    fn test_invalid_nickname_too_short() {
        let existing = HashSet::new();
        let result =
            process_hello(&Role::Human, "", 1, &None, &existing, 1, "R001", None);
        assert!(matches!(result, HandshakeResult::Error(_)));
    }

    #[test]
    fn test_invalid_nickname_too_long() {
        let existing = HashSet::new();
        let long_name = "A".repeat(20);
        let result =
            process_hello(&Role::Human, &long_name, 1, &None, &existing, 1, "R001", None);
        assert!(matches!(result, HandshakeResult::Error(_)));
    }

    #[test]
    fn test_duplicate_nickname() {
        let mut existing = HashSet::new();
        existing.insert("Alice".to_string());

        let result =
            process_hello(&Role::Human, "Alice", 1, &None, &existing, 2, "R001", None);
        match result {
            HandshakeResult::Success(ServerMessage::Welcome { nickname, .. }) => {
                assert!(nickname.starts_with("Alice_"));
                assert_ne!(nickname, "Alice");
            }
            _ => panic!("Expected success with modified nickname"),
        }
    }

    #[test]
    fn test_ai_without_token() {
        let existing = HashSet::new();
        let result = process_hello(
            &Role::Ai,
            "Bot1",
            1,
            &None,
            &existing,
            1,
            "R001",
            Some("secret123"),
        );
        assert!(matches!(result, HandshakeResult::Error(_)));
    }

    #[test]
    fn test_ai_with_valid_token() {
        let existing = HashSet::new();
        let result = process_hello(
            &Role::Ai,
            "Bot1",
            1,
            &Some("secret123".to_string()),
            &existing,
            1,
            "R001",
            Some("secret123"),
        );
        assert!(matches!(result, HandshakeResult::Success(_)));
    }

    #[test]
    fn test_wrong_protocol_version() {
        let existing = HashSet::new();
        let result =
            process_hello(&Role::Human, "Alice", 99, &None, &existing, 1, "R001", None);
        assert!(matches!(result, HandshakeResult::Error(_)));
    }
}
