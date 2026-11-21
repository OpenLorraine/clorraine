//! Current-program staging protection when an artifact child redirects outside.

use super::emulator_fixtures::*;
use super::emulator_staging::change_compiler_output;
use super::fixtures::*;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};

/// Build through the public command so an external compiler actually produces
/// the selected profile's output, rather than fabricating a resettable directory.
fn build_hello(e: &EmulatorFixture, profile: &str) -> PathBuf {
    let sdk = e.install.join("sdk/amiga");
    let output = run(&mut e.command(&[
        "build", "--profile", profile, "--sdk-root", text(&sdk), "hello.clo",
    ]));
    exit(&output, 0);
    let program = e.f.cwd.join("build/clorraine/hello").join(profile).join("hello");
    assert!(program.is_file(), "public build must produce the aliased executable");
    program.canonicalize().unwrap()
}

/// Redirect only the current source's artifact child. The workspace artifact
/// root stays local, while the actual program directory becomes external to it.
fn aliased_program(e: &EmulatorFixture, profile: &str) -> PathBuf {
    let artifacts = e.f.cwd.join("build/clorraine");
    let external = e.f.root.join("external program artifacts");
    std::fs::create_dir_all(&artifacts).unwrap();
    std::fs::create_dir(&external).unwrap();
    symlink(&external, artifacts.join("hello")).unwrap();
    let program = build_hello(e, profile);
    let canonical_root = artifacts.canonicalize().unwrap();
    let output_directory = program.parent().unwrap();
    assert_eq!(output_directory, external.join(profile).canonicalize().unwrap());
    assert!(!output_directory.starts_with(&canonical_root),
        "the fixture must escape canonical artifact-root containment");
    assert!(std::fs::symlink_metadata(artifacts.join("hello")).unwrap().file_type().is_symlink());
    program
}

/// Keep caller bytes beside, but not inside, the directory a rebuild resets.
/// The second file is unrelated to both the artifact alias and temporary parent.
fn canaries(e: &EmulatorFixture, program: &Path) -> Vec<PathBuf> {
    let paths = vec![
        program.parent().unwrap().parent().unwrap().join("keep outside profile"),
        e.f.root.join("unrelated caller file"),
    ];
    for path in &paths {
        std::fs::write(path, b"caller bytes must survive\n").unwrap();
    }
    paths
}

/// Rejection, rebuild and emulator cleanup must leave caller files unchanged.
fn unchanged(paths: &[PathBuf]) {
    for path in paths {
        assert_eq!(std::fs::read(path).unwrap(), b"caller bytes must survive\n",
            "caller file changed: {}", path.display());
    }
}

/// A canonical current-program guard must coexist with the workspace-root
/// guard. An artifact-child alias can put the resettable output beyond that root.
fn reject_aliased_output(profile: &str) {
    let e = EmulatorFixture::new("reject-aliased-program-output");
    let program = aliased_program(&e, profile);
    let original = std::fs::read(&program).unwrap();
    let protected = canaries(&e, &program);
    let temporary = program.parent().unwrap().to_owned();
    assert!(temporary.canonicalize().unwrap().starts_with(program.parent().unwrap()),
        "selected TMPDIR must lie in the actual current program output");
    e.f.clear_logs();
    let output = run(e.native(profile, &[], "hello.clo").env("TMPDIR", &temporary));
    unchanged(&protected);

    // Exercise the same public reset that makes this parent unsuitable for
    // live emulator storage, and prove the rebuilt executable really changes.
    change_compiler_output(&e);
    let rebuilt = build_hello(&e, profile);
    assert_eq!(rebuilt, program, "the rebuild must use the same external output");
    assert_ne!(std::fs::read(&rebuilt).unwrap(), original);
    unchanged(&protected);

    e.assert_no_emulator();
    exit(&output, 1);
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains("TMPDIR"), "identify the rejected temporary setting: {error}");
    let lower = error.to_ascii_lowercase();
    assert!(["safe", "outside", "different", "another"].iter().any(|word| lower.contains(word)),
        "give a safe temporary-location remedy: {error}");
}

/// The regular Amiga profile must reject its aliased canonical output as TMPDIR.
#[test]
fn hosted_amiga_rejects_staging_in_aliased_current_output() {
    reject_aliased_output("hosted-amiga");
}

/// The baseline profile retains the same current-program lifetime protection.
#[test]
fn hosted_amiga_13_rejects_staging_in_aliased_current_output() {
    reject_aliased_output("hosted-amiga-13");
}

/// An artifact alias alone is not an error. Safe external temporary storage
/// still stages the actual built bytes and is removed after the child exits.
fn safe_staging_with_alias(profile: &str) {
    let e = EmulatorFixture::new("safe-staging-aliased-program-output");
    let program = aliased_program(&e, profile);
    let protected = canaries(&e, &program);
    let output = run(&mut e.native(profile, &[], "hello.clo"));
    exit(&output, 0);
    e.check_copy("cli", &e.rom, &program);
    let fields = e.fields("cli");
    let drive = Path::new(&fields["drive"]);
    assert!(drive.starts_with(&e.temporary));
    assert!(!drive.starts_with(program.parent().unwrap()));
    assert!(!drive.starts_with(e.f.cwd.join("build/clorraine").canonicalize().unwrap()));
    cleaned(&fields);
    unchanged(&protected);
}

/// Safe temporary parents remain usable with redirected regular-profile output.
#[test]
fn hosted_amiga_allows_safe_staging_with_aliased_current_output() {
    safe_staging_with_alias("hosted-amiga");
}

/// Redirected baseline output also keeps ordinary temporary-parent behavior.
#[test]
fn hosted_amiga_13_allows_safe_staging_with_aliased_current_output() {
    safe_staging_with_alias("hosted-amiga-13");
}
