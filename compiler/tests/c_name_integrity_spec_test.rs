//! Specification tests for C name integrity.
//!
//! Clorraine identifiers follow the Clorraine lexical rules, not C's. A name
//! that is legal in Clorraine can still be a C keyword (`int`, `restrict`,
//! `_Bool`), an identifier C reserves for the implementation (`__LINE__`,
//! `_Pragma`), or a name that the emitted translation unit's own headers and
//! runtime define or declare (`NULL`, `errno`, `memcpy`, `SysBase`,
//! `FreeString`). The backend must still write valid C99 for every such
//! program, and the program must print exactly what its source says on every
//! profile of the compile matrix: the host build, and `hosted-amiga` and
//! `hosted-amiga-13` under `vamos`, each with the strict warning flags. Some
//! cases also build the `hosted-amiga` unit with vbcc, the second m68k
//! toolchain.
//!
//! Every fixture is a valid program. The front end must accept it with no
//! diagnostic at all, so a failure always comes from the emitted C or from the
//! program's output, never from a rejected source.
//!
//! Two guards pin behaviour that must not change: a name outside every escape
//! class keeps its exact source spelling and the emission stays
//! byte-identical, and an `ffi` function used as a function value still calls
//! the real C symbol.
//!
//! The m68k legs need `m68k-amigaos-gcc` and `vamos`, and the vbcc leg needs
//! `vc`. Each leg skips itself with a `skip:` note when its tools are absent,
//! so the host leg always runs.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

/// How long one host program may run before its leg fails as a hang.
const RUN_TIMEOUT: Duration = Duration::from_secs(30);

/// How many seconds `timeout` lets one `vamos` run take.
const VAMOS_TIMEOUT_SECS: &str = "120";

/// How many lines of a failing tool's output a failure message quotes.
const EXCERPT_LINES: usize = 40;

/// Temporary directory that removes itself when the test finishes.
struct ScratchDir {
    path: PathBuf,
}

impl ScratchDir {
    /// Creates a unique directory for one test process and tag.
    fn new(tag: &str) -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "clc-cnames-{}-{tag}-{sequence}",
            std::process::id()
        ));
        std::fs::create_dir_all(&path)
            .unwrap_or_else(|err| panic!("cannot create {}: {err}", path.display()));
        Self { path }
    }

    /// Path of the scratch directory.
    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for ScratchDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

/// Path of one program in this suite's fixture directory.
fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/c_name_integrity")
        .join(name)
}

/// Decodes tool output as text, dropping the carriage returns `vamos` writes.
fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).replace('\r', "")
}

/// The first lines of a tool's output, enough to name the first error.
fn excerpt(output: &str) -> String {
    let lines: Vec<&str> = output.lines().collect();
    if lines.len() <= EXCERPT_LINES {
        return lines.join("\n");
    }
    format!(
        "{}\n... ({} more lines)",
        lines[..EXCERPT_LINES].join("\n"),
        lines.len() - EXCERPT_LINES
    )
}

/// True when `tool` names a file in one of the PATH directories.
fn tool_available(tool: &str) -> bool {
    match std::env::var_os("PATH") {
        Some(path) => std::env::split_paths(&path).any(|dir| dir.join(tool).is_file()),
        None => false,
    }
}

/// A target profile of the compile matrix.
#[derive(Clone, Copy, Debug)]
enum Profile {
    Host,
    HostedAmiga,
    HostedAmiga13,
}

impl Profile {
    /// The spelling `clc --profile` accepts.
    fn name(self) -> &'static str {
        match self {
            Profile::Host => "host",
            Profile::HostedAmiga => "hosted-amiga",
            Profile::HostedAmiga13 => "hosted-amiga-13",
        }
    }
}

/// One leg of the compile matrix a case is checked on.
#[derive(Clone, Copy, Debug)]
enum Leg {
    /// Build with the host C compiler under the strict flags, run, and compare.
    Host,
    /// Build `hosted-amiga` with m68k GCC and libnix 2.0, run under `vamos`.
    HostedAmiga,
    /// Build `hosted-amiga-13` with m68k GCC and libnix 1.3, run under `vamos`.
    HostedAmiga13,
    /// Build the `hosted-amiga` unit with vbcc; build-only.
    Vbcc,
}

/// The three profiles every run case builds and runs on.
const RUN_LEGS: &[Leg] = &[Leg::Host, Leg::HostedAmiga, Leg::HostedAmiga13];

/// The three run legs plus the vbcc build of the `hosted-amiga` unit.
const RUN_AND_VBCC_LEGS: &[Leg] = &[Leg::Host, Leg::HostedAmiga, Leg::HostedAmiga13, Leg::Vbcc];

