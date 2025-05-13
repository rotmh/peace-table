use std::ops::Not;

use crate::PieceTable;

/// `(piece_idx, byte_idx)`.
type Position = (usize, usize);

#[derive(Debug)]
pub struct Slice<'a> {
    /// The position of the start piece and byte index in it, inclusive.
    start: Position,
    /// The position of the end piece and byte index in it, exclusive.
    end: Position,
    table: &'a PieceTable<'a>,
}

impl<'a> Slice<'a> {
    pub(crate) const fn new(
        start: Position,
        end: Position,
        table: &'a PieceTable,
    ) -> Self {
        Self { start, end, table }
    }

    pub fn iter(&self) -> impl Iterator<Item = &str> {
        let pieces = dbg!(&self.table.pieces[self.start.0..=self.end.0]);
        let buffers = &self.table.buffers;

        pieces.iter().enumerate().filter_map(move |(i, piece)| {
            let range = piece.byte_range();
            let range = if i == 0 && i == pieces.len() - 1 {
                range.start + self.start.1..range.start + self.end.1
            } else if i == 0 {
                range.start + self.start.1..range.end
            } else if i == pieces.len() - 1 {
                piece.start..range.start + self.end.1
            } else {
                range
            };

            let s = &buffers[piece.buffer][range];
            s.is_empty().not().then_some(s)
        })
    }
}

impl<'a> std::fmt::Display for Slice<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.iter().try_for_each(|s| write!(f, "{s}"))
    }
}
