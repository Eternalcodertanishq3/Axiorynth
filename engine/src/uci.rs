use std::io::{self, BufRead, Write};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crate::board::{Board, STARTPOS_FEN};
use crate::movegen::find_legal_move_by_uci;
use crate::search::{
    MATE_SCORE, SearchControl, SearchLimits, SearchResult, SearchStats,
    iterative_deepening_with_callback,
};
use crate::types::Color;

const ENGINE_NAME: &str = "Axiorynth 0.3.0";
const ENGINE_AUTHOR: &str = "Axiorynth Project";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UciOptions {
    pub search_depth: u8,
    pub quiescence_depth: u8,
    pub candidate_count: usize,
    pub hash_size_mb: usize,
}

impl Default for UciOptions {
    fn default() -> Self {
        UciOptions {
            search_depth: 4,
            quiescence_depth: 4,
            candidate_count: 5,
            hash_size_mb: 16,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GoCommand {
    pub depth: Option<u8>,
    pub movetime: Option<Duration>,
    pub nodes: Option<u64>,
    pub wtime: Option<Duration>,
    pub btime: Option<Duration>,
    pub winc: Option<Duration>,
    pub binc: Option<Duration>,
    pub movestogo: Option<u32>,
    pub infinite: bool,
}

impl GoCommand {
    pub fn to_limits(self, _side_to_move: Color, options: UciOptions) -> SearchLimits {
        SearchLimits {
            max_depth: self
                .depth
                .unwrap_or(if self.infinite {
                    64
                } else {
                    options.search_depth
                })
                .max(1),
            quiescence_depth: options.quiescence_depth,
            candidate_count: options.candidate_count,
            move_time: self.movetime,
            node_limit: self.nodes,
            hash_size_mb: options.hash_size_mb,
            wtime: self.wtime.map(|d| d.as_millis() as u64),
            btime: self.btime.map(|d| d.as_millis() as u64),
            winc: self.winc.map(|d| d.as_millis() as u64),
            binc: self.binc.map(|d| d.as_millis() as u64),
        }
    }
}

pub fn run_uci_stdio() -> io::Result<()> {
    let stdin = io::stdin();
    let stdout = Arc::new(Mutex::new(io::stdout()));
    run_uci_loop(stdin.lock(), stdout)
}

fn run_uci_loop<R: BufRead>(reader: R, stdout: Arc<Mutex<io::Stdout>>) -> io::Result<()> {
    let mut board = Board::startpos().expect("start position must be valid");
    let mut options = UciOptions::default();
    let mut active: Option<ActiveSearch> = None;

    for line in reader.lines() {
        cleanup_finished_search(&mut active);

        let line = line?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let mut parts = trimmed.split_whitespace();
        let command = parts.next().unwrap_or_default();

        match command {
            "uci" => write_lines(
                &stdout,
                &[
                    format!("id name {ENGINE_NAME}"),
                    format!("id author {ENGINE_AUTHOR}"),
                    "option name SearchDepth type spin default 4 min 1 max 64".to_string(),
                    "option name QuiescenceDepth type spin default 4 min 0 max 16".to_string(),
                    "option name CandidateCount type spin default 5 min 1 max 20".to_string(),
                    "option name Hash type spin default 16 min 1 max 1024".to_string(),
                    "uciok".to_string(),
                ],
            )?,
            "isready" => write_lines(&stdout, &["readyok".to_string()])?,
            "ucinewgame" => {
                board = Board::startpos().expect("start position must be valid");
            }
            "position" => match parse_position(trimmed) {
                Ok(next_board) => board = next_board,
                Err(err) => write_lines(&stdout, &[format!("info string position error: {err}")])?,
            },
            "setoption" => {
                if let Err(err) = apply_setoption(trimmed, &mut options) {
                    write_lines(&stdout, &[format!("info string option error: {err}")])?;
                }
            }
            "go" => {
                stop_active_search(&mut active);
                let go = parse_go(trimmed);
                let limits = go.to_limits(board.side_to_move(), options);
                let search_board = board.clone();
                active = Some(start_search(search_board, limits, stdout.clone()));
            }
            "stop" => stop_active_search(&mut active),
            "quit" => {
                stop_active_search(&mut active);
                break;
            }
            "d" => write_lines(&stdout, &[format!("Fen: {}", board.to_fen())])?,
            _ => write_lines(
                &stdout,
                &[format!("info string unknown command: {command}")],
            )?,
        }
    }

    stop_active_search(&mut active);
    Ok(())
}

struct ActiveSearch {
    control: SearchControl,
    handle: JoinHandle<()>,
}

fn start_search(
    mut board: Board,
    limits: SearchLimits,
    stdout: Arc<Mutex<io::Stdout>>,
) -> ActiveSearch {
    let control = SearchControl::new();
    let thread_control = control.clone();
    let handle = thread::spawn(move || {
        let started_at = Instant::now();
        let info_stdout = stdout.clone();
        let result =
            iterative_deepening_with_callback(&mut board, limits, &thread_control, |result, _| {
                let elapsed = started_at.elapsed();
                let _ = write_lines(&info_stdout, &[uci_info_line(result, elapsed)]);
            });
        let _ = write_lines(&stdout, &[uci_bestmove_line(&result)]);
    });

    ActiveSearch { control, handle }
}

fn cleanup_finished_search(active: &mut Option<ActiveSearch>) {
    if active
        .as_ref()
        .is_some_and(|search| search.handle.is_finished())
    {
        if let Some(search) = active.take() {
            let _ = search.handle.join();
        }
    }
}

fn stop_active_search(active: &mut Option<ActiveSearch>) {
    if let Some(search) = active.take() {
        search.control.request_stop();
        let _ = search.handle.join();
    }
}

fn write_lines(stdout: &Arc<Mutex<io::Stdout>>, lines: &[String]) -> io::Result<()> {
    let mut out = stdout.lock().expect("stdout lock must not be poisoned");
    for line in lines {
        writeln!(out, "{line}")?;
    }
    out.flush()
}

fn uci_info_line(result: &SearchResult, elapsed: Duration) -> String {
    let nodes = total_nodes(result.stats);
    let millis = elapsed.as_millis().max(1);
    let nps = nodes as u128 * 1_000 / millis;
    let pv = format_pv(result);
    let score = format_score(result.score);

    format!(
        "info depth {} {score} nodes {nodes} nps {nps} hashfull {} time {} pv {pv}",
        result.depth,
        result.stats.hashfull_permill,
        elapsed.as_millis()
    )
}

fn uci_bestmove_line(result: &SearchResult) -> String {
    let best = result
        .best_move
        .map(|mv| mv.uci())
        .unwrap_or_else(|| "0000".to_string());

    format!("bestmove {best}")
}

fn total_nodes(stats: SearchStats) -> u64 {
    stats.nodes + stats.qnodes
}

fn format_pv(result: &SearchResult) -> String {
    if result.principal_variation.is_empty() {
        result
            .best_move
            .map(|mv| mv.uci())
            .unwrap_or_else(|| "0000".to_string())
    } else {
        result
            .principal_variation
            .iter()
            .map(|mv| mv.uci())
            .collect::<Vec<_>>()
            .join(" ")
    }
}

fn format_score(score: i32) -> String {
    if score.abs() > MATE_SCORE - 1_000 {
        let plies = (MATE_SCORE - score.abs()).max(0);
        let mate = ((plies + 1) / 2).max(1);
        if score >= 0 {
            format!("score mate {mate}")
        } else {
            format!("score mate -{mate}")
        }
    } else {
        format!("score cp {score}")
    }
}

pub fn parse_position(command: &str) -> Result<Board, String> {
    let tokens = command.split_whitespace().collect::<Vec<_>>();
    if tokens.first() != Some(&"position") {
        return Err("expected position command".to_string());
    }

    let Some(kind) = tokens.get(1).copied() else {
        return Err("missing position type".to_string());
    };

    let mut moves_index = None;
    for (idx, token) in tokens.iter().enumerate() {
        if *token == "moves" {
            moves_index = Some(idx);
            break;
        }
    }

    let mut board = match kind {
        "startpos" => Board::startpos().map_err(|err| err.to_string())?,
        "fen" => {
            let end = moves_index.unwrap_or(tokens.len());
            if end < 8 {
                return Err("FEN position must have six fields".to_string());
            }
            let fen = tokens[2..end].join(" ");
            Board::from_fen(&fen).map_err(|err| err.to_string())?
        }
        other => return Err(format!("unknown position type: {other}")),
    };

    if let Some(idx) = moves_index {
        for uci in &tokens[idx + 1..] {
            let mv = find_legal_move_by_uci(&mut board, uci)
                .ok_or_else(|| format!("illegal move in position command: {uci}"))?;
            board.make_move(mv);
        }
    }

    Ok(board)
}

pub fn parse_go(command: &str) -> GoCommand {
    let tokens = command.split_whitespace().collect::<Vec<_>>();
    let mut go = GoCommand::default();
    let mut idx = 1;

    while idx < tokens.len() {
        match tokens[idx] {
            "depth" => {
                if let Some(value) = tokens.get(idx + 1).and_then(|value| value.parse().ok()) {
                    go.depth = Some(value);
                }
                idx += 2;
            }
            "movetime" => {
                if let Some(value) = parse_millis(tokens.get(idx + 1).copied()) {
                    go.movetime = Some(value);
                }
                idx += 2;
            }
            "nodes" => {
                if let Some(value) = tokens.get(idx + 1).and_then(|value| value.parse().ok()) {
                    go.nodes = Some(value);
                }
                idx += 2;
            }
            "wtime" => {
                go.wtime = parse_millis(tokens.get(idx + 1).copied());
                idx += 2;
            }
            "btime" => {
                go.btime = parse_millis(tokens.get(idx + 1).copied());
                idx += 2;
            }
            "winc" => {
                go.winc = parse_millis(tokens.get(idx + 1).copied());
                idx += 2;
            }
            "binc" => {
                go.binc = parse_millis(tokens.get(idx + 1).copied());
                idx += 2;
            }
            "movestogo" => {
                if let Some(value) = tokens.get(idx + 1).and_then(|value| value.parse().ok()) {
                    go.movestogo = Some(value);
                }
                idx += 2;
            }
            "infinite" => {
                go.infinite = true;
                idx += 1;
            }
            _ => idx += 1,
        }
    }

    go
}

fn parse_millis(value: Option<&str>) -> Option<Duration> {
    value
        .and_then(|value| value.parse::<u64>().ok())
        .map(Duration::from_millis)
}

pub fn apply_setoption(command: &str, options: &mut UciOptions) -> Result<(), String> {
    let tokens = command.split_whitespace().collect::<Vec<_>>();
    let name_pos = tokens
        .iter()
        .position(|token| *token == "name")
        .ok_or_else(|| "missing option name".to_string())?;
    let value_pos = tokens
        .iter()
        .position(|token| *token == "value")
        .ok_or_else(|| "missing option value".to_string())?;

    if value_pos <= name_pos + 1 || value_pos + 1 >= tokens.len() {
        return Err("invalid setoption command".to_string());
    }

    let name = tokens[name_pos + 1..value_pos]
        .join("")
        .to_ascii_lowercase();
    let value = tokens[value_pos + 1]
        .parse::<u32>()
        .map_err(|_| "option value must be a number".to_string())?;

    match name.as_str() {
        "searchdepth" => options.search_depth = value.clamp(1, 64) as u8,
        "quiescencedepth" => options.quiescence_depth = value.clamp(0, 16) as u8,
        "candidatecount" => options.candidate_count = value.clamp(1, 20) as usize,
        "hash" => options.hash_size_mb = value.clamp(1, 1024) as usize,
        other => return Err(format!("unknown option: {other}")),
    }

    Ok(())
}

#[allow(dead_code)]
pub fn startpos_fen() -> &'static str {
    STARTPOS_FEN
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_startpos_with_moves() {
        let board = parse_position("position startpos moves e2e4 e7e5 g1f3").unwrap();
        assert_eq!(
            board.to_fen(),
            "rnbqkbnr/pppp1ppp/8/4p3/4P3/5N2/PPPP1PPP/RNBQKB1R b KQkq - 1 2"
        );
    }

    #[test]
    fn parses_fen_position() {
        let board = parse_position("position fen 7k/8/5K2/8/8/6Q1/8/8 w - - 0 1").unwrap();
        assert_eq!(board.side_to_move(), Color::White);
    }

    #[test]
    fn parses_go_depth_and_movetime() {
        let go = parse_go("go depth 5 movetime 250 nodes 1000");
        assert_eq!(go.depth, Some(5));
        assert_eq!(go.movetime, Some(Duration::from_millis(250)));
        assert_eq!(go.nodes, Some(1000));
    }

    #[test]
    fn clock_go_allocates_a_budget() {
        let go = parse_go("go wtime 30000 btime 40000 winc 1000 movestogo 20");
        let limits = go.to_limits(Color::White, UciOptions::default());
        assert!(limits.move_time.is_none());
        assert_eq!(limits.wtime, Some(30000));
        assert_eq!(limits.btime, Some(40000));
        assert_eq!(limits.winc, Some(1000));
        assert_eq!(limits.max_depth, UciOptions::default().search_depth);
    }

    #[test]
    fn setoption_updates_engine_options() {
        let mut options = UciOptions::default();
        apply_setoption("setoption name SearchDepth value 7", &mut options).unwrap();
        apply_setoption("setoption name QuiescenceDepth value 2", &mut options).unwrap();
        apply_setoption("setoption name CandidateCount value 9", &mut options).unwrap();
        apply_setoption("setoption name Hash value 32", &mut options).unwrap();
        assert_eq!(options.search_depth, 7);
        assert_eq!(options.quiescence_depth, 2);
        assert_eq!(options.candidate_count, 9);
        assert_eq!(options.hash_size_mb, 32);
    }
}
