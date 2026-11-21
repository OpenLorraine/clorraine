//! Native failures retain existing status classes and diagnostic formats.

use super::fixtures::*;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::{Command, Output};

/// Checks that a tool failure names the tool and offers configuration or
/// installation guidance, without requiring one particular English sentence.
fn missing_tool(output: &Output, name: &str) {
    exit(output, 1);
    let message = String::from_utf8_lossy(&output.stderr);
    assert!(message.contains(name), "missing tool must be named: {message}");
    assert!(
        ["--sdk-root", "CLORRAINE_SDK", "install", "restore", "SDK", "sdk"]
            .iter().any(|remedy| message.contains(remedy)),
        "missing tool needs a configuration remedy: {message}"
    );
}

/// Uses the system Python JSON parser to validate every diagnostic line; no
/// crate dependency or hand-written partial JSON parser enters this std-only
/// integration suite. The existing diagnostic object shape stays unchanged.
fn ndjson(f: &Fixture, output: &Output) {
    let path = f.root.join("diagnostics.ndjson");
    std::fs::write(&path, &output.stderr).expect("save owned diagnostic stream");
    let validation = Command::new("timeout")
        .args(["10s", "/usr/bin/python3", "-I", "-c", r#"import json, sys
lines = open(sys.argv[1], encoding='utf-8').read().splitlines()
assert lines, 'no diagnostics'
required = {'code','severity','message','help','file','line','column','endLine','endColumn'}
for line in lines:
    item = json.loads(line)
    assert isinstance(item, dict) and set(item) == required, item
    assert item['severity'] == 'error', item
    assert isinstance(item['message'], str) and item['message'], item
"#])
        .arg(&path).output().expect("validate NDJSON with system Python");
    exit(&validation, 0);
    assert!(output.stdout.is_empty(), "failed builds have no standard output");
}

/// Deletes only one required fixture file or its owned companion directory.
fn remove_tool(install: &Path, relative: &str) {
    let path = install.join(relative);
    if path.is_dir() {
        std::fs::remove_dir_all(path).unwrap();
    } else {
        std::fs::remove_file(path).unwrap();
    }
}

/// Missing tools in valid metadata are environment failures, never successful
/// fallback builds. Each generated case also exercises the JSON error path.
macro_rules! missing_case {
    ($test:ident, $relative:literal, $name:literal, $action:literal) => {
        /// A missing required native tool returns one with a named remedy.
        #[test]
        fn $test() {
            for json in [false, true] {
                let f = Fixture::new(stringify!($test));
                let install = f.install("candidate", "candidate", "cp310");
                remove_tool(&install, $relative);
                let extra: &[&str] = if json { &["--diagnostics=json"] } else { &[] };
                let output = f.native(&install, $action, extra);
                missing_tool(&output, $name);
                if json {
                    ndjson(&f, &output);
                }
                f.assert_no_processes();
            }
        }
    };
}

missing_case!(missing_native_compiler_returns_one,
    "sdk/amiga/bin/m68k-amigaos-gcc", "m68k-amigaos-gcc", "build");
missing_case!(missing_companion_interpreter_returns_one,
    "sdk/vamos/bin/python", "python", "run");
missing_case!(missing_companion_directory_returns_one,
    "sdk/vamos", "vamos", "run");
missing_case!(missing_venv_configuration_returns_one,
    "sdk/vamos/pyvenv.cfg", "pyvenv.cfg", "run");

/// A present but non-executable interpreter is also a missing required tool.
#[test]
fn non_executable_companion_interpreter_returns_one() {
    let f = Fixture::new("python-permission");
    let install = f.install("candidate", "candidate", "cp310");
    std::fs::set_permissions(install.join("sdk/vamos/bin/python"),
        std::fs::Permissions::from_mode(0o644)).unwrap();
    let output = f.native(&install, "run", &[]);
    missing_tool(&output, "python");
    f.assert_no_processes();
}

/// Front-end errors retain exit one and valid NDJSON after native resolution,
/// without compiling C or starting a runner.
#[test]
fn native_source_error_preserves_human_and_json_compiler_diagnostics() {
    for json in [false, true] {
        let f = Fixture::new("source-error");
        let install = f.install("candidate", "candidate", "cp310");
        f.write_source("module hello;\nfunc main(): i32 { let = 5; return 0; }\n");
        let extra: &[&str] = if json { &["--diagnostics=json"] } else { &[] };
        let output = f.native(&install, "build", extra);
        exit(&output, 1);
        if json {
            ndjson(&f, &output);
        } else {
            assert!(String::from_utf8_lossy(&output.stderr).contains("error E"));
        }
        f.assert_no_processes();
    }
}

/// External compiler failures are build errors, not the compiler process's raw
/// exit code. Raw terminal escape bytes must not reach human error display.
#[test]
fn native_c_compiler_failure_returns_one_and_sanitizes_human_display() {
    for json in [false, true] {
        let f = Fixture::new("c-error");
        let install = f.install("candidate", "candidate", "cp310");
        let mut args = vec!["build", "--profile", "hosted-amiga", "--sdk-root"];
        let sdk = install.join("sdk/amiga");
        args.push(text(&sdk));
        if json {
            args.push("--diagnostics=json");
        }
        args.push("hello.clo");
        let output = run(f.command(&f.driver, &args)
            .env("TEST_NATIVE_COMPILER_ERROR", "native-c-error \u{1b}[31mred\u{1b}[0m\n"));
        exit(&output, 1);
        let message = String::from_utf8_lossy(&output.stderr);
        assert!(message.contains("native-c-error"), "compiler error remains identifiable");
        assert!(!output.stderr.contains(&0x1b), "terminal escape must be displayed safely");
        if json {
            ndjson(&f, &output);
        }
        f.argv("candidate", "compiler");
        assert!(!f.logs.join("candidate-runner.argv").exists());
        assert!(!f.logs.join("legacy-compiler.argv").exists());
    }
}

/// A selected path containing terminal controls must be diagnosed safely,
/// while still identifying its printable path components.
#[test]
fn invalid_sdk_path_sanitizes_human_display_without_losing_selection_identity() {
    let f = Fixture::new("sdk-control-display");
    let invalid = f.root.join("chosen-\u{1b}[31m-sdk");
    let output = run(&mut f.command(&f.driver,
        &["build", "--profile", "hosted-amiga", "--sdk-root", text(&invalid), "hello.clo"]));
    exit(&output, 2);
    assert!(!output.stderr.contains(&0x1b));
    assert!(String::from_utf8_lossy(&output.stderr).contains("chosen-"),
        "diagnostic must identify selected path, not just reject a flag");
    f.assert_no_processes();
}

/// Invalid native metadata remains a usage/environment error in JSON mode.
#[test]
fn invalid_native_metadata_returns_two_with_valid_ndjson() {
    let f = Fixture::new("metadata-json");
    let install = f.install("candidate", "candidate", "cp310");
    replace(&install.join("installation.json"), "\"schema\":1", "\"schema\":false");
    let output = f.native(&install, "build", &["--diagnostics=json"]);
    exit(&output, 2);
    ndjson(&f, &output);
    assert!(String::from_utf8_lossy(&output.stderr).contains("candidate"));
    f.assert_no_processes();
}
