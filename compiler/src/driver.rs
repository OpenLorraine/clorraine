//! Driver: the command-line contract and the front-end pipeline.
//!
//! The driver parses the command line, loads every input file up front, runs
//! the front-end stages, renders diagnostics to stderr, and writes the
//! compiler output. Until the emitter lands, a successful run writes an empty
//! translation unit, so the command-line behavior and the diagnostic stream
//! are already exercised end to end.

use crate::analysis;
use crate::codegen;
use crate::diag::{
    format_error, format_from_args, registry, Diagnostic, DiagnosticFormat, Diagnostics,
};
use crate::profile::Profile;
use crate::span::SourceMap;

/// Exit code for command-line and input problems.
const EXIT_USAGE: i32 = 2;

/// The parsed command line.
struct Options {
    /// Diagnostic code to explain; an early-exit mode.
    explain: Option<String>,
    /// Diagnostic output format.
    diagnostics: DiagnosticFormat,
    /// `--version` was requested.
    version: bool,
    /// Selected profile.
    profile: Option<String>,
    /// Stop after the parser; the stage-select mode for parse-shaped sources.
    parse_only: bool,
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
            explain: None,
            diagnostics: DiagnosticFormat::Human,
            version: false,
            profile: None,
            parse_only: false,
            deny_warnings: false,
            max_errors: None,
            output: None,
            files: Vec::new(),
        };
        let mut index = 0;
        while index < args.len() {
            let arg = args[index].as_str();
            match arg {
                "--explain" => {
                    index += 1;
                    options.explain = Some(argument(args, index, "--explain")?);
                }
                "--diagnostics" => {
                    index += 1;
                    let raw = argument(args, index, "--diagnostics")?;
                    options.diagnostics = DiagnosticFormat::parse(&raw)?;
                }
                "--version" => options.version = true,
                "--deny-warnings" => options.deny_warnings = true,
                "--parse-only" => options.parse_only = true,
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
                _ if arg.starts_with("--explain=") => {
                    let code = &arg["--explain=".len()..];
                    if code.is_empty() {
                        return Err("--explain needs a value".to_owned());
                    }
                    options.explain = Some(code.to_owned());
                }
                _ if arg.starts_with("--diagnostics=") => {
                    options.diagnostics =
                        DiagnosticFormat::parse(&arg["--diagnostics=".len()..])?;
                }
                _ if arg.starts_with('-') => {
                    return Err(format!("unknown option '{arg}'"));
                }
                _ => options.files.push(arg.to_owned()),
            }
            index += 1;
        }
        if options.explain.is_some() {
            return Ok(options);
        }
        if options.version {
            return Ok(options);
        }
        if options.profile.is_none() {
            return Err(
                "--profile <host|hosted-amiga|hosted-amiga-13|freestanding> is required".to_owned(),
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
    let limit = raw
        .parse::<usize>()
        .map_err(|_| format!("--max-errors expects a number, got '{raw}'"))?;
    if limit == 0 {
        return Err("--max-errors expects a positive number".to_owned());
    }
    Ok(limit)
}

/// Runs the compiler and returns the process exit code.
pub fn run(args: Vec<String>) -> i32 {
    let requested = format_from_args(&args);
    let options = match Options::parse(&args) {
        Ok(options) => options,
        Err(message) => {
            eprint!(
                "{}",
                format_error(requested, &format!("usage error: {message}"))
            );
            return EXIT_USAGE;
        }
    };
    let format = options.diagnostics;
    if let Some(code) = &options.explain {
        return match registry::explain(code) {
            Some(text) => {
                print!("{text}");
                0
            }
            None => {
                eprint!(
                    "{}",
                    format_error(
                        format,
                        &format!("usage error: unknown diagnostic code '{code}'")
                    )
                );
                EXIT_USAGE
            }
        };
    }
    if options.version {
        println!("clc {}", env!("CARGO_PKG_VERSION"));
        return 0;
    }
    let mut diagnostics = Diagnostics::new(options.deny_warnings, options.max_errors);
    let Some(profile) = options.profile.as_deref() else {
        eprint!(
            "{}",
            format_error(
                format,
                "usage error: --profile <host|hosted-amiga|hosted-amiga-13|freestanding> is required"
            )
        );
        return EXIT_USAGE;
    };
    if !profile_available(profile, &mut diagnostics) {
        emit_diagnostics(&diagnostics, &SourceMap::new(), format);
        return diagnostics.exit_code();
    }
    let mut inputs = Vec::new();
    for path in &options.files {
        match std::fs::read_to_string(path) {
            Ok(text) => inputs.push(analysis::SourceFile {
                path: path.clone(),
                text,
            }),
            Err(error) => {
                eprint!(
                    "{}",
                    format_error(format, &format!("clc: cannot read '{path}': {error}"))
                );
                return EXIT_USAGE;
            }
        }
    }
    // A stage-select run stops after the parser: the source shapes a stage
    // test exercises may be rejected by a later stage, while the parse is the
    // stage under test. Syntax errors still report and fail the run.
    if options.parse_only {
        let parsed = analysis::parse_with(inputs, diagnostics);
        emit_diagnostics(&parsed.diagnostics, &parsed.sources, format);
        return parsed.diagnostics.exit_code();
    }
    let Some(kind) = Profile::parse(profile) else {
        return EXIT_USAGE;
    };
    let mut analyzed = analysis::analyze_with(
        kind,
        inputs,
        analysis::Mode::Full,
        true,
        diagnostics,
    );
    // A front-end error rules out code generation; rendered here so the
    // command-line contract still reports the diagnostics before exiting.
    if analyzed.diagnostics.exit_code() != 0 {
        emit_diagnostics(&analyzed.diagnostics, &analyzed.sources, format);
        return analyzed.diagnostics.exit_code();
    }
    // A successful compile emits the translation unit; an absent `-o` keeps the
    // compiler silent, matching the command-line contract. The analysis is
    // present whenever code generation runs, because a diagnostics-clean
    // program always produced it. Code generation may add the large-frame
    // warning, so the diagnostics render only after it runs.
    let emitted = match (&analyzed.loaded, &analyzed.typed) {
        (Some(loaded), Some((sema, ownership))) => codegen::emit(
            loaded,
            sema,
            ownership,
            kind,
            &analyzed.sources,
            &mut analyzed.diagnostics,
        ),
        _ => String::new(),
    };
    emit_diagnostics(&analyzed.diagnostics, &analyzed.sources, format);
    if analyzed.diagnostics.exit_code() != 0 {
        return analyzed.diagnostics.exit_code();
    }
    if let Some(path) = &options.output {
        if let Err(error) = std::fs::write(path, emitted) {
            eprint!(
                "{}",
                format_error(format, &format!("clc: cannot write '{path}': {error}"))
            );
            return EXIT_USAGE;
        }
    }
    0
}

/// Writes the collected diagnostics in the selected format.
fn emit_diagnostics(diagnostics: &Diagnostics, sources: &SourceMap, format: DiagnosticFormat) {
    eprint!("{}", diagnostics.render_with(sources, format));
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
                    .with_help("use `hosted-amiga`, `hosted-amiga-13`, `host`, or `freestanding`"),
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
        let joined = Options::parse(&args(&["--profile=hosted-amiga", "--max-errors=2", "a.clo"])).expect("joined flags");
        assert_eq!(joined.profile.as_deref(), Some("hosted-amiga"));
        assert_eq!(joined.max_errors, Some(2));
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
        assert!(profile_available("hosted-amiga-13", &mut diagnostics));
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
