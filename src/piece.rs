use crate::Buffer;

#[derive(Debug)]
pub(crate) struct Piece {
    /// Which [`Buffer`] is this piece referencing.
    pub(crate) buffer: Buffer,
    /// Start index in the buffer.
    pub(crate) start: usize,

    pub(crate) len_bytes: usize,
    pub(crate) len_chars: usize,
}

impl Piece {
    pub(crate) fn byte_range(&self) -> std::ops::Range<usize> {
        self.start..self.start + self.len_bytes
    }
}
