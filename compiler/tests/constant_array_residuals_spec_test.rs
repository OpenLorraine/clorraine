//! Behaviour oracle for function-local constant scalar array initializers.
//!
//! A complete explicit list of guard-free constants in a local fixed array
//! may copy from one read-only template instead of one store per element. The
//! visible result must not change: every element is written on each execution
//! of the declaration, the writable local keeps its address, and every shape
//! outside the narrow rule keeps the store form.
//!
//! Every case compiles a program, builds it on the host with a strict C
//! compiler, runs it, and compares stdout. The emitted-shape pins live in the
//! implementation suite.
//!
//! The suite opts into both sanitizer legs. `RDC_ASAN` adds AddressSanitizer
//! to every child build; `RDC_UBSAN` adds UndefinedBehaviorSanitizer.

const _ASAN_SUITE: &str = "RDC_ASAN";
const _UBSAN_SUITE: &str = "RDC_UBSAN";

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

struct ScratchDir {
    path: PathBuf,
}

impl ScratchDir {
    fn new(tag: &str) -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "clc-constant-array-residual-spec-{}-{tag}-{sequence}",
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

fn clc_path() -> String {
    match std::env::var("RDC_CLC") {
        Ok(value) => {
            let path = PathBuf::from(&value);
            if path.is_absolute() {
                value
            } else {
                Path::new(env!("CARGO_MANIFEST_DIR"))
                    .parent()
                    .expect("the workspace root")
                    .join(path)
                    .display()
                    .to_string()
            }
        }
        Err(_) => env!("CARGO_BIN_EXE_clc").to_owned(),
    }
}

fn run_clc(args: &[String]) -> Output {
    Command::new(clc_path())
        .args(args)
        .output()
        .expect("failed to spawn the compiler binary")
}

fn emit(tag: &str, source: &str) -> (ScratchDir, String) {
    let scratch = ScratchDir::new(tag);
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, source)
        .unwrap_or_else(|error| panic!("{tag}: write main.clo: {error}"));
    let output_path = scratch.path().join("out.c");
    let output = run_clc(&[
        "--profile".to_owned(),
        "host".to_owned(),
        "-o".to_owned(),
        output_path.display().to_string(),
        source_path.display().to_string(),
    ]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{tag}: the front end rejected the program; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let emitted = std::fs::read_to_string(&output_path).expect("read the emitted C");
    (scratch, emitted)
}

fn build_host(tag: &str, scratch: &ScratchDir, emitted: &str) -> PathBuf {
    let c_path = scratch.path().join("build.c");
    std::fs::write(&c_path, emitted).expect("write the emitted C");
    let program = scratch.path().join("prog");
    let mut command = Command::new("cc");
    command.args(["-std=c99", "-O0", "-g", "-Wall", "-Wextra", "-Werror"]);
    if std::env::var("RDC_ASAN").is_ok() {
        command.args(["-fsanitize=address", "-fno-omit-frame-pointer"]);
    }
    if std::env::var("RDC_UBSAN").is_ok() {
        command.args(["-fsanitize=undefined", "-fno-sanitize-recover=all"]);
    }
    command.args([
        "-o",
        program.to_str().expect("program path"),
        c_path.to_str().expect("C path"),
    ]);
    let built = command.output().expect("spawn cc");
    assert!(
        built.status.success(),
        "{tag}: the host C compiler rejected the emitted C; stderr:\n{}",
        String::from_utf8_lossy(&built.stderr)
    );
    program
}

fn assert_output(tag: &str, source: &str, expected: &str) {
    let (scratch, emitted) = emit(tag, source);
    let program = build_host(tag, &scratch, &emitted);
    let output = Command::new(&program).output().expect("run the program");
    assert_eq!(
        output.status.code(),
        Some(0),
        "{tag}: exit; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        expected,
        "{tag}: unexpected stdout"
    );
}

fn assert_compile_fail(tag: &str, source: &str, code: &str) {
    let scratch = ScratchDir::new(tag);
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, source)
        .unwrap_or_else(|error| panic!("{tag}: write main.clo: {error}"));
    let output_path = scratch.path().join("out.c");
    let output = run_clc(&[
        "--profile".to_owned(),
        "host".to_owned(),
        "-o".to_owned(),
        output_path.display().to_string(),
        source_path.display().to_string(),
    ]);
    assert_ne!(
        output.status.code(),
        Some(0),
        "{tag}: the compiler accepted the shape"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(&format!("error {code}:")),
        "{tag}: expected {code}; stderr:\n{stderr}"
    );
    assert!(!output_path.is_file(), "{tag}: a rejected shape wrote C");
}

