import sys
import os
import time
import argparse
import threading

# Ensure we can import common modules
sys.path.append(os.path.abspath(os.path.join(os.path.dirname(__file__), '../..')))

from clients.common.connection import NetworkClient
from clients.common.heartbeat import HeartbeatClient

class HumanCLI:
    def __init__(self, host: str, port: int, nickname: str):
        self.host = host
        self.port = port
        self.nickname = nickname
        self.client = NetworkClient()
        self.hb_client = None
        self.input_event = threading.Event()
        self.my_turn_data = None
        self.running = True

    def run(self):
        print(f"Connecting to {self.host}:{self.port}...")
        if not self.client.connect(self.host, self.port):
            print("Connection failed.")
            return

        # Start Heartbeat (UDP port = TCP port + 1)
        self.hb_client = HeartbeatClient(self.host, self.port + 1)
        self.hb_client.start()

        # Handshake
        self.client.send({
            "type": "HELLO",
            "role": "HUMAN",
            "nickname": self.nickname,
            "proto": 1
        })

        # Start input thread
        input_thread = threading.Thread(target=self.input_loop, daemon=True)
        input_thread.start()

        try:
            while self.running:
                msg = self.client.get_message()
                if msg:
                    self.handle_message(msg)
                else:
                    time.sleep(0.05)
        except KeyboardInterrupt:
            print("\nExiting...")
        finally:
            if self.hb_client:
                self.hb_client.stop()
            self.client.close()
            self.running = False

    def handle_message(self, msg):
        m_type = msg.get("type")

        if m_type == "WELCOME":
            print(f"✅ Connected! ID: {msg.get('player_id')} | Room: {msg.get('room')}")
            if self.hb_client:
                print(f"❤️  Heartbeat Active. Metrics: {self.hb_client.get_metrics()}")
            print("Waiting for other players...")

        elif m_type == "ROOM_WAIT":
             print(f"⏳ Room Status: {len(msg.get('players', []))}/{msg.get('need')} players.")

        elif m_type == "ROOM_START":
            print("🚀 Game Started!")

        elif m_type == "DEAL":
            hand = msg.get("hand", [])
            print(f"\n🎴 Hand Dealt: {', '.join(hand)}")

        elif m_type == "YOUR_TURN":
            self.my_turn_data = msg
            print(f"\n*** YOUR TURN! *** (Trick #{msg.get('trick')})")
            print(f"   Table: {self._fmt_table(msg.get('table', []))}")
            print(f"   Legal Moves: {msg.get('legal')}")
            # Signal input thread to wake up
            self.input_event.set()

        elif m_type == "PLAY_BROADCAST":
            print(f"📢 {msg.get('player_id')} played {msg.get('card')}")

        elif m_type == "PLAY_REJECT":
            print(f"❌ Invalid Move: {msg.get('reason')}")
            # Re-trigger input if it was us (usually server resends YOUR_TURN, but just in case)
        
        elif m_type == "TRICK_RESULT":
            print(f"🏆 Trick Winner: {msg.get('winner')}")
            print(f"   Score - HUMAN: {msg.get('score', {}).get('HUMAN', 0)} | AI: {msg.get('score', {}).get('AI', 0)}")
            if self.hb_client:
                m = self.hb_client.get_metrics()
                print(f"   📊 Ping: {m['rtt_ms']}ms | Loss: {m['loss_rate']}%")
            print("-" * 40)

        elif m_type == "GAME_OVER":
            print(f"\n🏁 GAME OVER! Winner: {msg.get('winner')}")
            self.running = False
            sys.exit(0)

        elif m_type == "ERROR":
            print(f"⛔ Error: {msg.get('message')}")

    def input_loop(self):
        """Thread that handles user input only when it's their turn."""
        while self.running:
            self.input_event.wait() # Block until it's my turn
            if not self.running: break
            
            data = self.my_turn_data
            if not data: 
                self.input_event.clear()
                continue

            valid_input = False
            while not valid_input and self.running:
                try:
                    card = input("Choose card > ").strip().upper()
                    if not card: continue
                    
                    # Basic client-side validation (UX only)
                    legal = data.get('legal', [])
                    if card not in legal:
                        print(f"⚠️  Illegal move. Choose from: {legal}")
                        continue
                    
                    self.client.send({
                        "type": "PLAY",
                        "card": card
                    })
                    valid_input = True
                except EOFError:
                    self.running = False
                    break
            
            self.my_turn_data = None
            self.input_event.clear()

    def _fmt_table(self, table):
        if not table: return "Empty"
        return ", ".join([f"{c['player_id']}:{c['card']}" for c in table])

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="CardArena Human CLI Client")
    parser.add_argument("--host", default="127.0.0.1", help="Server IP")
    parser.add_argument("--port", type=int, default=8888, help="Server Port")
    parser.add_argument("--name", default="Player_CLI", help="Nickname")
    
    args = parser.parse_args()
    
    cli = HumanCLI(args.host, args.port, args.name)
    cli.run()
