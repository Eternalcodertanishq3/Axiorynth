//! Axiorynth engine core.
//!
//! Phase 1 focuses on the parts that must be correct before strength work
//! begins: board state, FEN, legal move generation, make/undo, perft, and a
//! first numeric evaluator.

pub mod analysis;
pub mod bench;
pub mod board;
pub mod bot;
pub mod eval;
pub mod game;
pub mod memory;
pub mod movegen;
pub mod mv;
pub mod perft;
pub mod research;
pub mod search;
pub mod training;
pub mod types;
pub mod uci;
pub mod zobrist;

pub use analysis::{AnalysisReport, analyze_position};
pub use bench::{BenchReport, BenchRow, run_bench};
pub use board::{Board, BoardError, STARTPOS_FEN};
pub use bot::{BotLevel, BotMove, BotProfile, choose_bot_move, choose_bot_move_with_callback};
pub use eval::{EvalBreakdown, EvalConfig, evaluate, evaluate_side_to_move, load_nnue, unload_nnue, evaluate_nnue};
pub use game::{Game, GameRecord, GameResult};
pub use memory::{PlayerMemory, PlayerResult};
pub use movegen::{find_legal_move_by_uci, generate_legal_moves, generate_pseudo_legal_moves};
pub use mv::{Move, MoveKind};
pub use perft::{divide, perft};
pub use research::{ResearchMilestone, ResearchRoadmap, TuningParameter, research_roadmap};
pub use search::{
    CandidateMove, MATE_SCORE, SearchConfig, SearchControl, SearchLimits, SearchResult,
    SearchStats, best_move, best_move_with_control, iterative_deepening,
    iterative_deepening_with_callback,
};
pub use training::{TrainingGameRow, TrainingReport, build_training_report};
pub use types::{Color, Piece, PieceKind, Square};
pub use uci::{GoCommand, UciOptions, run_uci_stdio};

pub mod book;
pub use book::{OpeningBook, BookEntry};

pub mod nnue;
pub use nnue::NnueNetwork;

pub mod tablebase;
pub use tablebase::{probe_tablebase, probe_root_tablebase, WdlResult};

