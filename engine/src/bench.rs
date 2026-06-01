use std::time::Instant;

use crate::board::{Board, STARTPOS_FEN};
use crate::search::{SearchControl, SearchLimits, iterative_deepening};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BenchRow {
    pub name: &'static str,
    pub depth: u8,
    pub best_move: String,
    pub score: i32,
    pub nodes: u64,
    pub elapsed_ms: u128,
    pub nps: u128,
    pub hashfull_permill: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BenchReport {
    pub rows: Vec<BenchRow>,
    pub total_nodes: u64,
    pub total_elapsed_ms: u128,
}

impl BenchReport {
    pub fn as_lines(&self) -> Vec<String> {
        let mut lines = vec!["Axiorynth benchmark".to_string()];
        for row in &self.rows {
            lines.push(format!(
                "{} depth {} best {} score {:+} nodes {} nps {} hashfull {} time {}ms",
                row.name,
                row.depth,
                row.best_move,
                row.score,
                row.nodes,
                row.nps,
                row.hashfull_permill,
                row.elapsed_ms
            ));
        }
        lines.push(format!("total nodes: {}", self.total_nodes));
        lines.push(format!("total time: {}ms", self.total_elapsed_ms));
        lines
    }
}

pub fn run_bench(depth: u8) -> Result<BenchReport, String> {
    let depth = depth.max(1);
    let positions = [
        ("startpos", STARTPOS_FEN),
        (
            "kiwipete",
            "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
        ),
        (
            "tactical",
            "r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10",
        ),
    ];

    let mut rows = Vec::new();
    let mut total_nodes = 0;
    let started_all = Instant::now();

    for (name, fen) in positions {
        let mut board = Board::from_fen(fen).map_err(|err| err.to_string())?;
        let control = SearchControl::new();
        let started = Instant::now();
        let result = iterative_deepening(
            &mut board,
            SearchLimits {
                max_depth: depth,
                hash_size_mb: 8,
                ..SearchLimits::default()
            },
            &control,
        );
        let elapsed = started.elapsed();
        let nodes = result.stats.nodes + result.stats.qnodes;
        total_nodes += nodes;
        let elapsed_ms = elapsed.as_millis().max(1);
        let best_move = result
            .best_move
            .map(|mv| mv.uci())
            .unwrap_or_else(|| "0000".to_string());

        rows.push(BenchRow {
            name,
            depth: result.depth,
            best_move,
            score: result.score,
            nodes,
            elapsed_ms,
            nps: nodes as u128 * 1_000 / elapsed_ms,
            hashfull_permill: result.stats.hashfull_permill,
        });
    }

    Ok(BenchReport {
        rows,
        total_nodes,
        total_elapsed_ms: started_all.elapsed().as_millis(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bench_runs_all_positions() {
        let report = run_bench(1).unwrap();
        assert_eq!(report.rows.len(), 3);
        assert!(report.total_nodes > 0);
    }
}
