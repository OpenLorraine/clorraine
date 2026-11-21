//! Invalid selected assemblies must never fall back to another installation.

use super::fixtures::*;
use std::os::unix::fs::{symlink, PermissionsExt};
use std::path::Path;

/// Asserts a selected SDK rejection, including its identity and lack of any
/// compiler/runner invocation. Exit two alone could hide an unknown-option bug.
fn rejected(f: &Fixture, install: &Path, action: &str) {
    let output = f.native(install, action, &[]);
    exit(&output, 2);
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains(text(install)), "diagnostic identifies selected SDK: {error}");
    f.assert_no_processes();
}

/// A nonexistent explicit SDK cannot fall back to valid environment or adjacent
/// candidates, even when legacy tools are also available.
#[test]
fn invalid_cli_sdk_does_not_fall_back() {
    let f = Fixture::new("invalid-cli");
    let adjacent = f.install("adjacent", "adjacent", "cp310");
    let environment = f.install("environment", "environment", "cp310");
    let invalid = f.root.join("missing chosen SDK");
    let output = run(f.command(
        &adjacent.join("bin/clorraine"),
        &["build", "--profile", "hosted-amiga", "--sdk-root", text(&invalid), "hello.clo"],
    ).env("CLORRAINE_SDK", environment.join("sdk/amiga")));
    exit(&output, 2);
    assert!(String::from_utf8_lossy(&output.stderr).contains(text(&invalid)));
    f.assert_no_processes();
}

/// Invalid environment selection outranks valid adjacent discovery.
#[test]
fn invalid_environment_sdk_does_not_fall_back() {
    let f = Fixture::new("invalid-env");
    let adjacent = f.install("adjacent", "adjacent", "cp310");
    let invalid = f.root.join("missing environment SDK");
    let output = run(f.command(
        &adjacent.join("bin/clorraine"),
        &["build", "--profile", "hosted-amiga", "hello.clo"],
    ).env("CLORRAINE_SDK", &invalid));
    exit(&output, 2);
    assert!(String::from_utf8_lossy(&output.stderr).contains(text(&invalid)));
    f.assert_no_processes();
}

/// Either adjacent candidate marker, including a broken link, prevents legacy
/// fallback. This avoids silently changing execution mode on partial state.
#[test]
fn invalid_adjacent_candidate_does_not_fall_back() {
    for marker in ["installation.json", "sdk"] {
        let f = Fixture::new("invalid-adjacent");
        let adjacent = f.driver.parent().unwrap().parent().unwrap();
        symlink("nonexistent", adjacent.join(marker))
            .expect("create broken adjacent candidate");
        let output = run(&mut f.command(
            &f.driver, &["build", "--profile", "hosted-amiga", "hello.clo"],
        ));
        exit(&output, 2);
        assert!(String::from_utf8_lossy(&output.stderr).contains(text(adjacent)),
            "diagnostic must identify invalid adjacent selection");
        f.assert_no_processes();
    }
}

/// Each generated case changes only one field of an otherwise complete,
/// independently specified descriptor.
macro_rules! descriptor_case {
    ($name:ident, $from:expr, $to:expr) => {
        /// Invalid descriptor fields reject the selected SDK before tool use.
        #[test]
        fn $name() {
            let f = Fixture::new(stringify!($name));
            let install = f.install("candidate", "candidate", "cp310");
            replace(&install.join("installation.json"), $from, $to);
            rejected(&f, &install, "build");
        }
    };
}

descriptor_case!(descriptor_rejects_boolean_schema, "\"schema\":1", "\"schema\":true");
descriptor_case!(descriptor_rejects_unknown_schema, "\"schema\":1", "\"schema\":2");
descriptor_case!(descriptor_rejects_duplicate_keys, "\"schema\":1", "\"schema\":1,\"schema\":1");
descriptor_case!(descriptor_rejects_unknown_fields, "\"schema\":1", "\"schema\":1,\"extra\":0");
descriptor_case!(descriptor_rejects_missing_field, "\"core_version\":\"1.1.2\",", "");
descriptor_case!(descriptor_rejects_wrong_field_type, "\"core_version\":\"1.1.2\"", "\"core_version\":112");
descriptor_case!(descriptor_rejects_invalid_version, "\"1.1.2\"", "\"1.1\"");
descriptor_case!(descriptor_rejects_unsupported_python_abi, "\"cp310\"", "\"cp311\"");
descriptor_case!(descriptor_rejects_invalid_hash, &"a".repeat(64), &"A".repeat(64));