/// Emits one translation unit for `profile` from `sources` into `c_path`.
///
/// The first source is the entry; the rest follow in order, exactly as a
/// multi-file program is compiled. The front end must accept the program with
/// exit status 0 and print nothing; anything else fails the case here, before
/// any C compiler runs, so a rejected fixture is never mistaken for a backend
/// failure.
fn emit(tag: &str, profile: Profile, sources: &[PathBuf], c_path: &Path) {
    assert!(!sources.is_empty(), "{tag}: no source file");
    let output = Command::new(env!("CARGO_BIN_EXE_clc"))
        .args(["--profile", profile.name(), "-o"])
        .arg(c_path)
        .args(sources)
        .output()
        .expect("failed to spawn the compiler binary");
    assert_eq!(
        output.status.code(),
        Some(0),
        "{tag}: the front end rejected the fixture on {}; stderr:\n{}",
        profile.name(),
        text(&output.stderr)
    );
    assert!(
        output.stderr.is_empty(),
        "{tag}: the fixture must compile with zero diagnostics on {}; stderr:\n{}",
        profile.name(),
        text(&output.stderr)
    );
}

/// Emits `sources` for `profile` into `c_path` and returns the unit's bytes.
fn emit_bytes(tag: &str, profile: Profile, sources: &[PathBuf], c_path: &Path) -> Vec<u8> {
    emit(tag, profile, sources, c_path);
    std::fs::read(c_path)
        .unwrap_or_else(|err| panic!("{tag}: cannot read {}: {err}", c_path.display()))
}

/// Checks one finished run: exit status 0 and exactly the expected stdout.
fn check_run(leg: &str, code: Option<i32>, stdout: &str, stderr: &str, expected: &str) -> Result<(), String> {
    if stdout != expected {
        return Err(format!(
            "{leg}: wrong output: expected {expected:?}, got {stdout:?}; exit {code:?}; stderr:\n{}",
            excerpt(stderr)
        ));
    }
    if code != Some(0) {
        return Err(format!(
            "{leg}: expected exit status 0, got {code:?}; stderr:\n{}",
            excerpt(stderr)
        ));
    }
    Ok(())
}

/// Runs a built host program in `dir` and stops it if it outlives the limit.
fn run_bounded(program: &Path, dir: &Path) -> Result<Output, String> {
    let mut child = Command::new(program)
        .current_dir(dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|err| format!("host: cannot run the program: {err}"))?;
    let deadline = Instant::now() + RUN_TIMEOUT;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(20));
            }
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("host: the program did not finish within {RUN_TIMEOUT:?}"));
            }
            Err(err) => return Err(format!("host: cannot wait for the program: {err}")),
        }
    }
    child
        .wait_with_output()
        .map_err(|err| format!("host: cannot collect the program output: {err}"))
}

/// The host leg: emit, build with `cc -std=c99 -O0 -g -Wall -Wextra -Werror`,
/// run, and compare stdout and the exit status.
fn host_leg(scratch: &ScratchDir, tag: &str, sources: &[PathBuf], expected: &str) -> Result<(), String> {
    let c_path = scratch.path().join("host.c");
    emit(tag, Profile::Host, sources, &c_path);
    let program = scratch.path().join("host-program");
    let built = Command::new("cc")
        .args(["-std=c99", "-O0", "-g", "-Wall", "-Wextra", "-Werror", "-o"])
        .arg(&program)
        .arg(&c_path)
        .output()
        .expect("failed to spawn the host C compiler");
    if !built.status.success() {
        return Err(format!(
            "host: the C compiler rejected the emitted C:\n{}",
            excerpt(&text(&built.stderr))
        ));
    }
    let run = run_bounded(&program, scratch.path())?;
    check_run("host", run.status.code(), &text(&run.stdout), &text(&run.stderr), expected)
}

/// An m68k leg: emit for `profile`, build with the debug line of the compile
/// matrix and the given libnix runtime, run under a bounded `vamos` in the
/// program's directory, and compare stdout and the exit status.
fn m68k_leg(
    scratch: &ScratchDir,
    tag: &str,
    profile: Profile,
    crt: &str,
    sources: &[PathBuf],
    expected: &str,
) -> Result<(), String> {
    let leg = profile.name();
    let c_path = scratch.path().join(format!("{leg}.c"));
    emit(tag, profile, sources, &c_path);
    let binary = scratch.path().join(format!("{leg}-program"));
    let built = Command::new("m68k-amigaos-gcc")
        .arg("-std=c99")
        .arg(format!("-mcrt={crt}"))
        .args(["-m68000", "-fwrapv", "-O0", "-g", "-Wall", "-Wextra", "-Werror", "-o"])
        .arg(&binary)
        .arg(&c_path)
        .args(["-lamiga", "-lm"])
        .output()
        .expect("failed to spawn m68k-amigaos-gcc");
    if !built.status.success() {
        return Err(format!(
            "{leg}: m68k-amigaos-gcc rejected the emitted C:\n{}",
            excerpt(&text(&built.stderr))
        ));
    }
    let run = Command::new("timeout")
        .arg(VAMOS_TIMEOUT_SECS)
        .arg("vamos")
        .arg(&binary)
        .current_dir(scratch.path())
        .stdin(Stdio::null())
        .output()
        .expect("failed to spawn vamos");
    check_run(leg, run.status.code(), &text(&run.stdout), &text(&run.stderr), expected)
}

