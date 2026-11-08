//! Markdown generation from `///` documentation comments.
//!
//! The generator is deliberately parse-only: it reads the listed files, uses
//! the compiler parser for declarations and spans, and slices the source for
//! exact signatures. It never checks types, emits C, or needs the SDK.
//!
//! Output is deterministic: modules are sorted, declarations stay in source
//! order, line endings are LF, and no timestamp or absolute path is written.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::analysis::{self, SourceFile};
use crate::ast::{Item, Type, TypeKind};
use crate::clorraine::cli::DocOptions;
use crate::span::{FileId, SourceMap, Span};

/// One documented declaration.
struct DocItem {
    /// Exact identifier.
    name: String,
    /// Extracted documentation text.
    doc: Option<String>,
    /// Exact signature text.
    signature: String,
    /// Named types used by the declaration.
    related: Vec<String>,
}

/// One documented module.
struct DocModule {
    /// Extracted module documentation.
    doc: Option<String>,
    /// Declarations in source order.
    items: Vec<DocItem>,
}

/// Runs the generator and returns the process exit code.
pub fn run(options: &DocOptions) -> i32 {
    let mut inputs = Vec::new();
    for path in &options.files {
        match std::fs::read_to_string(path) {
            Ok(text) => inputs.push(SourceFile {
                path: path.clone(),
                text,
            }),
            Err(error) => {
                eprintln!("docgen: cannot read '{path}': {error}");
                return 1;
            }
        }
    }
    let parsed = analysis::parse(inputs);
    if parsed.diagnostics.exit_code() != 0 {
        eprint!("{}", parsed.diagnostics.render(&parsed.sources));
        return 1;
    }
    let modules = collect_modules(&parsed.sources, &parsed.programs);
    if modules.is_empty() {
        eprintln!("docgen: no modules to document");
        return 1;
    }
    let root = PathBuf::from(options.output.clone().unwrap_or_else(|| "build/doc".to_owned()));
    if let Err(error) = std::fs::create_dir_all(&root) {
        eprintln!("docgen: cannot create '{}': {error}", root.display());
        return 2;
    }
    let root = match std::fs::canonicalize(&root) {
        Ok(root) => root,
        Err(error) => {
            eprintln!("docgen: cannot resolve '{}': {error}", root.display());
            return 2;
        }
    };
    let rendered = render(&root, &modules);
    for (path, text) in rendered {
        if let Err(error) = std::fs::write(&path, text) {
            eprintln!("docgen: cannot write '{}': {error}", path.display());
            return 1;
        }
    }
    println!("{}", root.display());
    0
}

/// Groups parsed items by module, preserving command-line order.
fn collect_modules(
    sources: &SourceMap,
    programs: &[(FileId, crate::ast::Program)],
) -> BTreeMap<String, DocModule> {
    let mut modules: BTreeMap<String, DocModule> = BTreeMap::new();
    for (file, program) in programs {
        let name = program.module.name.to_dotted();
        if !valid_module(&name) {
            continue;
        }
        let module = modules.entry(name.clone()).or_insert(DocModule {
            doc: None,
            items: Vec::new(),
        });
        if module.doc.is_none() {
            module.doc = doc_comment(sources.text(*file), program.module.span.start as usize);
        }
        for item in &program.items {
            if let Some(entry) = item_docs(sources, *file, item) {
                module.items.push(entry);
            }
        }
    }
    modules
}

/// Builds one declaration's documentation.
fn item_docs(sources: &SourceMap, file: FileId, item: &Item) -> Option<DocItem> {
    let (name, span, signature_end, types, exported) = item_shape(sources, item)?;
    let signature = signature_text(sources, file, span, signature_end, exported);
    Some(DocItem {
        name,
        doc: doc_comment(sources.text(file), span.start as usize),
        signature,
        related: types,
    })
}