/// UTF-8 decoding is part of metadata validation, not lossy terminal display.
#[test]
fn descriptor_rejects_invalid_utf8() {
    let f = Fixture::new("invalid-utf8");
    let install = f.install("candidate", "candidate", "cp310");
    std::fs::write(install.join("installation.json"), [0xff]).unwrap();
    rejected(&f, &install, "build");
}

/// Oversized metadata must be rejected rather than read without a bound.
#[test]
fn descriptor_rejects_metadata_larger_than_sixteen_mebibytes() {
    let f = Fixture::new("oversized");
    let install = f.install("candidate", "candidate", "cp310");
    let mut bytes = descriptor("cp310").into_bytes();
    bytes.resize(16 * 1024 * 1024 + 1, b' ');
    std::fs::write(install.join("installation.json"), bytes).unwrap();
    rejected(&f, &install, "build");
}

/// Metadata must be an ordinary file, not even an internal symlink.
#[test]
fn descriptor_rejects_symlink_metadata() {
    let f = Fixture::new("metadata-link");
    let install = f.install("candidate", "candidate", "cp310");
    let path = install.join("installation.json");
    std::fs::rename(&path, install.join("descriptor-copy")).unwrap();
    symlink("descriptor-copy", &path).unwrap();
    rejected(&f, &install, "build");
}

/// SDK preparation and the assembly descriptor must agree on all three hashes.
#[test]
fn preparation_receipt_must_match_descriptor_hashes() {
    for original in ["a", "b", "c"] {
        let f = Fixture::new("ndk-mismatch");
        let install = f.install("candidate", "candidate", "cp310");
        replace(&install.join("sdk/amiga/.ndk-prepared.json"),
            &original.repeat(64), &"d".repeat(64));
        rejected(&f, &install, "build");
    }
}

/// Missing preparation metadata is invalid selection, not permission to use
/// the legacy compiler.
#[test]
fn missing_preparation_receipt_rejects_selection() {
    let f = Fixture::new("ndk-missing");
    let install = f.install("candidate", "candidate", "cp310");
    std::fs::remove_file(install.join("sdk/amiga/.ndk-prepared.json")).unwrap();
    rejected(&f, &install, "build");
}

/// Native overrides cannot identify assemblies without their descriptor.
#[test]
fn missing_installation_descriptor_rejects_selection() {
    let f = Fixture::new("descriptor-missing");
    let install = f.install("candidate", "candidate", "cp310");
    std::fs::remove_file(install.join("installation.json")).unwrap();
    rejected(&f, &install, "build");
}

/// Preparation receipts follow the same exact schema rather than merely
/// containing matching hashes somewhere in an arbitrary JSON document.
#[test]
fn malformed_preparation_receipt_rejects_selection() {
    let f = Fixture::new("receipt-schema");
    let install = f.install("candidate", "candidate", "cp310");
    replace(&install.join("sdk/amiga/.ndk-prepared.json"),
        "\"schema\":1", "\"schema\":true");
    rejected(&f, &install, "build");
}

/// A present interpreter does not replace the required source wheel receipt.
#[test]
fn missing_companion_wheel_receipt_is_invalid_metadata() {
    let f = Fixture::new("wheel-receipt-missing");
    let install = f.install("candidate", "candidate", "cp310");
    std::fs::remove_file(install.join("sdk/vamos/wheels.json")).unwrap();
    rejected(&f, &install, "run");
}

/// Run requires the copied wheel receipt to match both ABI and support lock.
#[test]
fn companion_wheel_receipt_must_match_abi_and_support_lock() {
    for (from, to) in [("\"cp310\"".to_owned(), "\"cp312\"".to_owned()),
                       ("a".repeat(64), "d".repeat(64))] {
        let f = Fixture::new("wheel-pairing");
        let install = f.install("candidate", "candidate", "cp310");
        replace(&install.join("sdk/vamos/wheels.json"), &from, &to);
        rejected(&f, &install, "run");
    }
}

/// The copied receipt remains a complete schema, not just an ABI hint.
#[test]
fn companion_wheel_receipt_rejects_invalid_package_records() {
    for (from, to) in [
        ("\"schema\":1", "\"schema\":true"),
        ("\"bytes\":1", "\"bytes\":true"),
        ("\"source\":\"amitools\"", "\"source\":\"../amitools\""),
        ("\"purpose\":\"bootstrap\"", "\"purpose\":\"runtime\""),
        ("\"tags\":[\"py3-none-any\"]", "\"tags\":[]"),
        ("wheels/amitools-1.0-py3-none-any.whl", "../escape.whl"),
    ] {
        let f = Fixture::new("wheel-record");
        let install = f.install("candidate", "candidate", "cp310");
        replace(&install.join("sdk/vamos/wheels.json"), from, to);
        rejected(&f, &install, "run");
    }
}

