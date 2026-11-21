//! Existing output, status, build-mode and container-handoff compatibility.

use super::fixtures::*;
use std::process::{Command, Output};

/// Supplies deliberately unusable settings for tools the command must ignore.
fn irrelevant_environment(command: &mut Command, f: &Fixture) {
    command
        .env("CLORRAINE_SDK", f.root.join("unused invalid SDK"))
        .env("CLORRAINE_FS_UAE", f.root.join("unused invalid emulator"))
        .env("CLORRAINE_KICKSTART", f.root.join("unused invalid ROM"));
}

/// Host output and program status do not depend on any native SDK setting.
#[test]
fn host_run_ignores_unused_native_environment_and_preserves_output_and_status() {
    let f = Fixture::new("host-run");
    for status in [0, 7] {
        f.write_source(&source(status));
        let mut command = f.command(&f.driver, &["run", "hello.clo"]);
        irrelevant_environment(&mut command, &f);
        let output = run(&mut command);
        exit(&output, status);
        assert_eq!(output.stdout, HELLO.as_bytes());
        assert!(output.stderr.is_empty());
    }
}

/// Host capture keeps output in its established file and returns program status.
#[test]
fn host_capture_ignores_unused_native_environment_and_preserves_exit_seven() {
    let f = Fixture::new("host-capture");
    f.write_source(&source(7));
    let mut command = f.command(&f.driver, &["run", "--capture", "hello.clo"]);
    irrelevant_environment(&mut command, &f);
    let output = run(&mut command);
    exit(&output, 7);
    assert_capture(&f, "host", &output);
}

/// Emit is compiler-only, even for an Amiga profile with invalid tool settings.
#[test]
fn emit_ignores_unused_native_environment_and_matches_clc() {
    let f = Fixture::new("emit");
    for profile in ["host", "hosted-amiga", "hosted-amiga-13", "freestanding"] {
        // No std.io import is needed to compare profile-specific translation.
        f.write_source("module hello;\nfunc main(): i32 { return 0; }\n");
        let reference = f.cwd.join("reference.c");
        let output = Command::new("timeout")
            .args(["30s", env!("CARGO_BIN_EXE_clc"), "--profile", profile, "-o"])
            .arg(&reference).arg("hello.clo")
            .current_dir(&f.cwd).output().expect("run reference compiler");
        exit(&output, 0);
        let mut command = f.command(
            &f.driver, &["emit", "--profile", profile, "hello.clo"],
        );
        irrelevant_environment(&mut command, &f);
        let output = run(&mut command);
        exit(&output, 0);
        assert!(output.stdout == std::fs::read(reference).unwrap(),
            "emit must match clc for {profile} with the same source path spelling");
        assert!(output.stderr.is_empty());
    }
    f.assert_no_processes();
}

/// Informational commands remain usable without configured native tools.
#[test]
fn help_and_version_ignore_unused_native_environment() {
    let f = Fixture::new("help-version");
    for flag in ["--help", "--version"] {
        let mut command = f.command(&f.driver, &[flag]);
        irrelevant_environment(&mut command, &f);
        let output = run(&mut command);
        exit(&output, 0);
        assert!(!output.stdout.is_empty());
        assert!(output.stderr.is_empty());
    }
    f.assert_no_processes();
}

/// Freestanding links with the selected cross compiler, but Run still compiles
/// and executes the existing host leg, not the synthetic cross-linked file.
#[test]
fn freestanding_build_and_host_execution_use_native_compiler_without_vamos() {
    let f = Fixture::new("freestanding");
    let install = f.install("candidate", "freestanding", "cp310");
    std::fs::remove_dir_all(install.join("sdk/vamos")).unwrap();
    f.write_source("module hello;\nfunc main(): i32 { return 7; }\n");
    for action in ["build", "run"] {
        f.clear_logs();
        let output = run(f.command(&f.driver,
            &[action, "--profile", "freestanding", "--sdk-root",
              text(&install.join("sdk/amiga")), "hello.clo"])
            .env("CLORRAINE_FS_UAE", f.root.join("unused emulator"))
            .env("CLORRAINE_KICKSTART", f.root.join("unused ROM")));
        exit(&output, if action == "run" { 7 } else { 0 });
        let cross = f.argv("freestanding", "compiler");
        assert_eq!(cross[0], text(&install.join("sdk/amiga/bin/m68k-amigaos-gcc")));
        for flag in ["-ffreestanding", "-fno-builtin", "-nostdlib", "-m68000", "-lg"] {
            assert!(cross.iter().any(|arg| arg == flag), "missing cross flag {flag}");
        }
        if action == "run" {
            f.argv("host", "cc");
            assert!(output.stdout.is_empty(), "cross helper must not be executed");
        }
        assert!(!f.logs.join("freestanding-runner.argv").exists());
        assert!(!f.logs.join("legacy-runner.argv").exists());
    }
}

