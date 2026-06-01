# Axiorynth - Phase 1 Engine Foundation

## Identity

The engine is named **Axiorynth**.

The name combines the feeling of an **axiom**, something mathematically solid,
with a deep strategic maze. That fits the project goal: a chess engine whose
strength is built from correctness, measurable algorithms, search, data
structures, and visible numeric reasoning.

## Phase 1 Goal

Phase 1 establishes the core chess rules layer. This is the part every strong
engine needs before search strength, bot levels, learning, history, UI, or
analysis panels can be trusted.

The goal was:

- create the Rust engine crate
- represent the board efficiently
- parse and export FEN
- generate legal chess moves
- make and undo moves
- verify correctness with perft tests
- expose a first numeric evaluation breakdown

## Tech Used

- **Rust 2024 edition**
- **Cargo workspace**
- **No external Rust dependencies**
- **Bitboards** for piece placement and board operations
- **Perft** for move-generation correctness testing

Current structure:

```text
chess/
  Cargo.toml
  docs/
    phase-1-engine-foundation.md
  engine/
    Cargo.toml
    src/
      lib.rs
      board.rs
      types.rs
      mv.rs
      movegen.rs
      perft.rs
      eval.rs
```

## Core Modules

### `types.rs`

Defines the core chess types:

- `Color`
- `PieceKind`
- `Piece`
- `Square`

Squares use the standard engine-friendly index layout:

```text
a1 = 0
b1 = 1
...
h8 = 63
```

Each square can produce a `u64` bit mask with exactly one bit enabled.

### `board.rs`

Stores the full board state:

- piece bitboards
- side to move
- castling rights
- en passant square
- halfmove clock
- fullmove number

Pieces are stored as:

```rust
pieces[color][piece_kind] -> u64 bitboard
```

This means Axiorynth can ask questions like:

```text
Where are all white knights?
Where are all black rooks?
Which squares are occupied by white?
Which piece is on e4?
```

### `mv.rs`

Defines moves:

- source square
- target square
- optional promotion piece
- move kind

Supported move kinds:

- quiet move
- capture
- double pawn push
- en passant
- king-side castle
- queen-side castle

Moves can be displayed in UCI format:

```text
e2e4
e7e8q
e1g1
```

### `movegen.rs`

Generates pseudo-legal moves, then filters them into legal moves.

Implemented rules:

- pawn pushes
- double pawn pushes
- pawn captures
- en passant
- promotions to queen, rook, bishop, knight
- knight moves
- bishop moves
- rook moves
- queen moves
- king moves
- castling
- check filtering
- attacked-square detection

Also added:

```rust
find_legal_move_by_uci(board, "e2e4")
```

This will be useful later when the backend or frontend sends player moves as
UCI strings.

### `perft.rs`

Implements move-generation verification:

```rust
perft(board, depth)
divide(board, depth)
```

`perft` counts all legal move paths to a fixed depth. If the count differs from
known reference numbers, the move generator has a bug.

`divide` returns the per-move node counts, which helps isolate exactly which
move branch is wrong during debugging.

### `eval.rs`

Implements the first numeric evaluation breakdown.

Current evaluation terms:

- material
- mobility
- total score from White's perspective
- total score from side-to-move perspective

Example output from `EvalBreakdown::as_math_lines()`:

```text
material: 3900 - 3900 = +0
mobility: (20 - 20) * 2 = +0
total: +0 centipawns from White perspective
side-to-move total: +0 centipawns
```

This is the beginning of the future analysis panel. Later phases will add more
terms such as king safety, center control, pawn structure, piece-square tables,
threats, passed pawns, and search statistics.

## Algorithms And Data Structures

### Bitboards

A bitboard is a 64-bit integer. Each bit represents one chess square.

Example:

```text
bit 0  -> a1
bit 7  -> h1
bit 56 -> a8
bit 63 -> h8
```

This lets the engine represent all pieces of one type compactly and perform
fast board operations.

### Legal Move Generation

Axiorynth currently uses this pipeline:

```text
generate pseudo-legal moves
make each move
check whether own king is attacked
undo the move
keep only legal moves
```

This is correctness-first. Later, when the search engine arrives, we can
optimize move generation without changing the public behavior.

### Make And Undo

Phase 1 uses a simple and reliable undo system:

```text
clone previous board state
make move
restore previous board state on undo
```

This is not the fastest final design, but it is very safe for Phase 1. In a
future high-performance search phase, we can replace it with a compact
incremental undo record.

### Attack Detection

The engine can detect whether a square is attacked by:

- pawns
- knights
- bishops
- rooks
- queens
- kings

This powers:

- legal move filtering
- check detection
- castling validation

### Perft

Perft is the main correctness gate for chess engines. It does not evaluate
positions or pick good moves. It only verifies that legal move generation is
correct.

The Phase 1 suite checks:

- start position
- Kiwipete
- tricky endgame with special pawn cases
- promotion-heavy pressure position
- tactical middlegame position

Reference counts are aligned with common chess-engine perft test positions,
including the Chessprogramming perft result set:

https://www.chessprogramming.org/Perft_Results

## Verification

Commands run:

```powershell
cargo fmt
cargo test
```

Result:

```text
13 tests passed
0 tests failed
```

Covered by tests:

- FEN start position round trip
- invalid king-count rejection
- start position legal move count
- castling generation
- en passant make/undo
- UCI legal move lookup
- material evaluation
- start position perft depths 1 to 3
- Kiwipete perft depths 1 to 3
- tricky endgame perft depths 1 to 3
- promotion pressure perft depths 1 to 3
- tactical middlegame perft depths 1 to 3

## What Phase 1 Does Not Do Yet

Phase 1 intentionally does not include:

- minimax
- alpha-beta search
- bot move selection
- UCI engine loop
- transposition table
- opening book
- database
- frontend
- game history
- adaptive learning

Those belong in later phases. The important thing is that the chess rules layer
is now ready to support them.

## Phase 2 Recommended Target

Phase 2 should turn the rules engine into the first playable bot.

Recommended Phase 2 features:

- static evaluation improvements
- minimax
- alpha-beta pruning
- quiescence search starter
- basic move ordering
- fixed-depth bot move selection
- first `best_move` API
- simple CLI or backend-callable engine entry point

After Phase 2, Axiorynth should be able to play legal chess moves as a real bot,
even if it is still weak compared with future versions.