/// Returns the name, full span, signature end, and named types of an item.
fn item_shape(
    sources: &SourceMap,
    item: &Item,
) -> Option<(String, Span, u32, Vec<String>, bool)> {
    let mut types = Vec::new();
    match item {
        Item::Func(decl) => {
            for param in &decl.params {
                collect_type_names(&param.ty, &mut types);
            }
            collect_type_names(&decl.returns, &mut types);
            Some((decl.name.clone(), decl.span, decl.body.span.start, types, decl.exported))
        }
        Item::Comptime(decl) => {
            for param in &decl.params {
                collect_type_names(&param.ty, &mut types);
            }
            collect_type_names(&decl.returns, &mut types);
            Some((decl.name.clone(), decl.span, decl.body.span.start, types, decl.exported))
        }
        Item::Struct(decl) => {
            for field in &decl.fields {
                collect_type_names(&field.ty, &mut types);
            }
            Some((
                decl.name.clone(),
                decl.span,
                cut_at_brace(sources, decl.span),
                types,
                decl.exported,
            ))
        }
        Item::Enum(decl) => {
            collect_type_names(&decl.backing, &mut types);
            Some((
                decl.name.clone(),
                decl.span,
                cut_at_brace(sources, decl.span),
                types,
                decl.exported,
            ))
        }
        Item::Handle(decl) => {
            types.push(decl.freed_by.to_dotted());
            Some((decl.name.clone(), decl.span, decl.span.end, types, decl.exported))
        }
        Item::Ffi(decl) => Some((
            decl.name.clone(),
            decl.span,
            cut_at_brace(sources, decl.span),
            types,
            decl.exported,
        )),
        Item::Intrinsic(decl) => {
            for param in &decl.params {
                collect_type_names(&param.ty, &mut types);
            }
            collect_type_names(&decl.returns, &mut types);
            Some((decl.name.clone(), decl.span, decl.span.end, types, false))
        }
        Item::GlobalLet(decl) => {
            collect_type_names(&decl.ty, &mut types);
            Some((decl.name.clone(), decl.span, decl.span.end, types, decl.exported))
        }
        Item::GlobalConst(decl) => {
            collect_type_names(&decl.ty, &mut types);
            Some((decl.name.clone(), decl.span, decl.span.end, types, decl.exported))
        }
    }
}

/// Collects named types from a type expression.
fn collect_type_names(ty: &Type, out: &mut Vec<String>) {
    match &ty.kind {
        TypeKind::Named { name, args } => {
            if let Some(last) = name.segments.last() {
                out.push(last.clone());
            }
            for argument in args {
                collect_type_names(argument, out);
            }
        }
        TypeKind::Raw(inner) => collect_type_names(inner, out),
        TypeKind::Array { element, .. } => collect_type_names(element, out),
        TypeKind::Function { params, returns } => {
            for param in params {
                collect_type_names(&param.ty, out);
            }
            collect_type_names(returns, out);
        }
        TypeKind::Void
        | TypeKind::Scalar(_)
        | TypeKind::Str
        | TypeKind::CString
        | TypeKind::StaticStr
        | TypeKind::Error => {}
    }
}

/// Returns the offset of the first `{`, or the end of the span.
fn cut_at_brace(sources: &SourceMap, span: Span) -> u32 {
    sources
        .snippet(span)
        .find('{')
        .map(|index| span.start + index as u32)
        .unwrap_or(span.end)
}

/// Slices and normalizes a declaration signature.
fn signature_text(
    sources: &SourceMap,
    file: FileId,
    span: Span,
    end: u32,
    exported: bool,
) -> String {
    let end = end.max(span.start).min(span.end);
    let text = sources.snippet(Span::new(file, span.start as usize, end as usize));
    let text = text
        .trim()
        .trim_end_matches('{')
        .trim()
        .trim_end_matches(';')
        .trim();
    if exported {
        format!("export {text}")
    } else {
        text.to_owned()
    }
}

/// Extracts the contiguous `///` run immediately above `start`.
fn doc_comment(text: &str, start: usize) -> Option<String> {
    let mut lines = Vec::new();
    let mut cursor = text[..start.min(text.len())]
        .rfind('\n')
        .map(|index| index + 1)
        .unwrap_or(0);
    while cursor > 0 {
        let previous_end = cursor - 1;
        let previous_start = text[..previous_end]
            .rfind('\n')
            .map(|index| index + 1)
            .unwrap_or(0);
        let line = &text[previous_start..previous_end];
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix("///") {
            lines.push(rest.strip_prefix(' ').unwrap_or(rest).to_owned());
            cursor = previous_start;
        } else {
            break;
        }
    }
    if lines.is_empty() {
        None
    } else {
        lines.reverse();
        Some(lines.join("\n"))
    }
}

