//! Native-emulator applicability, precedence and missing-input diagnostics.

use super::emulator_fixtures::*;
use super::fixtures::*;
use std::os::unix::fs::PermissionsExt;

/// Native capture is unsupported for either existing Amiga profile and must be
/// rejected before any compiler or emulator process starts.
#[test]
fn native_emulator_capture_is_rejected_before_build_for_both_profiles() {
    for profile in ["hosted-amiga", "hosted-amiga-13"] {
        for json in [false, true] {
            let e = EmulatorFixture::new("emulator-capture");
            let extra: &[&str] = if json {
                &["--capture", "--diagnostics=json"]
            } else {
                &["--capture"]
            };
            let output = run(&mut e.native(profile, extra, "hello.clo"));
            usage_rejection(&e, &output, "--capture");
            if json {
                e.check_error_json(&output);
            }
        }
    }
}

/// Generates focused applicability cases without modifying a shared environment.
macro_rules! inapplicable {
    ($name:ident, $action:literal, $profile:literal, $flag:literal) => {
        /// An explicit option for an unused tool is a semantic usage error.
        #[test]
        fn $name() {
            let e = EmulatorFixture::new(stringify!($name));
            let value = if $flag == "--sdk-root" {
                e.install.join("sdk/amiga")
            } else if $flag == "--fs-uae" {
                e.cli.clone()
            } else {
                e.rom.clone()
            };
            let output = run(&mut e.command(
                &[$action, "--profile", $profile, $flag, text(&value), "hello.clo"]));
            usage_rejection(&e, &output, $flag);
        }
    };
}

inapplicable!(host_build_rejects_sdk_option, "build", "host", "--sdk-root");
inapplicable!(host_run_rejects_sdk_option, "run", "host", "--sdk-root");
inapplicable!(emit_rejects_sdk_option, "emit", "hosted-amiga", "--sdk-root");
inapplicable!(host_run_rejects_emulator_option, "run", "host", "--fs-uae");
inapplicable!(host_run_rejects_rom_option, "run", "host", "--kickstart");
inapplicable!(amiga_build_rejects_emulator_option, "build", "hosted-amiga", "--fs-uae");
inapplicable!(amiga_build_rejects_rom_option, "build", "hosted-amiga", "--kickstart");
inapplicable!(emit_rejects_emulator_option, "emit", "hosted-amiga", "--fs-uae");
inapplicable!(emit_rejects_rom_option, "emit", "hosted-amiga", "--kickstart");
inapplicable!(freestanding_run_rejects_emulator_option, "run", "freestanding", "--fs-uae");
inapplicable!(freestanding_run_rejects_rom_option, "run", "freestanding", "--kickstart");
inapplicable!(amiga_run_without_emulator_rejects_emulator_option, "run", "hosted-amiga", "--fs-uae");
inapplicable!(amiga_run_without_emulator_rejects_rom_option, "run", "hosted-amiga", "--kickstart");

/// Each native-tool path flag must diagnose its own missing value rather than being
/// mistaken for an unknown option or a source file.
macro_rules! missing_value {
    ($name:ident, $flag:literal) => {
        /// A missing path is a usage error with the incomplete option named.
        #[test]
        fn $name() {
            let e = EmulatorFixture::new(stringify!($name));
            let output = run(&mut e.command(&["run", "--profile", "hosted-amiga", $flag]));
            usage_rejection(&e, &output, $flag);
        }
    };
}

missing_value!(sdk_option_requires_a_value, "--sdk-root");
missing_value!(emulator_option_requires_a_value, "--fs-uae");
missing_value!(rom_option_requires_a_value, "--kickstart");

/// Explicit emulator/media flags cannot silently switch the legacy mode.
/// Capture and both profiles retain this pre-build native-SDK remedy.
fn legacy_explicit(flags: &[&str]) {
    for profile in ["hosted-amiga", "hosted-amiga-13"] {
        for capture in [false, true] {
            let e = EmulatorFixture::new("legacy-explicit-emulator");
            let mut args = vec!["run", "--profile", profile, "--emulator"];
            for flag in flags {
                args.push(flag);
                args.push(if *flag == "--fs-uae" { text(&e.cli) } else { text(&e.rom) });
            }
            if capture {
                args.push("--capture");
            }
            args.push("hello.clo");
            let output = run(&mut e.command(&args));
            exit(&output, 2);
            let message = String::from_utf8_lossy(&output.stderr);
            assert!(message.to_ascii_lowercase().contains("native"), "{message}");
            assert!(message.contains("--sdk-root") || message.contains("CLORRAINE_SDK"), "{message}");
            e.f.assert_no_processes();
        }
    }
}

/// Explicit FS-UAE selection in legacy mode needs a native SDK, not a build.
#[test]
fn legacy_explicit_emulator_flag_requires_native_sdk_before_build() {
    legacy_explicit(&["--fs-uae"]);
}

/// Explicit ROM selection in legacy mode needs a native SDK, not a build.
#[test]
fn legacy_explicit_rom_flag_requires_native_sdk_before_build() {
    legacy_explicit(&["--kickstart"]);
}

/// Combining both emulator options does not bypass legacy-mode validation.
#[test]
fn legacy_explicit_emulator_and_rom_flags_require_native_sdk_before_build() {
    legacy_explicit(&["--fs-uae", "--kickstart"]);
}

