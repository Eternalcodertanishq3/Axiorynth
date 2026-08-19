# Axiorynth - Phase 18 Opening Book

## Scope

This documents completion of Sub-Phase 4 (Phase E: Research Program - Opening Book).

It introduces a lightweight, Zobrist-hash based Opening Book system to store and replay standard opening lines.

## The Opening Book System

Implemented in:

```text
engine/src/book.rs
engine/src/lib.rs
```

Features:
- `OpeningBook` and `BookEntry` structs serialize/deserialize to/from standard JSON files (`axiorynth.book`).
- Positions are mapped via their Zobrist hash values (represented as hex strings).
- Tracks played UCI moves, play weights (frequency), and average win rates.
- `generate_from_games` parses games represented as lists of UCI moves and results (1-0, 0-1, 1/2-1/2).
- Extracts and indexes the first 12 plies of each game.
- Dynamically updates move frequencies and average scores per position.

## CLI Integration

Implemented in:

```text
engine/src/bin/axiorynth.rs
```

Commands:
- `book-gen <num_games> <depth>`: Runs self-play matches at specified depth to construct and save an opening book to `axiorynth.book`.
- `book-probe <fen_or_startpos>`: Loads the opening book and displays available moves, weights, and win rates for the specified position.

## Search Integration

Implemented in:

```text
engine/src/bot.rs
```

Features:
- When choosing a move for a bot player, the opening book is probed first using the current position's Zobrist hash.
- If book moves are present, the highest weighted move is selected instantly without running a search tree, saving CPU time and ensuring standard chess openings are played.

## Verification

Tests cover:
- Serialization and deserialization to/from files.
- Averaging of game outcomes (merging duplicate moves).
- Correct probing of book moves by position hashes.
