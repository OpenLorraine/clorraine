//! Waiting, process results and ownership of concurrent emulator staging.

use super::emulator_fixtures::*;
use super::emulator_staging::*;
use super::fixtures::*;
use std::path::Path;

/// A child held at its external barrier keeps all staged files live. Its own
/// eventual process status, rather than a guessed guest status, is returned.
fn waiting_status(status: i32) {
    let e = EmulatorFixture::new("emulator-waits");
    let mut command = e.native("hosted-amiga", &[], "hello.clo");
    command.env("TEST_EMULATOR_STATUS", status.to_string());
    let mut waiting = WaitingRun::start(&e, command, "cli");
    waiting.ready();
    waiting.assert_waiting();
    e.check("cli", &e.rom);
    let fields = e.fields("cli");
    assert!(staged_program(&fields).is_file());
    assert!(Path::new(&fields["config"]).is_file());
    let output = waiting.finish();
    exit(&output, status);
    e.check("cli", &e.rom);
    cleaned(&fields);
}

/// A zero emulator process code is returned only after the process stops.
#[test]
fn driver_waits_for_emulator_zero_and_cleans_after_child_exit() {
    waiting_status(0);
}

/// A nonzero emulator process code is returned unchanged after the same wait.
#[test]
fn driver_waits_for_emulator_seven_and_returns_exact_process_status() {
    waiting_status(7);
}

/// Signal termination has no process exit code and must become an emulator
/// failure, with cleanup occurring only after the child has stopped.
#[test]
fn signalled_emulator_returns_one_and_cleans_after_child_stops() {
    for json in [false, true] {
        let e = EmulatorFixture::new("emulator-signal");
        let extra: &[&str] = if json { &["--diagnostics=json"] } else { &[] };
        let mut command = e.native("hosted-amiga", extra, "hello.clo");
        command.env("TEST_EMULATOR_SIGNAL", "1");
        let mut waiting = WaitingRun::start(&e, command, "cli");
        waiting.ready();
        waiting.assert_waiting();
        let fields = e.fields("cli");
        let output = waiting.finish();
        exit(&output, 1);
        let error = String::from_utf8_lossy(&output.stderr).to_ascii_lowercase();
        assert!(error.contains("emulator") || error.contains("fs-uae"),
            "signal failure must identify the emulator: {error}");
        if json {
            e.check_error_json(&output);
        }
        e.check("cli", &e.rom);
        cleaned(&fields);
    }
}

/// Compiler success invalidates the selected executable after preflight. The
/// driver must report launch failure and remove the staging it just created.
#[test]
fn emulator_launch_failure_after_build_returns_one_without_staging_leak() {
    for json in [false, true] {
        let e = EmulatorFixture::new("emulator-launch-failure");
        break_emulator_after_compilation(&e);
        let extra: &[&str] = if json { &["--diagnostics=json"] } else { &[] };
        let output = run(e.native("hosted-amiga", extra, "hello.clo")
            .env("TEST_EMULATOR_BREAK_PATH", &e.cli));
        exit(&output, 1);
        let error = String::from_utf8_lossy(&output.stderr).to_ascii_lowercase();
        assert!(error.contains("emulator") || error.contains("fs-uae"),
            "failed launch must name the emulator: {error}");
        if json {
            e.check_error_json(&output);
        }
        e.f.argv("native", "compiler");
        e.assert_no_emulator();
        assert_eq!(std::fs::read_dir(&e.temporary).unwrap().count(), 0,
            "failed child launch must not leave private run storage");
    }
}

/// Separate completed builds produce private mounted copies and independent
/// writable scopes. Rebuilding the first source must not change its live copy,
/// and stopping the second emulator must not clean the first emulator's files.
#[test]
fn overlapping_runs_and_same_source_rebuild_keep_staging_and_state_independent() {
    let e = EmulatorFixture::new("overlapping-emulators");
    std::fs::write(e.f.cwd.join("second.clo"), source(0)).unwrap();
    let mut first = WaitingRun::start(&e, e.native("hosted-amiga", &[], "hello.clo"), "cli");
    first.ready();
    first.assert_waiting();
    e.check("cli", &e.rom);
    let first_fields = e.fields("cli");
    let mounted_before = std::fs::read(staged_program(&first_fields)).unwrap();
    let config_before = std::fs::read(&first_fields["config"]).unwrap();

    let second_tool = e.tool("second");
    let sdk = e.install.join("sdk/amiga");
    let command = e.command(
        &["run", "--profile", "hosted-amiga", "--sdk-root", text(&sdk),
          "--emulator", "--fs-uae", text(&second_tool), "--kickstart", text(&e.rom), "second.clo"]);
    let mut second = WaitingRun::start(&e, command, "second");
    second.ready();
    second.assert_waiting();
    e.check("second", &e.rom);
    let second_fields = e.fields("second");
    for key in ["drive", "config", "cwd", "base", "cache", "state", "logs"] {
        assert_ne!(first_fields[key], second_fields[key], "{key} cannot be shared between runs");
    }
    assert_eq!(first_fields["guest"], second_fields["guest"],
        "both source stems use the same fixed guest filename");

    change_compiler_output(&e);
    let rebuild = run(&mut e.command(
        &["build", "--profile", "hosted-amiga", "--sdk-root", text(&sdk), "hello.clo"]));
    exit(&rebuild, 0);
    let rebuilt = e.f.cwd.join("build/clorraine/hello/hosted-amiga/hello");
    assert_ne!(std::fs::read(rebuilt).unwrap(), mounted_before,
        "the rebuild control must actually change executable bytes");
    assert_eq!(std::fs::read(staged_program(&first_fields)).unwrap(), mounted_before);
    assert_eq!(std::fs::read(&first_fields["config"]).unwrap(), config_before);
    first.assert_waiting();
    second.assert_waiting();

    let output = second.finish();
    exit(&output, 0);
    e.check("second", &e.rom);
    cleaned(&second_fields);
    assert!(staged_program(&first_fields).is_file(), "first mounted program must remain");
    assert!(Path::new(&first_fields["config"]).is_file(), "first config must remain");
    first.assert_waiting();

    let output = first.finish();
    exit(&output, 0);
    e.check("cli", &e.rom);
    cleaned(&first_fields);
}
