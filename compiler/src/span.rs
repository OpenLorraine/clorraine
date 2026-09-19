//! Source files, byte spans, and line/column resolution.
//!
//! Every loaded file is registered in a [`SourceMap`] and referred to through
//! [`Span`] values: half-open byte ranges tagged with the file they belong to.
//! Diagnostic rendering resolves those offsets to one-based line and
//! character columns.

/// Identifies one source file registered in a [`SourceMap`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FileId(u32);

/// A half-open byte range `[start, end)` inside one source file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Span {
    /// The file the range belongs to.
    pub file: FileId,
    /// Byte offset of the first covered byte.
    pub start: u32,
    /// Byte offset one past the last covered byte.
    pub end: u32,
}

impl Span {
    /// Builds a span from byte offsets.
    ///
    /// An `end` below `start` is clamped to `start`, so construction can never
    /// produce a reversed range.
    pub fn new(file: FileId, start: usize, end: usize) -> Self {
        let start = start as u32;
        let end = end.max(start as usize) as u32;
        Self { file, start, end }
    }

    /// Number of bytes covered by the span.
    pub fn len(self) -> usize {
        (self.end - self.start) as usize
    }

    /// True when the span covers no text.
    pub fn is_empty(self) -> bool {
        self.start == self.end
    }
}

/// One-based position in a source file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Location {
    /// One-based line number.
    pub line: u32,
    /// One-based column, counted in characters.
    pub column: u32,
}

/// One loaded source file and its line index.
struct SourceFile {
    path: String,
    text: String,
    line_starts: Vec<u32>,
}

/// The set of source files the compiler has loaded.
#[derive(Default)]
pub struct SourceMap {
    files: Vec<SourceFile>,
}

impl SourceMap {
    /// Creates an empty map.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers one file under its logical path (the spelling from the
    /// command line) and returns its identity.
    pub fn add_file(&mut self, path: String, text: String) -> FileId {
        let mut line_starts = vec![0];
        for (index, byte) in text.bytes().enumerate() {
            if byte == b'\n' {
                line_starts.push(index as u32 + 1);
            }
        }
        let id = FileId(self.files.len() as u32);
        self.files.push(SourceFile {
            path,
            text,
            line_starts,
        });
        id
    }

    /// Looks up a registered file.
    fn file(&self, id: FileId) -> &SourceFile {
        &self.files[id.0 as usize]
    }

    /// Source text of a file.
    pub fn text(&self, id: FileId) -> &str {
        &self.file(id).text
    }

    /// Logical path of a file, used when rendering spans.
    pub fn path(&self, id: FileId) -> &str {
        &self.file(id).path
    }

    /// Text covered by a span.
    pub fn snippet(&self, span: Span) -> &str {
        &self.text(span.file)[span.start as usize..span.end as usize]
    }

    /// Resolves a byte offset to a one-based line and character column.
    ///
    /// Offsets past the end of the file resolve to the position just after the
    /// last character.
    pub fn location(&self, id: FileId, offset: usize) -> Location {
        let file = self.file(id);
        let offset = offset.min(file.text.len()) as u32;
        let line_index = match file.line_starts.binary_search(&offset) {
            Ok(index) => index,
            Err(index) => index - 1,
        };
        let line_start = file.line_starts[line_index] as usize;
        let column = file.text[line_start..offset as usize].chars().count() + 1;
        Location {
            line: line_index as u32 + 1,
            column: column as u32,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn span_never_reverses_and_measures_bytes() {
        let map_id = FileId(0);
        let span = Span::new(map_id, 3, 7);
        assert_eq!(span.len(), 4);
        assert!(!span.is_empty());
        let reversed = Span::new(map_id, 7, 3);
        assert_eq!((reversed.start, reversed.end), (7, 7));
        assert!(reversed.is_empty());
    }

    #[test]
    fn location_counts_lines_and_characters() {
        let mut sources = SourceMap::new();
        let id = sources.add_file("main.clo".to_owned(), "let x = 1;\nlet y = 2;\n".to_owned());
        assert_eq!(
            sources.location(id, 0),
            Location { line: 1, column: 1 }
        );
        assert_eq!(
            sources.location(id, 5),
            Location { line: 1, column: 6 }
        );
        assert_eq!(
            sources.location(id, 11),
            Location { line: 2, column: 1 }
        );
        assert_eq!(
            sources.location(id, 999),
            Location { line: 3, column: 1 }
        );
    }

    #[test]
    fn location_counts_unicode_columns_in_characters() {
        let mut sources = SourceMap::new();
        let id = sources.add_file("unicode.clo".to_owned(), "let é = 1;".to_owned());
        let byte_of_equals = sources.text(id).find('=').expect("equals sign");
        assert_eq!(
            sources.location(id, byte_of_equals),
            Location { line: 1, column: 7 }
        );
    }

    #[test]
    fn snippet_and_path_round_trip() {
        let mut sources = SourceMap::new();
        let id = sources.add_file("program.clo".to_owned(), "func main(): i32 {}".to_owned());
        let span = Span::new(id, 5, 9);
        assert_eq!(sources.snippet(span), "main");
        assert_eq!(sources.path(id), "program.clo");
    }
}
