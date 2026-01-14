use crate::protocol::Card;

/// 花色
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Suit {
    Spades,   // ♠
    Hearts,   // ♥
    Diamonds, // ♦
    Clubs,    // ♣
}

impl Suit {
    pub fn all() -> [Suit; 4] {
        [Suit::Spades, Suit::Hearts, Suit::Diamonds, Suit::Clubs]
    }

    pub fn symbol(&self) -> char {
        match self {
            Suit::Spades => 'S',
            Suit::Hearts => 'H',
            Suit::Diamonds => 'D',
            Suit::Clubs => 'C',
        }
    }

    pub fn from_char(c: char) -> Option<Suit> {
        match c.to_ascii_uppercase() {
            'S' => Some(Suit::Spades),
            'H' => Some(Suit::Hearts),
            'D' => Some(Suit::Diamonds),
            'C' => Some(Suit::Clubs),
            _ => None,
        }
    }
}

/// 點數 (2-14, 其中 11=J, 12=Q, 13=K, 14=A)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Rank(pub u8);

impl Rank {
    pub const TWO: Rank = Rank(2);
    pub const THREE: Rank = Rank(3);
    pub const FOUR: Rank = Rank(4);
    pub const FIVE: Rank = Rank(5);
    pub const SIX: Rank = Rank(6);
    pub const SEVEN: Rank = Rank(7);
    pub const EIGHT: Rank = Rank(8);
    pub const NINE: Rank = Rank(9);
    pub const TEN: Rank = Rank(10);
    pub const JACK: Rank = Rank(11);
    pub const QUEEN: Rank = Rank(12);
    pub const KING: Rank = Rank(13);
    pub const ACE: Rank = Rank(14);

    pub fn all() -> [Rank; 13] {
        [
            Rank::TWO,
            Rank::THREE,
            Rank::FOUR,
            Rank::FIVE,
            Rank::SIX,
            Rank::SEVEN,
            Rank::EIGHT,
            Rank::NINE,
            Rank::TEN,
            Rank::JACK,
            Rank::QUEEN,
            Rank::KING,
            Rank::ACE,
        ]
    }

    pub fn symbol(&self) -> String {
        match self.0 {
            2..=10 => self.0.to_string(),
            11 => "J".to_string(),
            12 => "Q".to_string(),
            13 => "K".to_string(),
            14 => "A".to_string(),
            _ => "?".to_string(),
        }
    }

    pub fn from_str(s: &str) -> Option<Rank> {
        match s.to_uppercase().as_str() {
            "2" => Some(Rank::TWO),
            "3" => Some(Rank::THREE),
            "4" => Some(Rank::FOUR),
            "5" => Some(Rank::FIVE),
            "6" => Some(Rank::SIX),
            "7" => Some(Rank::SEVEN),
            "8" => Some(Rank::EIGHT),
            "9" => Some(Rank::NINE),
            "10" => Some(Rank::TEN),
            "J" => Some(Rank::JACK),
            "Q" => Some(Rank::QUEEN),
            "K" => Some(Rank::KING),
            "A" => Some(Rank::ACE),
            _ => None,
        }
    }
}

/// 撲克牌
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CardData {
    pub suit: Suit,
    pub rank: Rank,
}

impl CardData {
    pub fn new(suit: Suit, rank: Rank) -> Self {
        Self { suit, rank }
    }

    /// 轉換為協議字串格式 (e.g., "AS", "10H", "KC")
    pub fn to_protocol_string(&self) -> Card {
        format!("{}{}", self.rank.symbol(), self.suit.symbol())
    }

    /// 從協議字串解析 (e.g., "AS", "10H", "KC")
    pub fn from_protocol_string(s: &str) -> Option<CardData> {
        if s.len() < 2 {
            return None;
        }

        let suit_char = s.chars().last()?;
        let rank_str = &s[..s.len() - 1];

        let suit = Suit::from_char(suit_char)?;
        let rank = Rank::from_str(rank_str)?;

        Some(CardData::new(suit, rank))
    }
}

/// 牌組
pub struct Deck {
    cards: Vec<CardData>,
}

impl Deck {
    /// 建立完整的 52 張牌
    pub fn new() -> Self {
        let mut cards = Vec::with_capacity(52);
        for suit in Suit::all() {
            for rank in Rank::all() {
                cards.push(CardData::new(suit, rank));
            }
        }
        Self { cards }
    }

