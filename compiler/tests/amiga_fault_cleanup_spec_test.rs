//! Specification tests for releasing AmigaOS resources when a hosted-amiga
//! program ends abnormally.
//!
//! A runtime fault, a `panic`, or a failed `assert` skips every Clorraine drop,
//! but the runtime must still hand back the OS resources the program holds:
//! locks, ports, messages, and library opens. `vamos` reports what is left
//! behind when the program ends, as orphaned memory or a library it cannot
//! expunge, so each probe faults with resources live and must leave no such
//! report. Every probe runs on both hosted profiles.
//!
//! A clean exit must print nothing extra, and a program that never acquires a
//! resource must not link the exit machinery at all.
//!
//! The cases need the m68k cross toolchain and `vamos`; they skip themselves
//! with a note when those tools are absent, like the other hosted-amiga suites.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

/// The exit code the hosted-amiga runtime leaves with after a fault or panic.
const FAULT_EXIT: i32 = 20;

/// The two hosted profiles and the libnix runtime each one links.
const PROFILES: [(&str, &str); 2] = [("hosted-amiga", "nix20"), ("hosted-amiga-13", "nix13")];

/// Temporary directory that removes itself when the test finishes.
struct ScratchDir {
    path: PathBuf,
}

impl ScratchDir {
    fn new(tag: &str) -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "clorraine-fault-cleanup-{}-{tag}-{sequence}",
            std::process::id()
        ));
        std::fs::create_dir_all(&path)
            .unwrap_or_else(|error| panic!("cannot create {}: {error}", path.display()));
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for ScratchDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn probe(name: &str) -> PathBuf {
    repo_root()
        .join("examples/robustness/fault-cleanup")
        .join(name)
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).replace('\r', "")
}

fn tool_available(tool: &str) -> bool {
    Command::new(tool).arg("--version").output().is_ok()
}

/// True when the m68k compiler and `vamos` are both on PATH.
fn sdk_present() -> bool {
    if tool_available("m68k-amigaos-gcc") && tool_available("vamos") {
        return true;
    }
    eprintln!("skip: the hosted-amiga toolchain or vamos is absent");
    false
}

/// How a probe is compiled: the debug line of the compile matrix with warnings
/// as errors, or the release line without the final strip so the symbol table
/// can be inspected.
#[derive(Clone, Copy)]
enum Build {
    Debug,
    ReleaseUnstripped,
}

/// Emits `source` for `profile` with zero diagnostics and cross-builds it with
/// the named libnix runtime. Returns the program path inside `scratch`.
fn build(scratch: &ScratchDir, tag: &str, profile: &str, crt: &str, source: &Path, how: Build) -> PathBuf {
    let c_path = scratch.path().join(format!("{tag}.c"));
    let emit = Command::new(env!("CARGO_BIN_EXE_clc"))
        .args(["--profile", profile, "-o"])
        .arg(&c_path)
        .arg(source)
        .output()
        .expect("run clc");
    assert!(
        emit.status.success(),
        "{tag}: clc failed; stderr:\n{}",
        text(&emit.stderr)
    );
    assert!(
        emit.stderr.is_empty(),
        "{tag}: the probe must compile with zero diagnostics; stderr:\n{}",
        text(&emit.stderr)
    );
    let binary = scratch.path().join(tag);
    let mut args: Vec<String> = vec![
        "-std=c99".into(),
        format!("-mcrt={crt}"),
        "-m68000".into(),
        "-fwrapv".into(),
    ];
    match how {
        Build::Debug => {
            for flag in ["-O0", "-g", "-Wall", "-Wextra", "-Werror"] {
                args.push(flag.into());
            }
        }
        Build::ReleaseUnstripped => {
            for flag in ["-O2", "-ffunction-sections", "-fdata-sections", "-Wl,--gc-sections"] {
                args.push(flag.into());
            }
        }
    }
    args.push("-o".into());
    args.push(binary.display().to_string());
    args.push(c_path.display().to_string());
    args.push("-lamiga".into());
    args.push("-lm".into());
    let gcc = Command::new("m68k-amigaos-gcc")
        .args(&args)
        .output()
        .expect("run m68k-amigaos-gcc");
    assert!(
        gcc.status.success(),
        "{tag}: m68k-amigaos-gcc failed; stderr:\n{}",
        text(&gcc.stderr)
    );
    binary
}

