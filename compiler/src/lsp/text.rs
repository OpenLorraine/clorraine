//! Line and UTF-16 position conversion for LSP ranges.
//!
//! LSP positions are zero-based line and UTF-16 code-unit columns. The
//! compiler works in byte offsets. This module owns the single conversion
//! between the two so every feature reports the same range for the same span.

use crate::span::{FileId, Span};

/// A zero-based protocol position.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Position {
    /// Zero-based line.
    pub line: u32,
    /// Zero-based UTF-16 code-unit column.
    pub character: u32,
}

/// A protocol range.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Range {
    /// Inclusive start.
    pub start: Position,
    /// Exclusive end.
    pub end: Position,
}

/// Byte offsets at which each line starts.
pub struct LineIndex {
    starts: Vec<usize>,
}

impl LineIndex {
    /// Builds the index for one text.
    pub fn new(text: &str) -> Self {
        let mut starts = vec![0];
        for (offset, byte) in text.bytes().enumerate() {
            if byte == b'\n' {
                starts.push(offset + 1);
            }
        }
        Self { starts }
    }

    /// Converts a protocol position to a byte offset.
    ///
    /// A line past the end clamps to the last line. A column past the line end
    /// clamps to the line end. A column inside a surrogate pair moves to the
    /// start of that code point, which is the only position that can be
    /// represented.
    pub fn offset(&self, text: &str, position: Position) -> Option<usize> {
        let line = (position.line as usize).min(self.starts.len().saturating_sub(1));
        let start = *self.starts.get(line)?;
        let end = self.line_end(text, line);
        let mut offset = start;
        let mut units = 0u32;
        for character in text[start..end].chars() {
            let width = character.len_utf16() as u32;
            if units + width > position.character {
                return Some(offset);
            }
            units += width;
            offset += character.len_utf8();
            if units == position.character {
                return Some(offset);
            }
        }
        Some(end)
    }

    /// Converts a byte offset to a protocol position.
    pub fn position(&self, text: &str, offset: usize) -> Position {
        let offset = offset.min(text.len());
        let line = match self.starts.binary_search(&offset) {
            Ok(index) => index,
            Err(index) => index.saturating_sub(1),
        };
        let start = self.starts[line];
        let character = text[start..offset].chars().map(|c| c.len_utf16() as u32).sum();
        Position {
            line: line as u32,
            character,
        }
    }

    /// Converts a compiler span to a protocol range.
    pub fn range(&self, text: &str, span: Span) -> Range {
        Range {
            start: self.position(text, span.start as usize),
            end: self.position(text, span.end as usize),
        }
    }

    /// Byte offset of the end of a line, excluding its newline.
    fn line_end(&self, text: &str, line: usize) -> usize {
        match self.starts.get(line + 1) {
            Some(next) => {
                let mut end = *next;
                if end > 0 && text.as_bytes().get(end - 1) == Some(&b'\n') {
                    end -= 1;
                }
                if end > 0 && text.as_bytes().get(end - 1) == Some(&b'\r') {
                    end -= 1;
                }
                end
            }
            None => text.len(),
        }
    }
}

/// A range for a span in a document, keeping the file identity.
pub fn span_range(index: &LineIndex, text: &str, file: FileId, span: Span) -> (FileId, Range) {
    (file, index.range(text, span))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_convert_lines_and_ascii_columns() {
        let text = "let x = 1;\nlet y = 2;\n";
        let index = LineIndex::new(text);
        assert_eq!(
            index.position(text, 0),
            Position {
                line: 0,
                character: 0
            }
        );
        assert_eq!(
            index.offset(
                text,
                Position {
                    line: 1,
                    character: 4
                }
            ),
            Some(15)
        );
        assert_eq!(
            index.position(text, 15),
            Position {
                line: 1,
                character: 4
            }
        );
    }

    #[test]
    fn should_count_utf16_units_not_characters() {
        // U+1F600 is one character and two UTF-16 units.
        let text = "let x = \"😀\";";
        let index = LineIndex::new(text);
        let quote = text.find('😀').expect("emoji");
        assert_eq!(
            index.position(text, quote),
            Position {
                line: 0,
                character: 9
            }
        );
        assert_eq!(
            index.offset(
                text,
                Position {
                    line: 0,
                    character: 11
                }
            ),
            Some(quote + 4)
        );
        // A column inside the pair clamps to the character start.
        assert_eq!(
            index.offset(
                text,
                Position {
                    line: 0,
                    character: 10
                }
            ),
            Some(quote)
        );
    }

    #[test]
    fn should_clamp_a_line_and_column_past_the_end() {
        let text = "a\nbc\n";
        let index = LineIndex::new(text);
        assert_eq!(
            index.offset(
                text,
                Position {
                    line: 99,
                    character: 99
                }
            ),
            Some(text.len())
        );
        assert_eq!(
            index.offset(
                text,
                Position {
                    line: 0,
                    character: 99
                }
            ),
            Some(1)
        );
    }

    #[test]
    fn should_exclude_crlf_from_a_line_end() {
        let text = "a\r\nb\r\n";
        let index = LineIndex::new(text);
        assert_eq!(
            index.offset(
                text,
                Position {
                    line: 0,
                    character: 99
                }
            ),
            Some(1)
        );
    }
}
