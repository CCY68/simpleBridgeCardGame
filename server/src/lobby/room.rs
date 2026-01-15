use crate::ai::AiPlayer;
use crate::net::ConnectionId;
use crate::protocol::{PlayerInfo, Role, RoomId, ServerMessage, Team};
use std::collections::{HashMap, HashSet};

const MAX_PLAYERS: usize = 4;
const REQUIRED_HUMANS: usize = 2;

/// 虛擬連線 ID (用於內建 AI 玩家)
/// AI 玩家不佔用真實 TCP 連線
pub const AI_VIRTUAL_CONN_ID_1: ConnectionId = ConnectionId::MAX - 1;
pub const AI_VIRTUAL_CONN_ID_2: ConnectionId = ConnectionId::MAX;

/// 玩家狀態
#[derive(Debug, Clone)]
pub struct Player {
    pub conn_id: ConnectionId,
    pub player_id: String,
    pub nickname: String,
    pub role: Role,
    pub team: Option<Team>,
}

/// 房間狀態
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoomState {
    /// 等待玩家加入
    Waiting,
    /// 遊戲進行中
    Playing,
    /// 遊戲結束
    #[allow(dead_code)]
    Finished,
}

/// 房間
pub struct Room {
    pub id: RoomId,
    pub state: RoomState,
    pub players: Vec<Player>,
    pub nicknames: HashSet<String>,
    pub seed: u64,
    /// Bridge Mode: Server 內建 2 AI，等待 2 Human 加入
    pub bridge_mode: bool,
}

