//! Fresh saved-disk dependency selection, independent of live editor overlays.
//!
//! The scan fails closed rather than returning a truncated program. Both file
//! budgets are checked before inserting an extra path, and every selected file
//! is processed even when the closure already exactly fills its budget.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use crate::analysis::{self, SourceFile};
use crate::load;
use crate::lsp::json::Json;
use crate::lsp::source_members::SourceMembers;
use crate::lsp::source_references;
use crate::lsp::symbols::SymbolTable;
use crate::lsp::uri::{self, DocumentUri};
use crate::lsp::workspace::{excluded_directory, MAX_ANALYSIS_FILES, MAX_INDEX_FILES};
use crate::stdlib;

/// Selects canonical, entry-first source paths from the explicit saved root.
///
/// This never reads or mutates the live document store or workspace index.
/// Errors explain which input, read, import or budget prevents a complete list.
pub(super) fn select(params: &Json) -> Result<Vec<String>, String> {
    let root = file_path(params, "rootUri")?;
    let entry = file_path(params, "entryUri")?;
    if !root.is_dir() {
        return Err("rootUri must name a readable directory; select the containing folder.".into());
    }
    if !entry.starts_with(&root) || !entry.is_file() || entry.extension().is_none_or(|e| e != "clo") {
        return Err("entryUri must name a saved .clo file inside rootUri; save the entry and retry.".into());
    }
    let files = scan(&root)?;
    let key = path_text(&entry)?;
    let parsed = analysis::parse_partial(files);
    let table = SymbolTable::build(&parsed.sources, &parsed.programs);
    let members = SourceMembers::new(&parsed.programs, &table);
    let mut modules: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    let mut entries = BTreeMap::new();
    for (index, (file, program)) in parsed.programs.iter().enumerate() {
        entries.insert(*file, index);
        let module = program.module.name.to_dotted();
        if !module.is_empty() {
            modules.entry(module).or_default().push(index);
        }
    }
    let first_file = parsed.files.get(&key)
        .ok_or("Entry is excluded from the saved index; move it into the source root and retry.")?;
    let first = *entries.get(first_file).ok_or("Cannot parse the saved entry; fix its module declaration.")?;
    let inventory: BTreeSet<String> = modules.keys().cloned().collect();
    let mut selected = BTreeSet::from([first]);
    let mut queue = vec![first];
    let mut cursor = 0;
    while cursor < queue.len() {
        let index = queue[cursor];
        cursor += 1;
        let (file, program) = &parsed.programs[index];
        let mut wanted = source_references::modules(program, *file, &table, &members, &inventory);
        wanted.extend(
            load::referenced_modules(std::slice::from_ref(&parsed.programs[index])).into_keys()
        );
        let own_module = program.module.name.to_dotted();
        if !own_module.is_empty() {
            wanted.insert(own_module);
        }
        for import in &program.imports {
            let name = import.from.to_dotted();
            if !modules.contains_key(&name) && stdlib::lookup(&name).is_none() {
                return Err(format!("Missing imported module '{name}'; save its sources inside rootUri and retry."));
            }
            wanted.insert(name);
        }
        for name in wanted {
            if let Some(paths) = modules.get(&name) {
                for &next in paths {
                    if selected.contains(&next) {
                        continue;
                    }
                    if selected.len() == MAX_ANALYSIS_FILES {
                        return Err(format!("Saved selection exceeds {MAX_ANALYSIS_FILES} files; reduce the program's module closure."));
                    }
                    selected.insert(next);
                    queue.push(next);
                }
            }
        }
    }
    let mut tail = BTreeSet::new();
    for index in selected {
        let file = parsed.programs[index].0;
        let text = parsed.sources.path(file).to_owned();
        if text != key {
            tail.insert(text);
        }
    }
    let mut result = vec![key];
    result.extend(tail);
    Ok(result)
}

/// Decode and canonicalize a required file URI before any containment decision.
fn file_path(params: &Json, key: &str) -> Result<PathBuf, String> {
    let value = params.get(key).and_then(Json::as_str)
        .ok_or_else(|| format!("{key} must be a file URI; save the entry and select its source folder."))?;
    let path = match uri::parse(value)? {
        DocumentUri::File(path) if path.is_absolute() => path,
        _ => return Err(format!("{key} must name an absolute saved file URI.")),
    };
    path.canonicalize().map_err(|error| format!("Cannot resolve {key}: {error}; check the path and permissions."))
}

/// Avoid lossy path spellings that could name different files in a CLI argument.
fn path_text(path: &Path) -> Result<String, String> {
    path.to_str().map(str::to_owned)
        .ok_or_else(|| "Saved source paths must be valid UTF-8; rename the file and retry.".into())
}

/// Enumerate directories incrementally; never retain an over-limit source list.
fn scan(root: &Path) -> Result<Vec<SourceFile>, String> {
    let read_error = |error| format!("Cannot read the complete source index: {error}; check file permissions and retry.");
    let mut stack = vec![std::fs::read_dir(root).map_err(&read_error)?];
    let mut files = Vec::new();
    while let Some(directory) = stack.last_mut() {
        let Some(next) = directory.next() else {
            stack.pop();
            continue;
        };
        let entry = next.map_err(&read_error)?;
        let kind = entry.file_type().map_err(&read_error)?;
        let path = entry.path();
        if kind.is_symlink() {
            // As in the live index, symlinked directories/files are not indexed.
            // A selected import that exists only through one therefore fails.
            continue;
        }
        if kind.is_dir() {
            if !entry.file_name().to_str().is_some_and(excluded_directory) {
                stack.push(std::fs::read_dir(&path).map_err(&read_error)?);
            }
        } else if kind.is_file() && path.extension().is_some_and(|e| e == "clo") {
            if files.len() == MAX_INDEX_FILES {
                return Err(format!("Saved index exceeds {MAX_INDEX_FILES} source files; reduce the source root."));
            }
            let canonical = path.canonicalize().map_err(&read_error)?;
            if !canonical.starts_with(root) {
                return Err("A saved source escapes rootUri; move it inside the source root and retry.".into());
            }
            let text = std::fs::read_to_string(&canonical).map_err(&read_error)?;
            files.push(SourceFile { path: path_text(&canonical)?, text });
        }
    }
    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(files)
}
