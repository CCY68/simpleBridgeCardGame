from clients.ai_cli.fallback import FallbackStrategy


def test_fallback_picks_smallest_legal():
    strategy = FallbackStrategy()
    legal_moves = ["KS", "2H", "10D", "AC"]
    chosen = strategy.choose(legal_moves, hand=legal_moves, table_cards=[], trick_num=1)

    assert chosen == "2H"
