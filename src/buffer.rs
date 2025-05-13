use crate::{line, str_utils};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BufferType {
    Original,
    Add,
}

#[derive(Debug)]
pub(crate) struct Buffer<T> {
    pub(crate) content: T,
    #[cfg(feature = "lines")]
    pub(crate) line_breaks: Vec<(usize, line::Break)>,
}

#[derive(Debug)]
pub(crate) struct Buffers<'b> {
    pub(crate) original: Buffer<&'b str>,
    pub(crate) add: Buffer<String>,
}

impl<'b> Buffers<'b> {
    pub(crate) fn from_initial(initial: &'b str) -> Self {
        let mut line_breaks = vec![];
        str_utils::line_breaks(initial, &mut line_breaks, 0);

        Self {
            original: Buffer {
                content: initial,
                #[cfg(feature = "lines")]
                line_breaks,
            },
            add: Buffer {
                content: String::new(),
                #[cfg(feature = "lines")]
                line_breaks: vec![],
            },
        }
    }

    #[cfg(feature = "lines")]
    pub(crate) fn line_breaks(
        &self,
        ty: BufferType,
    ) -> &[(usize, line::Break)] {
        match ty {
            BufferType::Original => &self.original.line_breaks,
            BufferType::Add => &self.add.line_breaks,
        }
    }
}

impl<'b> std::ops::Index<BufferType> for Buffers<'b> {
    type Output = str;

    fn index(&self, index: BufferType) -> &Self::Output {
        match index {
            BufferType::Original => self.original.content,
            BufferType::Add => &self.add.content,
        }
    }
}
