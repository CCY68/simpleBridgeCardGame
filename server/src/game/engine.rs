use super::deck::{CardData, Deck};
use crate::net::ConnectionId;
use crate::protocol::{Card, PlayerId, Score, ServerMessage, TablePlay, Team, TrickHistory};

const CARDS_PER_PLAYER: usize = 13;
const TOTAL_TRICKS: usize = 13;
const TIMEOUT_MS: u32 = 30000; // 30 秒

/// 遊戲階段
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GamePhase {
    /// 等待發牌
    WaitingToDeal,
    /// 等待玩家出牌
    WaitingForPlay {
        current_player_idx: usize,
    },
    /// Trick 結束，等待下一 trick
    TrickComplete,
    /// 遊戲結束
    GameOver,
}

/// 玩家在遊戲中的狀態
#[derive(Debug, Clone)]
pub struct GamePlayer {
    pub conn_id: ConnectionId,
    pub player_id: PlayerId,
    pub team: Team,
    pub hand: Vec<CardData>,
}

/// 遊戲引擎
pub struct GameEngine {
    /// 遊戲種子
    pub seed: u64,
    /// 玩家列表 (順序固定: P1, P2, P3, P4)
    pub players: Vec<GamePlayer>,
    /// 當前遊戲階段
    pub phase: GamePhase,
    /// 當前 trick 編號 (1-based)
    pub current_trick: u32,
    /// 當前 trick 的出牌
    pub table: Vec<(usize, CardData)>, // (player_idx, card)
    /// 分數
    pub score: Score,
    /// Trick 歷史
    pub history: Vec<TrickHistory>,
    /// 上一 trick 的贏家 index (用於決定下一 trick 誰先出)
    pub last_trick_winner: Option<usize>,
}

impl GameEngine {
    /// 建立新遊戲
    pub fn new(seed: u64, players: Vec<(ConnectionId, PlayerId, Team)>) -> Self {
        let game_players: Vec<GamePlayer> = players
            .into_iter()
            .map(|(conn_id, player_id, team)| GamePlayer {
                conn_id,
                player_id,
                team,
                hand: Vec::new(),
            })
            .collect();

        Self {
            seed,
            players: game_players,
            phase: GamePhase::WaitingToDeal,
            current_trick: 0,
            table: Vec::new(),
            score: Score::default(),
            history: Vec::new(),
            last_trick_winner: None,
        }
    }

    /// 發牌
    pub fn deal(&mut self) -> Vec<(ConnectionId, ServerMessage)> {
        let mut deck = Deck::new();
        deck.shuffle(self.seed);

        let hands = deck.deal(4, CARDS_PER_PLAYER);

        // 分配手牌給玩家
        for (i, hand) in hands.into_iter().enumerate() {
            self.players[i].hand = hand;
            // 排序手牌 (方便玩家閱讀)
            self.players[i].hand.sort_by_key(|c| (c.suit as u8, c.rank.0));
        }

        // 設定遊戲狀態
        self.current_trick = 1;
        self.phase = GamePhase::WaitingForPlay {
            current_player_idx: 0, // P1 先出
        };

        // 產生 DEAL 訊息
        self.players
            .iter()
            .map(|p| {
                let hand: Vec<Card> = p.hand.iter().map(|c| c.to_protocol_string()).collect();
                let msg = ServerMessage::Deal {
                    hand,
                    total_tricks: TOTAL_TRICKS as u32,
                };
                (p.conn_id, msg)
            })
            .collect()
    }

    /// 取得當前應該出牌的玩家 index
    pub fn current_player_idx(&self) -> Option<usize> {
        match &self.phase {
            GamePhase::WaitingForPlay { current_player_idx } => Some(*current_player_idx),
            _ => None,
        }
    }

    /// 取得當前應該出牌的玩家 conn_id
    #[allow(dead_code)]
    pub fn current_player_conn_id(&self) -> Option<ConnectionId> {
        self.current_player_idx().map(|idx| self.players[idx].conn_id)
    }

    /// 產生 YOUR_TURN 訊息
    pub fn your_turn_message(&self, player_idx: usize) -> ServerMessage {
        let legal = self.get_legal_moves(player_idx);

        let table: Vec<TablePlay> = self
            .table
            .iter()
            .map(|(idx, card)| TablePlay {
                player_id: self.players[*idx].player_id.clone(),
                card: card.to_protocol_string(),
            })
            .collect();

        ServerMessage::YourTurn {
            trick: self.current_trick,
            table,
            legal: legal.iter().map(|c| c.to_protocol_string()).collect(),
            timeout_ms: TIMEOUT_MS,
        }
    }

