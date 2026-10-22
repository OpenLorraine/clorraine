//! Runnable specification tests for evaluation and emission integrity.
//!
//! Each case compiles a small Clorraine program with the built `clc` binary,
//! builds the emitted C with the host C compiler under the fixed warning
//! flags, runs the binary, and asserts the process result. Programs that must
//! be rejected assert the diagnostics instead, and no translation unit may be
//! written for them.
//!
//! The module-initializer group pins the six accepted positions for a
//! fixed-array-returning call (whole value, element and member projection,
//! user-function and intrinsic arguments, conditional arm), the
//! single-evaluation rule for an impure guard operand, and the rejection of a
//! slice-typed binding initialized from an array literal.
//!
//! The memory helper runs a case as an ordinary strict host build and adds
//! leak and use-after-free detection when `RDC_ASAN` is set.

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
            "clc-eei-{}-{tag}-{sequence}",
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

/// Compiles `source` for the host profile and returns the scratch directory,
/// the compiler output, and the emitted-C path.
fn compile_only(tag: &str, source: &str) -> (ScratchDir, Output, PathBuf) {
    let scratch = ScratchDir::new(tag);
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
    (scratch, output, c_path)
}

/// Compiles an existing source file for the host profile.
fn compile_path(tag: &str, source_path: &Path) -> (ScratchDir, Output, PathBuf) {
    let scratch = ScratchDir::new(tag);
    let c_path = scratch.path().join("out.c");
    let output = run_clc(&[
        "--profile".to_owned(),
        "host".to_owned(),
        "-o".to_owned(),
        c_path.display().to_string(),
        source_path.display().to_string(),
    ]);
    (scratch, output, c_path)
}

/// Asserts one compiler run rejected the program with exactly one `code`
/// diagnostic, that the message contains every fragment and none of the
/// absent markers, and that no translation unit is written.
fn assert_rejected_output(
    tag: &str,
    output: &Output,
    c_path: &Path,
    code: &str,
    fragments: &[&str],
    absent: &[&str],
) {
    assert_ne!(
        output.status.code(),
        Some(0),
        "{tag}: the front end accepted a program that must be rejected"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    let marker = format!("error {code}:");
    assert_eq!(
        stderr.matches(&marker).count(),
        1,
        "{tag}: expected exactly one {code}; stderr:\n{stderr}"
    );
    for fragment in fragments {
        assert!(
            stderr.contains(fragment),
            "{tag}: expected the message to contain {fragment:?}; stderr:\n{stderr}"
        );
    }
    for absent_marker in absent {
        assert!(
            !stderr.contains(absent_marker),
            "{tag}: the report {absent_marker:?} must be absent; stderr:\n{stderr}"
        );
    }
    assert!(
        !c_path.exists(),
        "{tag}: a rejected program must not produce a translation unit"
    );
}

/// Compiles `source` and asserts the rejection contract on the output.
fn assert_rejected_once(tag: &str, source: &str, code: &str, fragments: &[&str], absent: &[&str]) {
    let (_scratch, output, c_path) = compile_only(tag, source);
    assert_rejected_output(tag, &output, &c_path, code, fragments, absent);
}

/// Compiles an existing file and asserts the rejection contract on the
/// output.
fn assert_rejected_once_at(
    tag: &str,
    source_path: &Path,
    code: &str,
    fragments: &[&str],
    absent: &[&str],
) {
    let (_scratch, output, c_path) = compile_path(tag, source_path);
    assert_rejected_output(tag, &output, &c_path, code, fragments, absent);
}

/// Emits one host-profile program and asserts the front end accepted it.
fn emit_host(tag: &str, source: &str) -> String {
    let (_scratch, output, c_path) = compile_only(tag, source);
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

/// Runs a built program with a time limit and the given environment values.
fn run_with_timeout(tag: &str, program: &Path, env: &[(&str, &str)]) -> Output {
    let mut command = Command::new(program);
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    for (key, value) in env {
        command.env(key, value);
    }
    let mut child = command
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

/// Emits, builds, and runs one host program, returning its output.
fn run_host(tag: &str, source: &str) -> Output {
    let scratch = ScratchDir::new(tag);
    let emitted = emit_host(tag, source);
    let program = build_host_c(tag, &scratch, &emitted);
    run_with_timeout(tag, &program, &[])
}

/// Emits a program, builds it with AddressSanitizer, and runs it with leak
/// detection on, returning its output.
fn run_asan(tag: &str, source: &str) -> Output {
    let scratch = ScratchDir::new(tag);
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, source)
        .unwrap_or_else(|err| panic!("{tag}: cannot write the program: {err}"));
    let c_path = scratch.path().join("out.c");
    let emitted = run_clc(&[
        "--profile".to_owned(),
        "host".to_owned(),
        "-o".to_owned(),
        c_path.display().to_string(),
        source_path.display().to_string(),
    ]);
    assert_eq!(
        emitted.status.code(),
        Some(0),
        "{tag}: the front end rejected the program; stderr:\n{}",
        String::from_utf8_lossy(&emitted.stderr)
    );
    let program = scratch.path().join("prog");
    let built = Command::new("cc")
        .args([
            "-std=c99",
            "-O0",
            "-g",
            "-fsanitize=address",
            "-no-pie",
            "-o",
            program.to_str().expect("program path"),
            c_path.to_str().expect("emitted C path"),
        ])
        .output()
        .expect("failed to spawn the host C compiler");
    assert!(
        built.status.success(),
        "{tag}: the sanitizer build failed; stderr:\n{}",
        String::from_utf8_lossy(&built.stderr)
    );
    run_with_timeout(
        tag,
        &program,
        &[("ASAN_OPTIONS", "detect_leaks=1:symbolize=0")],
    )
}

/// Runs a memory-safety case: under AddressSanitizer when `RDC_ASAN` is set,
/// otherwise as an ordinary strict host build.
fn run_memory(tag: &str, source: &str) -> Output {
    if std::env::var_os("RDC_ASAN").is_some() {
        run_asan(tag, source)
    } else {
        run_host(tag, source)
    }
}

/// Asserts the program exited with exactly `expected`, reporting its stderr.
fn assert_exit(tag: &str, output: &Output, expected: i32) {
    assert_eq!(
        output.status.code(),
        Some(expected),
        "{tag}: expected exit code {expected}; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Asserts the run produced no AddressSanitizer report.
fn assert_no_asan(tag: &str, output: &Output) {
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("AddressSanitizer"),
        "{tag}: the run reported a memory error; stderr:\n{stderr}"
    );
}

/// A fixed-array-returning call as a whole module-binding value builds and
/// runs.
///
/// The initializer stores through the callee's hidden destination, so the
/// emitted C passes the array's destination instead of assigning a call
/// result.
#[test]
fn should_build_and_run_the_module_init_whole_value() {
    let source = r#"module spec.eei.initwhole;

func mk(): i32[2] {
    return [4, 2];
}

let a: i32[2] = mk();

func main(): i32 {
    return a[1];
}
"#;
    let output = run_memory("init-whole", source);
    assert_exit("init-whole", &output, 2);
}

/// The element-projection init position builds and runs.
#[test]
fn should_build_and_run_the_module_init_element_projection() {
    let source = r#"module spec.eei.initelement;

func mk(): i32[2] {
    return [7, 2];
}

let b: i32 = mk()[1];

func main(): i32 {
    return b;
}
"#;
    let output = run_memory("init-element", source);
    assert_exit("init-element", &output, 2);
}

/// The member-projection init position builds and runs.
#[test]
fn should_build_and_run_the_module_init_member_projection() {
    let source = r#"module spec.eei.initmember;

struct Box {
    arr: i32[2];
}

func mkBox(): Box {
    return Box { arr: [5, 2] };
}

let c: i32[2] = mkBox().arr;

func main(): i32 {
    return c[1];
}
"#;
    let output = run_memory("init-member", source);
    assert_exit("init-member", &output, 2);
}

/// The user-function and intrinsic argument positions build and run.
#[test]
fn should_build_and_run_the_module_init_argument_positions() {
    let source = r#"module spec.eei.initargs;

import { slice } from std.mem;

func mk(): i32[2] {
    return [3, 9];
}

func eat(arr: i32[2]): i32 {
    return arr[0] + arr[1];
}

let d: i32 = eat(mk());
let e: i32 = i32(len(slice(mk(), 0, 2)));

func main(): i32 {
    return d + e;
}
"#;
    let output = run_memory("init-args", source);
    assert_exit("init-args", &output, 14);
}

/// The conditional-arm init position builds and runs.
#[test]
fn should_build_and_run_the_module_init_conditional_arm() {
    let source = r#"module spec.eei.initcond;

func mk(): i32[2] {
    return [6, 1];
}

func pick(): bool {
    return true;
}

let y: i32[2] = [8, 8];
let f: i32[2] = mk();
let g: i32[2] = pick() ? f : y;

func main(): i32 {
    return g[0];
}
"#;
    let output = run_memory("init-cond", source);
    assert_exit("init-cond", &output, 6);
}

/// An impure module-initializer guard operand is evaluated once.
///
/// The index function counts its calls and goes out of range on a second
/// evaluation; the bounds guard and the read must share one evaluation, so
/// the count is one and the read is in bounds (a second evaluation reads out
/// of bounds, which the sanitizer probe observes at the baseline).
#[test]
fn should_evaluate_the_module_init_guard_operand_once() {
    let source = r#"module spec.eei.guardonce;

let calls: i32 = 0;

func idx(): i32 {
    calls = calls + 1;
    if (calls == 1) {
        return 0;
    }
    return 5;
}

let s: u8[2] = [1, 2];
let v: u8 = s[idx()];

func main(): i32 {
    return calls;
}
"#;
    let output = run_memory("guard-once", source);
    assert_exit("guard-once", &output, 1);
}

/// A slice-typed binding from an array literal is rejected; the slice form
/// stays accepted.
#[test]
fn should_reject_the_literal_slice_binding_and_accept_the_slice_form() {
    let rejected = r#"module spec.eei.literalslice;

let x: i32[] = [1, 2];

func main(): i32 {
    return i32(len(x));
}
"#;
    assert_rejected_once("literal-slice", rejected, "E10200", &[], &[]);

    let rejected_in_body = r#"module spec.eei.literalslicebody;

func main(): i32 {
    let x: i32[] = [1, 2];
    return i32(len(x));
}
"#;
    assert_rejected_once("literal-slice-body", rejected_in_body, "E10200", &[], &[]);

    let rejected_fill = r#"module spec.eei.fillslicetarget;

let x: i32[] = [1, 2; 0];

func main(): i32 {
    return i32(len(x));
}
"#;
    assert_rejected_once(
        "literal-slice-fill",
        rejected_fill,
        "E10215",
        &[],
        &["E10200"],
    );

    let accepted = r#"module spec.eei.slicedform;

import { slice } from std.mem;

func main(): i32 {
    let arr: i32[2] = [1, 2];
    let y: i32[] = slice(arr, 0, 2);
    return i32(len(y));
}
"#;
    let output = run_host("slice-form", accepted);
    assert_exit("slice-form", &output, 2);
}

/// Emits one freestanding-profile program and asserts the front end accepted
/// it.
fn emit_freestanding(tag: &str, source: &str) -> String {
    let scratch = ScratchDir::new(tag);
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, source)
        .unwrap_or_else(|err| panic!("{tag}: cannot write the program: {err}"));
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
        "{tag}: the front end rejected the program; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    std::fs::read_to_string(&c_path).unwrap_or_default()
}

/// Emits a freestanding program, builds it on the host with the no-libc
/// stance, and runs it.
fn run_freestanding_host(tag: &str, source: &str) -> Output {
    let scratch = ScratchDir::new(tag);
    let emitted = emit_freestanding(tag, source);
    let c_path = scratch.path().join("build.c");
    std::fs::write(&c_path, &emitted).expect("write the emitted C");
    let program = scratch.path().join("prog");
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
        "{tag}: the host C compiler rejected the emitted C; stderr:\n{}",
        String::from_utf8_lossy(&built.stderr)
    );
    run_with_timeout(tag, &program, &[])
}

/// Emits a host program, links it with a C stub, and runs it.
fn run_host_with_stub(tag: &str, source: &str, stub: &str) -> Output {
    let scratch = ScratchDir::new(tag);
    let emitted = emit_host(tag, source);
    let c_path = scratch.path().join("build.c");
    std::fs::write(&c_path, &emitted).expect("write the emitted C");
    let stub_path = scratch.path().join("stub.c");
    std::fs::write(&stub_path, stub).expect("write the stub");
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
            stub_path.to_str().expect("stub path"),
        ])
        .output()
        .expect("failed to spawn the host C compiler");
    assert!(
        built.status.success(),
        "{tag}: the host C compiler rejected the linked unit; stderr:\n{}",
        String::from_utf8_lossy(&built.stderr)
    );
    run_with_timeout(tag, &program, &[])
}

/// A generic `.len` member of a fixed-array call result builds and runs: the
/// extent is structural, so the length does not need the value.
#[test]
fn should_build_a_generic_structural_len_member() {
    let source = r#"module spec.eei.lengeneric;

func mkArr<T>(x: T): T[2] {
    return [x, x];
}

func g<T>(x: T): i32 {
    return i32(mkArr(x).len);
}

func main(): i32 {
    return g<i32>(5) * 10 + 2;
}
"#;
    let output = run_host("len-generic", source);
    assert_exit("len-generic", &output, 22);
}

/// The `.len` member of a fixed array is the structural extent for a named
/// local and for a call result alike, in all three spellings.
#[test]
fn should_build_the_structural_len_member_spellings() {
    let source = r#"module spec.eei.lenmember;

func mkArr(): i32[2] {
    return [3, 9];
}

func main(): i32 {
    let xs: u8[3] = [1, 2, 3];
    let n: usize = xs.len;
    let m: i32 = i32(len(xs));
    let k: i32 = i32(xs.len());
    return i32(n) * 10 + m + k + i32(mkArr().len);
}
"#;
    let output = run_host("len-member", source);
    assert_exit("len-member", &output, 38);
}

/// A structural `.len` is not a write target: a store, a compound store, an
/// update, a value-position update, and an address-of each report `E10221`
/// instead of emitting nothing; a read-only parameter keeps its own report.
#[test]
fn should_reject_a_structural_len_write_target() {
    for (tag, body) in [
        ("len-store", "    xs.len = 4;\n    return i32(xs.len);"),
        ("len-compound", "    xs.len += 1;\n    return i32(xs.len);"),
        ("len-update", "    xs.len++;\n    return i32(xs.len);"),
        ("len-value-update", "    let y: usize = xs.len++;\n    return i32(y);"),
        (
            "len-address",
            "    unsafe {\n        let p = &xs.len;\n        if (p == null) {\n            return 1;\n        }\n    }\n    return 0;",
        ),
    ] {
        let source = format!(
            "module spec.eei.lentarget;\n\nfunc main(): i32 {{\n    let xs: i32[3] = [1, 2, 3];\n{body}\n}}\n"
        );
        assert_rejected_once(tag, &source, "E10221", &[], &[]);
    }

    let read_only = r#"module spec.eei.lenreadonly;

func f(xs: i32[3]): i32 {
    xs.len = 4;
    return i32(xs.len);
}

func main(): i32 {
    let m: i32[3] = [1, 2, 3];
    return f(m);
}
"#;
    assert_rejected_once("len-readonly", read_only, "E10024", &[], &["E10221"]);
}

/// A struct-returning function that falls off its end returns the zero value
/// without a compound literal, and the fields read as zero.
#[test]
fn should_build_and_run_a_struct_empty_return_without_a_compound_literal() {
    let source = r#"module spec.eei.emptyret;

struct P {
    x: i32;
    y: i32;
}

func maybe(flag: bool): P {
    if (flag) {
        return P { x: 1, y: 2 };
    }
}

func main(): i32 {
    let p: P = maybe(false);
    return p.x + p.y + maybe(true).x;
}
"#;
    let emitted = emit_host("empty-return", source);
    assert!(
        !emitted.contains("(ClP){0}"),
        "empty-return: the C99-subset contract forbids the struct compound literal"
    );
    let output = run_host("empty-return", source);
    assert_exit("empty-return", &output, 1);
}