/// The vbcc leg: emit for `hosted-amiga` and build it with the vbcc line of
/// the compile matrix. Only the exit status counts, because the NDK headers
/// carry vbcc warnings of their own. vbcc sometimes reports `No config file!`
/// on its first run in a fresh container; that failure is retried once.
fn vbcc_leg(scratch: &ScratchDir, tag: &str, sources: &[PathBuf]) -> Result<(), String> {
    let c_path = scratch.path().join("vbcc.c");
    emit(tag, Profile::HostedAmiga, sources, &c_path);
    let binary = scratch.path().join("vbcc-program");
    let attempt = || {
        Command::new("vc")
            .args([
                "+aos68k",
                "-c99",
                "-O2",
                "-cpu=68000",
                "-I/opt/amiga/m68k-amigaos/ndk-include",
                "-o",
            ])
            .arg(&binary)
            .arg(&c_path)
            .args(["-lamiga", "-lmieee"])
            .current_dir(scratch.path())
            .stdin(Stdio::null())
            .output()
            .expect("failed to spawn vc")
    };
    let combined = |output: &Output| format!("{}{}", text(&output.stdout), text(&output.stderr));
    let mut built = attempt();
    if !built.status.success() && combined(&built).contains("No config file!") {
        built = attempt();
    }
    if built.status.success() {
        return Ok(());
    }
    Err(format!(
        "vbcc: vc rejected the emitted hosted-amiga C:\n{}",
        excerpt(&combined(&built))
    ))
}

/// Checks `sources` on every leg in `legs` and fails with every failing leg.
///
/// A run leg must build under the strict flags, exit 0, and print exactly
/// `expected`; the vbcc leg must build. The m68k legs skip with a `skip:` note
/// when `m68k-amigaos-gcc` or `vamos` is absent, and the vbcc leg when `vc` is
/// absent. Every leg runs even after an earlier one fails, so one report names
/// every leg the case breaks on.
fn assert_legs(tag: &str, sources: &[PathBuf], expected: &str, legs: &[Leg]) {
    let scratch = ScratchDir::new(tag);
    let m68k = tool_available("m68k-amigaos-gcc") && tool_available("vamos");
    let vbcc = tool_available("vc");
    let mut failures = Vec::new();
    for &leg in legs {
        let outcome = match leg {
            Leg::Host => host_leg(&scratch, tag, sources, expected),
            Leg::HostedAmiga | Leg::HostedAmiga13 if !m68k => {
                eprintln!("skip: {tag}: m68k-amigaos-gcc or vamos is absent; the {leg:?} leg did not run");
                Ok(())
            }
            Leg::HostedAmiga => m68k_leg(&scratch, tag, Profile::HostedAmiga, "nix20", sources, expected),
            Leg::HostedAmiga13 => {
                m68k_leg(&scratch, tag, Profile::HostedAmiga13, "nix13", sources, expected)
            }
            Leg::Vbcc if !vbcc => {
                eprintln!("skip: {tag}: vc is absent; the vbcc leg did not run");
                Ok(())
            }
            Leg::Vbcc => vbcc_leg(&scratch, tag, sources),
        };
        if let Err(message) = outcome {
            failures.push(message);
        }
    }
    assert!(
        failures.is_empty(),
        "{tag}: {} of {} legs failed:\n\n{}",
        failures.len(),
        legs.len(),
        failures.join("\n\n")
    );
}

/// The identifier tokens of a C text: maximal runs of letters, digits, and
/// `_` that do not start with a digit.
fn identifier_tokens(c: &str) -> HashSet<&str> {
    c.split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_'))
        .filter(|token| token.chars().next().is_some_and(|first| !first.is_ascii_digit()))
        .collect()
}

// ---------------------------------------------------------------------------
// Names that C reserves or that the emitted unit already declares
// ---------------------------------------------------------------------------