fn assert_fault(tag: &str, source: &str, code: &str) {
    let (scratch, emitted) = emit(tag, source);
    let program = build_host(tag, &scratch, &emitted);
    let output = Command::new(&program).output().expect("run the program");
    assert_ne!(
        output.status.code(),
        Some(0),
        "{tag}: the program did not fault"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(&format!("error {code}:")),
        "{tag}: expected {code}; stderr:\n{stderr}"
    );
}

fn seq(start: i32, count: usize) -> String {
    (0..count)
        .map(|offset| (start + offset as i32).to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

#[test]
fn should_run_a_function_local_i32_table() {
    assert_output(
        "local-i32",
        &format!(
            r#"
module spec.ConstArrayLocalI32;
import {{ println }} from std.io;
func total(): i32 {{
    let t: i32[32] = [{list}];
    let sum: i32 = 0;
    let i: i32 = 0;
    while (i < 32) {{ sum = sum + t[i]; i = i + 1; }}
    return sum;
}}
func main(): i32 {{
    println($"sum: {{total()}}");
    return 0;
}}
"#,
            list = seq(1, 32)
        ),
        "sum: 528\n",
    );
}

#[test]
fn should_run_a_function_local_u16_table() {
    assert_output(
        "local-u16",
        &format!(
            r#"
module spec.ConstArrayLocalU16;
import {{ println }} from std.io;
func total(): i32 {{
    let t: u16[32] = [{list}];
    let sum: i32 = 0;
    let i: i32 = 0;
    while (i < 32) {{ sum = sum + i32(t[i]); i = i + 1; }}
    return sum;
}}
func main(): i32 {{
    println($"sum: {{total()}}");
    return 0;
}}
"#,
            list = seq(1, 32)
        ),
        "sum: 528\n",
    );
}

#[test]
fn should_run_a_function_local_usize_table() {
    assert_output(
        "local-usize",
        &format!(
            r#"
module spec.ConstArrayLocalUsize;
import {{ println }} from std.io;
func total(): usize {{
    let t: usize[32] = [{list}];
    let sum: usize = 0;
    let i: usize = 0;
    while (i < 32) {{ sum = sum + t[i]; i = i + 1; }}
    return sum;
}}
func main(): i32 {{
    println($"sum: {{total()}}");
    return 0;
}}
"#,
            list = seq(1, 32)
        ),
        "sum: 528\n",
    );
}

#[test]
fn should_reinitialize_a_local_table_on_each_loop_entry() {
    assert_output(
        "local-loop",
        &format!(
            r#"
module spec.ConstArrayLocalLoop;
import {{ println }} from std.io;
func main(): i32 {{
    let round: i32 = 0;
    while (round < 2) {{
        let t: i32[32] = [{list}];
        println($"round {{round}}: first {{t[0]}} last {{t[31]}}");
        t[31] = 999;
        round = round + 1;
    }}
    return 0;
}}
"#,
            list = seq(1, 32)
        ),
        "round 0: first 1 last 32\nround 1: first 1 last 32\n",
    );
}

#[test]
fn should_run_a_later_local_table_assignment() {
    assert_output(
        "local-assign",
        &format!(
            r#"
module spec.ConstArrayLocalAssign;
import {{ println }} from std.io;
func total(): i32 {{
    let t: i32[32];
    t = [{list}];
    let sum: i32 = 0;
    let i: i32 = 0;
    while (i < 32) {{ sum = sum + t[i]; i = i + 1; }}
    return sum;
}}
func main(): i32 {{
    println($"sum: {{total()}}");
    return 0;
}}
"#,
            list = seq(1, 32)
        ),
        "sum: 528\n",
    );
}

#[test]
fn should_run_a_struct_field_local_table_assignment() {
    assert_output(
        "local-field",
        &format!(
            r#"
module spec.ConstArrayLocalField;
import {{ println }} from std.io;
struct Holder {{ values: i32[32]; }}
func total(): i32 {{
    let h: Holder = Holder {{ values: [; 0] }};
    h.values = [{list}];
    let sum: i32 = 0;
    let i: i32 = 0;
    while (i < 32) {{ sum = sum + h.values[i]; i = i + 1; }}
    return sum;
}}
func main(): i32 {{
    println($"sum: {{total()}}");
    return 0;
}}
"#,
            list = seq(1, 32)
        ),
        "sum: 528\n",
    );
}

#[test]
fn should_keep_the_zero_tail_of_a_short_local_list() {
    assert_output(
        "local-short",
        &format!(
            r#"
module spec.ConstArrayLocalShort;
import {{ println }} from std.io;
func main(): i32 {{
    let t: i32[32] = [{list}];
    println($"tail: {{t[31]}}");
    return 0;
}}
"#,
            list = seq(1, 16)
        ),
        "tail: 0\n",
    );
}

#[test]
fn should_run_a_function_local_u8_table() {
    assert_output(
        "local-u8",
        &format!(
            r#"
module spec.ConstArrayLocalU8;
import {{ println }} from std.io;
func total(): i32 {{
    let t: u8[64] = [{list}];
    let sum: i32 = 0;
    let i: i32 = 0;
    while (i < 64) {{ sum = sum + i32(t[i]); i = i + 1; }}
    return sum;
}}
func main(): i32 {{
    println($"sum: {{total()}}");
    return 0;
}}
"#,
            list = (1..=64).map(|value| value.to_string()).collect::<Vec<_>>().join(", ")
        ),
        "sum: 2080\n",
    );
}

#[test]
fn should_evaluate_a_local_call_element_once() {
    assert_output(
        "local-call",
        r#"
module spec.ConstArrayLocalCall;
import { println } from std.io;
let calls: i32 = 0;
func next(): i32 {
    calls = calls + 1;
    return calls;
}
func main(): i32 {
    let t: i32[32] = [next(), 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32];
    println($"calls: {calls} first: {t[0]}");
    return 0;
}
"#,
        "calls: 1 first: 1\n",
    );
}

#[test]
fn should_reject_an_out_of_range_local_literal() {
    assert_compile_fail(
        "local-oor",
        r#"
module spec.ConstArrayLocalOor;
func main(): i32 {
    let t: i32[4] = [1, 2, 3, 4, 5];
    return t[0];
}
"#,
        "E10215",
    );
}

#[test]
fn should_not_shadow_a_user_local_template_name() {
    assert_output(
        "local-shadow",
        &format!(
            r#"
module spec.ConstArrayLocalShadow;
import {{ println }} from std.io;
func main(): i32 {{
    let __cl_ctmpl_1: i32 = 5;
    let t: i32[32] = [{list}];
    println($"shadow: {{__cl_ctmpl_1}} first: {{t[0]}}");
    return 0;
}}
"#,
            list = seq(1, 32)
        ),
        "shadow: 5 first: 1\n",
    );
}

#[test]
fn should_trap_a_runtime_indexed_member_out_of_range() {
    assert_fault(
        "local-member-oob",
        &format!(
            r#"
module spec.ConstArrayLocalMemberOob;
import {{ println }} from std.io;
struct Row {{ values: i32[32]; tag: i32; }}
struct Grid {{ rows: Row[2]; }}
func main(): i32 {{
    let g: Grid = Grid {{ rows: [Row {{ values: [; 0], tag: 10 }}, Row {{ values: [; 0], tag: 20 }}] }};
    let i: i32 = 2;
    g.rows[i].values = [{list}];
    println($"tag {{g.rows[0].tag}}");
    return 0;
}}
"#,
            list = seq(1, 32)
        ),
        "E11211",
    );
}

#[test]
fn should_run_an_adjusted_array_parameter_table() {
    assert_output(
        "local-param",
        &format!(
            r#"
module spec.ConstArrayLocalParam;
import {{ println }} from std.io;
func fill(borrow a: i32[32]): void {{
    a = [{list}];
}}
func main(): i32 {{
    let t: i32[32] = [; 0];
    fill(t);
    let total: i32 = 0;
    let i: i32 = 0;
    while (i < 32) {{ total = total + t[i]; i = i + 1; }}
    println($"param: {{total}}");
    return 0;
}}
"#,
            list = seq(1, 32)
        ),
        "param: 528\n",
    );
}
