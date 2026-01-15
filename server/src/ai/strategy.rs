//! AI 出牌策略
//!
//! 提供可插拔的出牌策略系統

use crate::game::deck::{CardData, Rank, Suit};
use std::collections::HashMap;

/// AI 策略 trait
pub trait AiStrategy: Send + Sync {
    /// 選擇要出的牌
    ///
    /// # Arguments
    /// * `hand` - AI 的手牌
    /// * `legal_moves` - 合法的出牌選項
    /// * `table` - 當前桌面上的牌 (player_idx, card)
    /// * `is_leader` - 是否為首家 (第一個出牌)
    ///
    /// # Returns
    /// 選擇要出的牌
    fn choose_card(
        &self,
        hand: &[CardData],
        legal_moves: &[CardData],
        table: &[(usize, CardData)],
        is_leader: bool,
    ) -> CardData;
}

/// 智慧策略 (SmartStrategy)
///
/// ## 策略規則
///
/// ### 首家 (領牌)
/// 出「最長花色的最小牌」(試探策略)
/// - 統計手牌中各花色數量
/// - 選擇數量最多的花色 (平手時: S > H > D > C)
/// - 出該花色中點數最小的牌
///
/// ### 非首家 (跟牌)
/// - 有同花色: 找「大於桌面最大牌至少 3 點」的最小牌，若無則出最小牌
/// - 無同花色: 出任意花色最小牌 (墊牌)
#[derive(Debug, Clone, Default)]
pub struct SmartStrategy;

impl SmartStrategy {
    /// 建立新的 SmartStrategy 實例 - 預留供未來擴充
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self
    }

    /// 找出最長花色
    fn find_longest_suit(hand: &[CardData]) -> Suit {
        let mut counts: HashMap<Suit, usize> = HashMap::new();
        for card in hand {
            *counts.entry(card.suit).or_insert(0) += 1;
        }

        // 找出最大數量
        let max_count = counts.values().max().copied().unwrap_or(0);

        // 平手時依優先順序: S > H > D > C
        let priority = [Suit::Spades, Suit::Hearts, Suit::Diamonds, Suit::Clubs];
        for suit in priority {
            if counts.get(&suit).copied().unwrap_or(0) == max_count {
                return suit;
            }
        }

        Suit::Spades // fallback
    }

    /// 找出某花色中點數最小的牌
    fn find_smallest_of_suit(cards: &[CardData], suit: Suit) -> Option<CardData> {
        cards
            .iter()
            .filter(|c| c.suit == suit)
            .min_by_key(|c| c.rank.0)
            .copied()
    }

    /// 找出點數最小的牌
    fn find_smallest(cards: &[CardData]) -> Option<CardData> {
        cards.iter().min_by_key(|c| c.rank.0).copied()
    }

    /// 找出大於 threshold 至少 min_diff 點的最小牌
    fn find_smallest_above_threshold(
        cards: &[CardData],
        threshold: Rank,
        min_diff: u8,
    ) -> Option<CardData> {
        let target_min = threshold.0.saturating_add(min_diff);

        cards
            .iter()
            .filter(|c| c.rank.0 >= target_min)
            .min_by_key(|c| c.rank.0)
            .copied()
    }
}

