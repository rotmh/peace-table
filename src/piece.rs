use crate::buffer::BufferType;

#[derive(Debug)]
pub(crate) struct Piece {
    /// Which [`Buffer`] is this piece referencing.
    pub(crate) buffer: BufferType,
    /// Start index in the buffer.
    pub(crate) start: usize,

    /// The index of the first line break index in the buffer's `line_breaks`.
    #[cfg(feature = "lines")]
    pub(crate) first_line_break: Option<usize>,

    pub(crate) len_bytes: usize,
    pub(crate) len_chars: usize,
}

impl Piece {
    pub(crate) fn byte_range(&self) -> std::ops::Range<usize> {
        self.start..self.start + self.len_bytes
    }
}

#[cfg(test)]
mod tests {
    use crate::{PieceTable, line};

    #[test]
    #[cfg(feature = "lines")]
    fn first_line_break() {
        let pt = PieceTable::new("012\r\n567");
        let idx = pt.pieces[0].first_line_break.unwrap();
        let &(lb_idx, lb_type) = &pt.buffers.original.line_breaks[idx];

        assert_eq!(lb_type, line::Break::Crlf);
        assert_eq!(pt.buffers.original.content.as_bytes()[lb_idx], b'\r');
        assert_eq!(pt.buffers.original.content.as_bytes()[lb_idx + 1], b'\n');
    }
}
