//! Temporary-parent boundaries checked before any emulator child can start.

use super::emulator_fixtures::*;
use super::emulator_staging::change_compiler_output;
use super::fixtures::*;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::Output;

/// An outside file makes accidental traversal or cleanup observable.
fn canary(parent: &Path) -> PathBuf {
    std::fs::create_dir_all(parent).unwrap();
    let path = parent.join("untouched");
    std::fs::write(&path, b"outside caller bytes\n").unwrap();
    path
}

/// Rejection names the temporary-directory setting and a usable location remedy.
/// Compilation may already have happened, but no emulator may have been started.
fn rejected_parent(e: &EmulatorFixture, output: &Output) {
    e.assert_no_emulator();
    exit(output, 1);
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains("TMPDIR"), "name the unsafe staging setting: {error}");
    let lower = error.to_ascii_lowercase();
    assert!(["safe", "outside", "different", "another"].iter().any(|word| lower.contains(word)),
        "explain how to select a safe temporary location: {error}");
}

/// Commas survive path canonicalisation but cannot survive the emulator's
/// comma-delimited mount representation. An innocent alias must not hide one.
fn comma_parent(profile: &str, alias: bool) {
    let e = EmulatorFixture::new("comma-staging-parent");
    let outside = e.f.root.join("outside");
    let sentinel = canary(&outside);
    let unsafe_parent = e.f.root.join("outside,0");
    std::fs::create_dir(&unsafe_parent).unwrap();
    let selected = if alias {
        let selected = e.f.root.join("innocent temporary alias");
        symlink(&unsafe_parent, &selected).unwrap();
        selected
    } else {
        unsafe_parent.clone()
    };
    assert!(text(&selected.canonicalize().unwrap()).contains(','));
    let output = run(e.native(profile, &[], "hello.clo").env("TMPDIR", &selected));
    assert_eq!(std::fs::read(&sentinel).unwrap(), b"outside caller bytes\n");
    assert_eq!(std::fs::read_dir(&outside).unwrap().count(), 1);
    assert_eq!(std::fs::read_dir(&unsafe_parent).unwrap().count(), 0,
        "unsafe temporary parent must retain no run storage");
    rejected_parent(&e, &output);
}

/// The ordinary hosted profile must not expose a comma-truncated outside drive.
#[test]
fn hosted_amiga_rejects_comma_staging_parent_before_launch() {
    comma_parent("hosted-amiga", false);
}

/// The baseline profile needs the same host-side mount boundary protection.
#[test]
fn hosted_amiga_13_rejects_comma_staging_parent_before_launch() {
    comma_parent("hosted-amiga-13", false);
}

/// A simple alias cannot make a comma-containing canonical parent safe.
#[test]
fn hosted_amiga_rejects_alias_to_comma_staging_parent() {
    comma_parent("hosted-amiga", true);
}

/// Canonical validation also applies to the baseline profile's temporary parent.
#[test]
fn hosted_amiga_13_rejects_alias_to_comma_staging_parent() {
    comma_parent("hosted-amiga-13", true);
}

/// Produces real build artifacts through the public driver, using the existing
/// external compiler double rather than fabricating an artifact directory.
fn build_other(e: &EmulatorFixture, profile: &str) -> PathBuf {
    let sdk = e.install.join("sdk/amiga");
    let output = run(&mut e.command(&[
        "build", "--profile", profile, "--sdk-root", text(&sdk), "other.clo",
    ]));
    exit(&output, 0);
    let program = e.f.cwd.join("build/clorraine/other").join(profile).join("other");
    assert!(program.is_file(), "the other program must actually have been built");
    program
}

/// Another source/profile's directory is resettable while this invocation runs.
/// Reject it even through an alias; then rebuild it and preserve outside files.
fn other_artifact_parent(profile: &str, other_profile: &str, alias: bool) {
    let e = EmulatorFixture::new("other-build-staging-parent");
    std::fs::write(e.f.cwd.join("other.clo"), source(0)).unwrap();
    let other = build_other(&e, other_profile);
    let original = std::fs::read(&other).unwrap();
    let sentinel = canary(&e.f.root.join("outside artifacts"));
    let selected = if alias {
        let selected = e.f.root.join("temporary alias");
        symlink(other.parent().unwrap(), &selected).unwrap();
        selected
    } else {
        other.parent().unwrap().to_owned()
    };
    let artifact_root = e.f.cwd.join("build/clorraine").canonicalize().unwrap();
    assert!(selected.canonicalize().unwrap().starts_with(&artifact_root));
    e.f.clear_logs();
    let output = run(e.native(profile, &[], "hello.clo").env("TMPDIR", &selected));

    // A later build demonstrably changes this directory's executable. Neither
    // that reset nor rejection may write to or remove the outside canary.
    change_compiler_output(&e);
    let rebuilt = build_other(&e, other_profile);
    assert_ne!(std::fs::read(&rebuilt).unwrap(), original);
    assert_eq!(std::fs::read(&sentinel).unwrap(), b"outside caller bytes\n");
    assert_eq!(std::fs::read_dir(sentinel.parent().unwrap()).unwrap().count(), 1);
    rejected_parent(&e, &output);
}

/// A different source in the same profile is not safe temporary storage.
#[test]
fn hosted_amiga_rejects_another_sources_build_directory() {
    other_artifact_parent("hosted-amiga", "hosted-amiga", false);
}

/// Profile separation does not protect a staging directory from another build.
#[test]
fn hosted_amiga_13_rejects_another_profiles_build_directory() {
    other_artifact_parent("hosted-amiga-13", "hosted-amiga", false);
}

/// Canonical artifact-tree containment must survive a benign-looking alias.
#[test]
fn hosted_amiga_rejects_alias_to_another_profiles_build_directory() {
    other_artifact_parent("hosted-amiga", "hosted-amiga-13", true);
}

/// Baseline runs must reject aliased sibling-source artifact directories too.
#[test]
fn hosted_amiga_13_rejects_alias_to_another_sources_build_directory() {
    other_artifact_parent("hosted-amiga-13", "hosted-amiga-13", true);
}

/// Safe temporary locations remain literal, including sibling names sharing the
/// artifact root's prefix. Containment is by path component, not string prefix.
fn safe_parent(profile: &str) {
    let e = EmulatorFixture::new("safe-staging-parent");
    let sibling = e.f.cwd.join("build/clorraine-safe");
    std::fs::create_dir_all(&sibling).unwrap();
    for parent in [&e.temporary, &sibling] {
        let sentinel = canary(parent);
        let output = run(e.native(profile, &[], "hello.clo").env("TMPDIR", parent));
        exit(&output, 0);
        e.check("cli", &e.rom);
        let fields = e.fields("cli");
        assert!(Path::new(&fields["drive"]).starts_with(parent));
        cleaned(&fields);
        assert_eq!(std::fs::read(&sentinel).unwrap(), b"outside caller bytes\n");
        assert_eq!(std::fs::read_dir(parent).unwrap().count(), 1);
    }
}

/// Ordinary and similarly prefixed sibling parents remain supported.
#[test]
fn hosted_amiga_preserves_safe_temporary_parent_controls() {
    safe_parent("hosted-amiga");
}

/// Safe-parent compatibility is independent of the selected Amiga runtime.
#[test]
fn hosted_amiga_13_preserves_safe_temporary_parent_controls() {
    safe_parent("hosted-amiga-13");
}
