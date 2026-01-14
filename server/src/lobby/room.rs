use crate::net::ConnectionId;
use crate::protocol::{PlayerInfo, Role, RoomId, ServerMessage, Team};
use std::collections::{HashMap, HashSet};

const MAX_PLAYERS: usize = 4;

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
}

impl Room {
    /// 建立新房間
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            state: RoomState::Waiting,
            players: Vec::with_capacity(MAX_PLAYERS),
            nicknames: HashSet::new(),
            seed: generate_seed(),
        }
    }

    /// 檢查房間是否已滿
    pub fn is_full(&self) -> bool {
        self.players.len() >= MAX_PLAYERS
    }

    /// 取得下一個 player slot (1-4)
    pub fn next_slot(&self) -> u32 {
        (self.players.len() + 1) as u32
    }

    /// 取得所有暱稱
    pub fn get_nicknames(&self) -> &HashSet<String> {
        &self.nicknames
    }

    /// 新增玩家
    pub fn add_player(&mut self, conn_id: ConnectionId, player_id: &str, nickname: &str, role: Role) {
        let player = Player {
            conn_id,
            player_id: player_id.to_string(),
            nickname: nickname.to_string(),
            role,
            team: None,
        };
        self.nicknames.insert(nickname.to_string());
        self.players.push(player);
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
        (MAX_PLAYERS - self.players.len()) as u32
    }

    /// 檢查是否可以開始遊戲
    /// 條件：4 人，且至少 1 個 HUMAN
    pub fn can_start(&self) -> bool {
        if self.players.len() != MAX_PLAYERS {
            return false;
        }
        // 至少需要 1 個 HUMAN
        self.players.iter().any(|p| p.role == Role::Human)
    }

    /// 分配隊伍
    pub fn assign_teams(&mut self) {
        // HUMAN 和 AI 各一隊
        // 如果混合情況，依加入順序分配
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
}

impl RoomManager {
    pub fn new() -> Self {
        Self {
            rooms: HashMap::new(),
            conn_to_room: HashMap::new(),
            next_room_id: 1,
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
        let room = Room::new(&room_id);
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
        let mut manager = RoomManager::new();

        let room = manager.get_or_create_waiting_room();
        let room_id = room.id.clone();
        room.add_player(1, "P1", "Alice", Role::Human);
        manager.associate_conn(1, &room_id);

        assert!(manager.get_room_for_conn(1).is_some());
        assert!(manager.get_room(&room_id).is_some());
    }
}
