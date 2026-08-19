# Axiorynth Next Plans And Current Limitations

## Where Axiorynth Stands Now

Axiorynth has evolved from a local engine prototype into a complete, production-ready multiplayer chess platform. The Rust engine owns the rules of chess and evaluation math, and the Web/API layers enable local play, bot matches, and online multiplayer matches.

Completed systems:
- **Rust Bitboard Engine**: Legal move generation, Zobrist hashing, transposition tables.
- **Search Upgrades**: PV Search, Aspiration Windows, Null-Move Pruning, LMR, SEE, Singular & Check Extensions, and Countermove heuristics.
- **Opening Book**: Generation of books from self-play games and automatic bot probing.
- **NNUE Evaluator**: Feature extractor (HalfKP), training pipeline (backpropagation, SGD, dataset generator), weight saving, and live inference.
- **Tablebase Integration**: Direct Syzygy probing via Lichess HTTP API for positions with 7 or fewer pieces, with optimal DTZ pathfinding.
- **Dedicated Axum Backend**: User authentication, FIFO matchmaking queue, and WebSocket live game handlers.
- **Next.js Web App**: Offline bot-play mode, saved local games, live search stream, and multiplayer online match rooms with spectator support. Features a premium themeable `<ChessBoard>` component with inline coordinates, check highlighting, settings popover, and synthesized Web Audio sound cues.

## What Complete Means In This Pass

This pass concludes the primary development of all core phases:
- A user can register, login, view their Elo, and queue for live multiplayer.
- Matches are paired and moves are validated server-side.
- In endgames, the engine play is guided perfectly by tablebase hits.
- Neural evaluation evaluates positions if NNUE weights are trained and loaded.

## Current Limitations & Future Plans

The current architecture is stable and clean, leaving several avenues for future research and optimization:

### 1. Engine Strength (Future Research)
- **Local Syzygy Lookups**: Transition from Lichess HTTP Syzygy probing to local `.rtbw` / `.rtbz` file lookups to bypass web API rate limits.
- **NNUE Scaling**: Train the HalfKP network with millions of positions on GPU instead of thousands on CPU.
- **Search Enhancements**: Implement multi-cut, history pruning, and double-null-move pruning.

### 2. Infrastructure Scaling
- **PostgreSQL Database**: Migrate from the local SQLite database to a cloud-managed PostgreSQL database once user volume scales.
- **JWT Authentication**: Transition from stateful sessions in memory to stateless JWT tokens to support multi-node backend clustering.
- **Matchmaking Ratings**: Expand matchmaking to pair players within expanding rating ranges rather than immediate pairing.

### 3. Frontend Polishing
- **Drag-and-Drop Board**: Replace the current button-click chessboard with a standard drag-and-drop library (e.g., Chessground).
- **Elo Rating Graphs**: Display rating history charts for players in the user profile dashboard.
