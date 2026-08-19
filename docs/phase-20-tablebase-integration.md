# Axiorynth - Phase 20 Tablebase Integration

## Scope

This documents completion of Sub-Phase 6 (Phase E: Research Program - Tablebases).

It integrates Lichess Syzygy tablebase probing into the engine search root to guarantee perfect play in endgames with 7 or fewer pieces.

## Syzygy API Probing

Implemented in:

```text
engine/src/tablebase.rs
engine/src/lib.rs
```

Features:
- Interfaces with the public Lichess Syzygy API: `https://tablebase.lichess.ovh/standard`.
- Probes only if the total piece count on the board is $\le 7$.
- Formats FEN parameters safely into URL strings.
- Implements an HTTP agent using the lightweight `ureq` crate with a strict 500ms timeout.
- Caches lookup results in an in-memory `RwLock<HashMap<u64, Option<WdlResult>>>` to prevent redundant network lookups.

## Search Integration

Implemented in:

```text
engine/src/search.rs
```

Features:
- Probes the tablebase at the root of `iterative_deepening_internal`.
- If a position is in the tablebase:
  - Fetches the WDL and DTZ metrics of all legal moves.
  - Determines the best move according to exact endgame rules:
    1. Minimizes opponent WDL (maximizing our own advantage).
    2. Ties in winning lines are resolved by choosing the move that minimizes DTZ (closest to mate).
    3. Ties in losing lines are resolved by choosing the move that maximizes DTZ (stretching the game to force draw opportunities).
  - Bypasses the alpha-beta search tree completely.
  - Instantly returns the best move and its mapped score (win, blessed win, draw, cursed loss, or loss).

## Verification

Tests cover:
- Piece counting on standard vs sparse boards.
- Mapping WDL results to correct search scores (win mapped close to mate, draw mapped to zero).
- Warning-free compilation.
- Graceful fallbacks on network timeouts or API errors.
