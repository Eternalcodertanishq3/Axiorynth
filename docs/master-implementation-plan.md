# Axiorynth Master Implementation Plan

## Purpose

This document is the master plan for Axiorynth, the custom Rust chess engine and
future chess platform.

The project goal is to grow in layers:

1. correct chess rules
2. playable search
3. engine protocol
4. performance systems
5. visible numeric analysis
6. game history and replay
7. bot levels
8. adaptive memory
9. training and export reports
10. research roadmap toward stronger play

The full web app, backend database, online play, and visual chessboard will be a
separate application layer on top of this engine core. The engine-side work here
is designed so that frontend/backend work can call clean Rust APIs later.

## Completed Phases

### Phase 1 - Engine Foundation

Implemented:

- bitboard board representation
- FEN parse/export
- legal move generation
- castling, en passant, promotion
- make/undo
- perft and divide
- first numeric evaluation

### Phase 2 - First Bot Search

Implemented:

- negamax
- alpha-beta pruning
- quiescence search
- candidate ranking
- principal variation
- search stats
- CLI commands for best move, eval, and perft

### Phase 3 - UCI Protocol

Implemented:

- UCI loop
- position commands
- go commands
- stop and quit
- time-aware search limits
- UCI options

### Phase 4 - Strength And Performance

Implemented:

- Zobrist hashing
- compact undo
- transposition table
- hash move ordering
- killer moves
- history heuristic
- per-depth UCI info streaming
- benchmark command

### Phase 5 - Analysis Report Layer

Implemented:

- structured analysis report
- legal move list
- evaluation math
- search math
- candidate moves
- analysis CLI command

## Engine-Side Phases Completed In Final Pass

### Phase 6 - Game History And Replay

Goal:

- store complete local games
- replay any ply
- export readable move/history reports

Implemented:

- `Game`
- `GameRecord`
- `GameResult`
- UCI move replay
- FEN after every move
- result detection
- text export

### Phase 7 - Bot Levels

Goal:

- provide named bot levels from easy to stronger
- expose deterministic search limits for each level

Implemented:

- `BotLevel`
- `BotProfile`
- level presets
- `choose_bot_move`
- CLI bot command

### Phase 8 - Adaptive Memory

Goal:

- remember player tendencies over games
- generate notes the bot/backend can use later

Implemented:

- `PlayerMemory`
- opening move counters
- result counters
- repeated first-move detection
- simple adaptive notes

### Phase 9 - Training And Export Reports

Goal:

- summarize games and memory into training-friendly reports
- provide text output now, JSON/database later

Implemented:

- `TrainingReport`
- game summaries
- memory summaries
- CSV-like game rows

### Phase 10 - Research Roadmap Artifacts

Goal:

- keep future high-strength engine work concrete and measurable

Implemented:

- research milestones
- benchmark targets
- tuning parameter list
- recommended next engineering tasks

## Future App Layer

After the engine-side phases, the separate app layer should include:

- Rust or Node backend API
- persistent database
- React/TypeScript frontend
- chessboard UI
- side math panel
- history page
- replay viewer
- bot selection screen
- adaptive profile display
- optional online multiplayer

Those require frontend/backend scaffolding and dependency choices, so they are
tracked after the engine core is ready.
