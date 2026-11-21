//! Literal paths and declarative isolation at the emulator process boundary.

use super::emulator_fixtures::*;
use super::fixtures::*;
use std::path::{Path, PathBuf};

/// Both existing Amiga profiles use the approved native directory-drive recipe.
/// This checks staged settings, never a guest boot or FS-UAE's effective state.
#[test]
fn both_profiles_stage_private_a1200_directory_drive_settings() {
    for profile in ["hosted-amiga", "hosted-amiga-13"] {
        let e = EmulatorFixture::new("emulator-recipe");
        std::fs::remove_dir_all(e.install.join("sdk/vamos")).unwrap();
        let output = run(&mut e.native(profile, &[], "hello.clo"));
        exit(&output, 0);
        let program = e.f.cwd.join("build/clorraine/hello").join(profile).join("hello");
        e.check_copy("cli", &e.rom, &program);
        let fields = e.fields("cli");
        cleaned(&fields);
        let compiler = e.f.argv("native", "compiler");
        let runtime = if profile == "hosted-amiga" { "-mcrt=nix20" } else { "-mcrt=nix13" };
        assert!(compiler.iter().any(|argument| argument == runtime),
            "emulator choice must not change the compiled profile");
        assert!(!e.f.logs.join("native-runner.argv").exists(), "no vamos required");
        assert!(!e.f.logs.join("legacy-runner.argv").exists());
    }
}

/// Creates hostile caller configs without reading or editing real user files.
/// Returned snapshots let tests detect both rewriting and outside-drive writes.
fn hostile_settings(e: &EmulatorFixture) -> Vec<(PathBuf, Vec<u8>)> {
    let outside = e.f.root.join("protected outside drive");
    std::fs::create_dir_all(&outside).unwrap();
    let sentinel = outside.join("untouched");
    std::fs::write(&sentinel, b"protected outside contents\n").unwrap();
    let contents = format!(
        "[fs-uae]\namiga_model=A500\nhard_drive_1={}\n\
         hard_drive_0_priority=127\nbase_dir={}\nend_config=0\n",
        outside.display(), outside.display()
    );
    let mut files = vec![(sentinel, b"protected outside contents\n".to_vec())];
    for path in [
        e.f.cwd.join("Config.fs-uae"),
        e.home.join("FS-UAE/Configurations/Default.fs-uae"),
        e.home.join("FS-UAE/Configurations/Host.fs-uae"),
        e.home.join("Documents/FS-UAE/Configurations/Default.fs-uae"),
        e.home.join("Documents/FS-UAE/Configurations/Host.fs-uae"),
    ] {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, contents.as_bytes()).unwrap();
        files.push((path, contents.as_bytes().to_vec()));
    }
    let base_file = e.home.join(".config/fs-uae/base-dir");
    std::fs::create_dir_all(base_file.parent().unwrap()).unwrap();
    let bytes = format!("{}\n", outside.display()).into_bytes();
    std::fs::write(&base_file, &bytes).unwrap();
    files.push((base_file, bytes));
    files
}

/// Explicit termination of configuration loading and private initialization
/// controls must coexist with hostile inherited settings, not rewrite them.
/// Whether real FS-UAE honors these controls remains a separate acceptance gate.
#[test]
fn hostile_caller_configs_are_not_mounted_or_rewritten_in_generated_handoff() {
    let e = EmulatorFixture::new("hostile-emulator-configs");
    let snapshots = hostile_settings(&e);
    let output = run(&mut e.native("hosted-amiga", &[], "hello.clo"));
    exit(&output, 0);
    e.check("cli", &e.rom);
    for (path, bytes) in snapshots {
        assert_eq!(std::fs::read(&path).unwrap(), bytes,
            "caller-owned config or sentinel changed: {}", path.display());
    }
    let outside = e.f.root.join("protected outside drive");
    assert_eq!(std::fs::read_dir(outside).unwrap().count(), 1,
        "no private state may be written to the outside drive");
    cleaned(&e.fields("cli"));
}