/// What one `vamos` run produced.
struct Run {
    code: Option<i32>,
    stdout: String,
    /// Everything `vamos` and the program wrote, stdout and stderr together.
    all: String,
}

/// Runs `binary` under `vamos` in its own directory, bounded so a hang fails
/// the case instead of the whole suite.
fn run_vamos(binary: &Path) -> Run {
    let dir = binary.parent().expect("the program has a directory");
    let output: Output = Command::new("timeout")
        .arg("120")
        .arg("vamos")
        .arg(binary)
        .current_dir(dir)
        .output()
        .expect("run vamos");
    let stdout = text(&output.stdout);
    let all = format!("{stdout}{}", text(&output.stderr));
    Run {
        code: output.status.code(),
        stdout,
        all,
    }
}

/// True for a `vamos` report of a resource the program left behind: a library
/// or device it cannot expunge at shutdown, or memory nobody freed.
fn is_leak_line(line: &str) -> bool {
    if line.contains("can't expunge") {
        return true;
    }
    match line.find("mem_alloc:WARNING:") {
        Some(at) => {
            let rest = &line[at + "mem_alloc:WARNING:".len()..];
            rest.starts_with(char::is_whitespace) && rest.trim_start().starts_with("orphan")
        }
        None => false,
    }
}

/// Builds `probe_name` on both profiles, runs it, and asserts the fault line,
/// the fault exit code, and that nothing was left behind.
fn assert_faults_cleanly(probe_name: &str, first_line: &str) {
    assert_faults_cleanly_on(probe_name, first_line, &PROFILES);
}

/// The same check on the given profiles only.
fn assert_faults_cleanly_on(probe_name: &str, first_line: &str, profiles: &[(&str, &str)]) {
    if !sdk_present() {
        return;
    }
    for &(profile, crt) in profiles {
        let tag = format!("{}-{profile}", probe_name.trim_end_matches(".clo"));
        let scratch = ScratchDir::new(&tag);
        let binary = build(&scratch, &tag, profile, crt, &probe(probe_name), Build::Debug);
        let run = run_vamos(&binary);
        assert_eq!(
            run.stdout.lines().next(),
            Some(first_line),
            "{tag}: the first output line must be the fault line; output:\n{}",
            run.all
        );
        assert_eq!(
            run.code,
            Some(FAULT_EXIT),
            "{tag}: the program must leave with the fault exit code; output:\n{}",
            run.all
        );
        let leaks: Vec<&str> = run.all.lines().filter(|line| is_leak_line(line)).collect();
        assert!(
            leaks.is_empty(),
            "{tag}: the runtime left resources behind:\n{}\nfull output:\n{}",
            leaks.join("\n"),
            run.all
        );
    }
}

// A DOS lock and a path-info block are both live when an index goes out of
// range; both must be released on the way out.
#[test]
fn a_lock_and_a_path_info_are_released_when_an_index_faults() {
    assert_faults_cleanly("lock-fault.clo", "error E11211: index out of range");
}

// A private and a public port are live when a division by zero faults; both
// must be deleted.
#[test]
fn private_and_public_ports_are_deleted_when_a_division_faults() {
    assert_faults_cleanly("port-fault.clo", "error E11210: integer division by zero");
}

// A message the program created is still queued on one of its own ports. No
// other task can reply to it, so it must be freed together with both ports.
#[test]
fn a_queued_own_message_is_freed_when_the_program_faults() {
    assert_faults_cleanly("message-fault.clo", "error E11211: index out of range");
}

// A message the program created was fetched and replied, so the reply waits on
// the program's own reply port when the fault happens. It must still be freed.
#[test]
fn a_replied_own_message_is_freed_when_the_program_faults() {
    assert_faults_cleanly("reply-fault.clo", "error E11211: index out of range");
}

