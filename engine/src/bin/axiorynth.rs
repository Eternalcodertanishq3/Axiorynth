use std::env;

use axiorynth_engine::{
    Board, BotLevel, BotMove, Color, EvalBreakdown, EvalConfig, Game, GameRecord, PlayerMemory, STARTPOS_FEN,
    SearchControl, SearchLimits, SearchResult, analyze_position, build_training_report,
    choose_bot_move, evaluate, generate_legal_moves, iterative_deepening, perft, research_roadmap,
    run_bench, run_uci_stdio, OpeningBook, NnueNetwork, load_nnue,
};

fn main() {
    if let Err(err) = run() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    if std::path::Path::new("tuned_eval.json").exists() {
        if let Ok(file) = std::fs::File::open("tuned_eval.json") {
            if let Ok(config) = serde_json::from_reader(file) {
                axiorynth_engine::eval::update_config(config);
            }
        }
    }
    if std::path::Path::new("axiorynth.nnue").exists() {
        if let Err(e) = load_nnue("axiorynth.nnue") {
            eprintln!("Warning: Failed to load NNUE weights: {e}");
        } else {
            println!("Loaded NNUE weights from axiorynth.nnue");
        }
    }
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
            let bot_move = choose_bot_move(&board, BotLevel::new(level), &control, "", &std::collections::HashMap::new());
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
        "self-play" => {
            let games_count = args.get(1).and_then(|v| v.parse::<usize>().ok()).unwrap_or(10);
            let lvl_a = BotLevel::new(args.get(2).and_then(|v| v.parse::<u8>().ok()).unwrap_or(5));
            let lvl_b = BotLevel::new(args.get(3).and_then(|v| v.parse::<u8>().ok()).unwrap_or(5));
            
            println!("Starting self-play matches: {} games, Bot Level {} vs Bot Level {}", games_count, lvl_a.value(), lvl_b.value());
            
            let mut fens_output = Vec::new();
            let mut results = [0; 3]; // [white_wins, black_wins, draws]
            
            for g in 0..games_count {
                let mut game = Game::new()?;
                let control = SearchControl::new();
                let mut game_fens = Vec::new();
                
                while game.result() == axiorynth_engine::GameResult::Ongoing {
                    let board = game.board();
                    game_fens.push(board.to_fen());
                    
                    let level = if board.side_to_move() == Color::White { lvl_a } else { lvl_b };
                    let bot_move = choose_bot_move(board, level, &control, "", &std::collections::HashMap::new());
                    
                    if let Some(mv) = bot_move.selected_move {
                        game.play_uci(&mv.uci())?;
                    } else {
                        break;
                    }
                }
                
                let result = game.result();
                match result {
                    axiorynth_engine::GameResult::WhiteWin => results[0] += 1,
                    axiorynth_engine::GameResult::BlackWin => results[1] += 1,
                    axiorynth_engine::GameResult::DrawStalemate | axiorynth_engine::GameResult::DrawFiftyMove => results[2] += 1,
                    _ => results[2] += 1,
                }
                
                println!("Game {} finished. Result: {:?}", g + 1, result);
                
                for fen in game_fens {
                    fens_output.push(format!("{} | {:?}", fen, result));
                }
            }
            
            println!("Self-play complete! Wins: {}, Losses: {}, Draws: {}", results[0], results[1], results[2]);
            
            std::fs::write("self_play_data.txt", fens_output.join("\n")).map_err(|e| e.to_string())?;
            println!("Saved training FENs to self_play_data.txt");
        }
        "spsa-tune" => {
            let iterations = args.get(1).and_then(|v| v.parse::<usize>().ok()).unwrap_or(5);
            println!("Starting SPSA tuning loop for {} iterations...", iterations);
            
            let mut theta = config_to_vec(&axiorynth_engine::eval::get_config());
            let alpha = 0.602;
            let gamma = 0.101;
            let a = 10.0;
            let c = 5.0;
            let a_const = 2.0;
            
            for iter in 1..=iterations {
                let ak = a / (iter as f64 + 1.0 + a_const).powf(alpha);
                let ck = c / (iter as f64 + 1.0).powf(gamma);
                
                let delta: Vec<f64> = (0..13).map(|_| if rand_sign() { 1.0 } else { -1.0 }).collect();
                
                let mut theta_plus = vec![0.0; 13];
                let mut theta_minus = vec![0.0; 13];
                for i in 0..13 {
                    theta_plus[i] = theta[i] + ck * delta[i];
                    theta_minus[i] = theta[i] - ck * delta[i];
                }
                
                let config_plus = vec_to_config(&theta_plus);
                let config_minus = vec_to_config(&theta_minus);
                let current_baseline = vec_to_config(&theta);
                
                let score_plus = run_spsa_eval_matches(&config_plus, &current_baseline, 4)?;
                let score_minus = run_spsa_eval_matches(&config_minus, &current_baseline, 4)?;
                
                let diff = score_plus - score_minus;
                
                for i in 0..13 {
                    let grad = diff / (2.0 * ck * delta[i]);
                    theta[i] += ak * grad;
                }
                
                let current_config = vec_to_config(&theta);
                theta = config_to_vec(&current_config);
                
                println!("Iteration {}: pawn_val = {}, knight_val = {}, bishop_val = {}, rook_val = {}, queen_val = {}",
                         iter, current_config.pawn_val, current_config.knight_val, current_config.bishop_val, current_config.rook_val, current_config.queen_val);
            }
            
            let final_config = vec_to_config(&theta);
            axiorynth_engine::eval::update_config(final_config);
            
            let file = std::fs::File::create("tuned_eval.json").map_err(|e| e.to_string())?;
            serde_json::to_writer_pretty(file, &final_config).map_err(|e| e.to_string())?;
            
            println!("SPSA tuning complete. Saved final parameters to tuned_eval.json.");
        }
        "load-config" => {
            let file = std::fs::File::open("tuned_eval.json")
                .map_err(|e| format!("Failed to open tuned_eval.json: {e}"))?;
            let config: EvalConfig = serde_json::from_reader(file)
                .map_err(|e| format!("Failed to parse tuned_eval.json: {e}"))?;
            axiorynth_engine::eval::update_config(config);
            println!("Successfully loaded and applied config from tuned_eval.json:");
            println!("{:#?}", config);
        }
        "gauntlet" => {
            let games = args.get(1).and_then(|v| v.parse::<usize>().ok()).ok_or_else(|| "Missing games argument".to_string())?;
            let depth_a = args.get(2).and_then(|v| v.parse::<u8>().ok()).ok_or_else(|| "Missing depth_a argument".to_string())?;
            let depth_b = args.get(3).and_then(|v| v.parse::<u8>().ok()).ok_or_else(|| "Missing depth_b argument".to_string())?;
            
            println!("Starting gauntlet: {} games, Depth A = {} vs Depth B = {}", games, depth_a, depth_b);
            
            let mut wins_a = 0;
            let mut wins_b = 0;
            let mut draws = 0;
            let control = SearchControl::new();
            
            for g in 0..games {
                let mut game = Game::new()?;
                let is_a_white = g % 2 == 0;
                
                while game.result() == axiorynth_engine::GameResult::Ongoing {
                    let board = game.board();
                    let side = board.side_to_move();
                    let current_depth = if (side == Color::White && is_a_white) || (side == Color::Black && !is_a_white) {
                        depth_a
                    } else {
                        depth_b
                    };
                    
                    let mut search_board = board.clone();
                    let search = iterative_deepening(
                        &mut search_board,
                        SearchLimits {
                            max_depth: current_depth,
                            ..SearchLimits::default()
                        },
                        &control,
                    );
                    
                    if let Some(mv) = search.best_move {
                        game.play_uci(&mv.uci())?;
                    } else {
                        break;
                    }
                }
                
                match game.result() {
                    axiorynth_engine::GameResult::WhiteWin => {
                        if is_a_white {
                            wins_a += 1;
                        } else {
                            wins_b += 1;
                        }
                    }
                    axiorynth_engine::GameResult::BlackWin => {
                        if is_a_white {
                            wins_b += 1;
                        } else {
                            wins_a += 1;
                        }
                    }
                    _ => {
                        draws += 1;
                    }
                }
                println!("Game {}/{} complete. Wins A: {}, Wins B: {}, Draws: {}", g + 1, games, wins_a, wins_b, draws);
            }
            
            let score = (wins_a as f64 + draws as f64 * 0.5) / games as f64;
            let elo_diff = if score <= 0.0 {
                -999.0
            } else if score >= 1.0 {
                999.0
            } else {
                -400.0 * (1.0 / score - 1.0).log10()
            };
            let score_pct = score * 100.0;
            
            println!("\n+---------------------------------------+");
            println!("|          Gauntlet Results             |");
            println!("+---------------------------------------+");
            println!("| Games   : {:<27} |", games);
            println!("| Depth A : {:<27} |", depth_a);
            println!("| Depth B : {:<27} |", depth_b);
            println!("| Wins A  : {:<27} |", wins_a);
            println!("| Draws   : {:<27} |", draws);
            println!("| Wins B  : {:<27} |", wins_b);
            println!("| Elo Diff: {:<+27.2} |", elo_diff);
            println!("| Score % : {:<27.1}% |", score_pct);
            println!("+---------------------------------------+");
            
            let results_json = serde_json::json!({
                "games": games,
                "depth_a": depth_a,
                "depth_b": depth_b,
                "wins_a": wins_a,
                "draws": draws,
                "wins_b": wins_b,
                "elo_diff": (elo_diff * 10.0).round() / 10.0,
                "score_pct": (score_pct * 10.0).round() / 10.0
            });
            
            let file = std::fs::File::create("gauntlet_results.json").map_err(|e| e.to_string())?;
            serde_json::to_writer_pretty(file, &results_json).map_err(|e| e.to_string())?;
            println!("Saved results to gauntlet_results.json");
        }
        "book-gen" => {
            let num_games = args.get(1).and_then(|v| v.parse::<usize>().ok()).ok_or_else(|| "Missing num_games argument".to_string())?;
            let depth = args.get(2).and_then(|v| v.parse::<u8>().ok()).ok_or_else(|| "Missing depth argument".to_string())?;
            
            println!("Generating opening book: {} games at depth {}", num_games, depth);
            
            let mut games_records = Vec::new();
            let control = SearchControl::new();
            
            for g in 0..num_games {
                let mut game = Game::new()?;
                
                while game.result() == axiorynth_engine::GameResult::Ongoing {
                    let board = game.board();
                    let mut search_board = board.clone();
                    let search = iterative_deepening(
                        &mut search_board,
                        SearchLimits {
                            max_depth: depth,
                            ..SearchLimits::default()
                        },
                        &control,
                    );
                    
                    if let Some(mv) = search.best_move {
                        game.play_uci(&mv.uci())?;
                    } else {
                        break;
                    }
                }
                
                let result_str = match game.result() {
                    axiorynth_engine::GameResult::WhiteWin => "1-0",
                    axiorynth_engine::GameResult::BlackWin => "0-1",
                    _ => "1/2-1/2",
                };
                
                let moves = game.uci_moves();
                games_records.push((moves, result_str.to_string()));
                println!("Game {}/{} complete. Result: {}", g + 1, num_games, result_str);
            }
            
            let games_ref: Vec<(Vec<String>, &str)> = games_records
                .iter()
                .map(|(moves, result)| (moves.clone(), result.as_str()))
                .collect();
                
            let book = OpeningBook::generate_from_games(&games_ref);
            book.save("axiorynth.book").map_err(|e| format!("Failed to save book: {e}"))?;
            println!("Opening book generated and saved to axiorynth.book. Total positions: {}", book.len());
        }
        "book-probe" => {
            let fen = normalize_fen(&join_fen_args(&args[1..])?);
            let board = Board::from_fen(&fen).map_err(|err| err.to_string())?;
            
            let book = OpeningBook::load("axiorynth.book")
                .map_err(|e| format!("Failed to load book: {e}"))?;
                
            let hash = board.hash();
            if let Some(entries) = book.probe(hash) {
                println!("Book moves for position (hash: {:x}):", hash);
                for entry in entries {
                    println!("  {} | weight: {} | win rate: {:.1}%", 
                        entry.uci_move, entry.weight, entry.score * 100.0);
                }
                if let Some(best) = book.best_move(hash) {
                    println!("Best move: {}", best.uci_move);
                }
            } else {
                println!("No book moves found for position (hash: {:x})", hash);
            }
        }
        "nnue-gen" => {
            let games = args.get(1).and_then(|v| v.parse::<usize>().ok()).ok_or_else(|| "Missing games argument".to_string())?;
            let depth = args.get(2).and_then(|v| v.parse::<u8>().ok()).ok_or_else(|| "Missing depth argument".to_string())?;
            
            println!("Generating NNUE training data: {} games at search depth {}", games, depth);
            
            let mut dataset = Vec::new();
            let control = SearchControl::new();
            
            for g in 0..games {
                let mut game = Game::new()?;
                
                while game.result() == axiorynth_engine::GameResult::Ongoing {
                    let board = game.board();
                    
                    let mut search_board = board.clone();
                    let search = iterative_deepening(
                        &mut search_board,
                        SearchLimits {
                            max_depth: depth,
                            ..SearchLimits::default()
                        },
                        &control,
                    );
                    
                    let fen = board.to_fen();
                    let target = if board.side_to_move() == Color::White {
                        search.score
                    } else {
                        -search.score
                    };
                    
                    let target_clipped = target.clamp(-2000, 2000);
                    dataset.push(format!("{}|{}", fen, target_clipped));
                    
                    if let Some(mv) = search.best_move {
                        game.play_uci(&mv.uci())?;
                    } else {
                        break;
                    }
                }
                println!("Game {}/{} completed, current dataset size: {}", g + 1, games, dataset.len());
            }
            
            let mut file = std::fs::File::create("nnue_data.txt").map_err(|e| e.to_string())?;
            use std::io::Write;
            for line in &dataset {
                writeln!(file, "{}", line).map_err(|e| e.to_string())?;
            }
            println!("NNUE data generation complete. Saved {} positions to nnue_data.txt.", dataset.len());
        }
        "nnue-train" => {
            let data_file = args.get(1).ok_or_else(|| "Missing data-file argument".to_string())?;
            let epochs = args.get(2).and_then(|v| v.parse::<usize>().ok()).unwrap_or(20);
            
            println!("Training NNUE network using data from {} for {} epochs...", data_file, epochs);
            
            let content = std::fs::read_to_string(data_file).map_err(|e| format!("Failed to read data-file: {e}"))?;
            let mut examples = Vec::new();
            
            for line in content.lines() {
                let line = line.trim();
                if line.is_empty() { continue; }
                let parts: Vec<&str> = line.split('|').collect();
                if parts.len() != 2 { continue; }
                
                let fen = parts[0];
                let score: f32 = parts[1].parse().map_err(|e| format!("Failed to parse score: {e}"))?;
                
                if let Ok(board) = Board::from_fen(fen) {
                    let w_feat = axiorynth_engine::eval::get_half_kp_features(&board, Color::White);
                    let b_feat = axiorynth_engine::eval::get_half_kp_features(&board, Color::Black);
                    examples.push((w_feat, b_feat, score));
                }
            }
            
            if examples.is_empty() {
                return Err("No valid training examples found in data file.".to_string());
            }
            
            println!("Loaded {} training positions.", examples.len());
            
            let mut net = if std::path::Path::new("axiorynth.nnue").exists() {
                println!("Loading existing weights from axiorynth.nnue...");
                NnueNetwork::load("axiorynth.nnue").map_err(|e| format!("Failed to load existing weights: {e}"))?
            } else {
                println!("Initializing new random weights...");
                NnueNetwork::new_random()
            };
            
            let batch_size = 32;
            let initial_lr = 0.01f32;
            
            for epoch in 1..=epochs {
                let mut seed = 12345u64 + epoch as u64;
                let mut next_rand = move || {
                    seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
                    (seed >> 32) as usize
                };
                
                for i in (1..examples.len()).rev() {
                    let j = next_rand() % (i + 1);
                    examples.swap(i, j);
                }
                
                let lr = initial_lr / (1.0 + (epoch - 1) as f32 * 0.05);
                let mut epoch_loss = 0.0f32;
                let mut batches_count = 0;
                
                for chunk in examples.chunks(batch_size) {
                    let loss = net.train_batch(chunk, lr);
                    epoch_loss += loss;
                    batches_count += 1;
                }
                
                let avg_loss = epoch_loss / batches_count as f32;
                println!("Epoch {}/{} | Avg Loss: {:.4} | LR: {:.6}", epoch, epochs, avg_loss, lr);
            }
            
            net.save("axiorynth.nnue").map_err(|e| format!("Failed to save weights: {e}"))?;
            println!("NNUE training complete. Weights saved to axiorynth.nnue.");
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
    println!("  axiorynth load-config");
    println!("  axiorynth gauntlet <games> <depth_a> <depth_b>");
    println!("  axiorynth book-gen <num_games> <depth>");
    println!("  axiorynth book-probe <fen_or_startpos>");
    println!("  axiorynth nnue-gen <games> <depth>");
    println!("  axiorynth nnue-train <data-file> <epochs>");
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
    let bot = choose_bot_move(board, BotLevel::new(bot_level), &control, "", &std::collections::HashMap::new());

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

fn rand_sign() -> bool {
    use std::time::SystemTime;
    let nanos = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    nanos % 2 == 0
}

fn run_spsa_eval_matches(config: &EvalConfig, baseline: &EvalConfig, count: usize) -> Result<f64, String> {
    let mut total_score = 0.0;
    let control = SearchControl::new();
    
    for g in 0..count {
        let mut game = Game::new()?;
        let is_config_white = g % 2 == 0;
        
        while game.result() == axiorynth_engine::GameResult::Ongoing {
            let board = game.board();
            let side = board.side_to_move();
            
            // Set eval weights depending on whose turn it is
            let is_config_turn = (side == Color::White && is_config_white) || (side == Color::Black && !is_config_white);
            if is_config_turn {
                axiorynth_engine::eval::update_config(*config);
            } else {
                axiorynth_engine::eval::update_config(*baseline);
            }
            
            let bot_move = choose_bot_move(board, BotLevel::new(3), &control, "", &std::collections::HashMap::new());
            if let Some(mv) = bot_move.selected_move {
                game.play_uci(&mv.uci())?;
            } else {
                break;
            }
        }
        
        axiorynth_engine::eval::update_config(*baseline);
        
        match game.result() {
            axiorynth_engine::GameResult::WhiteWin => {
                total_score += if is_config_white { 1.0 } else { 0.0 };
            }
            axiorynth_engine::GameResult::BlackWin => {
                total_score += if is_config_white { 0.0 } else { 1.0 };
            }
            _ => {
                total_score += 0.5;
            }
        }
    }
    
    Ok(total_score / count as f64)
}

fn config_to_vec(config: &EvalConfig) -> Vec<f64> {
    vec![
        config.pawn_val as f64,
        config.knight_val as f64,
        config.bishop_val as f64,
        config.rook_val as f64,
        config.queen_val as f64,
        config.center_attack as f64,
        config.center_occupancy as f64,
        config.pawn_doubled_penalty as f64,
        config.pawn_isolated_penalty as f64,
        config.pawn_passed_bonus as f64,
        config.king_safety_shield as f64,
        config.king_safety_attacked_ring as f64,
        config.mobility_multiplier as f64,
    ]
}

fn vec_to_config(v: &[f64]) -> EvalConfig {
    EvalConfig {
        pawn_val: v[0].clamp(50.0, 200.0) as i32,
        knight_val: v[1].clamp(200.0, 450.0) as i32,
        bishop_val: v[2].clamp(200.0, 450.0) as i32,
        rook_val: v[3].clamp(350.0, 700.0) as i32,
        queen_val: v[4].clamp(700.0, 1200.0) as i32,
        center_attack: v[5].clamp(0.0, 30.0) as i32,
        center_occupancy: v[6].clamp(0.0, 30.0) as i32,
        pawn_doubled_penalty: v[7].clamp(0.0, 40.0) as i32,
        pawn_isolated_penalty: v[8].clamp(0.0, 40.0) as i32,
        pawn_passed_bonus: v[9].clamp(0.0, 30.0) as i32,
        king_safety_shield: v[10].clamp(0.0, 30.0) as i32,
        king_safety_attacked_ring: v[11].clamp(0.0, 40.0) as i32,
        mobility_multiplier: v[12].clamp(0.0, 8.0) as i32,
    }
}
