use std::fmt;

use crate::types::{PieceKind, Square};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MoveKind {
    Quiet,
    Capture,
    DoublePawnPush,
    EnPassant,
    KingCastle,
    QueenCastle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Move {
    pub from: Square,
    pub to: Square,
    pub promotion: Option<PieceKind>,
    pub kind: MoveKind,
}

impl Move {
    pub const fn new(
        from: Square,
        to: Square,
        promotion: Option<PieceKind>,
        kind: MoveKind,
    ) -> Move {
        Move {
            from,
            to,
            promotion,
            kind,
        }
    }

    pub fn is_capture(self) -> bool {
        matches!(self.kind, MoveKind::Capture | MoveKind::EnPassant)
    }

    pub fn uci(self) -> String {
        let mut out = format!("{}{}", self.from, self.to);
        if let Some(promotion) = self.promotion {
            out.push(promotion.fen_lower());
        }
        out
    }
}

impl fmt::Display for Move {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.uci())
    }
}
