//! Shared front-end analysis over in-memory sources.
//!
//! The command-line compiler, the language server, and the documentation
//! generator all need the same lexer, parser, loader, resolver, and checkers.
//! This module owns the order in which those stages run and the rule that a
//! later stage never consumes a program an earlier stage rejected.
//!
//! The driver keeps the file-read and command-line behavior; it hands the text
//! it read to [`analyze`]. Keeping the pipeline in one place means a tool can
//! never quietly run a different sequence from the compiler.
//!
//! # Examples
//!
//! ```text
//! let files = vec![SourceFile { path: "app.clo".into(), text: source }];
//! let analysis = analyze(Profile::Host, files, Mode::Full, true);
//! for diagnostic in analysis.diagnostics.items() {
//!     eprintln!("{}: {}", diagnostic.code, diagnostic.message);
//! }
//! ```

use std::collections::{BTreeMap, BTreeSet};

use crate::ast::Program;
use crate::diag::Diagnostics;
use crate::load::{self, Loaded};
use crate::own::{self, OwnershipOutput};
use crate::parse::Parser;
use crate::profile::Profile;
use crate::resolve;
use crate::sema::{self, SemaOutput};
use crate::span::{FileId, SourceMap};
use crate::stdlib;

/// One source file: the logical path a tool reports and its full text.
pub struct SourceFile {
    /// Path used in diagnostics and as the identity key.
    pub path: String,
    /// Complete UTF-8 source text.
    pub text: String,
}

/// How much of the front end to run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// Stop after parsing; the stage-select mode.
    ParseOnly,
    /// Run every front-end stage.
    Full,
}

/// The result of a complete analysis.
pub struct Analysis {
    /// Every input, including injected standard-library modules.
    pub sources: SourceMap,
    /// File identity by logical path.
    pub files: BTreeMap<String, FileId>,
    /// Path identity and the parsed tree, in input order.
    ///
    /// [`Mode::Full`] consumes the programs during loading, so this vector is
    /// empty after a full analysis; a caller that needs the tree uses
    /// [`parse`].
    pub programs: Vec<(FileId, Program)>,
    /// The loaded program, when loading ran.
    pub loaded: Option<Loaded>,
    /// Diagnostics from every stage that ran.
    pub diagnostics: Diagnostics,
    /// Type and ownership output when type checking succeeded.
    ///
    /// Ownership diagnostics may still be present; the driver checks the
    /// diagnostics before it emits C.
    pub typed: Option<(SemaOutput, OwnershipOutput)>,
}

/// The result of a parse-only run.
pub struct Parsed {
    /// Every input file.
    pub sources: SourceMap,
    /// File identity by logical path.
    pub files: BTreeMap<String, FileId>,
    /// Parsed programs, in input order.
    pub programs: Vec<(FileId, Program)>,
    /// Parse diagnostics.
    pub diagnostics: Diagnostics,
}

/// Runs the front end over in-memory sources.
///
/// `check_entry_point` is false for a library workspace: the language server
/// must not report a missing `func main(): i32` for a module that is not an
/// executable. The command-line compiler always passes true.
pub fn analyze(
    profile: Profile,
    files: Vec<SourceFile>,
    mode: Mode,
    check_entry_point: bool,
) -> Analysis {
    analyze_with(
        profile,
        files,
        mode,
        check_entry_point,
        Diagnostics::new(false, None),
    )
}

/// Like [`analyze`], but with a caller-configured diagnostic collector.
///
/// The command-line driver passes the collector it created from
/// `--deny-warnings` and `--max-errors`; tools that have no such flags use
/// [`analyze`].
pub fn analyze_with(
    profile: Profile,
    files: Vec<SourceFile>,
    mode: Mode,
    check_entry_point: bool,
    diagnostics: Diagnostics,
) -> Analysis {
    let Parsed {
        mut sources,
        files: file_ids,
        mut programs,
        mut diagnostics,
    } = parse_with(files, diagnostics);

    if mode == Mode::ParseOnly {
        return Analysis {
            sources,
            files: file_ids,
            programs,
            loaded: None,
            diagnostics,
            typed: None,
        };
    }

    if report_unavailable_profile_imports(profile, &programs, &mut diagnostics) {
        inject_stdlib(&mut programs, &mut sources, &mut diagnostics);
    } else {
        return Analysis {
            sources,
            files: file_ids,
            programs,
            loaded: None,
            diagnostics,
            typed: None,
        };
    }

    let loaded = load::load_with_entry_check(programs, check_entry_point, &mut diagnostics);
    resolve::resolve(&loaded, &mut diagnostics);
    let mut typed = None;
    if diagnostics.error_count() == 0 {
        // A file that failed resolution has no trustworthy names, so the type
        // stage would only produce dependent errors.
        let sema = sema::check(&loaded, profile, &mut diagnostics);
        if diagnostics.error_count() == 0 {
            // The ownership stage consumes the types the type stage recorded.
            let ownership = own::check(&loaded, &sema, &mut diagnostics);
            typed = Some((sema, ownership));
        }
    }
    Analysis {
        sources,
        files: file_ids,
        programs: Vec::new(),
        loaded: Some(loaded),
        diagnostics,
        typed,
    }
}