impl Room {
    /// 建立新房間 (傳統模式: 等待 4 人)
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            state: RoomState::Waiting,
            players: Vec::with_capacity(MAX_PLAYERS),
            nicknames: HashSet::new(),
            seed: generate_seed(),
            bridge_mode: false,
        }
    }

    /// 建立 Bridge Mode 房間 (內建 2 AI，等待 2 Human)
    pub fn new_bridge_mode(id: impl Into<String>) -> Self {
        let mut room = Self {
            id: id.into(),
            state: RoomState::Waiting,
            players: Vec::with_capacity(MAX_PLAYERS),
            nicknames: HashSet::new(),
            seed: generate_seed(),
            bridge_mode: true,
        };

        // 預先加入 2 個內建 AI (P3, P4 位置)
        let (ai1, ai2) = AiPlayer::create_partners();
        room.add_builtin_ai(&ai1, AI_VIRTUAL_CONN_ID_1);
        room.add_builtin_ai(&ai2, AI_VIRTUAL_CONN_ID_2);

        room
    }

    /// 加入內建 AI 玩家
    fn add_builtin_ai(&mut self, ai: &AiPlayer, virtual_conn_id: ConnectionId) {
        let player = Player {
            conn_id: virtual_conn_id,
            player_id: ai.player_id.clone(),
            nickname: ai.nickname.clone(),
            role: Role::Ai,
            team: Some(Team::Ai), // AI 隊伍固定
        };
        self.nicknames.insert(ai.nickname.clone());
        self.players.push(player);
    }

    /// 檢查房間是否已滿
    pub fn is_full(&self) -> bool {
        if self.bridge_mode {
            // Bridge Mode: 2 humans = full
            self.human_count() >= REQUIRED_HUMANS
        } else {
            self.players.len() >= MAX_PLAYERS
        }
    }

    /// 取得下一個 player slot (1-4)
    pub fn next_slot(&self) -> u32 {
        if self.bridge_mode {
            // Bridge Mode: Human 分配 P1, P2
            (self.human_count() + 1) as u32
        } else {
            (self.players.len() + 1) as u32
        }
    }

    /// 計算人類玩家數量
    pub fn human_count(&self) -> usize {
        self.players.iter().filter(|p| p.role == Role::Human).count()
    }

    /// 取得所有暱稱
    pub fn get_nicknames(&self) -> &HashSet<String> {
        &self.nicknames
    }

    /// 新增玩家
    pub fn add_player(&mut self, conn_id: ConnectionId, player_id: &str, nickname: &str, role: Role) {
        let team = if self.bridge_mode && role == Role::Human {
            Some(Team::Human) // Bridge Mode: Human 固定 Human 隊伍
        } else {
            None
        };

        let player = Player {
            conn_id,
            player_id: player_id.to_string(),
            nickname: nickname.to_string(),
            role,
            team,
        };
        self.nicknames.insert(nickname.to_string());

        if self.bridge_mode {
            // Bridge Mode: Human 插入到 AI 前面 (維持 P1, P2, P3, P4 順序)
            let insert_pos = self.human_count();
            self.players.insert(insert_pos, player);
        } else {
            self.players.push(player);
        }
    }

    /// 移除玩家 (透過 conn_id)
    pub fn remove_player(&mut self, conn_id: ConnectionId) -> Option<Player> {
        if let Some(pos) = self.players.iter().position(|p| p.conn_id == conn_id) {
            let player = self.players.remove(pos);
            self.nicknames.remove(&player.nickname);
            Some(player)
        } else {
            None
        }
    }

    /// 取得需要的玩家數量
    pub fn players_needed(&self) -> u32 {
        if self.bridge_mode {
            // Bridge Mode: 需要幾個 Human
            (REQUIRED_HUMANS - self.human_count()) as u32
        } else {
            (MAX_PLAYERS - self.players.len()) as u32
        }
    }

    /// 檢查是否可以開始遊戲
    pub fn can_start(&self) -> bool {
        if self.bridge_mode {
            // Bridge Mode: 2 Humans 即可開始
            self.human_count() >= REQUIRED_HUMANS
        } else {
            // 傳統模式: 4 人且至少 1 個 HUMAN
            if self.players.len() != MAX_PLAYERS {
                return false;
            }
            self.players.iter().any(|p| p.role == Role::Human)
        }
    }

    /// 分配隊伍
    pub fn assign_teams(&mut self) {
        if self.bridge_mode {
            // Bridge Mode: 隊伍已在加入時分配，無需處理
            return;
        }

        // 傳統模式: HUMAN 和 AI 各一隊
        // 依加入順序分配 (前 2 人 Human 隊，後 2 人 AI 隊)
        for (i, player) in self.players.iter_mut().enumerate() {
            player.team = Some(if i < 2 { Team::Human } else { Team::Ai });
        }
    }

    /// 產生 ROOM_WAIT 訊息
    pub fn room_wait_message(&self) -> ServerMessage {
        ServerMessage::RoomWait {
            room: self.id.clone(),
            players: self.players.iter().map(|p| p.to_player_info()).collect(),
            need: self.players_needed(),
        }
    }

    /// 產生 ROOM_START 訊息
    pub fn room_start_message(&self) -> ServerMessage {
        ServerMessage::RoomStart {
            room: self.id.clone(),
            players: self.players.iter().map(|p| p.to_player_info()).collect(),
            seed: self.seed,
        }
    }

    /// 取得所有連線 ID
    pub fn conn_ids(&self) -> Vec<ConnectionId> {
        self.players.iter().map(|p| p.conn_id).collect()
    }

    /// 取得真實連線 ID (排除 AI 虛擬連線) - 預留供未來擴充
    #[allow(dead_code)]
    pub fn real_conn_ids(&self) -> Vec<ConnectionId> {
        self.players
            .iter()
            .filter(|p| !Self::is_virtual_conn(p.conn_id))
            .map(|p| p.conn_id)
            .collect()
    }

    /// 檢查是否為虛擬連線 (內建 AI)
    pub fn is_virtual_conn(conn_id: ConnectionId) -> bool {
        conn_id == AI_VIRTUAL_CONN_ID_1 || conn_id == AI_VIRTUAL_CONN_ID_2
    }

    /// 檢查 player_id 是否為內建 AI - 預留供未來擴充
    #[allow(dead_code)]
    pub fn is_builtin_ai(&self, player_id: &str) -> bool {
        player_id == "P3" || player_id == "P4"
    }

    /// 透過 conn_id 找玩家
    #[allow(dead_code)]
    pub fn find_player(&self, conn_id: ConnectionId) -> Option<&Player> {
        self.players.iter().find(|p| p.conn_id == conn_id)
    }

    /// 透過 player_id 找玩家
    #[allow(dead_code)]
    pub fn find_player_by_id(&self, player_id: &str) -> Option<&Player> {
        self.players.iter().find(|p| p.player_id == player_id)
    }

    /// 重置房間 (Bridge Mode 專用)
    /// 移除所有 Human 玩家，保留 AI，重置狀態為 Waiting
    pub fn reset_for_bridge_mode(&mut self) -> Vec<ConnectionId> {
        if !self.bridge_mode {
            return vec![];
        }

        // 收集要移除的 Human 連線 ID
        let human_conn_ids: Vec<ConnectionId> = self
            .players
            .iter()
            .filter(|p| p.role == Role::Human)
            .map(|p| p.conn_id)
            .collect();

        // 移除所有 Human 玩家
        self.players.retain(|p| p.role == Role::Ai);

        // 清除 Human 暱稱
        self.nicknames.retain(|n| n.starts_with("AI_"));

        // 重置狀態
        self.state = RoomState::Waiting;

        // 重新產生 seed
        self.seed = generate_seed();

        human_conn_ids
    }
}

