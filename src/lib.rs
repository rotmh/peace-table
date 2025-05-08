#![allow(dead_code)]

use std::{
    num::{NonZero, NonZeroUsize},
    ops::Index,
};

pub struct PieceTable<'b> {
    pieces: Vec<Piece>,
    buffers: Buffers<'b>,
    total_size: usize,
}

impl<'b> PieceTable<'b> {
    /// Create a new [`PieceTable`] with the initial contents set to `initial`.
    ///
    /// ## Examples
    ///
    /// ```
    /// # use peace_table::PieceTable;
    /// let mut pt = PieceTable::new(b"initial");
    /// assert_eq!(pt.content(), b"initial");
    /// ```
    pub fn new(initial: &'b [u8]) -> Self {
        let first_piece =
            Piece { buffer: Buffer::Original, start: 0, length: initial.len() };

        Self {
            pieces: vec![first_piece],
            buffers: Buffers { original: initial, add: vec![] },
            total_size: initial.len(),
        }
    }

    /// Insert `content` at position `index`.
    ///
    /// ## Examples
    ///
    /// ```
    /// # use peace_table::PieceTable;
    /// let mut pt = PieceTable::new(b"rld");
    /// pt.insert(0, b"hellowo");
    /// pt.insert(5, b" ");
    /// assert_eq!(pt.content(), b"hello world");
    /// ```
    ///
    /// ## Panics
    ///
    /// Will panic if index is larger than the size of the contents.
    ///
    /// ```should_panic
    /// # use peace_table::PieceTable;
    /// let mut pt = PieceTable::new(b"012");
    /// pt.insert(4, b""); // will panic
    /// ```
    pub fn insert(&mut self, index: usize, content: &[u8]) {
        let (piece_index, insertion_location) =
            self.find_insertion_location(index);

        match insertion_location {
            InsertionLocation::Start => {
                self.insert_piece(piece_index, content);
            }
            InsertionLocation::Index(index) => {
                let split_index = index.get();
                self.split_piece_and_insert(piece_index, split_index, content);
            }
            InsertionLocation::End => {
                self.insert_piece(piece_index + 1, content);
            }
        }

        dbg!(&self.pieces);

        self.total_size += content.len();
    }

    /// Allocate a vector with the entire contents of the piece table.
    ///
    /// ## Examples
    ///
    /// ```
    /// # use peace_table::PieceTable;
    /// let mut pt = PieceTable::new(b"12");
    /// pt.insert(2, b"34");
    /// assert_eq!(pt.content(), b"1234");
    /// ```
    pub fn content(&self) -> Vec<u8> {
        let mut content = Vec::with_capacity(self.total_size);
        for piece in &self.pieces {
            let buffer = &self.buffers[piece.buffer];
            let end = piece.start + piece.length;
            let slice = &buffer[piece.start..end];
            content.extend_from_slice(slice);
        }
        content
    }

    /// Returns the total size of the contents of this piece table.
    ///
    /// ## Examples
    ///
    /// ```
    /// # use peace_table::PieceTable;
    /// let mut pt = PieceTable::new(b"123456");
    /// assert_eq!(pt.size(), 6);
    /// ```
    #[inline(always)]
    pub fn size(&self) -> usize {
        self.total_size
    }

    fn split_piece_and_insert(
        &mut self,
        piece_index: usize,
        split_index: usize,
        content: &[u8],
    ) {
        let piece = &mut self.pieces[piece_index];

        // Create the `after` piece, before modifying `piece`.
        let after = Piece {
            buffer: piece.buffer,
            start: piece.start + split_index,
            length: piece.length - split_index,
        };

        // Modify `piece` to be the `before` piece.
        piece.length = split_index;

        // Insert the new and the `after` piece.
        self.insert_piece(piece_index + 1, content);
        self.pieces.insert(piece_index + 2, after);
    }

    /// Create a new "add" piece with `content`, and insert that piece at
    /// `index`.
    fn insert_piece(&mut self, index: usize, content: &[u8]) {
        let start = self.buffers.add.len();
        self.buffers.add.extend_from_slice(content);
        let piece = Piece { buffer: Buffer::Add, start, length: content.len() };
        self.pieces.insert(index, piece);
    }

    fn find_insertion_location(
        &self,
        index: usize,
    ) -> (usize, InsertionLocation) {
        assert!(index <= self.total_size, "index out of bounds");

        let mut offset = 0;
        for (i, piece) in self.pieces.iter().enumerate() {
            offset += piece.length;

            if offset >= index {
                let relative_index = index - (offset - piece.length);
                return (i, piece.calc_insertion_location(relative_index));
            }
        }

        unreachable!(
            "this code will be ran only if `index` is larger than the total \
             size of all the pieces together, but this was already asserted"
        )
    }
}

/// Where an insertion should occur, relative to some piece.
enum InsertionLocation {
    Start,
    Index(NonZeroUsize),
    End,
}

#[derive(Debug, Clone, Copy)]
enum Buffer {
    Original,
    Add,
}

struct Buffers<'b> {
    original: &'b [u8],
    add: Vec<u8>,
}

impl<'b> Index<Buffer> for Buffers<'b> {
    type Output = [u8];

    fn index(&self, index: Buffer) -> &Self::Output {
        match index {
            Buffer::Original => self.original,
            Buffer::Add => &self.add,
        }
    }
}

#[derive(Debug)]
struct Piece {
    /// Which [`Buffer`] is this piece referencing.
    buffer: Buffer,
    /// Start index in the buffer.
    start: usize,
    /// Length of the content this piece is referencing.
    length: usize,
}

impl Piece {
    /// ## Panics
    ///
    /// Will panic if `index` is larger than the piece's length.
    ///
    /// Note: that means that even though the index is zero-based, it can still
    /// be equal to the length of the piece (i.e., `0..=length` and not
    /// `0..length`).
    fn calc_insertion_location(&self, index: usize) -> InsertionLocation {
        assert!(self.length >= index, "index out of bounds");

        if index == 0 {
            InsertionLocation::Start
        } else if index == self.length {
            InsertionLocation::End
        } else {
            // SAFETY: `index == 0` was covered by an earlier branch.
            InsertionLocation::Index(unsafe { NonZero::new_unchecked(index) })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert() {
        let mut pt = PieceTable::new(b"helloworld");
        pt.insert(5, b" ");
        assert_eq!(pt.content(), b"hello world");
    }
}
