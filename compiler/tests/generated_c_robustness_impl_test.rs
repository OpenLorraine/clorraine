//! Implementation tests for the generated-C robustness bundle.
//!
//! The behavior oracle is `generated_c_robustness_spec_test.rs`. These cases
//! pin the qualified module-path emission, the owned text argument
//! materialization, the initializer intrinsic, and the init-local reservation.

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
            "clc-generated-robustness-impl-{}-{tag}-{sequence}",
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

/// The compiler under test; a relative `RDC_CLC` resolves against the
/// workspace root because Cargo runs integration tests from `compiler/`.
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

/// Emits one translation unit for a profile.
fn emitted_profile(tag: &str, profile: &str, source: &str) -> String {
    let scratch = ScratchDir::new(tag);
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, source)
        .unwrap_or_else(|error| panic!("{tag}: write main.clo: {error}"));
    let output_path = scratch.path().join("out.c");
    let output = run_clc(&[
        "--profile".to_owned(),
        profile.to_owned(),
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
    std::fs::read_to_string(&output_path).expect("read the emitted C")
}

/// Emits a program and its declaring module as one build.
fn emitted_pair(tag: &str, holder: &str, main: &str) -> String {
    let scratch = ScratchDir::new(tag);
    let holder_path = scratch.path().join("holder.clo");
    let main_path = scratch.path().join("main.clo");
    std::fs::write(&holder_path, holder)
        .unwrap_or_else(|error| panic!("{tag}: write holder.clo: {error}"));
    std::fs::write(&main_path, main)
        .unwrap_or_else(|error| panic!("{tag}: write main.clo: {error}"));
    let output_path = scratch.path().join("out.c");
    let output = run_clc(&[
        "--profile".to_owned(),
        "host".to_owned(),
        "-o".to_owned(),
        output_path.display().to_string(),
        main_path.display().to_string(),
        holder_path.display().to_string(),
    ]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{tag}: the front end rejected the pair; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    std::fs::read_to_string(&output_path).expect("read the emitted C")
}

/// Compiles one emitted translation unit to an object under strict flags.
fn build_object_strict(tag: &str, c: &str) {
    let scratch = ScratchDir::new(tag);
    let c_path = scratch.path().join("strict.c");
    std::fs::write(&c_path, c).expect("write the emitted C");
    let object = scratch.path().join("strict.o");
    let built = Command::new("cc")
        .args(["-std=c99", "-O0", "-g", "-Wall", "-Wextra", "-Werror", "-c"])
        .args([
            "-o",
            object.to_str().expect("object path"),
            c_path.to_str().expect("C path"),
        ])
        .output()
        .expect("spawn cc");
    assert!(
        built.status.success(),
        "{tag}: the host C compiler rejected the emitted C; stderr:\n{}",
        String::from_utf8_lossy(&built.stderr)
    );
}

/// IT-1: a module-level `std` value uses the `std.fmt.text` module path.
#[test]
fn it1_module_level_std_emits_the_intrinsic() {
    let source = r#"module spec.gcr.it1;
import { println } from std.io;
import { String, stringOf } from std.text;
struct Fmt { text: func(i32): String; }
struct Std { fmt: Fmt; }
func userText(n: i32): String { return stringOf("u"); }
let std: Std = Std { fmt: Fmt { text: userText } };
func main(): i32 {
    println(std.fmt.text(7));
    return 0;
}
"#;
    let c = emitted_profile("modulehead", "host", source);
    assert!(c.contains("cl_text_i32((i32)(7))"), "{c}");
    assert!(!c.contains("std.fmt.text(7)"), "{c}");
}

/// IT-2: a lexical `std` value keeps the member call.
#[test]
fn it2_lexical_std_emits_the_member_call() {
    let source = r#"module spec.gcr.it2;
import { println } from std.io;
import { String, stringOf } from std.text;
struct Fmt { text: func(i32): String; }
struct Std { fmt: Fmt; }
func userText(n: i32): String { return stringOf("u"); }
func main(): i32 {
    let std: Std = Std { fmt: Fmt { text: userText } };
    println(std.fmt.text(7));
    return 0;
}
"#;
    let c = emitted_profile("lexical", "host", source);
    assert!(c.contains("(std.fmt.text)(7)"), "{c}");
    assert!(!c.contains("cl_text_i32((i32)(7))"), "{c}");
}

/// IT-3: an owned text argument is materialized and released; a named place
/// is copied without a move.
#[test]
fn it3_owned_text_argument_is_materialized() {
    let owned = r#"module spec.gcr.it3a;
import { println } from std.io;
import { String, stringOf } from std.text;
import { text } from std.fmt;
func main(): i32 {
    println(text(stringOf("x")));
    return 0;
}
"#;
    let c = emitted_profile("textarg", "host", owned);
    assert!(c.contains("__cl_owned1 = cl_string_copy(__cl_owned0)"), "{c}");
    assert!(
        c.contains("if (__cl_owned0) { FreeString(__cl_owned0); }"),
        "{c}"
    );

    let place = r#"module spec.gcr.it3b;
import { println } from std.io;
import { String, stringOf } from std.text;
import { text } from std.fmt;
func main(): i32 {
    let s: String = stringOf("x");
    println(text(s));
    println(s);
    return 0;
}
"#;
    let c = emitted_profile("textplace", "host", place);
    assert!(c.contains("cl_string_copy(s)"), "{c}");
}

/// IT-4: a user `ffi __cl_itmp0` makes the init local `__cl_itmp1`.
#[test]
fn it4_init_local_skips_a_taken_symbol() {
    let source = r#"module spec.gcr.it4;
ffi probe { func __cl_itmp0(): i32; }
func mk(): i32[32] { return [; 7]; }
let call: i32[32] = mk();
let y: i32 = __cl_itmp0();
func main(): i32 { return call[0] + y; }
"#;
    let c = emitted_profile("itmp", "host", source);
    assert!(c.contains("__cl_itmp1[32]"), "{c}");
    assert!(!c.contains("__cl_itmp0[32]"), "{c}");
    build_object_strict("itmp", &c);
}

/// IT-5: an ordinary initializer keeps `__cl_itmp0`.
#[test]
fn it5_ordinary_init_keeps_itmp0() {
    let source = r#"module spec.gcr.it5;
func mk(): i32[32] { return [; 7]; }
let call: i32[32] = mk();
func main(): i32 { return call[0]; }
"#;
    let c = emitted_profile("normalitmp", "host", source);
    assert!(c.contains("__cl_itmp0[32]"), "{c}");
}

/// IT-6: an initializer text call with a module-level `std` uses the module
/// path.
#[test]
fn it6_initializer_emits_the_intrinsic() {
    let source = r#"module spec.gcr.it6;
import { String, stringOf } from std.text;
struct Fmt { text: func(i32): String; }
struct Std { fmt: Fmt; }
func userText(n: i32): String { return stringOf("u"); }
let std: Std = Std { fmt: Fmt { text: userText } };
let n: usize = len(std.fmt.text(7));
func main(): i32 { return i32(n); }
"#;
    let c = emitted_profile("inithead", "host", source);
    assert!(c.contains("cl_text_i32((i32)(7))"), "{c}");
    assert!(!c.contains("std.fmt.text(7)"), "{c}");
}

/// IT-8: a user `std.fmt` func-typed global `text` keeps its symbol.
#[test]
fn it8_user_global_text_keeps_its_symbol() {
    let holder = r#"module std.fmt;
import { String, stringOf } from std.text;
func userText(n: i32): String { return stringOf("gg"); }
export let text: func(i32): String = userText;
"#;
    let main = r#"module spec.gcr.it8;
import { println } from std.io;
import { String, stringOf } from std.text;
struct Fmt { text: func(i32): String; }
struct Std { fmt: Fmt; }
func userText(n: i32): String { return stringOf("u"); }
let std: Std = Std { fmt: Fmt { text: userText } };
let n: usize = len(std.fmt.text(1234));
func main(): i32 { println($"{n}"); return 0; }
"#;
    let c = emitted_pair("userglobal", holder, main);
    assert!(c.contains("(cl_std_fmt_text)(1234)"), "{c}");
    assert!(!c.contains("cl_text_i32((i32)(1234))"), "{c}");
}

/// IT-9: a real `std.fmt.text` ffi wins in an initializer.
#[test]
fn it9_ffi_text_wins_in_an_initializer() {
    let holder = r#"module std.fmt;
ffi stdfmtuser { func text(n: i32): String; }
"#;
    let main = r#"module spec.gcr.it9;
import { println } from std.io;
import { String, stringOf } from std.text;
struct Fmt { text: func(i32): String; }
struct Std { fmt: Fmt; }
func userText(n: i32): String { return stringOf("u"); }
let std: Std = Std { fmt: Fmt { text: userText } };
let n: usize = len(std.fmt.text(1234));
func main(): i32 { println($"{n}"); return 0; }
"#;
    let c = emitted_pair("ffiinit", holder, main);
    assert!(c.contains("text(1234)"), "{c}");
    assert!(!c.contains("cl_text_i32((i32)(1234))"), "{c}");
    build_object_strict("ffiinit", &c);
}

/// IT-10: a non-callable `std.fmt.text` declaration in an initializer is
/// rejected, not emitted as the intrinsic or a literal path.
#[test]
fn it10_non_callable_text_declaration_is_rejected() {
    let holder = r#"module std.fmt;
export struct text { x: i32; }
"#;
    let main = r#"module spec.gcr.it10;
import { String, stringOf } from std.text;
struct Fmt { text: func(i32): String; }
struct Std { fmt: Fmt; }
func userText(n: i32): String { return stringOf("u"); }
let std: Std = Std { fmt: Fmt { text: userText } };
let n: i32 = std.fmt.text(1234);
func main(): i32 { return n; }
"#;
    let scratch = ScratchDir::new("noncall");
    let holder_path = scratch.path().join("holder.clo");
    let main_path = scratch.path().join("main.clo");
    std::fs::write(&holder_path, holder).expect("write holder");
    std::fs::write(&main_path, main).expect("write main");
    let output_path = scratch.path().join("out.c");
    let output = run_clc(&[
        "--profile".to_owned(),
        "host".to_owned(),
        "-o".to_owned(),
        output_path.display().to_string(),
        main_path.display().to_string(),
        holder_path.display().to_string(),
    ]);
    assert_eq!(output.status.code(), Some(1), "expected a rejection");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("error E10309:"), "stderr:\n{stderr}");
    assert!(!output_path.exists(), "a rejected program wrote output C");
}

/// IT-11: a real `std.fmt.text` ffi keeps the qualified spelling, so a
/// same-named function of the initializing module cannot capture it.
#[test]
fn it11_ffi_initializer_keeps_the_qualified_spelling() {
    let holder = r#"module std.fmt;
ffi stdfmtuser { func text(n: i32): i32; }
"#;
    let main = r#"module spec.gcr.it11;
import { String, stringOf } from std.text;
struct Fmt { text: func(i32): String; }
struct Std { fmt: Fmt; }
func userText(n: i32): String { return stringOf("u"); }
func text(n: i32): i32 { return 99; }
let std: Std = Std { fmt: Fmt { text: userText } };
let n: i32 = std.fmt.text(1234);
func main(): i32 { return n; }
"#;
    let c = emitted_pair("ffiqualified", holder, main);
    assert!(c.contains("text(1234)"), "{c}");
    assert!(!c.contains("cl_spec_gcr_it11_text(1234)"), "{c}");
    build_object_strict("ffiqualified", &c);
}

/// IT-12: an imported/bare non-callable `std.fmt.text` declaration is
/// rejected with E10309, like the qualified spelling.
#[test]
fn it12_imported_non_callable_text_declaration_is_rejected() {
    let holder = r#"module std.fmt;
export struct text { x: i32; }
"#;
    for (tag, main) in [
        (
            "importnoncall",
            r#"module spec.gcr.it12a;
import { text } from std.fmt;
let n: i32 = text(1234);
func main(): i32 { return n; }
"#,
        ),
        (
            "aliasnoncall",
            r#"module spec.gcr.it12b;
import { text as t } from std.fmt;
let n: i32 = t(1234);
func main(): i32 { return n; }
"#,
        ),
    ] {
        let scratch = ScratchDir::new(tag);
        let holder_path = scratch.path().join("holder.clo");
        let main_path = scratch.path().join("main.clo");
        std::fs::write(&holder_path, holder).expect("write holder");
        std::fs::write(&main_path, main).expect("write main");
        let output_path = scratch.path().join("out.c");
        let output = run_clc(&[
            "--profile".to_owned(),
            "host".to_owned(),
            "-o".to_owned(),
            output_path.display().to_string(),
            main_path.display().to_string(),
            holder_path.display().to_string(),
        ]);
        assert_eq!(output.status.code(), Some(1), "{tag}: expected a rejection");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("error E10309:"), "{tag}: stderr:\n{stderr}");
        assert!(!output_path.exists(), "{tag}: a rejected program wrote output C");
    }
}
