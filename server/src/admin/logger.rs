//! Admin Logger - 遊戲事件記錄器
//!
//! 使用 Ring Buffer 儲存最近的遊戲事件，供管理介面查詢。

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

/// 預設保留的訊息數量
const DEFAULT_CAPACITY: usize = 500;

/// 事件類型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventType {
    PlayerJoin,
    PlayerLeave,
    GameStart,
    GameEnd,
    Play,
    TrickResult,
    AdminAction,
    Error,
}

impl EventType {
    pub fn as_str(&self) -> &'static str {
        match self {
            EventType::PlayerJoin => "PLAYER_JOIN",
            EventType::PlayerLeave => "PLAYER_LEAVE",
            EventType::GameStart => "GAME_START",
            EventType::GameEnd => "GAME_END",
            EventType::Play => "PLAY",
            EventType::TrickResult => "TRICK_RESULT",
            EventType::AdminAction => "ADMIN",
            EventType::Error => "ERROR",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "PLAYER_JOIN" => Some(EventType::PlayerJoin),
            "PLAYER_LEAVE" => Some(EventType::PlayerLeave),
            "GAME_START" => Some(EventType::GameStart),
            "GAME_END" => Some(EventType::GameEnd),
            "PLAY" => Some(EventType::Play),
            "TRICK_RESULT" => Some(EventType::TrickResult),
            "ADMIN" => Some(EventType::AdminAction),
            "ERROR" => Some(EventType::Error),
            _ => None,
        }
    }
}

/// 日誌條目
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub timestamp: u64,
    pub event_type: EventType,
    pub message: String,
}

impl LogEntry {
    pub fn new(event_type: EventType, message: impl Into<String>) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        Self {
            timestamp,
            event_type,
            message: message.into(),
        }
    }

    /// 格式化為顯示字串
    pub fn format(&self) -> String {
        let datetime = format_timestamp(self.timestamp);
        format!("[{}] {}: {}", datetime, self.event_type.as_str(), self.message)
    }
}

/// 格式化時間戳記
fn format_timestamp(timestamp: u64) -> String {
    // 簡單格式化 (UTC)
    let secs = timestamp % 86400;
    let hours = secs / 3600;
    let mins = (secs % 3600) / 60;
    let secs = secs % 60;

    let days = timestamp / 86400;
    let year = 1970 + days / 365; // 簡化計算
    let day_of_year = days % 365;
    let month = day_of_year / 30 + 1;
    let day = day_of_year % 30 + 1;

    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        year, month, day, hours, mins, secs
    )
}

/// 日誌管理器 (線程安全)
#[derive(Clone)]
pub struct GameLogger {
    entries: Arc<Mutex<VecDeque<LogEntry>>>,
    capacity: usize,
}

impl GameLogger {
    /// 建立新的日誌管理器
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_CAPACITY)
    }

    /// 建立指定容量的日誌管理器
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            entries: Arc::new(Mutex::new(VecDeque::with_capacity(capacity))),
            capacity,
        }
    }

    /// 記錄事件
    pub fn log(&self, event_type: EventType, message: impl Into<String>) {
        let entry = LogEntry::new(event_type, message);

        if let Ok(mut entries) = self.entries.lock() {
            if entries.len() >= self.capacity {
                entries.pop_front();
            }
            entries.push_back(entry);
        }
    }

    /// 取得最近 n 條日誌
    pub fn get_recent(&self, n: usize) -> Vec<LogEntry> {
        if let Ok(entries) = self.entries.lock() {
            let start = entries.len().saturating_sub(n);
            entries.iter().skip(start).cloned().collect()
        } else {
            Vec::new()
        }
    }

    /// 取得最近 n 條指定類型的日誌
    pub fn get_recent_by_type(&self, n: usize, event_type: EventType) -> Vec<LogEntry> {
        if let Ok(entries) = self.entries.lock() {
            entries
                .iter()
                .filter(|e| e.event_type == event_type)
                .rev()
                .take(n)
                .cloned()
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect()
        } else {
            Vec::new()
        }
    }

    /// 搜尋包含關鍵字的日誌
    #[allow(dead_code)]
    pub fn search(&self, keyword: &str, limit: usize) -> Vec<LogEntry> {
        let keyword_lower = keyword.to_lowercase();
        if let Ok(entries) = self.entries.lock() {
            entries
                .iter()
                .filter(|e| e.message.to_lowercase().contains(&keyword_lower))
                .rev()
                .take(limit)
                .cloned()
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect()
        } else {
            Vec::new()
        }
    }

    /// 取得日誌總數
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.entries.lock().map(|e| e.len()).unwrap_or(0)
    }

    /// 清除所有日誌
    #[allow(dead_code)]
    pub fn clear(&self) {
        if let Ok(mut entries) = self.entries.lock() {
            entries.clear();
        }
    }
}

