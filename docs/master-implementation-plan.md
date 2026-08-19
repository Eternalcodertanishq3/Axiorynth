# Axiorynth Master Implementation Plan

## Purpose

This document is the master plan for Axiorynth, the custom Rust chess engine and chess platform.

The project grew in the following layers:
1. Correct chess rules (Bitboards, legal move generator, perft)
2. Playable search (Alpha-beta, transposition tables, move ordering)
3. Engine protocols (UCI, CLI commands)
4. Local game play & Bot Levels
5. Adaptive bot memory (Player tendencies, mistake clustering)
6. Dedicated Rust backend (Axum HTTP API, SQLite)
7. Live search streaming (WebSockets, cancellation)
8. Engine strength upgrades (Singular & Check extensions, countermove heuristics)
9. Research features (SPSA tuning, Gauntlet runner, Opening Book)
10. Machine learning (NNUE training pipeline & inference)
11. Endgames (Syzygy Tablebase HTTP integration)
12. Online multiplayer (User Auth, matchmaking, live WS play, spectating)

---

## Completed Phases

### Phase 1 to 5 - Engine Core & CLI
Implemented basic board representations, move generators, alpha-beta negamax search, quiescence, Zobrist hashing, transposition tables, move ordering (killers, history), UCI streaming, and evaluation breakdown.

### Phase 6 to 10 - Bot Levels & Memory
Implemented game state recording, replay plys, ten named bot levels, player opening tendencies tracking, mistake clustering, CSV training exports, and the research roadmap.

### Phase 11 - Next.js Play App
Implemented the local React chessboard interface, human-vs-bot settings, promotion modal, move history, and static evaluation breakdown math panels.

### Phase 12 - Stronger Backend Layer
Implemented a Rust Axum API server backed by SQLite to persist user profiles, game summaries, bot memory adjustments, and active session histories.

### Phase 13 - Real-Time Math Streaming
Implemented WebSocket search progress streaming from the Rust engine, enabling live updates (depth, NPS, PV line, transposition table hits) on the frontend.

### Phase 14 & 15 - Time Management & Adaptation
Implemented time-aware Search limits, mistake clustering, and persistent bot adjustments to penalize previously failed candidate moves.

### Phase 16 - Engine Extensions (Sub-Phase 1)
Implemented check extensions, singular extensions, and countermove heuristics in `engine/src/search.rs` to make bot searches tactically resilient.
- [Phase 16 Engine Extensions](file:///c:/Personal%20Projects/chess/docs/phase-16-engine-extensions.md)

### Phase 17 & 18 - SPSA, Gauntlet & Opening Book (Sub-Phases 2-4)
Implemented full 13-parameter SPSA gradient tuning, a CLI-based gauntlet runner, and Zobrist-hashed opening book lookup tables.
- [Phase 17 Research Tuning](file:///c:/Personal%20Projects/chess/docs/phase-17-research-tuning.md)
- [Phase 18 Opening Book](file:///c:/Personal%20Projects/chess/docs/phase-18-opening-book.md)

### Phase 19 & 20 - NNUE & Tablebases (Sub-Phases 5-6)
Implemented a sparse HalfKP-to-accumulator NNUE evaluation network, weight save/load systems, a backpropagation training CLI, and root Syzygy tablebase probing with optimal DTZ pathfinding.
- [Phase 19 NNUE Pipeline](file:///c:/Personal%20Projects/chess/docs/phase-19-nnue-pipeline.md)
- [Phase 20 Tablebase Integration](file:///c:/Personal%20Projects/chess/docs/phase-20-tablebase-integration.md)

### Phase 21 - Online Platform (Sub-Phases 7-8)
Implemented session-based user authentication, matchmaking pairing, live multiplayer WebSocket streams, automatic Elo rating updates, and spectating support.
- [Phase 21 Online Platform](file:///c:/Personal%20Projects/chess/docs/phase-21-online-platform.md)
