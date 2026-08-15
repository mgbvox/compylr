//! Source locations, kept independent of the parser's types.
//!
//! Diagnostics need to point at the offending construct, but the IR must not borrow from the
//! parsed source or leak ruff types into its public shape. A [`Span`] is therefore a plain pair
//! of byte offsets, converted from ruff's `TextRange` at the frontend boundary. Rendering it as
//! `line:column` needs the source text, so that is a separate step rather than a field.

use std::fmt;

use ruff_source_file::LineIndex;
use ruff_text_size::{TextRange, TextSize};

/// A half-open byte range `[start, end)` into a Python source string.
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
    pub fn line_column(self, source: &str) -> LineColumn {
        let index = LineIndex::from_source_text(source);
        let offset = TextSize::from(self.start.min(source.len() as u32));
        let location = index.line_column(offset, source);
        LineColumn {
            line: location.line.get(),
            column: location.column.get(),
        }
    }
}

impl From<TextRange> for Span {
    fn from(range: TextRange) -> Self {
        Self::new(range.start().to_u32(), range.end().to_u32())
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
    fn from_text_range_preserves_offsets() {
        let range = TextRange::new(TextSize::from(3u32), TextSize::from(9u32));
        let span = Span::from(range);
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
    fn display_renders_byte_range() {
        assert_eq!(Span::new(2, 8).to_string(), "bytes 2..8");
        assert_eq!(LineColumn { line: 4, column: 9 }.to_string(), "4:9");
    }
}