    /// 取得合法出牌
    pub fn get_legal_moves(&self, player_idx: usize) -> Vec<CardData> {
        let hand = &self.players[player_idx].hand;

        if self.table.is_empty() {
            // 第一個出牌的人可以出任何牌
            return hand.clone();
        }

        // 取得領牌花色
        let lead_suit = self.table[0].1.suit;

        // 檢查手中是否有同花色的牌
        let same_suit: Vec<CardData> = hand.iter().filter(|c| c.suit == lead_suit).copied().collect();

        if same_suit.is_empty() {
            // 沒有同花色，可以出任何牌
            hand.clone()
        } else {
            // 必須跟牌
            same_suit
        }
    }

    /// 驗證出牌是否合法
    pub fn validate_play(&self, conn_id: ConnectionId, card_str: &str) -> Result<(usize, CardData), PlayError> {
        // 找到玩家
        let player_idx = self
            .players
            .iter()
            .position(|p| p.conn_id == conn_id)
            .ok_or(PlayError::NotInGame)?;

        // 檢查是否輪到該玩家
        let current_idx = self.current_player_idx().ok_or(PlayError::NotYourTurn)?;
        if player_idx != current_idx {
            return Err(PlayError::NotYourTurn);
        }

        // 解析卡牌
        let card = CardData::from_protocol_string(card_str).ok_or(PlayError::InvalidCard)?;

        // 檢查牌是否在手牌中
        if !self.players[player_idx].hand.contains(&card) {
            return Err(PlayError::NotInHand);
        }

        // 檢查是否符合跟牌規則
        let legal_moves = self.get_legal_moves(player_idx);
        if !legal_moves.contains(&card) {
            return Err(PlayError::NotLegal);
        }

        Ok((player_idx, card))
    }

    /// 執行出牌
    pub fn play_card(&mut self, player_idx: usize, card: CardData) -> PlayResult {
        // 從手牌移除
        let hand = &mut self.players[player_idx].hand;
        let pos = hand.iter().position(|c| *c == card).unwrap();
        hand.remove(pos);

        // 加入桌面
        self.table.push((player_idx, card));

        // 產生 broadcast 訊息
        let broadcast = ServerMessage::PlayBroadcast {
            player_id: self.players[player_idx].player_id.clone(),
            card: card.to_protocol_string(),
            trick: self.current_trick,
        };

        // 檢查是否 4 人都出完
        if self.table.len() == 4 {
            self.phase = GamePhase::TrickComplete;
            PlayResult::TrickComplete(broadcast)
        } else {
            // 下一位玩家
            let next_idx = (player_idx + 1) % 4;
            self.phase = GamePhase::WaitingForPlay {
                current_player_idx: next_idx,
            };
            PlayResult::Continue(broadcast, next_idx)
        }
    }

    /// 結算 Trick
    pub fn resolve_trick(&mut self) -> TrickResolution {
        // 判定 winner
        let lead_suit = self.table[0].1.suit;
        let winner_idx = self
            .table
            .iter()
            .filter(|(_, card)| card.suit == lead_suit)
            .max_by_key(|(_, card)| card.rank.0)
            .map(|(idx, _)| *idx)
            .unwrap();

        let winner_team = self.players[winner_idx].team;

        // 更新分數 (每 trick 1 分)
        match winner_team {
            Team::Human => self.score.human += 1,
            Team::Ai => self.score.ai += 1,
        }

        // 記錄歷史
        let trick_history = TrickHistory {
            trick: self.current_trick,
            winner: self.players[winner_idx].player_id.clone(),
            cards: self.table.iter().map(|(_, c)| c.to_protocol_string()).collect(),
        };
        self.history.push(trick_history);

        // 產生 TRICK_RESULT 訊息
        let result_msg = ServerMessage::TrickResult {
            trick: self.current_trick,
            plays: self
                .table
                .iter()
                .map(|(idx, card)| TablePlay {
                    player_id: self.players[*idx].player_id.clone(),
                    card: card.to_protocol_string(),
                })
                .collect(),
            winner: self.players[winner_idx].player_id.clone(),
            score: self.score.clone(),
        };

        // 清除桌面
        self.table.clear();
        self.last_trick_winner = Some(winner_idx);

        // 檢查是否遊戲結束
        if self.current_trick >= TOTAL_TRICKS as u32 {
            self.phase = GamePhase::GameOver;
            TrickResolution::GameOver(result_msg)
        } else {
            // 下一 trick
            self.current_trick += 1;
            self.phase = GamePhase::WaitingForPlay {
                current_player_idx: winner_idx,
            };
            TrickResolution::NextTrick(result_msg, winner_idx)
        }
    }