/// A freestanding program that uses `bool` builds on the host no-libc leg and
/// runs.
#[test]
fn should_build_and_run_a_freestanding_bool_program() {
    let source = r#"module spec.eei.fsboot;

func flag(): bool {
    return true;
}

let g: bool = flag();

func main(): i32 {
    if (g) {
        return 1;
    }
    return 0;
}
"#;
    let output = run_freestanding_host("freestanding-bool", source);
    assert_exit("freestanding-bool", &output, 1);
}

/// A slice passed to an uncontracted extern coerces to its pointer, so the
/// linked stub reads the backing bytes.
#[test]
fn should_build_and_run_a_slice_passed_to_an_uncontracted_extern() {
    let source = r#"module spec.eei.viewextern;

import { slice } from std.mem;

ffi lib {
    func RawSecond(data: u8[]): i32;
}

func main(): i32 {
    let xs: u8[4] = [7, 2, 3, 4];
    let s: u8[] = slice(xs, 0, 4);
    let n: i32 = 0;
    unsafe {
        n = RawSecond(s);
    }
    return n;
}
"#;
    let stub = "typedef void *APTR;\ntypedef int LONG;\nLONG RawSecond(APTR data) { return ((unsigned char *)data)[1]; }\n";
    let output = run_host_with_stub("view-extern", source, stub);
    assert_exit("view-extern", &output, 2);

    let coercions = r#"module spec.eei.viewexterncoerce;

import { slice } from std.mem;
import { String, stringOf } from std.text;

ffi lib {
    func CText(s: cstring): i32;
    func RStr(s: str): i32;
    func RSlice(data: u8[]): i32;
}

func main(): i32 {
    let xs: u8[4] = [7, 2, 3, 4];
    let s: u8[] = slice(xs, 0, 4);
    let t: str = "hi";
    let owned: String = stringOf("z");
    let n: i32 = 0;
    unsafe {
        n = CText("x") + RStr(t) + RStr(owned) + RSlice(s);
    }
    return n;
}
"#;
    let stub = "typedef void *APTR;\ntypedef char *STRPTR;\ntypedef int LONG;\nLONG CText(STRPTR s) { return s != 0; }\nLONG RStr(APTR s) { return s != 0; }\nLONG RSlice(APTR s) { return s != 0; }\n";
    let output = run_host_with_stub("view-extern-coerce", coercions, stub);
    assert_exit("view-extern-coerce", &output, 4);
}

/// Source bindings named like the compiler-generated locals de-duplicate
/// instead of colliding: a `__cl_ret` parameter in a function with cleanup, a
/// `__env` local inside a capturing lambda, and a `__cl_tmp0` binding.
#[test]
fn should_de_duplicate_source_bindings_named_like_generated_locals() {
    let source = r#"module spec.eei.gennames;

import { String, stringOf } from std.text;

func demo(__cl_ret: i32): i32 {
    let s: String = stringOf("x");
    if (__cl_ret > 0) {
        return __cl_ret + i32(len(s));
    }
    return 0;
}

func outer(x: i32): i32 {
    let base: i32 = x;
    return ([base] () => {
        let __env: i32 = base + 1;
        return __env + 1;
    })();
}

func main(): i32 {
    return demo(4) + outer(3);
}
"#;
    let output = run_host("generated-names", source);
    assert_exit("generated-names", &output, 10);
}

/// A multi-extent row slice of a struct builds after the row typedef follows
/// the struct definition, and the rows read correctly.
#[test]
fn should_build_and_run_a_multi_extent_row_slice_of_a_struct() {
    let source = r#"module spec.eei.rowslice;

import { slice } from std.mem;

struct P {
    x: i32;
    y: i32;
}

func total(rows: P[][2]): i32 {
    return rows[0][0].x + rows[1][1].y;
}

func main(): i32 {
    let back: P[3][2] = [[P { x: 1, y: 2 }, P { x: 3, y: 4 }], [P { x: 5, y: 6 }, P { x: 7, y: 8 }], [P { x: 9, y: 10 }, P { x: 11, y: 12 }]];
    let rows: P[][2] = slice(back, 0, 3);
    return total(rows);
}
"#;
    let output = run_host("row-slice", source);
    assert_exit("row-slice", &output, 9);
}

/// The single-extent pointer-element slice control still builds and runs.
#[test]
fn should_build_and_run_a_pointer_element_slice_control() {
    let source = r#"module spec.eei.ptrslice;

import { slice } from std.mem;

struct P {
    x: i32;
    y: i32;
}

func main(): i32 {
    let ps: P[3] = [P { x: 1, y: 2 }, P { x: 3, y: 4 }, P { x: 5, y: 6 }];
    let s: P[] = slice(ps, 0, 3);
    return s[2].x + s[0].y;
}
"#;
    let output = run_host("ptr-slice", source);
    assert_exit("ptr-slice", &output, 7);
}

/// A call of a callable-bound parameter's field builds and runs.
#[test]
fn should_build_and_run_a_callable_bound_field_call() {
    let source = r#"module spec.eei.boundfield;

struct Holder<T> {
    call: T;
}

func use<T: func(i32): i32>(h: Holder<T>): i32 {
    return h.call(5);
}

func addOne(x: i32): i32 {
    return x + 1;
}

func main(): i32 {
    return use<func(i32): i32>(Holder<func(i32): i32> { call: addOne });
}
"#;
    let output = run_host("bound-field", source);
    assert_exit("bound-field", &output, 6);
}

/// A callable field named like the builtin `len` is called through the field,
/// not lowered as the length intrinsic.
#[test]
fn should_call_a_func_field_named_len_through_the_field() {
    let source = r#"module spec.eei.funcfieldlen;

struct Holder {
    len: func(i32): i32;
    tag: i32;
}

func addOne(x: i32): i32 {
    return x + 1;
}

func main(): i32 {
    let h: Holder = Holder { len: addOne, tag: 7 };
    return h.len(5) + h.tag;
}
"#;
    let output = run_host("func-field-len", source);
    assert_exit("func-field-len", &output, 13);
}

/// A generic borrow's struct-field projection reads the instance field's real
/// length, not the fallback.
#[test]
fn should_read_the_instance_field_length_through_a_generic_borrow() {
    let source = r#"module spec.eei.fieldlen;

import { String, stringOf } from std.text;

struct Box {
    s: String;
}

func takeBox<T>(borrow b: T, fallback: i32): i32 {
    if (i32(len(b.s)) > 0) {
        return i32(len(b.s));
    }
    return fallback;
}

func main(): i32 {
    let b: Box = Box { s: stringOf("abcdefg") };
    return takeBox<Box>(b, 3);
}
"#;
    let output = run_host("field-len", source);
    assert_exit("field-len", &output, 7);
}

/// A growable array's projected length through a generic borrow reads the
/// element count, not the folded zero.
#[test]
fn should_read_the_growable_length_through_a_generic_borrow() {
    let source = r#"module spec.eei.growlen;

import { Array, arrayNew } from std.mem;

struct Basket {
    items: Array<i32>;
}

func countOf<T>(borrow b: T): i32 {
    return i32(len(b.items));
}

func main(): i32 {
    let b: Basket = Basket { items: arrayNew<i32>() };
    b.items.push(5);
    b.items.push(7);
    return countOf<Basket>(b);
}
"#;
    let output = run_host("growable-len", source);
    assert_exit("growable-len", &output, 2);
}

/// A local inferred from `null` keeps the nullable type and builds under the
/// strict flags.
#[test]
fn should_build_and_run_an_inferred_null_local() {
    let source = r#"module spec.eei.nullinfer;

func main(): i32 {
    let x = null;
    if (x == null) {
        return 1;
    }
    return 0;
}
"#;
    let output = run_host("null-infer", source);
    assert_exit("null-infer", &output, 1);
}

/// The inferred null local pairs with a view: the pairing is the view's
/// emptiness test, so a non-empty view does not match.
#[test]
fn should_build_and_run_a_null_local_paired_with_a_view() {
    let source = r#"module spec.eei.nullpair;

func main(): i32 {
    let x = null;
    let s: str = "ab";
    if (x == s) {
        return 1;
    }
    return 2;
}
"#;
    let output = run_host("null-pair", source);
    assert_exit("null-pair", &output, 2);
}

/// Two `str` views have no defined equality: the pair reports `E10222` and no
/// translation unit is written.
#[test]
fn should_reject_view_pair_equality() {
    let source = r#"module spec.eei.vieweq;

func main(): i32 {
    let a: str = "ab";
    let b: str = "ab";
    if (a == b) {
        return 1;
    }
    return 0;
}
"#;
    assert_rejected_once("view-eq", source, "E10222", &["operator '=='"], &[]);
}

/// The untyped-operand spelling reports on the view side, exactly like the
/// slice case.
#[test]
fn should_reject_an_untyped_operand_compared_with_a_str() {
    let source = r#"module spec.eei.untypedeq;

func main(): i32 {
    let x;
    x = 5;
    let s: str = "ab";
    if (x == s) {
        return 1;
    }
    return 0;
}
"#;
    assert_rejected_once("untyped-str-eq", source, "E10222", &["operator '=='"], &[]);
}

/// A `str` element loop reports the non-iterable rule; fixed-array and slice
/// loops keep building and run.
#[test]
fn should_reject_a_str_element_loop() {
    let source = r#"module spec.eei.strloop;

func sumStr(s: str): i32 {
    let total: i32 = 0;
    for (borrow c in s) {
        total = total + i32(c);
    }
    return total;
}

func main(): i32 {
    return sumStr("ab");
}
"#;
    assert_rejected_once("str-loop", source, "E10043", &["cannot be iterated"], &[]);

    let control = r#"module spec.eei.loopcontrol;

import { slice } from std.mem;

func sumArray(xs: i32[3]): i32 {
    let total: i32 = 0;
    for (borrow v in xs) {
        total = total + v;
    }
    return total;
}

func sumSlice(xs: i32[]): i32 {
    let total: i32 = 0;
    for (v in xs) {
        total = total + v;
    }
    return total;
}

func countEach<T>(xs: T): i32 {
    let n: i32 = 0;
    for (v in xs) {
        n = n + 1;
    }
    return n;
}

func main(): i32 {
    let m: i32[3] = [1, 2, 3];
    let s: i32[] = slice(m, 0, 3);
    return sumArray(m) + sumSlice(s) + countEach<i32[3]>(m) + countEach<i32[]>(s);
}
"#;
    let output = run_host("loop-control", control);
    assert_exit("loop-control", &output, 18);
}

/// A write through a `str` loop element reports the same non-iterable rule,
/// once for the site.
#[test]
fn should_reject_a_write_through_a_str_element_loop() {
    let source = r#"module spec.eei.strloopwrite;

func fill(s: str): void {
    for (borrow c in s) {
        c = 65;
    }
}

func main(): i32 {
    fill("ab");
    return 0;
}
"#;
    assert_rejected_once("str-loop-write", source, "E10043", &[], &[]);
}

/// A member call on a bare type parameter reports an unknown member; concrete
/// receivers and callable-bound members keep their call paths.
#[test]
fn should_reject_a_member_call_on_a_bare_type_parameter() {
    let source = r#"module spec.eei.bareparam;

struct S {
    x: i32;
}

func g<T>(x: T): i32 {
    return x.foo(5);
}

func main(): i32 {
    return g<i32>(5);
}
"#;
    assert_rejected_once(
        "bare-param-member",
        source,
        "E10212",
        &["'foo' is not a member of 'T'"],
        &["E10307"],
    );

    let control = r#"module spec.eei.dotcontrol;

struct Holder<T> {
    call: T;
}

struct Plain {
    call: func(i32): i32;
    tag: i32;
}

func addOne(x: i32): i32 {
    return x + 1;
}

func use<T: func(i32): i32>(h: Holder<T>): i32 {
    return h.call(5);
}

func main(): i32 {
    let p: Plain = Plain { call: addOne, tag: 7 };
    return p.call(5) + p.tag + use<func(i32): i32>(Holder<func(i32): i32> { call: addOne });
}
"#;
    let output = run_host("dot-control", control);
    assert_exit("dot-control", &output, 19);
}

/// A value-position update on a `const` binding reports the immutable-assign
/// rule, in both the fixed-array and slice spellings.
#[test]
fn should_reject_a_value_update_on_a_const_binding() {
    let fixed = r#"module spec.eei.constupdate;

func main(): i32 {
    const a: u8[2] = [1, 2];
    let y: i32 = i32(a[0]++);
    return y;
}
"#;
    assert_rejected_once("const-update", fixed, "E10023", &[], &[]);

    let slice_source = r#"module spec.eei.constsliceupdate;

import { slice } from std.mem;

func main(): i32 {
    let base: u8[2] = [1, 2];
    const s: u8[] = slice(base, 0, 2);
    let y: i32 = i32(s[0]++);
    return y;
}
"#;
    assert_rejected_once("const-slice-update", slice_source, "E10023", &[], &[]);
}

/// A value-position update through an unmarked parameter reports the
/// read-only rule.
#[test]
fn should_reject_a_value_update_through_a_readonly_parameter() {
    let source = r#"module spec.eei.paramupdate;

import { slice } from std.mem;

func bump(xs: u8[]): i32 {
    let z: i32 = i32(xs[0]++);
    return z;
}

func main(): i32 {
    let a: u8[2] = [1, 2];
    return bump(slice(a, 0, 2));
}
"#;
    assert_rejected_once("readonly-update", source, "E10024", &[], &[]);
}

/// The `str` value update keeps its immutable-view report.
#[test]
fn should_reject_a_value_update_through_a_str_view() {
    let source = r#"module spec.eei.strvalueupdate;

func bump(borrow s: str): i32 {
    let w: i32 = i32(s[0]++);
    return w;
}

func main(): i32 {
    return bump("ab");
}
"#;
    assert_rejected_once(
        "str-value-update",
        source,
        "E10042",
        &["immutable view 's'"],
        &[],
    );
}

/// A `cstring` element write reports the immutable-view rule.
#[test]
fn should_reject_a_cstring_element_write() {
    let source = r#"module spec.eei.cstringwrite;

func fill(borrow c: cstring): i32 {
    c[0] = 65;
    return 0;
}

func main(): i32 {
    return fill("ab");
}
"#;
    assert_rejected_once(
        "cstring-write",
        source,
        "E10042",
        &["immutable view 'c'"],
        &[],
    );
}

/// The statement forms keep their taxonomy: a const element assign reports
/// the immutable-assign rule, and a writable slice write builds and runs.
#[test]
fn should_keep_the_statement_write_taxonomy() {
    let const_assign = r#"module spec.eei.constassign;

func main(): i32 {
    const a: u8[2] = [1, 2];
    a[0] = 5;
    return 0;
}
"#;
    assert_rejected_once("const-assign", const_assign, "E10023", &[], &[]);

    let writable = r#"module spec.eei.slicewrite;

import { slice } from std.mem;

func main(): i32 {
    let a: u8[2] = [1, 2];
    let s: u8[] = slice(a, 0, 2);
    s[0] = 7;
    return i32(s[0]) + i32(a[0]);
}
"#;
    let output = run_host("slice-write", writable);
    assert_exit("slice-write", &output, 14);
}

