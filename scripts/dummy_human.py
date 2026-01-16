import sys
import json
import socket
import time
import threading

def run_dummy(host, port, name):
    s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    try:
        s.connect((host, port))
    except Exception as e:
        print(f"Connection failed: {e}")
        return

    # Helper to send
    def send_json(msg):
        s.sendall(json.dumps(msg).encode() + b'\n')

    # Helper to recv line
    buffer = b""
    def recv_json():
        nonlocal buffer
        while b'\n' not in buffer:
            chunk = s.recv(4096)
            if not chunk: return None
            buffer += chunk
        line, buffer = buffer.split(b'\n', 1)
        return json.loads(line.decode())

    # Handshake
    send_json({
        "type": "HELLO",
        "role": "HUMAN",
        "nickname": name,
        "proto": 1
    })

    print(f"[{name}] Connected and sent HELLO")

    while True:
        try:
            msg = recv_json()
            if not msg: break
            
            m_type = msg.get("type")
            if m_type == "YOUR_TURN":
                legal = msg.get("legal", [])
                if legal:
                    card = legal[0]
                    print(f"[{name}] Playing {card}")
                    send_json({"type": "PLAY", "card": card})
            elif m_type == "GAME_OVER":
                print(f"[{name}] Game Over")
                break
        except Exception as e:
            print(f"[{name}] Error: {e}")
            break
    
    s.close()

if __name__ == "__main__":
    if len(sys.argv) < 3:
        print("Usage: dummy_human.py <port> <name>")
        sys.exit(1)
    run_dummy("127.0.0.1", int(sys.argv[1]), sys.argv[2])
