#!/usr/bin/env python3
"""
Integration Test: 1 Human + 3 AI clients
自動化測試 Server 與 Client 整合
"""

import sys
import os
import time
import threading
import json

# Add parent directory to path
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), '..')))

from clients.common.connection import NetworkClient

class AutoHumanClient:
    """自動化的 Human Client，自動選擇第一個合法牌"""

    def __init__(self, host: str, port: int, nickname: str):
        self.host = host
        self.port = port
        self.nickname = nickname
        self.client = NetworkClient()
        self.running = True
        self.game_over = False
        self.tricks_played = 0

    def run(self):
        print(f"[HUMAN] Connecting to {self.host}:{self.port}...")
        if not self.client.connect(self.host, self.port):
            print("[HUMAN] Connection failed.")
            return False

        # Handshake
        self.client.send({
            "type": "HELLO",
            "role": "HUMAN",
            "nickname": self.nickname,
            "proto": 1
        })

        print("[HUMAN] Connected. Waiting for game events...")

        try:
            while self.running and not self.game_over:
                msg = self.client.get_message()
                if msg:
                    self.handle_message(msg)
                else:
                    time.sleep(0.05)
        except KeyboardInterrupt:
            print("[HUMAN] Interrupted.")
        finally:
            self.client.close()

        return self.game_over

    def handle_message(self, msg):
        m_type = msg.get("type")

        if m_type == "WELCOME":
            print(f"[HUMAN] Welcome! ID: {msg.get('player_id')} Room: {msg.get('room')}")

        elif m_type == "ROOM_WAIT":
            players = len(msg.get('players', []))
            need = msg.get('need', 0)
            print(f"[HUMAN] Room: {players}/4 players, need {need} more")

        elif m_type == "ROOM_START":
            print("[HUMAN] Game starting!")

        elif m_type == "DEAL":
            hand = msg.get("hand", [])
            print(f"[HUMAN] Dealt {len(hand)} cards: {hand[:5]}...")

        elif m_type == "YOUR_TURN":
            trick = msg.get("trick", 0)
            legal = msg.get("legal", [])
            print(f"[HUMAN] Trick #{trick} - My turn. Legal: {legal}")

            # 自動選擇第一個合法牌
            if legal:
                card = legal[0]
                print(f"[HUMAN] Auto-playing: {card}")
                self.client.send({
                    "type": "PLAY",
                    "card": card
                })

        elif m_type == "PLAY_BROADCAST":
            print(f"[HUMAN] {msg.get('player_id')} played {msg.get('card')}")

        elif m_type == "PLAY_REJECT":
            print(f"[HUMAN] Play rejected: {msg.get('reason')}")

        elif m_type == "TRICK_RESULT":
            self.tricks_played += 1
            winner = msg.get("winner")
            score = msg.get("score", {})
            print(f"[HUMAN] Trick {self.tricks_played}/13 - Winner: {winner} | Score: HUMAN={score.get('HUMAN', 0)} AI={score.get('AI', 0)}")

        elif m_type == "GAME_OVER":
            winner = msg.get("winner")
            score = msg.get("final_score", {})
            print(f"[HUMAN] GAME OVER! Winner: {winner} | Final: HUMAN={score.get('HUMAN', 0)} AI={score.get('AI', 0)}")
            self.game_over = True

        elif m_type == "ERROR":
            print(f"[HUMAN] Error: {msg.get('message')}")


class AutoAIClient:
    """自動化的 AI Client，使用 Fallback 策略"""

    def __init__(self, host: str, port: int, nickname: str, auth_token: str):
        self.host = host
        self.port = port
        self.nickname = nickname
        self.auth_token = auth_token
        self.client = NetworkClient()
        self.running = True
        self.game_over = False
        self.hand = []

    def run(self):
        print(f"[{self.nickname}] Connecting...")
        if not self.client.connect(self.host, self.port):
            print(f"[{self.nickname}] Connection failed.")
            return False

        # Handshake
        self.client.send({
            "type": "HELLO",
            "role": "AI",
            "nickname": self.nickname,
            "auth": self.auth_token,
            "proto": 1
        })

        try:
            while self.running and not self.game_over:
                msg = self.client.get_message()
                if msg:
                    self.handle_message(msg)
                else:
                    time.sleep(0.05)
        except KeyboardInterrupt:
            pass
        finally:
            self.client.close()

        return self.game_over

    def handle_message(self, msg):
        m_type = msg.get("type")

        if m_type == "WELCOME":
            print(f"[{self.nickname}] Welcome! ID: {msg.get('player_id')}")

        elif m_type == "DEAL":
            self.hand = msg.get("hand", [])
            print(f"[{self.nickname}] Got {len(self.hand)} cards")

        elif m_type == "YOUR_TURN":
            legal = msg.get("legal", [])
            if legal:
                # Fallback: 出最小的牌
                card = self.choose_smallest(legal)
                print(f"[{self.nickname}] Playing: {card}")
                self.client.send({
                    "type": "PLAY",
                    "card": card
                })
                if card in self.hand:
                    self.hand.remove(card)

        elif m_type == "GAME_OVER":
            self.game_over = True

    def choose_smallest(self, legal):
        """選擇最小的牌"""
        rank_order = {
            "2": 2, "3": 3, "4": 4, "5": 5, "6": 6, "7": 7, "8": 8,
            "9": 9, "10": 10, "J": 11, "Q": 12, "K": 13, "A": 14
        }

        def card_value(card):
            rank = card[:-1]  # Remove suit
            return rank_order.get(rank, 0)

        return min(legal, key=card_value)


def run_test():
    """執行整合測試"""
    host = "127.0.0.1"
    port = 8888
    auth_token = "secret"

    print("=" * 50)
    print("CardArena Integration Test")
    print("1 Human + 3 AI Clients")
    print("=" * 50)

    # 建立 clients
    human = AutoHumanClient(host, port, "TestHuman")
    bots = [
        AutoAIClient(host, port, f"Bot{i}", auth_token)
        for i in range(1, 4)
    ]

    # 在背景啟動 AI clients
    threads = []
    for bot in bots:
        t = threading.Thread(target=bot.run, daemon=True)
        t.start()
        threads.append(t)
        time.sleep(0.1)  # 錯開連線時間

    # 在主線程執行 Human client
    time.sleep(0.5)  # 等待 AI clients 連線
    success = human.run()

    # 等待結束
    time.sleep(1)

    print("=" * 50)
    if success:
        print("TEST PASSED: Game completed successfully!")
    else:
        print("TEST FAILED: Game did not complete.")
    print("=" * 50)

    return success


if __name__ == "__main__":
    success = run_test()
    sys.exit(0 if success else 1)
