#!/usr/bin/env python3
"""
CardArena Admin CLI Client

獨立的管理介面工具，用於連接 Server 的 Admin Port (8890)

Usage:
    python admin_client.py [--host HOST] [--port PORT] [--token TOKEN]

Examples:
    python admin_client.py
    python admin_client.py --host 192.168.1.100 --port 8890 --token my_secret
"""

import argparse
import socket
import sys
import threading
import os

DEFAULT_HOST = "127.0.0.1"
DEFAULT_PORT = 8890
DEFAULT_TOKEN = "admin"


class AdminClient:
    def __init__(self, host: str, port: int):
        self.host = host
        self.port = port
        self.sock = None
        self.running = False
        self.authenticated = False

    def connect(self) -> bool:
        """連接到 Admin Server"""
        try:
            self.sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
            self.sock.connect((self.host, self.port))
            self.running = True
            print(f"Connected to {self.host}:{self.port}")
            return True
        except Exception as e:
            print(f"Failed to connect: {e}")
            return False

    def disconnect(self):
        """斷開連線"""
        self.running = False
        if self.sock:
            try:
                self.sock.close()
            except:
                pass

    def send(self, message: str):
        """發送指令"""
        if self.sock:
            try:
                self.sock.sendall((message + "\n").encode())
            except Exception as e:
                print(f"Send error: {e}")
                self.running = False

    def receive_loop(self):
        """接收回應的執行緒"""
        buffer = ""
        while self.running:
            try:
                data = self.sock.recv(4096)
                if not data:
                    print("\nConnection closed by server")
                    self.running = False
                    break

                buffer += data.decode()
                while "\n" in buffer:
                    line, buffer = buffer.split("\n", 1)
                    if line.strip():
                        print(line)

                # 檢查是否有 prompt
                if buffer.endswith("> "):
                    print(buffer, end="", flush=True)
                    buffer = ""

            except socket.timeout:
                continue
            except Exception as e:
                if self.running:
                    print(f"\nReceive error: {e}")
                self.running = False
                break

    def run(self, auto_auth_token: str = None):
        """主迴圈"""
        if not self.connect():
            return

        # 設定 socket timeout
        self.sock.settimeout(0.5)

        # 啟動接收執行緒
        recv_thread = threading.Thread(target=self.receive_loop, daemon=True)
        recv_thread.start()

        # 等待一下讓歡迎訊息顯示
        import time
        time.sleep(0.3)

        # 自動認證
        if auto_auth_token:
            self.send(f"AUTH {auto_auth_token}")
            time.sleep(0.2)

        # 輸入迴圈
        try:
            while self.running:
                try:
                    line = input()
                    if not self.running:
                        break
                    self.send(line)

                    # 檢查是否退出
                    cmd = line.strip().upper()
                    if cmd in ("QUIT", "EXIT", "BYE"):
                        import time
                        time.sleep(0.2)
                        break

                except EOFError:
                    break
        except KeyboardInterrupt:
            print("\n\nInterrupted. Disconnecting...")

        self.disconnect()
        print("Disconnected.")


def print_help():
    """顯示使用說明"""
    print("""
╔═══════════════════════════════════════════════════════════════╗
║              CardArena Admin CLI Client                      ║
╠═══════════════════════════════════════════════════════════════╣
║  Commands (after authentication):                            ║
║    HELP             Show available commands                  ║
║    STATUS           Show server status                       ║
║    ROOMS            List all rooms                           ║
║    PLAYERS          List all players                         ║
║    LOGS [n] [type]  Show recent logs                         ║
║    KICK <player_id> Kick a player                            ║
║    RESET [room_id]  Reset a room                             ║
║    QUIT             Disconnect                               ║
╚═══════════════════════════════════════════════════════════════╝
""")


def main():
    parser = argparse.ArgumentParser(
        description="CardArena Admin CLI Client",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  python admin_client.py
  python admin_client.py --host 192.168.1.100
  python admin_client.py --port 8890 --token my_secret
        """,
    )
    parser.add_argument(
        "--host", "-H",
        default=os.environ.get("ADMIN_HOST", DEFAULT_HOST),
        help=f"Server host (default: {DEFAULT_HOST})",
    )
    parser.add_argument(
        "--port", "-p",
        type=int,
        default=int(os.environ.get("ADMIN_PORT", DEFAULT_PORT)),
        help=f"Admin port (default: {DEFAULT_PORT})",
    )
    parser.add_argument(
        "--token", "-t",
        default=os.environ.get("ADMIN_AUTH_TOKEN", DEFAULT_TOKEN),
        help=f"Auth token (default: {DEFAULT_TOKEN})",
    )
    parser.add_argument(
        "--no-auth",
        action="store_true",
        help="Don't auto-authenticate on connect",
    )

    args = parser.parse_args()

    print_help()

    client = AdminClient(args.host, args.port)
    token = None if args.no_auth else args.token
    client.run(auto_auth_token=token)


if __name__ == "__main__":
    main()
