//! The `clorraine` developer driver: build and run Clorraine programs.
//!
//! The driver fronts the emit-only compiler (`clc`) and owns the build/run
//! orchestration: it emits the C in process, compiles it with the profile's
//! toolchain, and runs it, so a developer sees only the program's output. This
//! module is the testable entry point; the `clorraine` binary is a thin
//! wrapper.
//!
//! Exit codes are part of the contract: 0 on success (or a printed help,
//! version, or FS-UAE handoff), 1 for a Clorraine diagnostic, a missing tool,
//! or a C compiler failure, and 2 for a malformed command line or an invalid
//! or missing toolchain selection.

mod artifacts;
mod build;
mod cli;
mod docgen;
mod display;
mod emulator;
mod environment;
mod new;
pub mod platform;
mod run;
mod setup;
mod toolchain;

pub use emulator::fs_uae_version_for;
pub use new::launcher_for;
pub use setup::{validate_for, SetupError, Toolchain, Validation};
pub use toolchain::native_syscalls_for;

use std::ffi::{OsStr, OsString};
use std::path::Path;

use crate::diag::{format_error, format_from_args, registry, DiagnosticFormat};
use crate::profile::Profile;

use cli::{Action, Command, Options};
use platform::HostOs;

/// Exit code for command-line and environment problems.
const EXIT_USAGE: i32 = 2;

/// The tool-list name that stands for the host C compiler; the environment
/// gate replaces it with the compiler [`environment::resolve_host_compiler`]
/// picks.
const HOST_CC: &str = "cc";

/// The command a user runs to select an upstream toolchain.
pub(super) const SETUP_GUIDANCE: &str = "clorraine setup --toolchain PATH";

/// Runs the driver with the given arguments (without the program name).
///
/// Returns the process exit code.
pub fn run(args: Vec<String>) -> i32 {
    run_with_marker(args, environment::SDK_MARKER)
}

/// Like [`run`] but with an explicit SDK marker path.
///
/// The marker is a parameter so tests can exercise the outside-the-container
/// exit path without changing the filesystem layout.
fn run_with_marker(args: Vec<String>, marker: &str) -> i32 {
    let requested = format_from_args(&args);
    match cli::parse(&args) {
        Err(message) => {
            report_error(
                requested,
                &format!("usage error: {}", display::sanitize(&message)),
            );
            EXIT_USAGE
        }
        Ok(Action::Help) => {
            print!("{}", cli::help_text());
            0
        }
        Ok(Action::Version) => {
            println!("{}", cli::version_text());
            0
        }
        Ok(Action::Doc(options)) => docgen::run(&options),
        Ok(Action::Explain(code)) => match registry::explain(&code) {
            Some(text) => {
                print!("{text}");
                0
            }
            None => {
                report_error(
                    requested,
                    &format!(
                        "usage error: unknown diagnostic code '{}'",
                        display::sanitize(&code)
                    ),
                );
                EXIT_USAGE
            }
        },
        Ok(Action::Invoke(options)) => {
            let search_path = std::env::var_os("PATH").unwrap_or_default();
            invoke(options, marker, &search_path)
        }
    }
}

/// Checks the environment, then performs the selected command.
///
/// The marker path and the `PATH` value are parameters so the two environment
/// gates can be exercised with synthetic inputs.
fn invoke(options: Options, marker: &str, search_path: &OsStr) -> i32 {
    if options.command == Command::New {
        return match new::run(&options, search_path) {
            Ok(()) => 0,
            Err(error) => {
                report_error(options.diagnostics, &error.message);
                error.code
            }
        };
    }
    if options.command == Command::Setup {
        return match setup::run(&options) {
            Ok(()) => 0,
            Err(error) => {
                report_error(options.diagnostics, &error.message);
                error.code
            }
        };
    }
    let native = match environment::resolve_sdk(&options, marker) {
        Ok(native) => native,
        Err(error) => {
            report_error(options.diagnostics, &error.message);
            return error.code;
        }
    };
    if native.is_none() && (options.fs_uae.is_some() || options.kickstart.is_some()) {
        report_error(options.diagnostics,
            "usage error: --fs-uae and --kickstart require a native toolchain; select --toolchain or run 'clorraine setup --toolchain PATH'");
        return EXIT_USAGE;
    }
    let override_cc = std::env::var_os("CLORRAINE_CC");
    let host_cc = match environment_exit(&options, search_path, native.as_ref(), override_cc.as_deref()) {
        Ok(host_cc) => host_cc,
        Err((code, message)) => {
            report_error(options.diagnostics, &message);
            return code;
        }
    };
    // A native emulator selection is validated before the build, so a missing
    // emulator or ROM never produces a half-built program.
    let guest = if options.emulator && native.is_some() {
        match emulator::Emulator::prepare(&options, search_path) {
            Ok(prepared) => Some(prepared),
            Err(error) => {
                report_error(options.diagnostics, &error.message);
                return error.code;
            }
        }
    } else {
        None
    };
    let compilers = Compilers { native: native.as_ref(), host: host_cc.as_deref().map(Path::new) };
    match options.command {
        Command::Emit => emit(&options),
        Command::Build => build_command(&options, &compilers),
        Command::Run => run_command(options, &compilers, guest),
        Command::Setup | Command::New => {
            unreachable!("setup and new are handled before resolution")
        }
    }
}