/// Validates a dotted module name before it becomes a file name.
fn valid_module(name: &str) -> bool {
    !name.is_empty()
        && name.split('.').all(|segment| {
            let mut characters = segment.chars();
            matches!(characters.next(), Some(first) if first.is_ascii_alphabetic() || first == '_')
                && characters.all(|character| character.is_ascii_alphanumeric() || character == '_')
        })
}

/// Renders every page and returns its path and text.
fn render(root: &Path, modules: &BTreeMap<String, DocModule>) -> Vec<(PathBuf, String)> {
    let mut type_index: BTreeMap<String, (String, String)> = BTreeMap::new();
    for (module_name, module) in modules {
        let mut used: BTreeMap<String, usize> = BTreeMap::new();
        for item in &module.items {
            let base = anchor(&item.name);
            let count = used.entry(base.clone()).or_insert(0);
            *count += 1;
            let value = if *count == 1 {
                base.clone()
            } else {
                format!("{base}-{count}")
            };
            type_index
                .entry(item.name.clone())
                .or_insert((module_name.clone(), value));
        }
    }
    let mut output = Vec::new();
    let mut index = String::from("# Clorraine API\n\n");
    for (module_name, module) in modules {
        let summary = module
            .doc
            .as_deref()
            .and_then(|doc| doc.lines().find(|line| !line.trim().is_empty()))
            .unwrap_or("No module documentation.");
        index.push_str(&format!("- [{module_name}]({module_name}.md) — {summary}\n"));
    }
    output.push((root.join("index.md"), index));
    for (module_name, module) in modules {
        output.push((
            root.join(format!("{module_name}.md")),
            render_module(module_name, module, &type_index),
        ));
    }
    output
}

/// Renders one module page.
fn render_module(
    name: &str,
    module: &DocModule,
    type_index: &BTreeMap<String, (String, String)>,
) -> String {
    let mut page = format!("# {name}\n\n");
    if let Some(doc) = &module.doc {
        page.push_str(doc);
        page.push_str("\n\n");
    }
    for item in &module.items {
        page.push_str(&format!("## {}\n\n", item.name));
        page.push_str("```clorraine\n");
        page.push_str(&item.signature);
        page.push_str("\n```\n\n");
        if let Some(doc) = &item.doc {
            page.push_str(doc);
            page.push_str("\n\n");
        }
        let mut related: Vec<String> = Vec::new();
        let mut seen = std::collections::BTreeSet::new();
        for type_name in &item.related {
            if type_name == &item.name || !seen.insert(type_name.clone()) {
                continue;
            }
            let Some((target_module, target_anchor)) = type_index.get(type_name) else {
                continue;
            };
            let link = if target_module == name {
                format!("[{}](#{})", type_name, target_anchor)
            } else {
                format!(
                    "[{}]({}.md#{})",
                    type_name, target_module, target_anchor
                )
            };
            related.push(link);
        }
        if !related.is_empty() {
            page.push_str("**Related:** ");
            page.push_str(&related.join(", "));
            page.push_str("\n\n");
        }
    }
    page.truncate(page.trim_end().len());
    page.push('\n');
    page
}

/// Lowercases an identifier and replaces other bytes with `-`.
fn anchor(name: &str) -> String {
    let value: String = name
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '_' || character == '-' {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    let value = value.trim_matches('-').to_owned();
    if value.is_empty() {
        "item".to_owned()
    } else {
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_attach_a_contiguous_doc_run() {
        let text = "/// one\n/// two\nfunc f(): void {}\n";
        let start = text.find("func").expect("func");
        assert_eq!(doc_comment(text, start).as_deref(), Some("one\ntwo"));
    }

    #[test]
    fn should_stop_at_a_blank_line() {
        let text = "/// orphan\n\nfunc f(): void {}\n";
        let start = text.find("func").expect("func");
        assert_eq!(doc_comment(text, start), None);
    }

    #[test]
    fn should_accept_a_valid_dotted_module_name() {
        assert!(valid_module("app.core"));
        assert!(valid_module("_app.io_2"));
        assert!(!valid_module(""));
        assert!(!valid_module(".."));
        assert!(!valid_module("a/b"));
        assert!(!valid_module(".leading"));
        assert!(!valid_module("trailing."));
    }

    #[test]
    fn should_lowercase_and_slug_an_anchor() {
        assert_eq!(anchor("Entry"), "entry");
        assert_eq!(anchor("a_b"), "a_b");
        assert_eq!(anchor("--"), "item");
    }
}
