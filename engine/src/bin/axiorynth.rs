use std::env;

use axiorynth_engine::{
    Board, BotLevel, Color, Game, PlayerMemory, STARTPOS_FEN, SearchControl, SearchLimits,
    analyze_position, build_training_report, choose_bot_move, evaluate, iterative_deepening, perft,
    research_roadmap, run_bench, run_uci_stdio,
};

fn main() {
    if let Err(err) = run() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let Some(command) = args.first().map(String::as_str) else {
        return run_uci_stdio().map_err(|err| err.to_string());
    };

    match command {
        "uci" => return run_uci_stdio().map_err(|err| err.to_string()),
        "eval" => {
            let fen = normalize_fen(&join_fen_args(&args[1..])?);
            let board = Board::from_fen(&fen).map_err(|err| err.to_string())?;
            for line in evaluate(&board).as_math_lines() {
                println!("{line}");
            }
        }
        "best" => {
            let (fen, depth) = parse_fen_and_depth(&args[1..], 3)?;
            let mut board = Board::from_fen(&fen).map_err(|err| err.to_string())?;
            let control = SearchControl::new();
            let result = iterative_deepening(
                &mut board,
                SearchLimits {
                    max_depth: depth,
                    ..SearchLimits::default()
                },
                &control,
            );
            for line in result.as_math_lines() {
                println!("{line}");
            }
        }
        "perft" => {
            let (fen, depth) = parse_fen_and_depth(&args[1..], 3)?;
            let mut board = Board::from_fen(&fen).map_err(|err| err.to_string())?;
            let nodes = perft(&mut board, depth as u32);
            println!("perft depth: {depth}");
            println!("nodes: {nodes}");
        }
        "bench" => {
            let depth = args
                .get(1)
                .and_then(|value| value.parse::<u8>().ok())
                .unwrap_or(3)
                .max(1);
            let report = run_bench(depth)?;
            for line in report.as_lines() {
                println!("{line}");
            }
        }
        "analyze" => {
            let (fen, depth) = parse_fen_and_depth(&args[1..], 2)?;
            let board = Board::from_fen(&fen).map_err(|err| err.to_string())?;
            let control = SearchControl::new();
            let report = analyze_position(
                &board,
                SearchLimits {
                    max_depth: depth,
                    ..SearchLimits::default()
                },
                &control,
            );
            for line in report.as_lines() {
                println!("{line}");
            }
        }
        "bot" => {
            let level = args
                .get(1)
                .and_then(|value| value.parse::<u8>().ok())
                .unwrap_or(5);
            let fen = args
                .get(2)
                .map(|value| normalize_fen(value))
                .unwrap_or_else(|| STARTPOS_FEN.to_string());
            let board = Board::from_fen(&fen).map_err(|err| err.to_string())?;
            let control = SearchControl::new();
            let bot_move = choose_bot_move(&board, BotLevel::new(level), &control);
            for line in bot_move.as_lines() {
                println!("{line}");
            }
        }
        "game" => {
            let game = build_game_from_moves(&args[1..])?;
            for line in game.as_lines() {
                println!("{line}");
            }
        }
        "memory" => {
            let game = build_game_from_moves(&args[1..])?;
            let mut memory = PlayerMemory::new("local-player");
            memory.learn_from_game(&game, Color::White);
            for line in memory.as_lines() {
                println!("{line}");
            }
        }
        "train" => {
            let game = build_game_from_moves(&args[1..])?;
            let mut memory = PlayerMemory::new("local-player");
            memory.learn_from_game(&game, Color::White);
            let report = build_training_report(&[game], &memory);
            for line in report.as_lines() {
                println!("{line}");
            }
        }
        "roadmap" => {
            for line in research_roadmap().as_lines() {
                println!("{line}");
            }
        }
        "help" | "--help" | "-h" => print_usage(),
        other => return Err(format!("unknown command: {other}")),
    }

    Ok(())
}

fn print_usage() {
    println!("Axiorynth engine CLI");
    println!();
    println!("Usage:");
    println!("  axiorynth eval startpos");
    println!("  axiorynth eval \"<fen>\"");
    println!("  axiorynth best startpos 3");
    println!("  axiorynth best \"<fen>\" 3");
    println!("  axiorynth perft startpos 3");
    println!("  axiorynth bench 3");
    println!("  axiorynth analyze startpos 2");
    println!("  axiorynth bot 5 startpos");
    println!("  axiorynth game e2e4 e7e5 g1f3");
    println!("  axiorynth memory e2e4 e7e5");
    println!("  axiorynth train e2e4 e7e5");
    println!("  axiorynth roadmap");
    println!("  axiorynth uci");
    println!();
    println!("Running without arguments starts the UCI protocol loop.");
}

fn parse_fen_and_depth(args: &[String], default_depth: u8) -> Result<(String, u8), String> {
    if args.is_empty() {
        return Ok((STARTPOS_FEN.to_string(), default_depth));
    }

    let maybe_depth = args.last().and_then(|value| value.parse::<u8>().ok());
    let (fen_args, depth) = if let Some(depth) = maybe_depth {
        (&args[..args.len() - 1], depth)
    } else {
        (args, default_depth)
    };

    Ok((normalize_fen(&join_fen_args(fen_args)?), depth.max(1)))
}

fn join_fen_args(args: &[String]) -> Result<String, String> {
    if args.is_empty() {
        return Ok(STARTPOS_FEN.to_string());
    }

    let fen = args.join(" ");
    if fen.trim().is_empty() {
        Err("FEN cannot be empty".to_string())
    } else {
        Ok(fen)
    }
}

fn normalize_fen(fen: &str) -> String {
    if fen.eq_ignore_ascii_case("startpos") {
        STARTPOS_FEN.to_string()
    } else {
        fen.to_string()
    }
}

fn build_game_from_moves(moves: &[String]) -> Result<Game, String> {
    let mut game = Game::new()?;
    for mv in moves {
        game.play_uci(mv)?;
    }
    Ok(game)
}
