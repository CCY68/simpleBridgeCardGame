import tkinter as tk

class CardRenderer:
    """Helper class to render playing cards on a Tkinter Canvas."""
    
    WIDTH = 70
    HEIGHT = 100
    RADIUS = 10  # For rounded corners (simulated)
    
    SUIT_Symbols = {
        'S': '♠', 'H': '♥', 'D': '♦', 'C': '♣'
    }
    
    SUIT_COLORS = {
        'S': 'black', 'H': 'red', 'D': 'red', 'C': 'black'
    }

    @staticmethod
    def draw_card(canvas: tk.Canvas, x: int, y: int, rank: str, suit: str, 
                  hidden: bool = False, selected: bool = False, tag: str = "card"):
        """
        Draws a card at (x, y).
        
        Args:
            canvas: The Tkinter Canvas widget.
            x, y: Top-left coordinates.
            rank: 'A', '2'...'10', 'J', 'Q', 'K'.
            suit: 'S', 'H', 'D', 'C'.
            hidden: If True, draw the card back.
            selected: If True, draw with a highlight border or offset.
            tag: Canvas tag for event handling (clicking).
        """
        w = CardRenderer.WIDTH
        h = CardRenderer.HEIGHT
        
        # Offset for selection effect
        if selected:
            y -= 10

        # Draw card body (Background)
        # Tkinter doesn't support native rounded rectangles easily, so we use a polygon or just a rect.
        # For simplicity in this MVP, we use a standard rectangle with a thick border.
        fill_color = "white" if not hidden else "#3366cc"  # Blue back
        outline_color = "gold" if selected else "black"
        border_width = 3 if selected else 1

        # Body
        canvas.create_rectangle(
            x, y, x + w, y + h,
            fill=fill_color, outline=outline_color, width=border_width,
            tags=(tag, f"{tag}_bg")
        )

        if hidden:
            # Draw pattern on the back
            canvas.create_line(x, y, x+w, y+h, fill="white", width=1, tags=tag)
            canvas.create_line(x, y+h, x+w, y, fill="white", width=1, tags=tag)
            return

        # Draw Corner Rank/Suit (Top-Left)
        color = CardRenderer.SUIT_COLORS.get(suit, 'black')
        symbol = CardRenderer.SUIT_Symbols.get(suit, '?')
        
        # Rank (Top-Left)
        canvas.create_text(
            x + 5, y + 5,
            text=rank,
            fill=color,
            font=("Arial", 12, "bold"),
            anchor="nw",
            tags=tag
        )
        # Small Suit (Below Rank)
        canvas.create_text(
            x + 5, y + 20,
            text=symbol,
            fill=color,
            font=("Arial", 12),
            anchor="nw",
            tags=tag
        )

        # Draw Center Suit (Large)
        canvas.create_text(
            x + w/2, y + h/2,
            text=symbol,
            fill=color,
            font=("Arial", 32),
            anchor="center",
            tags=tag
        )

        # Draw Corner Rank/Suit (Bottom-Right) - Inverted
        # Note: Tkinter text rotation is complex, so we draw upright but in the corner.
        # Rank (Bottom-Right)
        canvas.create_text(
            x + w - 5, y + h - 5,
            text=rank,
            fill=color,
            font=("Arial", 12, "bold"),
            anchor="se",
            tags=tag
        )
        # Small Suit (Above Rank)
        canvas.create_text(
            x + w - 5, y + h - 20,
            text=symbol,
            fill=color,
            font=("Arial", 12),
            anchor="se",
            tags=tag
        )
