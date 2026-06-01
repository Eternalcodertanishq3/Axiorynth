use std::env;

use axiorynth_engine::{
    Board, BotLevel, BotMove, Color, EvalBreakdown, Game, GameRecord, PlayerMemory, STARTPOS_FEN,
    SearchControl, SearchLimits, SearchResult, analyze_position, build_training_report,
    choose_bot_move, evaluate, generate_legal_moves, iterative_deepening, perft, research_roadmap,
    run_bench, run_uci_stdio,
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
        "frontend-state" => print_frontend_state(&args[1..])?,
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
    println!("  axiorynth frontend-state --bot-level 3 --depth 2 e2e4 e7e5");
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

fn print_frontend_state(args: &[String]) -> Result<(), String> {
    let (bot_level, depth, moves) = parse_frontend_state_args(args)?;
    let game = build_game_from_moves(&moves)?;
    let board = game.board();

    let mut legal_board = board.clone();
    let mut legal_moves = generate_legal_moves(&mut legal_board)
        .into_iter()
        .map(|mv| mv.uci())
        .collect::<Vec<_>>();
    legal_moves.sort();

    let evaluation = evaluate(board);
    let control = SearchControl::new();
    let mut search_board = board.clone();
    let search = iterative_deepening(
        &mut search_board,
        SearchLimits {
            max_depth: depth,
            quiescence_depth: depth.min(4),
            candidate_count: 6,
            hash_size_mb: 4,
            ..SearchLimits::default()
        },
        &control,
    );
    let bot = choose_bot_move(board, BotLevel::new(bot_level), &control);

    println!(
        "{}",
        frontend_state_json(&game, &legal_moves, &evaluation, &search, &bot)
    );
    Ok(())
}

fn parse_frontend_state_args(args: &[String]) -> Result<(u8, u8, Vec<String>), String> {
    let mut bot_level = 3u8;
    let mut depth = 2u8;
    let mut moves = Vec::new();
    let mut idx = 0;

    while idx < args.len() {
        match args[idx].as_str() {
            "--bot-level" | "-b" => {
                idx += 1;
                let value = args
                    .get(idx)
                    .ok_or_else(|| "--bot-level requires a value".to_string())?;
                bot_level = value
                    .parse::<u8>()
                    .map_err(|_| format!("invalid bot level: {value}"))?
                    .clamp(1, 10);
            }
            "--depth" | "-d" => {
                idx += 1;
                let value = args
                    .get(idx)
                    .ok_or_else(|| "--depth requires a value".to_string())?;
                depth = value
                    .parse::<u8>()
                    .map_err(|_| format!("invalid analysis depth: {value}"))?
                    .clamp(1, 5);
            }
            "--moves" => {}
            value => moves.push(value.to_string()),
        }
        idx += 1;
    }

    Ok((bot_level, depth, moves))
}

fn frontend_state_json(
    game: &Game,
    legal_moves: &[String],
    evaluation: &EvalBreakdown,
    search: &SearchResult,
    bot: &BotMove,
) -> String {
    let board = game.board();
    let side_to_move = match board.side_to_move() {
        Color::White => "white",
        Color::Black => "black",
    };
    let in_check = board.in_check(board.side_to_move());
    let moves = game.uci_moves();

    format!(
        concat!(
            "{{",
            "\"engine\":{},",
            "\"ply\":{},",
            "\"moves\":{},",
            "\"result\":{},",
            "\"fen\":{},",
            "\"sideToMove\":{},",
            "\"inCheck\":{},",
            "\"legalMoves\":{},",
            "\"history\":{},",
            "\"evaluation\":{},",
            "\"search\":{},",
            "\"bot\":{}",
            "}}"
        ),
        json_string("Axiorynth"),
        game.records().len(),
        json_string_array(&moves),
        json_string(game.result().as_str()),
        json_string(&board.to_fen()),
        json_string(side_to_move),
        in_check,
        json_string_array(legal_moves),
        history_json(game.records()),
        evaluation_json(evaluation),
        search_json(search),
        bot_json(bot)
    )
}

fn history_json(records: &[GameRecord]) -> String {
    let rows = records
        .iter()
        .map(|record| {
            format!(
                concat!(
                    "{{",
                    "\"ply\":{},",
                    "\"uci\":{},",
                    "\"evalAfter\":{},",
                    "\"resultAfter\":{},",
                    "\"fenAfter\":{}",
                    "}}"
                ),
                record.ply,
                json_string(&record.uci),
                record.eval_after,
                json_string(record.result_after.as_str()),
                json_string(&record.fen_after)
            )
        })
        .collect::<Vec<_>>();
    format!("[{}]", rows.join(","))
}

fn evaluation_json(evaluation: &EvalBreakdown) -> String {
    let math_lines = evaluation.as_math_lines();
    format!(
        concat!(
            "{{",
            "\"materialWhite\":{},",
            "\"materialBlack\":{},",
            "\"materialScore\":{},",
            "\"pieceSquareWhite\":{},",
            "\"pieceSquareBlack\":{},",
            "\"pieceSquareScore\":{},",
            "\"mobilityWhite\":{},",
            "\"mobilityBlack\":{},",
            "\"mobilityScore\":{},",
            "\"centerWhite\":{},",
            "\"centerBlack\":{},",
            "\"centerScore\":{},",
            "\"pawnStructureWhite\":{},",
            "\"pawnStructureBlack\":{},",
            "\"pawnStructureScore\":{},",
            "\"kingSafetyWhite\":{},",
            "\"kingSafetyBlack\":{},",
            "\"kingSafetyScore\":{},",
            "\"totalWhitePerspective\":{},",
            "\"totalSideToMovePerspective\":{},",
            "\"mathLines\":{}",
            "}}"
        ),
        evaluation.material_white,
        evaluation.material_black,
        evaluation.material_score,
        evaluation.piece_square_white,
        evaluation.piece_square_black,
        evaluation.piece_square_score,
        evaluation.mobility_white,
        evaluation.mobility_black,
        evaluation.mobility_score,
        evaluation.center_white,
        evaluation.center_black,
        evaluation.center_score,
        evaluation.pawn_structure_white,
        evaluation.pawn_structure_black,
        evaluation.pawn_structure_score,
        evaluation.king_safety_white,
        evaluation.king_safety_black,
        evaluation.king_safety_score,
        evaluation.total_white_perspective,
        evaluation.total_side_to_move_perspective,
        json_string_array(&math_lines)
    )
}

fn search_json(search: &SearchResult) -> String {
    let math_lines = search.as_math_lines();
    let pv = search
        .principal_variation
        .iter()
        .map(|mv| mv.uci())
        .collect::<Vec<_>>();
    let candidates = search
        .candidates
        .iter()
        .map(|candidate| {
            format!(
                "{{\"move\":{},\"score\":{}}}",
                json_string(&candidate.mv.uci()),
                candidate.score
            )
        })
        .collect::<Vec<_>>();

    format!(
        concat!(
            "{{",
            "\"bestMove\":{},",
            "\"score\":{},",
            "\"depth\":{},",
            "\"nodes\":{},",
            "\"qnodes\":{},",
            "\"betaCutoffs\":{},",
            "\"qBetaCutoffs\":{},",
            "\"ttHits\":{},",
            "\"ttStores\":{},",
            "\"hashfullPermill\":{},",
            "\"killerUses\":{},",
            "\"stopped\":{},",
            "\"principalVariation\":{},",
            "\"candidates\":[{}],",
            "\"mathLines\":{}",
            "}}"
        ),
        optional_move_json(search.best_move),
        search.score,
        search.depth,
        search.stats.nodes,
        search.stats.qnodes,
        search.stats.beta_cutoffs,
        search.stats.q_beta_cutoffs,
        search.stats.tt_hits,
        search.stats.tt_stores,
        search.stats.hashfull_permill,
        search.stats.killer_uses,
        search.stats.stopped,
        json_string_array(&pv),
        candidates.join(","),
        json_string_array(&math_lines)
    )
}

fn bot_json(bot: &BotMove) -> String {
    let math_lines = bot.as_lines();
    format!(
        concat!(
            "{{",
            "\"level\":{},",
            "\"name\":{},",
            "\"description\":{},",
            "\"selectedMove\":{},",
            "\"searchScore\":{},",
            "\"searchDepth\":{},",
            "\"mathLines\":{}",
            "}}"
        ),
        bot.profile.level.value(),
        json_string(bot.profile.name),
        json_string(bot.profile.description),
        optional_move_json(bot.selected_move),
        bot.search.score,
        bot.search.depth,
        json_string_array(&math_lines)
    )
}

fn optional_move_json(mv: Option<axiorynth_engine::Move>) -> String {
    mv.map(|value| json_string(&value.uci()))
        .unwrap_or_else(|| "null".to_string())
}

fn json_string_array(values: &[String]) -> String {
    let escaped = values
        .iter()
        .map(|value| json_string(value))
        .collect::<Vec<_>>();
    format!("[{}]", escaped.join(","))
}

fn json_string(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch if ch.is_control() => out.push_str(&format!("\\u{:04x}", ch as u32)),
            ch => out.push(ch),
        }
    }
    out.push('"');
    out
}
