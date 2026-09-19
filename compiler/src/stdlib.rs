//! Embedded standard-library export surface.
//!
//! The standard library is ordinary Clorraine that the compiler compiles with
//! every program. Loading and name resolution need one fact before those
//! sources can be compiled: which modules exist and which names each one
//! exports. This table carries exactly that surface. The module bodies are
//! added later; when they land, this table stays the single place that names
//! the modules and their exports.

/// One standard-library module and the names it exports.
pub struct StdlibModule {
    /// Dotted module name, for example `std.mem`.
    pub name: &'static str,
    /// Names the module exports, in the order the library declares them.
    pub exports: &'static [&'static str],
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
    fn every_module_name_is_unique() {
        let mut names: Vec<&str> = MODULES.iter().map(|module| module.name).collect();
        names.sort_unstable();
        let count = names.len();
        names.dedup();
        assert_eq!(names.len(), count);
    }
}
