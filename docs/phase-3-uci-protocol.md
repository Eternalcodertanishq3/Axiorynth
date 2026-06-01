# Axiorynth - Phase 3 UCI Protocol And Engine Process

## Phase 3 Goal

Phase 3 makes Axiorynth usable as a real chess engine process.

Before this phase, Axiorynth could be used as:

- a Rust library
- a small CLI for `eval`, `best`, and `perft`

After this phase, it can also speak **UCI**, the Universal Chess Interface used
by chess GUIs and engine managers.

This means future desktop apps, web backends, and external chess GUIs can talk
to Axiorynth through a standard command protocol.

## New Capability

Implemented:

- UCI protocol loop
- `uci`
- `isready`
- `ucinewgame`
- `position startpos`
- `position fen ...`
- position move replay
- `go depth N`
- `go movetime N`
- `go wtime ... btime ... winc ... binc ... movestogo ...`
- `go nodes N`
- `go infinite`
- `stop`
- `quit`
- `setoption`
- asynchronous search worker
- stop-aware search control
- iterative deepening
- UCI `info` output
- UCI `bestmove` output

## New And Updated Files

```text
engine/src/uci.rs
engine/src/search.rs
engine/src/bin/axiorynth.rs
engine/src/lib.rs
docs/phase-3-uci-protocol.md
```

## Running The Engine

Running without arguments starts UCI mode:

```powershell
cargo run -p axiorynth_engine --bin axiorynth
```

You can also start it explicitly:

```powershell
cargo run -p axiorynth_engine --bin axiorynth -- uci
```

The older helper commands still work:

```powershell
cargo run -p axiorynth_engine --bin axiorynth -- eval startpos
cargo run -p axiorynth_engine --bin axiorynth -- best startpos 3
cargo run -p axiorynth_engine --bin axiorynth -- perft startpos 3
```

## UCI Handshake

Input:

```text
uci
isready
```

Output:

```text
id name Axiorynth 0.3.0
id author Axiorynth Project
option name SearchDepth type spin default 4 min 1 max 64
option name QuiescenceDepth type spin default 4 min 0 max 16
option name CandidateCount type spin default 5 min 1 max 20
uciok
readyok
```

## UCI Options

Supported options:

```text
SearchDepth
QuiescenceDepth
CandidateCount
```

Examples:

```text
setoption name SearchDepth value 5
setoption name QuiescenceDepth value 3
setoption name CandidateCount value 8
```

These options control the default search when a `go` command does not provide a
specific depth.

## Position Commands

Start position:

```text
position startpos
```

Start position with moves:

```text
position startpos moves e2e4 e7e5 g1f3
```

FEN position:

```text
position fen 7k/8/5K2/8/8/6Q1/8/8 w - - 0 1
```

FEN position with moves:

```text
position fen rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1 moves e2e4
```

Axiorynth validates replayed moves through its legal move generator. If a move
is illegal, it reports a UCI `info string` error.

## Go Commands

Fixed depth:

```text
go depth 4
```

Fixed move time:

```text
go movetime 1000
```

Clock-based:

```text
go wtime 30000 btime 30000 winc 1000 binc 1000 movestogo 20
```

Node-limited:

```text
go nodes 5000
```

Infinite until stopped:

```text
go infinite
stop
```

## Search Output

Example:

```text
info depth 1 score cp 71 nodes 60 nps 1304 time 46 pv a2a3
bestmove a2a3
```

The `info` line includes:

- depth
- score
- searched nodes
- nodes per second
- elapsed time
- principal variation

The `bestmove` line is the move the GUI/backend should play.

## Iterative Deepening

Phase 3 adds:

```rust
iterative_deepening(board, limits, control)
```

Instead of jumping straight to one depth, the engine now searches:

```text
depth 1
depth 2
depth 3
...
```

The last completed depth is preserved. This matters for time-managed searches:
if time runs out while depth 5 is incomplete, Axiorynth can still return the best
result from depth 4.

## Stop-Aware Search

Phase 3 adds:

```rust
SearchControl
```

The UCI loop starts search on a worker thread. When the main protocol loop
receives:

```text
stop
quit
```

it calls:

```rust
control.request_stop()
```

The search checks this control during main search and quiescence search, then
returns the best available result.

## Backend-Ready Shape

The UCI layer is useful for GUIs, but the same Phase 3 pieces also prepare the
future backend:

```rust
SearchLimits {
    max_depth,
    quiescence_depth,
    candidate_count,
    move_time,
    node_limit,
}

SearchControl

SearchResult {
    best_move,
    score,
    depth,
    stats,
    principal_variation,
    candidates,
}
```

The frontend math panel can eventually use `SearchResult::as_math_lines()` and
the evaluator's `EvalBreakdown::as_math_lines()`.

## Verification

Commands run:

```powershell
cargo fmt
cargo test
cargo run -p axiorynth_engine --bin axiorynth -- uci
```

Live UCI smoke test:

```text
uci
isready
position startpos moves e2e4 e7e5
go depth 1
```

Observed output included:

```text
uciok
readyok
info depth 1 score cp 71 nodes 60 nps 1304 time 46 pv a2a3
bestmove a2a3
```

Stop smoke test:

```text
go depth 64
stop
quit
```

Observed output included a legal `bestmove` after the stop request.

Unit test result:

```text
25 tests passed
0 tests failed
```

## New Tests

Phase 3 adds tests for:

- iterative deepening returns a move
- search control can stop search
- parsing `position startpos moves ...`
- parsing `position fen ...`
- parsing `go depth`, `go movetime`, and `go nodes`
- clock-based time allocation
- `setoption` updates engine options

## Current Limitations

The UCI layer is functional, but still early.

Known limitations:

- no transposition table yet
- no hash option yet
- no ponder mode
- no multipv option
- no tablebase support
- no opening book
- no per-depth streaming during iterative search; it reports the final result
- stop can still take a little time during deeper searches because the current
  make/undo and move generation are correctness-first, not optimized yet

These are normal for this stage. The important achievement is that Axiorynth now
has a standard engine-process interface.

## Phase 4 Recommended Target

Phase 4 should make the engine stronger and faster internally.

Recommended Phase 4:

- Zobrist hashing
- transposition table
- incremental make/undo
- iterative per-depth UCI info streaming
- principal variation stability improvements
- better move ordering
- search benchmarks

After Phase 4, Axiorynth should search much deeper and become a more serious
opponent.