impl Player {
    /// 轉換為 PlayerInfo (用於協議訊息)
    pub fn to_player_info(&self) -> PlayerInfo {
        PlayerInfo {
            id: self.player_id.clone(),
            nickname: self.nickname.clone(),
            role: self.role,
            team: self.team,
        }
    }
}

/// 房間管理器
pub struct RoomManager {
    rooms: HashMap<RoomId, Room>,
    conn_to_room: HashMap<ConnectionId, RoomId>,
    next_room_id: u32,
    /// 是否啟用 Bridge Mode (預設 true)
    pub bridge_mode: bool,
}

impl RoomManager {
    pub fn new() -> Self {
        Self {
            rooms: HashMap::new(),
            conn_to_room: HashMap::new(),
            next_room_id: 1,
            bridge_mode: true, // 預設啟用 Bridge Mode
        }
    }

    /// 建立傳統模式的 RoomManager
    #[allow(dead_code)]
    pub fn new_classic() -> Self {
        Self {
            rooms: HashMap::new(),
            conn_to_room: HashMap::new(),
            next_room_id: 1,
            bridge_mode: false,
        }
    }

    /// 取得或建立等待中的房間
    pub fn get_or_create_waiting_room(&mut self) -> &mut Room {
        // 找尋等待中且未滿的房間
        let waiting_room_id = self
            .rooms
            .iter()
            .find(|(_, r)| r.state == RoomState::Waiting && !r.is_full())
            .map(|(id, _)| id.clone());

        if let Some(id) = waiting_room_id {
            return self.rooms.get_mut(&id).unwrap();
        }

        // 建立新房間
        let room_id = format!("R{:03}", self.next_room_id);
        self.next_room_id += 1;
        let room = if self.bridge_mode {
            Room::new_bridge_mode(&room_id)
        } else {
            Room::new(&room_id)
        };
        self.rooms.insert(room_id.clone(), room);
        self.rooms.get_mut(&room_id).unwrap()
    }

    /// 將連線關聯到房間
    pub fn associate_conn(&mut self, conn_id: ConnectionId, room_id: &str) {
        self.conn_to_room.insert(conn_id, room_id.to_string());
    }

    /// 取得連線所在的房間
    #[allow(dead_code)]
    pub fn get_room_for_conn(&self, conn_id: ConnectionId) -> Option<&Room> {
        self.conn_to_room
            .get(&conn_id)
            .and_then(|room_id| self.rooms.get(room_id))
    }

    /// 取得連線所在的房間 (mutable)
    #[allow(dead_code)]
    pub fn get_room_for_conn_mut(&mut self, conn_id: ConnectionId) -> Option<&mut Room> {
        if let Some(room_id) = self.conn_to_room.get(&conn_id).cloned() {
            self.rooms.get_mut(&room_id)
        } else {
            None
        }
    }

    /// 處理連線斷開
    pub fn handle_disconnect(&mut self, conn_id: ConnectionId) -> Option<Player> {
        if let Some(room_id) = self.conn_to_room.remove(&conn_id) {
            if let Some(room) = self.rooms.get_mut(&room_id) {
                return room.remove_player(conn_id);
            }
        }
        None
    }

    /// 透過 ID 取得房間
    #[allow(dead_code)]
    pub fn get_room(&self, room_id: &str) -> Option<&Room> {
        self.rooms.get(room_id)
    }

    /// 透過 ID 取得房間 (mutable)
    pub fn get_room_mut(&mut self, room_id: &str) -> Option<&mut Room> {
        self.rooms.get_mut(room_id)
    }
}

impl Default for RoomManager {
    fn default() -> Self {
        Self::new()
    }
}