/// The compilers one invocation builds with.
struct Compilers<'a> {
    /// The selected native Amiga toolchain, if any.
    native: Option<&'a environment::NativeSdk>,
    /// The host C compiler, when the command needs one.
    host: Option<&'a Path>,
}

impl Compilers<'_> {
    /// The compiler a build of `profile` uses in place of its default name.
    fn for_build(&self, profile: Profile) -> Option<&Path> {
        if profile == Profile::Host {
            self.host
        } else {
            self.native.map(|sdk| sdk.compiler.as_path())
        }
    }
}

/// The environment gate: `Err((code, message))` when the driver must stop.
///
/// A missing SDK marker stops a command that needs the cross toolchain (exit
/// 2); the host profile and `emit` work on a bare host, so they continue to
/// the tool check. A required tool that is missing or not executable on `PATH`
/// is reported by name (exit 1). The host C compiler is chosen by
/// [`environment::resolve_host_compiler`] at the point the tool list names
/// `cc`, and returned as `Ok(Some(program))`. Every input is a parameter so
/// unit tests need not touch the process environment.
fn environment_exit(
    options: &Options,
    search_path: &OsStr,
    native: Option<&environment::NativeSdk>,
    override_cc: Option<&OsStr>,
) -> Result<Option<OsString>, (i32, String)> {
    // A native compiler was checked as an absolute path. A freestanding run
    // still needs the host cc; a native Amiga run without FS-UAE needs vamos
    // from PATH. The legacy container branch checks every tool on PATH.
    let tools = if native.is_some() {
        let mut tools = Vec::new();
        if options.profile == Profile::Freestanding && options.command == Command::Run {
            tools.push(HOST_CC);
        }
        if matches!(options.profile, Profile::HostedAmiga | Profile::HostedAmiga13)
            && options.command == Command::Run
            && !options.emulator
        {
            tools.push("vamos");
        }
        tools
    } else {
        required_tools(options)
    };
    let mut host_cc = None;
    for tool in tools {
        if tool == HOST_CC {
            let chosen = environment::resolve_host_compiler(HostOs::current(), override_cc, search_path)
                .map_err(|error| (error.code, error.message))?;
            host_cc = Some(chosen);
        } else if let Some(missing) = environment::missing_tool_in(&[tool], search_path) {
            return Err((1, environment::missing_tool_message(&missing)));
        }
    }
    Ok(host_cc)
}

/// The external tools a command needs before it can run.
///
/// `emit` only calls the compiler in process. A build needs the profile's C
/// compiler; a hosted-amiga run also needs `vamos` unless the FS-UAE handoff
/// replaces it.
fn required_tools(options: &Options) -> Vec<&'static str> {
    let mut tools = Vec::new();
    match options.command {
        Command::Emit | Command::Setup | Command::New => {}
        Command::Build => tools.push(toolchain::tool(options.profile).program),
        Command::Run => {
            tools.push(toolchain::tool(options.profile).program);
            if options.profile == Profile::Freestanding {
                // The run leg compiles the same translation unit for the host
                // with `cc`, on top of the m68k build the run also performs.
                tools.push(HOST_CC);
            }
            if matches!(
                options.profile,
                Profile::HostedAmiga | Profile::HostedAmiga13
            ) && !options.emulator
            {
                tools.push("vamos");
            }
        }
    }
    tools
}