/// Parses sources without loading or checking them.
///
/// The documentation generator uses this entry point; it needs declarations
/// and spans, not types or ownership.
pub fn parse(files: Vec<SourceFile>) -> Parsed {
    parse_with(files, Diagnostics::new(false, None))
}

/// Parses sources and keeps the partial tree of a file with syntax errors.
///
/// The language server uses this so a file that is being edited still offers
/// symbols and completion for the declarations the parser recovered. The
/// compiler pipeline never uses it: a rejected file is not loaded or checked.
pub fn parse_partial(files: Vec<SourceFile>) -> Parsed {
    parse_inner(files, Diagnostics::new(false, None), true)
}

/// Like [`parse`], but with a caller-configured diagnostic collector.
pub fn parse_with(files: Vec<SourceFile>, diagnostics: Diagnostics) -> Parsed {
    parse_inner(files, diagnostics, false)
}

/// The shared parse body.
fn parse_inner(files: Vec<SourceFile>, diagnostics: Diagnostics, keep_partial: bool) -> Parsed {
    let mut sources = SourceMap::new();
    let mut diagnostics = diagnostics;
    let mut file_ids = BTreeMap::new();
    let mut ids = Vec::with_capacity(files.len());
    for file in files {
        let id = sources.add_file(file.path.clone(), file.text);
        file_ids.insert(file.path, id);
        ids.push(id);
    }
    let mut programs = Vec::new();
    for id in ids {
        let before = diagnostics.error_count();
        let program = Parser::new(&sources, id, &mut diagnostics).parse_program();
        if diagnostics.error_count() > before && !keep_partial {
            // A file the parser rejected has no trustworthy tree; later stages
            // would only add dependent errors.
            continue;
        }
        programs.push((id, program));
    }
    Parsed {
        sources,
        files: file_ids,
        programs,
        diagnostics,
    }
}

/// Parses and adds every embedded standard-library module the program imports.
///
/// The standard library is ordinary Clorraine compiled with the program, so an
/// imported module is loaded exactly like a user file. Only modules with an
/// embedded source are injected; a module that still has only a declaration
/// surface resolves its imports through the standard-library export table, so
/// its callers keep compiling while the module body is ported.
///
/// An embedded module may itself import another embedded module (for example
/// the graphics module borrows the screen handle from intuition). Injection
/// therefore runs to a fixpoint: each pass requests every module the current
/// programs reference, and a pass that injects nothing ends the loop.
fn inject_stdlib(
    programs: &mut Vec<(FileId, Program)>,
    sources: &mut SourceMap,
    diagnostics: &mut Diagnostics,
) {
    loop {
        let requested = requested_modules(programs);
        if requested.is_empty() {
            break;
        }
        for name in requested {
            let Some(text) = stdlib::source(&name) else {
                continue;
            };
            let path = format!("stdlib/{name}.clo");
            let file = sources.add_file(path, text.to_owned());
            let before = diagnostics.error_count();
            let mut program = Parser::new(sources, file, diagnostics).parse_program();
            if diagnostics.error_count() > before {
                // A malformed embedded module would be a compiler defect; the
                // parser already recorded the reason, so the stage stops here.
                continue;
            }
            // Mark the item as embedded so the reserved-intrinsic check allows
            // the library to declare the compiler-known helpers.
            program.embedded = true;
            programs.push((file, program));
        }
    }
}

/// The embedded standard-library modules the programs reference but do not yet
/// include, for which an embedded source exists.
///
/// A qualified reference to a standard-library module is as good as an import
/// for loading it, so both kinds of request are collected.
fn requested_modules(programs: &[(FileId, Program)]) -> Vec<String> {
    let mut imported: BTreeSet<String> = load::referenced_modules(programs).into_keys().collect();
    let mut present: BTreeSet<String> = BTreeSet::new();
    for (_, program) in programs.iter() {
        present.insert(program.module.name.to_dotted());
        for import in &program.imports {
            imported.insert(import.from.to_dotted());
        }
    }
    imported
        .into_iter()
        .filter(|name| !present.contains(name) && stdlib::source(name).is_some())
        .collect()
}

