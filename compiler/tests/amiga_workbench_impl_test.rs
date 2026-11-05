//! Implementation tests for the Bundle 8 Workbench, icon, and expansion
//! surface.
//!
//! Phase 1 pins the public message port and the `CL_HOSTED_AMIGA_20` profile
//! define. The module-specific cases are added by their phases.

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
            "clc-workbench-impl-{}-{tag}-{sequence}",
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

fn run_clc(args: &[String]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_clc"))
        .args(args)
        .output()
        .expect("failed to spawn the compiler binary")
}

/// Compiles `source` for `profile` and returns the emitted C.
fn emit(tag: &str, profile: &str, source: &str) -> String {
    let scratch = ScratchDir::new(tag);
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, source).expect("write the snippet");
    let output_path = scratch.path().join("out.c");
    let run = run_clc(&[
        "--profile".to_owned(),
        profile.to_owned(),
        "-o".to_owned(),
        output_path.display().to_string(),
        source_path.display().to_string(),
    ]);
    assert_eq!(
        run.status.code(),
        Some(0),
        "{tag}: the front end rejected the program; stderr:\n{}",
        String::from_utf8_lossy(&run.stderr)
    );
    std::fs::read_to_string(&output_path).expect("read the emitted C")
}

fn assert_contains(tag: &str, haystack: &str, needle: &str) {
    assert!(
        haystack.contains(needle),
        "{tag}: emitted C does not contain {needle:?}"
    );
}

/// The shim source, read directly so the C contract is pinned where it is
/// implemented rather than through a compiled target.
fn shim() -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("runtime/shim_hosted_amiga.c");
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()))
}

const PUBLIC_PORT_SOURCE: &str = r#"module tests.wb.publicportimpl;

import { MsgPort, createPublicPort, deleteMsgPort } from std.amiga.exec;
import { String, stringOf, toCString } from std.text;

func main(): i32 {
    let name: String = stringOf("ClorraineWbTest");
    let port: MsgPort? = createPublicPort(toCString(name));
    if (port == null) {
        return 1;
    }
    deleteMsgPort(port);
    return 0;
}
"#;

/// The public port wrapper lowers to the shim, and the caller cleanup block
/// deletes the port exactly once.
#[test]
fn the_public_port_wrapper_lowers_to_the_shim() {
    let c = emit("public-port-routing", "hosted-amiga", PUBLIC_PORT_SOURCE);
    assert_contains("public-port-routing", &c, "cl_create_public_port");
    assert_contains(
        "public-port-routing",
        &c,
        "__cl_c1:\n    if (port) { cl_delete_msg_port(port); }",
    );
}

/// The shim rejects a null or empty name and uses the 1.3-safe `CreatePort`
/// constructor.
#[test]
fn the_public_port_shim_validates_the_name() {
    let shim = shim();
    for needle in [
        "void *cl_create_public_port(char *name)",
        "name == NULL",
        "name[0] == '\\0'",
        "CreatePort((STRPTR)name, 0)",
    ] {
        assert_contains("public-port-shim", &shim, needle);
    }
}

/// The 2.0-only shim section is enabled by the profile define, and the 1.3
/// emit does not define it.
#[test]
fn the_20_shim_define_is_emitted_only_for_hosted_amiga() {
    let source = "module tests.wb.define;\n\nfunc main(): i32 {\n    return 0;\n}\n";
    let modern = emit("define-20", "hosted-amiga", source);
    assert_contains(
        "define-20",
        &modern,
        "#define CL_HOSTED_AMIGA_20 1",
    );
    let a500 = emit("define-13", "hosted-amiga-13", source);
    assert!(
        !a500.contains("#define CL_HOSTED_AMIGA_20 1"),
        "the 1.3 emit must not define CL_HOSTED_AMIGA_20"
    );
}

/// The shim guards its 2.0-only section with the profile define.
#[test]
fn the_shim_guards_the_20_section() {
    let shim = shim();
    assert_contains("shim-guard", &shim, "#if defined(CL_HOSTED_AMIGA_20)");
    assert_contains("shim-guard", &shim, "#endif");
}