    /// 使用 seed 洗牌 (Fisher-Yates shuffle with LCG PRNG)
    pub fn shuffle(&mut self, seed: u64) {
        let mut rng = SimpleLcg::new(seed);
        let n = self.cards.len();

        for i in (1..n).rev() {
            let j = (rng.next() as usize) % (i + 1);
            self.cards.swap(i, j);
        }
    }

    /// 發牌給 n 位玩家，每人 cards_per_player 張
    pub fn deal(&self, num_players: usize, cards_per_player: usize) -> Vec<Vec<CardData>> {
        let mut hands: Vec<Vec<CardData>> = vec![Vec::with_capacity(cards_per_player); num_players];

        for (i, card) in self.cards.iter().take(num_players * cards_per_player).enumerate() {
            let player_idx = i % num_players;
            hands[player_idx].push(*card);
        }

        hands
    }

    /// 取得所有牌
    #[allow(dead_code)]
    pub fn cards(&self) -> &[CardData] {
        &self.cards
    }
}

impl Default for Deck {
    fn default() -> Self {
        Self::new()
    }
}

/// 簡易 LCG 隨機數產生器 (Linear Congruential Generator)
/// 用於確定性洗牌
struct SimpleLcg {
    state: u64,
}

impl SimpleLcg {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next(&mut self) -> u64 {
        // LCG parameters (same as glibc)
        const A: u64 = 1103515245;
        const C: u64 = 12345;
        const M: u64 = 1 << 31;

        self.state = (A.wrapping_mul(self.state).wrapping_add(C)) % M;
        self.state
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deck_has_52_cards() {
        let deck = Deck::new();
        assert_eq!(deck.cards().len(), 52);
    }

    #[test]
    fn test_deck_unique_cards() {
        let deck = Deck::new();
        let mut seen = std::collections::HashSet::new();
        for card in deck.cards() {
            assert!(seen.insert((card.suit, card.rank)));
        }
    }

    #[test]
    fn test_deterministic_shuffle() {
        let seed = 12345u64;

        let mut deck1 = Deck::new();
        deck1.shuffle(seed);

        let mut deck2 = Deck::new();
        deck2.shuffle(seed);

        // 相同 seed 應該產生相同順序
        assert_eq!(deck1.cards(), deck2.cards());
    }

    #[test]
    fn test_different_seeds_different_order() {
        let mut deck1 = Deck::new();
        deck1.shuffle(12345);

        let mut deck2 = Deck::new();
        deck2.shuffle(67890);

        // 不同 seed 應該產生不同順序
        assert_ne!(deck1.cards(), deck2.cards());
    }

    #[test]
    fn test_deal_4_players_13_cards() {
        let mut deck = Deck::new();
        deck.shuffle(42);

        let hands = deck.deal(4, 13);

        assert_eq!(hands.len(), 4);
        for hand in &hands {
            assert_eq!(hand.len(), 13);
        }

        // 確認沒有重複的牌
        let mut all_cards: Vec<_> = hands.iter().flatten().collect();
        all_cards.sort_by_key(|c| (c.suit as u8, c.rank.0));
        for i in 1..all_cards.len() {
            assert_ne!(all_cards[i - 1], all_cards[i]);
        }
    }

    #[test]
    fn test_card_protocol_string() {
        let card = CardData::new(Suit::Spades, Rank::ACE);
        assert_eq!(card.to_protocol_string(), "AS");

        let card = CardData::new(Suit::Hearts, Rank::TEN);
        assert_eq!(card.to_protocol_string(), "10H");

        let card = CardData::new(Suit::Clubs, Rank::KING);
        assert_eq!(card.to_protocol_string(), "KC");
    }

    #[test]
    fn test_card_from_protocol_string() {
        let card = CardData::from_protocol_string("AS").unwrap();
        assert_eq!(card.suit, Suit::Spades);
        assert_eq!(card.rank, Rank::ACE);

        let card = CardData::from_protocol_string("10H").unwrap();
        assert_eq!(card.suit, Suit::Hearts);
        assert_eq!(card.rank, Rank::TEN);

        let card = CardData::from_protocol_string("KC").unwrap();
        assert_eq!(card.suit, Suit::Clubs);
        assert_eq!(card.rank, Rank::KING);
    }
}
