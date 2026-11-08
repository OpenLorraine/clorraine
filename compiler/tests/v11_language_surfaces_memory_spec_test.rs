//! AddressSanitizer memory proof for the v1.1 drop paths.
//!
//! One program exercises every owning path the feature adds: the `Box<T>`
//! allocation, replacement, nested, container, struct, recursive-list, loop,
//! reassignment, defer, and null shapes; and an ordinary `freedBy` deleter that
//! consumes its handle and drops an owned field. It runs in its own suite so
//! the sanitizer child build is not mixed with the behavior suite's many
//! short-lived compilations.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Temporary directory that removes itself when the test finishes.
struct ScratchDir {
    path: PathBuf,
}

impl ScratchDir {
    /// Creates a unique directory for one test process.
    fn new(tag: &str) -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "clc-v11memory-{}-{tag}-{sequence}",
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

/// The comprehensive memory program.
const SOURCE: &str = r#"module spec.v11.memory;
import { boxNew, boxSet, boxWith, arrayNew, push, Array } from std.mem;
import { String, stringOf, bytes } from std.text;
struct Node { value: i32; next: Box<Node>?; }
struct Holder { s: String; }
struct Payload { s: String; }
handle Widget freedBy freeWidget;
ffi widget {
    func cl_widget_new(): Widget;
    func cl_widget_free(take w: Widget): void;
}
func freeWidget(take w: Widget): void {
    unsafe {
        let payload: Payload = *raw Payload(w);
    }
    cl_widget_free(w);
}
func demo(): i32 {
    let b = boxNew<i32>(7);
    let n: i32 = 0;
    boxWith(b, [borrow n] (v: i32) => { n = v; });
    if (n != 7) { return 1; }
    let s = boxNew<String>(stringOf("ab"));
    boxSet(s, stringOf("cde"));
    boxWith(s, [borrow n] (v: String) => { n = i32(len(bytes(v))); });
    if (n != 3) { return 2; }
    let inner = boxNew<String>(stringOf("x"));
    let outer = boxNew<Box<String>>(inner);
    let xs = arrayNew<String>();
    push<String>(xs, stringOf("a"));
    let arr = boxNew<Array<String>>(xs);
    let h = boxNew<Holder>(Holder { s: stringOf("z") });
    let c = Node { value: 3, next: null };
    let l1 = boxNew<Node>(c);
    let l2 = boxNew<Node>(Node { value: 2, next: l1 });
    let l3 = boxNew<Node>(Node { value: 1, next: l2 });
    let i: i32 = 0;
    while (i < 2) {
        let loopBox = boxNew<i32>(i);
        i = i + 1;
    }
    let r = boxNew<String>(stringOf("aa"));
    r = boxNew<String>(stringOf("bb"));
    defer {
        let d = boxNew<i32>(9);
    }
    let w = cl_widget_new();
    unsafe {
        *raw Payload(w) = Payload { s: stringOf("owned") };
    }
    return 0;
}
func main(): i32 { return demo(); }
"#;

/// The C fixture behind the widget allocator/release.
const FIXTURE: &str = r#"#include <stdlib.h>
#include <string.h>
void *cl_widget_new(void) { return calloc(1, 64); }
void cl_widget_free(void *p) { free(p); }
"#;

/// Compiles, builds with the fixture under ASan, and runs the program.
fn run_memory_case(tag: &str) -> std::process::Output {
    let scratch = ScratchDir::new(tag);
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, SOURCE).expect("write the program");
    let c_path = scratch.path().join("out.c");
    let compile = Command::new("timeout")
        .args(["60", env!("CARGO_BIN_EXE_clc")])
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
        compile.status.code(),
        Some(0),
        "the memory program was rejected; stderr:\n{}",
        String::from_utf8_lossy(&compile.stderr)
    );
    let fixture_path = scratch.path().join("fixture.c");
    std::fs::write(&fixture_path, FIXTURE).expect("write the fixture");
    let program = scratch.path().join("prog");
    let built = Command::new("timeout")
        .args(["60", "cc"])
        .args([
            "-std=c99",
            "-O0",
            "-g",
            "-Wall",
            "-Wextra",
            "-Werror",
            "-fsanitize=address",
            "-fno-omit-frame-pointer",
            "-o",
        ])
        .arg(&program)
        .arg(&c_path)
        .arg(&fixture_path)
        .output()
        .expect("failed to spawn the host C compiler");
    assert!(
        built.status.success(),
        "the strict host C compiler rejected the memory program; stderr:\n{}",
        String::from_utf8_lossy(&built.stderr)
    );
    Command::new("timeout")
        .args(["30", program.to_str().expect("program path")])
        .env("ASAN_OPTIONS", "detect_leaks=1:abort_on_error=1")
        .output()
        .expect("failed to run the memory program")
}

/// The comprehensive Box and ordinary-deleter program is ASan-clean.
#[test]
fn the_v11_memory_case_is_clean() {
    if std::env::var_os("RDC_ASAN").is_none() {
        eprintln!("skip: RDC_ASAN is not set");
        return;
    }
    let result = run_memory_case("case");
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        result.status.code() == Some(0),
        "the memory program exited {:?}; stderr:\n{stderr}",
        result.status.code()
    );
    assert!(
        !stderr.contains("AddressSanitizer") && !stderr.contains("LeakSanitizer"),
        "the sanitizer reported a memory problem:\n{stderr}"
    );
}
