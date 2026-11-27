//! Specification tests for hosted-amiga stdlib entries that must not crash a
//! machine when they run before the program has opened anything.
//!
//! `showAlert` and `savePublicScreen` can be a program's first Intuition call.
//! Nothing has opened intuition.library at that point, so the runtime must open
//! it on demand. When the library cannot be opened, the alert is skipped and
//! the capture reports failure; neither may call through a missing library
//! base. `vamos` runs each probe with intuition.library switched off, which is
//! the case where the library cannot be opened, and reports any resource the
//! program left behind when it ends.
//!
//! The runtime helpers these fixes add cost nothing in a program that never
//! calls the entries that need them: a release build without them must not
//! link them.
//!
//! The cases need the m68k cross toolchain and `vamos`; they skip themselves
//! with a note when those tools are absent, like the other hosted-amiga suites.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

/// The `vamos` option that makes every attempt to open intuition.library fail.
const INTUITION_OFF: &str = "intuition.library=mode:off";

/// Temporary directory that removes itself when the test finishes.
struct ScratchDir {
    path: PathBuf,
}

impl ScratchDir {
    fn new(tag: &str) -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "clorraine-crash-safety-{}-{tag}-{sequence}",
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
        .join("examples/robustness/crash-safety")
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
    let flags: &[&str] = match how {
        Build::Debug => &["-O0", "-g", "-Wall", "-Wextra", "-Werror"],
        Build::ReleaseUnstripped => &["-O2", "-ffunction-sections", "-fdata-sections", "-Wl,--gc-sections"],
    };
    let gcc = Command::new("m68k-amigaos-gcc")
        .args(["-std=c99", &format!("-mcrt={crt}"), "-m68000", "-fwrapv"])
        .args(flags)
        .arg("-o")
        .arg(&binary)
        .arg(&c_path)
        .args(["-lamiga", "-lm"])
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

/// Runs `binary` under `vamos` with intuition.library switched off, in its own
/// directory, bounded so a hang fails the case instead of the whole suite.
fn run_without_intuition(binary: &Path) -> Run {
    let dir = binary.parent().expect("the program has a directory");
    let output: Output = Command::new("timeout")
        .arg("120")
        .arg("vamos")
        .args(["-O", INTUITION_OFF])
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

/// Builds `probe_name` for each profile, runs it with intuition.library
/// switched off, and asserts its exact output, a clean exit, and that nothing
/// was left behind.
fn assert_runs_without_intuition(probe_name: &str, expected: &str, profiles: &[(&str, &str)]) {
    if !sdk_present() {
        return;
    }
    for &(profile, crt) in profiles {
        let tag = format!("{}-{profile}", probe_name.trim_end_matches(".clo"));
        let scratch = ScratchDir::new(&tag);
        let binary = build(&scratch, &tag, profile, crt, &probe(probe_name), Build::Debug);
        let run = run_without_intuition(&binary);
        assert_eq!(
            run.stdout, expected,
            "{tag}: the program must print exactly its lines; output:\n{}",
            run.all
        );
        assert_eq!(
            run.code,
            Some(0),
            "{tag}: the program must return normally; output:\n{}",
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

// An alert shown before any screen exists, on a system where intuition.library
// cannot be opened, is skipped: the program carries on and exits cleanly. Both
// hosted profiles carry `showAlert`.
#[test]
fn an_alert_before_any_screen_is_skipped_when_intuition_cannot_open() {
    assert_runs_without_intuition(
        "alert-first.clo",
        "calling\nback\n",
        &[("hosted-amiga", "nix20"), ("hosted-amiga-13", "nix13")],
    );
}

// A public-screen capture as the first Intuition call, on a system where
// intuition.library cannot be opened, reports failure and the program carries
// on. Only hosted-amiga carries `savePublicScreen`.
#[test]
fn a_public_screen_capture_before_any_screen_fails_when_intuition_cannot_open() {
    assert_runs_without_intuition(
        "pubscreen-first.clo",
        "calling\nsaved false\n",
        &[("hosted-amiga", "nix20")],
    );
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

/// Builds `source` release-unstripped for `profile` and reports whether it
/// defines `symbol`.
fn links(source: &Path, tag: &str, profile: &str, crt: &str, symbol: &str) -> bool {
    let scratch = ScratchDir::new(&format!("symbols-{tag}-{profile}"));
    let binary = build(&scratch, tag, profile, crt, source, Build::ReleaseUnstripped);
    defines_symbol(&binary, symbol)
}

// The registry walk that freeing a region or closing a guide uses is linked
// only into programs that free regions or close guides: hello and a program
// that only shows an alert do not carry it, on either hosted profile, and a
// program that frees a region and one that closes a guide do.
#[test]
fn only_programs_that_free_regions_or_close_guides_link_the_registry_walk() {
    if !sdk_present() || !tool_available("m68k-amigaos-nm") {
        return;
    }
    let hello = repo_root().join("examples/hello/hello.clo");
    for (profile, crt) in [("hosted-amiga", "nix20"), ("hosted-amiga-13", "nix13")] {
        assert!(
            !links(&hello, "hello", profile, crt, "cl_res_each"),
            "{profile}: hello links the registry walk"
        );
        assert!(
            !links(&probe("alert-first.clo"), "alert-first", profile, crt, "cl_res_each"),
            "{profile}: a program that only shows an alert links the registry walk"
        );
    }
    assert!(
        links(&probe("region-free.clo"), "region-free", "hosted-amiga", "nix20", "cl_res_each"),
        "hosted-amiga: a program that frees a region must link the registry walk"
    );
    assert!(
        links(&probe("guide-late.clo"), "guide-late", "hosted-amiga", "nix20", "cl_res_each"),
        "hosted-amiga: a program that closes a guide must link the registry walk"
    );
}
