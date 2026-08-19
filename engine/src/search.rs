use std::mem;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};

use crate::board::{Board, bit_squares, step_square, knight_targets, king_targets};
use crate::eval::evaluate_side_to_move;
use crate::movegen::generate_legal_moves;
use crate::mv::{Move, MoveKind};
use crate::types::{Color, Piece, PieceKind, Square};

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
    pub wtime: Option<u64>,
    pub btime: Option<u64>,
    pub winc: Option<u64>,
    pub binc: Option<u64>,
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
            wtime: None,
            btime: None,
            winc: None,
            binc: None,
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
            wtime: None,
            btime: None,
            winc: None,
            binc: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SearchControl {
    stopped: Arc<AtomicBool>,
}

pub struct TimeManager {
    started_at: Instant,
    soft_deadline: Option<Instant>,
    hard_deadline: Option<Instant>,
}

impl TimeManager {
    pub fn new(limits: &SearchLimits, color: Color) -> Self {
        let started_at = Instant::now();
        let mut soft_deadline = None;
        let mut hard_deadline = None;

        if let Some(move_time) = limits.move_time {
            soft_deadline = Some(started_at + move_time);
            hard_deadline = Some(started_at + move_time);
        } else {
            let (time_left, inc) = match color {
                Color::White => (limits.wtime, limits.winc),
                Color::Black => (limits.btime, limits.binc),
            };

            if let Some(ms) = time_left {
                let ms_inc = inc.unwrap_or(0);
                let soft_budget = (ms / 25) + (ms_inc / 2);
                let hard_budget = (ms / 5) + ms_inc;
                
                let safe_hard_budget = hard_budget.min(ms.saturating_sub(50));
                let safe_soft_budget = soft_budget.min(safe_hard_budget);

                if safe_hard_budget > 0 {
                    soft_deadline = Some(started_at + Duration::from_millis(safe_soft_budget));
                    hard_deadline = Some(started_at + Duration::from_millis(safe_hard_budget));
                }
            }
        }

        Self {
            started_at,
            soft_deadline,
            hard_deadline,
        }
    }

    pub fn should_stop_hard(&self) -> bool {
        self.hard_deadline.is_some_and(|d| Instant::now() >= d)
    }

    pub fn should_stop_soft(&self) -> bool {
        self.soft_deadline.is_some_and(|d| Instant::now() >= d)
    }

