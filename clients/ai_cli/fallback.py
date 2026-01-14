from typing import List, Dict, Any

class FallbackStrategy:
    """Rule-based strategy for choosing a card when AI fails."""

    def choose(self, legal_moves: List[str], hand: List[str], table_cards: List[Dict], trick_num: int) -> str:
        """
        Choose a card from legal_moves.
        Strategy: Play the smallest legal card to conserve power, 
        unless we can win the trick cheaply (simple heuristic).
        For MVP, we just play the smallest rank.
        """
        if not legal_moves:
            # Should not happen if server logic is correct
            return hand[0] if hand else ""

        # Sort moves by value
        sorted_moves = sorted(legal_moves, key=self._card_value)
        
        # Simple Logic: play the smallest card
        return sorted_moves[0]

    def _card_value(self, card: str) -> int:
        """Get numeric value of card for comparison (2=2, ..., A=14)."""
        if not card: return 0
        rank = card[:-1]  # Remove suit (last char)
        
        rank_order = {
            "A": 14, "K": 13, "Q": 12, "J": 11, "10": 10,
            "9": 9, "8": 8, "7": 7, "6": 6, "5": 5,
            "4": 4, "3": 3, "2": 2
        }
        return rank_order.get(rank, 0)
