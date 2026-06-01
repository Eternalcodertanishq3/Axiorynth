# Axiorynth - Phase 4 Strength And Performance

## Phase 4 Goal

Phase 4 upgrades Axiorynth from a simple searchable engine into a stronger and
more engine-like core.

This phase focuses on:

- faster make/undo
- stable position hashing
- transposition table search
- better move ordering
- per-depth UCI info streaming
- benchmark command

## New And Updated Files

```text
engine/src/zobrist.rs
engine/src/board.rs
engine/src/search.rs
engine/src/uci.rs
engine/src/bench.rs
engine/src/bin/axiorynth.rs
docs/phase-4-strength-and-performance.md
```

## Zobrist Hashing

Every board now carries a stable 64-bit hash:

```rust
board.hash()
```

The hash includes:

- piece locations
- side to move
- castling rights
- en passant file

The engine also exposes:

```rust
board.compute_hash()
board.recompute_hash()
```

These are used in tests to prove that make/undo restores the exact same
position identity.

## Compact Undo

Phase 1 to 3 used full board cloning for undo. Phase 4 replaces that with a
compact `UndoState`.

Undo now stores only:

- the move
- moved piece
- captured piece, if any
- castling rook movement, if any
- previous castling rights
- previous en passant square
- previous clocks
- previous hash

This keeps move search cleaner and prepares the engine for deeper search.

## Transposition Table

Search now has a transposition table.

It stores:

- position hash
- search depth
- score
- bound type
- best move

Bound types:

```text
Exact
Lower
Upper
```

The table is reused across iterative deepening depths, so information found at
shallower depths can guide deeper searches.

Search stats now include:

```text
tt hits
tt stores
hashfull permill
```

## Better Move Ordering

Move ordering now uses:

- transposition-table best move first
- MVV-LVA captures
- promotions
- killer moves
- history heuristic
- castling bonus
- center-square bonus

This helps alpha-beta reach cutoffs sooner.

## Per-Depth UCI Streaming

Phase 3 only printed final UCI search output.

Phase 4 streams one UCI `info` line after each completed iterative-deepening
depth:

```text
info depth 1 score cp 73 nodes 40 nps 1200 hashfull 1 time 33 pv a2a3
info depth 2 score cp 12 nodes 481 nps 3500 hashfull 3 time 137 pv a2a3 b8c6
bestmove a2a3
```

This is important for GUIs and for the future math panel.

## Benchmark Command

New command:

```powershell
cargo run -p axiorynth_engine --bin axiorynth -- bench 3
```

It searches a small fixed suite:

- start position
- Kiwipete
- tactical middlegame

It prints:

- best move
- score
- searched nodes
- nodes per second
- hashfull
- elapsed time

## Verification

Commands run:

```powershell
cargo fmt
cargo test
```

Result:

```text
33 tests passed
0 tests failed
```

Phase 4 tests cover:

- Zobrist key determinism
- hash restoration after make/undo
- compact undo restoring FEN and hash
- transposition table stores
- iterative per-depth callback
- benchmark suite execution

## Current Limitations

Still planned for later:

- true incremental hash updates for every metadata micro-change
- replacement strategy tuning
- larger configurable search heuristics
- null-move pruning
- late move reductions
- aspiration windows
- principal variation search
- deeper benchmark suite

Phase 4 gives Axiorynth its first real search-performance backbone.