/// 產生隨機 seed
fn generate_seed() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(12345)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_room_creation() {
        let room = Room::new("R001");
        assert_eq!(room.id, "R001");
        assert_eq!(room.state, RoomState::Waiting);
        assert!(room.players.is_empty());
        assert!(!room.is_full());
        assert!(!room.bridge_mode);
    }

    #[test]
    fn test_add_players() {
        let mut room = Room::new("R001");

        room.add_player(1, "P1", "Alice", Role::Human);
        assert_eq!(room.players.len(), 1);
        assert_eq!(room.next_slot(), 2);

        room.add_player(2, "P2", "Bob", Role::Human);
        room.add_player(3, "P3", "Bot1", Role::Ai);
        room.add_player(4, "P4", "Bot2", Role::Ai);

        assert!(room.is_full());
        assert!(room.can_start());
    }

    #[test]
    fn test_cannot_start_without_human() {
        let mut room = Room::new("R001");
        room.add_player(1, "P1", "Bot1", Role::Ai);
        room.add_player(2, "P2", "Bot2", Role::Ai);
        room.add_player(3, "P3", "Bot3", Role::Ai);
        room.add_player(4, "P4", "Bot4", Role::Ai);

        assert!(room.is_full());
        assert!(!room.can_start()); // 沒有 HUMAN 不能開始
    }

    #[test]
    fn test_room_manager() {
        let mut manager = RoomManager::new_classic();

        let room = manager.get_or_create_waiting_room();
        let room_id = room.id.clone();
        room.add_player(1, "P1", "Alice", Role::Human);
        manager.associate_conn(1, &room_id);

        assert!(manager.get_room_for_conn(1).is_some());
        assert!(manager.get_room(&room_id).is_some());
    }

    // ========== Bridge Mode Tests ==========

    #[test]
    fn test_bridge_mode_room_creation() {
        let room = Room::new_bridge_mode("R001");

        assert!(room.bridge_mode);
        assert_eq!(room.players.len(), 2); // AI 已預先加入
        assert_eq!(room.players[0].player_id, "P3");
        assert_eq!(room.players[1].player_id, "P4");
        assert_eq!(room.players[0].team, Some(Team::Ai));
        assert_eq!(room.human_count(), 0);
        assert!(!room.is_full()); // 需要 2 Humans
        assert_eq!(room.players_needed(), 2);
    }

    #[test]
    fn test_bridge_mode_add_humans() {
        let mut room = Room::new_bridge_mode("R001");

        // 加入第一個 Human
        room.add_player(1, "P1", "Alice", Role::Human);
        assert_eq!(room.players.len(), 3);
        assert_eq!(room.human_count(), 1);
        assert_eq!(room.players[0].player_id, "P1"); // Human 在前
        assert_eq!(room.players[0].team, Some(Team::Human));
        assert!(!room.is_full());
        assert!(!room.can_start());

        // 加入第二個 Human
        room.add_player(2, "P2", "Bob", Role::Human);
        assert_eq!(room.players.len(), 4);
        assert_eq!(room.human_count(), 2);
        assert_eq!(room.players[1].player_id, "P2"); // Human 在前
        assert!(room.is_full());
        assert!(room.can_start());

        // 驗證順序: P1, P2 (Human), P3, P4 (AI)
        assert_eq!(room.players[0].player_id, "P1");
        assert_eq!(room.players[1].player_id, "P2");
        assert_eq!(room.players[2].player_id, "P3");
        assert_eq!(room.players[3].player_id, "P4");
    }

    #[test]
    fn test_bridge_mode_real_conn_ids() {
        let mut room = Room::new_bridge_mode("R001");
        room.add_player(1, "P1", "Alice", Role::Human);
        room.add_player(2, "P2", "Bob", Role::Human);

        let all_conns = room.conn_ids();
        let real_conns = room.real_conn_ids();

        assert_eq!(all_conns.len(), 4);
        assert_eq!(real_conns.len(), 2);
        assert!(real_conns.contains(&1));
        assert!(real_conns.contains(&2));
    }

    #[test]
    fn test_bridge_mode_manager() {
        let mut manager = RoomManager::new(); // Bridge mode by default

        let room = manager.get_or_create_waiting_room();
        assert!(room.bridge_mode);
        assert_eq!(room.players.len(), 2); // AI 已加入
    }

    #[test]
    fn test_bridge_mode_reset() {
        let mut room = Room::new_bridge_mode("R001");

        // 加入 2 個 Human
        room.add_player(1, "P1", "Alice", Role::Human);
        room.add_player(2, "P2", "Bob", Role::Human);
        room.state = RoomState::Playing;

        assert_eq!(room.players.len(), 4);
        assert_eq!(room.human_count(), 2);

        // 重置
        let removed = room.reset_for_bridge_mode();

        assert_eq!(removed.len(), 2);
        assert!(removed.contains(&1));
        assert!(removed.contains(&2));
        assert_eq!(room.players.len(), 2); // 只剩 AI
        assert_eq!(room.human_count(), 0);
        assert_eq!(room.state, RoomState::Waiting);
        assert_eq!(room.players_needed(), 2);
    }
}
