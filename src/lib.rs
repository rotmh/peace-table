mod buffer;
mod piece;

use str_indices::chars::count as count_chars;
use str_indices::chars::to_byte_idx as char_to_byte;

use buffer::{Buffer, Buffers};
use piece::Piece;

#[derive(Debug)]
pub struct PieceTable<'b> {
    pieces: Vec<Piece>,
    buffers: Buffers<'b>,

    len_chars: usize,
    len_bytes: usize,
}

impl<'b> PieceTable<'b> {
    /// Create a new [`PieceTable`] with the initial contents set to `initial`.
    ///
    /// ## Examples
    ///
    /// ```
    /// # use peace_table::PieceTable;
    /// let mut pt = PieceTable::new("initial");
    /// assert_eq!(pt.text(), "initial");
    /// ```
    pub fn new(initial: &'b str) -> Self {
        let initial_piece = Piece {
            buffer: Buffer::Original,
            start: 0,
            len_bytes: initial.len(),
            len_chars: count_chars(initial),
        };

        Self {
            pieces: vec![initial_piece],
            buffers: Buffers::from_initial(initial),
            len_chars: count_chars(initial),
            len_bytes: initial.len(),
        }
    }

    /// Collect the text from the piece table.
    pub fn text(&self) -> String {
        let mut text = String::with_capacity(self.len_bytes);
        for piece in &self.pieces {
            let end = piece.start + piece.len_bytes;
            text.push_str(&self.buffers[piece.buffer][piece.start..end]);
        }

        dbg!(&text);
        debug_assert_eq!(text.len(), self.len_bytes);
        debug_assert_eq!(count_chars(&text), self.len_chars);

        text
    }

    /// Removes the text in the given char index range.
    ///
    /// ## Examples
    ///
    /// ```
    /// # use peace_table::PieceTable;
    /// let mut pt = PieceTable::new("hello_there");
    /// pt.insert(5, "  ");
    /// pt.insert(7, " ");
    /// pt.remove(6..=8);
    /// assert_eq!(pt.text(), "hello there");
    /// ```
    ///
    /// ```
    /// # use peace_table::PieceTable;
    /// let mut pt = PieceTable::new("012345");
    /// pt.remove(0..=5);
    /// assert_eq!(pt.text(), "");
    /// ```
    ///
    /// ```
    /// # use peace_table::PieceTable;
    /// let mut pt = PieceTable::new("012345");
    /// pt.remove(5..0);
    /// assert_eq!(pt.text(), "012345"); // unchanged
    /// ```
    pub fn remove<R>(&mut self, range: R)
    where
        R: std::ops::RangeBounds<usize>,
    {
        let (start, end) = self.simplify_range_bounds(range);
        if start >= end {
            return; // the range is empty
        }

        let (start_piece_idx, start_char_idx) = self.piece_at_char(start);
        let (end_piece_idx, end_char_idx) = self.piece_at_char(end);

        if start_piece_idx == end_piece_idx {
            let piece_idx = start_piece_idx;
            self.remove_within_piece(piece_idx, start_char_idx, end_char_idx);
            return;
        }

        self.trim_piece_start(end_piece_idx, end_char_idx);
        self.remove_pieces(start_piece_idx + 1..end_piece_idx);
        self.trim_piece_end(start_piece_idx, start_char_idx);
    }

    /// Insert `content` at position `index`.
    ///
    /// ## Examples
    ///
    /// ```
    /// # use peace_table::PieceTable;
    /// let mut pt = PieceTable::new("rld");
    /// pt.insert(0, "hellowo");
    /// pt.insert(5, " ");
    /// assert_eq!(pt.text(), "hello world");
    /// ```
    ///
    /// ## Panics
    ///
    /// Will panic if index is larger than the size of the contents.
    ///
    /// ```should_panic
    /// # use peace_table::PieceTable;
    /// let mut pt = PieceTable::new("012");
    /// pt.insert(4, " "); // will panic
    /// ```
    pub fn insert(&mut self, char_idx: usize, text: &str) {
        let (piece_idx, char_idx) = self.piece_at_char(char_idx);

        if char_idx == 0 {
            self.insert_piece(piece_idx, text);
        } else if char_idx == self.pieces[piece_idx].len_chars {
            self.insert_piece(piece_idx + 1, text);
        } else {
            // This is guarenteed to be a valid char index inside the piece, due
            // to an earlier assertion in `piece_at_char`.
            self.split_piece_and_insert(piece_idx, char_idx, text);
        }

        self.len_chars += text.chars().count();
        self.len_bytes += text.len();
    }

    /// Total number of chars in the piece table.
    ///
    /// ## Examples
    ///
    /// ```
    /// # use peace_table::PieceTable;
    /// let mut pt = PieceTable::new("123456");
    /// assert_eq!(pt.len_chars(), 6);
    /// ```
    #[inline(always)]
    pub fn len_chars(&self) -> usize {
        self.len_chars
    }

    /// Total number of bytes in the piece table.
    ///
    /// ## Examples
    ///
    /// ```
    /// # use peace_table::PieceTable;
    /// let mut pt = PieceTable::new("1234⑤");
    /// assert_eq!(pt.len_bytes(), 7); // the 5 takes 3 bytes
    /// ```
    #[inline(always)]
    pub fn len_bytes(&self) -> usize {
        self.len_bytes
    }

