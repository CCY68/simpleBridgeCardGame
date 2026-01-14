import tkinter as tk
from tkinter import messagebox, ttk
import sys
import os

# 確保可以 import clients.common
sys.path.append(os.path.abspath(os.path.join(os.path.dirname(__file__), '../..')))
from clients.common.connection import NetworkClient
from clients.human_gui.components.card import CardRenderer

class CardArenaApp:
    def __init__(self, root):
        self.root = root
        self.root.title("CardArena - LAN Card Game")
        self.root.geometry("1024x768")  # Resize for game board
        
        self.client = NetworkClient()
        self.player_id = None
        self.nickname = ""

        # 初始化介面
        self.setup_login_ui()
        
        # 啟動訊息檢查迴圈
        self.check_messages()

    def setup_login_ui(self):
        """建立登入畫面。"""
        self.login_frame = ttk.Frame(self.root, padding="20")
        self.login_frame.place(relx=0.5, rely=0.5, anchor="center")

        ttk.Label(self.login_frame, text="CardArena", font=("Arial", 24, "bold")).grid(row=0, column=0, columnspan=2, pady=20)
        
        ttk.Label(self.login_frame, text="Host IP:").grid(row=1, column=0, sticky="e", pady=5)
        self.entry_host = ttk.Entry(self.login_frame)
        self.entry_host.insert(0, "127.0.0.1")
        self.entry_host.grid(row=1, column=1, pady=5)

        ttk.Label(self.login_frame, text="Nickname:").grid(row=2, column=0, sticky="e", pady=5)
        self.entry_name = ttk.Entry(self.login_frame)
        self.entry_name.insert(0, "Player")
        self.entry_name.grid(row=2, column=1, pady=5)

        self.btn_connect = ttk.Button(self.login_frame, text="Connect", command=self.on_connect)
        self.btn_connect.grid(row=3, column=0, columnspan=2, pady=20)

        # Temporary Dev Button to skip login (for GUI testing)
        ttk.Button(self.login_frame, text="[Dev] Preview UI", command=self.setup_game_ui).grid(row=4, column=0, columnspan=2, pady=5)


    def on_connect(self):
        """處理連線按鈕點擊。"""
        host = self.entry_host.get()
        self.nickname = self.entry_name.get()
        
        if not self.nickname:
            messagebox.showwarning("Warning", "Please enter a nickname.")
            return

        if self.client.connect(host, 8888):
            # 送出 HELLO 封包
            self.client.send({
                "type": "HELLO",
                "role": "HUMAN",
                "nickname": self.nickname,
                "proto": 1
            })
            self.btn_connect.config(state="disabled")
        else:
            messagebox.showerror("Error", "Could not connect to server.")

    def setup_game_ui(self):
        """切換到遊戲大廳/桌面畫面。"""
        if self.login_frame:
            self.login_frame.destroy()
        
        self.main_frame = ttk.Frame(self.root)
        self.main_frame.pack(fill="both", expand=True)

        # Top Bar: Status
        self.top_bar = ttk.Frame(self.main_frame, padding="5")
        self.top_bar.pack(side="top", fill="x")
        self.lbl_status = ttk.Label(self.top_bar, text=f"Player: {self.nickname} | Room: Waiting...", font=("Arial", 12))
        self.lbl_status.pack(side="left")

        # Center: Game Table (Canvas)
        self.game_canvas = tk.Canvas(self.main_frame, bg="#35654d") # Felt Green color
        self.game_canvas.pack(side="left", fill="both", expand=True, padx=5, pady=5)

        # Right: Log / Chat
        self.right_panel = ttk.Frame(self.main_frame, padding="5", width=200)
        self.right_panel.pack(side="right", fill="y")
        
        ttk.Label(self.right_panel, text="Game Log").pack(anchor="w")
        self.log_text = tk.Text(self.right_panel, width=25, height=20, state="disabled", font=("Consolas", 9))
        self.log_text.pack(fill="both", expand=True)

        # Render Initial Demo Scene
        self.render_demo_scene()

    def render_demo_scene(self):
        """繪製測試用的牌局場景 (S7.2 Demo)。"""
        w, h = 800, 600 # Approximate canvas size
        center_x, center_y = w/2, h/2
        
        # 1. Draw My Hand (Bottom)
        my_cards = [("A", "S"), ("10", "H"), ("7", "D"), ("K", "C"), ("2", "S")]
        start_x = 250
        for i, (rank, suit) in enumerate(my_cards):
            selected = (i == 2) # Simulate selecting the 3rd card
            CardRenderer.draw_card(self.game_canvas, start_x + i*80, 500, rank, suit, selected=selected, tag=f"hand_{i}")

        # 2. Draw Opponents (Hidden)
        # Top
        for i in range(3):
            CardRenderer.draw_card(self.game_canvas, 300 + i*60, 50, "", "", hidden=True)
        
        # Left
        for i in range(3):
            CardRenderer.draw_card(self.game_canvas, 50, 200 + i*40, "", "", hidden=True)
            
        # Right
        for i in range(3):
            CardRenderer.draw_card(self.game_canvas, 700, 200 + i*40, "", "", hidden=True)

        # 3. Draw Table (Played Cards)
        # North played
        CardRenderer.draw_card(self.game_canvas, center_x, center_y - 60, "Q", "H")
        # West played
        CardRenderer.draw_card(self.game_canvas, center_x - 80, center_y, "9", "S")
        # East played
        CardRenderer.draw_card(self.game_canvas, center_x + 80, center_y, "J", "D")

        self.log_message("System: Demo UI loaded.")
        self.log_message("System: Select a card to play.")

    def log_message(self, msg):
        """Append text to the log widget."""
        self.log_text.config(state="normal")
        self.log_text.insert("end", msg + "\n")
        self.log_text.see("end")
        self.log_text.config(state="disabled")

    def check_messages(self):
        """每 100ms 檢查一次是否有新訊息。"""
        msg = self.client.get_message()
        if msg:
            self.handle_message(msg)
        
        self.root.after(100, self.check_messages)

    def handle_message(self, msg):
        """處理來自 Server 的 JSON 封包。"""
        m_type = msg.get("type")
        
        if m_type == "WELCOME":
            self.player_id = msg.get("player_id")
            self.setup_game_ui()
            print(f"Joined as {self.player_id}")
            
        elif m_type == "ERROR":
            messagebox.showerror("Server Error", msg.get("message", "Unknown error"))
            self.btn_connect.config(state="normal")

if __name__ == "__main__":
    root = tk.Tk()
    app = CardArenaApp(root)
    root.mainloop()
