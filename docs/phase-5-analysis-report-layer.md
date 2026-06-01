# Axiorynth - Phase 5 Analysis Report Layer

## Phase 5 Goal

Phase 5 creates the first structured analysis layer for the future frontend math
panel and backend API.

The engine can now produce one complete report containing:

- current FEN
- legal move list
- legal move count
- numeric evaluation breakdown
- search result
- principal variation
- ranked candidate moves
- node counts and search stats
- transposition table stats

## New And Updated Files

```text
engine/src/analysis.rs
engine/src/bin/axiorynth.rs
engine/src/lib.rs
docs/phase-5-analysis-report-layer.md
```

## Main API

```rust
analyze_position(board, limits, control)
```

Returns:

```rust
AnalysisReport {
    fen,
    legal_moves,
    evaluation,
    search,
}
```

The report can be rendered as text:

```rust
report.as_lines()
```

## CLI Command

New command:

```powershell
cargo run -p axiorynth_engine --bin axiorynth -- analyze startpos 2
```

You can also analyze a FEN:

```powershell
cargo run -p axiorynth_engine --bin axiorynth -- analyze "<fen>" 2
```

## Report Shape

Example sections:

```text
Axiorynth analysis report
fen: rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1
legal move count: 20
legal moves: a2a3 a2a4 b1a3 ...

Evaluation math
material: 4000 - 4000 = +0
piece-square: -75 - -75 = +0
mobility: (20 - 20) * 2 = +0
...

Search math
search depth: 2
best move: a2a3
score: +12 centipawns
principal variation: a2a3 b8c6
main nodes: ...
quiescence nodes: ...
transposition table: ...
candidate 1: ...
```

## Why This Matters

The user-facing app needs a side panel that shows the actual numeric chess
thinking. Phase 5 is the first reusable backend shape for that.

The frontend can eventually display:

- legal moves
- best line
- candidate ranking
- material math
- mobility math
- pawn structure math
- king safety math
- search nodes
- pruning stats
- table hits

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

Phase 5 tests cover:

- analysis report includes 20 start position legal moves
- evaluation math is included
- search result is included
- analysis does not mutate the input board

## Current Limitations

The report is text-renderable but not JSON-serialized yet. A later backend phase
can add `serde` and expose this as HTTP/WebSocket data.

Phase 5 completes the first real math-report foundation for the future UI.
