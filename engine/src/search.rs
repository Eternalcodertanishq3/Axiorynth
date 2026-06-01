use std::mem;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};

use crate::board::Board;
use crate::eval::evaluate_side_to_move;
use crate::movegen::generate_legal_moves;
use crate::mv::{Move, MoveKind};
use crate::types::{Piece, PieceKind, Square};

pub const MATE_SCORE: i32 = 30_000;
const INF: i32 = 32_000;
const MAX_PLY: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SearchConfig {
    pub max_depth: u8,
    pub quiescence_depth: u8,
    pub candidate_count: usize,
}

impl Default for SearchConfig {
    fn default() -> Self {
        SearchConfig {
            max_depth: 3,
            quiescence_depth: 4,
            candidate_count: 5,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SearchLimits {
    pub max_depth: u8,
    pub quiescence_depth: u8,
    pub candidate_count: usize,
    pub move_time: Option<Duration>,
    pub node_limit: Option<u64>,
    pub hash_size_mb: usize,
}

impl Default for SearchLimits {
    fn default() -> Self {
        SearchLimits {
            max_depth: 4,
            quiescence_depth: 4,
            candidate_count: 5,
            move_time: None,
            node_limit: None,
            hash_size_mb: 8,
        }
    }
}

impl From<SearchConfig> for SearchLimits {
    fn from(config: SearchConfig) -> Self {
        SearchLimits {
            max_depth: config.max_depth,
            quiescence_depth: config.quiescence_depth,
            candidate_count: config.candidate_count,
            move_time: None,
            node_limit: None,
            hash_size_mb: 1,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SearchControl {
    stopped: Arc<AtomicBool>,
}

impl SearchControl {
    pub fn new() -> SearchControl {
        SearchControl {
            stopped: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn request_stop(&self) {
        self.stopped.store(true, Ordering::Relaxed);
    }

    pub fn reset(&self) {
        self.stopped.store(false, Ordering::Relaxed);
    }

    pub fn is_stopped(&self) -> bool {
        self.stopped.load(Ordering::Relaxed)
    }
}

impl Default for SearchControl {
    fn default() -> Self {
        SearchControl::new()
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SearchStats {
    pub nodes: u64,
    pub qnodes: u64,
    pub beta_cutoffs: u64,
    pub q_beta_cutoffs: u64,
    pub tt_hits: u64,
    pub tt_stores: u64,
    pub killer_uses: u64,
    pub max_ply: u8,
    pub hashfull_permill: u16,
    pub stopped: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateMove {
    pub mv: Move,
    pub score: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchResult {
    pub best_move: Option<Move>,
    pub score: i32,
    pub depth: u8,
    pub stats: SearchStats,
    pub principal_variation: Vec<Move>,
    pub candidates: Vec<CandidateMove>,
}

impl SearchResult {
    pub fn as_math_lines(&self) -> Vec<String> {
        let best = self
            .best_move
            .map(|mv| mv.uci())
            .unwrap_or_else(|| "(none)".to_string());
        let pv = if self.principal_variation.is_empty() {
            "(none)".to_string()
        } else {
            self.principal_variation
                .iter()
                .map(|mv| mv.uci())
                .collect::<Vec<_>>()
                .join(" ")
        };

        let mut lines = vec![
            format!("search depth: {}", self.depth),
            format!("best move: {best}"),
            format!("score: {:+} centipawns", self.score),
            format!("principal variation: {pv}"),
            format!("main nodes: {}", self.stats.nodes),
            format!("quiescence nodes: {}", self.stats.qnodes),
            format!(
                "beta cutoffs: {} main, {} quiescence",
                self.stats.beta_cutoffs, self.stats.q_beta_cutoffs
            ),
            format!(
                "transposition table: {} hits, {} stores, hashfull {} permill",
                self.stats.tt_hits, self.stats.tt_stores, self.stats.hashfull_permill
            ),
            format!("killer move uses: {}", self.stats.killer_uses),
            format!("stopped: {}", self.stats.stopped),
        ];

        for (idx, candidate) in self.candidates.iter().enumerate() {
            lines.push(format!(
                "candidate {}: {} = {:+}",
                idx + 1,
                candidate.mv,
                candidate.score
            ));
        }

        lines
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Bound {
    Exact,
    Lower,
    Upper,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TtEntry {
    key: u64,
    depth: u8,
    score: i32,
    bound: Bound,
    best_move: Option<Move>,
}

#[derive(Debug, Clone)]
struct TranspositionTable {
    entries: Vec<Option<TtEntry>>,
}

impl TranspositionTable {
    fn new(size_mb: usize) -> TranspositionTable {
        let bytes = size_mb.max(1) * 1024 * 1024;
        let entry_size = mem::size_of::<Option<TtEntry>>().max(1);
        let entry_count = (bytes / entry_size).max(1);
        TranspositionTable {
            entries: vec![None; entry_count],
        }
    }

    fn probe(&self, key: u64) -> Option<TtEntry> {
        let entry = self.entries[self.index(key)]?;
        (entry.key == key).then_some(entry)
    }

    fn store(&mut self, entry: TtEntry) -> bool {
        let index = self.index(entry.key);
        let replace = self.entries[index].is_none_or(|old| {
            old.key != entry.key || entry.depth >= old.depth || entry.bound == Bound::Exact
        });
        if replace {
            self.entries[index] = Some(entry);
        }
        replace
    }

    fn hashfull_permill(&self) -> u16 {
        let sample = self.entries.len().min(1000);
        if sample == 0 {
            return 0;
        }
        let used = self
            .entries
            .iter()
            .take(sample)
            .filter(|entry| entry.is_some())
            .count();
        ((used * 1000) / sample) as u16
    }

    fn index(&self, key: u64) -> usize {
        key as usize % self.entries.len()
    }
}

#[derive(Clone)]
struct SearchHeuristics {
    killers: [[Option<Move>; 2]; MAX_PLY],
    history: [[i32; 64]; 64],
}

impl Default for SearchHeuristics {
    fn default() -> Self {
        SearchHeuristics {
            killers: [[None; 2]; MAX_PLY],
            history: [[0; 64]; 64],
        }
    }
}

impl SearchHeuristics {
    fn killer_score(&self, ply: u8, mv: Move) -> Option<i32> {
        let ply = ply as usize;
        if ply >= MAX_PLY {
            return None;
        }
        if self.killers[ply][0] == Some(mv) {
            Some(80_000)
        } else if self.killers[ply][1] == Some(mv) {
            Some(70_000)
        } else {
            None
        }
    }

    fn record_killer(&mut self, ply: u8, mv: Move) {
        let ply = ply as usize;
        if ply >= MAX_PLY || self.killers[ply][0] == Some(mv) {
            return;
        }
        self.killers[ply][1] = self.killers[ply][0];
        self.killers[ply][0] = Some(mv);
    }

    fn history_score(&self, mv: Move) -> i32 {
        self.history[mv.from.index()][mv.to.index()]
    }

    fn record_history(&mut self, mv: Move, depth: u8) {
        let bonus = (depth as i32 + 1).pow(2);
        let score = &mut self.history[mv.from.index()][mv.to.index()];
        *score = (*score + bonus).min(100_000);
    }
}

pub fn best_move(board: &mut Board, config: SearchConfig) -> SearchResult {
    let control = SearchControl::new();
    best_move_with_control(board, config, &control)
}

pub fn best_move_with_control(
    board: &mut Board,
    config: SearchConfig,
    control: &SearchControl,
) -> SearchResult {
    let mut tt = TranspositionTable::new(1);
    let mut heuristics = SearchHeuristics::default();
    search_fixed_depth(board, config, control, None, None, &mut tt, &mut heuristics)
}

pub fn iterative_deepening(
    board: &mut Board,
    limits: SearchLimits,
    control: &SearchControl,
) -> SearchResult {
    iterative_deepening_internal(board, limits, control, None)
}

pub fn iterative_deepening_with_callback<F>(
    board: &mut Board,
    limits: SearchLimits,
    control: &SearchControl,
    mut on_depth: F,
) -> SearchResult
where
    F: FnMut(&SearchResult, Duration),
{
    iterative_deepening_internal(board, limits, control, Some(&mut on_depth))
}

fn iterative_deepening_internal(
    board: &mut Board,
    limits: SearchLimits,
    control: &SearchControl,
    mut on_depth: Option<&mut dyn FnMut(&SearchResult, Duration)>,
) -> SearchResult {
    let started_at = Instant::now();
    let deadline = limits.move_time.map(|move_time| started_at + move_time);
    let max_depth = limits.max_depth.max(1);
    let mut best_complete: Option<SearchResult> = None;
    let mut tt = TranspositionTable::new(limits.hash_size_mb);
    let mut heuristics = SearchHeuristics::default();

    for depth in 1..=max_depth {
        if control.is_stopped() || deadline.is_some_and(|value| Instant::now() >= value) {
            break;
        }

        let result = search_fixed_depth(
            board,
            SearchConfig {
                max_depth: depth,
                quiescence_depth: limits.quiescence_depth,
                candidate_count: limits.candidate_count,
            },
            control,
            deadline,
            limits.node_limit,
            &mut tt,
            &mut heuristics,
        );

        let stopped = result.stats.stopped;
        if !stopped {
            if let Some(callback) = on_depth.as_mut() {
                callback(&result, started_at.elapsed());
            }
            best_complete = Some(result);
        } else {
            if best_complete.is_none() {
                best_complete = Some(result);
            }
            break;
        }
    }

    best_complete.unwrap_or_else(|| {
        search_fixed_depth(
            board,
            SearchConfig {
                max_depth: 1,
                quiescence_depth: limits.quiescence_depth,
                candidate_count: limits.candidate_count,
            },
            control,
            deadline,
            limits.node_limit,
            &mut tt,
            &mut heuristics,
        )
    })
}

fn search_fixed_depth(
    board: &mut Board,
    config: SearchConfig,
    control: &SearchControl,
    deadline: Option<Instant>,
    node_limit: Option<u64>,
    tt: &mut TranspositionTable,
    heuristics: &mut SearchHeuristics,
) -> SearchResult {
    let depth = config.max_depth.max(1);
    let mut stats = SearchStats::default();
    let ctx = SearchContext {
        deadline,
        node_limit,
        control: control.clone(),
    };
    let tt_move = tt.probe(board.hash()).and_then(|entry| entry.best_move);
    let mut moves = ordered_legal_moves(board, tt_move, heuristics, 0, &mut stats);

    if moves.is_empty() {
        let score = terminal_score(board, 0);
        stats.hashfull_permill = tt.hashfull_permill();
        return SearchResult {
            best_move: None,
            score,
            depth,
            stats,
            principal_variation: Vec::new(),
            candidates: Vec::new(),
        };
    }

    let mut candidates = Vec::with_capacity(moves.len());
    let mut alpha = -INF;
    let beta = INF;
    let mut best = moves[0];
    let mut best_score = -INF;

    for mv in moves.drain(..) {
        if ctx.should_stop(&stats) {
            stats.stopped = true;
            break;
        }

        let undo = board.make_move(mv);
        let score = -negamax(
            board,
            depth - 1,
            -beta,
            -alpha,
            1,
            config.quiescence_depth,
            &ctx,
            tt,
            heuristics,
            &mut stats,
        );
        board.undo_move(undo);

        candidates.push(CandidateMove { mv, score });
        if score > best_score || (score == best_score && mv.uci() < best.uci()) {
            best = mv;
            best_score = score;
        }
        alpha = alpha.max(score);

        if stats.stopped {
            break;
        }
    }

    if candidates.is_empty() {
        stats.hashfull_permill = tt.hashfull_permill();
        return SearchResult {
            best_move: Some(best),
            score: evaluate_side_to_move(board),
            depth,
            stats,
            principal_variation: vec![best],
            candidates,
        };
    }

    let stored = tt.store(TtEntry {
        key: board.hash(),
        depth,
        score: best_score,
        bound: Bound::Exact,
        best_move: Some(best),
    });
    if stored {
        stats.tt_stores += 1;
    }

    candidates.sort_by(|a, b| {
        b.score
            .cmp(&a.score)
            .then_with(|| a.mv.uci().cmp(&b.mv.uci()))
    });
    candidates.truncate(config.candidate_count.max(1));

    let principal_variation = if stats.stopped {
        vec![best]
    } else {
        build_principal_variation(board, best, depth, tt)
    };
    stats.hashfull_permill = tt.hashfull_permill();

    SearchResult {
        best_move: Some(best),
        score: best_score,
        depth,
        stats,
        principal_variation,
        candidates,
    }
}

#[derive(Debug, Clone)]
struct SearchContext {
    deadline: Option<Instant>,
    node_limit: Option<u64>,
    control: SearchControl,
}

impl SearchContext {
    fn should_stop(&self, stats: &SearchStats) -> bool {
        self.control.is_stopped()
            || self
                .deadline
                .is_some_and(|deadline| Instant::now() >= deadline)
            || self
                .node_limit
                .is_some_and(|limit| stats.nodes + stats.qnodes >= limit)
    }
}

#[allow(clippy::too_many_arguments)]
fn negamax(
    board: &mut Board,
    depth: u8,
    mut alpha: i32,
    beta: i32,
    ply: u8,
    quiescence_depth: u8,
    ctx: &SearchContext,
    tt: &mut TranspositionTable,
    heuristics: &mut SearchHeuristics,
    stats: &mut SearchStats,
) -> i32 {
    stats.nodes += 1;
    stats.max_ply = stats.max_ply.max(ply);
    if ctx.should_stop(stats) {
        stats.stopped = true;
        return evaluate_side_to_move(board);
    }

    if depth == 0 {
        return quiescence(
            board,
            alpha,
            beta,
            quiescence_depth,
            ply,
            ctx,
            heuristics,
            stats,
        );
    }

    let original_alpha = alpha;
    let key = board.hash();
    let tt_entry = tt.probe(key);
    if let Some(entry) = tt_entry {
        if entry.depth >= depth {
            stats.tt_hits += 1;
            match entry.bound {
                Bound::Exact => return entry.score,
                Bound::Lower if entry.score >= beta => return entry.score,
                Bound::Upper if entry.score <= alpha => return entry.score,
                _ => {}
            }
        }
    }

    let tt_move = tt_entry.and_then(|entry| entry.best_move);
    let moves = ordered_legal_moves(board, tt_move, heuristics, ply, stats);
    if moves.is_empty() {
        return terminal_score(board, ply);
    }

    let mut best = -INF;
    let mut best_move = None;
    for mv in moves {
        let undo = board.make_move(mv);
        let score = -negamax(
            board,
            depth - 1,
            -beta,
            -alpha,
            ply + 1,
            quiescence_depth,
            ctx,
            tt,
            heuristics,
            stats,
        );
        board.undo_move(undo);

        if stats.stopped {
            return score;
        }

        if score > best {
            best = score;
            best_move = Some(mv);
        }

        if score >= beta {
            stats.beta_cutoffs += 1;
            if !mv.is_capture() {
                heuristics.record_killer(ply, mv);
                heuristics.record_history(mv, depth);
            }
            if tt.store(TtEntry {
                key,
                depth,
                score,
                bound: Bound::Lower,
                best_move: Some(mv),
            }) {
                stats.tt_stores += 1;
            }
            return beta;
        }
        alpha = alpha.max(score);
    }

    let bound = if best <= original_alpha {
        Bound::Upper
    } else {
        Bound::Exact
    };
    if tt.store(TtEntry {
        key,
        depth,
        score: best,
        bound,
        best_move,
    }) {
        stats.tt_stores += 1;
    }

    best
}

#[allow(clippy::too_many_arguments)]
fn quiescence(
    board: &mut Board,
    mut alpha: i32,
    beta: i32,
    depth: u8,
    ply: u8,
    ctx: &SearchContext,
    heuristics: &SearchHeuristics,
    stats: &mut SearchStats,
) -> i32 {
    stats.qnodes += 1;
    stats.max_ply = stats.max_ply.max(ply);
    if ctx.should_stop(stats) {
        stats.stopped = true;
        return evaluate_side_to_move(board);
    }

    let in_check = board.in_check(board.side_to_move());
    if !in_check {
        let stand_pat = evaluate_side_to_move(board);
        if depth == 0 {
            return stand_pat;
        }
        if stand_pat >= beta {
            stats.q_beta_cutoffs += 1;
            return beta;
        }
        alpha = alpha.max(stand_pat);
    } else if depth == 0 {
        return evaluate_side_to_move(board);
    }

    let moves = ordered_legal_moves(board, None, heuristics, ply, stats);
    if moves.is_empty() {
        return terminal_score(board, ply);
    }

    for mv in moves.into_iter().filter(|mv| in_check || is_noisy(*mv)) {
        let undo = board.make_move(mv);
        let score = -quiescence(
            board,
            -beta,
            -alpha,
            depth - 1,
            ply + 1,
            ctx,
            heuristics,
            stats,
        );
        board.undo_move(undo);

        if stats.stopped {
            return score;
        }

        if score >= beta {
            stats.q_beta_cutoffs += 1;
            return beta;
        }
        alpha = alpha.max(score);
    }

    alpha
}

fn build_principal_variation(
    board: &mut Board,
    first: Move,
    depth: u8,
    tt: &TranspositionTable,
) -> Vec<Move> {
    let mut pv = vec![first];
    let mut undos = vec![board.make_move(first)];

    for _ in 1..depth {
        let Some(entry) = tt.probe(board.hash()) else {
            break;
        };
        let Some(next) = entry.best_move else {
            break;
        };
        let legal_moves = generate_legal_moves(board);
        if !legal_moves.contains(&next) {
            break;
        }
        pv.push(next);
        undos.push(board.make_move(next));
    }

    while let Some(undo) = undos.pop() {
        board.undo_move(undo);
    }

    pv
}

fn ordered_legal_moves(
    board: &mut Board,
    tt_move: Option<Move>,
    heuristics: &SearchHeuristics,
    ply: u8,
    stats: &mut SearchStats,
) -> Vec<Move> {
    let mut scored = generate_legal_moves(board)
        .into_iter()
        .map(|mv| {
            (
                move_order_score(board, mv, tt_move, heuristics, ply, stats),
                mv,
            )
        })
        .collect::<Vec<_>>();
    scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.uci().cmp(&b.1.uci())));
    scored.into_iter().map(|(_, mv)| mv).collect()
}

fn move_order_score(
    board: &Board,
    mv: Move,
    tt_move: Option<Move>,
    heuristics: &SearchHeuristics,
    ply: u8,
    stats: &mut SearchStats,
) -> i32 {
    if tt_move == Some(mv) {
        return 1_000_000;
    }

    let attacker = board.piece_at(mv.from);
    let mut score = 0;

    if let Some(promotion) = mv.promotion {
        score += 800_000 + promotion.material_value();
    }

    if mv.is_capture() {
        let victim = capture_victim(board, mv);
        score += 500_000 + mvv_lva(attacker, victim);
    } else if let Some(killer_score) = heuristics.killer_score(ply, mv) {
        stats.killer_uses += 1;
        score += killer_score;
    } else {
        score += heuristics.history_score(mv);
    }

    if matches!(mv.kind, MoveKind::KingCastle | MoveKind::QueenCastle) {
        score += 50;
    }

    if is_center(mv.to) {
        score += 20;
    }

    score
}

fn capture_victim(board: &Board, mv: Move) -> Option<Piece> {
    if mv.kind == MoveKind::EnPassant {
        return Some(Piece {
            color: board.side_to_move().opposite(),
            kind: PieceKind::Pawn,
        });
    }

    board.piece_at(mv.to)
}

fn mvv_lva(attacker: Option<Piece>, victim: Option<Piece>) -> i32 {
    let victim_value = victim.map_or(0, |piece| piece.kind.material_value());
    let attacker_value = attacker.map_or(0, |piece| piece.kind.material_value());
    victim_value * 10 - attacker_value
}

fn is_center(square: Square) -> bool {
    matches!(
        (square.file(), square.rank()),
        (3, 3) | (4, 3) | (3, 4) | (4, 4)
    )
}

fn is_noisy(mv: Move) -> bool {
    mv.is_capture() || mv.promotion.is_some()
}

fn terminal_score(board: &Board, ply: u8) -> i32 {
    if board.in_check(board.side_to_move()) {
        -MATE_SCORE + ply as i32
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn best_move_finds_simple_mate_in_one() {
        let mut board = Board::from_fen("7k/8/5K2/8/8/6Q1/8/8 w - - 0 1").unwrap();
        let result = best_move(
            &mut board,
            SearchConfig {
                max_depth: 2,
                quiescence_depth: 2,
                candidate_count: 3,
            },
        );

        assert_eq!(
            result.best_move.map(|mv| mv.uci()),
            Some("g3g7".to_string())
        );
        assert!(result.score > MATE_SCORE - 10);
    }

    #[test]
    fn search_does_not_mutate_board() {
        let mut board = Board::startpos().unwrap();
        let before = board.to_fen();
        let before_hash = board.hash();
        let _ = best_move(
            &mut board,
            SearchConfig {
                max_depth: 2,
                quiescence_depth: 1,
                candidate_count: 3,
            },
        );
        assert_eq!(board.to_fen(), before);
        assert_eq!(board.hash(), before_hash);
        assert_eq!(board.hash(), board.compute_hash());
    }

    #[test]
    fn search_reports_candidate_moves() {
        let mut board = Board::startpos().unwrap();
        let result = best_move(
            &mut board,
            SearchConfig {
                max_depth: 1,
                quiescence_depth: 1,
                candidate_count: 4,
            },
        );

        assert!(result.best_move.is_some());
        assert_eq!(result.candidates.len(), 4);
        assert!(result.stats.qnodes > 0);
    }

    #[test]
    fn iterative_search_returns_at_least_depth_one() {
        let mut board = Board::startpos().unwrap();
        let control = SearchControl::new();
        let result = iterative_deepening(
            &mut board,
            SearchLimits {
                max_depth: 3,
                move_time: Some(Duration::from_millis(1)),
                ..SearchLimits::default()
            },
            &control,
        );

        assert!(result.best_move.is_some());
        assert!(result.depth >= 1);
    }

    #[test]
    fn iterative_search_uses_transposition_table() {
        let mut board = Board::startpos().unwrap();
        let control = SearchControl::new();
        let result = iterative_deepening(
            &mut board,
            SearchLimits {
                max_depth: 3,
                hash_size_mb: 1,
                ..SearchLimits::default()
            },
            &control,
        );

        assert!(result.stats.tt_stores > 0);
        assert!(result.stats.hashfull_permill > 0);
    }

    #[test]
    fn iterative_search_can_report_each_completed_depth() {
        let mut board = Board::startpos().unwrap();
        let control = SearchControl::new();
        let mut depths = Vec::new();
        let result = iterative_deepening_with_callback(
            &mut board,
            SearchLimits {
                max_depth: 3,
                hash_size_mb: 1,
                ..SearchLimits::default()
            },
            &control,
            |depth_result, _elapsed| depths.push(depth_result.depth),
        );

        assert_eq!(result.depth, 3);
        assert_eq!(depths, vec![1, 2, 3]);
    }

    #[test]
    fn search_control_can_stop_a_search() {
        let mut board = Board::startpos().unwrap();
        let control = SearchControl::new();
        control.request_stop();
        let result = best_move_with_control(
            &mut board,
            SearchConfig {
                max_depth: 4,
                quiescence_depth: 2,
                candidate_count: 3,
            },
            &control,
        );

        assert!(result.stats.stopped);
    }

    #[test]
    fn search_prefers_winning_a_hanging_queen() {
        let mut board = Board::from_fen("4k3/8/8/8/8/8/4q3/4K2R w - - 0 1").unwrap();
        let result = best_move(
            &mut board,
            SearchConfig {
                max_depth: 1,
                quiescence_depth: 2,
                candidate_count: 3,
            },
        );

        assert_eq!(
            result.best_move.map(|mv| mv.uci()),
            Some("e1e2".to_string())
        );
        assert!(result.score > 500);
    }
}
