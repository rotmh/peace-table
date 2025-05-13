//! A UTF-8; char and line oriented; text editing optimized; [Piece Table]
//! implementation.
//!
//! [Piece Table]: https://en.wikipedia.org/wiki/Piece_table

#![feature(test, if_let_guard, stmt_expr_attributes)]

mod buffer;
#[cfg(feature = "lines")]
mod line;
mod piece;
mod rbtree;
mod slice;
mod str_utils;

use buffer::{BufferType, Buffers};
use piece::Piece;
use slice::Slice;

#[derive(Debug)]
pub struct PieceTable<'b> {
    pieces: Vec<Piece>,
    buffers: Buffers<'b>,

    len_bytes: usize,
    len_chars: usize,
    #[cfg(feature = "lines")]
    len_lines: usize,

    /// The char index after the last insertion, and the piece the last
    /// insertion was inserting to (i.e., `(char_idx, piece_idx)`). If there is
    /// no last insertion, or the last edit is not an insertion (thus
    /// invalidating the `last_insert` value), it will contain a [`None`].
    ///
    /// This is used as an optimization, so that instead of creating a new
    /// piece when inserting contiguous text (for every insert), we will just
    /// expand the last piece.
    #[cfg(feature = "contiguous-inserts")]
    last_insert: Option<(usize, usize)>,
}

impl<'b> PieceTable<'b> {
    /// Create a new [`PieceTable`] with the initial contents set to `initial`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use peace_table::PieceTable;
    /// let pt = PieceTable::new("initial");
    /// assert_eq!(pt.text(), "initial");
    /// ```
    pub fn new(initial: &'b str) -> Self {
        let buffers = Buffers::from_initial(initial);
        let initial_piece = Piece {
            buffer: BufferType::Original,
            start: 0,
            len_bytes: initial.len(),
            len_chars: str_utils::count_chars(initial),
            first_line_break: if buffers.original.line_breaks.is_empty() {
                None
            } else {
                Some(0)
            },
        };

        Self {
            len_bytes: initial.len(),
            len_chars: str_utils::count_chars(initial),
            #[cfg(feature = "lines")]
            len_lines: buffers.original.line_breaks.len() + 1,

            #[cfg(feature = "contiguous-inserts")]
            last_insert: None,

            buffers,
            pieces: vec![initial_piece],
        }
    }

    /// Collect the text from the piece table.
    ///
    /// This function allocates a new string. You can use [`PieceTable::iter`]
    /// to iterate over `&str` chunks without allocations.
    ///
    /// # Examples
    ///
    /// ```
    /// # use peace_table::PieceTable;
    /// let mut pt = PieceTable::new("content");
    /// pt.insert(0, "abcd, ");
    /// assert_eq!(pt.text(), "abcd, content");
    /// ```
    pub fn text(&self) -> String {
        let mut text = String::with_capacity(self.len_bytes);

        for piece in &self.pieces {
            text.push_str(&self.buffers[piece.buffer][piece.byte_range()]);
        }

        debug_assert_eq!(text.len(), self.len_bytes);
        debug_assert_eq!(str_utils::count_chars(&text), self.len_chars);

        text
    }

    /// Returns a [`Slice`] containing the `line_idx`-th line, **without** the
    /// line break sequence.
    ///
    /// # Panics
    ///
    /// Will panic if `line_idx` is out of bounds (i.e., there is no such line).
    ///
    /// # Examples
    ///
    /// ```
    /// # use peace_table::PieceTable;
    /// let mut pt = PieceTable::new("FirstSecond");
    /// pt.insert(5, "\r\n");
    /// assert_eq!(pt.line(1).to_string(), "Second");
    /// ```
    #[cfg(feature = "lines")]
    pub fn line(&self, line_idx: usize) -> Slice {
        assert!(line_idx < self.len_lines, "line index out of bounds");

        let mut current_line = 0;
        let mut start = (0, 0);

        for (piece_idx, piece) in self.pieces.iter().enumerate() {
            let Some(first_lb) = piece.first_line_break else { continue };

            let line_breaks = self.buffers.line_breaks(piece.buffer)
                [first_lb..]
                .iter()
                .take_while(|(i, _)| *i < piece.byte_range().end);

            for &(idx, ty) in line_breaks {
                let relative_idx = idx - piece.start;

                if current_line == line_idx {
                    let end = (piece_idx, relative_idx);
                    return Slice::new(start, end, self);
                }
                if current_line + 1 == line_idx {
                    start = (piece_idx, relative_idx + ty.len_bytes());
                }

                current_line += 1;
            }
        }

        debug_assert_eq!(line_idx, self.len_lines - 1);

        let last_idx = self.pieces.len() - 1;
        let end_byte = self.pieces[last_idx].len_bytes;
        Slice::new(start, (last_idx, end_byte), self)
    }

    /// Removes the text in the given char index range.
    ///
    /// # Examples
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
    /// pt.remove(5..0); // an empty range
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

