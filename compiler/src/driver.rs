//! Driver: the command-line contract and the front-end pipeline.
//!
//! The driver parses the command line, loads every input file up front, runs
//! the front-end stages, renders diagnostics to stderr, and writes the
//! compiler output. Until the emitter lands, a successful run writes an empty
//! translation unit, so the command-line behavior and the diagnostic stream
//! are already exercised end to end.

use crate::diag::{Diagnostic, Diagnostics};
use crate::parse::Parser;
use crate::span::{FileId, SourceMap};

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
            return Err("--profile <host|hosted-amiga> is required".to_owned());
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
        eprintln!("usage error: --profile <host|hosted-amiga> is required");
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
    for file in files {
        // The front end's deliverable is the diagnostic stream; later stages
        // will consume the parsed program instead of dropping it here.
        let _program = Parser::new(&sources, file, &mut diagnostics).parse_program();
    }
    eprint!("{}", diagnostics.render(&sources));
    let exit_code = diagnostics.exit_code();
    if exit_code != 0 {
        return exit_code;
    }
    if let Some(path) = &options.output {
        if let Err(error) = std::fs::write(path, "") {
            eprintln!("clc: cannot write '{path}': {error}");
            return EXIT_USAGE;
        }
    }
    0
}

/// Checks the selected profile name.
///
/// `freestanding` is designed but not implemented, which is rejected with its
/// own code; any other unknown name is rejected as unknown.
fn profile_available(name: &str, diagnostics: &mut Diagnostics) -> bool {
    match name {
        "host" | "hosted-amiga" => true,
        "freestanding" => {
            diagnostics.push(Diagnostic::cli_error(
                "E11200",
                format!("'{name}' is not available on profile '{name}'"),
            ));
            false
        }
        _ => {
            diagnostics.push(
                Diagnostic::cli_error("E11201", format!("unknown profile '{name}'"))
                    .with_help("use `hosted-amiga`, `host`, or `freestanding`"),
            );
            false
        }
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
        assert!(!profile_available("freestanding", &mut diagnostics));
        assert!(!profile_available("nope", &mut diagnostics));
        let codes: Vec<&str> = diagnostics
            .items()
            .iter()
            .map(|diagnostic| diagnostic.code)
            .collect();
        assert_eq!(codes, ["E11200", "E11201"]);
    }
}
