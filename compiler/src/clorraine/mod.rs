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
//! or a C compiler failure, and 2 for a malformed command line or a process
//! running outside the SDK container.

mod artifacts;
mod build;
mod cli;
mod environment;
mod run;
mod toolchain;

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::profile::Profile;

use cli::{Action, Command, Options};

/// Exit code for command-line and environment problems.
const EXIT_USAGE: i32 = 2;

/// Runs the driver with the given arguments (without the program name).
///
/// Returns the process exit code.
pub fn run(args: Vec<String>) -> i32 {
    match cli::parse(&args) {
        Err(message) => {
            eprintln!("usage error: {message}");
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
        Ok(Action::Invoke(options)) => invoke(options),
    }
}

/// Checks the environment, then performs the selected command.
fn invoke(options: Options) -> i32 {
    if !environment::sdk_present(environment::SDK_MARKER) {
        eprintln!("{}", environment::sdk_guidance());
        return EXIT_USAGE;
    }
    let tools = required_tools(&options);
    if let Some(tool) = environment::missing_tool(&tools) {
        eprintln!("{}", environment::missing_tool_message(&tool));
        return 1;
    }
    match options.command {
        Command::Emit => emit(&options),
        Command::Build => build_command(&options),
        Command::Run => run_command(options),
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
            if options.profile == Profile::HostedAmiga && !options.emulator {
                tools.push("vamos");
            }
        }
    }
    tools
}

/// Emits the C and prints it, or writes it to `-o`.
fn emit(options: &Options) -> i32 {
    let temp = temp_c_path();
    let code = build::emit_c(options.profile, &options.files, &temp);
    if code != 0 {
        // The compiler already reported its diagnostics to stderr.
        return code;
    }
    let c = match std::fs::read_to_string(&temp) {
        Ok(c) => c,
        Err(error) => {
            eprintln!("clorraine: cannot read the emitted C: {error}");
            return 1;
        }
    };
    let _ = std::fs::remove_file(&temp);
    match &options.output {
        Some(path) => {
            if let Err(error) = std::fs::write(path, &c) {
                eprintln!("clorraine: cannot write '{path}': {error}");
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
    let built = match build::build(options.profile, options.release, &options.files) {
        Ok(built) => built,
        Err(build::BuildFailure::Diagnostics(code)) => return code,
        Err(build::BuildFailure::Error(message)) => {
            eprint!("{message}");
            return 1;
        }
    };
    println!("built {}", built.bin.display());
    if options.emit_c {
        print_c(&built.c);
    }
    0
}

/// Builds, then runs the program (or hands it off to FS-UAE).
fn run_command(options: Options) -> i32 {
    let built = match build::build(options.profile, options.release, &options.files) {
        Ok(built) => built,
        Err(build::BuildFailure::Diagnostics(code)) => return code,
        Err(build::BuildFailure::Error(message)) => {
            eprint!("{message}");
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
    let code = run::run_program(&runner, &built.bin, capture.as_deref());
    if let Some(path) = &capture {
        println!("captured {}", path.display());
    }
    code
}

/// Prints an emitted translation unit to stdout, ignoring a read failure.
fn print_c(c_path: &std::path::Path) {
    if let Ok(c) = std::fs::read_to_string(c_path) {
        print!("{c}");
    }
}

/// A unique temporary path for the `emit` command's C file.
///
/// `emit` does not need the artifact directory; it writes the C somewhere
/// neutral and either prints or copies it, then removes it.
fn temp_c_path() -> PathBuf {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("clorraine-{}-{sequence}.c", std::process::id()))
}