        // If the removal is _after_ the index of the last insert, it does not
        // affect it.
        #[cfg(feature = "contiguous-inserts")]
        if self.last_insert.is_some_and(|(i, _piece)| i >= start) {
            self.last_insert = None;
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
    /// # Examples
    ///
    /// ```
    /// # use peace_table::PieceTable;
    /// let mut pt = PieceTable::new("rld");
    /// pt.insert(0, "hellowo");
    /// pt.insert(5, " ");
    /// assert_eq!(pt.text(), "hello world");
    /// ```
    ///
    /// # Panics
    ///
    /// Will panic if index is larger than the size of the contents.
    ///
    /// ```should_panic
    /// # use peace_table::PieceTable;
    /// let mut pt = PieceTable::new("012");
    /// pt.insert(4, " "); // will panic
    /// ```
    pub fn insert(&mut self, char_idx: usize, text: &str) {
        let len_chars = str_utils::count_chars(text);

        self.len_chars += len_chars;
        self.len_bytes += text.len();

        #[cfg(feature = "contiguous-inserts")]
        if let Some((ref mut i, piece_idx)) = self.last_insert
            && *i == char_idx
        {
            *i += len_chars;
            self.extend_piece(text, len_chars, piece_idx);
            return;
        }

        let (piece_idx, relative_char_idx) = self.piece_at_char(char_idx);

        if relative_char_idx == 0 {
            self.insert_piece(piece_idx, text);
        } else if relative_char_idx == self.pieces[piece_idx].len_chars {
            self.insert_piece(piece_idx + 1, text);
        } else {
            // This is guarenteed to be a valid char index inside the piece, due
            // to an earlier assertion in `piece_at_char`.
            self.split_piece_and_insert(piece_idx, relative_char_idx, text);
        }

        #[cfg(feature = "contiguous-inserts")]
        {
            let piece_idx =
                if relative_char_idx == 0 { piece_idx } else { piece_idx + 1 };
            self.last_insert = Some((char_idx + len_chars, piece_idx));
        }
    }

    /// Total number of chars in the piece table.
    ///
    /// Runs in `O(1)`.
    ///
    /// # Examples
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
    /// Runs in `O(1)`.
    ///
    /// # Examples
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

        let piece_text = &self.buffers[piece.buffer][piece.start..];

        // TODO: should we make this a `debug_assert!`?
        assert!(
            !(piece_text.as_bytes()[char_idx - 1] == 0x0D
                && piece_text.as_bytes()[char_idx] == 0x0A),
            "inserting inside a CRLF sequece is invalid"
        );

        // Create the `after` piece, before modifying `piece`.
        let byte_idx = str_utils::char_to_byte(piece_text, char_idx);
        let after_start = piece.start + byte_idx;
        let after_end = after_start + piece.len_bytes - byte_idx;
        let first_line_break = piece.first_line_break.and_then(|flb| {
            let mut lbs = self.buffers.line_breaks(piece.buffer)[flb..].iter();
            lbs.find(|(idx, _ty)| *idx >= after_start && *idx < after_end)
        });
        let after = Piece {
            buffer: piece.buffer,
            start: after_start,
            first_line_break: first_line_break.map(|(idx, _ty)| idx).copied(),
            len_bytes: piece.len_bytes - byte_idx,
            len_chars: piece.len_chars - char_idx,
        };

        // Modify `piece` in-place to be the `before` piece.
        piece.len_bytes = byte_idx;
        piece.len_chars = char_idx;

        // Insert the new and the `after` piece.
        self.insert_piece(piece_idx + 1, text);
        self.pieces.insert(piece_idx + 2, after);
    }

    /// Returns an iterator over all the `&str` chunks in the table.
    ///
    /// # Examples
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
        let first_lb = self.buffers.add.line_breaks.len();
        self.len_lines += str_utils::line_breaks(
            text,
            &mut self.buffers.add.line_breaks,
            self.buffers.add.content.len(),
        );

        let piece = Piece {
            buffer: BufferType::Add,
            start: self.buffers.add.content.len(),
            first_line_break: (first_lb < self.buffers.add.line_breaks.len())
                .then_some(first_lb),
            len_chars: str_utils::count_chars(text),
            len_bytes: text.len(),
        };

