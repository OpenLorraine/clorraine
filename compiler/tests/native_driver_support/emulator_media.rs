//! Private local ROM copies for filenames that an emulator cannot preserve literally.
//!
//! Synthetic bytes exercise process ownership only. Actual licensed-ROM boots and
//! version-specific emulator path handling require separate acceptance evidence.

use super::emulator_fixtures::*;
use super::emulator_staging::staged_program;
use super::fixtures::*;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Distinct binary test data, not licensed firmware or a usable Kickstart image.
const ROM_BYTES: &[u8] = b"synthetic trailing-space ROM\0\xff\n";

/// Create the exceptional filename without modifying the fixture's ordinary ROM.
fn selected_rom(e: &EmulatorFixture) -> PathBuf {
    let path = e.f.root.join("selected ROM ; literal.rom ");
    std::fs::write(&path, ROM_BYTES).unwrap();
    assert!(text(&path).ends_with(' '));
    path
}

/// Start the public native command with literal paths and its existing join guard.
fn start_media_run(
    e: &EmulatorFixture,
    profile: &str,
    emulator: &Path,
    rom: &Path,
    source: &str,
    identity: &str,
) -> WaitingRun {
    let sdk = e.install.join("sdk/amiga");
    let command = e.command(
        &["run", "--profile", profile, "--sdk-root", text(&sdk),
          "--emulator", "--fs-uae", text(emulator), "--kickstart", text(rom), source],
    );
    WaitingRun::start(e, command, identity)
}

/// Require the selected file to survive both staging and cleanup unchanged.
fn original_unchanged(original: &Path) {
    assert!(original.symlink_metadata().unwrap().file_type().is_file());
    assert_eq!(std::fs::read(original).unwrap(), ROM_BYTES);
}

/// Inspect the observer's effective ROM rather than assuming a media basename.
///
/// Both lexical and canonical containment matter: an apparently private path
/// must not redirect into mounted files or caller-owned storage through a link.
/// The unchanged observer checker still validates all other handoff settings.
fn private_rom(
    e: &EmulatorFixture,
    identity: &str,
    fields: &BTreeMap<String, String>,
    original: &Path,
) -> PathBuf {
    let rom = PathBuf::from(&fields["rom"]);
    let drive = Path::new(&fields["drive"]);
    let root = drive.parent().unwrap();
    assert_ne!(rom, original, "the unrepresentable name needs a private copy");
    assert!(rom.is_absolute());
    assert!(rom.starts_with(root), "ROM copy belongs to this run");
    assert!(!rom.starts_with(drive), "ROM copy is never guest-mounted");
    assert!(!text(&rom).ends_with(' '), "the effective path must survive option trimming");
    assert!(!text(&rom).chars().any(|character| character.is_control()));
    assert!(rom.symlink_metadata().unwrap().file_type().is_file(),
        "the private ROM must be an ordinary file, not an alias");
    let canonical = rom.canonicalize().unwrap();
    assert!(canonical.starts_with(root.canonicalize().unwrap()));
    assert!(!canonical.starts_with(drive.canonicalize().unwrap()));
    assert_eq!(std::fs::read(&rom).unwrap(), ROM_BYTES);
    original_unchanged(original);
    e.check(identity, &rom);
    rom
}

/// Check removal only after the owned process has been released and joined.
fn media_cleaned(fields: &BTreeMap<String, String>, rom: &Path, original: &Path) {
    assert!(!rom.exists() && !rom.is_symlink(), "the stopped run's ROM copy remains");
    cleaned(fields);
    let root = Path::new(&fields["drive"]).parent().unwrap();
    assert!(!root.exists(), "the stopped run's private storage remains");
    original_unchanged(original);
}