// A library the program opened is still open at the fault; it must be closed
// so every library can be expunged.
#[test]
fn an_opened_library_is_closed_when_the_program_faults() {
    assert_faults_cleanly("library-fault.clo", "error E11211: index out of range");
}

// The 2.0+ optional surface: a tag list, a date stamp, and a semaphore are
// live at a fault and must all be freed. Only hosted-amiga carries it.
#[test]
fn utility_objects_are_freed_when_the_program_faults() {
    assert_faults_cleanly_on(
        "utility-fault.clo",
        "error E11211: index out of range",
        &[("hosted-amiga", "nix20")],
    );
}

// A panic skips every drop, but a live lock must still be released.
#[test]
fn a_lock_is_released_when_the_program_panics() {
    assert_faults_cleanly("panic-after-acquire.clo", "panic: stop");
}

// A failed assertion skips every drop, but a live lock must still be released.
#[test]
fn a_lock_is_released_when_an_assertion_fails() {
    assert_faults_cleanly("assert-after-acquire.clo", "error E11213: assertion failed");
}

// A clean exit prints nothing beyond the program's own output: the committed
// transcripts still match byte for byte on both profiles.
#[test]
fn clean_runs_print_exactly_their_transcripts() {
    if !sdk_present() {
        return;
    }
    for program in ["exec-self-test", "hamurabi"] {
        let source = repo_root().join(format!("examples/amiga/{program}.clo"));
        let expected = std::fs::read_to_string(
            repo_root().join(format!("examples/amiga/{program}.expected.txt")),
        )
        .expect("the expected transcript is readable");
        for (profile, _) in PROFILES {
            let scratch = ScratchDir::new(&format!("{program}-{profile}"));
            let output = Command::new(env!("CARGO_BIN_EXE_clorraine"))
                .args(["run", "--profile", profile])
                .arg(&source)
                .current_dir(scratch.path())
                .output()
                .expect("run clorraine");
            assert_eq!(
                output.status.code(),
                Some(0),
                "{program} on {profile}: expected exit 0; stderr:\n{}",
                text(&output.stderr)
            );
            assert_eq!(
                String::from_utf8_lossy(&output.stdout),
                expected,
                "{program} on {profile}: the clean run must print exactly its transcript"
            );
        }
    }
}

/// True when the unstripped program at `binary` defines the C symbol `name`
/// (m68k symbols carry one leading underscore).
fn defines_symbol(binary: &Path, name: &str) -> bool {
    let nm = Command::new("m68k-amigaos-nm")
        .arg(binary)
        .output()
        .expect("run m68k-amigaos-nm");
    assert!(
        nm.status.success(),
        "m68k-amigaos-nm failed on {}; stderr:\n{}",
        binary.display(),
        text(&nm.stderr)
    );
    let wanted = format!("_{name}");
    text(&nm.stdout)
        .lines()
        .filter_map(|line| line.split_whitespace().last())
        .any(|symbol| symbol == wanted)
}

// The exit machinery costs nothing in a program that never acquires an OS
// resource: a release build of hello links neither the exit hook nor
// `atexit`, while a program that takes a lock links both.
#[test]
fn only_programs_that_acquire_resources_link_the_exit_hook() {
    if !sdk_present() || !tool_available("m68k-amigaos-nm") {
        return;
    }
    for (profile, crt) in PROFILES {
        let scratch = ScratchDir::new(&format!("symbols-{profile}"));
        let hello = build(
            &scratch,
            "hello",
            profile,
            crt,
            &repo_root().join("examples/hello/hello.clo"),
            Build::ReleaseUnstripped,
        );
        let lock = build(
            &scratch,
            "lock-fault",
            profile,
            crt,
            &probe("lock-fault.clo"),
            Build::ReleaseUnstripped,
        );
        for symbol in ["cl_exit_hook", "atexit"] {
            assert!(
                !defines_symbol(&hello, symbol),
                "{profile}: hello acquires nothing but links {symbol}"
            );
            assert!(
                defines_symbol(&lock, symbol),
                "{profile}: a program that takes a lock must link {symbol}"
            );
        }
    }
}
