# Axiorynth - Phase 19 NNUE Pipeline

## Scope

This documents completion of Sub-Phase 5 (Phase E: Research Program - NNUE Pipeline).

It introduces a fully functional Neural Network Update Efficiently (NNUE) evaluation system in Rust, featuring network initialization, weight persistence, live inference, training data generation, and a backpropagation training loop.

## Architecture

Implemented in:

```text
engine/src/nnue.rs
```

The neural network is custom-designed for fast evaluation:
- **Input Layer**: Sparse HalfKP features of size 40,960 (per perspective).
- **Hidden Layer 1 (Accumulator)**: Size 256. White and Black accumulators are activated via Clipped ReLU (clamped to $[0.0, 1.0]$) and concatenated to size 512.
- **Hidden Layer 2**: Size 32. Activated via Clipped ReLU.
- **Output Layer**: Size 1 (scalar evaluation in centipawns).

## Inference

Implemented in:

```text
engine/src/eval.rs
```

Features:
- Global `static` reference to the active NNUE network.
- `load_nnue` and `unload_nnue` control memory management thread-safely via a read-write lock (`RwLock`).
- `evaluate_side_to_move` automatically probes the NNUE network if weights are loaded. If no weights are present, it falls back to the handcrafted evaluation system.

## Data Generation CLI

Implemented in:

```text
engine/src/bin/axiorynth.rs
```

Command:
```text
axiorynth nnue-gen <games> <depth>
```
- Performs self-play games.
- At each position, it runs an alpha-beta search to the specified depth to get the evaluation score.
- Records the position FEN and the search score (clamped to $[-2000, 2000]$ to avoid mate score inflation).
- Outputs the training pairs to `nnue_data.txt` in format `FEN|score`.

## Training Pipeline CLI

Implemented in:

```text
engine/src/bin/axiorynth.rs
```

Command:
```text
axiorynth nnue-train <data-file> <epochs>
```
- Loads the dataset and parses FENs into active HalfKP features.
- Implements a backpropagation training loop.
- Uses mini-batch Gradient Descent (size 32) with learning rate decay and gradient clipping.
- Performs deterministic shuffling of training pairs at each epoch.
- Prints average training loss at each epoch.
- Saves the trained weights to `axiorynth.nnue`.

## Verification

Tests cover:
- Network forward pass producing valid non-NaN outputs.
- Weight file saving and exact restoration.
- Training loss reduction on dummy batches.
- Correct loading and evaluation fallback mechanisms.