/// Native selection does not change compilation arguments. The full argument
/// vector is compared with the retained legacy path for the same build, with
/// independently pinned runtime and optimization flags checked as well.
fn modes_match_legacy(mode: Option<&str>) {
    let f = Fixture::new("mode-flags");
    let install = f.install("candidate", "native", "cp310");
    for profile in ["hosted-amiga", "hosted-amiga-13"] {
        for action in ["build", "run"] {
            f.clear_logs();
            let mut args = vec![action, "--profile", profile];
            if let Some(mode) = mode {
                args.push(mode);
            }
            args.push("hello.clo");
            let legacy = run(&mut f.command(&f.driver, &args));
            exit(&legacy, 0);
            let legacy_args = f.argv("legacy", "compiler");
            f.clear_logs();
            let sdk = install.join("sdk/amiga");
            args.pop();
            args.extend(["--sdk-root", text(&sdk), "hello.clo"]);
            let native = run(&mut f.command(&f.driver, &args));
            exit(&native, 0);
            let actual = f.argv("native", "compiler");
            assert_eq!(&actual[1..], &legacy_args[1..], "target flags must stay unchanged");
            let runtime = if profile == "hosted-amiga" { "-mcrt=nix20" } else { "-mcrt=nix13" };
            let optimization = match mode {
                Some("--release") => "-O2",
                Some("--size") => "-Os",
                _ => "-O0",
            };
            for flag in ["-std=c99", "-m68000", "-fwrapv", "-lamiga", "-lm",
                         runtime, optimization] {
                assert!(actual.iter().any(|arg| arg == flag), "missing target flag {flag}");
            }
            if action == "run" {
                assert_eq!(native.stdout, HELLO.as_bytes());
                selected_profile(&f, &install, "native", profile);
            } else {
                assert!(!String::from_utf8_lossy(&native.stdout).contains(HELLO.trim()));
                assert!(!f.logs.join("native-runner.argv").exists());
            }
        }
    }
}

/// Both Amiga runtime profiles retain their debug compiler arguments and output.
#[test]
fn native_debug_build_and_run_preserve_both_profile_flag_vectors() {
    modes_match_legacy(None);
}

/// Both Amiga runtime profiles retain release arguments and output.
#[test]
fn native_release_build_and_run_preserve_both_profile_flag_vectors() {
    modes_match_legacy(Some("--release"));
}

/// Both Amiga runtime profiles retain size arguments and output.
#[test]
fn native_size_build_and_run_preserve_both_profile_flag_vectors() {
    modes_match_legacy(Some("--size"));
}

/// Validates established capture-file semantics without treating a printed
/// status as proof of the returned process status.
fn assert_capture(f: &Fixture, profile: &str, output: &Output) {
    let relative = format!("build/clorraine/hello/{profile}/out.txt");
    assert_eq!(std::fs::read(f.cwd.join(&relative)).unwrap(), HELLO.as_bytes());
    let printed = String::from_utf8_lossy(&output.stdout);
    assert!(printed.contains(&relative));
    assert!(!printed.contains(HELLO.trim()), "capture must not stream guest output");
}

