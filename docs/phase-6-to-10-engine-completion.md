# Axiorynth - Phases 6 To 10 Engine Completion

## Scope

This document covers the remaining engine-side phases after the analysis layer.

The full graphical app, database backend, online mode, and visual replay pages
are intentionally left for the future application layer. These phases complete
the engine-side foundations those systems need.

## Phase 6 - Game History And Replay

Implemented in:

```text
engine/src/game.rs
```

Features:

- `Game`
- `GameRecord`
- `GameResult`
- local UCI move application
- FEN before and after each move
- evaluation after each move
- result detection
- replay to any ply
- text history export

CLI:

```powershell
cargo run -p axiorynth_engine --bin axiorynth -- game e2e4 e7e5 g1f3
```

## Phase 7 - Bot Levels

Implemented in:

```text
engine/src/bot.rs
```

Features:

- 10 named bot levels
- level-specific search depth
- level-specific quiescence depth
- level-specific candidate counts
- deterministic lower-level candidate selection
- full current-strength level 10 profile

CLI:

```powershell
cargo run -p axiorynth_engine --bin axiorynth -- bot 5 startpos
```

## Phase 8 - Adaptive Memory

Implemented in:

```text
engine/src/memory.rs
```

Features:

- `PlayerMemory`
- result counters
- favorite first move detection
- repeated early sequence tracking
- adaptive notes

CLI:

```powershell
cargo run -p axiorynth_engine --bin axiorynth -- memory e2e4 e7e5
```

This is the first version of the learning system. It does not train a neural
network yet, but it starts storing concrete player tendencies.

## Phase 9 - Training And Export Reports

Implemented in:

```text
engine/src/training.rs
```

Features:

- `TrainingReport`
- `TrainingGameRow`
- memory summary export
- CSV-like game rows
- final FEN and UCI move log export

CLI:

```powershell
cargo run -p axiorynth_engine --bin axiorynth -- train e2e4 e7e5
```

## Phase 10 - Research Roadmap Artifacts

Implemented in:

```text
engine/src/research.rs
```

Features:

- measurable research milestones
- tuning parameter list
- benchmark target list
- roadmap text export

CLI:

```powershell
cargo run -p axiorynth_engine --bin axiorynth -- roadmap
```

## Public Engine APIs

New exports:

```rust
Game
GameRecord
GameResult
BotLevel
BotProfile
BotMove
PlayerMemory
PlayerResult
TrainingReport
TrainingGameRow
ResearchRoadmap
ResearchMilestone
TuningParameter
```

## Verification

The full test suite covers:

- game recording
- replay
- checkmate result detection
- bot profile generation
- bot legal move selection
- memory learning
- training report export
- research roadmap generation

These phases complete the first engine-core roadmap from rules to search,
analysis, history, bot levels, memory, export, and research planning.
