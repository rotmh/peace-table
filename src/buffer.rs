#[derive(Debug, Clone, Copy)]
pub(crate) enum Buffer {
    Original,
    Add,
}

#[derive(Debug)]
pub(crate) struct Buffers<'b> {
    pub(crate) original: &'b str,
    pub(crate) add: String,
}

impl<'b> Buffers<'b> {
    pub(crate) fn from_initial(initial: &'b str) -> Self {
        Self { original: initial, add: String::new() }
    }
}

impl<'b> std::ops::Index<Buffer> for Buffers<'b> {
    type Output = str;

    fn index(&self, index: Buffer) -> &Self::Output {
        match index {
            Buffer::Original => self.original,
            Buffer::Add => &self.add,
        }
    }
}
