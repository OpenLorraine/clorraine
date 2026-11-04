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
//! or a C compiler failure, and 2 for a malformed command line or a command
//! that needs the SDK container but runs outside it.

mod artifacts;
mod build;
mod cli;
mod display;
mod environment;
mod run;
mod toolchain;

use std::ffi::OsStr;

use crate::diag::{format_error, format_from_args, registry, DiagnosticFormat};
use crate::profile::Profile;

use cli::{Action, Command, Options};

/// Exit code for command-line and environment problems.
const EXIT_USAGE: i32 = 2;

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
    if let Some((code, message)) = environment_exit(&options, marker, search_path) {
        report_error(options.diagnostics, &message);
        return code;
    }
    match options.command {
        Command::Emit => emit(&options),
        Command::Build => build_command(&options),
        Command::Run => run_command(options),
    }
}

/// The environment gate: `Some((code, message))` when the driver must stop.
///
/// A missing SDK marker stops a command that needs the cross toolchain (exit
/// 2); the host profile and `emit` work on a bare host, so they continue to
/// the tool check. A required tool that is missing or not executable on `PATH`
/// is reported by name (exit 1). Both inputs are parameters so unit tests need
/// not touch the process environment.
fn environment_exit(options: &Options, marker: &str, search_path: &OsStr) -> Option<(i32, String)> {
    if sdk_required(options) && !environment::sdk_present(marker) {
        return Some((EXIT_USAGE, environment::sdk_guidance()));
    }
    environment::missing_tool_in(&required_tools(options), search_path)
        .map(|tool| (1, environment::missing_tool_message(&tool)))
}

/// True when the selected command needs the pinned SDK image.
///
/// `emit` only runs the in-process compiler, and the host profile builds with
/// the host `cc`, so both work on a bare host. The Amiga and freestanding
/// profiles need the SDK cross toolchain and emulator.
fn sdk_required(options: &Options) -> bool {
    match options.command {
        Command::Emit => false,
        Command::Build | Command::Run => options.profile != Profile::Host,
    }
}

/// The external tools a command needs before it can run.
///
/// `emit` only calls the compiler in process. A build needs the profile's C
/// compiler; a hosted-amiga run also needs `vamos` unless the FS-UAE handoff
/// replaces it.
fn required_tools(options: &Options) -> Vec<&'static str> {
    let mut tools = Vec::new();
    match options.command {
        Command::Emit => {}
        Command::Build => tools.push(toolchain::tool(options.profile).program),
        Command::Run => {
            tools.push(toolchain::tool(options.profile).program);
            if options.profile == Profile::Freestanding {
                // The run leg compiles the same translation unit for the host
                // with `cc`, on top of the m68k build the run also performs.
                tools.push("cc");
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
fn build_command(options: &Options) -> i32 {
    let built = match build::build(
        options.profile,
        options.release,
        &options.files,
        options.diagnostics,
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
fn run_command(options: Options) -> i32 {
    if options.profile == Profile::Freestanding {
        return run_freestanding(options);
    }
    let built = match build::build(
        options.profile,
        options.release,
        &options.files,
        options.diagnostics,
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
        // The run is handed to the host; printing the command is success.
        print!("{}", toolchain::fs_uae_handoff(&built.bin, options.capture));
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
fn run_freestanding(options: Options) -> i32 {
    let built = match build::build(
        options.profile,
        options.release,
        &options.files,
        options.diagnostics,
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
    if let Err(ice) = build::compile(&tool, options.release, &built.c, &host_bin) {
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

    /// A marker path that exists on every host, so the environment gate passes
    /// and the tool check is the one under test.
    const PRESENT_MARKER: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml");

    #[test]
    fn an_absent_sdk_marker_exits_two_and_prints_guidance() {
        let options = invoke_options(&["run", "--profile", "hosted-amiga", "hello.clo"]);
        let (code, message) = environment_exit(&options, "/no/such/sdk-marker", &OsString::new())
            .expect("an absent marker must stop an Amiga command");
        assert_eq!(code, EXIT_USAGE);
        assert!(
            message.contains("SDK container"),
            "the guidance names the SDK container, got:\n{message}"
        );
        // The whole run entry point returns the same code.
        assert_eq!(
            run_with_marker(
                args(&["run", "--profile", "hosted-amiga", "hello.clo"]),
                "/no/such/sdk-marker"
            ),
            EXIT_USAGE
        );
    }

    #[test]
    fn a_bare_host_runs_emit_and_the_host_profile_without_the_marker() {
        let emit = invoke_options(&["emit", "hello.clo"]);
        assert!(
            environment_exit(&emit, "/no/such/sdk-marker", &OsString::new()).is_none(),
            "emit only needs the in-process compiler"
        );

        let host = invoke_options(&["run", "hello.clo"]);
        let (code, message) = environment_exit(&host, "/no/such/sdk-marker", &OsString::new())
            .expect("the host profile still needs cc");
        assert_eq!(code, 1, "a missing host tool is not an SDK-marker error");
        assert!(message.contains("'cc'"), "the report names cc, got:\n{message}");
    }

    #[test]
    fn a_missing_required_tool_is_named_and_exits_one() {
        let options = invoke_options(&["run", "hello.clo"]);
        // The host profile needs `cc`; an empty search path finds nothing.
        let (code, message) = environment_exit(&options, PRESENT_MARKER, &OsString::new())
            .expect("a missing tool must stop the driver");
        assert_eq!(code, 1);
        assert!(
            message.contains("'cc'"),
            "the report names the missing tool, got:\n{message}"
        );
    }

    #[test]
    fn a_present_marker_and_an_executable_tool_proceed() {
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
        let gate = environment_exit(&options, PRESENT_MARKER, &OsString::from(dir.as_os_str()));
        let _ = std::fs::remove_dir_all(&dir);
        assert!(gate.is_none(), "a ready tool must let the command through");
    }
}
