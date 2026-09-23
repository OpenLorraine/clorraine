//! Command-line parsing for the `clorraine` developer driver.
//!
//! The grammar is `clorraine <command> [options] FILE...`. Parsing has one
//! job: turn a raw argument vector into either a validated [`Options`] value, a
//! request for help or the version, or a message naming the malformed input.
//! Every malformed input is a usage error that the caller prints to stderr and
//! answers with exit code 2; parse itself stays silent.
//!
//! Validation is deliberately separate from parsing. A flag can be spelled
//! correctly and still be inapplicable to the chosen command (for example `-o`
//! with `run`), so the rules are applied only once the whole line is read.

use crate::profile::Profile;

/// The command the developer asked for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    /// Build and run the program.
    Run,
    /// Build the program without running it.
    Build,
    /// Produce the C translation unit only.
    Emit,
}

/// A parsed and validated invocation.
#[derive(Debug, PartialEq, Eq)]
pub struct Options {
    /// Which command to perform.
    pub command: Command,
    /// Target profile; `host` unless `--profile` overrides it.
    pub profile: Profile,
    /// Optimize with `-O2` instead of the debug `-O0 -g`.
    pub release: bool,
    /// Print the generated C after a build or before a run.
    pub emit_c: bool,
    /// Hand the hosted-amiga run to the host FS-UAE wrapper.
    pub emulator: bool,
    /// Write the program stdout to `out.txt` and print that path.
    pub capture: bool,
    /// Output file for `emit -o`; without it the C goes to stdout.
    pub output: Option<String>,
    /// Input source files, in command-line order.
    pub files: Vec<String>,
}

/// What a parsed command line asks the driver to do.
#[derive(Debug, PartialEq, Eq)]
pub enum Action {
    /// Print the help text and exit zero.
    Help,
    /// Print the version line and exit zero.
    Version,
    /// Perform a command.
    Invoke(Options),
}

/// The help text shown for no command, `-h`, or `--help`.
const HELP: &str = "\
clorraine: build and run Clorraine programs

Usage:
  clorraine <command> [options] FILE...

Commands:
  run     Build and run a program
  build   Build a program without running it
  emit    Print the generated C (or write it with -o)

Options:
  -p, --profile <host|hosted-amiga>   Target profile (default: host)
      --release                       Optimize (-O2) instead of debug (-O0 -g)
      --emit-c                        Print the generated C after building
      --emulator                      hosted-amiga run: use FS-UAE (host handoff)
      --capture                       Write program stdout to out.txt; print its path
  -o FILE                             emit only: write C to FILE (default: stdout)
  -h, --help                          Print this help
  -V, --version                       Print the version
";

/// Returns the help text printed for `-h` and `--help`.
pub fn help_text() -> &'static str {
    HELP
}

/// Returns the version line printed for `-V` and `--version`.
pub fn version_text() -> String {
    format!("clorraine {}", env!("CARGO_PKG_VERSION"))
}

/// Parses and validates the command line.
///
/// `args` excludes the program name. A malformed line returns a message that
/// describes the problem without a prefix; the caller adds the `usage error:`
/// prefix and exits 2.
pub fn parse(args: &[String]) -> Result<Action, String> {
    // `-h`/`-V` are honoured wherever they appear, so `clorraine run --help`
    // behaves like `clorraine --help` instead of being rejected as a flag.
    if args
        .iter()
        .any(|arg| arg == "-h" || arg == "--help")
    {
        return Ok(Action::Help);
    }
    if args
        .iter()
        .any(|arg| arg == "-V" || arg == "--version")
    {
        return Ok(Action::Version);
    }
    let Some(first) = args.first() else {
        // No command at all is the same as asking for help.
        return Ok(Action::Help);
    };
    let command = match first.as_str() {
        "run" => Command::Run,
        "build" => Command::Build,
        "emit" => Command::Emit,
        other if other.starts_with('-') => return Err(format!("unknown option '{other}'")),
        other => return Err(format!("unknown command '{other}'")),
    };
    let options = parse_options(command, &args[1..])?;
    validate(&options)?;
    Ok(Action::Invoke(options))
}

/// Reads the options and files after the command word.
fn parse_options(command: Command, rest: &[String]) -> Result<Options, String> {
    let mut options = Options {
        command,
        profile: Profile::Host,
        release: false,
        emit_c: false,
        emulator: false,
        capture: false,
        output: None,
        files: Vec::new(),
    };
    let mut profile: Option<String> = None;
    let mut index = 0;
    while index < rest.len() {
        let arg = rest[index].as_str();
        match arg {
            "-p" | "--profile" => {
                index += 1;
                profile = Some(value(rest, index, arg)?);
            }
            "-o" => {
                index += 1;
                options.output = Some(value(rest, index, arg)?);
            }
            "--release" => options.release = true,
            "--emit-c" => options.emit_c = true,
            "--emulator" => options.emulator = true,
            "--capture" => options.capture = true,
            _ if arg.starts_with("--profile=") => {
                profile = Some(arg["--profile=".len()..].to_owned());
            }
            _ if arg.starts_with('-') => return Err(format!("unknown option '{arg}'")),
            _ => options.files.push(arg.to_owned()),
        }
        index += 1;
    }
    if let Some(name) = profile {
        options.profile = Profile::parse(&name)
            .ok_or_else(|| format!("unknown profile '{name}' (expected host or hosted-amiga)"))?;
    }
    Ok(options)
}

/// Reads the value that follows a flag, or reports the flag as incomplete.
fn value(rest: &[String], index: usize, flag: &str) -> Result<String, String> {
    rest.get(index)
        .cloned()
        .ok_or_else(|| format!("{flag} needs a value"))
}

/// Applies the rules that only hold once the command is known.
fn validate(options: &Options) -> Result<(), String> {
    if options.files.is_empty() {
        return Err("at least one input file is required".to_owned());
    }
    if options.command == Command::Emit {
        // `emit` stops after producing C, so build-and-run flags do not apply.
        if options.release {
            return Err("--release is not valid with 'emit'".to_owned());
        }
        if options.emit_c {
            return Err("--emit-c is not valid with 'emit'".to_owned());
        }
        if options.emulator {
            return Err("--emulator is not valid with 'emit'".to_owned());
        }
        if options.capture {
            return Err("--capture is not valid with 'emit'".to_owned());
        }
        return Ok(());
    }
    // `-o` names the emit destination, never a built artifact.
    if options.output.is_some() {
        return Err("-o is only valid with 'emit'".to_owned());
    }
    if options.command == Command::Build {
        if options.emulator {
            return Err("--emulator is only valid with 'run'".to_owned());
        }
        if options.capture {
            return Err("--capture is only valid with 'run'".to_owned());
        }
    }
    // `--emulator` selects the FS-UAE runner, which only exists on Amiga runs.
    if options.emulator && options.profile != Profile::HostedAmiga {
        return Err("--emulator requires --profile hosted-amiga".to_owned());
    }
    Ok(())
}
