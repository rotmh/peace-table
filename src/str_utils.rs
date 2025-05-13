pub(crate) use str_indices::chars::count as count_chars;
pub(crate) use str_indices::chars::to_byte_idx as char_to_byte;

use crate::line;

/// Insert the indexes of the line breaks in `text` into `v`. `base_idx` will be
/// added to every index.
pub(crate) fn line_breaks(
    text: &str,
    v: &mut Vec<(usize, line::Break)>,
    base_idx: usize,
) -> usize {
    let mut bytes = text
        .as_bytes()
        .iter()
        .enumerate()
        .map(|(idx, byte)| (idx + base_idx, byte))
        .peekable();

    let mut line_breaks = 0;

    while let Some((idx, byte)) = bytes.next() {
        line_breaks += 1;

        match byte {
            0x0A => v.push((idx, line::Break::Lf)),
            0x0D => {
                if bytes.next_if(|&(_idx, &byte)| byte == 0x0A).is_some() {
                    v.push((idx, line::Break::Crlf));
                } else {
                    #[cfg(feature = "unicode-line-breaks")]
                    v.push((idx, line::Break::Cr));
                }
            }
            #[cfg(feature = "unicode-line-breaks")]
            0x0B => v.push((idx, line::Break::Vt)),
            #[cfg(feature = "unicode-line-breaks")]
            0x0C => v.push((idx, line::Break::Ff)),
            // Nel is part of a two-byte UTF-8 sequence, hence there is no need
            // to peek, because if the next byte is not Nel's second byte, it
            // cannot start a new char anyway (thus irrelevant).
            #[cfg(feature = "unicode-line-breaks")]
            0xC2 if let Some((_idx, 0x85)) = bytes.next() => {
                v.push((idx, line::Break::Nel));
            }
            #[cfg(feature = "unicode-line-breaks")]
            0xE2 => {
                let n1 = bytes.next().map(|(_idx, byte)| byte);
                let n2 = bytes.next().map(|(_idx, byte)| byte);
                if n1.is_some_and(|&b| b == 0x80) {
                    match n2 {
                        Some(0xA8) => v.push((idx, line::Break::Ls)),
                        Some(0xA9) => v.push((idx, line::Break::Ps)),
                        _ => {}
                    }
                }
            }
            _ => line_breaks -= 1,
        }
    }

    line_breaks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn count_lines() {
        let mut v = vec![];
        let text = "My name is:\nNot 123, but it is\r\nNot 321 either.";
        line_breaks(text, &mut v, 0);
        dbg!(&v);
        assert_eq!(v.len(), 2)
    }
}
