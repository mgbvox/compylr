//! Source locations, kept independent of any parser's types.
//!
//! Diagnostics need to point at the offending construct, but the IR must not borrow from the
//! parsed source or leak a parser's types into its public shape. A [`Span`] is therefore a plain
//! pair of byte offsets, converted from whatever range type a frontend's parser uses at the
//! frontend boundary. Rendering it as `line:column` needs the source text, so that is a separate
//! step rather than a field.
//!
//! Resolving a line and column is written out here rather than delegated to a parser's line
//! index, because this crate is below the IR: a dependency added here reaches every crate in the
//! workspace, and a source-language parser is the one thing that must not.

use std::fmt;

/// A half-open byte range `[start, end)` into a source string.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct Span {
    start: u32,
    end: u32,
}

impl Span {
    /// Create a span from raw byte offsets.
    ///
    /// The range is normalised so that `start <= end`, which keeps later arithmetic total.
    pub fn new(start: u32, end: u32) -> Self {
        if start <= end {
            Self { start, end }
        } else {
            Self {
                start: end,
                end: start,
            }
        }
    }

    /// Byte offset of the first character.
    pub fn start(self) -> u32 {
        self.start
    }

    /// Byte offset one past the last character.
    pub fn end(self) -> u32 {
        self.end
    }

    /// Number of bytes covered.
    pub fn len(self) -> u32 {
        self.end - self.start
    }

    /// Whether the span covers no bytes.
    pub fn is_empty(self) -> bool {
        self.start == self.end
    }

    /// Render as `line:column`, both 1-based, resolving offsets against `source`.
    ///
    /// Column counts UTF-8 characters rather than bytes, so a position after a multi-byte
    /// character reads the way a person counting columns in an editor would expect.
    /// A span that outruns the text is clamped to its end rather than panicking: a diagnostic
    /// arriving with a stale offset should still be readable, and a compiler that aborts while
    /// reporting an error has replaced a message with nothing.
    pub fn line_column(self, source: &str) -> LineColumn {
        let mut offset = (self.start as usize).min(source.len());
        while offset > 0 && !source.is_char_boundary(offset) {
            offset -= 1;
        }
        let before = &source[..offset];
        let line = before.bytes().filter(|byte| *byte == b'\n').count() + 1;
        let line_start = before.rfind('\n').map_or(0, |index| index + 1);
        let column = source[line_start..offset].chars().count() + 1;
        LineColumn { line, column }
    }
}

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "bytes {}..{}", self.start, self.end)
    }
}

/// A 1-based line and column pair, produced by [`Span::line_column`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LineColumn {
    /// 1-based line number.
    pub line: usize,
    /// 1-based column number, counted in characters.
    pub column: usize,
}

impl fmt::Display for LineColumn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.line, self.column)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_offsets_are_preserved() {
        let span = Span::new(3, 9);
        assert_eq!(span.start(), 3);
        assert_eq!(span.end(), 9);
        assert_eq!(span.len(), 6);
        assert!(!span.is_empty());
    }

    #[test]
    fn reversed_bounds_are_normalised() {
        let span = Span::new(9, 3);
        assert_eq!(span.start(), 3);
        assert_eq!(span.end(), 9);
    }

    #[test]
    fn spans_compare_structurally() {
        assert_eq!(Span::new(1, 4), Span::new(1, 4));
        assert_ne!(Span::new(1, 4), Span::new(1, 5));
    }

    #[test]
    fn empty_span_reports_empty() {
        assert!(Span::new(7, 7).is_empty());
        assert_eq!(Span::new(7, 7).len(), 0);
    }

    #[test]
    fn line_column_on_first_line() {
        let source = "def f():\n    pass\n";
        assert_eq!(
            Span::new(0, 3).line_column(source),
            LineColumn { line: 1, column: 1 }
        );
        assert_eq!(
            Span::new(4, 5).line_column(source),
            LineColumn { line: 1, column: 5 }
        );
    }

    #[test]
    fn line_column_on_later_line() {
        let source = "a = 1\nb = 2\nc = 3\n";
        // Offset 12 is the start of the third line.
        let position = Span::new(12, 13).line_column(source);
        assert_eq!(position, LineColumn { line: 3, column: 1 });
    }

    #[test]
    fn line_column_counts_characters_not_bytes() {
        // "é" is two bytes, so a byte-based column would report 3 instead of 2.
        let source = "é = 1\n";
        let position = Span::new(2, 3).line_column(source);
        assert_eq!(position, LineColumn { line: 1, column: 2 });
    }

    #[test]
    fn line_column_clamps_past_end_of_source() {
        let source = "x\n";
        // Should not panic even if a span outruns the text.
        let _ = Span::new(500, 600).line_column(source);
    }

    #[test]
    fn line_column_never_splits_a_character() {
        // Offset 1 is inside the two-byte "é"; resolving it must not panic on a slice.
        let source = "é = 1\n";
        assert_eq!(
            Span::new(1, 2).line_column(source),
            LineColumn { line: 1, column: 1 }
        );
    }

    #[test]
    fn line_column_on_the_final_newline() {
        let source = "a = 1\nb = 2\n";
        assert_eq!(
            Span::new(11, 12).line_column(source),
            LineColumn { line: 2, column: 6 }
        );
    }

    #[test]
    fn display_renders_byte_range() {
        assert_eq!(Span::new(2, 8).to_string(), "bytes 2..8");
        assert_eq!(LineColumn { line: 4, column: 9 }.to_string(), "4:9");
    }
}
