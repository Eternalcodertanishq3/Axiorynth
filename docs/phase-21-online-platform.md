# Axiorynth - Phase 21 Online Platform

## Scope

This documents completion of Sub-Phases 7 & 8 (Phase F: Online Platform Backend & Frontend).

It establishes a secure authenticated multiplayer platform with real-time matchmaking, live WebSocket-driven game play, Elo rating updates, and spectating support.

## Dedicated Rust Backend

Implemented in:

```text
backend/src/auth.rs
backend/src/matchmaking.rs
backend/src/live.rs
backend/src/db.rs
backend/src/main.rs
```

Features:
- **Authentication**: Robust session-based authentication mapping token strings to user records. Password hashing implemented via salted default hashing (`DefaultHasher`).
- **Database Schema**: Extends the SQLite schema to include `users` and `live_games` tables.
- **Matchmaking**: In-memory FIFO matchmaking queue. Computes rating parity color assignments on game pairing.
- **Live Games Store**: Thread-safe in-memory cache of ongoing games. All locks are scoped to avoid holding guards across `.await` boundaries, satisfying Rust's WebSocket `Send` constraints.
- **Move Validation**: Replays moves on the engine core, checks for move legality, updates the active FEN, and saves move logs to SQLite.
- **Elo Ratings**: Automatically recalculates ratings of both players on game completion using the standard Elo formula:
  $$R_{\text{new}} = R_{\text{old}} + 32 \times (S - E)$$
  and updates user profiles.

## Next.js Frontend

Implemented in:

```text
apps/web/app/online/page.tsx
apps/web/app/page.tsx
apps/web/components/ChessBoard.tsx
```

Features:
- **Premium Themes**: Exposes 5 beautifully curated color palettes (`emerald`, `midnight`, `cyberpunk`, `charcoal`, `wood`) that update pieces, squares, coordinates, and borders instantly.
- **Dynamic Board & Inline Coordinates**: Encapsulated into a reusable `<ChessBoard>` component that positions rank and file labels inside corner squares of the edges, aligning correctly based on player orientation.
- **Client-Side Check Detector**: Automatically checks positions client-side to apply pulsing red glow highlights around checked kings in both offline and online play.
- **Web Audio Sound Synthesizer**: Generates 4 distinct chess sounds (move, capture, check, gameover) using native browser oscillators/gains to guarantee 100% offline compatibility with no asset overhead.
- **Settings Popover Menu**: Adds a sleek appearance configuration drawer to the game toolbar allowing toggles for coordinates, sound, and theme selection.
- **Authentication Forms**: Clean login and registration interfaces with input validation.
- **Dashboard**: Displays user credentials, Elo rating, a matchmaking queue button, and a list of active ongoing games.
- **Pulsing Queue Status**: Shows elapsed queue time and supports cancellation.
- **Online Game Board**: Real-time interactive board using the shared component, syncing states via WebSocket.
- **Client Turn Enforcement**: Restricts move inputs to the active player's pieces only on their turn.
- **Pawn Promotion Modal**: Seamless selection of promotion pieces (Queen, Rook, Bishop, Knight) with a cancel option.
- **Spectator Mode**: Connects to ongoing games in read-only mode to spectate live multiplayer play.
- **Navigation**: Integrated header links between local engine play and the online play platform.

## Verification

Tests cover:
- Database migrations and table seeding.
- User registration, login, and bearer authentication header extractors.
- Safe WebSocket streaming without lock contentions.
- Client board clicks replaying legally on backend and broadcasting correctly.
- Automatic Elo updates on checkmate, draw, or resignation.
- Clean Next.js static build compilation without type errors.