impl Default for GameLogger {
    fn default() -> Self {
        Self::new()
    }
}

// === 便捷記錄函數 ===

impl GameLogger {
    pub fn player_join(&self, player_id: &str, nickname: &str, room_id: &str) {
        self.log(
            EventType::PlayerJoin,
            format!("{} ({}) joined {}", nickname, player_id, room_id),
        );
    }

    pub fn player_leave(&self, player_id: &str, nickname: &str, room_id: &str) {
        self.log(
            EventType::PlayerLeave,
            format!("{} ({}) left {}", nickname, player_id, room_id),
        );
    }

    pub fn game_start(&self, room_id: &str, seed: u64) {
        self.log(
            EventType::GameStart,
            format!("{} started (seed: {})", room_id, seed),
        );
    }

    pub fn game_end(&self, room_id: &str, human_score: u32, ai_score: u32) {
        self.log(
            EventType::GameEnd,
            format!("{} ended (HUMAN: {}, AI: {})", room_id, human_score, ai_score),
        );
    }

    pub fn play(&self, player_id: &str, card: &str, trick: u32) {
        self.log(
            EventType::Play,
            format!("{} plays {} (trick {})", player_id, card, trick),
        );
    }

    pub fn trick_result(&self, winner: &str, trick: u32) {
        self.log(
            EventType::TrickResult,
            format!("{} wins trick {}", winner, trick),
        );
    }

    pub fn admin_action(&self, action: &str, detail: &str) {
        self.log(EventType::AdminAction, format!("{}: {}", action, detail));
    }

    /// 記錄錯誤訊息 - 預留供未來擴充
    #[allow(dead_code)]
    pub fn error(&self, message: &str) {
        self.log(EventType::Error, message.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_logger_basic() {
        let logger = GameLogger::new();

        logger.log(EventType::PlayerJoin, "Alice joined R001");
        logger.log(EventType::Play, "P1 plays 5H");

        let entries = logger.get_recent(10);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].event_type, EventType::PlayerJoin);
        assert_eq!(entries[1].event_type, EventType::Play);
    }

    #[test]
    fn test_logger_capacity() {
        let logger = GameLogger::with_capacity(3);

        logger.log(EventType::Play, "1");
        logger.log(EventType::Play, "2");
        logger.log(EventType::Play, "3");
        logger.log(EventType::Play, "4"); // 應該擠掉 "1"

        let entries = logger.get_recent(10);
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].message, "2");
    }

    #[test]
    fn test_logger_by_type() {
        let logger = GameLogger::new();

        logger.log(EventType::PlayerJoin, "join1");
        logger.log(EventType::Play, "play1");
        logger.log(EventType::PlayerJoin, "join2");
        logger.log(EventType::Play, "play2");

        let joins = logger.get_recent_by_type(10, EventType::PlayerJoin);
        assert_eq!(joins.len(), 2);
        assert!(joins.iter().all(|e| e.event_type == EventType::PlayerJoin));
    }

    #[test]
    fn test_convenience_methods() {
        let logger = GameLogger::new();

        logger.player_join("P1", "Alice", "R001");
        logger.play("P1", "5H", 1);
        logger.trick_result("P2", 1);
        logger.game_end("R001", 7, 6);

        let entries = logger.get_recent(10);
        assert_eq!(entries.len(), 4);
    }
}
