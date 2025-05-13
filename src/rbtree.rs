#![allow(dead_code)]

use std::ptr::NonNull;

struct Piece {
    len_bytes: usize,
    len_chars: usize,
}

pub(crate) enum Error {
    CharIndexOutOfBounds,
}

pub(crate) type Result<T> = std::result::Result<T, Error>;

enum Color {
    Red,
    Black,
}

struct Node {
    piece: Piece,

    color: Color,

    /// The amount of characters there are in the left subtree.
    left_len: usize,
    left_line_breaks: usize,

    parent: Option<NodePtr>,
    left: Option<NodePtr>,
    right: Option<NodePtr>,
}

type NodePtr = NonNull<Node>;

impl Node {
    const NIL: Option<NodePtr> = None;

    fn new(piece: Piece) -> NodePtr {
        let node = Self {
            piece,
            color: Color::Black,
            left_len: 0,
            left_line_breaks: 0,
            parent: Self::NIL,
            left: Self::NIL,
            right: Self::NIL,
        };
        let ptr = Box::into_raw(Box::new(node));
        // SAFETY: `Box::into_raw` guarantees a non-null pointer.
        unsafe { NonNull::new_unchecked(ptr) }
    }
}

pub(crate) struct Tree {
    root: Option<NodePtr>,
}

impl Tree {
    pub(crate) fn insert(&mut self, text: &str, char_idx: usize) -> Result<()> {
        if let Some(root) = self.root {
            let (node_start, node) = self.node_at_char(char_idx)?;

            if node_start == char_idx {
                self.insert_before(text, node);
            }
        }

        Ok(())
    }

    fn insert_before(&mut self, text: &str, node: NodePtr) {
        //
    }

    fn node_at_char(&self, mut char_idx: usize) -> Result<(usize, NodePtr)> {
        let mut curr = self.root;
        // The start offset of the node in the document, in chars.
        let mut node_start = 0;

        while let Some(node) = curr {
            let node = unsafe { node.as_ref() };

            if node.left_len > char_idx {
                curr = node.left;
            } else if node.left_len + node.piece.len_chars >= char_idx {
                node_start += node.left_len;
                return Ok((node_start, node.into()));
            } else {
                char_idx -= node.left_len + node.piece.len_chars;
                node_start += node.left_len + node.piece.len_chars;
                curr = node.right;
            }
        }

        Err(Error::CharIndexOutOfBounds)
    }
}