    fn split_piece_and_insert(
        &mut self,
        piece_idx: usize,
        char_idx: usize,
        text: &str,
    ) {
        let piece = &mut self.pieces[piece_idx];

        // Create the `after` piece, before modifying `piece`.
        let piece_text = &self.buffers[piece.buffer][piece.start..];
        let byte_idx = char_to_byte(piece_text, char_idx);
        let after = Piece {
            buffer: piece.buffer,
            start: piece.start + byte_idx,
            len_bytes: piece.len_bytes - byte_idx,
            len_chars: piece.len_chars - char_idx,
        };

        // Modify `piece` inplace to be the `before` piece.
        piece.len_bytes = byte_idx;
        piece.len_chars = char_idx;

        // Insert the new and the `after` piece.
        self.insert_piece(piece_idx + 1, text);
        self.pieces.insert(piece_idx + 2, after);
    }

    /// Returns an iterator over all the `&str` chunks in the table.
    ///
    /// ## Examples
    ///
    /// ```
    /// # use peace_table::PieceTable;
    /// let mut pt = PieceTable::new("hithere");
    /// pt.insert(2, ", and hello, ");
    /// assert_eq!(pt.iter().collect::<String>(), "hi, and hello, there");
    /// ```
    pub fn iter(&self) -> impl Iterator<Item = &str> {
        self.pieces.iter().map(|p| &self.buffers[p.buffer][p.byte_range()])
    }

    /// Create a new "add" piece with `content`, and insert that piece at
    /// `index`.
    fn insert_piece(&mut self, index: usize, text: &str) {
        let piece = Piece {
            buffer: Buffer::Add,
            start: self.buffers.add.len(),
            len_chars: text.chars().count(),
            len_bytes: text.len(),
        };

        self.buffers.add.push_str(text);
        self.pieces.insert(index, piece);
    }

    fn piece_at_char(&self, char_idx: usize) -> (usize, usize) {
        assert!(char_idx <= self.len_chars, "index out of bounds");

        let mut char_offset = 0;
        for (i, piece) in self.pieces.iter().enumerate() {
            char_offset += piece.len_chars;

            if char_offset >= char_idx {
                let relative_idx = char_idx - (char_offset - piece.len_chars);
                return (i, relative_idx);
            }
        }

        unreachable!(
            "this code will be ran only if `index` is larger than the total \
             size of all the pieces together, but this was already asserted"
        )
    }

    fn simplify_range_bounds<R>(&mut self, range: R) -> (usize, usize)
    where
        R: std::ops::RangeBounds<usize>,
    {
        let start = match range.start_bound() {
            std::ops::Bound::Included(&i) => i,
            std::ops::Bound::Excluded(&i) => i + 1,
            std::ops::Bound::Unbounded => 0,
        };
        let end = match range.end_bound() {
            std::ops::Bound::Included(&i) => i + 1,
            std::ops::Bound::Excluded(&i) => i,
            std::ops::Bound::Unbounded => self.len_chars,
        };
        (start, end)
    }

    fn trim_piece_end(&mut self, piece_idx: usize, start_char_idx: usize) {
        let piece = &mut self.pieces[piece_idx];
        let text = &self.buffers[piece.buffer][piece.byte_range()];

        if start_char_idx == 0 {
            self.pieces.remove(piece_idx);
        } else if start_char_idx < piece.len_chars {
            let byte_idx = char_to_byte(text, start_char_idx);
            self.len_chars -= piece.len_chars - start_char_idx;
            self.len_bytes -= piece.len_bytes - byte_idx;
            piece.len_bytes = byte_idx;
            piece.len_chars = start_char_idx;
        }
    }

    fn trim_piece_start(&mut self, piece_idx: usize, end_char_idx: usize) {
        let piece = &mut self.pieces[piece_idx];
        let text = &self.buffers[piece.buffer][piece.byte_range()];

        if end_char_idx == piece.len_chars {
            self.pieces.remove(piece_idx);
        } else if end_char_idx > 0 {
            let byte_idx = char_to_byte(text, end_char_idx);
            piece.start += byte_idx;
            piece.len_bytes -= byte_idx;
            piece.len_chars -= end_char_idx;
            self.len_chars -= end_char_idx;
            self.len_bytes -= byte_idx;
        }
    }

    fn remove_within_piece(
        &mut self,
        piece_idx: usize,
        start_char_idx: usize,
        end_char_idx: usize,
    ) {
        let piece = &mut self.pieces[piece_idx];
        let text = &self.buffers[piece.buffer][piece.byte_range()];

        // If the range describes an entire piece, remove it.
        if start_char_idx == 0 && end_char_idx == piece.len_chars {
            let piece = &self.pieces[piece_idx];
            self.len_bytes -= piece.len_bytes;
            self.len_chars -= piece.len_chars;
            self.pieces.remove(piece_idx);
            return;
        }

        let start_offset = char_to_byte(text, start_char_idx);
        let end_offset = char_to_byte(text, end_char_idx);

        let new_len_bytes = end_offset - start_offset;
        let new_len_chars = end_char_idx - start_char_idx;

        piece.start += start_offset;
        piece.len_bytes = new_len_bytes;
        piece.len_chars = new_len_chars;

        let removed_bytes = piece.len_bytes - new_len_bytes;
        self.len_bytes -= removed_bytes;
        let removed_chars = piece.len_chars - new_len_chars;
        self.len_chars -= removed_chars;
    }

    fn remove_pieces(&mut self, range: std::ops::Range<usize>) {
        self.pieces.drain(range).for_each(|p| {
            self.len_chars -= p.len_chars;
            self.len_bytes -= p.len_bytes;
        });
    }
}
