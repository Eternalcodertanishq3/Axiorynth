# Axiorynth - Phase 14 Engine Time Management

## Scope

This documents Phase C (Time Management).

Added a dynamic `TimeManager` in `engine/src/search.rs` that monitors search budgets based on remaining `wtime`/`btime` and increments. Search limits are no longer hardcoded to depths, but calculate soft and hard time deadlines. The iterative deepening loop breaks cleanly when the `TimeManager` triggers `should_stop_soft()` or `should_stop_hard()`.

## UCI Updates

Implemented in:

```text
engine/src/uci.rs
```

Features:

- Parses standard UCI clock parameters (`wtime`, `btime`, `winc`, `binc`).
- Directly seeds these time constraints into `SearchLimits`.
- Feeds `SearchLimits` to the `TimeManager` for dynamic deadline enforcement during search.

## TimeManager Implementation

Implemented in:

```text
engine/src/search.rs
```

Features:

- Monitors search budgets based on `SearchLimits` time parameters.
- Replaces hardcoded depth limitations with adaptive soft and hard time deadlines.
- `should_stop_soft()` signals the iterative deepening loop to cleanly stop before the next depth starts.
- `should_stop_hard()` aborts the search immediately during deep tree traversal, preventing the engine from ever flagging (running out of time).

## Verification

Phase 14 checks cover:

- The UCI parser correctly identifies and processes clock parameters.
- `SearchLimits` correctly calculates target time bounds.
- The `TimeManager` stops iterative deepening dynamically upon hitting the soft boundary.
- Deep searches correctly break instantly when hitting the hard boundary, returning the best move found so far.
