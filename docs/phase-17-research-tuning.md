# Axiorynth - Phase 17 Research Tuning

## Scope

This documents completion of Sub-Phases 2 & 3 (Phase E: Research Program - SPSA & Gauntlet).

It expands the SPSA tuner to optimize all 13 evaluation configuration parameters simultaneously and introduces a CLI-based gauntlet runner to measure relative engine strength using Elo difference calculations.

## Full 13-Parameter SPSA Tuning

Implemented in:

```text
engine/src/bin/axiorynth.rs
engine/src/eval.rs
```

Features:
- Extends the vector SPSA parameter tuner from 2 to all 13 key parameters in `EvalConfig` (pawn, knight, bishop, rook, queen, center attack, center occupancy, doubled pawn penalty, isolated pawn penalty, passed pawn bonus, king safety shield, king safety attacked ring, and mobility multiplier).
- Simultaneously perturbs all 13 variables at each iteration.
- Evaluates the gradient by running evaluation matches against the current active baseline config.
- Saves the final optimized evaluation config as a JSON file named `tuned_eval.json`.
- Implements a `load-config` command to load and apply any saved `tuned_eval.json` instantly.

## Gauntlet Runner

Implemented in:

```text
engine/src/bin/axiorynth.rs
```

Features:
- Command:
  ```text
  axiorynth gauntlet <games> <depth_a> <depth_b>
  ```
- Executes a series of head-to-head matches between two versions of the engine at different depths.
- Alternates colors (White vs Black) to guarantee fairness.
- Calculates rating difference (Elo) from wins, draws, and losses using the standard formula:
  $$\Delta \text{Elo} = -400 \log_{10}(1 / \text{score} - 1)$$
  where $\text{score} = (\text{wins} + \text{draws} \times 0.5) / \text{total}$.
- Caps boundary Elo calculations (+/- 999.0 for 100% or 0% wins).
- Persists final match stats and Elo estimates to `gauntlet_results.json`.

## Verification

Tests cover:
- Verification of parameters update after SPSA tuning.
- Creation of `tuned_eval.json` after SPSA completes.
- Successful CLI parsing and run of `gauntlet 10 2 3` matches.
- Correct calculations of score percentages and Elo differentials.
