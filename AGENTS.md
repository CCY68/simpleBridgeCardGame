# Repository Guidelines

## Project Structure & Module Organization

CardArena is a LAN-only card game with a Rust server and Python clients. The repository is currently scaffolded; implementation files are expected to land in the paths below (see `README.md` and `PROJECT.md`).

- `server/`: Rust host node (`server/src/` for code, `server/tests/` for tests, `Cargo.toml` at root).
- `clients/`: Python clients (`clients/human_cli/`, `clients/ai_cli/`, `clients/common/`).
- `protocol/`: Protocol specs (`protocol/protocol.md`, `protocol/posix_mapping.md`).
- `progress/`: Story tracking and notes (`progress/stories.md`, `progress/notes.md`).
- `scripts/`: Local demo helpers (planned; see `README.md`).

## Build, Test, and Development Commands

Commands are documented in `.claude/CLAUDE.md` and `GEMINI.md` and apply once code is present:

- `cd server && cargo check` - Rust compile check.
- `cd server && cargo test` - Rust unit/integration tests.
- `cd server && cargo fmt` - Rust formatting via rustfmt.
- `pytest clients/` - Python client tests.
- `cd server && cargo run -- --port 8888` - Start the server (planned).
- `python clients/human_cli/main.py --host 127.0.0.1 --port 8888` - Run a human client (planned).

## Coding Style & Naming Conventions

- Rust: format with `cargo fmt`, lint with `cargo clippy`; modules live under `server/src/` and use standard `snake_case` for files and functions.
- Python: format with `black`, lint with `ruff` or `flake8`; follow `snake_case` for modules/functions and `PascalCase` for classes.
- Protocol: NDJSON messages must be one JSON object per line (see `protocol/protocol.md`).

## Testing Guidelines

- Rust: use `cargo test`; keep tests under `server/tests/` or module tests in `server/src/`.
- Python: use `pytest clients/`; name tests `test_*.py` and test functions `test_*`.
- Coverage targets are not defined yet; focus on protocol framing, socket lifecycle, and game-state rules.

## Commit & Pull Request Guidelines

- Commit messages follow the story tag format from `.claude/CLAUDE.md`, e.g. `[S1.2] Implement accept loop`.
- One story per commit or PR when possible; include a brief description and reference the story in `progress/stories.md`.
- PRs should include: summary, testing performed (commands + results), and any protocol changes.

## Agent-Specific Instructions

- Claude Code: implementation and tests; follow `.claude/CLAUDE.md`.
- Gemini CLI: code review and QA; follow `GEMINI.md`.
- Codex (this agent): architecture co-designer, primary PM, and integration test QA; keep progress tracking in `progress/stories.md` and `progress/notes.md`, and focus on cross-module readiness and end-to-end verification planning.

## Codex Status Summary

**Timestamp**: 2026-01-14 17:48:33 +0800

- Server dev marked complete in `progress/srv_stories.md`; client dev marked complete in `progress/clnt_stories.md`.
- PM QA status in `progress/stories.md` still pending update; integration tests reported OK by another QA.
- Local tests run: `cd server && cargo test` (37 passed, warnings only), `pytest clients/` (5 passed after adding smoke tests), `python scripts/test_heartbeat.py` (HB_PING/HB_PONG OK).
- New commit created: `64dcaac` `[S6.3] Add client QA smoke tests` (adds `clients/tests/*`, updates `progress/notes.md`).
- Push to `origin` pending due to network restrictions in this environment.