impl AiStrategy for SmartStrategy {
    fn choose_card(
        &self,
        hand: &[CardData],
        legal_moves: &[CardData],
        table: &[(usize, CardData)],
        is_leader: bool,
    ) -> CardData {
        // 安全檢查
        if legal_moves.is_empty() {
            // 不應該發生，但作為 fallback
            return hand.first().copied().unwrap_or_else(|| CardData {
                suit: Suit::Clubs,
                rank: Rank::TWO,
            });
        }

        if is_leader {
            // === 首家策略 ===
            // 出最長花色的最小牌
            let longest_suit = Self::find_longest_suit(hand);

            // 從 legal_moves 中找該花色最小牌
            if let Some(card) = Self::find_smallest_of_suit(legal_moves, longest_suit) {
                return card;
            }
            // Fallback: 出任意最小牌
            Self::find_smallest(legal_moves).unwrap_or(legal_moves[0])
        } else {
            // === 非首家策略 ===
            let lead_suit = table[0].1.suit;

            // 檢查 legal_moves 是否有同花色 (必須跟牌的情況)
            let same_suit_moves: Vec<CardData> =
                legal_moves.iter().filter(|c| c.suit == lead_suit).copied().collect();

            if !same_suit_moves.is_empty() {
                // 有同花色，找桌面同花色最大牌
                let highest_on_table = table
                    .iter()
                    .filter(|(_, c)| c.suit == lead_suit)
                    .map(|(_, c)| c.rank)
                    .max()
                    .unwrap_or(Rank::TWO);

                // 找「大於 highest 至少 3 點」的最小牌
                if let Some(winning_card) =
                    Self::find_smallest_above_threshold(&same_suit_moves, highest_on_table, 3)
                {
                    return winning_card;
                }

                // 無法贏取，出同花色最小牌
                Self::find_smallest(&same_suit_moves).unwrap_or(legal_moves[0])
            } else {
                // 無同花色，出任意最小牌 (墊牌)
                Self::find_smallest(legal_moves).unwrap_or(legal_moves[0])
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_card(suit: Suit, rank: u8) -> CardData {
        CardData { suit, rank: Rank(rank) }
    }

    #[test]
    fn test_leader_longest_suit() {
        let strategy = SmartStrategy::new();

        // 手牌: 3H 5H 9H 2D 7D (H 最長=3張)
        let hand = vec![
            make_card(Suit::Hearts, 3),
            make_card(Suit::Hearts, 5),
            make_card(Suit::Hearts, 9),
            make_card(Suit::Diamonds, 2),
            make_card(Suit::Diamonds, 7),
        ];

        let result = strategy.choose_card(&hand, &hand, &[], true);

        // 應該出 3H (Hearts 最長，最小牌)
        assert_eq!(result.suit, Suit::Hearts);
        assert_eq!(result.rank.0, 3);
    }

    #[test]
    fn test_follower_win_attempt() {
        let strategy = SmartStrategy::new();

        // 桌面: 7H
        let table = vec![(0, make_card(Suit::Hearts, 7))];

        // 手牌: 3H 9H QH 5D
        let hand = vec![
            make_card(Suit::Hearts, 3),
            make_card(Suit::Hearts, 9),
            make_card(Suit::Hearts, 12), // Q
            make_card(Suit::Diamonds, 5),
        ];

        // 合法牌 (跟牌必須出 Hearts)
        let legal = vec![
            make_card(Suit::Hearts, 3),
            make_card(Suit::Hearts, 9),
            make_card(Suit::Hearts, 12),
        ];

        let result = strategy.choose_card(&hand, &legal, &table, false);

        // 需要 >= 7+3=10，9H 只有 9 點不夠，QH=12 符合
        assert_eq!(result.suit, Suit::Hearts);
        assert_eq!(result.rank.0, 12); // Q
    }

    #[test]
    fn test_follower_give_up() {
        let strategy = SmartStrategy::new();

        // 桌面: 7H
        let table = vec![(0, make_card(Suit::Hearts, 7))];

        // 手牌: 3H 8H 5D (無法贏)
        let hand = vec![
            make_card(Suit::Hearts, 3),
            make_card(Suit::Hearts, 8),
            make_card(Suit::Diamonds, 5),
        ];

        let legal = vec![
            make_card(Suit::Hearts, 3),
            make_card(Suit::Hearts, 8),
        ];

        let result = strategy.choose_card(&hand, &legal, &table, false);

        // 8H 只比 7H 大 1 點，不夠 +3，應該放棄出最小牌 3H
        assert_eq!(result.suit, Suit::Hearts);
        assert_eq!(result.rank.0, 3);
    }

    #[test]
    fn test_follower_discard() {
        let strategy = SmartStrategy::new();

        // 桌面: 7H
        let table = vec![(0, make_card(Suit::Hearts, 7))];

        // 手牌: 2D 5S KC (無 Hearts)
        let hand = vec![
            make_card(Suit::Diamonds, 2),
            make_card(Suit::Spades, 5),
            make_card(Suit::Clubs, 13),
        ];

        let result = strategy.choose_card(&hand, &hand, &table, false);

        // 無 Hearts，出最小牌 2D
        assert_eq!(result.suit, Suit::Diamonds);
        assert_eq!(result.rank.0, 2);
    }

    #[test]
    fn test_find_longest_suit_priority() {
        // 平手時 S > H > D > C
        let hand = vec![
            make_card(Suit::Hearts, 3),
            make_card(Suit::Spades, 5),
        ];

        let result = SmartStrategy::find_longest_suit(&hand);
        assert_eq!(result, Suit::Spades); // S 優先
    }
}
