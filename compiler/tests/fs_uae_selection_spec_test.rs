//! Selection and version contracts for the verified native FS-UAE runner.
//!
//! These cases complement the runner suite: they prove flag/environment/PATH
//! precedence, the no-fallback rule, the 3.2.0 version boundary, and the
//! wrapper-build failure path.

#![cfg(target_os = "linux")]

#[path = "fs_uae_support/mod.rs"]
mod support;

use std::fs;
use std::path::Path;

use support::*;

#[test]
fn a_wrapper_compile_failure_stops_before_the_emulator() {
    let s = Scratch::new("compile-failure");
    let toolchain = s.toolchain("toolchain");
    let emulator = successful(&s, "fs-uae");
    let rom = s.rom("kickstart.rom", 524_299);
    let args = run_args(&toolchain, Some(&emulator), Some(&rom), "hosted-amiga", &[]);
    let stderr = exit(
        &s.run(&argv(&args), &[("FAKE_COMPILER_FAIL", "1".into())]),
        1,
    );
    assert!(
        stderr.contains("guest runner build failed"),
        "the wrapper build failure is named, got:\n{stderr}"
    );
    assert!(
        !s.logs.join("fs-uae.log").exists(),
        "a wrapper build failure must not start the emulator"
    );
    assert!(
        fs::read_dir(&s.run_tmp)
            .expect("read the private TMPDIR")
            .next()
            .is_none(),
        "a wrapper build failure removes its private run"
    );
}

#[test]
fn the_environment_selects_the_emulator_and_rom() {
    let s = Scratch::new("environment");
    let toolchain = s.toolchain("toolchain");
    let emulator = s.fs_uae("environment fs-uae", "3.2.35", SUCCESS);
    let rom = s.rom("environment.rom", 524_299);
    let args = run_args(&toolchain, None, None, "hosted-amiga", &[]);
    let output = s.run(
        &argv(&args),
        &[
            ("CLORRAINE_FS_UAE", emulator.as_os_str().to_owned()),
            ("CLORRAINE_KICKSTART", rom.as_os_str().to_owned()),
        ],
    );
    exit(&output, 0);
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "Hello, Amiga, from Clorraine!\n"
    );
}

#[test]
fn an_invalid_explicit_emulator_does_not_fall_back() {
    let s = Scratch::new("no-fallback");
    let toolchain = s.toolchain("toolchain");
    let rom = s.rom("kickstart.rom", 524_299);
    let old = s.fs_uae("old environment fs-uae", "3.1.66", SUCCESS);
    let args = run_args(&toolchain, None, Some(&rom), "hosted-amiga", &[]);
    let stderr = exit(
        &s.run(&argv(&args), &[("CLORRAINE_FS_UAE", old.as_os_str().to_owned())]),
        2,
    );
    assert!(
        stderr.contains("3.2.0"),
        "an invalid environment selection must not use PATH, got:\n{stderr}"
    );

    let good = s.fs_uae("good environment fs-uae", "3.2.35", SUCCESS);
    let args = run_args(
        &toolchain,
        Some(Path::new("/no/such/fs-uae")),
        Some(&rom),
        "hosted-amiga",
        &[],
    );
    let stderr = exit(
        &s.run(&argv(&args), &[("CLORRAINE_FS_UAE", good.as_os_str().to_owned())]),
        2,
    );
    assert!(
        stderr.contains("--fs-uae"),
        "an invalid flag must not fall back to the environment, got:\n{stderr}"
    );
}

#[test]
fn the_version_boundary_is_three_two_zero() {
    let s = Scratch::new("version-boundary");
    let toolchain = s.toolchain("toolchain");
    let rom = s.rom("kickstart.rom", 524_299);
    for (version, expected) in [("3.1.99", 2), ("3.2.0", 0), ("3.2.35-2", 0)] {
        let emulator = s.fs_uae(&format!("fs-uae-{version}"), version, SUCCESS);
        let args = run_args(&toolchain, Some(&emulator), Some(&rom), "hosted-amiga", &[]);
        let output = s.run(&argv(&args), &[]);
        let stderr = exit(&output, expected);
        if expected == 2 {
            assert!(
                stderr.contains("3.2.0"),
                "the minimum is named for {version}, got:\n{stderr}"
            );
        }
    }
}