/// Actual executable and ROM observations prove CLI precedence. Invalid unused
/// environment settings are tested too, so checking all candidates is wrong.
#[test]
fn cli_emulator_and_rom_override_environment_and_path_candidates() {
    let e = EmulatorFixture::new("emulator-cli-priority");
    let environment_rom = e.f.root.join("environment.rom");
    std::fs::write(&environment_rom, b"different environment ROM\n").unwrap();
    for invalid_lower in [false, true] {
        e.f.clear_logs();
        let invalid = e.f.root.join("missing lower candidate");
        let emulator = if invalid_lower { &invalid } else { &e.environment };
        let rom = if invalid_lower { &invalid } else { &environment_rom };
        let output = run(e.native("hosted-amiga", &[], "hello.clo")
            .env("CLORRAINE_FS_UAE", emulator)
            .env("CLORRAINE_KICKSTART", rom));
        exit(&output, 0);
        e.check("cli", &e.rom);
        assert_eq!(e.fields("cli")["executable"], text(&e.cli));
        assert!(!e.record("environment").exists());
        assert!(!e.record("path").exists());
    }
}

/// Environment emulator and ROM settings outrank the PATH emulator.
#[test]
fn environment_emulator_and_rom_override_path_candidate() {
    let e = EmulatorFixture::new("emulator-environment-priority");
    let sdk = e.install.join("sdk/amiga");
    let output = run(e.command(
        &["run", "--profile", "hosted-amiga", "--sdk-root", text(&sdk),
          "--emulator", "hello.clo"])
        .env("CLORRAINE_FS_UAE", &e.environment)
        .env("CLORRAINE_KICKSTART", &e.rom));
    exit(&output, 0);
    e.check("environment", &e.rom);
    assert_eq!(e.fields("environment")["executable"], text(&e.environment));
    assert!(!e.record("path").exists());
    assert!(!e.record("cli").exists());
}

/// PATH supplies an emulator only when no higher candidate was selected.
#[test]
fn path_emulator_uses_explicitly_configured_environment_rom() {
    let e = EmulatorFixture::new("emulator-path-priority");
    let sdk = e.install.join("sdk/amiga");
    let output = run(e.command(
        &["run", "--profile", "hosted-amiga", "--sdk-root", text(&sdk),
          "--emulator", "hello.clo"])
        .env("CLORRAINE_KICKSTART", &e.rom));
    exit(&output, 0);
    e.check("path", &e.rom);
    assert_eq!(e.fields("path")["executable"], text(&e.f.root.join("path tools/fs-uae")));
}

/// Missing required executable/media is exit one with a named remedy, not a
/// lower-priority fallback or a usage error.
fn required_input(e: &EmulatorFixture, output: &std::process::Output, name: &str, remedy: &str) {
    exit(output, 1);
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.to_ascii_lowercase().contains(&name.to_ascii_lowercase()),
        "missing input must be named: {error}");
    let environment = if remedy == "--fs-uae" { "CLORRAINE_FS_UAE" } else { "CLORRAINE_KICKSTART" };
    assert!(error.contains(remedy) || error.contains(environment)
        || error.to_ascii_lowercase().contains("install"),
        "configuration remedy must be named: {error}");
    e.f.assert_no_processes();
}

/// An invalid selected executable does not fall back to a valid environment/PATH.
#[test]
fn missing_selected_emulator_returns_one_without_fallback() {
    let e = EmulatorFixture::new("emulator-missing");
    let missing = e.f.root.join("missing chosen fs-uae");
    let sdk = e.install.join("sdk/amiga");
    let output = run(e.command(
        &["run", "--profile", "hosted-amiga", "--sdk-root", text(&sdk),
          "--emulator", "--fs-uae", text(&missing), "--kickstart", text(&e.rom), "hello.clo"])
        .env("CLORRAINE_FS_UAE", &e.environment));
    required_input(&e, &output, "fs-uae", "--fs-uae");
}

/// An unreadable selected ROM cannot silently use the environment ROM.
#[test]
fn unreadable_selected_rom_returns_one_without_fallback() {
    let e = EmulatorFixture::new("rom-unreadable");
    let unreadable = e.f.root.join("unreadable selected Kickstart.rom");
    std::fs::write(&unreadable, b"unreadable ROM fixture\n").unwrap();
    std::fs::set_permissions(&unreadable, std::fs::Permissions::from_mode(0o000)).unwrap();
    assert!(std::fs::File::open(&unreadable).is_err(),
        "run this suite as the ordinary SDK user");
    let sdk = e.install.join("sdk/amiga");
    let output = run(e.command(
        &["run", "--profile", "hosted-amiga", "--sdk-root", text(&sdk),
          "--emulator", "--fs-uae", text(&e.cli), "--kickstart", text(&unreadable), "hello.clo"])
        .env("CLORRAINE_KICKSTART", &e.rom));
    required_input(&e, &output, "Kickstart", "--kickstart");
}

/// A ROM placed in plausible user locations is not an implicit ROM setting.
#[test]
fn missing_rom_setting_returns_one_without_scanning_private_home() {
    let e = EmulatorFixture::new("rom-no-scan");
    let directory = e.home.join("FS-UAE/Kickstarts");
    std::fs::create_dir_all(&directory).unwrap();
    std::fs::copy(&e.rom, directory.join("kickstart.rom")).unwrap();
    let sdk = e.install.join("sdk/amiga");
    let output = run(&mut e.command(
        &["run", "--profile", "hosted-amiga", "--sdk-root", text(&sdk),
          "--emulator", "--fs-uae", text(&e.cli), "hello.clo"]));
    required_input(&e, &output, "Kickstart", "--kickstart");
}
