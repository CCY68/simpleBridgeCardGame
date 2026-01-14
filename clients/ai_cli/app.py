import sys
import os
import time
import argparse
import logging

# Ensure we can import common modules
sys.path.append(os.path.abspath(os.path.join(os.path.dirname(__file__), '../..')))

from clients.common.connection import NetworkClient
from clients.common.heartbeat import HeartbeatClient
from clients.ai_cli.fallback import FallbackStrategy
from clients.ai_cli.gemini_bridge import GeminiBridge

# Setup Logging
logging.basicConfig(level=logging.INFO, format='%(asctime)s [%(levelname)s] %(message)s')
logger = logging.getLogger("AI_BOT")

class AIClient:
    def __init__(self, host: str, port: int, nickname: str, auth_token: str, use_gemini: bool = True):
        self.host = host
        self.port = port
        self.nickname = nickname
        self.auth_token = auth_token
        
        self.client = NetworkClient()
        self.hb_client = None  # UDP Heartbeat
        self.fallback = FallbackStrategy()
        self.gemini = GeminiBridge() if use_gemini else None

        # Game State
        self.hand = []
        self.my_score = 0
        self.opp_score = 0
        self.last_played_card = None  # 追蹤最後出的牌，用於 PLAY_REJECT 恢復

    def run(self):
        """Main client loop."""
        logger.info(f"Connecting to {self.host}:{self.port}...")
        if not self.client.connect(self.host, self.port):
            logger.error("Connection failed.")
            return

        # Start Heartbeat (UDP port = TCP port + 1)
        self.hb_client = HeartbeatClient(self.host, self.port + 1)
        self.hb_client.start()
        logger.info(f"Heartbeat started on UDP port {self.port + 1}")

        # Handshake
        self.client.send({
            "type": "HELLO",
            "role": "AI",
            "nickname": self.nickname,
            "auth": self.auth_token,
            "proto": 1
        })

        logger.info("Connected. Waiting for game events...")

        try:
            while True:
                msg = self.client.get_message()
                if msg:
                    self.handle_message(msg)
                else:
                    time.sleep(0.1)
        except KeyboardInterrupt:
            logger.info("Stopping AI client.")
        finally:
            if self.hb_client:
                self.hb_client.stop()
            self.client.close()

    def handle_message(self, msg):
        """Dispatch based on message type."""
        m_type = msg.get("type")

        if m_type == "WELCOME":
            logger.info(f"Welcome! ID: {msg.get('player_id')} Room: {msg.get('room')}")
        
        elif m_type == "DEAL":
            self.hand = msg.get("hand", [])
            logger.info(f"Dealt hand: {self.hand}")

        elif m_type == "ROOM_WAIT":
            players = msg.get("players", [])
            need = msg.get("need", 0)
            logger.info(f"Room status: {len(players)}/4 players, need {need} more")

        elif m_type == "ROOM_START":
            logger.info("Game starting!")

        elif m_type == "YOUR_TURN":
            self.on_your_turn(msg)

        elif m_type == "PLAY_BROADCAST":
            player_id = msg.get("player_id")
            card = msg.get("card")
            logger.info(f"{player_id} played {card}")

        elif m_type == "PLAY_REJECT":
            self.on_play_reject(msg)

        elif m_type == "TRICK_RESULT":
            winner = msg.get("winner")
            score = msg.get("score", {})
            self.my_score = score.get("AI", 0)
            self.opp_score = score.get("HUMAN", 0)
            logger.info(f"Trick winner: {winner} | Score - HUMAN: {self.opp_score} AI: {self.my_score}")
            if self.hb_client:
                m = self.hb_client.get_metrics()
                logger.info(f"Ping: {m['rtt_ms']}ms | Loss: {m['loss_rate']}%")

        elif m_type == "GAME_OVER":
            logger.info("Game Over.")
            # Could exit or wait for new game
            
        elif m_type == "ERROR":
            logger.error(f"Server Error: {msg.get('message')}")

    def on_your_turn(self, msg):
        """Decide move and send PLAY."""
        legal = msg.get("legal", [])
        trick_num = msg.get("trick", 0)
        table = msg.get("table", [])

        logger.info(f"My Turn (Trick #{trick_num}). Legal: {legal}")

        # 1. Try Gemini
        chosen_card = None
        if self.gemini:
            chosen_card = self.gemini.decide_move(
                self.hand, legal, table, trick_num, self.my_score, self.opp_score
            )

        # 2. Fallback if Gemini failed or is disabled
        if not chosen_card:
            logger.info("Using Fallback Strategy.")
            chosen_card = self.fallback.choose(legal, self.hand, table, trick_num)

        # 3. Send Move
        logger.info(f"Playing: {chosen_card}")
        self.last_played_card = chosen_card  # 追蹤最後出的牌
        self.client.send({
            "type": "PLAY",
            "card": chosen_card
        })

        # Optimistically remove from hand (server will correct if rejected)
        if chosen_card in self.hand:
            self.hand.remove(chosen_card)

    def on_play_reject(self, msg):
        """Handle PLAY_REJECT - restore card to hand and wait for next YOUR_TURN."""
        card = msg.get("card")
        reason = msg.get("reason")
        logger.warning(f"Play rejected: {card} - Reason: {reason}")

        # 恢復手牌 (如果卡牌已被移除)
        if self.last_played_card and self.last_played_card not in self.hand:
            self.hand.append(self.last_played_card)
            logger.info(f"Restored {self.last_played_card} to hand. Current hand: {self.hand}")

        self.last_played_card = None
        # Server 會重新發送 YOUR_TURN，等待下一次決策

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="CardArena AI Client")
    parser.add_argument("--host", default="127.0.0.1", help="Server IP")
    parser.add_argument("--port", type=int, default=8888, help="Server Port")
    parser.add_argument("--name", default="Bot_1", help="AI Nickname")
    parser.add_argument("--token", default="secret", help="Auth Token")
    parser.add_argument("--no-llm", action="store_true", help="Disable Gemini LLM")
    
    args = parser.parse_args()
    
    bot = AIClient(args.host, args.port, args.name, args.token, use_gemini=not args.no_llm)
    bot.run()