/// Whole-view rebinds and writable slice writes stay accepted and run.
#[test]
fn should_accept_view_rebinds_and_writable_writes() {
    let source = r#"module spec.eei.writecontrol;

import { slice } from std.mem;
import { stringOf, toCString } from std.text;

func main(): i32 {
    let a: u8[2] = [1, 2];
    let s: u8[] = slice(a, 0, 2);
    s[0] = 7;
    let t: str = "ab";
    t = "cd";
    let c: cstring = toCString(stringOf("ef"));
    c = toCString(stringOf("gh"));
    if (i32(t[0]) != 99) {
        return 1;
    }
    if (i32(c[0]) != 103) {
        return 2;
    }
    return i32(s[0]) + i32(a[0]);
}
"#;
    let output = run_host("write-control", source);
    assert_exit("write-control", &output, 14);
}

/// The view bound from an owned projection temporary releases its backing per
/// iteration.
///
/// `(Box { v: stringOf("ab") }).v` produces an owned `String` whose field the
/// `str` binding views. The binding must own the backing the view reads, so
/// each loop iteration releases the previous allocation; without an owner the
/// loop leaks one allocation per iteration (the base behavior).
#[test]
fn should_release_the_view_bound_projection_temporary_per_iteration() {
    let source = r#"module spec.eei.viewtemp;

import { String, stringOf } from std.text;

struct Box {
    v: String;
}

func measure(s: str): i32 {
    return i32(len(s));
}

func main(): i32 {
    let i: i32 = 0;
    let total: i32 = 0;
    while (i < 3) {
        let s: str = (Box { v: stringOf("ab") }).v;
        total = total + measure(s);
        i = i + 1;
    }
    return total;
}
"#;
    let output = run_memory("view-temp", source);
    assert_exit("view-temp", &output, 6);
    assert_no_asan("view-temp", &output);
}

/// A view that escapes its owner's scope reports the outlives-its-owner rule.
///
/// The view bound from an owned projection temporary is backed by a local in
/// the binding's scope; copying it into an outer binding and using it after
/// the scope ends reports `E10030`, exactly as a view of a named local does.
#[test]
fn should_reject_a_view_that_escapes_its_owner_scope() {
    let source = r#"module spec.eei.escapedowner;

import { String, stringOf } from std.text;

struct Box {
    v: String;
}

func use(s: str): i32 {
    return i32(len(s));
}

func main(): i32 {
    let s: str = "zz";
    let i: i32 = 0;
    while (i < 2) {
        let t: str = (Box { v: stringOf("ab") }).v;
        if (i == 0) { s = t; }
        i = i + 1;
    }
    return use(s);
}
"#;
    assert_rejected_once(
        "escaped-owner",
        source,
        "E10030",
        &["does not live long enough"],
        &[],
    );
}

/// The owned `String` destination control keeps its existing release.
#[test]
fn should_keep_the_string_destination_projection_clean() {
    let source = r#"module spec.eei.stringdest;

import { String, stringOf } from std.text;

struct Box {
    v: String;
}

func main(): i32 {
    let i: i32 = 0;
    let total: i32 = 0;
    while (i < 3) {
        let s: String = (Box { v: stringOf("ab") }).v;
        total = total + i32(len(s));
        i = i + 1;
    }
    return total;
}
"#;
    let output = run_memory("string-dest", source);
    assert_exit("string-dest", &output, 6);
    assert_no_asan("string-dest", &output);
}

/// A deferred `break` preempting an ordinary owned return releases the
/// abandoned value on the preemption edge.
///
/// The case body stores `stringOf("ab")` into the result, then the older
/// deferred scope breaks out of the switch, so the stored value is abandoned;
/// the preemption edge must release it (the base leaks it).
#[test]
fn should_release_the_preempted_ordinary_return() {
    let source = r#"module spec.eei.preempt;

import { String, stringOf } from std.text;

func f(): String {
    switch (1) {
        case 1: {
            defer { break; }
            return stringOf("ab");
        }
        default: {
        }
    }
    return stringOf("cd");
}

func main(): i32 {
    let r: String = f();
    return i32(len(r));
}
"#;
    let output = run_memory("preempt-return", source);
    assert_exit("preempt-return", &output, 2);
    assert_no_asan("preempt-return", &output);
}

/// An ordinary return with no reachable preemption keeps its behavior.
#[test]
fn should_keep_the_ordinary_return_clean() {
    let source = r#"module spec.eei.plainreturn;

import { String, stringOf } from std.text;

func f(): String {
    let i: i32 = 0;
    while (i < 5) {
        if (i == 1) { return stringOf("ab"); }
        i = i + 1;
    }
    return stringOf("cd");
}

func main(): i32 {
    let r: String = f();
    return i32(len(r));
}
"#;
    let output = run_memory("plain-return", source);
    assert_exit("plain-return", &output, 2);
    assert_no_asan("plain-return", &output);
}

/// The owned temporary argument of a call through a func-typed value binds
/// and releases like the module-call path.
#[test]
fn should_release_the_func_value_temporary_argument() {
    let source = r#"module spec.eei.funcvalue;

import { String, stringOf } from std.text;

func copier(borrow v: String): String {
    return stringOf("y");
}

func wrap(f: func(borrow String): String): i32 {
    return i32(len(f(stringOf("x"))));
}

func main(): i32 {
    return wrap(copier);
}
"#;
    let output = run_memory("func-value-arg", source);
    assert_exit("func-value-arg", &output, 1);
    assert_no_asan("func-value-arg", &output);
}

/// A block-bodied return of a by-value generic capture reports the ruled
/// capture-move diagnostic, once, at the capture.
#[test]
fn should_reject_the_block_bodied_capture_return() {
    let source = r#"module spec.eei.blockreturn;

import { String, stringOf } from std.text;

struct Wrap {
    v: String;
}

func outer<T>(x: T): T {
    return ([x] () => { return x; })();
}

func main(): i32 {
    let w: Wrap = Wrap { v: stringOf("ab") };
    let r: Wrap = outer<Wrap>(w);
    return i32(len(r.v));
}
"#;
    assert_rejected_once(
        "block-return",
        source,
        "E10401",
        &["cannot move captured 'x'"],
        &[],
    );
}

/// A block-bodied take of a by-value generic capture reports the ruled
/// capture-move diagnostic, once, at the call.
#[test]
fn should_reject_the_block_bodied_capture_take() {
    let source = r#"module spec.eei.blocktake;

import { String, stringOf } from std.text;

struct Wrap {
    v: String;
}

func sink(take w: Wrap): i32 {
    return 0;
}

func outer<T>(x: T): i32 {
    return ([x] () => { sink(take x); return 0; })();
}

func main(): i32 {
    let w: Wrap = Wrap { v: stringOf("ab") };
    return outer<Wrap>(w);
}
"#;
    assert_rejected_once(
        "block-take",
        source,
        "E10401",
        &["cannot move captured 'x'"],
        &[],
    );
}

/// A conditional capture return reports the capture-move diagnostic instead of
/// double-freeing at run time.
#[test]
fn should_reject_the_conditional_capture_return() {
    let source = r#"module spec.eei.conditional;

import { String, stringOf } from std.text;

const C: bool = true;

struct Wrap {
    v: String;
}

func outer<T>(x: T): T {
    return ([x] () => C ? x : x)();
}

func main(): i32 {
    let w: Wrap = Wrap { v: stringOf("ab") };
    let r: Wrap = outer<Wrap>(w);
    return i32(len(r.v));
}
"#;
    assert_rejected_once(
        "conditional-return",
        source,
        "E10401",
        &["cannot move captured 'x'"],
        &[],
    );
}

/// The concrete and take-argument conditional variants report the same
/// capture-move diagnostic, once each.
#[test]
fn should_reject_the_concrete_and_take_argument_conditional_captures() {
    let concrete = r#"module spec.eei.conditionalconcrete;

import { String, stringOf } from std.text;

const C: bool = true;

func outer(x: String): String {
    return ([x] () => C ? x : x)();
}

func main(): i32 {
    let r: String = outer(stringOf("ab"));
    return i32(len(r));
}
"#;
    assert_rejected_once(
        "conditional-concrete",
        concrete,
        "E10401",
        &["cannot move captured 'x'"],
        &[],
    );

    let take_argument = r#"module spec.eei.conditionaltake;

import { String, stringOf } from std.text;

const C: bool = true;

struct Wrap {
    v: String;
}

func sink(take w: Wrap): i32 {
    return 0;
}

func outer<T>(x: T): i32 {
    return ([x] () => sink(take (C ? x : x)))();
}

func main(): i32 {
    let w: Wrap = Wrap { v: stringOf("ab") };
    return outer<Wrap>(w);
}
"#;
    assert_rejected_once(
        "conditional-take",
        take_argument,
        "E10401",
        &["cannot move captured 'x'"],
        &[],
    );
}

/// The duplicable conditional instance stays accepted and runs.
#[test]
fn should_accept_the_conditional_capture_when_the_instance_copies() {
    let source = r#"module spec.eei.copycontrol;

const C: bool = true;

func outer<T>(x: T): T {
    return ([x] () => C ? x : x)();
}

func main(): i32 {
    return outer<i32>(42);
}
"#;
    let output = run_host("conditional-copy", source);
    assert_exit("conditional-copy", &output, 42);
}

/// A store reached again through a loop back edge releases the result a
/// preemption abandoned.
///
/// The `defer { continue; }` preempts the second store; the loop reaches the
/// first store again, which overwrites the abandoned value. The release must
/// cover every ordinary store in the function, not only the stores emitted
/// after the preempting return.
#[test]
fn should_release_the_preempted_result_before_a_back_edge_store() {
    let source = r#"module spec.eei.backedge;

import { String, stringOf } from std.text;

func f(): String {
    let i: i32 = 0;
    while (i < 4) {
        i = i + 1;
        if (i == 2) { return stringOf("aa"); }
        if (i == 1) { defer { continue; } return stringOf("bb"); }
    }
    return stringOf("cc");
}

func main(): i32 {
    let r: String = f();
    return i32(len(r));
}
"#;
    let output = run_memory("back-edge-preempt", source);
    assert_exit("back-edge-preempt", &output, 2);
    assert_no_asan("back-edge-preempt", &output);
}

// ---------------------------------------------------------------------------
// callviewescapes: an aggregate argument whose view reaches an out-slot and a
// call-mediated view of an inline temporary or a local source are judged at
// the call.
// ---------------------------------------------------------------------------

