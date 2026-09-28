//! Implementation tests for borrow-element aliasing and binding identity.
//!
//! These cases cover what the specification suite cannot see: the shape of the
//! emitted C for a `for (borrow e in xs)` loop (an address temporary and a
//! dereferenced write, with no copy-in or write-back step), a replaced owned
//! field read back inside the same iteration, a slice iterable, an element
//! binding whose name collides with a local or spells a C keyword, a by-value
//! lambda capture, and a build-only compile for the freestanding profile.
//!
//! Every run goes through a helper with a time limit, so a loop that hangs
//! fails the case instead of freezing the test suite.

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

/// How long one program may run before the case fails as a hang.
const RUN_TIMEOUT: Duration = Duration::from_secs(30);

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
            "clc-ownimpl-{}-{tag}-{sequence}",
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

/// Runs the built compiler with the given arguments.
fn run_clc(args: &[String]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_clc"))
        .args(args)
        .output()
        .expect("failed to spawn the compiler binary")
}

/// Emits one host-profile program and asserts the front end accepted it.
///
/// Returns the emitted translation unit. A rejection is a failure of the case,
/// not a result to assert, because these cases are about what the emitter
/// produces for accepted programs.
fn emit_host(tag: &str, scratch: &ScratchDir, source: &str) -> String {
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, source)
        .unwrap_or_else(|err| panic!("{tag}: cannot write the program: {err}"));
    let c_path = scratch.path().join("out.c");
    let output = run_clc(&[
        "--profile".to_owned(),
        "host".to_owned(),
        "-o".to_owned(),
        c_path.display().to_string(),
        source_path.display().to_string(),
    ]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{tag}: the front end rejected the program; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let emitted = std::fs::read_to_string(&c_path).unwrap_or_default();
    assert!(
        !emitted.trim().is_empty(),
        "{tag}: the emitted C is empty, so the emitter is not implemented yet"
    );
    emitted
}

/// Compiles the emitted C with the host compiler under the strict flag set.
///
/// Panics with the compiler's stderr when the build fails. Returns the built
/// program path.
fn build_host_c(tag: &str, scratch: &ScratchDir, emitted: &str) -> PathBuf {
    let c_path = scratch.path().join("build.c");
    std::fs::write(&c_path, emitted).expect("write the emitted C");
    let program = scratch.path().join("prog");
    let built = Command::new("cc")
        .args([
            "-std=c99",
            "-O0",
            "-g",
            "-Wall",
            "-Wextra",
            "-Werror",
            "-o",
            program.to_str().expect("program path"),
            c_path.to_str().expect("emitted C path"),
        ])
        .output()
        .expect("failed to spawn the host C compiler");
    assert!(
        built.status.success(),
        "{tag}: the host C compiler rejected the emitted C; stderr:\n{}",
        String::from_utf8_lossy(&built.stderr)
    );
    program
}

/// Runs a built program with a time limit and returns its output.
///
/// A program that is still running when the limit passes is killed and the
/// case fails, so a regression that loops forever surfaces as one failing case
/// instead of a frozen suite.
fn run_with_timeout(tag: &str, program: &Path) -> Output {
    let mut child = Command::new(program)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|err| panic!("{tag}: cannot run the program: {err}"));
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
                panic!("{tag}: the program did not finish within {RUN_TIMEOUT:?}");
            }
            Err(err) => panic!("{tag}: cannot wait for the program: {err}"),
        }
    }
    child
        .wait_with_output()
        .unwrap_or_else(|err| panic!("{tag}: cannot collect the program output: {err}"))
}

