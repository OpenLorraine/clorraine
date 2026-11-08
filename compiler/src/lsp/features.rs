//! Hover, goto definition, completion, and document symbols.
//!
//! Every feature works from the parsed analysis set: no type checker is
//! required, no C compiler is invoked, and no symbol is re-derived from an
//! unchecked name. A name that resolves to nothing returns `null` rather than
//! a guess.

use std::collections::BTreeSet;

use crate::lsp::json::Json;
use crate::lsp::symbols::{dotted_at, word_at, Symbol, SymbolTable};
use crate::lsp::text::{LineIndex, Position, Range};
use crate::lsp::uri;
use crate::span::{FileId, SourceMap, Span};

/// The parsed inputs one request needs.
pub struct FeatureInput {
    /// All parsed sources in the analysis set.
    pub sources: SourceMap,
    /// The requested document.
    pub file: FileId,
    /// The declaration table for the set.
    pub table: SymbolTable,
}

impl FeatureInput {
    /// Source text of the requested document.
    pub fn text(&self) -> &str {
        self.sources.text(self.file)
    }

    /// Byte offset for a protocol position.
    pub fn offset(&self, position: Position) -> Option<usize> {
        LineIndex::new(self.text()).offset(self.text(), position)
    }

    /// Resolves the symbol a position names.
    fn symbol_at(&self, offset: u32) -> Option<&Symbol> {
        if let Some(symbol) = self.table.at(self.file, offset) {
            return Some(symbol);
        }
        let (_, word) = word_at(self.text(), offset as usize)?;
        if let Some(symbol) = self.table.local_at(self.file, &word, offset) {
            return Some(symbol);
        }
        let module = self.module_of_file();
        if let Some(symbol) = self.table.declaration(&module, &word) {
            return Some(symbol);
        }
        let import = self.table.import_in_file(self.file, &word)?;
        self.table
            .declaration(&import.module, &import.original)
    }

    /// The exact token span at, or immediately before, `offset`.
    fn token_span(&self, offset: u32) -> Option<Span> {
        let (start, word) = word_at(self.text(), offset as usize)?;
        Some(Span::new(self.file, start, start + word.len()))
    }

    /// The module declared by the requested file.
    fn module_of_file(&self) -> String {
        self.table
            .file_declarations(self.file)
            .next()
            .map(|symbol| symbol.module.clone())
            .unwrap_or_default()
    }
}

/// Returns the hover result for a position.
pub fn hover(input: &FeatureInput, position: Position) -> Option<Json> {
    let offset = input.offset(position)? as u32;
    let token = input.token_span(offset);
    if let Some(symbol) = input.symbol_at(offset) {
        return Some(symbol_hover(input, symbol, token));
    }
    let dotted = dotted_at(input.text(), offset as usize)?;
    if input.table.module(&dotted).is_some() {
        let mut fields = vec![(
            "contents",
            Json::object([
                ("kind", Json::String("markdown".to_owned())),
                (
                    "value",
                    Json::String(format!("```clorraine\nmodule {dotted}\n```\n\nmodule")),
                ),
            ]),
        )];
        if let Some(span) = token {
            fields.push(("range", range_json(input.token_range(span))));
        }
        return Some(Json::from_pairs(fields));
    }
    let mut segments: Vec<&str> = dotted.split('.').collect();
    let name = segments.pop()?;
    let module = segments.join(".");
    let symbol = input.table.declaration(&module, name)?;
    Some(symbol_hover(input, symbol, token))
}

/// Builds the hover JSON for one symbol.
fn symbol_hover(input: &FeatureInput, symbol: &Symbol, token: Option<Span>) -> Json {
    let signature = symbol.signature(&input.sources);
    let value = format!(
        "```clorraine\n{signature}\n```\n\n{} in module `{}`",
        symbol.kind.label(),
        symbol.module
    );
    let mut fields = vec![(
        "contents",
        Json::object([
            ("kind", Json::String("markdown".to_owned())),
            ("value", Json::String(value)),
        ]),
    )];
    if let Some(span) = token.or_else(|| (symbol.file == input.file).then_some(symbol.name_span)) {
        fields.push(("range", range_json(input.token_range(span))));
    }
    Json::from_pairs(fields)
}