/// The aggregate out-slot escape and the call-mediated view escapes report.
///
/// `aggoutslot` passes a local `Box` to a callee that stores `h.v` into the
/// caller's out-slot; `callinline` and `callinlinedirect` take a view through a
/// call of an inline temporary array; `callglobal` stores a call-mediated view
/// of a local source in a global. Each reports the concrete twin's existing
/// diagnostic once. `callstatic` reads a call-mediated view of a module-level
/// literal and stays accepted.
#[test]
fn should_reject_view_escapes_through_calls_and_aggregates() {
    let aggregate = r#"module spec.eei.aggoutslot;

import { String, stringOf } from std.text;

struct Box { v: String; }

func store(h: Box, borrow slot: str?): void { slot = h.v; }

func outer(borrow slot: str?): void {
    let b: Box = Box { v: stringOf("aa") };
    store(b, borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;
    assert_rejected_once(
        "aggoutslot",
        aggregate,
        "E10034",
        &["cannot release or store borrowed parameter 'h'"],
        &["E10030"],
    );

    let inline = r#"module spec.eei.callinline;

import { String, stringOf } from std.text;

struct Box { v: String; }

func mk(): Box { return Box { v: stringOf("aa") }; }
func pick(xs: str[2], i: i32): str { return xs[i]; }
func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): void {
    let s: str = pick([mk().v; "EE"], 0);
    fill(s, borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;
    assert_rejected_once(
        "callinline",
        inline,
        "E10034",
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );

    let inline_direct = r#"module spec.eei.callinlinedirect;

import { String, stringOf } from std.text;

struct Box { v: String; }

func mk(): Box { return Box { v: stringOf("aa") }; }
func pick(xs: str[2], i: i32): str { return xs[i]; }
func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): void {
    fill(pick([mk().v; "EE"], 0), borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;
    assert_rejected_once(
        "callinlinedirect",
        inline_direct,
        "E10034",
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );

    let global = r#"module spec.eei.callglobal;

import { String, stringOf } from std.text;

struct Box { v: String; }

let g: str = "AA";

func pick(xs: str[2], i: i32): str { return xs[i]; }

func set(): void {
    let a: Box = Box { v: stringOf("zz") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    g = pick(xs, 0);
}

func main(): i32 {
    set();
    return i32(g[0]) - 122;
}
"#;
    assert_rejected_once(
        "callglobal",
        global,
        "E10033",
        &["borrow of 'g' escapes its scope"],
        &[],
    );

    let scalar = r#"module spec.eei.callscalar;

func pick(xs: str[2], i: i32): str { return xs[i]; }

func use(): i32 {
    let n: i32 = 0;
    let s: str = pick(["AA", "BB"], n);
    n = 1;
    return i32(len(s)) - 2 + n - 1;
}

func main(): i32 {
    return use();
}
"#;
    let output = run_memory("callscalar", scalar);
    assert_no_asan("callscalar", &output);
    assert_exit("callscalar", &output, 0);

    let static_bound = r#"module spec.eei.callstaticbound;

func pick(xs: str[2], i: i32): str { return xs[i]; }
func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): void {
    let s: str = pick(["AA", "BB"], 0);
    fill(s, borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 65;
}
"#;
    assert_rejected_once(
        "callstaticbound",
        static_bound,
        "E10034",
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );
    let static_aggregate = r#"module spec.eei.aggstatic;

struct Holder { names: str[2]; }

func store(h: Holder, borrow slot: str?): void { slot = h.names[0]; }

func outer(borrow slot: str?): void {
    let h: Holder = Holder { names: ["AA", "BB"] };
    store(h, borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 65;
}
"#;
    assert_rejected_once(
        "aggstatic",
        static_aggregate,
        "E10034",
        &["cannot release or store borrowed parameter 'h'"],
        &["E10030"],
    );
    let alias = r#"module spec.eei.aggalias;

import { String, stringOf } from std.text;

struct Box { v: String; }
struct Holder { names: str[2]; }

func store(h: Holder, borrow slot: str?): void { slot = h.names[0]; }

func outer(boxes: Box[1], borrow slot: str?): void {
    let hs: Holder[1] = [Holder { names: [boxes[0].v, "EE"] }];
    for (borrow e in hs) { store(e, borrow slot); }
}

func main(): i32 {
    let b: Box[1] = [Box { v: stringOf("aa") }];
    let out: str? = null;
    outer(b, borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;
    assert_rejected_once(
        "aggalias",
        alias,
        "E10034",
        &["cannot release or store borrowed parameter 'h'"],
        &["E10030"],
    );
    let temporary_aggregate = r#"module spec.eei.aggtemporary;

import { String, stringOf } from std.text;

struct Holder { names: str[2]; }
struct Box { v: String; }

func mk(): Box { return Box { v: stringOf("aa") }; }
func store(h: Holder, borrow slot: str?): void { slot = h.names[0]; }

func outer(borrow slot: str?): void {
    let h: Holder = Holder { names: [mk().v, "EE"] };
    store(h, borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    return 0;
}
"#;
    assert_rejected_once(
        "aggtemporary",
        temporary_aggregate,
        "E10034",
        &["cannot release or store borrowed parameter 'h'"],
        &["E10030"],
    );

    let call_carrier = r#"module spec.eei.aggcallcarrier;

import { String, stringOf } from std.text;

struct Holder { names: str[2]; }
struct Box { v: String; }

func make(v: str): Holder { return Holder { names: [v, "EE"] }; }
func store(h: Holder, borrow slot: str?): void { slot = h.names[0]; }

func outer(borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa") };
    let h: Holder = make(a.v);
    store(h, borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    return 0;
}
"#;
    assert_rejected_once(
        "aggcallcarrier",
        call_carrier,
        "E10034",
        &["cannot release or store borrowed parameter 'h'"],
        &["E10030"],
    );

    let mixed_scalar = r#"module spec.eei.aggmixedscalar;

struct Mixed { names: str[2]; n: i32; }

func use(c: bool): i32 {
    let q: i32 = 1;
    let h2: Mixed = c ? Mixed { names: ["AA", "BB"], n: q } : Mixed { names: ["CC", "DD"], n: 2 };
    q = 3;
    return h2.n + q - 4;
}

func main(): i32 {
    return use(true);
}
"#;
    let output = run_memory("aggmixedscalar", mixed_scalar);
    assert_no_asan("aggmixedscalar", &output);
    assert_exit("aggmixedscalar", &output, 0);

    let static_str = r#"module spec.eei.aggstaticstr;

struct H { name: static str; }

func store(h: H, borrow slot: str?): void { slot = h.name; }

func outer(borrow slot: str?): void {
    let p: static str = "aa";
    let h: H = H { name: p };
    store(h, borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;
    let output = run_memory("aggstaticstr", static_str);
    assert_no_asan("aggstaticstr", &output);
    assert_exit("aggstaticstr", &output, 0);

    let shadow_static = r#"module spec.eei.aggshadowstatic;

struct Holder { names: str[2]; }

func store(h: Holder, borrow slot: str?): void { slot = h.names[0]; }

func outer(borrow slot: str?): void {
    let h: Holder = Holder { names: ["AA", "EE"] };
    {
        let h: Holder = h;
        store(h, borrow slot);
    }
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 65;
}
"#;
    assert_rejected_once(
        "aggshadowstatic",
        shadow_static,
        "E10034",
        &["cannot release or store borrowed parameter 'h'"],
        &["E10030"],
    );
    let alias_copy = r#"module spec.eei.aggaliscopy;

import { String, stringOf } from std.text;

struct Box { v: String; }
struct Holder { names: str[2]; }

func store(h: Holder, borrow slot: str?): void { slot = h.names[0]; }

func outer(boxes: Box[1], borrow slot: str?): void {
    let hs: Holder[1] = [Holder { names: [boxes[0].v, "EE"] }];
    for (borrow e in hs) {
        let h2: Holder = e;
        store(h2, borrow slot);
    }
}

func main(): i32 {
    let b: Box[1] = [Box { v: stringOf("aa") }];
    let out: str? = null;
    outer(b, borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;
    assert_rejected_once(
        "aggaliscopy",
        alias_copy,
        "E10034",
        &["cannot release or store borrowed parameter 'h'"],
        &["E10030"],
    );
    let call_static = r#"module spec.eei.aggcallstatic;

struct Holder { names: str[2]; }

func make(v: str): Holder { return Holder { names: [v, "EE"] }; }
func store(h: Holder, borrow slot: str?): void { slot = h.names[0]; }

func outer(borrow slot: str?): void {
    let h: Holder = make("AA");
    store(h, borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 65;
}
"#;
    assert_rejected_once(
        "aggcallstatic",
        call_static,
        "E10034",
        &["cannot release or store borrowed parameter 'h'"],
        &["E10030"],
    );
    let call_param = r#"module spec.eei.aggcallparam;

struct Holder { names: str[2]; }

func make(v: str): Holder { return Holder { names: [v, "EE"] }; }
func store(h: Holder, borrow slot: str?): void { slot = h.names[0]; }

func outer(p: str, borrow slot: str?): void {
    let h: Holder = make(p);
    store(h, borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer("AA", borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 65;
}
"#;
    assert_rejected_once(
        "aggcallparam",
        call_param,
        "E10034",
        &["cannot release or store borrowed parameter 'h'"],
        &["E10030"],
    );
    let projected_alias = r#"module spec.eei.aggaliasprojected;

import { String, stringOf } from std.text;

struct S { v: String; }

func store(p: str, borrow slot: str?): void { slot = p; }

func outer(ss: S[1], borrow slot: str?): void {
    for (borrow e in ss) { store(e.v, borrow slot); }
}

func main(): i32 {
    let ss: S[1] = [S { v: stringOf("aa") }];
    let out: str? = null;
    outer(ss, borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;
    assert_rejected_once(
        "aggaliasprojected",
        projected_alias,
        "E10034",
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );
    let take_aggregate = r#"module spec.eei.aggtake;

import { String, stringOf } from std.text;

struct Box { v: String; }

func store(take h: Box, borrow slot: str?): void { slot = h.v; }

func outer(borrow slot: str?): void {
    let b: Box = Box { v: stringOf("aa") };
    store(b, borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    return 0;
}
"#;
    assert_rejected_once(
        "aggtake",
        take_aggregate,
        "E10030",
        &["borrow of 'h' does not live long enough"],
        &[],
    );

    let static_view = r#"module spec.eei.callstatic;

func pick(xs: str[2], i: i32): str { return xs[i]; }

func use(): i32 {
    let s: str = pick(["AA", "BB"], 0);
    return i32(len(s)) - 2;
}

func main(): i32 {
    return use();
}
"#;
    let output = run_memory("callstatic", static_view);
    assert_no_asan("callstatic", &output);
    assert_exit("callstatic", &output, 0);
}

/// A function-returned call-mediated view consults its source's element loans.
///
/// `return pick(xs, 0)` after `xs[0] = a.v` hands the caller a view of `a`'s
/// storage, so the return reports the concrete twin's `E10035` once. A
/// parameter-backed source stays accepted and clean.
#[test]
fn should_reject_a_returned_call_mediated_view_over_a_frame_backing() {
    let source = r#"module spec.eei.returnview;

import { String, stringOf } from std.text;

struct Box { v: String; }

func pick(xs: str[2], i: i32): str { return xs[i]; }

func outer(): str {
    let a: Box = Box { v: stringOf("aa") };
    let xs: str[2] = ["CC", "DD"];
    xs[0] = a.v;
    return pick(xs, 0);
}

func main(): i32 {
    let s: str = outer();
    return i32(s[0]) - 97;
}
"#;
    assert_rejected_once(
        "returnview",
        source,
        "E10035",
        &["cannot return a view of local 'a'"],
        &[],
    );

    let dot = r#"module spec.eei.returndotview;

import { String, stringOf } from std.text;

struct Box { v: String; }

func first(xs: str[2], i: i32): str { return xs[i]; }

func outer(): str {
    let a: Box = Box { v: stringOf("aa") };
    let xs: str[2] = ["CC", "DD"];
    xs[0] = a.v;
    return xs.first(0);
}

func main(): i32 {
    let s: str = outer();
    return i32(s[0]) - 97;
}
"#;
    assert_rejected_once(
        "returndotview",
        dot,
        "E10035",
        &["cannot return a view of local 'a'"],
        &[],
    );

    let control = r#"module spec.eei.returnviewok;

func pick(xs: str[2], i: i32): str { return xs[i]; }

func outer(borrow xs: str[2]): str {
    return pick(xs, 0);
}

func main(): i32 {
    let xs: str[2] = ["AA", "BB"];
    let s: str = outer(borrow xs);
    return i32(s[0]) - 65;
}
"#;
    let output = run_memory("returnviewok", control);
    assert_no_asan("returnviewok", &output);
    assert_exit("returnviewok", &output, 0);
}

/// An inline call-result carrier with a frame-backed view is judged at the
/// call.
///
/// `store(make(a.v), borrow slot)` hands the callee a carrier whose view reads
/// the caller's local `a`, so the call boundary reports `E10030` once. A
/// static argument stays accepted and clean.
#[test]
fn should_reject_an_inline_call_carrier_with_a_frame_backing() {
    let source = r#"module spec.eei.inlinecarrier;

import { String, stringOf } from std.text;

struct Holder { names: str[2]; }
struct Box { v: String; }

func make(v: str): Holder { return Holder { names: [v, "EE"] }; }
func store(h: Holder, borrow slot: str?): void { slot = h.names[0]; }

func outer(borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa") };
    store(make(a.v), borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;
    assert_rejected_once(
        "inlinecarrier",
        source,
        "E10034",
        &["cannot release or store borrowed parameter 'h'"],
        &["E10030"],
    );

    let control = r#"module spec.eei.inlinecarrierok;

struct Holder { names: str[2]; }

func make(v: str): Holder { return Holder { names: [v, "EE"] }; }
func store(h: Holder, borrow slot: str?): void { slot = h.names[0]; }

func outer(borrow slot: str?): void {
    store(make("AA"), borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 65;
}
"#;
    assert_rejected_once(
        "inlinecarrierok",
        control,
        "E10034",
        &["cannot release or store borrowed parameter 'h'"],
        &["E10030"],
    );}

/// A conditional that mixes a static literal and a call carrier records the
/// call arm's backing.
///
/// The selected arm may read the caller's local `a`, so `store(h, slot)`
/// reports `E10030` once at the store. The all-static conditional stays
/// accepted and clean.
#[test]
fn should_reject_a_conditional_call_carrier_arm_with_a_frame_backing() {
    let source = r#"module spec.eei.condcallcarrier;

import { String, stringOf } from std.text;

struct Holder { names: str[2]; }
struct Box { v: String; }

func make(v: str): Holder { return Holder { names: [v, "EE"] }; }
func store(h: Holder, borrow slot: str?): void { slot = h.names[0]; }

func outer(c: bool, borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa") };
    let h: Holder = c ? Holder { names: ["AA", "BB"] } : make(a.v);
    store(h, borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(false, borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;
    assert_rejected_once(
        "condcallcarrier",
        source,
        "E10034",
        &["cannot release or store borrowed parameter 'h'"],
        &["E10030"],
    );

    let control = r#"module spec.eei.condcallcarrierok;

struct Holder { names: str[2]; }

func make(v: str): Holder { return Holder { names: [v, "EE"] }; }
func store(h: Holder, borrow slot: str?): void { slot = h.names[0]; }

func outer(c: bool, borrow slot: str?): void {
    let h: Holder = c ? Holder { names: ["AA", "BB"] } : make("CC");
    store(h, borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(false, borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 67;
}
"#;
    assert_rejected_once(
        "condcallcarrierok",
        control,
        "E10034",
        &["cannot release or store borrowed parameter 'h'"],
        &["E10030"],
    );}

/// Direct call-mediated views resolve every callable spelling.
///
/// The closure, func-typed binding, unsafe wrapper, and conditional wrapper
/// each name the same single lend source as a direct call, so a frame-backed
/// source reports `E10030` once. A caller-backed closure stays accepted and
/// clean.
#[test]
fn should_reject_direct_call_mediated_views_in_every_callable_form() {
    let closure = r#"module spec.eei.closureview;

import { String, stringOf } from std.text;

struct Box { v: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    let f = (ys: str[2], i: i32) => ys[i];
    fill(f(xs, 0), borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;
    assert_rejected_once(
        "closureview",
        closure,
        "E10034",
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );

    let func_local = r#"module spec.eei.funclocalview;

import { String, stringOf } from std.text;

struct Box { v: String; }

func fill(p: str, borrow slot: str?): void { slot = p; }
func first(xs: str[2], i: i32): str { return xs[i]; }

func outer(borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    let f: func(str[2], i32): str = first;
    fill(f(xs, 0), borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;
    assert_rejected_once(
        "funclocalview",
        func_local,
        "E10034",
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );

    let wrapped = r#"module spec.eei.wrappedview;

import { String, stringOf } from std.text;

struct Box { v: String; }

func mk(): Box { return Box { v: stringOf("aa") }; }
func pick(xs: str[2], i: i32): str { return xs[i]; }
func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): void {
    fill(unsafe (pick([mk().v; "EE"], 0)), borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;
    assert_rejected_once(
        "wrappedview",
        wrapped,
        "E10034",
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );

    let conditional = r#"module spec.eei.condwrappedview;

import { String, stringOf } from std.text;

struct Box { v: String; }

func mk(): Box { return Box { v: stringOf("aa") }; }
func pick(xs: str[2], i: i32): str { return xs[i]; }
func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): void {
    fill(true ? pick([mk().v; "EE"], 0) : "XX", borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;
    assert_rejected_once(
        "condwrappedview",
        conditional,
        "E10034",
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );

    let func_field = r#"module spec.eei.funcfieldview;

import { String, stringOf } from std.text;

struct Box { v: String; }
struct Dispatch { f: func(str[2], i32): str; }

func first(xs: str[2], i: i32): str { return xs[i]; }
func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow d: Dispatch, borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    fill(d.f(xs, 0), borrow slot);
}

func main(): i32 {
    let d: Dispatch = Dispatch { f: first };
    let out: str? = null;
    outer(borrow d, borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;
    assert_rejected_once(
        "funcfieldview",
        func_field,
        "E10034",
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );

    let generic_call = r#"module spec.eei.genericview;

import { String, stringOf } from std.text;

struct Box { v: String; }

func pick<T>(xs: T[2], i: i32): T { return xs[i]; }
func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    fill(pick<str>(xs, 0), borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;
    assert_rejected_once(
        "genericview",
        generic_call,
        "E10034",
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );

    let returned_callable = r#"module spec.eei.returnedcallableview;

import { String, stringOf } from std.text;

struct Box { v: String; }

func first(xs: str[2], i: i32): str { return xs[i]; }
func fill(p: str, borrow slot: str?): void { slot = p; }

func picker(): func(str[2], i32): str { return first; }

func outer(borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    fill(picker()(xs, 0), borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;
    assert_rejected_once(
        "returnedcallableview",
        returned_callable,
        "E10034",
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );

    let control = r#"module spec.eei.closureviewok;

func fill(p: str, borrow slot: str?): void { slot = p; }

func outer(xs: str[2], borrow slot: str?): void {
    let f = (ys: str[2], i: i32) => ys[i];
    fill(f(xs, 0), borrow slot);
}

func main(): i32 {
    let xs: str[2] = ["AA", "BB"];
    let out: str? = null;
    outer(xs, borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 65;
}
"#;
    assert_rejected_once(
        "closureviewok",
        control,
        "E10034",
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );}

/// A dot-call receiver is judged as a view source at parameter zero.
///
/// A local receiver (`b.store(borrow slot)`) and a fresh receiver
/// (`mk().store(borrow slot)`) both report `E10030` once; a caller-backed
/// receiver stays accepted and clean.
#[test]
fn should_judge_a_dot_call_receiver_as_a_view_source() {
    let local = r#"module spec.eei.dotreceiver;

import { String, stringOf } from std.text;

struct Box { v: String; }

func store(h: Box, borrow slot: str?): void { slot = h.v; }

func outer(borrow slot: str?): void {
    let b: Box = Box { v: stringOf("aa") };
    b.store(borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;
    assert_rejected_once(
        "dotreceiver",
        local,
        "E10034",
        &["cannot release or store borrowed parameter 'h'"],
        &["E10030"],
    );

    let temp = r#"module spec.eei.dottemp;

import { String, stringOf } from std.text;

struct Box { v: String; }

func mk(): Box { return Box { v: stringOf("aa") }; }
func store(h: Box, borrow slot: str?): void { slot = h.v; }

func outer(borrow slot: str?): void {
    mk().store(borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;
    assert_rejected_once(
        "dottemp",
        temp,
        "E10034",
        &["cannot release or store borrowed parameter 'h'"],
        &["E10030"],
    );

    let view_control = r#"module spec.eei.dotviewok;

import { String, stringOf } from std.text;

struct Box { v: String; }

func storeStr(h: str, borrow slot: str?): void { slot = h; }

func outer(borrow p: Box, borrow slot: str?): void {
    let s: str = p.v;
    s.storeStr(borrow slot);
}

func main(): i32 {
    let b: Box = Box { v: stringOf("aa") };
    let out: str? = null;
    outer(borrow b, borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;
    assert_rejected_once(
        "dotviewok",
        view_control,
        "E10034",
        &["cannot release or store borrowed parameter 'h'"],
        &["E10030"],
    );
    let static_view = r#"module spec.eei.dotstaticok;

struct Box { v: str[2]; }

func store(h: str[2], borrow slot: str?): void { slot = h[0]; }

func outer(borrow slot: str?): void {
    let v: str[2] = ["AA", "BB"];
    v.store(borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 65;
}
"#;
    assert_rejected_once(
        "dotstaticok",
        static_view,
        "E10034",
        &["cannot release or store borrowed parameter 'h'"],
        &["E10030"],
    );
    let control = r#"module spec.eei.dotreceiverok;

import { String, stringOf } from std.text;

struct Box { v: String; }

func store(h: Box, borrow slot: str?): void { slot = h.v; }

func outer(borrow b: Box, borrow slot: str?): void {
    b.store(borrow slot);
}

func main(): i32 {
    let b: Box = Box { v: stringOf("aa") };
    let out: str? = null;
    outer(borrow b, borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;
    assert_rejected_once(
        "dotreceiverok",
        control,
        "E10034",
        &["cannot release or store borrowed parameter 'h'"],
        &["E10030"],
    );}

/// A carrier-returning dot call, a module-level func value, and a carrier
/// return follow their source's storage.
///
/// The dot receiver and the global callable each name the same single source a
/// free call does, so a frame-backed source reports `E10030` once; a carrier
/// return reports the concrete twin's `E10030` at the return. The caller-backed
/// controls stay accepted and clean.
#[test]
fn should_follow_a_carrier_receiver_a_global_callable_and_a_carrier_return() {
    let receiver = r#"module spec.eei.carrierreceiver;

import { String, stringOf } from std.text;

struct Holder { names: str[2]; }
struct Box { v: String; }

func makeFrom(b: Box): Holder { return Holder { names: [b.v, "EE"] }; }
func store(h: Holder, borrow slot: str?): void { slot = h.names[0]; }

func outer(borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa") };
    a.makeFrom().store(borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;
    assert_rejected_once(
        "carrierreceiver",
        receiver,
        "E10034",
        &["cannot release or store borrowed parameter 'h'"],
        &["E10030"],
    );

    let global_callable = r#"module spec.eei.globalcallable;

import { String, stringOf } from std.text;

struct Box { v: String; }

func first(xs: str[2], i: i32): str { return xs[i]; }
func fill(p: str, borrow slot: str?): void { slot = p; }

let g: func(str[2], i32): str = first;

func outer(borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa") };
    let xs: str[2] = ["AA", "BB"];
    xs[0] = a.v;
    fill(g(xs, 0), borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;
    assert_rejected_once(
        "globalcallable",
        global_callable,
        "E10034",
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );

    let carrier_return = r#"module spec.eei.carrierreturn;

import { String, stringOf } from std.text;

struct Holder { names: str[2]; }
struct Box { v: String; }

func makeFrom(b: Box): Holder { return Holder { names: [b.v, "EE"] }; }

func outer(): Holder {
    let a: Box = Box { v: stringOf("aa") };
    return makeFrom(a);
}

func main(): i32 {
    let h: Holder = outer();
    return i32(h.names[0][0]) - 97;
}
"#;
    assert_rejected_once(
        "carrierreturn",
        carrier_return,
        "E10030",
        &["does not live long enough"],
        &[],
    );

    let control = r#"module spec.eei.carrierreceiverok;

struct Holder { names: str[2]; }

func makeFrom(v: str): Holder { return Holder { names: [v, "EE"] }; }

func outer(borrow slot: str?): void {
    let h: Holder = makeFrom("AA");
    store(h, borrow slot);
}

func store(h: Holder, borrow slot: str?): void { slot = h.names[0]; }

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 65;
}
"#;
    assert_rejected_once(
        "carrierreceiverok",
        control,
        "E10034",
        &["cannot release or store borrowed parameter 'h'"],
        &["E10030"],
    );}

/// A loop-alias receiver used as an out-slot still roots in the container.
///
/// The alias has no storage of its own, so the container's caller-backed slot
/// makes the pass-through a cross-frame write; the local source reports
/// `E10030` once.
#[test]
fn should_resolve_a_loop_alias_receiver_slot_across_frames() {
    let source = r#"module spec.eei.aliasreceiver;

import { String, stringOf } from std.text;

struct Box { v: String; }
struct Slot { tag: String; ss: str[2]; }

func fill(borrow slot: Slot, p: str): void { slot.ss[0] = p; }

func outer(borrow slots: Slot[1]): void {
    let a: Box = Box { v: stringOf("aa") };
    let p: str = a.v;
    for (borrow s in slots) {
        s.fill(p);
    }
}

func main(): i32 {
    let slots: Slot[1] = [Slot { tag: stringOf("t"), ss: ["AA", "BB"] }];
    outer(borrow slots);
    return i32(slots[0].ss[0][0]) - 97;
}
"#;
    assert_rejected_once(
        "aliasreceiver",
        source,
        "E10030",
        &["does not live long enough"],
        &[],
    );
}

// ---------------------------------------------------------------------------
// Carrier sources and callable expressions (Batch Q)
// ---------------------------------------------------------------------------

/// A projected member of a carrier-returning call is judged at the frame that
/// receives it.
///
/// `make(a.v).names` selects a view position the returned carrier derives from
/// the caller's local `a`, so the out-slot store reports `E10030` once. The
/// static control stays accepted and clean.
#[test]
fn should_reject_a_projected_carrier_member_with_a_frame_backing() {
    let source = r#"module spec.eei.projcarrier;

import { String, stringOf } from std.text;

struct Holder { names: str[2]; }
struct Box { v: String; }

func make(v: str): Holder { return Holder { names: [v, "EE"] }; }
func store(h: str[2], borrow slot: str?): void { slot = h[0]; }

func outer(borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa") };
    store(make(a.v).names, borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;
    assert_rejected_once(
        "projcarrier",
        source,
        "E10034",
        &["cannot release or store borrowed parameter 'h'"],
        &["E10030"],
    );

    let control = r#"module spec.eei.projcarrierok;

struct Holder { names: str[2]; }

func make(v: str): Holder { return Holder { names: [v, "EE"] }; }
func store(h: str[2], borrow slot: str?): void { slot = h[0]; }

func outer(borrow slot: str?): void {
    store(make("AA").names, borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 65;
}
"#;
    assert_rejected_once(
        "projcarrierok",
        control,
        "E10034",
        &["cannot release or store borrowed parameter 'h'"],
        &["E10030"],
    );}

/// A carrier-returning call with several lend sources contributes every
/// source's view positions.
///
/// `makePair(a.v, b.v)` may return a carrier holding views of either argument,
/// so the out-slot store names the first frame-dying source and reports
/// `E10030` once. The all-static pair stays accepted and clean.
#[test]
fn should_reject_a_two_source_carrier_call_with_a_frame_backing() {
    let source = r#"module spec.eei.paircarrier;

import { String, stringOf } from std.text;

struct Holder { names: str[2]; }
struct Box { v: String; }

func makePair(x: str, y: str): Holder { return Holder { names: [x, y] }; }
func store(h: Holder, borrow slot: str?): void { slot = h.names[0]; }

func outer(borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa") };
    let b: Box = Box { v: stringOf("bb") };
    store(makePair(a.v, b.v), borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;
    assert_rejected_once(
        "paircarrier",
        source,
        "E10034",
        &["cannot release or store borrowed parameter 'h'"],
        &["E10030"],
    );

    let control = r#"module spec.eei.paircarrierok;

struct Holder { names: str[2]; }

func makePair(x: str, y: str): Holder { return Holder { names: [x, y] }; }
func store(h: Holder, borrow slot: str?): void { slot = h.names[0]; }

func outer(borrow slot: str?): void {
    store(makePair("AA", "BB"), borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 65;
}
"#;
    assert_rejected_once(
        "paircarrierok",
        control,
        "E10034",
        &["cannot release or store borrowed parameter 'h'"],
        &["E10030"],
    );}

/// A carrier return through an owned aggregate or a `take` argument follows
/// that source.
///
/// `makeHolder(x, b)` views `b.v`, so the caller's store reports `E10030`
/// once. `id(take h1)` hands `h1`'s views back; the caller's store reports the
/// same code once, while returning and using the identity result in the same
/// frame stays accepted and clean.
#[test]
fn should_reject_a_carrier_call_through_an_owned_or_take_source() {
    let aggregate = r#"module spec.eei.ownedcarrier;

import { String, stringOf } from std.text;

struct Holder { names: str[2]; }
struct Box { v: String; }

func makeHolder(x: str, h: Box): Holder { return Holder { names: [x, h.v] }; }
func store(h: Holder, borrow slot: str?): void { slot = h.names[1]; }

func outer(borrow slot: str?): void {
    let b: Box = Box { v: stringOf("aa") };
    let h: Holder = makeHolder("AA", b);
    store(h, borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;
    assert_rejected_once(
        "ownedcarrier",
        aggregate,
        "E10034",
        &["cannot release or store borrowed parameter 'h'"],
        &["E10030"],
    );

    let identity = r#"module spec.eei.takecarrier;

import { String, stringOf } from std.text;

struct Holder { names: str[2]; }
struct Box { v: String; }

func id(take h: Holder): Holder { return h; }
func store(h: Holder, borrow slot: str?): void { slot = h.names[0]; }

func outer(borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa") };
    let h1: Holder = Holder { names: [a.v, "EE"] };
    let h2: Holder = id(take h1);
    store(h2, borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;
    assert_rejected_once(
        "takecarrier",
        identity,
        "E10034",
        &["cannot release or store borrowed parameter 'h'"],
        &["E10030"],
    );

    let generic_identity = r#"module spec.eei.takecarriergeneric;

import { String, stringOf } from std.text;

struct Holder { names: str[2]; }
struct Box { v: String; }

func id<T>(take h: T): T { return h; }
func store(h: Holder, borrow slot: str?): void { slot = h.names[0]; }

func outer(borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa") };
    let h1: Holder = Holder { names: [a.v, "EE"] };
    store(id<Holder>(take h1), borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;
    assert_rejected_once(
        "takecarriergeneric",
        generic_identity,
        "E10034",
        &["cannot release or store borrowed parameter 'h'"],
        &["E10030"],
    );

    let local_view_return = r#"module spec.eei.takecarrierlocalview;

import { String, stringOf } from std.text;

struct Box { v: String; }

func id<T>(take h: T): T {
    let a: Box = Box { v: stringOf("aa") };
    let s: str = a.v;
    return s;
}

func store(s: str, borrow slot: str?): void { slot = s; }

func outer(borrow slot: str?): void {
    let p: str = "PP";
    store(id<str>(take p), borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 65;
}
"#;
    assert_rejected_once(
        "takecarrierlocalview",
        local_view_return,
        "E10035",
        &["cannot return a view of local 'a'"],
        &[],
    );

    // A view carried through a local still reaches its parameter.
    let local_hop = r#"module spec.eei.takecarrierlocalhop;

import { String, stringOf } from std.text;

struct Holder { names: str[2]; }
struct Box { v: String; }

func make(b: Box): Holder {
    let v: str = b.v;
    return Holder { names: [v, "EE"] };
}
func store(h: Holder, borrow slot: str?): void { slot = h.names[0]; }

func outer(borrow slot: str?): void {
    let b: Box = Box { v: stringOf("aa") };
    store(make(b), borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;
    assert_rejected_once(
        "takecarrierlocalhop",
        local_hop,
        "E10034",
        &["cannot release or store borrowed parameter 'h'"],
        &["E10030"],
    );

    let projected_hop = r#"module spec.eei.takecarrierprojected;

import { String, stringOf } from std.text;

struct Holder { names: str[2]; }
struct Box { v: String; }

func make(b: Box): Holder {
    let h: Holder = Holder { names: ["AA", "EE"] };
    h.names = [b.v, "EE"];
    return h;
}
func store(h: Holder, borrow slot: str?): void { slot = h.names[0]; }

func outer(borrow slot: str?): void {
    let b: Box = Box { v: stringOf("aa") };
    store(make(b), borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;
    assert_rejected_once(
        "takecarrierprojected",
        projected_hop,
        "E10034",
        &["cannot release or store borrowed parameter 'h'"],
        &["E10030"],
    );

    let control = r#"module spec.eei.takecarrierok;

struct Holder { names: str[2]; }

func id(take h: Holder): Holder { return h; }

func main(): i32 {
    let h: Holder = Holder { names: ["AA", "BB"] };
    let g: Holder = id(take h);
    return i32(g.names[0][0]) - 65;
}
"#;
    let output = run_memory("takecarrierok", control);
    assert_no_asan("takecarrierok", &output);
    assert_exit("takecarrierok", &output, 0);

    // An identity relay forwards the take parameter and hands it back.
    let relay = r#"module spec.eei.takecarrierrelay;

struct Holder { names: str[2]; }

func id(take h: Holder): Holder { return h; }
func relay(take h: Holder): Holder { return id(take h); }

func main(): i32 {
    let h: Holder = Holder { names: ["AA", "BB"] };
    let g: Holder = relay(take h);
    return i32(g.names[0][0]) - 65;
}
"#;
    let output = run_memory("takecarrierrelay", relay);
    assert_no_asan("takecarrierrelay", &output);
    assert_exit("takecarrierrelay", &output, 0);

    // A conditional of two take parameters moves the selected arm out.
    let selected = r#"module spec.eei.takecarrierselect;

struct Holder { names: str[2]; }

func id(c: bool, take h1: Holder, take h2: Holder): Holder { return c ? h1 : h2; }

func main(): i32 {
    let a: Holder = Holder { names: ["AA", "BB"] };
    let b: Holder = Holder { names: ["CC", "DD"] };
    let g: Holder = id(true, take a, take b);
    return i32(g.names[0][0]) - 65;
}
"#;
    let output = run_memory("takecarrierselect", selected);
    assert_no_asan("takecarrierselect", &output);
    assert_exit("takecarrierselect", &output, 0);
}

/// A view stored into a type-parameter member is judged at its instance.
///
/// The declaration cannot see whether `T`'s member holds a view or an owner.
/// At `T = Holder` the member is a `str`, so a view of the frame's local `a`
/// stored into it and returned with the `take` carrier escapes (`E10030`).
/// A caller-backed parameter view stays accepted, and an owned member at
/// `T = Box` reports the concrete implicit conversion.
#[test]
fn should_judge_a_view_store_into_an_opaque_member_at_its_instance() {
    let local_view = r#"module spec.eei.opaqueviewsore;

import { String, stringOf } from std.text;

struct Holder { names: str[2]; }
struct Box { v: String; }

func id<T>(take h: T): T {
    let a: Box = Box { v: stringOf("aa") };
    let s: str = a.v;
    h.names[0] = s;
    return h;
}

func main(): i32 {
    let h: Holder = Holder { names: ["EE", "FF"] };
    let g: Holder = id<Holder>(take h);
    return i32(g.names[0][0]) - 97;
}
"#;
    assert_rejected_once(
        "opaqueviewsore",
        local_view,
        "E10030",
        &["borrow of 'a' does not live long enough"],
        &[],
    );

    let caller_view = r#"module spec.eei.opaqueviewsorecaller;

struct Holder { names: str[2]; }

func id<T>(take h: T, p: str): T {
    let v: str = p;
    h.names[0] = v;
    return h;
}

func main(): i32 {
    let h: Holder = Holder { names: ["EE", "FF"] };
    let g: Holder = id<Holder>(take h, "aa");
    return i32(g.names[0][0]) - 97;
}
"#;
    let output = run_memory("opaqueviewsorecaller", caller_view);
    assert_no_asan("opaqueviewsorecaller", &output);
    assert_exit("opaqueviewsorecaller", &output, 0);

    let owned_member = r#"module spec.eei.opaqueviewsoreowned;

import { String, stringOf } from std.text;

struct Box { v: String; }

func id<T>(take h: T): T {
    let a: Box = Box { v: stringOf("aa") };
    let s: str = a.v;
    h.v = s;
    return h;
}

func main(): i32 {
    let b: Box = Box { v: stringOf("EE") };
    let g: Box = id<Box>(take b);
    return i32(len(g.v)) - 2;
}
"#;
    assert_rejected_once(
        "opaqueviewsoreowned",
        owned_member,
        "E10200",
        &["cannot convert 'str' to 'String'"],
        &[],
    );

    // An owned local widened to the view member is a store, not a move: the
    // source stays readable and the emitted member holds the widened view.
    let widen = r#"module spec.eei.opaqueviewsorewiden;

import { String, stringOf } from std.text;

struct Holder { names: str[2]; }
struct Box { v: String; }

func f<T>(take h: T): i32 {
    let a: Box = Box { v: stringOf("aa") };
    h.names[0] = a.v;
    return i32(h.names[0][0]) - 97;
}

func main(): i32 {
    let h: Holder = Holder { names: ["EE", "FF"] };
    return f<Holder>(take h);
}
"#;
    let output = run_memory("opaqueviewsorewiden", widen);
    assert_no_asan("opaqueviewsorewiden", &output);
    assert_exit("opaqueviewsorewiden", &output, 0);
}

/// A stored view follows the carrier through conditionals, copies, and calls.
///
/// The carrier's stored view escapes whenever the returned value can select or
/// relay the binding. Each shape reports `E10030` once at the return, exactly
/// as its concrete twin does.
#[test]
fn should_follow_an_opaque_store_escape_through_carrier_returns() {
    let conditional = r#"module spec.eei.opaqueviewsorecond;

import { String, stringOf } from std.text;

struct Holder { names: str[2]; }
struct Box { v: String; }

func id<T>(take h: T, take h2: T, c: bool): T {
    let a: Box = Box { v: stringOf("aa") };
    h.names[0] = a.v;
    return c ? h : h2;
}

func main(): i32 {
    let h: Holder = Holder { names: ["EE", "FF"] };
    let h2: Holder = Holder { names: ["GG", "HH"] };
    let g: Holder = id<Holder>(take h, take h2, true);
    return i32(g.names[0][0]) - 97;
}
"#;
    assert_rejected_once(
        "opaqueviewsorecond",
        conditional,
        "E10030",
        &["does not live long enough"],
        &[],
    );

    let copied = r#"module spec.eei.opaqueviewsorecopy;

import { String, stringOf } from std.text;

struct Holder { names: str[2]; }
struct Box { v: String; }

func id<T>(take h: T): T {
    let a: Box = Box { v: stringOf("aa") };
    h.names[0] = a.v;
    let out: T = h;
    return out;
}

func main(): i32 {
    let h: Holder = Holder { names: ["EE", "FF"] };
    let g: Holder = id<Holder>(take h);
    return i32(g.names[0][0]) - 97;
}
"#;
    assert_rejected_once(
        "opaqueviewsorecopy",
        copied,
        "E10030",
        &["does not live long enough"],
        &[],
    );

    let relayed = r#"module spec.eei.opaqueviewsorerelay;

import { String, stringOf } from std.text;

struct Holder { names: str[2]; }
struct Box { v: String; }

func echo<T>(take h: T): T { return h; }

func id<T>(take h: T): T {
    let a: Box = Box { v: stringOf("aa") };
    h.names[0] = a.v;
    return echo<T>(take h);
}

func main(): i32 {
    let h: Holder = Holder { names: ["EE", "FF"] };
    let g: Holder = id<Holder>(take h);
    return i32(g.names[0][0]) - 97;
}
"#;
    assert_rejected_once(
        "opaqueviewsorerelay",
        relayed,
        "E10030",
        &["does not live long enough"],
        &[],
    );

    let loop_alias = r#"module spec.eei.opaqueviewsoreloop;

import { String, stringOf } from std.text;

struct Holder { names: str[2]; }
struct Box { v: String; }

func f<T>(take xs: T[1]): T[1] {
    let a: Box = Box { v: stringOf("aa") };
    let s: str = a.v;
    for (borrow e in xs) { e.names[0] = s; }
    return xs;
}

func main(): i32 {
    let xs: Holder[1] = [Holder { names: ["EE", "FF"] }];
    let g: Holder[1] = f<Holder>(take xs);
    return i32(g[0].names[0][0]) - 97;
}
"#;
    assert_rejected_once(
        "opaqueviewsoreloop",
        loop_alias,
        "E10030",
        &["does not live long enough"],
        &[],
    );

    let const_copy = r#"module spec.eei.opaqueviewsoreconst;

import { String, stringOf } from std.text;

struct Holder { names: str[2]; }
struct Box { v: String; }

func id<T>(take h: T): T {
    let a: Box = Box { v: stringOf("aa") };
    let s: str = a.v;
    h.names[0] = s;
    const out: T = h;
    return out;
}

func main(): i32 {
    let h: Holder = Holder { names: ["EE", "FF"] };
    let g: Holder = id<Holder>(take h);
    return i32(g.names[0][0]) - 97;
}
"#;
    assert_rejected_once(
        "opaqueviewsoreconst",
        const_copy,
        "E10030",
        &["does not live long enough"],
        &[],
    );

    let assigned = r#"module spec.eei.opaqueviewsoreassign;

import { String, stringOf } from std.text;

struct Holder { names: str[2]; }
struct Box { v: String; }

func id<T>(take h: T, take h2: T): T {
    let a: Box = Box { v: stringOf("aa") };
    let s: str = a.v;
    h.names[0] = s;
    let out: T = h2;
    out = h;
    return out;
}

func main(): i32 {
    let h: Holder = Holder { names: ["EE", "FF"] };
    let h2: Holder = Holder { names: ["GG", "HH"] };
    let g: Holder = id<Holder>(take h, take h2);
    return i32(g.names[0][0]) - 97;
}
"#;
    assert_rejected_once(
        "opaqueviewsoreassign",
        assigned,
        "E10030",
        &["does not live long enough"],
        &[],
    );

    let conditional_init = r#"module spec.eei.opaqueviewsorecondinit;

import { String, stringOf } from std.text;

struct Holder { names: str[2]; }
struct Box { v: String; }

func id<T>(take h: T, take h2: T, c: bool): T {
    let a: Box = Box { v: stringOf("aa") };
    let s: str = a.v;
    h.names[0] = s;
    let out: T = c ? h : h2;
    return out;
}

func main(): i32 {
    let h: Holder = Holder { names: ["EE", "FF"] };
    let h2: Holder = Holder { names: ["GG", "HH"] };
    let g: Holder = id<Holder>(take h, take h2, true);
    return i32(g.names[0][0]) - 97;
}
"#;
    assert_rejected_once(
        "opaqueviewsorecondinit",
        conditional_init,
        "E10030",
        &["does not live long enough"],
        &[],
    );

    let temporary = r#"module spec.eei.opaqueviewsoretemporary;

import { String, stringOf } from std.text;

struct Holder { names: str[2]; }

func f<T>(take h: T): T {
    h.names[0] = stringOf("aa");
    return h;
}

func main(): i32 {
    let h: Holder = Holder { names: ["EE", "FF"] };
    let g: Holder = f<Holder>(take h);
    return i32(g.names[0][0]) - 97;
}
"#;
    assert_rejected_once(
        "opaqueviewsoretemporary",
        temporary,
        "E10030",
        &["does not live long enough"],
        &[],
    );
}

/// A stored view's backing is watched after the store.
///
/// A backing that dies at an inner scope or is rewritten reports the concrete
/// escape or shared-loan conflict. An overwrite removes the earlier duty, and
/// a sibling return does not hand the view out.
#[test]
fn should_watch_an_opaque_store_backing_after_the_store() {
    let nested = r#"module spec.eei.opaqueviewsorenested;

import { String, stringOf } from std.text;

struct Holder { names: str[2]; }
struct Box { v: String; }

func f<T>(take h: T): i32 {
    {
        let a: Box = Box { v: stringOf("aa") };
        h.names[0] = a.v;
    }
    let v: str = h.names[0];
    return i32(v[0]) - 97;
}

func main(): i32 {
    let h: Holder = Holder { names: ["EE", "FF"] };
    return f<Holder>(take h);
}
"#;
    assert_rejected_once(
        "opaqueviewsorenested",
        nested,
        "E10030",
        &["does not live long enough"],
        &[],
    );

    let reassigned = r#"module spec.eei.opaqueviewsorereassign;

import { String, stringOf } from std.text;

struct Holder { names: str[2]; }

func first(s: str): i32 { return i32(s[0]); }

func f<T>(take h: T): i32 {
    let s: String = stringOf("aa");
    h.names[0] = s;
    s = stringOf("bb");
    return first(h.names[0]) - 97;
}

func main(): i32 {
    let h: Holder = Holder { names: ["EE", "FF"] };
    return f<Holder>(take h);
}
"#;
    assert_rejected_once(
        "opaqueviewsorereassign",
        reassigned,
        "E10031",
        &["while it is shared-borrowed"],
        &[],
    );

    let two_stores = r#"module spec.eei.opaqueviewsoretwo;

import { String, stringOf } from std.text;

struct Holder { names: str[2]; }
struct Box { v: String; }

func f<T>(take h: T): T {
    let a: Box = Box { v: stringOf("aa") };
    let b: Box = Box { v: stringOf("bb") };
    let s: str = a.v;
    let t: str = b.v;
    h.names[0] = s;
    h.names[1] = t;
    return h;
}

func main(): i32 {
    let h: Holder = Holder { names: ["EE", "FF"] };
    let g: Holder = f<Holder>(take h);
    return i32(g.names[0][0]) - 97;
}
"#;
    assert_rejected_once(
        "opaqueviewsoretwo",
        two_stores,
        "E10030",
        &["does not live long enough"],
        &[],
    );

    let overwrite = r#"module spec.eei.opaqueviewsoreoverwrite;

import { String, stringOf } from std.text;

struct Holder { names: str[2]; }
struct Box { v: String; }

func f<T>(take h: T): i32 {
    let a: Box = Box { v: stringOf("aa") };
    let s: str = a.v;
    h.names[0] = s;
    h.names[0] = "OK";
    return i32(h.names[0][0]) - 79;
}

func main(): i32 {
    let h: Holder = Holder { names: ["EE", "FF"] };
    return f<Holder>(take h);
}
"#;
    let output = run_memory("opaqueviewsoreoverwrite", overwrite);
    assert_no_asan("opaqueviewsoreoverwrite", &output);
    assert_exit("opaqueviewsoreoverwrite", &output, 0);

    let sibling = r#"module spec.eei.opaqueviewsoresibling;

import { String, stringOf } from std.text;

struct H2 { a: str[2]; b: str[2]; }
struct Box { v: String; }

func f<T>(take h: T): i32 {
    let a: Box = Box { v: stringOf("aa") };
    h.a[0] = a.v;
    let g: T = h;
    return i32(g.b[0][0]) - 69;
}

func main(): i32 {
    let h: H2 = H2 { a: ["EE", "FF"], b: ["EE", "FF"] };
    return f<H2>(take h);
}
"#;
    let output = run_memory("opaqueviewsoresibling", sibling);
    assert_no_asan("opaqueviewsoresibling", &output);
    assert_exit("opaqueviewsoresibling", &output, 0);

    let sibling_store = r#"module spec.eei.opaqueviewsoresiblingstore;

import { String, stringOf } from std.text;

struct Holder { names: str[2]; }
struct Box { v: String; }

func f<T>(take h: T): T {
    let a: Box = Box { v: stringOf("aa") };
    let s: str = a.v;
    h.names[0] = s;
    h.names[1] = "OK";
    return h;
}

func main(): i32 {
    let h: Holder = Holder { names: ["EE", "FF"] };
    let g: Holder = f<Holder>(take h);
    return i32(g.names[0][0]) - 97;
}
"#;
    assert_rejected_once(
        "opaqueviewsoresiblingstore",
        sibling_store,
        "E10030",
        &["does not live long enough"],
        &[],
    );

    let lambda_store = r#"module spec.eei.opaqueviewsorelambda;

import { String, stringOf } from std.text;

struct Holder { names: str[2]; }
struct Box { v: String; }

func f<T>(take h: T): i32 {
    ([borrow h] () => {
        let a: Box = Box { v: stringOf("aa") };
        h.names[0] = a.v;
        return 0;
    })();
    let v: str = h.names[0];
    return i32(v[0]) - 97;
}

func main(): i32 {
    let h: Holder = Holder { names: ["EE", "FF"] };
    return f<Holder>(take h);
}
"#;
    assert_rejected_once(
        "opaqueviewsorelambda",
        lambda_store,
        "E10030",
        &["does not live long enough"],
        &[],
    );

    let source_reset = r#"module spec.eei.opaqueviewsoresourcewrite;

import { String, stringOf } from std.text;

struct Box { v: String; }

func f<T>(take h: T): i32 {
    let a: Box = Box { v: stringOf("aa") };
    h.v = a.v;
    a = Box { v: stringOf("bb") };
    return i32(len(a.v)) - 2;
}

func main(): i32 {
    let b: Box = Box { v: stringOf("EE") };
    return f<Box>(take b);
}
"#;
    let output = run_memory("opaqueviewsoresourcewrite", source_reset);
    assert_no_asan("opaqueviewsoresourcewrite", &output);
    assert_exit("opaqueviewsoresourcewrite", &output, 0);

    let whole_array = r#"module spec.eei.opaqueviewsorewholearray;

struct Holder { names: str[2]; }

func f<T>(take h: T, p: str[2]): i32 {
    h.names = p;
    return i32(h.names[0][0]) - 65;
}

func main(): i32 {
    let h: Holder = Holder { names: ["EE", "FF"] };
    let p: str[2] = ["AA", "BB"];
    return f<Holder>(take h, p);
}
"#;
    let output = run_memory("opaqueviewsorewholearray", whole_array);
    assert_no_asan("opaqueviewsorewholearray", &output);
    assert_exit("opaqueviewsorewholearray", &output, 0);

    let whole_root = r#"module spec.eei.opaqueviewsorewholeroot;

import { String, stringOf } from std.text;

struct Holder { names: str[2]; }
struct Box { v: String; }

func f<T>(take h: T, take h2: T): T {
    let a: Box = Box { v: stringOf("aa") };
    h.names[0] = a.v;
    h = h2;
    return h;
}

func main(): i32 {
    let h: Holder = Holder { names: ["EE", "FF"] };
    let h2: Holder = Holder { names: ["GG", "HH"] };
    let g: Holder = f<Holder>(take h, take h2);
    return i32(g.names[0][0]) - 71;
}
"#;
    let output = run_memory("opaqueviewsorewholeroot", whole_root);
    assert_no_asan("opaqueviewsorewholeroot", &output);
    assert_exit("opaqueviewsorewholeroot", &output, 0);
}

/// The opaque-store duty closes over copies, aliases, and instance kinds.
///
/// An aggregate copy carries the stored view at its own path, a self-assignment
/// keeps the target's duty, a run-time sibling store does not erase a constant
/// element's view, a re-initialized source keeps its earlier read, a null value
/// at a fixed-array member reports the null rule, and a no-source view call or
/// a superseded view stays accepted.
#[test]
fn should_close_the_opaque_store_copy_and_liveness_residue() {
    let aggregate = r#"module spec.eei.opaqueviewsoreaggregate;

import { String, stringOf } from std.text;

struct Wrap<T> { inner: T; }
struct Holder { names: str[2]; }
struct Box { v: String; }

func f<T>(take h: T): Wrap<T> {
    let a: Box = Box { v: stringOf("aa") };
    h.names[0] = a.v;
    let out: Wrap<T> = Wrap<T> { inner: h };
    return out;
}

func main(): i32 {
    let h: Holder = Holder { names: ["EE", "FF"] };
    let g: Wrap<Holder> = f<Holder>(take h);
    return i32(g.inner.names[0][0]) - 97;
}
"#;
    assert_rejected_once(
        "opaqueviewsoreaggregate",
        aggregate,
        "E10030",
        &["does not live long enough"],
        &[],
    );

    let self_assign = r#"module spec.eei.opaqueviewsoreself;

import { String, stringOf } from std.text;

struct Holder { names: str[2]; }
struct Box { v: String; }

func f<T>(take h: T): T {
    let a: Box = Box { v: stringOf("aa") };
    let s: str = a.v;
    h.names[0] = s;
    h = h;
    return h;
}

func main(): i32 {
    let h: Holder = Holder { names: ["EE", "FF"] };
    let g: Holder = f<Holder>(take h);
    return i32(g.names[0][0]) - 97;
}
"#;
    assert_rejected_once(
        "opaqueviewsoreself",
        self_assign,
        "E10030",
        &["does not live long enough"],
        &[],
    );

    let conditional_self = r#"module spec.eei.opaqueviewsoreselfcond;

import { String, stringOf } from std.text;

struct Holder { names: str[2]; }
struct Box { v: String; }

func f<T>(take h: T, take h2: T, c: bool): T {
    let a: Box = Box { v: stringOf("aa") };
    let s: str = a.v;
    h.names[0] = s;
    h = c ? h : h2;
    return h;
}

func main(): i32 {
    let h: Holder = Holder { names: ["EE", "FF"] };
    let h2: Holder = Holder { names: ["GG", "HH"] };
    let g: Holder = f<Holder>(take h, take h2, true);
    return i32(g.names[0][0]) - 97;
}
"#;
    assert_rejected_once(
        "opaqueviewsoreselfcond",
        conditional_self,
        "E10030",
        &["does not live long enough"],
        &[],
    );

    let relay_self = r#"module spec.eei.opaqueviewsoreselfrelay;

import { String, stringOf } from std.text;

struct Holder { names: str[2]; }
struct Box { v: String; }

func echo<T>(take h: T): T { return h; }

func f<T>(take h: T): T {
    let a: Box = Box { v: stringOf("aa") };
    let s: str = a.v;
    h.names[0] = s;
    h = echo<T>(take h);
    return h;
}

func main(): i32 {
    let h: Holder = Holder { names: ["EE", "FF"] };
    let g: Holder = f<Holder>(take h);
    return i32(g.names[0][0]) - 97;
}
"#;
    assert_rejected_once(
        "opaqueviewsoreselfrelay",
        relay_self,
        "E10030",
        &["does not live long enough"],
        &[],
    );

    let runtime_sibling = r#"module spec.eei.opaqueviewsoresruntime;

import { String, stringOf } from std.text;

struct Holder { names: str[2]; }
struct Box { v: String; }

func f<T>(take h: T, i: i32): T {
    let a: Box = Box { v: stringOf("aa") };
    let s: str = a.v;
    h.names[0] = s;
    h.names[i] = "OK";
    return h;
}

func main(): i32 {
    let h: Holder = Holder { names: ["EE", "FF"] };
    let g: Holder = f<Holder>(take h, 1);
    return i32(g.names[0][0]) - 97;
}
"#;
    assert_rejected_once(
        "opaqueviewsoresruntime",
        runtime_sibling,
        "E10030",
        &["does not live long enough"],
        &[],
    );

    let read_then_reinit = r#"module spec.eei.opaqueviewsoresreadreinit;

import { String, stringOf } from std.text;

struct Box { v: String; }

func f<T>(take h: T): i32 {
    let a: Box = Box { v: stringOf("aa") };
    h.v = a.v;
    let n: i32 = i32(len(a.v));
    a = Box { v: stringOf("bb") };
    return n - 2;
}

func main(): i32 {
    let b: Box = Box { v: stringOf("EE") };
    return f<Box>(take b);
}
"#;
    assert_rejected_once(
        "opaqueviewsoresreadreinit",
        read_then_reinit,
        "E10020",
        &["'a' was moved"],
        &[],
    );

    let null_member = r#"module spec.eei.opaqueviewsoresnull;

struct Holder { names: str[2]; }

func f<T>(take h: T): i32 {
    let n = null;
    h.names = n;
    return 0;
}

func main(): i32 {
    let h: Holder = Holder { names: ["EE", "FF"] };
    return f<Holder>(take h);
}
"#;
    assert_rejected_once(
        "opaqueviewsoresnull",
        null_member,
        "E10012",
        &["'null' is not allowed here"],
        &[],
    );

    let partial = r#"module spec.eei.opaqueviewsorespartial;

import { String, stringOf } from std.text;

struct Box { v: String; }

func f<T>(take h: T): i32 {
    let a: Box[2] = [Box { v: stringOf("aa") }, Box { v: stringOf("bb") }];
    h.v = a[0].v;
    return 0;
}

func main(): i32 {
    let b: Box = Box { v: stringOf("EE") };
    return f<Box>(take b);
}
"#;
    assert_rejected_once(
        "opaqueviewsorespartial",
        partial,
        "E10026",
        &["partial moves are limited"],
        &[],
    );

    let dotted_copy = r#"module spec.eei.opaqueviewsorestdotted;

import { String, stringOf } from std.text;

struct Holder { names: str[2]; }
struct Wrap<T> { inner: T; }
struct Box { v: String; }

func f<T>(take h: T): Wrap<T> {
    let a: Box = Box { v: stringOf("aa") };
    h.names[0] = a.v;
    let w: Wrap<T> = Wrap<T> { inner: h };
    w.inner = h;
    return w;
}

func main(): i32 {
    let h: Holder = Holder { names: ["EE", "FF"] };
    let g: Wrap<Holder> = f<Holder>(take h);
    return i32(g.inner.names[0][0]) - 97;
}
"#;
    assert_rejected_once(
        "opaqueviewsorestdotted",
        dotted_copy,
        "E10030",
        &["does not live long enough"],
        &[],
    );

    let constant_copy = r#"module spec.eei.opaqueviewsorestconst;

import { String, stringOf } from std.text;

struct Wrap<T> { items: T[2]; }
struct Holder { names: str[2]; }
struct Box { v: String; }

func f<T>(take w: Wrap<T>): T {
    let a: Box = Box { v: stringOf("aa") };
    w.items[0].names[0] = a.v;
    let out: T = w.items[0];
    return out;
}

func main(): i32 {
    let w: Wrap<Holder> = Wrap<Holder> {
        items: [
            Holder { names: ["EE", "FF"] },
            Holder { names: ["GG", "HH"] },
        ],
    };
    let g: Holder = f<Holder>(take w);
    return i32(g.names[0][0]) - 97;
}
"#;
    assert_rejected_once(
        "opaqueviewsorestconst",
        constant_copy,
        "E10030",
        &["does not live long enough"],
        &[],
    );

    let concrete_copy = r#"module spec.eei.opaqueviewsorestconcrete;

import { String, stringOf } from std.text;

struct Holder { names: str[2]; }
struct Box { v: String; }

func f<T>(take h: T): Holder {
    let a: Box = Box { v: stringOf("aa") };
    h.names[0] = a.v;
    let out: Holder = h;
    return out;
}

func main(): i32 {
    let h: Holder = Holder { names: ["EE", "FF"] };
    let g: Holder = f<Holder>(take h);
    return i32(g.names[0][0]) - 97;
}
"#;
    assert_rejected_once(
        "opaqueviewsorestconcrete",
        concrete_copy,
        "E10030",
        &["does not live long enough"],
        &[],
    );

    let runtime_pair = r#"module spec.eei.opaqueviewsorestruntime;

import { String, stringOf } from std.text;

struct Holder { names: str[2]; }
struct Box { v: String; }

func f<T>(take h: T, i: i32, j: i32): T {
    let a: Box = Box { v: stringOf("aa") };
    let s: str = a.v;
    h.names[i] = s;
    h.names[j] = "OK";
    return h;
}

func main(): i32 {
    let h: Holder = Holder { names: ["EE", "FF"] };
    let g: Holder = f<Holder>(take h, 0, 1);
    return i32(g.names[0][0]) - 97;
}
"#;
    assert_rejected_once(
        "opaqueviewsorestruntime",
        runtime_pair,
        "E10030",
        &["does not live long enough"],
        &[],
    );

    let borrow_slot = r#"module spec.eei.opaqueviewsorestborrow;

import { String, stringOf } from std.text;

struct Holder { names: str[2]; }
struct Box { v: String; }

func f<T>(borrow h: T): void {
    let a: Box = Box { v: stringOf("aa") };
    h.names[0] = a.v;
}

func main(): i32 {
    let h: Holder = Holder { names: ["EE", "FF"] };
    f<Holder>(borrow h);
    return i32(h.names[0][0]) - 97;
}
"#;
    assert_rejected_once(
        "opaqueviewsorestborrow",
        borrow_slot,
        "E10030",
        &["does not live long enough"],
        &[],
    );

    let null_raw = r#"module spec.eei.opaqueviewsorestnullraw;

struct Holder { p: raw u8; }

func f<T>(take h: T): i32 {
    let n = null;
    h.p = n;
    return 0;
}

func main(): i32 {
    let h: Holder = Holder { p: raw u8(0) };
    return f<Holder>(take h);
}
"#;
    assert_rejected_once(
        "opaqueviewsorestnullraw",
        null_raw,
        "E10012",
        &["'null' is not allowed here"],
        &[],
    );

    let call_view = r#"module spec.eei.opaqueviewsorescall;

struct Holder { names: str[2]; }

func mk(): str { return "aa"; }

func f<T>(take h: T): i32 {
    h.names[0] = mk();
    return i32(h.names[0][0]) - 97;
}

func main(): i32 {
    let h: Holder = Holder { names: ["EE", "FF"] };
    return f<Holder>(take h);
}
"#;
    let output = run_memory("opaqueviewsorescall", call_view);
    assert_no_asan("opaqueviewsorescall", &output);
    assert_exit("opaqueviewsorescall", &output, 0);

    let superseded_copy = r#"module spec.eei.opaqueviewsoressuperseded;

import { String, stringOf } from std.text;

struct Holder { names: str[2]; }

func f<T>(take h: T): T {
    let s: String = stringOf("aa");
    h.names[0] = s;
    h.names[0] = "OK";
    let out: T = h;
    return out;
}

func main(): i32 {
    let h: Holder = Holder { names: ["EE", "FF"] };
    let g: Holder = f<Holder>(take h);
    return i32(g.names[0][0]) - 79;
}
"#;
    let output = run_memory("opaqueviewsoressuperseded", superseded_copy);
    assert_no_asan("opaqueviewsoressuperseded", &output);
    assert_exit("opaqueviewsoressuperseded", &output, 0);
}

/// The instance's member kind also decides the store's move and widening.
///
/// A view-typed member accepts an owned source as a widening and leaves the
/// source readable. An owned member keeps the concrete move: a by-value
/// parameter cannot be moved, and a later read of a moved local is the moved
/// value error.
#[test]
fn should_apply_the_instance_move_or_widening_to_an_opaque_store() {
    let param_view = r#"module spec.eei.opaqueviewsoreparam;

import { String, stringOf } from std.text;

struct Holder { names: str[2]; }

func id<T>(take h: T, s: String): T {
    h.names[0] = s;
    return h;
}

func main(): i32 {
    let h: Holder = Holder { names: ["EE", "FF"] };
    let s: String = stringOf("aa");
    let g: Holder = id<Holder>(take h, s);
    return i32(g.names[0][0]) - 97;
}
"#;
    let output = run_memory("opaqueviewsoreparam", param_view);
    assert_no_asan("opaqueviewsoreparam", &output);
    assert_exit("opaqueviewsoreparam", &output, 0);

    let param_owned = r#"module spec.eei.opaqueviewsoreparamowned;

import { String, stringOf } from std.text;

struct Box { v: String; }

func id<T>(take h: T, s: String): T {
    h.v = s;
    return h;
}

func main(): i32 {
    let b: Box = Box { v: stringOf("EE") };
    let s: String = stringOf("aa");
    let g: Box = id<Box>(take b, s);
    return i32(len(g.v)) - 2;
}
"#;
    assert_rejected_once(
        "opaqueviewsoreparamowned",
        param_owned,
        "E10034",
        &["borrowed parameter 's'"],
        &[],
    );

    let owned_later = r#"module spec.eei.opaqueviewsoreownedlater;

import { String, stringOf } from std.text;

struct Box { v: String; }

func f<T>(take h: T): i32 {
    let a: Box = Box { v: stringOf("aa") };
    h.v = a.v;
    return i32(len(a.v)) - 2;
}

func main(): i32 {
    let b: Box = Box { v: stringOf("EE") };
    return f<Box>(take b);
}
"#;
    assert_rejected_once(
        "opaqueviewsoreownedlater",
        owned_later,
        "E10020",
        &["'a' was moved"],
        &[],
    );

    let wrapper_view = r#"module spec.eei.opaqueviewsorewrap;

import { String, stringOf } from std.text;

struct Wrap<T> { items: T[2]; }
struct Box { v: String; }

func f<T>(take w: Wrap<T>): i32 {
    let a: Box = Box { v: stringOf("aa") };
    w.items[0] = a.v;
    return i32(w.items[0][0]) - 97;
}

func main(): i32 {
    let w: Wrap<str> = Wrap<str> { items: ["EE", "FF"] };
    return f<str>(take w);
}
"#;
    let output = run_memory("opaqueviewsorewrap", wrapper_view);
    assert_no_asan("opaqueviewsorewrap", &output);
    assert_exit("opaqueviewsorewrap", &output, 0);

    let source_read = r#"module spec.eei.opaqueviewsoresource;

import { String, stringOf } from std.text;

struct Holder { names: str[2]; }
struct Box { v: String; }

func f<T>(take h: T): i32 {
    let a: Box = Box { v: stringOf("aa") };
    h.names[0] = a.v;
    return i32(len(a.v)) - 2;
}

func main(): i32 {
    let h: Holder = Holder { names: ["EE", "FF"] };
    return f<Holder>(take h);
}
"#;
    let output = run_memory("opaqueviewsoresource", source_read);
    assert_no_asan("opaqueviewsoresource", &output);
    assert_exit("opaqueviewsoresource", &output, 0);

    let wrapper_owned = r#"module spec.eei.opaqueviewsorewrapowned;

import { String, stringOf } from std.text;

struct Wrap<T> { items: T[2]; }
struct Box { v: String; }

func f<T>(take w: Wrap<T>): i32 {
    let a: Box = Box { v: stringOf("aa") };
    w.items[0] = a.v;
    return i32(len(w.items[0])) - 2;
}

func main(): i32 {
    let w: Wrap<String> = Wrap<String> {
        items: [stringOf("EE"), stringOf("FF")],
    };
    return f<String>(take w);
}
"#;
    let output = run_memory("opaqueviewsorewrapowned", wrapper_owned);
    assert_no_asan("opaqueviewsorewrapowned", &output);
    assert_exit("opaqueviewsorewrapowned", &output, 0);
}

/// A carrier call nested in a returned struct literal is judged at the return.
///
/// The selected field is a carrier whose view reads the returned frame's local
/// `a`, so the return reports `E10030` once. The all-static literal stays
/// accepted and clean.
#[test]
fn should_reject_a_carrier_call_in_a_returned_struct_literal() {
    let source = r#"module spec.eei.literalcarrierreturn;

import { String, stringOf } from std.text;

struct Holder { names: str[2]; }
struct Box { v: String; }
struct S { h: Holder; }

func makeFrom(b: Box): Holder { return Holder { names: [b.v, "EE"] }; }

func outer(c: bool): S {
    let a: Box = Box { v: stringOf("aa") };
    return S { h: c ? makeFrom(a) : Holder { names: ["CC", "DD"] } };
}

func main(): i32 {
    let s: S = outer(true);
    return i32(s.h.names[0][0]) - 97;
}
"#;
    assert_rejected_once(
        "literalcarrierreturn",
        source,
        "E10030",
        &["does not live long enough"],
        &[],
    );

    let control = r#"module spec.eei.literalcarrierreturnok;

struct Holder { names: str[2]; }
struct S { h: Holder; }

func outer(c: bool): S {
    return S { h: c ? Holder { names: ["AA", "BB"] } : Holder { names: ["CC", "DD"] } };
}

func main(): i32 {
    let s: S = outer(false);
    return i32(s.h.names[0][0]) - 67;
}
"#;
    let output = run_memory("literalcarrierreturnok", control);
    assert_no_asan("literalcarrierreturnok", &output);
    assert_exit("literalcarrierreturnok", &output, 0);

    // The parameter-backed arm must not hide the carrier call in the other
    // arm: the carrier still reads the local `a`.
    let parameter_arm = r#"module spec.eei.literalcarrierparam;

import { String, stringOf } from std.text;

struct Holder { names: str[2]; }
struct Box { v: String; }
struct S { h: Holder; }

func makeFrom(b: Box): Holder { return Holder { names: [b.v, "EE"] }; }

func outer(c: bool, p: S): S {
    let a: Box = Box { v: stringOf("aa") };
    return S { h: c ? makeFrom(a) : p.h };
}

func main(): i32 {
    let p: S = S { h: Holder { names: ["CC", "DD"] } };
    let s: S = outer(true, p);
    return i32(s.h.names[0][0]) - 97;
}
"#;
    assert_rejected_once(
        "literalcarrierparam",
        parameter_arm,
        "E10030",
        &["does not live long enough"],
        &[],
    );
}

/// An inline lambda callee resolves its own signature in the source walks.
///
/// The lambda returns its `str` argument, so the store hands the caller's slot
/// a view of the frame-local `a`; the store reports `E10030` once. The
/// parameter-backed control stays accepted and clean.
#[test]
fn should_reject_an_inline_lambda_view_escape() {
    let source = r#"module spec.eei.lambdaview;

import { String, stringOf } from std.text;

struct Box { v: String; }

func store(s: str, borrow slot: str?): void { slot = s; }

func outer(borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa") };
    let s: str = a.v;
    store(((ys: str, j: i32) => ys)(s, 0), borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;
    assert_rejected_once(
        "lambdaview",
        source,
        "E10034",
        &["cannot release or store borrowed parameter 's'"],
        &["E10030"],
    );

    let control = r#"module spec.eei.lambdaviewok;

func store(s: str, borrow slot: str?): void { slot = s; }

func outer(p: str, borrow slot: str?): void {
    store(((ys: str, j: i32) => ys)(p, 0), borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer("AA", borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 65;
}
"#;
    assert_rejected_once(
        "lambdaviewok",
        control,
        "E10034",
        &["cannot release or store borrowed parameter 's'"],
        &["E10030"],
    );
    // A capturing lambda's returned view reads the capture's backing, not a
    // parameter, so the escaping store reports the frame-local source.
    let captured = r#"module spec.eei.lambdacaptureview;

import { String, stringOf } from std.text;

struct Box { v: String; }

func store(s: str, borrow slot: str?): void { slot = s; }

func outer(borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa") };
    let s: str = a.v;
    store(([s] () => s)(), borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;
    assert_rejected_once(
        "lambdacaptureview",
        captured,
        "E10034",
        &["cannot release or store borrowed parameter 's'"],
        &["E10030"],
    );

    // A write through a captured local stays in the same frame, so the view
    // does not escape.
    // A lambda with two view parameters still resolves the one its return
    // names.
    let two_view = r#"module spec.eei.lambdatwoview;

import { String, stringOf } from std.text;

struct Box { v: String; }

func store(s: str, borrow slot: str?): void { slot = s; }

func outer(z: str, borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa") };
    let av: str = a.v;
    store(((ys: str, zs: str) => zs)(z, av), borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer("ZZ", borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;
    assert_rejected_once(
        "lambdatwoview",
        two_view,
        "E10034",
        &["cannot release or store borrowed parameter 's'"],
        &["E10030"],
    );

    let two_view_control = r#"module spec.eei.lambdatwoviewok;

func store(s: str, borrow slot: str?): void { slot = s; }

func outer(p: str, q: str, borrow slot: str?): void {
    store(((ys: str, zs: str) => ys)(p, q), borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer("AA", "ZZ", borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 65;
}
"#;
    assert_rejected_once(
        "lambdatwoviewok",
        two_view_control,
        "E10034",
        &["cannot release or store borrowed parameter 's'"],
        &["E10030"],
    );
    // A conditional return over two view parameters judges every value
    // source, not just the first.
    let mixed_view = r#"module spec.eei.lambdamixedview;

import { String, stringOf } from std.text;

struct Box { v: String; }

func store(s: str, borrow slot: str?): void { slot = s; }

func outer(c: bool, p: str, borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa") };
    let av: str = a.v;
    store(([c] (ys: str, zs: str) => (c ? ys : zs))(p, av), borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(false, "ZZ", borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;
    assert_rejected_once(
        "lambdamixedview",
        mixed_view,
        "E10034",
        &["cannot release or store borrowed parameter 's'"],
        &["E10030"],
    );

    // A conditional return over two view parameters and a multi-capture
    // return judge every value source, not the first one.
    let mixed_params = r#"module spec.eei.lambdamixedparams;

import { String, stringOf } from std.text;

struct Box { v: String; }

func store(s: str, borrow slot: str?): void { slot = s; }

func outer(p: str, c: bool, borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa") };
    let av: str = a.v;
    store(([c] (ys: str, zs: str) => c ? ys : zs)(p, av, c), borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer("PP", false, borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;
    assert_rejected_once(
        "lambdamixedparams",
        mixed_params,
        "E10034",
        &["cannot release or store borrowed parameter 's'"],
        &["E10030"],
    );

    let capture_order = r#"module spec.eei.lambdacaptureorder;

import { String, stringOf } from std.text;

struct Box { v: String; }

func store(s: str, borrow slot: str?): void { slot = s; }

func outer(p: str, c: bool, borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa") };
    let s: str = a.v;
    store(([p, s, c] () => c ? p : s)(), borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer("PP", false, borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;
    assert_rejected_once(
        "lambdacaptureorder",
        capture_order,
        "E10034",
        &["cannot release or store borrowed parameter 's'"],
        &["E10030"],
    );

    let conditional_return = r#"module spec.eei.lambdacondreturn;

import { String, stringOf } from std.text;

struct Box { v: String; }

func store(s: str, borrow slot: str?): void { slot = s; }

func outer(borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa") };
    let av: str = a.v;
    store(((ys: str, zs: str) => true ? ys : zs)(av, "ZZ"), borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;
    assert_rejected_once(
        "lambdacondreturn",
        conditional_return,
        "E10034",
        &["cannot release or store borrowed parameter 's'"],
        &["E10030"],
    );

    let capture_param = r#"module spec.eei.lambdacaptureparam;

import { String, stringOf } from std.text;

struct Box { v: String; }

func store(s: str, borrow slot: str?): void { slot = s; }

func outer(p: str, borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa") };
    let av: str = a.v;
    store(([p] (ys: str) => false ? p : ys)(av), borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer("PP", borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;
    assert_rejected_once(
        "lambdacaptureparam",
        capture_param,
        "E10034",
        &["cannot release or store borrowed parameter 's'"],
        &["E10030"],
    );

    let local_destination = r#"module spec.eei.lambdalocaldest;

import { String, stringOf } from std.text;

struct Box { v: String; }

func outer(): i32 {
    let a: Box = Box { v: stringOf("aa") };
    let r: str = "";
    ([a, borrow r] () => { r = a.v; })();
    return i32(r[0]) - 97;
}

func main(): i32 { return outer(); }
"#;
    let output = run_memory("lambdalocaldest", local_destination);
    assert_no_asan("lambdalocaldest", &output);
    assert_exit("lambdalocaldest", &output, 0);
}

/// A marker that contradicts a value callee's signature reports `E10039`.
#[test]
fn should_validate_a_marker_on_a_value_callee() {
    let source = r#"module spec.eei.valuemarker;

func outer(s: str): i32 {
    return i32(((ys: str) => ys)(take s)[0]) - 65;
}

func main(): i32 { return outer("AA"); }
"#;
    assert_rejected_once(
        "valuemarker",
        source,
        "E10039",
        &["does not match the declaration"],
        &[],
    );
}

/// A conditional over func values resolves the common signature and lowers
/// through the selected callable value.
///
/// The escaping call reports `E10030` once. The same call used inside the frame
/// stays accepted, emits valid C that strict-builds, and runs.
#[test]
fn should_resolve_a_conditional_func_value_call() {
    let source = r#"module spec.eei.condfuncview;

import { String, stringOf } from std.text;

struct Box { v: String; }

func first(s: str, j: i32): str { return s; }
func second(s: str, j: i32): str { return s; }
func store(s: str, borrow slot: str?): void { slot = s; }

func outer(c: bool, borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa") };
    let s: str = a.v;
    store((c ? first : second)(s, 0), borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer(true, borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;
    assert_rejected_once(
        "condfuncview",
        source,
        "E10034",
        &["cannot release or store borrowed parameter 's'"],
        &["E10030"],
    );

    let control = r#"module spec.eei.condfuncviewok;

func first(s: str, j: i32): str { return s; }
func second(s: str, j: i32): str { return s; }

func outer(c: bool, p: str): i32 {
    let s: str = (c ? first : second)(p, 0);
    return i32(s[0]) - 65;
}

func main(): i32 {
    return outer(true, "AA");
}
"#;
    let output = run_host("condfuncviewok", control);
    assert_exit("condfuncviewok", &output, 0);
}

/// A by-value capture that backs a view stored into an out-slot reports the
/// owning escape.
///
/// The closure environment dies when the lambda call ends, so the caller's
/// slot would hold a view of freed storage; the store reports `E10030` once.
/// A parameter-backed capture survives the call and stays accepted and clean.
#[test]
fn should_reject_a_capture_backed_out_slot_view() {
    let source = r#"module spec.eei.captureoutslotview;

import { String, stringOf } from std.text;

struct Box { v: String; }

func outer(borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa") };
    ([a, borrow slot] () => {
        slot = a.v;
    })();
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;
    assert_rejected_once(
        "captureoutslotview",
        source,
        "E10030",
        &["does not live long enough"],
        &[],
    );

    let control = r#"module spec.eei.captureoutslotviewok;

func outer(p: str, borrow slot: str?): void {
    ([p, borrow slot] () => {
        slot = p;
    })();
}

func main(): i32 {
    let out: str? = null;
    outer("AA", borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 65;
}
"#;
    assert_rejected_once(
        "captureoutslotviewok",
        control,
        "E10034",
        &["cannot release or store borrowed parameter 'p'"],
        &["E10030"],
    );
    // A local copy of a parameter-backed view keeps the caller's backing, so
    // the capture survives the call exactly as the parameter does.
    let local_view = r#"module spec.eei.captureoutslotviewlocal;

func outer(p: str, borrow slot: str?): void {
    let s: str = p;
    ([s, borrow slot] () => {
        slot = s;
    })();
}

func main(): i32 {
    let out: str? = null;
    outer("AA", borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 65;
}
"#;
    assert_rejected_once(
        "captureoutslotviewlocal",
        local_view,
        "E10034",
        &["cannot release or store borrowed parameter 's'"],
        &["E10030"],
    );
    let static_view = r#"module spec.eei.captureoutslotviewstatic;

func outer(p: static str, borrow slot: str?): void {
    ([p, borrow slot] () => {
        slot = p;
    })();
}

func main(): i32 {
    let out: str? = null;
    outer("AA", borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 65;
}
"#;
    let output = run_memory("captureoutslotviewstatic", static_view);
    assert_no_asan("captureoutslotviewstatic", &output);
    assert_exit("captureoutslotviewstatic", &output, 0);

    // A view written into a captured local and then handed to the caller is
    // still an escape.
    let destination_escape = r#"module spec.eei.capturedestination;

import { String, stringOf } from std.text;

struct Box { v: String; }

func outer(borrow slot: str?): void {
    let a: Box = Box { v: stringOf("aa") };
    let r: str = "";
    ([a, borrow r] () => { r = a.v; })();
    slot = r;
}

func main(): i32 {
    let out: str? = null;
    outer(borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 97;
}
"#;
    assert_rejected_once(
        "capturedestination",
        destination_escape,
        "E10030",
        &["does not live long enough"],
        &[],
    );

    let returned_local = r#"module spec.eei.capturedreturnlocal;

import { String, stringOf } from std.text;

struct Holder { names: str[2]; }
struct Box { v: String; }

func outer(): Holder {
    let a: Box = Box { v: stringOf("aa") };
    let slot: Holder = Holder { names: ["AA", "EE"] };
    ([a, borrow slot] () => { slot.names[0] = a.v; })();
    return slot;
}

func main(): i32 {
    let g: Holder = outer();
    return i32(g.names[0][0]) - 97;
}
"#;
    assert_rejected_once(
        "capturedreturnlocal",
        returned_local,
        "E10030",
        &["does not live long enough"],
        &[],
    );

    let returned_take = r#"module spec.eei.capturedreturntake;

import { String, stringOf } from std.text;

struct Holder { names: str[2]; }

func outer(take h: Holder): Holder {
    let local: String = stringOf("zz");
    let av: str = local;
    ([av, borrow h] () => { h.names[0] = av; })();
    return h;
}

func main(): i32 {
    let h: Holder = Holder { names: ["AA", "EE"] };
    let g: Holder = outer(take h);
    return i32(g.names[0][0]) - 97;
}
"#;
    assert_rejected_once(
        "capturedreturntake",
        returned_take,
        "E10030",
        &["does not live long enough"],
        &[],
    );
}

/// A slice whose element is a `func` type emits a valid C typedef.
///
/// The element is itself a function pointer, so the slice's pointer names a
/// pointer to that function pointer. The accepted program indexes the slice and
/// calls the selected function.
#[test]
fn should_emit_a_slice_of_func_typedef() {
    let source = r#"module spec.eei.slicefunc;

import { slice } from std.mem;

func inc(x: i32): i32 { return x + 1; }
func twice(x: i32): i32 { return x * 2; }

func main(): i32 {
    let a: (func(i32): i32)[2] = [inc, twice];
    let v: (func(i32): i32)[] = slice(a, 0, 2);
    return v[0](20) - 21;
}
"#;
    let output = run_memory("slicefunc", source);
    assert_no_asan("slicefunc", &output);
    assert_exit("slicefunc", &output, 0);
}

/// A parameter-backed view written through a captured local survives.
///
/// The closure stores the captured parameter view into the enclosing local,
/// so the local holds the caller's storage. A later store to the caller's slot
/// is accepted, while the local-owner twin (the batch-Q captured-destination
/// case) still reports the escape.
#[test]
fn should_accept_a_parameter_backed_capture_write() {
    let source = r#"module spec.eei.captureparamsafe;

func store(s: str, borrow slot: str?): void { slot = s; }

func outer(p: str, borrow slot: str?): void {
    let r: str = "";
    ([p, borrow r] () => { r = p; })();
    store(r, borrow slot);
}

func main(): i32 {
    let out: str? = null;
    outer("AA", borrow out);
    if (out == null) { return 9; }
    return i32(out[0]) - 65;
}
"#;
    assert_rejected_once(
        "captureparamsafe",
        source,
        "E10034",
        &["cannot release or store borrowed parameter 's'"],
        &["E10030"],
    );}