/// Reports a reference to an embedded standard-library module the profile does
/// not provide, returning false when at least one was reported.
///
/// A profile names its own API surface, so a standard-library module outside it
/// is a profile-surface error rather than a missing-module error. Both an
/// `import` and a qualified reference such as `std.amiga.dos.delayTicks(1)`
/// name the module, so both are checked. The check is gated on the module
/// actually being an embedded library module: a user module whose name happens
/// to start with the same prefix is ordinary code and must not be rejected.
/// Reporting before the load stage keeps the diagnostic single instead of
/// letting the reference cascade into unresolved-name errors.
fn report_unavailable_profile_imports(
    profile: Profile,
    programs: &[(FileId, Program)],
    diagnostics: &mut Diagnostics,
) -> bool {
    // A module the program itself declares is user code, even when its name
    // matches an embedded library module. `inject_stdlib` prefers the user
    // module, so the profile check must agree and not reject it.
    let provided: BTreeSet<String> = programs
        .iter()
        .map(|(_, program)| program.module.name.to_dotted())
        .collect();
    let mut unavailable: BTreeMap<String, crate::span::Span> = BTreeMap::new();
    for (_, program) in programs {
        for import in &program.imports {
            let module = import.from.to_dotted();
            if is_unavailable_stdlib(profile, &module, &provided) {
                unavailable.entry(module).or_insert(import.span);
            }
        }
    }
    for (module, span) in load::referenced_modules_outside_staged(programs) {
        if is_unavailable_stdlib(profile, &module, &provided) {
            unavailable.entry(module).or_insert(span);
        }
    }
    let mut available = true;
    for (module, span) in unavailable {
        diagnostics.push(crate::diag::Diagnostic::error(
            "E11200",
            format!("'{module}' is not available on profile '{}'", profile.name()),
            span,
        ));
        available = false;
    }
    available
}

/// True when the module is an embedded standard-library module the profile does
/// not provide and that the program did not supply itself.
fn is_unavailable_stdlib(profile: Profile, module: &str, provided: &BTreeSet<String>) -> bool {
    stdlib::source(module).is_some()
        && !provided.contains(module)
        && !profile.supports_module(module)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source(path: &str, text: &str) -> SourceFile {
        SourceFile {
            path: path.to_owned(),
            text: text.to_owned(),
        }
    }

    const LIBRARY: &str = "module app.core;\nexport func add(a: i32, b: i32): i32 { return a + b; }\n";

    #[test]
    fn should_keep_the_typed_output_when_ownership_reports() {
        let program = "\
module app.move;

func observe(s: String): void {
}

func demo(): void {
    let n: u32 = 1;
    let s: String = $\"n={n}\";
    let t: String = s;
    observe(s);
    observe(t);
}

func main(): i32 {
    return 0;
}
";
        let analysis = analyze(
            Profile::Host,
            vec![source("app/move.clo", program)],
            Mode::Full,
            true,
        );
        assert!(
            analysis
                .diagnostics
                .items()
                .iter()
                .any(|diagnostic| diagnostic.code == "E10020"),
            "the move-then-use must report E10020"
        );
        assert!(
            analysis.typed.is_some(),
            "the type output stays available while ownership reports"
        );
        assert!(
            analysis.programs.is_empty(),
            "a full analysis consumes the parsed programs during loading"
        );
    }

    #[test]
    fn should_map_every_input_path_to_its_file_id() {
        let parsed = parse(vec![
            source("app/one.clo", LIBRARY),
            source("app/two.clo", "module app.two;\nexport func f(): i32 { return 1; }\n"),
        ]);
        assert!(parsed.diagnostics.items().is_empty());
        assert_eq!(parsed.files.len(), 2);
        assert_ne!(
            parsed.files["app/one.clo"],
            parsed.files["app/two.clo"],
            "distinct paths must have distinct identities"
        );
        assert_eq!(parsed.programs.len(), 2);
    }

    #[test]
    fn should_drop_only_the_file_the_parser_rejected() {
        let parsed = parse(vec![
            source("app/bad.clo", "module app.bad;\nfunc broken( {\n"),
            source("app/good.clo", LIBRARY),
        ]);
        assert!(parsed.diagnostics.error_count() >= 1);
        assert_eq!(parsed.programs.len(), 1);
        assert_eq!(
            parsed.sources.path(parsed.programs[0].0),
            "app/good.clo"
        );
    }

    #[test]
    fn should_not_inject_the_standard_library_in_parse_only_mode() {
        let analysis = analyze(
            Profile::Host,
            vec![source(
                "app/io.clo",
                "module app.io;\nimport { Buffer } from std.mem;\nexport func f(b: Buffer): void { }\n",
            )],
            Mode::ParseOnly,
            false,
        );
        assert!(analysis.loaded.is_none());
        assert_eq!(analysis.sources.path(analysis.programs[0].0), "app/io.clo");
        assert!(
            !analysis
                .files
                .keys()
                .any(|path| path.starts_with("stdlib/")),
            "parse-only must not inject standard-library sources"
        );
    }
}
