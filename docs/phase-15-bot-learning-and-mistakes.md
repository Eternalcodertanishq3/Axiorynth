# Axiorynth - Phase 15 Bot Learning and Mistakes

## Scope

This documents Phase D (Learning Bot).

The bot now persistently penalizes moves that previously led to it losing, and identifies human blunders to provide training recommendations.

## Move-Ordering Penalties

Implemented in:

```text
engine/src/bot.rs
```

Features:

- When the bot loses a game, its moves from that session are logged in the SQLite `bot_adjustments` table along with their move history context.
- `choose_bot_move_with_callback` reads these persistent penalties.
- Heavily deprioritizes those specific candidate moves during the `pick_candidate` selection step.
- Effectively forces the bot to explore alternative lines rather than repeating previously losing mistakes.

## Mistake Clustering

Implemented in:

```text
engine/src/memory.rs
```

Features:

- A fast blunder detection pass (`analyze_mistakes`) scans completed human games.
- Identifies player moves where the static evaluation drops by more than 2 pawns after the opponent's reply.
- These blunders are captured and persisted in the `bot_memory` table under `mistake_clusters`.

## Training Recommendations API

Implemented in the Axum Backend:

```text
backend/src/main.rs (or similar route handler)
```

Endpoint:

```text
GET /api/training/recommendations
```

Features:

- Returns a comprehensive summary of the player's wins and losses.
- Highlights favorite opening moves extracted from player history.
- Provides adaptive bot behavior hints.
- Returns a list of recent major blunders for the player to review and analyze.

## Verification

Phase 15 checks cover:

- SQLite correctly stores `bot_adjustments` and `mistake_clusters`.
- Candidate selection correctly deprioritizes penalized moves in subsequent matches against the same opening lines.
- Blunder detection accurately filters drops in static evaluation greater than 2 pawns.
- `GET /api/training/recommendations` properly responds with aggregate training insights and recent blunders.
