//! Literal paths, strict receipt tokens and preflight/staging error boundaries.

use super::emulator_fixtures::*;
use super::fixtures::*;
use clc::lsp::json::Json;
use std::path::Path;

/// Native receipt parsing keeps the original JSON token distinctions rather
/// than accepting values only because a map or integer conversion erased them.
#[test]
fn receipt_tokens_reject_escaped_duplicates_and_noncanonical_integers() {
    let f = Fixture::new("receipt-token-boundaries");
    let install = f.install("native", "native", "cp310");
    let valid = descriptor("cp310");
    let cases = [
        valid.replace("\"schema\":1", "\"schema\":01"),
        valid.replace("\"schema\":1", "\"schema\":1,\"sch\\u0065ma\":1"),
        valid.replace("\"schema\":1", "\"schema\":9223372036854775808"),
        valid.replace("\"schema\":1", "\"schema\":1.0"),
        valid.replace("\"schema\":1", "\"schema\":1e0"),
        valid.replace("\"core_version\":\"1.1.2\"", "\"core_version\":\"1.1.2\\u001b\""),
    ];
    for receipt in cases {
        std::fs::write(install.join("installation.json"), receipt).unwrap();
        let output = f.native(&install, "build", &["--diagnostics=json"]);
        exit(&output, 2);
        assert!(!output.stderr.contains(&0x1b));
        let diagnostic = String::from_utf8(output.stderr).unwrap();
        assert!(diagnostic.contains("installation.json"));
        assert!(!diagnostic.trim().is_empty());
        for line in diagnostic.lines() {
            let item = Json::parse(line).unwrap();
            assert_eq!(item.get("severity").and_then(Json::as_str), Some("error"));
        }
        f.assert_no_processes();
    }
}

/// Valid JSON escaping is not itself an invalid selection. Duplicate detection
/// must decode keys without rejecting their otherwise valid spelling.
#[test]
fn an_escaped_unique_receipt_key_remains_valid() {
    let f = Fixture::new("escaped-key");
    let install = f.install("native", "native", "cp312");
    std::fs::write(install.join("installation.json"),
        descriptor("cp312").replace("\"schema\"", "\"sch\\u0065ma\"")).unwrap();
    let output = f.native(&install, "build", &[]);
    exit(&output, 0);
    f.argv("native", "compiler");
}

/// Resolution is relative to the caller once, before native paths are passed
/// to processes or the emulator changes its own working directory.
#[test]
fn a_relative_sdk_path_reaches_the_actual_native_compiler() {
    let f = Fixture::new("relative-sdk");
    let install = f.install("native ; relative", "native", "cp310");
    let output = run(&mut f.command(&f.driver,
        &["run", "--profile", "hosted-amiga", "--sdk-root",
          "../native ; relative/sdk/amiga", "hello.clo"]));
    exit(&output, 0);
    selected(&f, &install, "native");
}

/// The executable and ROM must stay absolute after the child switches to its
/// private CWD. Their spaces and punctuation remain literal path components.
#[test]
fn relative_emulator_and_rom_paths_survive_the_private_working_directory() {
    let e = EmulatorFixture::new("relative-emulator");
    let sdk = e.install.join("sdk/amiga");
    let output = run(&mut e.command(&[
        "run", "--profile", "hosted-amiga", "--sdk-root", text(&sdk),
        "--emulator", "--fs-uae", "../cli emulator ; literal",
        "--kickstart", "../synthetic Kickstart ; literal.rom", "hello.clo",
    ]));
    exit(&output, 0);
    let rom = e.f.cwd.join("../synthetic Kickstart ; literal.rom");
    e.check("cli", &rom);
    let fields = e.fields("cli");
    assert!(Path::new(&fields["executable"]).is_absolute());
    assert!(Path::new(&fields["rom"]).is_absolute());
    cleaned(&fields);
}

/// An explicitly unsafe temporary location cannot put a live mount underneath
/// a later build's reset directory. This error follows a completed compilation.
#[test]
fn staging_rejects_a_temporary_parent_inside_the_resettable_build_directory() {
    let e = EmulatorFixture::new("unsafe-temp-parent");
    let reset = e.f.cwd.join("build/clorraine/hello/hosted-amiga");
    let output = run(e.native("hosted-amiga", &[], "hello.clo").env("TMPDIR", &reset));
    exit(&output, 1);
    assert!(String::from_utf8_lossy(&output.stderr).contains("TMPDIR"));
    e.f.argv("native", "compiler");
    e.assert_no_emulator();
    assert_eq!(std::fs::read_dir(&e.temporary).unwrap().count(), 0);
}

/// Observe the initialization environment at the actual external-process
/// boundary without modifying the frozen observer or asking a real X server.
#[test]
fn initialization_paths_are_private_and_caller_xauthority_keeps_its_meaning() {
    for relative_authority in [false, true] {
        let e = EmulatorFixture::new("initialization-environment");
        let delegate = e.cli.with_file_name("observer-delegate");
        std::fs::rename(&e.cli, &delegate).unwrap();
        executable(&e.cli, r#"#!/usr/bin/python3
"""Record the environment, then execute the unchanged external observer."""
import json
import os
from pathlib import Path
import sys
names = ["HOME", "XDG_DATA_HOME", "XDG_CONFIG_HOME", "XDG_CACHE_HOME",
         "XDG_STATE_HOME", "TMPDIR", "XAUTHORITY"]
if not sys.argv[1].startswith("--fixture-"):
    record = {name: os.environ.get(name) for name in names}
    record["cwd"] = str(Path.cwd())
    (Path(os.environ["TEST_NATIVE_LOG"]) / "initialization.json").write_text(json.dumps(record))
delegate = str(Path(__file__).with_name("observer-delegate"))
os.execv(delegate, [delegate, *sys.argv[1:]])
"#);
        let mut command = e.native("hosted-amiga", &[], "hello.clo");
        let expected_authority = if relative_authority {
            command.env("XAUTHORITY", "../caller-authority");
            e.f.cwd.join("../caller-authority")
        } else {
            command.env_remove("XAUTHORITY");
            e.home.join(".Xauthority")
        };
        let output = run(&mut command);
        exit(&output, 0);
        let fields = e.fields("cli");
        let root = Path::new(&fields["drive"]).parent().unwrap();
        let data = std::fs::read_to_string(e.f.logs.join("initialization.json")).unwrap();
        let record = Json::parse(&data).unwrap();
        for name in ["HOME", "XDG_DATA_HOME", "XDG_CONFIG_HOME", "XDG_CACHE_HOME",
                     "XDG_STATE_HOME", "TMPDIR", "cwd"] {
            let path = Path::new(record.get(name).and_then(Json::as_str).unwrap());
            assert!(path.starts_with(root), "{name}: {}", path.display());
            assert!(!path.starts_with(&fields["drive"]));
        }
        assert_eq!(record.get("XAUTHORITY").and_then(Json::as_str),
            expected_authority.to_str());
        cleaned(&fields);
    }
}
