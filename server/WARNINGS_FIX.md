# Rust Warning Cleanup Recommendations

This document records the current compile-time warnings observed in the server
and suggested fixes to silence them without changing behavior.

---

## Observed Warnings (cargo test)

1) Unused imports
- `server/src/game/engine.rs`: `use std::collections::HashMap;`
- `server/src/game/mod.rs`: `CardData`, `Deck`, `Rank`, `Suit`, `GamePlayer`
- `server/src/lobby/mod.rs`: `Player`, `Room`
- `server/src/net/handler.rs`: `ClientMessage`, `ServerMessage`
- `server/src/net/mod.rs`: `ConnectionInfo`, `ConnectionRegistry`, `SharedRegistry`,
  `create_shared_registry`, `create_client_channel`, `ClientHeartbeatState`,
  `HeartbeatTracker`, `check_stale_clients`, `get_heartbeat_stats`
- `server/src/main.rs`: `GamePhase`

2) Unused variables / assignments
- `server/src/game/engine.rs`: `player` (unused)
- `server/src/main.rs`: `players_data` assigned but never read
- `server/src/main.rs`: `code` unused

3) Dead code (items never used)
- `server/src/game/engine.rs`: `current_player_conn_id`, `find_player_idx`
- `server/src/lobby/handshake.rs`: `create_player_info`
- `server/src/lobby/room.rs`: `RoomState::Finished`, `find_player`, `find_player_by_id`,
  `get_room_for_conn_mut`
- `server/src/net/connection.rs`: `ConnectionInfo::new`, `SharedRegistry`,
  `create_shared_registry`, `ConnectionInfo`
- `server/src/net/heartbeat.rs`: `ClientHeartbeatState.addr`, `get_heartbeat_stats`
- `server/src/protocol/codec.rs`: `Codec::peer_addr`

---

## Suggested Fixes

### A) Remove or Gate Unused Imports

- Delete unused `use` lines, or
- Keep only in the configurations that need them:
  - If used only in tests, guard with `#[cfg(test)]`.
  - If used behind a feature flag, guard with `#[cfg(feature = "...")]`.

### B) Silence Unused Variables

Option 1: Prefix with underscore.
```rust
let _player = &self.players[player_idx];
```

Option 2: Remove if not needed.

### C) Remove Dead Code or Mark Intended API

- If truly unused, delete functions/types/variants.
- If planned to be used later, mark explicitly:
```rust
#[allow(dead_code)]
pub fn current_player_conn_id(&self) -> Option<ConnectionId> { ... }
```

### D) Consolidate Module Re-exports

In `server/src/game/mod.rs`, only re-export items that are consumed by other
modules. Unused re-exports trigger warnings in tests.

---

## Recommended Order

1) Remove unused imports (easy, no behavior change).
2) Rename unused variables (`_var`) to silence warnings.
3) Decide on dead code strategy:
   - delete if not needed
   - or `#[allow(dead_code)]` if reserved for public API

---

## Notes

- Keeping the warning list clean improves signal-to-noise in CI.
- Most of these warnings are non-functional; the cleanup is safe when done
  carefully.
