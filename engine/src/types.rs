use std::fmt;

pub const COLOR_COUNT: usize = 2;
pub const PIECE_KIND_COUNT: usize = 6;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Color {
    White,
    Black,
}

impl Color {
    pub const ALL: [Color; COLOR_COUNT] = [Color::White, Color::Black];

    #[inline]
    pub const fn idx(self) -> usize {
        match self {
            Color::White => 0,
            Color::Black => 1,
        }
    }

    #[inline]
    pub const fn opposite(self) -> Color {
        match self {
            Color::White => Color::Black,
            Color::Black => Color::White,
        }
    }

    #[inline]
    pub const fn fen(self) -> char {
        match self {
            Color::White => 'w',
            Color::Black => 'b',
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PieceKind {
    Pawn,
    Knight,
    Bishop,
    Rook,
    Queen,
    King,
}

impl PieceKind {
    pub const ALL: [PieceKind; PIECE_KIND_COUNT] = [
        PieceKind::Pawn,
        PieceKind::Knight,
        PieceKind::Bishop,
        PieceKind::Rook,
        PieceKind::Queen,
        PieceKind::King,
    ];

    pub const PROMOTIONS: [PieceKind; 4] = [
        PieceKind::Queen,
        PieceKind::Rook,
        PieceKind::Bishop,
        PieceKind::Knight,
    ];

    #[inline]
    pub const fn idx(self) -> usize {
        match self {
            PieceKind::Pawn => 0,
            PieceKind::Knight => 1,
            PieceKind::Bishop => 2,
            PieceKind::Rook => 3,
            PieceKind::Queen => 4,
            PieceKind::King => 5,
        }
    }

    #[inline]
    pub const fn from_fen_lower(ch: char) -> Option<PieceKind> {
        match ch {
            'p' => Some(PieceKind::Pawn),
            'n' => Some(PieceKind::Knight),
            'b' => Some(PieceKind::Bishop),
            'r' => Some(PieceKind::Rook),
            'q' => Some(PieceKind::Queen),
            'k' => Some(PieceKind::King),
            _ => None,
        }
    }

    #[inline]
    pub const fn fen_lower(self) -> char {
        match self {
            PieceKind::Pawn => 'p',
            PieceKind::Knight => 'n',
            PieceKind::Bishop => 'b',
            PieceKind::Rook => 'r',
            PieceKind::Queen => 'q',
            PieceKind::King => 'k',
        }
    }

    #[inline]
    pub const fn material_value(self) -> i32 {
        match self {
            PieceKind::Pawn => 100,
            PieceKind::Knight => 320,
            PieceKind::Bishop => 330,
            PieceKind::Rook => 500,
            PieceKind::Queen => 900,
            PieceKind::King => 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Piece {
    pub color: Color,
    pub kind: PieceKind,
}

impl Piece {
    pub fn from_fen(ch: char) -> Option<Piece> {
        let color = if ch.is_ascii_uppercase() {
            Color::White
        } else {
            Color::Black
        };
        let kind = PieceKind::from_fen_lower(ch.to_ascii_lowercase())?;
        Some(Piece { color, kind })
    }

    pub fn to_fen(self) -> char {
        let ch = self.kind.fen_lower();
        match self.color {
            Color::White => ch.to_ascii_uppercase(),
            Color::Black => ch,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Square(u8);

impl Square {
    #[inline]
    pub const fn new(index: u8) -> Option<Square> {
        if index < 64 {
            Some(Square(index))
        } else {
            None
        }
    }

    #[inline]
    pub const fn from_index(index: usize) -> Square {
        debug_assert!(index < 64);
        Square(index as u8)
    }

    #[inline]
    pub const fn from_coords(file: u8, rank: u8) -> Option<Square> {
        if file < 8 && rank < 8 {
            Some(Square(rank * 8 + file))
        } else {
            None
        }
    }

    pub fn from_name(name: &str) -> Option<Square> {
        let bytes = name.as_bytes();
        if bytes.len() != 2 {
            return None;
        }

        let file = match bytes[0] {
            b'a'..=b'h' => bytes[0] - b'a',
            b'A'..=b'H' => bytes[0] - b'A',
            _ => return None,
        };
        let rank = match bytes[1] {
            b'1'..=b'8' => bytes[1] - b'1',
            _ => return None,
        };

        Square::from_coords(file, rank)
    }

    #[inline]
    pub const fn index(self) -> usize {
        self.0 as usize
    }

    #[inline]
    pub const fn file(self) -> u8 {
        self.0 % 8
    }

    #[inline]
    pub const fn rank(self) -> u8 {
        self.0 / 8
    }

    #[inline]
    pub const fn bit(self) -> u64 {
        1u64 << self.0
    }
}

impl fmt::Display for Square {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let file = (b'a' + self.file()) as char;
        let rank = (b'1' + self.rank()) as char;
        write!(f, "{file}{rank}")
    }
}
