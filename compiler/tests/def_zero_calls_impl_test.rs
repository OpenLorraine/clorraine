//! Emission pins for the zero-defect calls and resolution batch.
//!
//! The specification tests state the diagnostics and runs. These cases pin the
//! emitted spellings that carry the call-resolution rules: the escaped type
//! key, the raw field-call arrow, the host-clean pointer cast, and the
//! import-renamed Array instance.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

struct ScratchDir {
    path: PathBuf,
}

impl ScratchDir {
    fn new(tag: &str) -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "clc-zerocallsimpl-{}-{tag}-{sequence}",
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

fn emit_host(tag: &str, source: &str) -> String {
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
        .expect("failed to spawn the compiler binary");
    assert_eq!(
        output.status.code(),
        Some(0),
        "{tag}: the front end rejected the program; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    std::fs::read_to_string(&c_path).unwrap_or_default()
}

/// The user type component that would collide with a func key is escaped, and
/// a callable field through a raw receiver uses the implied arrow.
#[test]
fn should_pin_the_escaped_key_and_the_raw_field_arrow() {
    let key = r#"module spec.zcil.key;

struct Holder<T> { value: T; }
struct Holder__fn_borrow_i32__i32 { x: i32; }
func inc(borrow x: i32): i32 { return x + 1; }
func main(): i32 {
    let h: Holder<func(borrow i32): i32> =
        Holder<func(borrow i32): i32> { value: inc };
    let s: Holder__fn_borrow_i32__i32 = Holder__fn_borrow_i32__i32 { x: 1 };
    return h.value(1) + s.x - 3;
}
"#;
    let emitted = emit_host("key", key);
    assert!(
        emitted.contains("struct Clu26_Holder__fn_borrow_i32__i32 {"),
        "key: the colliding user component must be escaped; emitted C:\n{emitted}"
    );
    assert!(
        emitted.contains("struct ClHolder__fn_borrow_i32__i32 {"),
        "key: the func-type instance keeps its own key; emitted C:\n{emitted}"
    );

    let raw = r#"module spec.zcil.rawfield;

struct S { foo: func(i32): i32; }
func inc(x: i32): i32 { return x + 1; }
func main(): i32 {
    let s: S = S { foo: inc };
    let p: raw S = unsafe &s;
    unsafe { return p.foo(1) - 2; }
}
"#;
    let emitted = emit_host("rawfield", raw);
    assert!(
        emitted.contains("p->foo"),
        "rawfield: the raw receiver must use the implied dereference; emitted C:\n{emitted}"
    );
}

/// The sanctioned pointer-to-integer cast routes through uintptr_t, and an
/// import-renamed type resolves to its declared Array instance.
#[test]
fn should_pin_the_host_clean_cast_and_the_alias_instance() {
    let cast = r#"module spec.zcil.cast;

func main(): i32 {
    let s: str = "ab";
    let n: usize = usize(s.ptr);
    return i32(n) - i32(n);
}
"#;
    let emitted = emit_host("cast", cast);
    assert!(
        emitted.contains("uintptr_t"),
        "cast: the raw-to-integer cast must route through uintptr_t; emitted C:\n{emitted}"
    );

    let alias = r#"module spec.zcil.alias;

import { Array as A, arrayNew } from std.mem;
struct S { a: A<i32>; }
func main(): i32 {
    let s: S = S { a: arrayNew<i32>() };
    return i32(s.a.count);
}
"#;
    let emitted = emit_host("alias", alias);
    assert!(
        emitted.contains("ClArray__i32"),
        "alias: the imported alias must reach its Array instance; emitted C:\n{emitted}"
    );
}