/// Emits the C and prints it, or writes it to `-o`.
///
/// The translation unit lands in a private scratch directory under the build
/// tree (see [`artifacts::create_scratch_c`]), which is removed before this
/// function returns.
fn emit(options: &Options) -> i32 {
    let (scratch_dir, temp) = match artifacts::create_scratch_c() {
        Ok(scratch) => scratch,
        Err(error) => {
            report_error(
                options.diagnostics,
                &format!(
                    "clorraine: cannot create a scratch file: {}",
                    display::sanitize(&error.to_string())
                ),
            );
            return 1;
        }
    };
    let result = emit_scratch(options, &temp);
    let _ = std::fs::remove_dir_all(&scratch_dir);
    result
}

/// Emits the C into `temp`, then prints it or copies it to the output path.
fn emit_scratch(options: &Options, temp: &std::path::Path) -> i32 {
    let code = build::emit_c(options.profile, &options.files, temp, options.diagnostics);
    if code != 0 {
        // The compiler already reported its diagnostics to stderr.
        return code;
    }
    let c = match std::fs::read_to_string(temp) {
        Ok(c) => c,
        Err(error) => {
            report_error(
                options.diagnostics,
                &format!(
                    "clorraine: cannot read the emitted C: {}",
                    display::sanitize(&error.to_string())
                ),
            );
            return 1;
        }
    };
    match &options.output {
        Some(path) => {
            if let Err(error) = std::fs::write(path, &c) {
                report_error(
                    options.diagnostics,
                    &format!(
                        "clorraine: cannot write '{}': {error}",
                        display::sanitize(path)
                    ),
                );
                return 1;
            }
            0
        }
        None => {
            print!("{c}");
            0
        }
    }
}

/// Builds the program and reports the artifact path.
fn build_command(options: &Options, compilers: &Compilers) -> i32 {
    let built = match build::build(
        options.profile,
        options.mode,
        &options.files,
        options.diagnostics,
        compilers.for_build(options.profile),
    ) {
        Ok(built) => built,
        Err(build::BuildFailure::Diagnostics(code)) => return code,
        Err(build::BuildFailure::Error(message)) => {
            report_error(options.diagnostics, &message);
            return 1;
        }
    };
    println!(
        "built {}",
        display::sanitize(&built.bin.display().to_string())
    );
    if options.emit_c {
        print_c(&built.c);
    }
    0
}

/// Builds, then runs the program (or hands it off to FS-UAE).
fn run_command(options: Options, compilers: &Compilers, guest: Option<emulator::Emulator>) -> i32 {
    if options.profile == Profile::Freestanding {
        return run_freestanding(options, compilers);
    }
    let built = match build::build(
        options.profile,
        options.mode,
        &options.files,
        options.diagnostics,
        compilers.for_build(options.profile),
    ) {
        Ok(built) => built,
        Err(build::BuildFailure::Diagnostics(code)) => return code,
        Err(build::BuildFailure::Error(message)) => {
            report_error(options.diagnostics, &message);
            return 1;
        }
    };
    if options.emit_c {
        print_c(&built.c);
    }
    if options.emulator {
        if let Some(prepared) = guest {
            // The verified runner needs the selected native compiler; the
            // selection check above guarantees it is present.
            let compiler = compilers
                .native
                .expect("a prepared emulator has a native toolchain")
                .compiler
                .as_path();
            let capture = if options.capture {
                artifacts::program_stem(&options.files)
                    .map(|stem| artifacts::out_path(options.profile, &stem))
            } else {
                None
            };
            let result = emulator::Emulator::run(
                prepared,
                compiler,
                &built.bin,
                capture.as_deref(),
                options.diagnostics,
            );
            if result.captured {
                if let Some(path) = &capture {
                    println!(
                        "captured {}",
                        display::sanitize(&path.display().to_string())
                    );
                }
            }
            return result.code;
        }
        // The marked SDK-container handoff leaves FS-UAE to the host.
        print!(
            "{}",
            toolchain::fs_uae_handoff(&built.bin, options.capture, options.profile)
        );
        return 0;
    }
    let runner = toolchain::runner(options.profile);
    let capture = if options.capture {
        artifacts::program_stem(&options.files).map(|stem| artifacts::out_path(options.profile, &stem))
    } else {
        None
    };
    let result = run::run_program(
        &runner,
        &built.bin,
        capture.as_deref(),
        options.diagnostics,
    );
    if result.captured {
        if let Some(path) = &capture {
            println!("captured {}", display::sanitize(&path.display().to_string()));
        }
    }
    result.code
}

