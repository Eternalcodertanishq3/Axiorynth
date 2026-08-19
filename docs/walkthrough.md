# Walkthrough - Axiorynth Full-Stack Platform Upgrades

This document summarizes the design, features, verification steps, and performance validation of the full-stack upgrades implemented for the Axiorynth chess platform.

---

## 1. Accomplished Features & Architecture

The Axiorynth codebase has been transformed from a single-process local command line tool into a complete full-stack chess platform with high-performance search selectivity, adaptive learning, database persistence, and real-time visualization.

### Engine Search & Evaluation Upgrades (`engine/src/`)
- **Principal Variation Search (PVS)**: Implemented PVS in [search.rs](file:///c:/Personal%20Projects/chess/engine/src/search.rs) to search the first move (PV move) with a full window, and subsequent moves with a null window (`alpha, alpha + 1`). Re-searches with a full window only if a null-window search fails high.
- **Aspiration Windows**: Narrows search boundaries by starting with a small window `[val - margin, val + margin]` around the prior depth's score, widening the window only on fails.
- **Null-Move Pruning (NMP)**: Prunes stable positions by passing the move (null move) and searching at a reduced depth. If the result remains above beta, we prune.
- **Late Move Reductions (LMR)**: Safely reduces depth for quiet, late-sorted moves, re-searching at full depth if they exceed alpha.
- **Futility Pruning**: Prunes low-promise quiet moves at shallow search depths near leaf nodes if the static evaluation plus a margin falls below alpha.
- **Static Exchange Evaluation (SEE)**: Evaluates capture sequences on a single square using recursive ray attacks, ordering bad captures after quiet moves to optimize alpha-beta cutoff efficiency.
- **SPSA Evaluation Parameters**: Introduced `EvalConfig` and thread-safe global `EVAL_CONFIG` in [eval.rs](file:///c:/Personal%20Projects/chess/engine/src/eval.rs) to support dynamic weights for Knight, Bishop, Rook, Queen, and pawn structure elements.
- **HalfKP NNUE Extractor**: Created `get_half_kp_features` to map pieces relative to white/black king positions into a standard feature index representation for NNUE network training.

### Axum Dedicated Backend Crate (`backend/`)
- **Next-Gen Workspace Crate**: Configured `axiorynth_backend` as a workspace member in the root [Cargo.toml](file:///c:/Personal%20Projects/chess/Cargo.toml) with dependencies like `axum`, `tokio`, `sqlx` (SQLite), and `tower-http` (CORS).
- **SQLite Database Persistence (`backend/src/db.rs`)**: Implemented dynamic sqlx queries with schema migration tables for:
  - `player_profiles`: Wins, losses, draws, creation times.
  - `saved_games`: Signatures, results, levels, and moves (stored as JSON arrays).
  - `bot_memory`: Opening move frequencies, mistake patterns, and bot adaptations.
- **Real-Time WebSocket Streams (`backend/src/ws.rs`)**: Connects Next.js to the search routine on background blocking threads, pushing live updates for depth, PV path, NPS, total nodes, transposition table hits, and beta cutoffs.
- **REST Endpoints (`backend/src/main.rs`)**: Bootstraps services on `127.0.0.1:8080` with CORS support for profile fetching, database game storage, and state mapping.

### Next.js WebSocket Integration & Premium Chessboard (`apps/web/`)
- **Reactive WebSocket Client**: Establishes a WebSocket connection in [page.tsx](file:///c:/Personal%20Projects/chess/apps/web/app/page.tsx) whenever the bot starts searching, streaming progress live to the user.
- **Search Math Dashboard**: Renders beautiful gauges for NPS, hashfull, cutoffs, PV paths, and depth.
- **REST State Syncing**: Syncs and retrieves profiles, saved game records, and adaptive bot memories directly from the Axum SQLite backend.
- **Sleek Board Aesthetics**: Uses the newly encapsulated `<ChessBoard>` component supporting 5 harmonized color themes (`emerald`, `midnight`, `cyberpunk`, `charcoal`, `wood`), coordinate toggles, check highlights, and Web Audio oscillator-synthesized sound effects (for moves, captures, checks, and gameover conditions).
- **Online & Local Settings Popover**: Includes a unified Settings dropdown popover inside both local and online platform toolbars.

---

## 2. CLI Research Commands Validation

We have added two CLI commands to enable self-play evaluation and hyperparameter tuning of evaluation terms.

### Self-Play Match Runner
Generates game data and saves FENs to `self_play_data.txt` for network training:
```powershell
.\target\release\axiorynth.exe self-play 2 3 3
```
*Output:*
```text
Starting self-play matches: 2 games, Bot Level 3 vs Bot Level 3
Game 1 finished. Result: DrawFiftyMove
Game 2 finished. Result: DrawFiftyMove
Self-play complete! Wins: 0, Losses: 0, Draws: 2
Saved training FENs to self_play_data.txt
```

### SPSA Parameter Optimizer
Runs Simultaneous Perturbation Stochastic Approximation to optimize evaluation parameters relative to a baseline player:
```powershell
.\target\release\axiorynth.exe spsa-tune 2
```
*Output:*
```text
Starting SPSA tuning loop for 2 iterations...
Iteration 1: Knight val = 320, Bishop val = 330
Iteration 2: Knight val = 320, Bishop val = 330
SPSA tuning complete. Final Knight: 320, Bishop: 330
```

---

## 3. Automated Verification & Build Metrics

### Rust Unit & Integration Tests
All 42 engine tests pass successfully, confirming correctness of the new search selectivity rules and board integrity:
```powershell
cargo test
```
*Result:*
```text
running 42 tests
test eval::tests::half_kp_features_are_extracted_correctly ... ok
test eval::tests::spsa_updates_evaluation_values ... ok
test eval::tests::material_advantage_is_visible_in_centipawns ... ok
...
test result: ok. 42 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

### Next.js Production Build
The Next.js client compiles successfully without warnings or static-site generation failures:
```powershell
npm run web:build
```
*Result:*
```text
▲ Next.js 16.2.7 (Turbopack)
  Creating an optimized production build ...
✓ Compiled successfully in 1288ms
  Running TypeScript ...
  Finished TypeScript in 2.1s ...
✓ Generating static pages using 5 workers (4/4) in 630ms
```

### Backend Crate Compilation
The backend compiles cleanly in release mode:
```powershell
cargo build --release -p axiorynth_backend
```
*Result:*
```text
    Finished `release` profile [optimized] target(s) in 49.16s
```
