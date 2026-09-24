//! Driver: the command-line contract and the front-end pipeline.
//!
//! The driver parses the command line, loads every input file up front, runs
//! the front-end stages, renders diagnostics to stderr, and writes the
//! compiler output. Until the emitter lands, a successful run writes an empty
//! translation unit, so the command-line behavior and the diagnostic stream
//! are already exercised end to end.

use std::collections::{BTreeMap, BTreeSet};

use crate::ast::Program;
use crate::codegen;
use crate::diag::{Diagnostic, Diagnostics};
use crate::load;
use crate::own;
use crate::parse::Parser;
use crate::profile::Profile;
use crate::resolve;
use crate::sema;
use crate::span::{FileId, SourceMap};
use crate::stdlib;

/// Exit code for command-line and input problems.
const EXIT_USAGE: i32 = 2;

/// The parsed command line.
struct Options {
    /// `--version` was requested.
    version: bool,
    /// Selected profile.
    profile: Option<String>,
    /// Promote warnings to failure.
    deny_warnings: bool,
    /// Cap on reported errors.
    max_errors: Option<usize>,
    /// Output file from `-o`; without it a successful run writes nothing.
    output: Option<String>,
    /// Input files in command-line order.
    files: Vec<String>,
}

impl Options {
    /// Parses the command line, rejecting malformed invocations.
    fn parse(args: &[String]) -> Result<Self, String> {
        let mut options = Self {
            version: false,
            profile: None,
            deny_warnings: false,
            max_errors: None,
            output: None,
            files: Vec::new(),
        };
        let mut index = 0;
        while index < args.len() {
            let arg = args[index].as_str();
            match arg {
                "--version" => options.version = true,
                "--deny-warnings" => options.deny_warnings = true,
                "--profile" => {
                    index += 1;
                    options.profile = Some(argument(args, index, "--profile")?);
                }
                "--max-errors" => {
                    index += 1;
                    let raw = argument(args, index, "--max-errors")?;
                    options.max_errors = Some(error_limit(&raw)?);
                }
                "-o" => {
                    index += 1;
                    options.output = Some(argument(args, index, "-o")?);
                }
                _ if arg.starts_with("--profile=") => {
                    options.profile = Some(arg["--profile=".len()..].to_owned());
                }
                _ if arg.starts_with("--max-errors=") => {
                    let raw = &arg["--max-errors=".len()..];
                    options.max_errors = Some(error_limit(raw)?);
                }
                _ if arg.starts_with('-') => {
                    return Err(format!("unknown option '{arg}'"));
                }
                _ => options.files.push(arg.to_owned()),
            }
            index += 1;
        }
        if options.version {
            return Ok(options);
        }
        if options.profile.is_none() {
            return Err(
                "--profile <host|hosted-amiga|freestanding> is required".to_owned(),
            );
        }
        if options.files.is_empty() {
            return Err("at least one input file is required".to_owned());
        }
        Ok(options)
    }
}

/// Reads the value of a flag that expects one.
fn argument(args: &[String], index: usize, flag: &str) -> Result<String, String> {
    args.get(index)
        .cloned()
        .ok_or_else(|| format!("{flag} needs a value"))
}

/// Parses the `--max-errors` value.
fn error_limit(raw: &str) -> Result<usize, String> {
    raw.parse::<usize>()
        .map_err(|_| format!("--max-errors expects a number, got '{raw}'"))
}

