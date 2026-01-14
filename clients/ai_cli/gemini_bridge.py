import os
import json
import logging
from typing import List, Dict, Optional
import google.generativeai as genai
from google.api_core import exceptions

# Setup logging
logging.basicConfig(level=logging.INFO, format='[AI] %(message)s')
logger = logging.getLogger(__name__)

class GeminiBridge:
    """Bridge to Google Gemini Pro API for card game decision making."""

    def __init__(self, api_key: Optional[str] = None):
        self.api_key = api_key or os.getenv("GEMINI_API_KEY")
        if not self.api_key:
            logger.warning("No GEMINI_API_KEY provided. AI will run in Fallback-only mode.")
            self.model = None
        else:
            genai.configure(api_key=self.api_key)
            self.model = genai.GenerativeModel('gemini-1.5-pro')

    def decide_move(self, hand: List[str], legal_moves: List[str], 
                    table_cards: List[Dict], trick_num: int, 
                    my_score: int, opp_score: int) -> Optional[str]:
        """
        Ask Gemini to choose a card.
        Returns the chosen card string (e.g. "AS") or None if failed.
        """
        if not self.model:
            return None

        prompt = self._build_prompt(hand, legal_moves, table_cards, trick_num, my_score, opp_score)
        
        try:
            # Generate content
            response = self.model.generate_content(prompt)
            
            # Parse response
            chosen_card = self._parse_response(response.text)
            
            # Validation
            if chosen_card in legal_moves:
                logger.info(f"Gemini chose: {chosen_card}")
                return chosen_card
            else:
                logger.warning(f"Gemini chose illegal move: {chosen_card}. Legal: {legal_moves}")
                return None

        except Exception as e:
            logger.error(f"Gemini API Error: {e}")
            return None

    def _build_prompt(self, hand, legal, table, trick, score_us, score_them) -> str:
        """Constructs the prompt for the LLM."""
        table_desc = ", ".join([f"{c['player']}:{c['card']}" for c in table]) if table else "Empty"
        
        return f"""
You are an expert Bridge/Whist player. Play a card to win the game.
Current Trick: #{trick}
Your Hand: {hand}
Legal Moves: {legal}
Table: {table_desc}
Score - You: {score_us}, Opponent: {score_them}

Rules:
1. You must follow suit if possible (already filtered in Legal Moves).
2. High card wins the trick.
3. Win tricks to increase score.

Task: Choose the BEST card from Legal Moves.
Output JSON ONLY: {{"card": "YOUR_CHOICE", "reason": "brief explanation"}}
"""

    def _parse_response(self, text: str) -> Optional[str]:
        """Extracts JSON from the response text."""
        try:
            # Clean up potential markdown code blocks
            clean_text = text.strip()
            if clean_text.startswith("```json"):
                clean_text = clean_text[7:]
            if clean_text.endswith("```"):
                clean_text = clean_text[:-3]
            
            data = json.loads(clean_text)
            return data.get("card")
        except json.JSONDecodeError:
            logger.error(f"Failed to parse JSON response: {text}")
            return None