/// Exercises dispatch and failure-status propagation for both documented ABI
/// descriptors. These helpers do not claim to execute either real wheel cohort.
fn exit_seven(abi: &str, capture: bool) {
    let f = Fixture::new("native-exit-seven");
    let install = f.install("candidate", "native", abi);
    f.write_source(&source(7));
    for profile in ["hosted-amiga", "hosted-amiga-13"] {
        f.clear_logs();
        let mut args = vec!["run", "--profile", profile, "--sdk-root"];
        let sdk = install.join("sdk/amiga");
        args.push(text(&sdk));
        if capture {
            args.push("--capture");
        }
        args.push("hello.clo");
        let output = run(f.command(&f.driver, &args).env("TEST_NATIVE_STATUS", "7"));
        exit(&output, 7);
        if capture {
            assert_capture(&f, profile, &output);
        } else {
            assert_eq!(output.stdout, HELLO.as_bytes());
        }
        selected_profile(&f, &install, "native", profile);
    }
}

/// The Python 3.10 dispatch contract returns the runner's nonzero status.
#[test]
fn cp310_native_run_prints_and_returns_exactly_seven() {
    exit_seven("cp310", false);
}

/// Python 3.10 capture preserves both output-file and nonzero-status behavior.
#[test]
fn cp310_native_capture_prints_path_and_returns_exactly_seven() {
    exit_seven("cp310", true);
}

/// The Python 3.12 dispatch contract returns the runner's nonzero status.
#[test]
fn cp312_native_run_prints_and_returns_exactly_seven() {
    exit_seven("cp312", false);
}

/// Python 3.12 capture preserves both output-file and nonzero-status behavior.
#[test]
fn cp312_native_capture_prints_path_and_returns_exactly_seven() {
    exit_seven("cp312", true);
}

/// The same exit assertion used for the failure fixture must reject a launcher
/// that prints the right text but deliberately returns success.
#[test]
fn forced_zero_runner_cannot_satisfy_exit_seven_assertion() {
    let f = Fixture::new("forced-zero");
    let install = f.install("candidate", "native", "cp310");
    f.write_source(&source(7));
    let output = f.native(&install, "run", &[]);
    exit(&output, 0);
    assert_eq!(output.stdout, HELLO.as_bytes());
    assert!(std::panic::catch_unwind(|| exit(&output, 7)).is_err());
}

/// No native override means exactly the historical Docker handoff; irrelevant
/// emulator environment must not launch anything or change capture selection.
#[test]
fn legacy_emulator_handoff_and_capture_remain_unchanged_for_both_profiles() {
    let f = Fixture::new("legacy-handoff");
    for profile in ["hosted-amiga", "hosted-amiga-13"] {
        let mut observations = Vec::new();
        for capture in [false, true] {
            let mut args = vec!["run", "--profile", profile, "--emulator"];
            if capture {
                args.push("--capture");
            }
            args.push("hello.clo");
            let clean = run(&mut f.command(&f.driver, &args));
            exit(&clean, 0);
            let inherited = run(f.command(&f.driver, &args)
                .env("CLORRAINE_FS_UAE", f.root.join("must not launch"))
                .env("CLORRAINE_KICKSTART", f.root.join("missing unused ROM")));
            exit(&inherited, 0);
            assert_eq!(inherited.stdout, clean.stdout, "unused environment cannot change handoff");
            assert_eq!(inherited.stderr, clean.stderr);
            let printed = String::from_utf8(clean.stdout).unwrap();
            let prefix = if capture { "RUN_CAPTURE=1 " } else { "" };
            if profile == "hosted-amiga" {
                assert_eq!(printed, format!(
                    "FS-UAE must run on the host. From the repository root:\n  \
                     {prefix}make -C docker run PROG='build/clorraine/hello/hosted-amiga/hello'\n"
                ));
            } else {
                assert_eq!(printed, format!(
                    "FS-UAE must run on the host. From the repository root:\n  \
                     AMIGA_MODEL=A500 KICKSTART=/amiga/roms/amiga-os-130.rom RUNNER=adf \
                     {prefix}make -C docker run PROG='build/clorraine/hello/hosted-amiga-13/hello'\n"
                ));
            }
            observations.push(printed);
        }
        assert_eq!(observations[1], observations[0].replacen(
            "make -C docker run ", "RUN_CAPTURE=1 make -C docker run ", 1),
            "capture changes only its prefix");
        assert!(!f.logs.join("legacy-runner.argv").exists());
    }
}