/// Host source, SDK and ROM punctuation remains data. Guest startup uses one
/// fixed filename across different source stems and never host shell fragments.
#[test]
fn metacharacter_paths_stay_literal_and_guest_name_is_independent_of_source() {
    let e = EmulatorFixture::new("literal-emulator-paths");
    let rom = e.f.root.join("ROM ; $HOME = # literal.rom");
    std::fs::write(&rom, b"different literal-path ROM\n").unwrap();
    let sdk = e.install.join("sdk/amiga");
    let mut guest_names = Vec::new();
    for source in ["first ; $(touch INJECTION).clo", "second & 'quoted' name.clo"] {
        std::fs::write(e.f.cwd.join(source), super::fixtures::source(0)).unwrap();
        let output = run(&mut e.command(
            &["run", "--profile", "hosted-amiga", "--sdk-root", text(&sdk),
              "--emulator", "--fs-uae", text(&e.cli), "--kickstart", text(&rom), source]));
        exit(&output, 0);
        let stem = Path::new(source).file_stem().unwrap();
        let program = e.f.cwd.join("build/clorraine").join(stem).join("hosted-amiga").join(stem);
        e.check_copy("cli", &rom, &program);
        let fields = e.fields("cli");
        assert!(!fields["startup"].contains(source));
        assert!(!fields["startup"].contains(text(&e.install)));
        guest_names.push(fields["guest"].clone());
        assert!(!e.f.cwd.join("INJECTION").exists(), "no shell expansion");
        assert_eq!(fields["rom"], text(&rom));
        cleaned(&fields);
    }
    assert_eq!(guest_names[0], guest_names[1], "guest filename is fixed, not source-derived");
}

/// A readable path with configuration-control bytes is still unsafe to encode
/// as a ROM setting. Reject it without executing the emulator or echoing escape
/// bytes into the terminal.
#[test]
fn rom_control_bytes_are_rejected_before_emulator_launch() {
    for control in ['\n', '\r', '\t', '\u{1b}'] {
        let e = EmulatorFixture::new("unsafe-rom-path");
        let rom = e.f.root.join(format!("unsafe-rom-{control}-injection.rom"));
        std::fs::write(&rom, b"readable synthetic ROM\n").unwrap();
        let sdk = e.install.join("sdk/amiga");
        let output = run(&mut e.command(
            &["run", "--profile", "hosted-amiga", "--sdk-root", text(&sdk),
              "--emulator", "--fs-uae", text(&e.cli), "--kickstart", text(&rom), "hello.clo"]));
        exit(&output, 2);
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(error.contains("unsafe-rom") || error.contains("--kickstart")
            || error.to_ascii_lowercase().contains("control"),
            "unsafe ROM input must be identified: {error}");
        assert!(!error.contains("unknown option"));
        assert!(!output.stderr.contains(&0x1b), "no raw terminal escape");
        e.assert_no_emulator();
        assert_eq!(std::fs::read_dir(&e.temporary).unwrap().count(), 0,
            "rejected encoding must leave no run storage");
    }
}

/// Installs a compiler-only wrapper that changes the next successful build's
/// bytes. This is an external tool fixture, not a driver pause or build hook.
pub(super) fn change_compiler_output(e: &EmulatorFixture) {
    wrap_compiler(e, r#"
if "-o" in sys.argv[1:]:
    output = Path(sys.argv[sys.argv.index("-o") + 1])
    output.write_bytes(b"different rebuilt executable bytes\n")
    output.chmod(0o755)
"#);
}

/// Makes an executable vanish into a non-launchable payload only after compiler
/// success, so the launch-failure test cannot merely pass on early preflight.
pub(super) fn break_emulator_after_compilation(e: &EmulatorFixture) {
    wrap_compiler(e, r#"
if "-o" in sys.argv[1:]:
    Path(os.environ["TEST_EMULATOR_BREAK_PATH"]).write_bytes(b"not an executable image\n")
"#);
}

/// Delegates to the original tiny compiler before a narrowly chosen fixture
/// action. All action text is static test code; paths remain argv/environment.
fn wrap_compiler(e: &EmulatorFixture, action: &str) {
    let compiler = e.install.join("sdk/amiga/bin/m68k-amigaos-gcc");
    let delegate = compiler.with_file_name("compiler-delegate");
    std::fs::rename(&compiler, &delegate).unwrap();
    let prefix = r#"#!/usr/bin/python3
"""Delegate an external compiler fixture, then alter controlled test data."""
import os
from pathlib import Path
import subprocess
import sys
delegate = Path(sys.argv[0]).with_name("compiler-delegate")
result = subprocess.run([str(delegate), *sys.argv[1:]], check=False)
if result.returncode:
    raise SystemExit(result.returncode)
"#;
    executable(&compiler, &format!("{prefix}{action}"));
}

/// The run's copy is inspected through its observed guest drive, never by
/// guessing the driver's internal temporary-directory name.
pub(super) fn staged_program(fields: &std::collections::BTreeMap<String, String>) -> PathBuf {
    Path::new(&fields["drive"]).join(&fields["guest"])
}