    /// 產生 GAME_OVER 訊息
    pub fn game_over_message(&self) -> ServerMessage {
        let winner = if self.score.human > self.score.ai {
            Team::Human
        } else if self.score.ai > self.score.human {
            Team::Ai
        } else {
            // 平手時，預設 HUMAN 贏 (或可自訂規則)
            Team::Human
        };

        ServerMessage::GameOver {
            final_score: self.score.clone(),
            winner,
            history: self.history.clone(),
        }
    }

    /// 取得所有玩家的 conn_id
    pub fn all_conn_ids(&self) -> Vec<ConnectionId> {
        self.players.iter().map(|p| p.conn_id).collect()
    }

    /// 透過 conn_id 找玩家 index
    #[allow(dead_code)]
    pub fn find_player_idx(&self, conn_id: ConnectionId) -> Option<usize> {
        self.players.iter().position(|p| p.conn_id == conn_id)
    }
}

/// 出牌錯誤
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlayError {
    NotInGame,
    NotYourTurn,
    InvalidCard,
    NotInHand,
    NotLegal,
}

/// 出牌結果
pub enum PlayResult {
    /// 繼續遊戲，下一位玩家出牌
    Continue(ServerMessage, usize), // (broadcast_msg, next_player_idx)
    /// Trick 完成
    TrickComplete(ServerMessage),
}

/// Trick 結算結果
pub enum TrickResolution {
    /// 下一 trick
    NextTrick(ServerMessage, usize), // (trick_result_msg, next_player_idx)
    /// 遊戲結束
    GameOver(ServerMessage),
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::deck::{Rank, Suit};

    fn create_test_engine() -> GameEngine {
        let players = vec![
            (1, "P1".to_string(), Team::Human),
            (2, "P2".to_string(), Team::Human),
            (3, "P3".to_string(), Team::Ai),
            (4, "P4".to_string(), Team::Ai),
        ];
        GameEngine::new(12345, players)
    }

    #[test]
    fn test_deal_cards() {
        let mut engine = create_test_engine();
        let messages = engine.deal();

        assert_eq!(messages.len(), 4);
        for player in &engine.players {
            assert_eq!(player.hand.len(), 13);
        }
    }

    #[test]
    fn test_legal_moves_first_play() {
        let mut engine = create_test_engine();
        engine.deal();

        // 第一個出牌的人可以出任何牌
        let legal = engine.get_legal_moves(0);
        assert_eq!(legal.len(), 13);
    }

    #[test]
    fn test_follow_suit_rule() {
        let mut engine = create_test_engine();
        engine.deal();

        // 模擬 P1 出了一張黑桃
        let p1_hand = &engine.players[0].hand;
        let spade_card = p1_hand.iter().find(|c| c.suit == Suit::Spades).copied();

        if let Some(card) = spade_card {
            engine.table.push((0, card));
            engine.phase = GamePhase::WaitingForPlay { current_player_idx: 1 };

            // P2 的合法出牌
            let legal = engine.get_legal_moves(1);

            // 如果 P2 有黑桃，只能出黑桃
            let p2_spades: Vec<_> = engine.players[1].hand.iter().filter(|c| c.suit == Suit::Spades).collect();
            if !p2_spades.is_empty() {
                assert!(legal.iter().all(|c| c.suit == Suit::Spades));
            }
        }
    }

    #[test]
    fn test_trick_winner() {
        let mut engine = create_test_engine();
        engine.deal();
        engine.current_trick = 1;

        // 模擬出牌 (都出黑桃)
        engine.table = vec![
            (0, CardData::new(Suit::Spades, Rank::FIVE)),
            (1, CardData::new(Suit::Spades, Rank::KING)),
            (2, CardData::new(Suit::Spades, Rank::TWO)),
            (3, CardData::new(Suit::Spades, Rank::TEN)),
        ];
        engine.phase = GamePhase::TrickComplete;

        let resolution = engine.resolve_trick();

        // P2 (黑桃 K) 應該是贏家
        match resolution {
            TrickResolution::NextTrick(_, winner_idx) => {
                assert_eq!(winner_idx, 1);
            }
            _ => panic!("Expected NextTrick"),
        }
    }
}
