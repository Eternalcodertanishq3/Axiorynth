# Axiorynth - Phase 2 First Bot And Search

## Phase 2 Goal

Phase 2 turns Axiorynth from a rules engine into the first playable bot core.

The engine can now:

- evaluate positions with richer numeric chess terms
- search legal moves
- choose a best move
- rank candidate moves
- detect simple checkmates
- stabilize tactical leaf nodes with quiescence search
- expose search math for the future analysis panel
- answer basic CLI commands before the backend exists

This is still an early engine. It is not trying to be Stockfish-class yet. The
purpose of Phase 2 is to create a correct search spine that later phases can
make faster, deeper, smarter, and more explainable.

## New Files

```text
engine/
  src/
    search.rs
    bin/
      axiorynth.rs

docs/
  phase-2-first-bot-search.md
```

## Updated Files

```text
engine/src/lib.rs
engine/src/eval.rs
```

## Evaluation Upgrades

Phase 1 had material and mobility. Phase 2 expands this into a more useful
centipawn-style evaluation.

Current terms:

- material
- piece-square tables
- mobility
- center control
- pawn structure
- king safety

The main function is:

```rust
evaluate(board)
```

For search, the direct side-to-move score is:

```rust
evaluate_side_to_move(board)
```

The evaluation can produce math lines:

```text
material: 3900 - 3900 = +0
piece-square: 12 - 8 = +4
mobility: (31 - 27) * 2 = +8
center: 26 - 18 = +8
pawn structure: -10 - -20 = +10
king safety: 12 - 4 = +8
total: +38 centipawns from White perspective
side-to-move total: +38 centipawns
```

These lines are designed for the future frontend math panel.

## Search Algorithm

Phase 2 adds a fixed-depth bot search in `search.rs`.

Implemented:

- negamax search
- alpha-beta pruning
- quiescence search
- checkmate and stalemate terminal scoring
- basic move ordering
- candidate move ranking
- principal variation output
- search statistics

Primary API:

```rust
best_move(board, SearchConfig::default())
```

Search configuration:

```rust
SearchConfig {
    max_depth: 3,
    quiescence_depth: 4,
    candidate_count: 5,
}
```

Search result:

```rust
SearchResult {
    best_move,
    score,
    depth,
    stats,
    principal_variation,
    candidates,
}
```

## Alpha-Beta Search

Axiorynth uses negamax alpha-beta:

```text
score(position) = -score(position after opponent move)
```

This gives one clean search function instead of separate maximizing and
minimizing functions.

Alpha-beta keeps two bounds:

- `alpha`: best score already guaranteed for the side to move
- `beta`: score the opponent can already avoid

When a move proves the position is at least `beta`, the remaining sibling moves
can be skipped.

Tracked numbers:

- main search nodes
- quiescence nodes
- beta cutoffs
- quiescence beta cutoffs
- max ply reached

## Quiescence Search

At depth zero, the engine does not immediately stop on every position. If the
position has forcing captures or promotions, it searches those noisy moves first.

This reduces horizon-effect mistakes such as:

```text
Engine thinks it wins a queen,
but one move later its own queen is captured.
```

Current noisy moves:

- captures
- promotions

If the side to move is in check, quiescence searches legal evasions instead of
using a quiet stand-pat score.

## Move Ordering

Move ordering helps alpha-beta prune more aggressively.

Current ordering bonuses:

- captures first
- MVV-LVA capture scoring
- promotions
- castling
- moves to central squares

MVV-LVA means:

```text
Most Valuable Victim - Least Valuable Attacker
```

Example:

```text
capturing a queen with a pawn is ordered very high
capturing a pawn with a queen is ordered much lower
```

## Mate Scores

The engine uses:

```rust
MATE_SCORE = 30000
```

Checkmate scores are adjusted by ply:

```text
mate now is better than mate later
getting mated later is better than getting mated now
```

## CLI

Phase 2 adds a small command-line tool:

```powershell
cargo run -p axiorynth_engine --bin axiorynth -- eval startpos
cargo run -p axiorynth_engine --bin axiorynth -- best startpos 3
cargo run -p axiorynth_engine --bin axiorynth -- perft startpos 3
```

You can also pass a quoted FEN:

```powershell
cargo run -p axiorynth_engine --bin axiorynth -- best "7k/8/5K2/8/8/6Q1/8/8 w - - 0 1" 2
```

Example search output:

```text
search depth: 1
best move: a2a3
score: +73 centipawns
principal variation: a2a3
main nodes: 20
quiescence nodes: 20
beta cutoffs: 0 main, 19 quiescence
candidate 1: a2a3 = +73
candidate 2: a2a4 = +73
candidate 3: b1a3 = +73
```

The exact best move will change as evaluation improves.

## Verification

Commands run:

```powershell
cargo fmt
cargo test
cargo run -p axiorynth_engine --bin axiorynth -- best startpos 1
```

Result:

```text
18 tests passed
0 tests failed
```

New Phase 2 tests cover:

- richer evaluation math lines
- simple mate-in-one search
- board restoration after search
- candidate move reporting
- winning a hanging queen

Phase 1 tests still cover:

- FEN round trip
- legal move generation
- castling
- en passant
- UCI move lookup
- perft reference positions

## Current Limitations

Phase 2 is a first bot, not a strong engine yet.

Known limitations:

- no transposition table
- no iterative deepening
- no time control
- no opening book
- no endgame tablebases
- no killer moves
- no history heuristic
- no null-move pruning
- no late move reductions
- no NNUE or trained evaluator
- make/undo still uses full board cloning

These are planned later. The important Phase 2 achievement is that Axiorynth can
now legally search, score, explain, and choose moves.

## Phase 3 Recommended Target

Phase 3 should make the engine usable by external systems.

Recommended Phase 3:

- UCI protocol loop
- iterative deepening
- time-managed search
- search cancellation
- engine options
- cleaner analysis trace events
- first backend-callable API shape

After Phase 3, Axiorynth should be able to plug into a chess GUI or backend and
respond like a real engine process.
