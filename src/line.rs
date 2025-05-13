#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub(crate) enum Break {
    /// Line Feed, U+000A
    Lf,
    /// CR (U+000D) followed by LF (U+000A)
    Crlf,
    /// Vertical Tab, U+000B
    #[cfg(feature = "unicode-line-breaks")]
    Vt,
    /// Form Feed, U+000C
    #[cfg(feature = "unicode-line-breaks")]
    Ff,
    /// Carriage Return, U+000D
    #[cfg(feature = "unicode-line-breaks")]
    Cr,
    /// Next Line, U+0085
    #[cfg(feature = "unicode-line-breaks")]
    Nel,
    /// Line Separator, U+2028
    #[cfg(feature = "unicode-line-breaks")]
    Ls,
    /// Paragraph Separator, U+2029
    #[cfg(feature = "unicode-line-breaks")]
    Ps,
}

impl Break {
    const LF: &str = "\u{000A}";
    const CRLF: &str = "\u{000D}\u{000A}";
    const VT: &str = "\u{000B}";
    const FF: &str = "\u{000C}";
    const CR: &str = "\u{000D}";
    const NEL: &str = "\u{0085}";
    const LS: &str = "\u{2028}";
    const PS: &str = "\u{2029}";

    /// The amount of characters this line break takes.
    pub(crate) const fn len_chars(&self) -> usize {
        match self {
            Self::Crlf => 2,
            _ => 1,
        }
    }

    /// The amount of bytes this line break takes.
    pub(crate) const fn len_bytes(&self) -> usize {
        match self {
            Self::Lf => Self::LF.len(),
            Self::Crlf => Self::CRLF.len(),
            #[cfg(feature = "unicode-line-breaks")]
            Self::Vt => Self::VT.len(),
            #[cfg(feature = "unicode-line-breaks")]
            Self::Ff => Self::FF.len(),
            #[cfg(feature = "unicode-line-breaks")]
            Self::Cr => Self::CR.len(),
            #[cfg(feature = "unicode-line-breaks")]
            Self::Nel => Self::NEL.len(),
            #[cfg(feature = "unicode-line-breaks")]
            Self::Ls => Self::LS.len(),
            #[cfg(feature = "unicode-line-breaks")]
            Self::Ps => Self::PS.len(),
        }
    }
}
