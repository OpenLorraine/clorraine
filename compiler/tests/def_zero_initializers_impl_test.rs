//! Implementation pins for the zero-defect initializer and carrier batch.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

/// Temporary directory that removes itself when the test finishes.
struct ScratchDir {
    path: PathBuf,
}

impl ScratchDir {
    fn new(tag: &str) -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "clc-zeroinit-impl-{}-{tag}-{sequence}",
            std::process::id()
        ));
        std::fs::create_dir_all(&path)
            .unwrap_or_else(|err| panic!("cannot create {}: {err}", path.display()));
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

/// Compiles one host program and returns the output and C path.
fn compile(tag: &str, source: &str) -> (ScratchDir, Output, PathBuf) {
    let scratch = ScratchDir::new(tag);
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, source)
        .unwrap_or_else(|err| panic!("{tag}: cannot write the program: {err}"));
    let c_path = scratch.path().join("out.c");
    let output = Command::new(env!("CARGO_BIN_EXE_clc"))
        .args([
            "--profile",
            "host",
            "-o",
            c_path.to_str().expect("C path"),
            source_path.to_str().expect("source path"),
        ])
        .output()
        .expect("failed to spawn clc");
    (scratch, output, c_path)
}

/// Compiles one accepted host program and returns its emitted C.
fn emit(tag: &str, source: &str) -> String {
    let (_scratch, output, c_path) = compile(tag, source);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{tag}: the front end rejected the program; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    std::fs::read_to_string(&c_path).unwrap_or_default()
}

/// the initializer element take reports the body's partial-move rule.
#[test]
fn should_reject_the_initializer_element_take() {
    let source = r#"module spec.zii.element_take;

import { String, stringOf } from std.text;

func two(): String[2] {
    return [stringOf("a"), stringOf("bb")];
}

func consume(take xs: String): i32 {
    return i32(len(xs));
}

let g: i32 = consume(two()[0]);

func main(): i32 {
    return g - 1;
}
"#;
    let (_scratch, output, c_path) = compile("element_take", source);
    assert_ne!(output.status.code(), Some(0));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        stderr.matches("error E10026:").count(),
        1,
        "element_take: one E10026; stderr:\n{stderr}"
    );
    assert!(!c_path.exists(), "element_take: a rejected program writes no C");
}

/// the guarded member argument binds its fresh root and releases it.
#[test]
fn should_bind_the_guarded_member_argument_root() {
    let source = r#"module spec.zii.guarded_member;

import { String, stringOf } from std.text;

struct Box { n: i32; v: String; }

func mk(): Box {
    return Box { n: 1, v: stringOf("0123456789") };
}

func use(x: i32): i32 {
    return x;
}

func main(): i32 {
    let g: bool = true;
    if (g && use(mk().n) == 1) {
        return 0;
    }
    return 1;
}
"#;
    let emitted = emit("guarded_member", source);
    assert!(
        emitted.contains("FreeString("),
        "guarded_member: the residual field must release; C:\n{emitted}"
    );
}

/// the initializer view-array element retains its fresh root.
#[test]
fn should_retain_the_initializer_view_array_root() {
    let source = r#"module spec.zii.carrier_root;

import { String, stringOf } from std.text;

struct Box { v: String; w: String; }
struct H { names: str[1]; tag: String; }

func mk(): Box {
    return Box { v: stringOf("ab"), w: stringOf("cd") };
}

let g: str = H { names: [mk().v], tag: stringOf("t") }.names[0];

func main(): i32 {
    return i32(len(g)) - 2;
}
"#;
    let emitted = emit("carrier_root", source);
    let init = emitted
        .split("static void cl_init_spec_zii_carrier_root")
        .nth(1)
        .expect("the initializer function must exist");
    assert!(
        init.contains("cl_probe") || init.contains("mk()"),
        "carrier_root: the fresh root must bind; C:\n{init}"
    );
    assert!(
        init.matches("FreeString(").count() >= 2,
        "carrier_root: the tag and the residual sibling must release; C:\n{init}"
    );
}
