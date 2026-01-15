//! AI 玩家定義

use crate::protocol::{PlayerId, Team};

/// 內建 AI 玩家
///
/// 與真人玩家不同，AI 玩家不需要 TCP 連線，
/// 其出牌決策由 Server 端的 Strategy 處理。
#[derive(Debug, Clone)]
pub struct AiPlayer {
    /// 玩家 ID (P3 或 P4)
    pub player_id: PlayerId,
    /// 暱稱
    pub nickname: String,
    /// 隊伍 (固定為 AI) - 預留供未來擴充
    #[allow(dead_code)]
    pub team: Team,
}

impl AiPlayer {
    /// 建立 AI Partner 1 (P3 位置)
    pub fn partner1() -> Self {
        Self {
            player_id: "P3".to_string(),
            nickname: "AI_Partner1".to_string(),
            team: Team::Ai,
        }
    }

    /// 建立 AI Partner 2 (P4 位置)
    pub fn partner2() -> Self {
        Self {
            player_id: "P4".to_string(),
            nickname: "AI_Partner2".to_string(),
            team: Team::Ai,
        }
    }

    /// 建立 AI 玩家對 (Partner 1 & 2)
    pub fn create_partners() -> (Self, Self) {
        (Self::partner1(), Self::partner2())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_partners() {
        let (p1, p2) = AiPlayer::create_partners();

        assert_eq!(p1.player_id, "P3");
        assert_eq!(p2.player_id, "P4");
        assert_eq!(p1.team, Team::Ai);
        assert_eq!(p2.team, Team::Ai);
    }
}
