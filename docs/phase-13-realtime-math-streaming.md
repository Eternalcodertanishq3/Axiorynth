# Axiorynth - Phase 13 Real-Time Math Streaming

## Scope

This phase adds real-time engine thinking visualization via WebSocket streaming.

The user sees the engine's search progress live: depth by depth, with changing
best moves, node counts, cutoff counters, transposition table usage, and
principal variation updates.

## WebSocket Protocol

Implemented in:

```text
backend/src/ws.rs
```

Connection:

```text
ws://127.0.0.1:8080/ws
```

### Client Messages

Start a search:

```json
{"action": "search", "fen": "...", "level": 3}
```

Cancel the running search:

```json
{"action": "cancel"}
```

### Server Messages

Progress (sent after each completed depth):

```json
{
  "type": "progress",
  "depth": 3,
  "best_move": "e2e4",
  "score": 12,
  "pv": ["e2e4", "e7e5", "g1f3"],
  "nodes": 1234,
  "qnodes": 567,
  "nps": 45000,
  "tt_hits": 89,
  "tt_stores": 234,
  "hashfull": 12,
  "beta_cutoffs": 45,
  "q_beta_cutoffs": 23,
  "killer_uses": 67,
  "elapsed_ms": 150
}
```

Result (final, sent when the search completes):

```json
{
  "type": "result",
  "selected_move": "e2e4",
  "best_move": "e2e4",
  "score": 12,
  "pv": ["e2e4", "e7e5"],
  "nodes": 5678,
  "nps": 50000,
  "elapsed_ms": 300
}
```

Error:

```json
{"type": "error", "message": "..."}
```

## Streaming Metrics

| Metric | Field | Description |
|---|---|---|
| Depth progress | `depth` | Current completed search depth |
| Best move | `best_move` | Best move found at this depth |
| Score | `score` | Evaluation score in centipawns |
| Principal variation | `pv` | Array of UCI moves in the PV line |
| Main nodes | `nodes` | Nodes searched in main search |
| Quiescence nodes | `qnodes` | Nodes searched in quiescence |
| Nodes per second | `nps` | Search speed |
| TT hits | `tt_hits` | Transposition table cache hits |
| TT stores | `tt_stores` | Transposition table entries stored |
| Hash fullness | `hashfull` | TT usage in permill (0–1000) |
| Beta cutoffs | `beta_cutoffs` | Alpha-beta cutoffs in main search |
| Q beta cutoffs | `q_beta_cutoffs` | Cutoffs in quiescence search |
| Killer uses | `killer_uses` | Killer move heuristic activations |
| Elapsed time | `elapsed_ms` | Time spent searching in milliseconds |

## Search Callback Mechanism

Implemented in:

```text
engine/src/search.rs
```

Function:

```rust
iterative_deepening_with_callback<F>(board, limits, control, on_depth)
```

- accepts `FnMut(&SearchResult, Duration)` callback
- fires after each completed depth of iterative deepening
- provides the full `SearchResult` with stats, PV, candidates, and score
- used by `choose_bot_move_with_callback()` in `bot.rs` for the WebSocket
  handler

## Frontend Display

Implemented in:

```text
apps/web/app/page.tsx
```

Features:

- live search progress panel with real-time updates
- metric grid showing: best move, total nodes, PV line, depth, score
- detailed math panel showing all WebSocket metrics
- cancel search button to abort long-running searches
- bot thinking indicator with animated state

## Verification

Phase 13 checks cover:

- WebSocket connection opens correctly during bot turn
- progress messages stream for each completed depth
- all 14 metrics are displayed in the frontend
- cancel action stops the search and returns control to the user

Commands run:

```powershell
cargo build --release -p axiorynth_backend
npm run web:build
```

Both commands complete without warnings or errors.

Phase 13 gives Axiorynth a live window into the engine's thinking, turning the
search from a black box into a transparent, depth-by-depth visualization.
