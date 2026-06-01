# Axiorynth Next Plans And Current Limitations

## Where Axiorynth Stands Now

Axiorynth has moved from a raw engine experiment into a complete local chess
platform foundation. The Rust core owns the chess truth, and the web app calls
that core for legal moves, bot decisions, numeric evaluation, search stats, and
candidate lines.

Completed systems:

- Rust bitboard engine
- FEN parsing and exporting
- legal move generation
- castling, en passant, promotion, make/undo
- perft correctness tests
- numeric evaluator with visible math
- alpha-beta negamax search
- quiescence search
- transposition table
- move ordering with hash moves, killer moves, and history heuristic
- UCI protocol
- benchmark command
- analysis reports
- game history and replay structures
- bot levels 1 to 10
- local adaptive memory model
- training report model
- research roadmap model
- machine-readable frontend state bridge
- Next.js play app with board, bot mode, move history, replay, saved games, and math panel

## What Complete Means In This Pass

This pass makes Axiorynth complete as a local full-stack chess application:

- a user can open the app
- play human vs human
- play human vs bot
- choose bot level
- see legal possibilities
- inspect real engine math
- save and replay local games
- view win/loss/draw history from local storage
- run the Rust engine through CLI or UCI

This does not mean Axiorynth is already stronger than Stockfish. That is a
multi-year research target. The project is now built so that future strength
work can happen seriously instead of being trapped inside a toy prototype.

## Current Limitations

### Engine Strength

- no principal variation search yet
- no aspiration windows yet
- no null-move pruning yet
- no late move reductions yet
- no futility pruning yet
- no static exchange evaluation yet
- no singular extensions yet
- no advanced check extensions yet
- no tuned evaluation weights yet
- no opening book yet
- no endgame tablebases yet
- no NNUE evaluator yet
- no self-play Elo framework yet

### Learning And Adaptation

- current adaptive memory is local engine-side logic, not a persistent database
- the bot does not yet update its strategy live during a game
- repeated-pattern learning is not yet plugged into search move ordering
- no long-term player profile storage beyond browser local saved games
- no anti-blunder training loop yet

### Backend

- the web app currently calls the Rust CLI through a Next.js API route
- there is no dedicated Axum service yet
- no SQLite/PostgreSQL persistence yet
- no authenticated users yet
- no WebSocket search stream yet
- no cloud deployment pipeline yet

### Frontend

- the current board is custom React/CSS, not a drag-and-drop chessboard package
- saved games use browser local storage
- online mode is not implemented yet
- replay is local to the current saved move list
- advanced analysis graphs are still future work

## Why Next.js Is The Right Frontend Choice Now

Next.js is a strong fit for this stage because it gives Axiorynth:

- a polished React interface
- built-in API routes for calling the Rust engine locally
- a path toward future deployment
- TypeScript safety around engine responses
- simple app routing for future history, training, and online pages

Rust remains the engine and eventual dedicated backend language. Next.js owns
the interactive application surface.

## Next Engineering Plan

### Phase A - Stronger Backend Layer

Replace the CLI bridge with a dedicated Rust backend:

- Axum HTTP API
- WebSocket analysis stream
- SQLite local database
- game sessions
- player profiles
- saved games
- bot memory tables

### Phase B - Real-Time Math Streaming

Show the engine thinking while it searches:

- depth 1, 2, 3 progress rows
- changing best move
- node count growth
- alpha-beta cutoff counters
- transposition table usage
- principal variation updates

### Phase C - Engine Strength Push

Add serious search upgrades:

- principal variation search
- aspiration windows
- null-move pruning
- late move reductions
- static exchange evaluation
- better time management
- tuned move ordering

### Phase D - Learning Bot

Turn memory into actual play adaptation:

- persistent player pattern database
- repeated opening detection
- mistake clustering
- move-ordering penalties for previously failed bot ideas
- per-player bot preparation
- post-game training recommendations

### Phase E - Research Program

Build the path toward top-class strength:

- self-play runner
- gauntlet runner against other engines
- Elo estimation
- SPSA tuning
- opening book generation
- NNUE feature extraction
- NNUE training pipeline
- tablebase integration

### Phase F - Online Platform

Add multiplayer and accounts after the local app is stable:

- authentication
- matchmaking
- live games
- spectating
- server-side validation
- cloud database
- deployable API and frontend

## North Star

Axiorynth should become a chess engine that does not hide its thinking. The user
should be able to see the numbers, the legal possibilities, the candidate moves,
the search tree signals, and eventually the learning model's memory.

The target is not only to play chess. The target is to make the intelligence
inspectable.