/// Removing a required package must fail even when interpreter files exist.
#[test]
fn companion_wheel_receipt_requires_every_runtime_and_bootstrap_name() {
    for name in ["amitools", "machine68k", "greenlet", "pysdl2", "pip"] {
        let f = Fixture::new("wheel-package-missing");
        let install = f.install("candidate", "candidate", "cp310");
        replace(&install.join("sdk/vamos/wheels.json"),
            &format!("\"{name}\":{{"), &format!("\"other-{name}\":{{"));
        rejected(&f, &install, "run");
    }
}

/// An override identifies the Amiga root, not the enclosing installation.
#[test]
fn installation_root_is_not_an_sdk_override() {
    let f = Fixture::new("wrong-level");
    let install = f.install("candidate", "candidate", "cp310");
    let output = run(&mut f.command(&f.driver,
        &["build", "--profile", "hosted-amiga", "--sdk-root", text(&install), "hello.clo"]));
    exit(&output, 2);
    assert!(String::from_utf8_lossy(&output.stderr).contains(text(&install)));
    f.assert_no_processes();
}

/// Directory aliases must not combine one installation's descriptor with
/// another installation's SDK or Python environment.
#[test]
fn cross_installation_sdk_and_companion_directory_aliases_are_rejected() {
    for relative in ["sdk/amiga", "sdk/vamos"] {
        let f = Fixture::new("directory-pairing");
        let install = f.install("candidate", "candidate", "cp310");
        let other = f.install("other", "other", "cp310");
        let path = install.join(relative);
        std::fs::rename(&path, path.with_file_name("unused-original")).unwrap();
        symlink(other.join(relative), &path).unwrap();
        rejected(&f, &install, "run");
    }
}

/// Internal compiler aliases are allowed, but cannot escape the selected SDK.
#[test]
fn compiler_alias_into_another_installation_is_rejected() {
    let f = Fixture::new("compiler-escape");
    let install = f.install("candidate", "candidate", "cp310");
    let other = f.install("other", "other", "cp310");
    let compiler = install.join("sdk/amiga/bin/m68k-amigaos-gcc");
    std::fs::remove_file(&compiler).unwrap();
    symlink(other.join("sdk/amiga/bin/m68k-amigaos-gcc"), &compiler).unwrap();
    rejected(&f, &install, "build");
}

/// Compiler-only builds do not depend on venv availability or wheel metadata.
#[test]
fn build_needs_neither_companion_files_nor_valid_unused_wheel_metadata() {
    for companion in [false, true] {
        let f = Fixture::new("compiler-only");
        let install = f.install("candidate", "candidate", "cp310");
        let venv = install.join("sdk/vamos");
        if companion {
            std::fs::write(venv.join("wheels.json"), b"not JSON").unwrap();
            std::fs::remove_file(venv.join("bin/python")).unwrap();
            std::fs::remove_file(venv.join("pyvenv.cfg")).unwrap();
        } else {
            std::fs::remove_dir_all(&venv).unwrap();
        }
        let output = f.native(&install, "build", &[]);
        exit(&output, 0);
        assert_eq!(f.argv("candidate", "compiler")[0],
            text(&install.join("sdk/amiga/bin/m68k-amigaos-gcc")));
        assert_eq!(std::fs::read_dir(&f.logs).unwrap().count(), 1);
    }
}

/// A valid SDK with a non-executable compiler has a missing-tool failure,
/// rather than malformed metadata or a lower-priority fallback.
#[test]
fn non_executable_native_compiler_is_a_missing_tool() {
    let f = Fixture::new("compiler-permission");
    let install = f.install("candidate", "candidate", "cp310");
    std::fs::set_permissions(install.join("sdk/amiga/bin/m68k-amigaos-gcc"),
        std::fs::Permissions::from_mode(0o644)).unwrap();
    let output = f.native(&install, "build", &[]);
    exit(&output, 1);
    assert!(String::from_utf8_lossy(&output.stderr).contains("m68k-amigaos-gcc"));
    f.assert_no_processes();
}
