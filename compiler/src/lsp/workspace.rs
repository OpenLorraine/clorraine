//! The language server's document store and module index.
//!
//! An editor workspace is not one program: it commonly holds independent
//! examples, libraries, and tests. The index records each file's declared
//! module and imports; a request analyzes only the requested document plus the
//! transitive closure of the modules it names, never every file at once.
//!
//! Unsaved buffers live in [`DocumentStore`] and override the on-disk text for
//! every request. The store is keyed by the URI spelling so an `untitled:`
//! buffer and a file can coexist.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use crate::analysis::{self, SourceFile};
use crate::load;
use crate::lsp::uri::{self, DocumentUri};

/// The maximum number of files the initial scan indexes.
pub const MAX_INDEX_FILES: usize = 2000;

/// The maximum number of files one analysis set contains.
pub const MAX_ANALYSIS_FILES: usize = 256;

/// Directories the workspace scan never enters.
const SKIP_DIRS: &[&str] = &[".git", "build", "target", "node_modules"];

/// Unsaved editor buffers, keyed by the URI spelling.
#[derive(Default)]
pub struct DocumentStore {
    overlays: BTreeMap<String, String>,
}

impl DocumentStore {
    /// Creates an empty store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Records a full document text.
    pub fn open(&mut self, uri: &DocumentUri, text: String) {
        self.overlays.insert(uri.key(), text);
    }

    /// Replaces a full document text.
    pub fn change(&mut self, uri: &DocumentUri, text: String) {
        self.overlays.insert(uri.key(), text);
    }

    /// Drops an overlay.
    pub fn close(&mut self, uri: &DocumentUri) {
        self.overlays.remove(&uri.key());
    }

    /// Looks up an overlay.
    pub fn text(&self, key: &str) -> Option<&str> {
        self.overlays.get(key).map(String::as_str)
    }

    /// True when a document is open.
    pub fn is_open(&self, uri: &DocumentUri) -> bool {
        self.overlays.contains_key(&uri.key())
    }

    /// Every open URI spelling.
    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.overlays.keys()
    }
}

/// One indexed file.
#[derive(Clone, Debug)]
pub struct IndexEntry {
    /// The `file:` URI.
    pub uri: String,
    /// The absolute path, when this is a file.
    pub path: Option<PathBuf>,
    /// The declared module, when the file parsed far enough to name one.
    pub module: Option<String>,
    /// Imported module names.
    pub imports: Vec<String>,
    /// Module names referenced by qualified names.
    pub references: Vec<String>,
}

impl IndexEntry {
    /// The logical key used by the analysis source map.
    pub fn key(&self) -> String {
        self.path
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| self.uri.clone())
    }
}

/// The workspace module index.
#[derive(Default)]
pub struct ModuleIndex {
    paths: BTreeMap<String, IndexEntry>,
    modules: BTreeMap<String, Vec<String>>,
    truncated: bool,
}