        self.buffers.add.content.push_str(text);
        self.pieces.insert(index, piece);
    }

    fn piece_at_char(&self, char_idx: usize) -> (usize, usize) {
        assert!(char_idx <= self.len_chars, "index out of bounds");

        let mut offset = 0;
        for (i, piece) in self.pieces.iter().enumerate() {
            offset += piece.len_chars;

            if offset >= char_idx {
                let relative_idx = char_idx - (offset - piece.len_chars);
                return (i, relative_idx);
            }
        }

        unreachable!(
            "this code will be ran only if `index` is larger than the total \
             size len all the pieces together, but this was already asserted"
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
            self.remove_piece(piece_idx);
        } else if start_char_idx < piece.len_chars {
            let byte_idx = str_utils::char_to_byte(text, start_char_idx);
            self.len_chars -= piece.len_chars - start_char_idx;
            self.len_bytes -= piece.len_bytes - byte_idx;
            piece.len_bytes = byte_idx;
            piece.len_chars = start_char_idx;

            // Unset the `first_line_break` if it was in the removed part.
            #[cfg(feature = "lines")]
            piece
                .first_line_break
                .as_ref()
                .take_if(|flb| **flb >= piece.byte_range().end);
        }
    }

    fn trim_piece_start(&mut self, piece_idx: usize, end_char_idx: usize) {
        let piece = &mut self.pieces[piece_idx];
        let text = &self.buffers[piece.buffer][piece.byte_range()];

        if end_char_idx == piece.len_chars {
            self.remove_piece(piece_idx);
        } else if end_char_idx > 0 {
            let byte_idx = str_utils::char_to_byte(text, end_char_idx);
            piece.start += byte_idx;
            piece.len_bytes -= byte_idx;
            piece.len_chars -= end_char_idx;
            self.len_chars -= end_char_idx;
            self.len_bytes -= byte_idx;

            #[cfg(feature = "lines")]
            piece.first_line_break = piece.first_line_break.and_then(|flb| {
                // WARNING: if there is no matching line break, it will iterate
                // over _all_ of the line breaks. fix that.
                let mut lbs =
                    self.buffers.line_breaks(piece.buffer)[flb..].iter();
                Some(lbs.find(|(idx, _ty)| piece.byte_range().contains(idx))?.0)
            });
        }
    }

    fn remove_piece(&mut self, piece_idx: usize) {
        let piece = &self.pieces[piece_idx];
        #[cfg(feature = "lines")]
        let lbs = self.count_piece_line_breaks(piece_idx);

        self.len_bytes -= piece.len_bytes;
        self.len_chars -= piece.len_chars;
        #[cfg(feature = "lines")]
        self.len_lines -= lbs;

        self.pieces.remove(piece_idx);
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

        let start_offset = str_utils::char_to_byte(text, start_char_idx);
        let end_offset = str_utils::char_to_byte(text, end_char_idx);

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

    /// Extend a piece's end, and inserts the text to the end of the `add`
    /// buffer. This function assumes that the last insert to the table was to
    /// the end of the piece.
    #[cfg(feature = "contiguous-inserts")]
    fn extend_piece(
        &mut self,
        text: &str,
        text_len_chars: usize,
        piece_idx: usize,
    ) {
        let piece = &mut self.pieces[piece_idx];

        debug_assert_eq!(piece.buffer, BufferType::Add);
        debug_assert_eq!(
            self.buffers.add.content.len(),
            piece.byte_range().end
        );

        self.len_lines += str_utils::line_breaks(
            text,
            &mut self.buffers.add.line_breaks,
            piece.byte_range().end,
        );

        piece.len_bytes += text.len();
        piece.len_chars += text_len_chars;

        self.buffers.add.content.push_str(text);
    }

    /// Count the amount of line breaks that a piece contains.
    ///
    /// Runs in `O(N)` where `N` is the amount of line breaks in the piece (this
    /// is a good time complexity).
    #[cfg(feature = "lines")]
    fn count_piece_line_breaks(&self, piece_idx: usize) -> usize {
        let piece = &self.pieces[piece_idx];
        if let Some(first_lb) = piece.first_line_break {
            let lbs = self.buffers.line_breaks(piece.buffer)[first_lb..].iter();
            let (s, e) = (piece.start, piece.byte_range().end);
            lbs.take_while(|(idx, _ty)| *idx >= s && *idx < e).count()
        } else {
            0
        }
    }
}

impl std::fmt::Display for PieceTable<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.iter().try_for_each(|p| write!(f, "{p}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contiguous_insertion() {
        let mut pt = PieceTable::new("ag");

        let letters = ('b'..='f').map(|ch| ch.to_string());
        letters.enumerate().for_each(|(i, ch)| pt.insert(i + 1, &ch));

        assert_eq!(pt.text(), "abcdefg");

        if cfg!(feature = "contiguous-inserts") {
            assert_eq!(pt.pieces.len(), 3);
        } else {
            assert_eq!(pt.pieces.len(), 7);
        }
    }
}

#[cfg(test)]
mod benches {
    extern crate test;

    use self::test::Bencher;
    use std::process::Termination;

    use super::*;

    #[bench]
    fn bench_sequential_inserts(b: &mut Bencher) -> impl Termination {
        b.iter(|| {
            const CH: &str = "a";
            let mut pt = PieceTable::new("asdfjlkajslkdfjlkajsldkfjlkasjdlkfj");
            for i in 10..10000 {
                pt.insert(i, CH);
            }
            pt.insert(2, CH);
            pt.remove(4..294);
            for i in 3..5531 {
                pt.insert(i, CH);
            }
        });
    }
}