/// Builds the m68k artifact, then compiles and runs the host no-libc artifact.
///
/// The freestanding profile has no operating system to run the m68k artifact
/// on, so `run` proves the program's semantics with the same translation unit
/// built for the host. The m68k build still runs first, so a `run` also proves
/// the target link. `--capture` writes the host run's output as usual.
fn run_freestanding(options: Options, compilers: &Compilers) -> i32 {
    let built = match build::build(
        options.profile,
        options.mode,
        &options.files,
        options.diagnostics,
        compilers.for_build(options.profile),
    ) {
        Ok(built) => built,
        Err(build::BuildFailure::Diagnostics(code)) => return code,
        Err(build::BuildFailure::Error(message)) => {
            report_error(options.diagnostics, &message);
            return 1;
        }
    };
    if options.emit_c {
        print_c(&built.c);
    }
    let stem = match artifacts::program_stem(&options.files) {
        Some(stem) => stem,
        None => {
            report_error(
                options.diagnostics,
                "clorraine: an input file name has no file stem",
            );
            return 1;
        }
    };
    let Some(tool) = toolchain::run_tool(options.profile, toolchain::native_syscalls()) else {
        report_error(
            options.diagnostics,
            "clorraine: the freestanding profile has no host run tool",
        );
        return 1;
    };
    let host_bin = artifacts::host_bin_path(options.profile, &stem);
    if let Err(ice) = build::compile(&tool, options.mode, &built.c, &host_bin, compilers.host) {
        report_error(options.diagnostics, &ice.to_string());
        return 1;
    }
    let capture = if options.capture {
        Some(artifacts::out_path(options.profile, &stem))
    } else {
        None
    };
    let result = run::run_program(
        &toolchain::Runner::Native,
        &host_bin,
        capture.as_deref(),
        options.diagnostics,
    );
    if result.captured {
        if let Some(path) = &capture {
            println!("captured {}", display::sanitize(&path.display().to_string()));
        }
    }
    result.code
}

/// Writes one driver-level error in the selected format.
///
/// Human mode preserves the exact existing bytes; JSON mode emits one object
/// line with a null code. Every user-facing error in this module goes through
/// here, so no path can silently keep the human shape under
/// `--diagnostics=json`.
fn report_error(format: DiagnosticFormat, message: &str) {
    eprint!("{}", format_error(format, message));
}

/// Prints an emitted translation unit to stdout, ignoring a read failure.
fn print_c(c_path: &std::path::Path) {
    if let Ok(c) = std::fs::read_to_string(c_path) {
        print!("{c}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;

    /// Builds an argument vector from string literals.
    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|item| (*item).to_owned()).collect()
    }

    /// The parsed invocation, panicking when the line is rejected.
    fn invoke_options(list: &[&str]) -> Options {
        match cli::parse(&args(list)) {
            Ok(Action::Invoke(options)) => options,
            other => panic!("expected an invocation, got {other:?}"),
        }
    }

    #[test]
    fn a_bare_host_runs_emit_and_the_host_profile_without_a_toolchain() {
        let emit = invoke_options(&["emit", "hello.clo"]);
        assert!(
            matches!(environment_exit(&emit, &OsString::new(), None, None), Ok(None)),
            "emit only needs the in-process compiler"
        );

        let host = invoke_options(&["run", "hello.clo"]);
        let (code, message) = environment_exit(&host, &OsString::new(), None, None)
            .expect_err("the host profile still needs cc");
        assert_eq!(code, 1, "a missing host tool is not a toolchain-selection error");
        assert!(message.contains("'cc'"), "the report names cc, got:\n{message}");
    }

    #[test]
    fn a_missing_required_tool_is_named_and_exits_one() {
        let options = invoke_options(&["run", "hello.clo"]);
        // The host profile needs `cc`; an empty search path finds nothing.
        let (code, message) = environment_exit(&options, &OsString::new(), None, None)
            .expect_err("a missing tool must stop the driver");
        assert_eq!(code, 1);
        assert!(
            message.contains("'cc'"),
            "the report names the missing tool, got:\n{message}"
        );
    }

    #[test]
    fn a_present_host_tool_proceeds() {
        let dir = std::env::temp_dir().join(format!("clorraine-gate-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("create the search directory");
        let tool = dir.join("cc");
        std::fs::write(&tool, b"").expect("write the tool file");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut permissions = std::fs::metadata(&tool)
                .expect("the tool file exists")
                .permissions();
            permissions.set_mode(0o755);
            std::fs::set_permissions(&tool, permissions).expect("set the execute bit");
        }

        let options = invoke_options(&["build", "hello.clo"]);
        let gate = environment_exit(&options, &OsString::from(dir.as_os_str()), None, None);
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(gate, Ok(Some(OsString::from("cc"))), "a ready tool must let the command through");
    }
}
