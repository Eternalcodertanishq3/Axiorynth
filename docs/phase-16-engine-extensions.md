# Axiorynth - Phase 16 Engine Extensions

## Scope

This documents completion of Sub-Phase 1 (Phase C: Engine Extensions).

It introduces core tactical search upgrades to the negamax search engine in Rust: check extensions, singular extensions, and the countermove heuristic.

## Singular Extensions

Implemented in:

```text
engine/src/search.rs
```

Features:
- At Principal Variation (PV) nodes, if a Transposition Table (TT) move is stored at sufficient depth, we verify if it is "singular" (the only clear best move).
- We perform a reduced-depth search excluding the TT move with a narrow aspiration window.
- If all other moves score significantly below the TT move's stored score, we extend the TT move's search depth by 1 ply.
- Prevents the engine from overlooking critical tactical responses or checking sequences.

## Check Extensions

Implemented in:

```text
engine/src/search.rs
```

Features:
- Extends the search depth by 1 ply if a legal move puts the opponent's king in check.
- Prevents the search tree from prematurely clipping sharp check lines, resolving check evasion combinations accurately.

## Countermove Heuristic

Implemented in:

```text
engine/src/search.rs (SearchHeuristics)
```

Features:
- Stores a 2D table `[[Option<Move>; 64]; 64]` mapping `[from_square][to_square]` of the previous move.
- After a beta cutoff, the quiet move that caused the cutoff is recorded as a "counter" to the opponent's previous move.
- During move ordering, countermoves are given a score bonus (higher than history heuristic but below killer moves).
- Dynamically improves move ordering by playing logical defensive or offensive counter responses.

## Verification

Tests cover:
- All unit and integration tests compile and run.
- Engine search successfully identifies check extensions and singular extensions.
- Move ordering ordering heuristics include countermove scores.