impl ModuleIndex {
    /// Scans a root for `*.clo` files.
    ///
    /// The scan is lexicographic, skips generated and dependency directories,
    /// never follows a directory symlink, and stops at [`MAX_INDEX_FILES`].
    /// A file that cannot be read or parsed is still indexed by path but
    /// contributes no module.
    pub fn index_root(root: &Path) -> Self {
        let mut index = Self::default();
        let mut files = Vec::new();
        let mut stack = vec![root.to_path_buf()];
        'walk: while let Some(directory) = stack.pop() {
            let Ok(entries) = std::fs::read_dir(&directory) else {
                continue;
            };
            let mut names: Vec<PathBuf> = entries
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .collect();
            names.sort();
            for path in names.into_iter().rev() {
                let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
                    continue;
                };
                let Ok(metadata) = std::fs::symlink_metadata(&path) else {
                    continue;
                };
                if metadata.file_type().is_symlink() {
                    continue;
                }
                if metadata.is_dir() {
                    if !SKIP_DIRS.contains(&name) && !name.starts_with('.') {
                        stack.push(path);
                    }
                    continue;
                }
                if metadata.is_file()
                    && path.extension().and_then(|ext| ext.to_str()) == Some("clo")
                {
                    files.push(path);
                    if files.len() >= MAX_INDEX_FILES {
                        index.truncated = true;
                        break 'walk;
                    }
                }
            }
        }
        files.sort();
        for path in files {
            let entry = index.entry_for_path(&path);
            index.insert(entry);
        }
        index
    }

    /// True when the scan hit its file cap.
    pub fn truncated(&self) -> bool {
        self.truncated
    }

    /// Every indexed entry in path order.
    pub fn entries(&self) -> impl Iterator<Item = &IndexEntry> {
        self.paths.values()
    }

    /// Looks up an entry by its logical key.
    pub fn entry(&self, key: &str) -> Option<&IndexEntry> {
        self.paths.get(key)
    }

    /// Module names present in the index.
    pub fn module_names(&self) -> impl Iterator<Item = &String> {
        self.modules.keys()
    }

    /// Files that declare a module.
    pub fn files_for_module(&self, module: &str) -> &[String] {
        self.modules
            .get(module)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    /// Builds the analysis set for one requested document.
    ///
    /// The set is the document plus the transitive closure of its imports and
    /// qualified module references. Every file that declares a module already
    /// in the set joins it, because the loader merges those files. Unsaved
    /// buffers override disk text.
    pub fn analysis_set(
        &self,
        requested: &DocumentUri,
        documents: &DocumentStore,
    ) -> Vec<SourceFile> {
        let mut keys: Vec<String> = Vec::new();
        let mut seen: BTreeSet<String> = BTreeSet::new();
        let first = requested.key();
        keys.push(first.clone());
        seen.insert(first);
        let mut cursor = 0;
        while cursor < keys.len() && keys.len() < MAX_ANALYSIS_FILES {
            let key = keys[cursor].clone();
            cursor += 1;
            let Some(entry) = self.resolved_entry(&key, documents) else {
                continue;
            };
            let mut wanted: BTreeSet<String> = entry.imports.iter().cloned().collect();
            wanted.extend(entry.references.iter().cloned());
            if let Some(module) = &entry.module {
                wanted.insert(module.clone());
            }
            for module in wanted {
                for path in self.files_for_module(&module) {
                    if keys.len() >= MAX_ANALYSIS_FILES {
                        break;
                    }
                    if seen.insert(path.clone()) {
                        keys.push(path.clone());
                    }
                }
            }
        }
        keys.into_iter()
            .filter_map(|key| self.source_for(&key, documents))
            .collect()
    }

    /// Returns the entry for a key, parsing an overlay that is not indexed.
    fn resolved_entry(
        &self,
        key: &str,
        documents: &DocumentStore,
    ) -> Option<IndexEntry> {
        if let Some(text) = documents.text(key) {
            let indexed_path = self.paths.get(key).and_then(|entry| entry.path.clone());
            let mut entry = Self::parse_entry(key, indexed_path, text);
            if let Some(indexed) = self.paths.get(key) {
                entry.uri = indexed.uri.clone();
            }
            return Some(entry);
        }
        self.paths.get(key).cloned()
    }

    /// Reads or overlays one source file.
    fn source_for(&self, key: &str, documents: &DocumentStore) -> Option<SourceFile> {
        if let Some(text) = documents.text(key) {
            return Some(SourceFile {
                path: key.to_owned(),
                text: text.to_owned(),
            });
        }
        let entry = self.paths.get(key)?;
        let path = entry.path.as_ref()?;
        let text = std::fs::read_to_string(path).ok()?;
        Some(SourceFile {
            path: key.to_owned(),
            text,
        })
    }

    /// Builds one entry for a path.
    fn entry_for_path(&self, path: &Path) -> IndexEntry {
        let key = path.display().to_string();
        let text = std::fs::read_to_string(path).unwrap_or_default();
        Self::parse_entry(&key, Some(path.to_path_buf()), &text)
    }

    /// Parses a file enough to record its module and references.
    fn parse_entry(key: &str, path: Option<PathBuf>, text: &str) -> IndexEntry {
        let parsed = analysis::parse(vec![SourceFile {
            path: key.to_owned(),
            text: text.to_owned(),
        }]);
        let mut module = None;
        let mut imports = Vec::new();
        let mut references = Vec::new();
        if let Some((_, program)) = parsed.programs.first() {
            let name = program.module.name.to_dotted();
            if !name.is_empty() {
                module = Some(name);
            }
            imports = program
                .imports
                .iter()
                .map(|import| import.from.to_dotted())
                .collect();
            references = load::referenced_modules(&parsed.programs)
                .into_keys()
                .collect();
        }
        IndexEntry {
            uri: path
                .as_ref()
                .map(|path| uri::from_path(path))
                .unwrap_or_else(|| key.to_owned()),
            path,
            module,
            imports,
            references,
        }
    }

    /// Adds one entry to both maps.
    fn insert(&mut self, entry: IndexEntry) {
        let key = entry.key();
        if let Some(module) = &entry.module {
            self.modules
                .entry(module.clone())
                .or_default()
                .push(key.clone());
        }
        self.paths.insert(key, entry);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(path: &Path, text: &str) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("parent");
        }
        std::fs::write(path, text).expect("write");
    }

    #[test]
    fn should_index_and_close_over_imported_modules() {
        let root = std::env::temp_dir().join(format!(
            "clorraine-lsp-workspace-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        write(
            &root.join("lib.clo"),
            "module app.lib;\nexport func add(a: i32, b: i32): i32 { return a + b; }\n",
        );
        write(
            &root.join("main.clo"),
            "module app.main;\nimport { add } from app.lib;\nfunc main(): i32 { return add(1, 2); }\n",
        );
        write(
            &root.join("other.clo"),
            "module app.other;\nfunc main(): i32 { return 0; }\n",
        );
        write(&root.join("build/skip.clo"), "module app.build;\n");
        let index = ModuleIndex::index_root(&root);
        let documents = DocumentStore::new();
        let requested = uri::parse(&uri::from_path(&root.join("main.clo"))).expect("uri");
        let set = index.analysis_set(&requested, &documents);
        let mut names: Vec<&str> = set.iter().map(|file| file.path.as_str()).collect();
        names.sort();
        let root_text = root.display().to_string();
        assert_eq!(
            names,
            vec![
                format!("{root_text}/lib.clo"),
                format!("{root_text}/main.clo"),
            ]
        );
        assert!(
            !names.iter().any(|name| name.ends_with("other.clo")),
            "an independent program must not join the set"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn should_use_an_overlay_import_when_building_the_closure() {
        let root = std::env::temp_dir().join(format!(
            "clorraine-lsp-overlay-import-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        write(&root.join("lib.clo"), "module app.lib;\nexport func add(): i32 { return 1; }\n");
        write(&root.join("main.clo"), "module app.main;\nfunc main(): i32 { return 0; }\n");
        let index = ModuleIndex::index_root(&root);
        let uri = uri::parse(&uri::from_path(&root.join("main.clo"))).expect("uri");
        let mut documents = DocumentStore::new();
        documents.open(
            &uri,
            "module app.main;\nimport { add } from app.lib;\nfunc main(): i32 { return add(); }\n"
                .to_owned(),
        );
        let set = index.analysis_set(&uri, &documents);
        assert_eq!(set.len(), 2, "the overlay import must join the library file");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn should_let_an_overlay_override_the_disk_text() {
        let root = std::env::temp_dir().join(format!(
            "clorraine-lsp-overlay-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        write(&root.join("main.clo"), "module app.main;\nfunc main(): i32 { return 0; }\n");
        let index = ModuleIndex::index_root(&root);
        let uri = uri::parse(&uri::from_path(&root.join("main.clo"))).expect("uri");
        let mut documents = DocumentStore::new();
        documents.open(&uri, "module app.main;\nfunc main(): i32 { return 1; }\n".to_owned());
        let set = index.analysis_set(&uri, &documents);
        assert_eq!(set.len(), 1);
        assert!(set[0].text.contains("return 1"));
        let _ = std::fs::remove_dir_all(&root);
    }
}