/// Both profiles keep a safe unmounted copy until their own emulator stops.
#[test]
fn trailing_space_rom_uses_private_exact_copy_until_child_stops() {
    for profile in ["hosted-amiga", "hosted-amiga-13"] {
        let e = EmulatorFixture::new("trailing-rom-lifetime");
        let original = selected_rom(&e);
        let mut waiting = start_media_run(&e, profile, &e.cli, &original, "hello.clo", "cli");
        waiting.ready();
        waiting.assert_waiting();
        let fields = e.fields("cli");
        let rom = private_rom(&e, "cli", &fields, &original);
        let program = e.f.cwd.join("build/clorraine/hello").join(profile).join("hello");
        e.check_copy("cli", &rom, &program);
        assert_eq!(fields["executable"], text(&e.cli));
        assert!(Path::new(&fields["config"]).is_file());
        assert!(staged_program(&fields).is_file());

        // Keep the child live across the byte checks; no early-returning driver
        // may satisfy the eventual cleanup assertions by deleting storage first.
        waiting.assert_waiting();
        assert_eq!(std::fs::read(&rom).unwrap(), std::fs::read(&original).unwrap());
        let output = waiting.finish();
        exit(&output, 0);
        media_cleaned(&fields, &rom, &original);
    }
}

/// Two runs selecting one source ROM must own independent copies and lifetimes.
#[test]
fn overlapping_runs_with_same_trailing_space_rom_keep_private_copies_independent() {
    for profile in ["hosted-amiga", "hosted-amiga-13"] {
        let e = EmulatorFixture::new("overlapping-trailing-rom");
        let original = selected_rom(&e);
        std::fs::write(e.f.cwd.join("second.clo"), source(0)).unwrap();
        let mut first = start_media_run(&e, profile, &e.cli, &original, "hello.clo", "cli");
        first.ready();
        first.assert_waiting();
        let first_fields = e.fields("cli");
        let first_rom = private_rom(&e, "cli", &first_fields, &original);

        // A separate source produces a completed second build without resetting
        // the first build directory while its mounted copy is still in use.
        let second_tool = e.tool("second");
        let mut second = start_media_run(
            &e, profile, &second_tool, &original, "second.clo", "second",
        );
        second.ready();
        second.assert_waiting();
        first.assert_waiting();
        let second_fields = e.fields("second");
        let second_rom = private_rom(&e, "second", &second_fields, &original);
        assert_eq!(first_fields["executable"], text(&e.cli));
        assert_eq!(second_fields["executable"], text(&second_tool));
        assert_ne!(first_rom, second_rom, "runs cannot share a ROM copy");
        for key in ["drive", "config"] {
            assert_ne!(first_fields[key], second_fields[key], "{key} cannot be shared");
        }
        assert_ne!(
            Path::new(&first_fields["drive"]).parent(),
            Path::new(&second_fields["drive"]).parent(),
            "each media copy needs its own run storage",
        );
        let first_program = e.f.cwd.join("build/clorraine/hello").join(profile).join("hello");
        let second_program = e.f.cwd.join("build/clorraine/second").join(profile).join("second");
        e.check_copy("cli", &first_rom, &first_program);
        e.check_copy("second", &second_rom, &second_program);
        assert_eq!(private_rom(&e, "cli", &first_fields, &original), first_rom);
        let config_before = std::fs::read(&second_fields["config"]).unwrap();
        let mounted_before = std::fs::read(staged_program(&second_fields)).unwrap();
        let startup = Path::new(&second_fields["drive"]).join("s/startup-sequence");
        let startup_before = std::fs::read(&startup).unwrap();

        let output = first.finish();
        exit(&output, 0);
        media_cleaned(&first_fields, &first_rom, &original);
        second.assert_waiting();
        assert_eq!(private_rom(&e, "second", &second_fields, &original), second_rom);
        assert!(Path::new(&second_fields["drive"]).is_dir());
        assert_eq!(std::fs::read(&second_fields["config"]).unwrap(), config_before);
        assert_eq!(std::fs::read(staged_program(&second_fields)).unwrap(), mounted_before);
        assert_eq!(std::fs::read(&startup).unwrap(), startup_before);
        e.check_copy("second", &second_rom, &second_program);

        let output = second.finish();
        exit(&output, 0);
        media_cleaned(&second_fields, &second_rom, &original);
    }
}