/// Parses and adds every embedded standard-library module the program imports.
///
/// The standard library is ordinary Clorraine compiled with the program, so an
/// imported module is loaded exactly like a user file. Only modules with an
/// embedded source are injected; a module that still has only a declaration
/// surface resolves its imports through the standard-library export table, so
/// its callers keep compiling while the module body is ported.
fn inject_stdlib(
    programs: &mut Vec<(FileId, Program)>,
    sources: &mut SourceMap,
    diagnostics: &mut Diagnostics,
) {
    // A qualified reference to a standard-library module is as good as an
    // import for loading it, so both kinds of request are collected.
    let mut imported: BTreeSet<String> =
        load::referenced_modules(programs).into_keys().collect();
    let mut present: BTreeSet<String> = BTreeSet::new();
    for (_, program) in programs.iter() {
        present.insert(program.module.name.to_dotted());
        for import in &program.imports {
            imported.insert(import.from.to_dotted());
        }
    }
    let requested: Vec<String> = imported
        .into_iter()
        .filter(|name| !present.contains(name) && stdlib::source(name).is_some())
        .collect();
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
        // Mark the item as embedded so the reserved-intrinsic check allows the
        // library to declare the compiler-known helpers.
        program.embedded = true;
        programs.push((file, program));
    }
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
        diagnostics.push(Diagnostic::error(
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

/// Runs the compiler and returns the process exit code.
pub fn run(args: Vec<String>) -> i32 {
    let options = match Options::parse(&args) {
        Ok(options) => options,
        Err(message) => {
            eprintln!("usage error: {message}");
            return EXIT_USAGE;
        }
    };
    if options.version {
        println!("clc {}", env!("CARGO_PKG_VERSION"));
        return 0;
    }
    let mut diagnostics = Diagnostics::new(options.deny_warnings, options.max_errors);
    let Some(profile) = options.profile.as_deref() else {
        eprintln!("usage error: --profile <host|hosted-amiga|freestanding> is required");
        return EXIT_USAGE;
    };
    if !profile_available(profile, &mut diagnostics) {
        eprint!("{}", diagnostics.render(&SourceMap::new()));
        return diagnostics.exit_code();
    }
    let mut sources = SourceMap::new();
    let mut files: Vec<FileId> = Vec::new();
    for path in &options.files {
        match std::fs::read_to_string(path) {
            Ok(text) => files.push(sources.add_file(path.clone(), text)),
            Err(error) => {
                eprintln!("clc: cannot read '{path}': {error}");
                return EXIT_USAGE;
            }
        }
    }
    let mut programs = Vec::new();
    for file in files {
        let before = diagnostics.error_count();
        let program = Parser::new(&sources, file, &mut diagnostics).parse_program();
        if diagnostics.error_count() > before {
            // The front end already poisoned this file; running the module
            // stage would only add dependent errors.
            continue;
        }
        programs.push((file, program));
    }
    let Some(kind) = Profile::parse(profile) else {
        return EXIT_USAGE;
    };
    if !report_unavailable_profile_imports(kind, &programs, &mut diagnostics) {
        eprint!("{}", diagnostics.render(&sources));
        return diagnostics.exit_code();
    }
    inject_stdlib(&mut programs, &mut sources, &mut diagnostics);
    // The module and name stages consume the loaded program instead of
    // dropping it; the emitter lands later and will take the same value.
    let loaded = load::load(programs, &mut diagnostics);
    resolve::resolve(&loaded, &mut diagnostics);
    let mut analysis = None;
    if diagnostics.error_count() == 0 {
        // A file that already failed resolution has no trustworthy names, so
        // the type stage would only produce dependent errors.
        let sema = sema::check(&loaded, kind, &mut diagnostics);
        if diagnostics.error_count() == 0 {
            // The ownership stage consumes the types the type stage recorded,
            // so it runs only on a program that typed cleanly. Its output is
            // the input code generation consumes.
            let ownership = own::check(&loaded, &sema, &mut diagnostics);
            analysis = Some((sema, ownership));
        }
    }
    // A front-end error rules out code generation; rendered here so the
    // command-line contract still reports the diagnostics before exiting.
    if diagnostics.exit_code() != 0 {
        eprint!("{}", diagnostics.render(&sources));
        return diagnostics.exit_code();
    }
    // A successful compile emits the translation unit; an absent `-o` keeps the
    // compiler silent, matching the command-line contract. The analysis is
    // present whenever code generation runs, because a diagnostics-clean
    // program always produced it. Code generation may add the large-frame
    // warning, so the diagnostics render only after it runs.
    let emitted = match &analysis {
        Some((sema, ownership)) => {
            codegen::emit(&loaded, sema, ownership, kind, &sources, &mut diagnostics)
        }
        None => String::new(),
    };
    eprint!("{}", diagnostics.render(&sources));
    if diagnostics.exit_code() != 0 {
        return diagnostics.exit_code();
    }
    if let Some(path) = &options.output {
        if let Err(error) = std::fs::write(path, emitted) {
            eprintln!("clc: cannot write '{path}': {error}");
            return EXIT_USAGE;
        }
    }
    0
}

/// Checks the selected profile name.
///
/// The profile facts live on [`Profile`], so this is the only place the name
/// list is resolved. An unknown name is `E11201`; a known but unimplemented
/// profile would be `E11200`. No profile is currently unimplemented, so that
/// branch is dormant but preserved for a future one.
fn profile_available(name: &str, diagnostics: &mut Diagnostics) -> bool {
    match Profile::parse(name) {
        None => {
            diagnostics.push(
                Diagnostic::cli_error("E11201", format!("unknown profile '{name}'"))
                    .with_help("use `hosted-amiga`, `host`, or `freestanding`"),
            );
            false
        }
        Some(profile) if !profile.implemented() => {
            diagnostics.push(Diagnostic::cli_error(
                "E11200",
                format!("'{name}' is not available on profile '{name}'"),
            ));
            false
        }
        Some(_) => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|item| (*item).to_owned()).collect()
    }

    #[test]
    fn options_require_a_profile_and_a_file() {
        assert!(Options::parse(&args(&[])).is_err());
        assert!(Options::parse(&args(&["--profile", "host"])).is_err());
        assert!(Options::parse(&args(&["main.clo"])).is_err());
        assert!(Options::parse(&args(&["--profile", "host", "main.clo"])).is_ok());
    }

    #[test]
    fn options_accept_both_flag_spellings() {
        let separate = Options::parse(&args(&["--profile", "host", "--max-errors", "3", "-o", "out.c", "a.clo"])).expect("separate flags");
        assert_eq!(separate.profile.as_deref(), Some("host"));
        assert_eq!(separate.max_errors, Some(3));
        assert_eq!(separate.output.as_deref(), Some("out.c"));
        let joined = Options::parse(&args(&["--profile=hosted-amiga", "--max-errors=0", "a.clo"])).expect("joined flags");
        assert_eq!(joined.profile.as_deref(), Some("hosted-amiga"));
        assert_eq!(joined.max_errors, Some(0));
    }

    #[test]
    fn options_reject_unknown_flags_and_bad_limits() {
        assert!(Options::parse(&args(&["--profile", "host", "--nope", "a.clo"])).is_err());
        assert!(Options::parse(&args(&["--profile", "host", "--max-errors=x", "a.clo"])).is_err());
    }

    #[test]
    fn profiles_are_validated_with_their_codes() {
        let mut diagnostics = Diagnostics::new(false, None);
        assert!(profile_available("host", &mut diagnostics));
        assert!(profile_available("hosted-amiga", &mut diagnostics));
        assert!(profile_available("freestanding", &mut diagnostics));
        assert!(!profile_available("nope", &mut diagnostics));
        let codes: Vec<&str> = diagnostics
            .items()
            .iter()
            .map(|diagnostic| diagnostic.code)
            .collect();
        assert_eq!(codes, ["E11201"]);
    }
}
