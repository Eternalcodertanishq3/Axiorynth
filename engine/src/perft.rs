use crate::board::Board;
use crate::movegen::generate_legal_moves;
use crate::mv::Move;

pub fn perft(board: &mut Board, depth: u32) -> u64 {
    if depth == 0 {
        return 1;
    }

    let moves = generate_legal_moves(board);
    if depth == 1 {
        return moves.len() as u64;
    }

    let mut nodes = 0;
    for mv in moves {
        let undo = board.make_move(mv);
        nodes += perft(board, depth - 1);
        board.undo_move(undo);
    }
    nodes
}

pub fn divide(board: &mut Board, depth: u32) -> Vec<(Move, u64)> {
    let mut rows = Vec::new();
    for mv in generate_legal_moves(board) {
        let undo = board.make_move(mv);
        let nodes = perft(board, depth.saturating_sub(1));
        board.undo_move(undo);
        rows.push((mv, nodes));
    }
    rows.sort_by_key(|(mv, _)| mv.uci());
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn startpos_perft_depths_1_to_3() {
        let mut board = Board::startpos().unwrap();
        assert_eq!(perft(&mut board, 1), 20);
        assert_eq!(perft(&mut board, 2), 400);
        assert_eq!(perft(&mut board, 3), 8_902);
    }

    #[test]
    fn kiwipete_perft_depths_1_to_3() {
        let mut board =
            Board::from_fen("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1")
                .unwrap();
        assert_eq!(perft(&mut board, 1), 48);
        assert_eq!(perft(&mut board, 2), 2_039);
        assert_eq!(perft(&mut board, 3), 97_862);
    }

    #[test]
    fn tricky_endgame_perft_depths_1_to_3() {
        let mut board = Board::from_fen("8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1").unwrap();
        assert_eq!(perft(&mut board, 1), 14);
        assert_eq!(perft(&mut board, 2), 191);
        assert_eq!(perft(&mut board, 3), 2_812);
    }

    #[test]
    fn promotion_pressure_perft_depths_1_to_3() {
        let mut board =
            Board::from_fen("r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1")
                .unwrap();
        assert_eq!(perft(&mut board, 1), 6);
        assert_eq!(perft(&mut board, 2), 264);
        assert_eq!(perft(&mut board, 3), 9_467);
    }

    #[test]
    fn tactical_middlegame_perft_depths_1_to_3() {
        let mut board = Board::from_fen(
            "r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10",
        )
        .unwrap();
        assert_eq!(perft(&mut board, 1), 46);
        assert_eq!(perft(&mut board, 2), 2_079);
        assert_eq!(perft(&mut board, 3), 89_890);
    }
}