/// Emits, builds, and runs one host program, asserting the exit code.
///
/// Returns the emitted translation unit so a case can also inspect the C.
fn run_host_case(tag: &str, source: &str, expected_exit: i32) -> String {
    let scratch = ScratchDir::new(tag);
    let emitted = emit_host(tag, &scratch, source);
    let program = build_host_c(tag, &scratch, &emitted);
    let output = run_with_timeout(tag, &program);
    assert_eq!(
        output.status.code(),
        Some(expected_exit),
        "{tag}: unexpected exit; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    emitted
}

/// True when `tool` can be spawned, so the m68k leg skips on a bare host.
fn tool_available(tool: &str) -> bool {
    Command::new(tool).arg("--version").output().is_ok()
}

/// Compiles the emitted C with the host's no-libc stance.
///
/// The freestanding profile has no host runtime to link against, so the
/// translation unit carries its own entry point and helpers.
fn build_host_no_libc(tag: &str, scratch: &ScratchDir, emitted: &str) -> PathBuf {
    let c_path = scratch.path().join("freestanding.c");
    std::fs::write(&c_path, emitted).expect("write the emitted C");
    let program = scratch.path().join("freestanding");
    let built = Command::new("cc")
        .args([
            "-std=c99",
            "-ffreestanding",
            "-fno-builtin",
            "-nostdlib",
            "-O0",
            "-g",
            "-Wall",
            "-Wextra",
            "-Werror",
            "-o",
            program.to_str().expect("program path"),
            c_path.to_str().expect("emitted C path"),
        ])
        .output()
        .expect("failed to spawn the host C compiler");
    assert!(
        built.status.success(),
        "{tag}: the host no-libc build rejected the emitted C; stderr:\n{}",
        String::from_utf8_lossy(&built.stderr)
    );
    program
}

/// Compiles the emitted C with the m68k cross toolchain.
///
/// Returns `None` and prints a note when the toolchain is absent, so a bare
/// host reports a skip rather than a false failure.
fn build_m68k(tag: &str, scratch: &ScratchDir, emitted: &str) -> Option<PathBuf> {
    if !tool_available("m68k-amigaos-gcc") {
        println!("{tag}: skipped (m68k-amigaos-gcc is not installed)");
        return None;
    }
    let c_path = scratch.path().join("m68k.c");
    std::fs::write(&c_path, emitted).expect("write the emitted C");
    let program = scratch.path().join("m68k");
    let built = Command::new("m68k-amigaos-gcc")
        .args([
            "-std=c99",
            "-ffreestanding",
            "-fno-builtin",
            "-nostdlib",
            "-m68000",
            "-fwrapv",
            "-O0",
            "-g",
            "-Wall",
            "-Wextra",
            "-Werror",
            "-o",
            program.to_str().expect("program path"),
            c_path.to_str().expect("emitted C path"),
            "-lg",
        ])
        .output()
        .expect("failed to spawn m68k-amigaos-gcc");
    assert!(
        built.status.success(),
        "{tag}: the m68k link rejected the emitted C; stderr:\n{}",
        String::from_utf8_lossy(&built.stderr)
    );
    Some(program)
}

/// A borrow element lowers to an address temporary, not a copied local.
///
/// The emitted C must refresh `__cl_tmpN` with the element's address each
/// iteration and write through `(*__cl_tmpN)`. A copy-in local (`u32 p`) or a
/// store-back through a computed address (`*(&(...))`) would mean the element
/// view is not an alias. The program writes four into both elements, so exit
/// eight proves the writes reached the array.
#[test]
fn the_borrow_element_aliases_through_an_address_temporary() {
    let source = r#"module codegen.ownimpl.structure;

func fill(borrow ps: u32[2]): void {
    for (borrow p in ps) {
        p = 4;
    }
}

func main(): i32 {
    let xs: u32[2] = [1, 2];
    fill(borrow xs);
    return i32(xs[0] + xs[1]);
}
"#;
    let emitted = run_host_case("structure", source, 8);
    assert!(
        emitted.contains("(*__cl_tmp"),
        "structure: the element must render as a dereferenced temporary"
    );
    assert!(
        emitted.contains("= &(ps["),
        "structure: the temporary must be refreshed with the element address"
    );
    assert!(
        !emitted.contains("*(&("),
        "structure: no computed-address write-back may remain"
    );
    assert!(
        !emitted.contains("u32 p = 0;"),
        "structure: the element binding must not declare a C local"
    );
}

/// Replacing an owned field and reading it back works within one iteration.
///
/// The replacement drops the old `String` and stores the new one in the
/// element, so reading the field back sees the new value and no old value is
/// released twice. Each iteration reads four bytes; with the two scalar `n`
/// fields the program exits eleven.
#[test]
fn a_replaced_field_reads_back_within_the_same_iteration() {
    let source = r#"module codegen.ownimpl.readback;

import { String, stringOf } from std.text;

struct P { name: String; n: i32; }

func vlen(s: str): i32 { return i32(len(s)); }

func fill(borrow ps: P[2]): i32 {
    let total: i32 = 0;
    for (borrow p in ps) {
        p.name = stringOf("qqqq");
        total = total + vlen(p.name);
    }
    return total;
}

func main(): i32 {
    let ps: P[2] = [P { name: stringOf("a"), n: 1 }, P { name: stringOf("b"), n: 2 }];
    let total: i32 = fill(borrow ps);
    return total + ps[0].n + ps[1].n;
}
"#;
    run_host_case("readback", source, 11);
}

/// A slice iterable writes through to the array it views.
///
/// The loop walks the middle two elements of a four-element array and sets
/// each to nine, so only the sliced range changes and the program exits
/// `1 + 9 + 9 + 4 = 23`.
#[test]
fn a_slice_iterable_writes_through_to_the_backing_array() {
    let source = r#"module codegen.ownimpl.slice;

import { slice } from std.mem;

func fill(borrow xs: u32[]): void {
    for (borrow e in xs) {
        e = 9;
    }
}

func main(): i32 {
    let xs: u32[4] = [1, 2, 3, 4];
    let part: u32[] = slice(xs, 1, 3);
    fill(borrow part);
    return i32(xs[0] + xs[1] + xs[2] + xs[3]);
}
"#;
    run_host_case("slice", source, 23);
}

/// A borrow element may shadow a local with the same name.
///
/// The loop binding `p` shadows the local `p`; the alias declares no C local,
/// so the outer `p` keeps its value (100) and the loop accumulates the
/// incremented elements (2 + 3). The program exits `100 + 5 + 2 + 3 = 110`.
#[test]
fn a_borrow_element_may_shadow_a_same_named_local() {
    let source = r#"module codegen.ownimpl.collide;

func fill(borrow ps: u32[2]): i32 {
    let p: i32 = 100;
    let total: i32 = 0;
    for (borrow p in ps) {
        p = p + 1;
        total = total + i32(p);
    }
    return total + p;
}

func main(): i32 {
    let xs: u32[2] = [1, 2];
    let total: i32 = fill(borrow xs);
    return total + i32(xs[0]) + i32(xs[1]);
}
"#;
    run_host_case("collide", source, 110);
}

/// A borrow element may be named after a C keyword.
///
/// The source name `int` is a valid Clorraine identifier but a C keyword, so
/// the outer local is emitted as `cl_int` and the alias renders through its
/// temporary. The loop doubles the elements (3, 4 become 6, 8), the outer
/// local keeps seven, and the program exits `7 + 14 + 6 + 8 = 35`.
#[test]
fn a_borrow_element_may_spell_a_c_keyword() {
    let source = r#"module codegen.ownimpl.keyword;

func fill(borrow xs: u32[2]): i32 {
    let int: u32 = 7;
    let total: i32 = 0;
    for (borrow int in xs) {
        int = int * 2;
        total = total + i32(int);
    }
    return total + i32(int);
}

func main(): i32 {
    let xs: u32[2] = [3, 4];
    let total: i32 = fill(borrow xs);
    return total + i32(xs[0]) + i32(xs[1]);
}
"#;
    run_host_case("keyword", source, 35);
}

/// A by-value lambda capture stays rooted in the environment field.
///
/// The lifted body reads its captured `base` through the environment, so it
/// must not declare a C local for it. The program builds with the strict
/// warning set, which a duplicate or unused local would break, and exits five.
#[test]
fn a_by_value_capture_declares_no_spurious_local() {
    let source = r#"module codegen.ownimpl.lambda;

func apply<T, F: func(T): T>(x: T, f: F): T {
    return f(x);
}

func main(): i32 {
    let base: i32 = 2;
    return apply(i32(3), [base] (x: i32) => x + base);
}
"#;
    run_host_case("lambda", source, 5);
}

/// A borrow-element program builds for the freestanding profile.
///
/// The freestanding artifact cannot run on the host, so the case proves the
/// emitted C builds: always through the host no-libc leg, and through the m68k
/// cross toolchain when it is installed. This keeps the element alias covered
/// on the profile that has no test corpus for the construct.
#[test]
fn a_borrow_element_program_builds_for_freestanding() {
    let source = r#"module codegen.ownimpl.freestanding;

func doubleAll(borrow xs: u32[4]): void {
    for (borrow e in xs) {
        e = e * 2;
    }
}

func main(): i32 {
    let xs: u32[4] = [1, 2, 3, 4];
    doubleAll(borrow xs);
    return i32(xs[0] + xs[1] + xs[2] + xs[3]);
}
"#;
    let scratch = ScratchDir::new("freestanding");
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, source).expect("write the source");
    let c_path = scratch.path().join("out.c");
    let output = run_clc(&[
        "--profile".to_owned(),
        "freestanding".to_owned(),
        "-o".to_owned(),
        c_path.display().to_string(),
        source_path.display().to_string(),
    ]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "freestanding: the front end rejected the program; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let emitted = std::fs::read_to_string(&c_path).unwrap_or_default();
    assert!(
        !emitted.trim().is_empty(),
        "freestanding: the emitted C is empty"
    );
    build_host_no_libc("freestanding", &scratch, &emitted);
    build_m68k("freestanding", &scratch, &emitted);
}
