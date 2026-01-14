import socket
import json
import time
import threading

HOST = '127.0.0.1'
PORT = 8888

def handle_client(conn, addr):
    print(f"[MockServer] Connection from {addr}")
    buffer = b""
    try:
        # 1. Receive HELLO
        while b'\n' not in buffer:
            data = conn.recv(1024)
            if not data: break
            buffer += data
        
        line, buffer = buffer.split(b'\n', 1)
        msg = json.loads(line.decode())
        print(f"[MockServer] Received: {msg}")

        # 2. Send WELCOME
        response = {
            "type": "WELCOME",
            "player_id": "P1",
            "room": "TEST_ROOM"
        }
        conn.sendall(json.dumps(response).encode() + b'\n')
        time.sleep(1)

        # 3. Send DEAL
        hand = ["AS", "KH", "10D", "2C", "5S"]
        conn.sendall(json.dumps({
            "type": "DEAL",
            "hand": hand,
            "n": 5
        }).encode() + b'\n')
        time.sleep(1)

        # 4. Send YOUR_TURN
        conn.sendall(json.dumps({
            "type": "YOUR_TURN",
            "trick": 1,
            "table": [{"player": "P2", "card": "3H"}, {"player": "P3", "card": "9H"}],
            "legal": ["KH", "10D", "2C", "5S"], # Assume AS is illegal just for testing logic
            "timeout_ms": 30000
        }).encode() + b'\n')

        # 5. Wait for PLAY
        while b'\n' not in buffer:
            data = conn.recv(1024)
            if not data: break
            buffer += data
        line, buffer = buffer.split(b'\n', 1)
        play_msg = json.loads(line.decode())
        print(f"[MockServer] Client played: {play_msg}")

        # 6. Send TRICK_RESULT (End test)
        conn.sendall(json.dumps({
            "type": "TRICK_RESULT",
            "winner": "P1",
            "score": {"HUMAN": 1, "AI": 0}
        }).encode() + b'\n')
        
        time.sleep(1)
        print("[MockServer] Test session finished.")

    except Exception as e:
        print(f"[MockServer] Error: {e}")
    finally:
        conn.close()

def start_server():
    server = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    server.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    server.bind((HOST, PORT))
    server.listen(5)
    print(f"[MockServer] Listening on {HOST}:{PORT}...")

    while True:
        conn, addr = server.accept()
        t = threading.Thread(target=handle_client, args=(conn, addr))
        t.start()

if __name__ == "__main__":
    start_server()