/// Locals spelled like C keywords (`int`, `char`, `restrict`, `inline`,
/// `union`, `_Bool`, ...) and a function `register` with a parameter `auto`
/// are ordinary Clorraine names; the program sums them and prints 121.
#[test]
fn should_build_and_run_on_every_leg_when_locals_are_named_after_c_keywords() {
    assert_legs(
        "keywords",
        &[fixture("keywords_as_names.clo")],
        "121\n",
        RUN_AND_VBCC_LEGS,
    );
}

/// Locals spelled like C library and NDK macros, typedefs, and globals
/// (`NULL`, `EOF`, `errno`, `stdout`, `INT_MAX`, `MEMF_CHIP`, `BYTE`, `APTR`,
/// `ULONG`, `SysBase`, `IntuitionBase`, ...) print their sum, 120.
#[test]
fn should_build_and_run_on_every_leg_when_locals_are_named_after_header_macros_and_globals() {
    assert_legs(
        "header-names",
        &[fixture("header_macro_names.clo")],
        "120\n",
        RUN_AND_VBCC_LEGS,
    );
}

/// Locals spelled like identifiers C reserves for the implementation
/// (`__LINE__`, `__func__`, `__STDC__`, `_Pragma`, `__chip`, `_Complex`,
/// `_Imaginary`) print their sum, 28.
#[test]
fn should_build_and_run_on_every_leg_when_locals_use_c_reserved_identifiers() {
    assert_legs(
        "reserved",
        &[fixture("reserved_identifiers.clo")],
        "28\n",
        RUN_AND_VBCC_LEGS,
    );
}

/// Locals named `memcpy`, `memset`, and `strlen` sit in the same function as
/// an array copy and a copy of a struct that holds the array; the copies keep
/// their values and the program prints 13.
#[test]
fn should_copy_arrays_and_structs_when_locals_are_named_after_memory_routines() {
    assert_legs(
        "memory-names",
        &[fixture("memory_routine_names.clo")],
        "13\n",
        RUN_LEGS,
    );
}

/// Locals named after Amiga structure fields, host macros, the libnix runtime,
/// and the `Close` and `FreeString` deleters sit in a function that creates an
/// owned `String` and drops it; the program prints 28.
#[test]
fn should_create_and_drop_a_string_when_locals_are_named_after_runtime_symbols() {
    assert_legs(
        "runtime-names",
        &[fixture("runtime_symbol_names.clo")],
        "28\n",
        RUN_LEGS,
    );
}

/// Parameters named `EOF`, `restrict`, and `errno`, and a capture-less lambda
/// parameter named `TRUE`, carry their arguments; the program prints `6 5`.
#[test]
fn should_pass_arguments_when_parameters_are_named_after_c_macros_and_qualifiers() {
    assert_legs(
        "parameter-names",
        &[fixture("parameter_and_lambda_names.clo")],
        "6 5\n",
        RUN_LEGS,
    );
}

// ---------------------------------------------------------------------------
// Behaviour that must not change
// ---------------------------------------------------------------------------

/// Locals `count` and `bufferSize`, a parameter `width`, and a struct field
/// `height` are in no escape class, so each keeps its exact source spelling in
/// the emitted C and gains no suffix. Emitting the same file twice with the
/// same profile and flags gives byte-identical C.
#[test]
fn should_keep_source_spelling_and_identical_bytes_when_no_name_needs_escaping() {
    let sources = [fixture("plain_names.clo")];
    for profile in [Profile::Host, Profile::HostedAmiga] {
        let tag = format!("plain-{}", profile.name());
        let scratch = ScratchDir::new(&tag);
        let c_path = scratch.path().join("out.c");
        let first = emit_bytes(&tag, profile, &sources, &c_path);
        let second = emit_bytes(&tag, profile, &sources, &c_path);
        assert!(
            first == second,
            "{tag}: two emissions of the same file with the same profile and flags differ"
        );
        let c = String::from_utf8_lossy(&first);
        let tokens = identifier_tokens(&c);
        for name in ["count", "bufferSize", "width", "height"] {
            assert!(
                tokens.contains(name),
                "{tag}: the emitted C does not keep the source name `{name}` as an identifier"
            );
            let suffixed = format!("{name}_0");
            assert!(
                !c.contains(&suffixed),
                "{tag}: the emitted C renames the plain name `{name}` to `{suffixed}`"
            );
        }
    }
}

/// An `ffi` function bound to a function value keeps its literal C symbol, so
/// calling the value calls the C library's `abs` and prints 5.
#[test]
fn should_call_the_c_symbol_when_an_ffi_function_is_used_as_a_function_value() {
    assert_legs(
        "ffi-value",
        &[fixture("ffi_function_value.clo")],
        "5\n",
        &[Leg::Host],
    );
}
