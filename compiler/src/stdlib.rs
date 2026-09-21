//! Embedded standard-library surface and sources.
//!
//! The standard library is ordinary Clorraine that the compiler compiles with
//! every program. Loading and name resolution need one fact before those
//! sources can be compiled: which modules exist and which names each one
//! exports. The module bodies themselves are embedded in the binary with
//! `include_str!`, so no install path or filesystem lookup is needed and the
//! emitted translation unit is deterministic.
//!
//! A module that has not been ported yet still contributes its export surface
//! here; the driver only injects the modules that have a source.

/// One standard-library module and the names it exports.
pub struct StdlibModule {
    /// Dotted module name, for example `std.mem`.
    pub name: &'static str,
    /// Names the module exports, in the order the library declares them.
    pub exports: &'static [&'static str],
}

/// One embedded standard-library module and its Clorraine source.
pub struct StdlibSource {
    /// Dotted module name, matching the module declaration in the source.
    pub name: &'static str,
    /// The module body, embedded at build time.
    pub source: &'static str,
}

/// The standard-library modules whose Clorraine sources are embedded.
///
/// `include_str!` reads the file at build time relative to this module, so the
/// path is `<repo>/stdlib/<name>.clo`. Adding a module here makes it available
/// to the driver; a module without an entry stays a declarations-only surface
/// described by [`MODULES`].
pub const SOURCES: &[StdlibSource] = &[
    StdlibSource {
        name: "std.mem",
        source: include_str!("../../stdlib/std.mem.clo"),
    },
];

/// The compiler-known intrinsic helper names.
///
/// These helpers are implemented by the compiler and declared by the embedded
/// standard library. A user module may not declare an `intrinsic func` with one
/// of these names.
pub const COMPILER_INTRINSICS: &[&str] = &["rawSlice", "rawStr", "slice", "len"];

/// True when `name` is a compiler-known intrinsic helper.
pub fn is_compiler_intrinsic(name: &str) -> bool {
    COMPILER_INTRINSICS.contains(&name)
}

/// The embedded source of a standard-library module, when it has one.
pub fn source(name: &str) -> Option<&'static str> {
    SOURCES
        .iter()
        .find(|module| module.name == name)
        .map(|module| module.source)
}

/// True when the module is a compiler-known standard-library module.
///
/// The visibility rules use this to grant the module's export surface even
/// where a generic declaration omits `export`.
pub fn is_standard_module(name: &str) -> bool {
    lookup(name).is_some()
}

/// Every standard-library module the compiler provides.
///
/// The export lists mirror the standard-library signatures: a handle or struct
/// name, a constant, or a function name. Overloads share one name, so `text`
/// appears once even though it has one overload per argument type.
pub const MODULES: &[StdlibModule] = &[
    StdlibModule {
        name: "std.mem",
        exports: &[
            "Buffer",
            "Status",
            "Span",
            "bufferNew",
            "bufferFrom",
            "bufferBytes",
            "bufferLen",
            "slice",
            "spanOf",
            "spanBytes",
            "Array",
            "arrayNew",
            "push",
            "get",
            "set",
            "reserve",
        ],
    },
    StdlibModule {
        name: "std.text",
        exports: &["String", "stringOf", "clone", "bytes", "toCString"],
    },
    StdlibModule {
        name: "std.fmt",
        exports: &["text", "toHex", "toDecimal", "padLeft", "padRight"],
    },
    StdlibModule {
        name: "std.io",
        exports: &[
            "File",
            "MODE_OLDFILE",
            "MODE_NEWFILE",
            "openFile",
            "createFile",
            "readFile",
            "writeBytes",
            "print",
            "println",
        ],
    },
    StdlibModule {
        name: "std.amiga.dos",
        exports: &["delayTicks"],
    },
    StdlibModule {
        name: "std.amiga.interrupt",
        exports: &["HandlerToken", "addHandler", "removeHandler"],
    },
];

/// Looks up a standard-library module by its dotted name.
pub fn lookup(name: &str) -> Option<&'static StdlibModule> {
    MODULES.iter().find(|module| module.name == name)
}

/// True when the standard-library module exports the name.
pub fn exports(module: &str, name: &str) -> bool {
    lookup(module).is_some_and(|found| found.exports.contains(&name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modules_are_found_by_their_dotted_name() {
        assert!(lookup("std.mem").is_some());
        assert!(lookup("std.amiga.interrupt").is_some());
        assert!(lookup("std.nope").is_none());
        assert!(lookup("std").is_none());
    }

    #[test]
    fn exports_are_scoped_to_their_module() {
        assert!(exports("std.mem", "Buffer"));
        assert!(exports("std.io", "println"));
        assert!(!exports("std.mem", "println"));
        assert!(!exports("std.mem", "Missing"));
        assert!(!exports("std.nope", "Buffer"));
    }

    #[test]
    fn embedded_sources_match_their_export_table_entry() {
        for module in SOURCES {
            assert!(
                lookup(module.name).is_some(),
                "embedded module '{}' has no export table entry",
                module.name
            );
            assert!(
                module.source.contains("module "),
                "embedded module '{}' has no module declaration",
                module.name
            );
        }
        assert!(source("std.mem").is_some());
        assert!(source("std.nope").is_none());
    }

    #[test]
    fn compiler_intrinsics_are_recognized() {
        assert!(is_compiler_intrinsic("rawSlice"));
        assert!(is_compiler_intrinsic("rawStr"));
        assert!(is_compiler_intrinsic("slice"));
        assert!(is_compiler_intrinsic("len"));
        assert!(!is_compiler_intrinsic("sizeof"));
        assert!(is_standard_module("std.mem"));
        assert!(!is_standard_module("app.core"));
    }

    #[test]
    fn every_module_name_is_unique() {
        let mut names: Vec<&str> = MODULES.iter().map(|module| module.name).collect();
        names.sort_unstable();
        let count = names.len();
        names.dedup();
        assert_eq!(names.len(), count);
    }

    #[test]
    fn every_corpus_import_is_exported() {
        let corpus = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../spec/tests/corpus");
        let mut checked = 0;
        for entry in std::fs::read_dir(&corpus).expect("corpus directory") {
            let program = entry.expect("corpus entry").path().join("program.clo");
            let text = std::fs::read_to_string(&program)
                .unwrap_or_else(|error| panic!("cannot read {}: {error}", program.display()));
            for (module, name) in corpus_imports(&text) {
                assert!(
                    exports(&module, &name),
                    "the corpus imports '{name}' from '{module}', but the table does not export it"
                );
                checked += 1;
            }
        }
        assert!(checked > 0, "no corpus imports were found to check");
    }

    /// Reads the `import { ... } from A.B;` lines of one program.
    fn corpus_imports(text: &str) -> Vec<(String, String)> {
        let mut imports = Vec::new();
        for line in text.lines() {
            let line = line.trim();
            let Some(rest) = line.strip_prefix("import") else {
                continue;
            };
            let Some(rest) = rest.trim().strip_prefix('{') else {
                continue;
            };
            let Some((names, tail)) = rest.split_once('}') else {
                continue;
            };
            let Some(module) = tail.trim().strip_prefix("from") else {
                continue;
            };
            let module = module.trim().trim_end_matches(';').trim();
            for name in names.split(',') {
                let name = name.split_whitespace().next().unwrap_or("");
                if !name.is_empty() {
                    imports.push((module.to_owned(), name.to_owned()));
                }
            }
        }
        imports
    }
}