/// Returns the definition location for a position.
pub fn definition(input: &FeatureInput, position: Position) -> Option<Json> {
    let offset = input.offset(position)? as u32;
    if let Some(symbol) = input.symbol_at(offset) {
        return location(input, symbol.file, symbol.name_span);
    }
    let dotted = dotted_at(input.text(), offset as usize)?;
    if let Some(module) = input.table.module(&dotted) {
        return location(input, module.file, module.name_span);
    }
    let mut segments: Vec<&str> = dotted.split('.').collect();
    let name = segments.pop()?;
    let module = segments.join(".");
    let symbol = input.table.declaration(&module, name)?;
    location(input, symbol.file, symbol.name_span)
}

/// Builds a location for a span in a file.
fn location(input: &FeatureInput, file: FileId, span: Span) -> Option<Json> {
    let text = input.sources.text(file);
    let path = input.sources.path(file);
    // An embedded standard-library buffer has no on-disk location.
    if path.starts_with("stdlib/") {
        return None;
    }
    let range = LineIndex::new(text).range(text, span);
    Some(Json::object([
        ("uri", Json::String(uri::from_path(std::path::Path::new(path)))),
        ("range", range_json(range)),
    ]))
}

/// Returns the completion list.
pub fn completion(input: &FeatureInput, modules: &BTreeSet<String>) -> Json {
    let keywords = [
        "borrow", "break", "case", "comptime", "const", "continue", "default", "do",
        "else", "enum", "export", "false", "ffi", "for", "freedBy", "from", "func",
        "handle", "if", "import", "in", "intrinsic", "let", "module", "null", "raw",
        "return", "static", "struct", "switch", "take", "true", "unsafe", "volatile",
        "while",
    ];
    let types = [
        "Array", "Box", "Buffer", "String", "bool", "char", "cstring", "f32", "f64",
        "i8", "i16", "i32", "i64", "isize", "raw", "str", "u8", "u16", "u32", "u64",
        "usize", "void",
    ];
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut entries: Vec<(String, i64, String)> = Vec::new();
    let mut push = |label: String, kind: i64, detail: String| {
        if seen.insert(label.clone()) {
            entries.push((label, kind, detail));
        }
    };
    for keyword in keywords {
        push(keyword.to_owned(), 14, "keyword".to_owned());
    }
    for name in types {
        push(name.to_owned(), 25, "built-in type".to_owned());
    }
    let module = input.module_of_file();
    let declarations: Vec<&Symbol> = if module.is_empty() {
        input.table.file_declarations(input.file).collect()
    } else {
        input.table.module_declarations(&module).collect()
    };
    for symbol in declarations {
        push(
            symbol.name.clone(),
            symbol.kind.lsp(),
            format!("{} in module `{}`", symbol.kind.label(), symbol.module),
        );
    }
    for import in &input.table.imports {
        if import.file == input.file {
            push(
                import.local.clone(),
                3,
                format!("import from `{}`", import.module),
            );
        }
    }
    for name in modules.iter().chain(input.table.module_names()) {
        push(name.clone(), 9, "module".to_owned());
    }
    entries.sort_by(|left, right| left.0.cmp(&right.0));
    Json::Array(
        entries
            .into_iter()
            .map(|(label, kind, detail)| {
                Json::object([
                    ("label", Json::String(label)),
                    ("kind", Json::Number(kind)),
                    ("detail", Json::String(detail)),
                ])
            })
            .collect(),
    )
}

/// Returns the document symbols for the requested file.
pub fn document_symbols(input: &FeatureInput) -> Json {
    let text = input.text();
    let index = LineIndex::new(text);
    let mut symbols: Vec<&Symbol> = input.table.file_declarations(input.file).collect();
    symbols.sort_by_key(|symbol| symbol.full_span.start);
    let entries: Vec<Json> = symbols
        .into_iter()
        .map(|symbol| {
            Json::object([
                ("name", Json::String(symbol.name.clone())),
                ("kind", Json::Number(symbol.kind.lsp())),
                ("range", range_json(index.range(text, symbol.full_span))),
                (
                    "selectionRange",
                    range_json(index.range(text, symbol.name_span)),
                ),
            ])
        })
        .collect();
    Json::Array(entries)
}

impl FeatureInput {
    /// Resolves a span in the requested document to a range.
    fn token_range(&self, span: Span) -> Range {
        LineIndex::new(self.text()).range(self.text(), span)
    }
}

/// Renders a protocol range.
fn range_json(range: Range) -> Json {
    Json::object([
        ("start", position_json(range.start)),
        ("end", position_json(range.end)),
    ])
}

/// Renders a protocol position.
fn position_json(position: Position) -> Json {
    Json::object([
        ("line", Json::Number(position.line as i64)),
        ("character", Json::Number(position.character as i64)),
    ])
}
