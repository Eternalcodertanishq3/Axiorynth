# Axiorynth - Phase 12 Stronger Backend Layer

## Scope

This phase builds a dedicated Rust Axum backend service that replaces the
previous Next.js API route → CLI bridge pattern.

The backend owns game sessions, player profiles, saved games, and bot memory
in a local SQLite database. All game state now lives server-side, giving the
frontend a clean REST interface instead of shelling out to the engine binary
on every request.

## Axum HTTP Service

Implemented in:

```text
backend/src/main.rs
```

Features:

- Axum HTTP server on port `8080` with CORS enabled
- REST API routes for game state, profiles, saved games, and bot memory
- session-based game management with persistent server-side state
- automatic database initialization and schema migrations

Routes:

```text
POST   /api/state              — stateless engine computation (legacy compatibility)
POST   /api/session            — create a new game session
GET    /api/session/:id        — fetch session state
POST   /api/session/:id/move   — apply a move to the session
DELETE /api/session/:id        — end/delete a session
GET    /api/profile            — fetch player profile
POST   /api/profile/result     — update win/loss/draw stats
GET    /api/games              — list saved games
POST   /api/games              — archive a game
GET    /api/bot/memory         — fetch bot memory
POST   /api/bot/memory         — save bot memory
```

## SQLite Persistence

Implemented in:

```text
backend/src/db.rs
```

Tables:

- `player_profiles` — id, name, wins, losses, draws, created_at
- `saved_games` — id, saved_at, moves (JSON), result, mode, bot_level
- `bot_memory` — player_id, opening_tendencies (JSON), mistake_clusters (JSON), bot_adjustments (JSON)
- `game_sessions` — id, fen, moves (JSON), mode, bot_level, result, created_at, updated_at

Features:

- automatic table creation on startup
- default player profile seeding
- session CRUD operations
- opening tendency extraction from completed games

## Bot Memory And Adaptation

After a game session ends, opening moves are extracted and stored as tendencies
in `bot_memory`.

The WebSocket handler reads player profiles and dynamically adjusts evaluation
weights based on win/loss history:

- stronger players face more aggressive bot play (higher center control,
  mobility weights)
- weaker players face gentler bot play (reduced aggression)

This builds on the Phase 8 `PlayerMemory` system by persisting adaptive data
across sessions through SQLite rather than in-memory structs.

## CLI

```powershell
cargo run -p axiorynth_backend
```

## Verification

Commands run:

```powershell
cargo build --release -p axiorynth_backend
```

Phase 12 checks cover:

- backend compiles in release mode
- database initializes correctly with all schema tables
- REST endpoints return proper JSON responses
- session-based gameplay persists across requests
- bot memory rows are created after completed games

Phase 12 gives Axiorynth a real application backend, moving game state off the
CLI bridge and into a persistent, session-aware service layer.
