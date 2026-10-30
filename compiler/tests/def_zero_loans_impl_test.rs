//! Implementation pins for the zero-defect borrow and loan audit.

use std::path::PathBuf;
use std::process::{Command, Output};

fn compile(tag: &str, source: &str) -> (PathBuf, Output, PathBuf) {
    let root = std::env::temp_dir().join(format!(
        "clc-zl-impl-{}-{tag}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("scratch directory");
    let input = root.join("main.clo");
    std::fs::write(&input, source).expect("write source");
    let output_path = root.join("out.c");
    let output = Command::new(env!("CARGO_BIN_EXE_clc"))
        .args([
            "--profile",
            "host",
            "-o",
            output_path.to_str().expect("output path"),
            input.to_str().expect("input path"),
        ])
        .output()
        .expect("spawn clc");
    (root, output, output_path)
}

#[test]
fn should_pin_the_static_take_carrier_shapes() {
    let source = r#"module spec.zlimpl.staticcarrier;

struct Holder { names: str[2]; }
func makeStatic(): Holder { return Holder { names: ["AA", "BB"] }; }
func store(take h: Holder, borrow slot: str?): void { slot = h.names[0]; }

func main(): i32 {
    let out: str? = null;
    let hs: Holder[1] = [Holder { names: ["AA", "BB"] }];
    for (borrow e in hs) { store(take e, borrow out); }
    if (out == null) { return 9; }
    return i32(out[0]) - 65;
}
"#;
    let (root, output, c_path) = compile("static", source);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let emitted = std::fs::read_to_string(&c_path).expect("emitted C");
    assert!(emitted.contains("cl_spec_zlimpl_staticcarrier_store"), "{emitted}");
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn should_reject_a_caller_retained_temporary_carrier() {
    let source = r#"module spec.zlimpl.temporary;

import { String, stringOf } from std.text;

struct Holder { names: str[2]; }
func store(take h: Holder, borrow slot: str?): void { slot = h.names[0]; }

func fill(borrow out: str?): void {
    store(Holder { names: [stringOf("AA"), "BB"] }, borrow out);
}

func main(): i32 {
    let out: str? = null;
    fill(borrow out);
    return 0;
}
"#;
    let (root, output, c_path) = compile("temporary", source);
    assert_ne!(output.status.code(), Some(0));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(stderr.matches("error E10030:").count(), 1, "{stderr}");
    assert!(!c_path.exists());
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn should_reject_a_capture_address_escape() {
    let source = r#"module spec.zlimpl.capture;

import { String, stringOf } from std.text;

struct Holder { names: str[2]; }
struct Box { v: String; }
func store(take h: Holder, borrow slot: str?): void { slot = h.names[0]; }

func fill(borrow out: str?): void {
    let a: Box = Box { v: stringOf("aa") };
    let h: Holder = Holder { names: ["AA", "BB"] };
    ([borrow h, borrow a] () => {
        unsafe { let p: raw Holder = &h; (*p).names[0] = a.v; }
    })();
    store(take h, borrow out);
}

func main(): i32 {
    let out: str? = null;
    fill(borrow out);
    return 0;
}
"#;
    let (root, output, c_path) = compile("capture", source);
    assert_ne!(output.status.code(), Some(0));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(stderr.matches("error E10030:").count(), 1, "{stderr}");
    assert!(!c_path.exists());
    let _ = std::fs::remove_dir_all(root);
}