    pub fn time_spent(&self) -> Duration {
        Instant::now().duration_since(self.started_at)
    }
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

fn adjust_score_for_tt(score: i32, ply: u8) -> i32 {
    if score > MATE_SCORE - 1000 {
        score + ply as i32
    } else if score < -MATE_SCORE + 1000 {
        score - ply as i32
    } else {
        score
    }
}

fn adjust_score_from_tt(score: i32, ply: u8) -> i32 {
    if score > MATE_SCORE - 1000 {
        score - ply as i32
    } else if score < -MATE_SCORE + 1000 {
        score + ply as i32
    } else {
        score
    }
}

fn is_repetition(key: u64, history: &[u64], halfmove: u16) -> bool {
    let len = history.len();
    if len < 2 {
        return false;
    }
    let max_back = (halfmove as usize).min(len - 1);
    for i in 1..=max_back {
        if history[len - 1 - i] == key {
            return true;
        }
    }
    false
}

#[derive(Clone)]
struct SearchHeuristics {
    killers: [[Option<Move>; 2]; MAX_PLY],
    history: [[i32; 64]; 64],
    countermoves: [[Option<Move>; 64]; 64],
    prev_move: Option<Move>,
}

impl Default for SearchHeuristics {
    fn default() -> Self {
        SearchHeuristics {
            killers: [[None; 2]; MAX_PLY],
            history: [[0; 64]; 64],
            countermoves: [[None; 64]; 64],
            prev_move: None,
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

    fn countermove_score(&self, mv: Move) -> Option<i32> {
        let prev = self.prev_move?;
        if self.countermoves[prev.from.index()][prev.to.index()] == Some(mv) {
            Some(60_000)
        } else {
            None
        }
    }

    fn record_countermove(&mut self, mv: Move) {
        if let Some(prev) = self.prev_move {
            self.countermoves[prev.from.index()][prev.to.index()] = Some(mv);
        }
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
    search_fixed_depth(board, config, control, None, None, &mut tt, &mut heuristics, None, None)
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
    // Probe tablebase first for endgames
    if crate::tablebase::total_pieces(board) <= 7 {
        if let Some((best_move_uci, score)) = crate::tablebase::probe_root_tablebase(board) {
            if let Some(mv) = crate::movegen::find_legal_move_by_uci(board, &best_move_uci) {
                let stats = SearchStats::default();
                let res = SearchResult {
                    best_move: Some(mv),
                    score,
                    depth: 1,
                    stats,
                    principal_variation: vec![mv],
                    candidates: vec![CandidateMove { mv, score }],
                };
                if let Some(callback) = on_depth {
                    callback(&res, Duration::from_secs(0));
                }
                return res;
            }
        }
    }

    let time_manager = TimeManager::new(&limits, board.side_to_move());
    let max_depth = limits.max_depth.max(1);
    let mut best_complete: Option<SearchResult> = None;
    let mut tt = TranspositionTable::new(limits.hash_size_mb);
    let mut heuristics = SearchHeuristics::default();

    for depth in 1..=max_depth {
        if control.is_stopped() || time_manager.should_stop_soft() || time_manager.should_stop_hard() {
            break;
        }

        let score = best_complete.as_ref().map_or(0, |r| r.score);
        let mut delta = 25; 
        let mut alpha = score - delta;
        let mut beta = score + delta;
        
        let mut result_opt = None;
        loop {
            if alpha < -INF { alpha = -INF; }
            if beta > INF { beta = INF; }

            if time_manager.should_stop_hard() {
                break;
            }
            
            let current_result = search_fixed_depth(
                board,
                SearchConfig {
                    max_depth: depth,
                    quiescence_depth: limits.quiescence_depth,
                    candidate_count: limits.candidate_count,
                },
                control,
                time_manager.hard_deadline,
                limits.node_limit,
                &mut tt,
                &mut heuristics,
                Some(alpha),
                Some(beta),
            );
            
            let stopped = current_result.stats.stopped;
            if stopped {
                result_opt = Some(current_result);
                break;
            }
            
            if current_result.score <= alpha && alpha > -INF {
                alpha -= delta;
                delta *= 2;
            } else if current_result.score >= beta && beta < INF {
                beta += delta;
                delta *= 2;
            } else {
                result_opt = Some(current_result);
                break;
            }
        }

        if let Some(result) = result_opt {
            let stopped = result.stats.stopped;
            if !stopped {
                if let Some(callback) = on_depth.as_mut() {
                    callback(&result, time_manager.time_spent());
                }
                best_complete = Some(result);
            } else {
                if best_complete.is_none() {
                    best_complete = Some(result);
                }
                break;
            }
        } else {
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
            None,
            limits.node_limit,
            &mut tt,
            &mut heuristics,
            None,
            None,
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
    alpha_bound: Option<i32>,
    beta_bound: Option<i32>,
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
    let mut alpha = alpha_bound.unwrap_or(-INF);
    let beta = beta_bound.unwrap_or(INF);
    let mut best = moves[0];
    let mut best_score = -INF;
    let mut history = vec![board.hash()];

    for mv in moves.drain(..) {
        if ctx.should_stop(&stats) {
            stats.stopped = true;
            break;
        }

        let undo = board.make_move(mv);
        history.push(board.hash());
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
            &mut history,
            &mut stats,
        );
        history.pop();
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
    history: &mut Vec<u64>,
    stats: &mut SearchStats,
) -> i32 {
    let is_pv_node = beta - alpha > 1;
    stats.nodes += 1;
    stats.max_ply = stats.max_ply.max(ply);
    if ctx.should_stop(stats) {
        stats.stopped = true;
        return evaluate_side_to_move(board);
    }

    if ply > 0 {
        if board.halfmove_clock() >= 100 {
            return 0;
        }
        if is_repetition(board.hash(), history, board.halfmove_clock()) {
            return 0;
        }
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
            let tt_score = adjust_score_from_tt(entry.score, ply);
            match entry.bound {
                Bound::Exact => return tt_score,
                Bound::Lower if tt_score >= beta => return tt_score,
                Bound::Upper if tt_score <= alpha => return tt_score,
                _ => {}
            }
        }
    }

    let us = board.side_to_move();
    
    // Null-Move Pruning (NMP)
    let has_major_pieces = board.piece_count(us, PieceKind::Knight) > 0
        || board.piece_count(us, PieceKind::Bishop) > 0
        || board.piece_count(us, PieceKind::Rook) > 0
        || board.piece_count(us, PieceKind::Queen) > 0;
    
    let was_in_check = board.in_check(us);

    if depth >= 3 && !was_in_check && has_major_pieces {
        let r = 2;
        let null_depth = depth.saturating_sub(1 + r);
        let undo = board.make_null_move();
        let score = -negamax(
            board,
            null_depth,
            -beta,
            -beta + 1,
            ply + 1,
            quiescence_depth,
            ctx,
            tt,
            heuristics,
            history,
            stats,
        );
        board.undo_null_move(undo);
        if score >= beta {
            return beta;
        }
    }

    let tt_move = tt_entry.and_then(|entry| entry.best_move);

    // Singular extension: verify the TT move is truly singular
    let mut singular_extension = 0u8;
    if is_pv_node && depth >= 6 && !was_in_check {
        if let Some(entry) = tt_entry {
            if entry.depth >= depth - 3 && entry.bound != Bound::Upper {
                if let Some(tt_mv) = entry.best_move {
                    let se_beta = entry.score - (depth as i32 * 2);
                    let se_depth = (depth - 1) / 2;
                    let saved_prev = heuristics.prev_move;
                    let mut se_best = -INF;
                    let se_moves = ordered_legal_moves(board, Some(tt_mv), heuristics, ply, stats);
                    for smv in se_moves.into_iter().filter(|m| *m != tt_mv).take(6) {
                        let se_undo = board.make_move(smv);
                        history.push(board.hash());
                        let se_score = -negamax(
                            board, se_depth, -se_beta, -se_beta + 1, ply + 1,
                            quiescence_depth, ctx, tt, heuristics, history, stats,
                        );
                        history.pop();
                        board.undo_move(se_undo);
                        se_best = se_best.max(se_score);
                        if se_best >= se_beta { break; }
                    }
                    heuristics.prev_move = saved_prev;
                    if se_best < se_beta {
                        singular_extension = 1;
                    }
                }
            }
        }
    }

    let moves = ordered_legal_moves(board, tt_move, heuristics, ply, stats);
    if moves.is_empty() {
        return terminal_score(board, ply);
    }

    let mut best = -INF;
    let mut best_move = None;
    let static_eval = evaluate_side_to_move(board);
    let is_futility = depth <= 2 && !was_in_check;

    for (idx, mv) in moves.into_iter().enumerate() {
        if ctx.should_stop(stats) {
            stats.stopped = true;
            break;
        }

        // Futility Pruning
        if is_futility && idx > 0 && !is_noisy(mv) {
            let is_killer = heuristics.killer_score(ply, mv).is_some();
            if !is_killer {
                let margin = if depth == 1 { 150 } else { 300 };
                if static_eval + margin <= alpha {
                    continue;
                }
            }
        }

        let undo = board.make_move(mv);
        history.push(board.hash());

        // Check extension: extend by 1 if the move puts opponent in check
        let gives_check = board.in_check(board.side_to_move());
        let ext = if gives_check { 1u8 } else { 0 }
            + if tt_move == Some(mv) { singular_extension } else { 0 };
        let effective_depth = (depth - 1).saturating_add(ext);

        let mut score;
        let saved_prev = heuristics.prev_move;
        heuristics.prev_move = Some(mv);

        if idx == 0 {
            // PV move: search with full window
            score = -negamax(
                board,
                effective_depth,
                -beta,
                -alpha,
                ply + 1,
                quiescence_depth,
                ctx,
                tt,
                heuristics,
                history,
                stats,
            );
        } else {
            // Non-PV move: try LMR and/or null-window search
            let is_killer = heuristics.killer_score(ply, mv).is_some();
            let can_reduce = depth >= 3 
                && !gives_check
                && !was_in_check
                && !is_noisy(mv)
                && !is_killer;

            if can_reduce {
                let r = if idx > 4 { 2 } else { 1 };
                let reduced_depth = depth.saturating_sub(1 + r);
                score = -negamax(
                    board,
                    reduced_depth,
                    -alpha - 1,
                    -alpha,
                    ply + 1,
                    quiescence_depth,
                    ctx,
                    tt,
                    heuristics,
                    history,
                    stats,
                );
                
                if score > alpha {
                    score = -negamax(
                        board,
                        effective_depth,
                        -alpha - 1,
                        -alpha,
                        ply + 1,
                        quiescence_depth,
                        ctx,
                        tt,
                        heuristics,
                        history,
                        stats,
                    );
                }
            } else {
                score = -negamax(
                    board,
                    effective_depth,
                    -alpha - 1,
                    -alpha,
                    ply + 1,
                    quiescence_depth,
                    ctx,
                    tt,
                    heuristics,
                    history,
                    stats,
                );
            }

            if score > alpha && score < beta {
                score = -negamax(
                    board,
                    effective_depth,
                    -beta,
                    -alpha,
                    ply + 1,
                    quiescence_depth,
                    ctx,
                    tt,
                    heuristics,
                    history,
                    stats,
                );
            }
        }

        history.pop();
        board.undo_move(undo);
        heuristics.prev_move = saved_prev;

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
                heuristics.record_countermove(mv);
            }
            let score_to_store = adjust_score_for_tt(score, ply);
            if tt.store(TtEntry {
                key,
                depth,
                score: score_to_store,
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
    let score_to_store = adjust_score_for_tt(best, ply);
    if tt.store(TtEntry {
        key,
        depth,
        score: score_to_store,
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
    let mut moves = generate_legal_moves(board);
    let mut scored = Vec::with_capacity(moves.len());
    for mv in moves.drain(..) {
        let score = move_order_score(board, mv, tt_move, heuristics, ply, stats);
        scored.push((score, mv));
    }
    scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.uci().cmp(&b.1.uci())));
    scored.into_iter().map(|(_, mv)| mv).collect()
}

fn move_order_score(
    board: &mut Board,
    mv: Move,
    tt_move: Option<Move>,
    heuristics: &SearchHeuristics,
    ply: u8,
    stats: &mut SearchStats,
) -> i32 {
    if tt_move == Some(mv) {
        return 1_000_000;
    }

    let mut score = 0;

    if let Some(promotion) = mv.promotion {
        score += 800_000 + promotion.material_value();
    }

    if mv.is_capture() {
        let see_val = see(board, mv);
        if see_val >= 0 {
            score += 500_000 + see_val;
        } else {
            score += 100_000 + see_val;
        }
    } else if let Some(killer_score) = heuristics.killer_score(ply, mv) {
        stats.killer_uses += 1;
        score += killer_score;
    } else if let Some(cm_score) = heuristics.countermove_score(mv) {
        score += cm_score;
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

pub fn see(board: &mut Board, mv: Move) -> i32 {
    let victim = capture_victim(board, mv);
    let victim_val = victim.map_or(0, |p| p.kind.material_value());
    
    let undo = board.make_move(mv);
    let score = victim_val - see_search(board, mv.to);
    board.undo_move(undo);
    score
}

fn see_search(board: &mut Board, target: Square) -> i32 {
    let us = board.side_to_move();
    let Some(mv) = least_valuable_attacker_move(board, target, us) else {
        return 0;
    };
    
    let victim = board.piece_at(target);
    let victim_val = victim.map_or(0, |p| p.kind.material_value());
    
    let undo = board.make_move(mv);
    let score = std::cmp::max(0, victim_val - see_search(board, target));
    board.undo_move(undo);
    score
}

fn least_valuable_attacker_move(board: &Board, target: Square, color: Color) -> Option<Move> {
    let order = [
        PieceKind::Pawn,
        PieceKind::Knight,
        PieceKind::Bishop,
        PieceKind::Rook,
        PieceKind::Queen,
        PieceKind::King,
    ];
    
    for kind in order {
        for from in bit_squares(board.pieces(color, kind)) {
            if is_attacking(board, from, kind, target) {
                let is_capture = board.piece_at(target).is_some();
                let kind_mv = if is_capture { MoveKind::Capture } else { MoveKind::Quiet };
                let promotion = if kind == PieceKind::Pawn && (target.rank() == 7 || target.rank() == 0) {
                    Some(PieceKind::Queen)
                } else {
                    None
                };
                return Some(Move::new(from, target, promotion, kind_mv));
            }
        }
    }
    
    None
}

fn is_attacking(board: &Board, from: Square, kind: PieceKind, target: Square) -> bool {
    match kind {
        PieceKind::Pawn => {
            let color = board.piece_at(from).unwrap().color;
            let pawn_attack_ranks = match color {
                Color::White => from.rank() + 1 == target.rank(),
                Color::Black => from.rank() as i8 - 1 == target.rank() as i8,
            };
            pawn_attack_ranks && (from.file() as i8 - target.file() as i8).abs() == 1
        }
        PieceKind::Knight => {
            knight_targets(from) & target.bit() != 0
        }
        PieceKind::King => {
            king_targets(from) & target.bit() != 0
        }
        PieceKind::Bishop => {
            is_sliding_attack(board, from, target, &[(-1, -1), (-1, 1), (1, -1), (1, 1)])
        }
        PieceKind::Rook => {
            is_sliding_attack(board, from, target, &[(-1, 0), (1, 0), (0, -1), (0, 1)])
        }
        PieceKind::Queen => {
            is_sliding_attack(board, from, target, &[
                (-1, -1), (-1, 1), (1, -1), (1, 1),
                (-1, 0), (1, 0), (0, -1), (0, 1)
            ])
        }
    }
}

fn is_sliding_attack(board: &Board, from: Square, target: Square, directions: &[(i8, i8)]) -> bool {
    for &(df, dr) in directions {
        let mut current = from;
        while let Some(next) = step_square(current, df, dr) {
            if next == target {
                return true;
            }
            if board.piece_at(next).is_some() {
                break;
            }
            current = next;
        }
    }
    false
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
        assert!(result.stats.hashfull_permill <= 1000);
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

    #[test]
    fn test_tt_mate_distance_adjustment() {
        let ply = 5;
        let original_win = MATE_SCORE - 3; // Mate in 3
        let stored = adjust_score_for_tt(original_win, ply);
        assert_eq!(stored, original_win + ply as i32);
        let retrieved = adjust_score_from_tt(stored, ply);
        assert_eq!(retrieved, original_win);

        let original_loss = -MATE_SCORE + 4;
        let stored_loss = adjust_score_for_tt(original_loss, ply);
        assert_eq!(stored_loss, original_loss - ply as i32);
        let retrieved_loss = adjust_score_from_tt(stored_loss, ply);
        assert_eq!(retrieved_loss, original_loss);
    }

    #[test]
    fn test_search_detects_repetition() {
        let h1 = 123456789;
        let h2 = 987654321;
        let history = vec![h1, h2];
        // If the current position hash matches h1 within halfmove window, is_repetition returns true
        assert!(is_repetition(h1, &history, 4));
        assert!(!is_repetition(99999, &history, 4));
    }
}
