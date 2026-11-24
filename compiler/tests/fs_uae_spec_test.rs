//! Public contracts for the verified native FS-UAE guest run.
//!
//! These tests drive the real driver against a fake `fs-uae` and a fake
//! `m68k-amigaos-gcc`. The fake emulator writes the guest rc, result, and status
//! markers exactly as the compiled guest wrapper would, so the suite covers
//! selection errors, staging, success, guest failure, partial markers, timeout,
//! bounded termination, capture, and cleanup without X, a ROM, or a real
//! emulator.

#![cfg(target_os = "linux")]

#[path = "fs_uae_support/mod.rs"]
mod support;

use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

use support::*;

#[test]
fn a_missing_fs_uae_is_a_selection_error() {
    let s = Scratch::new("missing-emulator");
    let toolchain = s.toolchain("toolchain");
    let rom = s.rom("kickstart.rom", 524_299);
    let args = run_args(&toolchain, None, Some(&rom), "hosted-amiga", &[]);
    let stderr = exit(
        &s.run(&argv(&args), &[("PATH", s.tools.as_os_str().to_owned())]),
        2,
    );
    assert!(
        stderr.contains("--fs-uae") && stderr.contains("fs-uae"),
        "the error names the missing emulator, got:\n{stderr}"
    );
}

#[test]
fn an_old_or_unreadable_fs_uae_version_is_a_selection_error() {
    let s = Scratch::new("old-emulator");
    let toolchain = s.toolchain("toolchain");
    let rom = s.rom("kickstart.rom", 524_299);

    let old = s.fs_uae("old fs-uae", "3.1.66", SUCCESS);
    let args = run_args(&toolchain, Some(&old), Some(&rom), "hosted-amiga", &[]);
    let stderr = exit(&s.run(&argv(&args), &[]), 2);
    assert!(
        stderr.contains("3.2.0"),
        "the minimum version is named, got:\n{stderr}"
    );

    let garbage = s.fs_uae("garbage fs-uae", "not-a-version", SUCCESS);
    let args = run_args(&toolchain, Some(&garbage), Some(&rom), "hosted-amiga", &[]);
    let stderr = exit(&s.run(&argv(&args), &[]), 2);
    assert!(
        stderr.contains("--fs-uae"),
        "an unreadable version is a selection error, got:\n{stderr}"
    );

    let args = run_args(
        &toolchain,
        Some(Path::new("/no/such/fs-uae")),
        Some(&rom),
        "hosted-amiga",
        &[],
    );
    let stderr = exit(&s.run(&argv(&args), &[]), 2);
    assert!(
        stderr.contains("--fs-uae"),
        "an unreadable executable names its flag, got:\n{stderr}"
    );
}

#[test]
fn the_rom_is_required_and_must_be_a_512_kb_a1200_image() {
    let s = Scratch::new("rom");
    let toolchain = s.toolchain("toolchain");
    let emulator = successful(&s, "fs-uae");

    let args = run_args(&toolchain, Some(&emulator), None, "hosted-amiga", &[]);
    let stderr = exit(&s.run(&argv(&args), &[]), 2);
    assert!(stderr.contains("--kickstart"), "the flag is named, got:\n{stderr}");

    let small = s.rom("kickstart-13.rom", 262_144);
    let args = run_args(&toolchain, Some(&emulator), Some(&small), "hosted-amiga", &[]);
    let stderr = exit(&s.run(&argv(&args), &[]), 2);
    assert!(
        stderr.contains("256 KB") && stderr.contains("A1200") && stderr.contains("512 KB"),
        "the A1200 requirement is explicit, got:\n{stderr}"
    );

    let odd = s.rom("odd.rom", 12_345);
    let args = run_args(&toolchain, Some(&emulator), Some(&odd), "hosted-amiga", &[]);
    let stderr = exit(&s.run(&argv(&args), &[]), 2);
    assert!(stderr.contains("512 KB"), "the expected size is named, got:\n{stderr}");

    for size in [524_288usize, 524_299] {
        let rom = s.rom(&format!("kickstart-{size}.rom"), size);
        let args = run_args(&toolchain, Some(&emulator), Some(&rom), "hosted-amiga", &[]);
        let output = s.run(&argv(&args), &[]);
        exit(&output, 0);
        assert_eq!(
            String::from_utf8_lossy(&output.stdout),
            "Hello, Amiga, from Clorraine!\n"
        );
    }
}

#[test]
fn an_invalid_timeout_is_a_usage_error() {
    let s = Scratch::new("timeout-value");
    let toolchain = s.toolchain("toolchain");
    let emulator = successful(&s, "fs-uae");
    let rom = s.rom("kickstart.rom", 524_299);
    for value in ["0", "3601", "abc"] {
        let args = run_args(&toolchain, Some(&emulator), Some(&rom), "hosted-amiga", &[]);
        let stderr = exit(
            &s.run(&argv(&args), &[("CLORRAINE_EMULATOR_TIMEOUT_SECONDS", value.into())]),
            2,
        );
        assert!(
            stderr.contains("CLORRAINE_EMULATOR_TIMEOUT_SECONDS"),
            "the variable is named for {value}, got:\n{stderr}"
        );
    }
}

#[test]
fn staging_compiles_the_wrapper_and_uses_the_cnext_startup() {
    let s = Scratch::new("staging");
    let toolchain = s.toolchain("toolchain");
    let body = format!(
        "cp \"$PWD/run.fs-uae\" \"$TEST_LOG/run.fs-uae\"\n\
         printf '%s\\n' \"$HOME\" \"$XDG_CONFIG_HOME\" \"$XDG_DATA_HOME\" \"$XDG_CACHE_HOME\" \"$XDG_STATE_HOME\" \"$FS_UAE_BASE_DIR\" \"$TMPDIR\" > \"$TEST_LOG/emulator-env.txt\"\n\
         stat -c '%a' \"$PWD\" > \"$TEST_LOG/mode.txt\"\n\
         find \"$drive\" -maxdepth 1 -type f -printf '%f\\n' | sort > \"$TEST_LOG/drive-files.txt\"\n\
         cp \"$drive/s/startup-sequence\" \"$TEST_LOG/startup-sequence\"\n\
         cp \"$drive/runner\" \"$TEST_LOG/runner\"\n\
         cp \"$drive/program\" \"$TEST_LOG/program\"\n\
         {SUCCESS}"
    );
    let emulator = s.fs_uae("fs-uae", "3.2.35", &body);
    let rom = s.rom("kickstart.rom", 524_299);
    let args = run_args(&toolchain, Some(&emulator), Some(&rom), "hosted-amiga", &[]);
    exit(&s.run(&argv(&args), &[]), 0);

    let startup = s.read(&s.logs.join("startup-sequence"));
    assert_eq!(startup, "CNEXT:runner\n", "the startup script runs the wrapper");
    assert!(s.logs.join("runner").exists(), "the compiled runner is on the drive");
    assert!(s.logs.join("program").exists(), "the program is on the drive");

    let wrapper = s.read(&s.logs.join("wrapper.c"));
    assert!(
        wrapper.contains("SystemTagList") && wrapper.contains("clorraine-status.txt"),
        "the host compiles the rc/status wrapper, got:\n{wrapper}"
    );
    assert!(
        !wrapper.contains("echo marker"),
        "the rejected two-line marker script must not return"
    );

    let compile = s.read(&toolchain.join("compiler.log"));
    let wrapper_line = compile
        .lines()
        .find(|line| line.contains("drive/runner"))
        .unwrap_or_else(|| panic!("no wrapper compile in:\n{compile}"));
    for flag in [
        "-std=c99",
        "-mcrt=nix20",
        "-m68000",
        "-fwrapv",
        "-O2",
        "-Wall",
        "-Wextra",
        "-Werror",
        "-lamiga",
        "-lm",
        "emulator_runner.c",
    ] {
        assert!(
            wrapper_line.contains(flag),
            "the wrapper compile lacks {flag}:\n{wrapper_line}"
        );
    }

    let private = recorded_private(&s);
    let config = s.read(&s.logs.join("run.fs-uae"));
    for line in [
        "[fs-uae]",
        "end_config=1",
        "amiga_model=A1200",
        "hard_drive_0_label=CNEXT",
        "hard_drive_0_priority=0",
        "floppy_drive_0=",
        "floppy_drive_1=",
        "cdrom_drive_0=",
    ] {
        assert!(
            config.lines().any(|entry| entry == line),
            "the A1200 config lacks {line}:\n{config}"
        );
    }
    assert!(
        !config.contains(&rom.display().to_string()),
        "the config must not embed a host ROM path:\n{config}"
    );

    let args = s.log("fs-uae.log");
    for (name, path) in [
        ("base_dir", private.join("base")),
        ("cache_dir", private.join("cache")),
        ("save_states_dir", private.join("state")),
        ("logs_dir", private.join("logs")),
        ("kickstarts_dir", private.join("kickstarts")),
        ("hard_drive_0", private.join("drive")),
    ] {
        let expected = format!("--{name}={}", path.display());
        assert!(
            args.contains(&expected),
            "the emulator argv lacks {expected}:\n{args}"
        );
    }
    let rom_option = format!("--kickstart_file={}", rom.display());
    assert!(
        args.contains(&rom_option),
        "the emulator argv lacks the ROM option:\n{args}"
    );

    let env = s.read(&s.logs.join("emulator-env.txt"));
    let env_lines: Vec<&str> = env.lines().collect();
    let expected_env: Vec<String> = [
        "home",
        "home/.config",
        "home/.local/share",
        "cache",
        "state",
        "base",
        "tmp",
    ]
    .iter()
    .map(|relative| private.join(relative).display().to_string())
    .collect();
    assert_eq!(
        env_lines,
        expected_env.iter().map(String::as_str).collect::<Vec<_>>(),
        "the emulator environment is isolated under the private run"
    );
    assert_eq!(
        s.read(&s.logs.join("mode.txt")).trim(),
        "700",
        "the private run is owner-only"
    );

    let drive_files = s.read(&s.logs.join("drive-files.txt"));
    assert!(drive_files.lines().any(|name| name == "program"));
    assert!(drive_files.lines().any(|name| name == "runner"));
    assert!(
        !drive_files.contains("kickstart") && !drive_files.contains(".rom"),
        "no ROM may be copied into the guest drive:\n{drive_files}"
    );
}

#[test]
fn a_successful_guest_prints_the_trimmed_result() {
    let s = Scratch::new("success");
    let toolchain = s.toolchain("toolchain");
    let emulator = successful(&s, "fs-uae");
    let rom = s.rom("kickstart.rom", 524_299);
    let args = run_args(&toolchain, Some(&emulator), Some(&rom), "hosted-amiga", &[]);
    let output = s.run(&argv(&args), &[]);
    let stderr = exit(&output, 0);
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "Hello, Amiga, from Clorraine!\n",
        "the guest result is printed with terminal line endings"
    );
    assert!(stderr.is_empty(), "a success has no diagnostics, got:\n{stderr}");
    let private = recorded_private(&s);
    assert!(
        !private.exists(),
        "a completed run removes its private directory: {}",
        private.display()
    );
}

#[test]
fn an_empty_guest_result_is_valid() {
    let s = Scratch::new("empty-result");
    let toolchain = s.toolchain("toolchain");
    let body = "\
        printf '0\\n' > \"$drive/clorraine-rc.txt\"\n\
        printf 'ok\\n' > \"$drive/clorraine-status.txt\"\n\
        : > \"$drive/clorraine-result.txt\"\n";
    let emulator = s.fs_uae("fs-uae", "3.2.35", body);
    let rom = s.rom("kickstart.rom", 524_299);
    let args = run_args(&toolchain, Some(&emulator), Some(&rom), "hosted-amiga", &[]);
    let output = s.run(&argv(&args), &[]);
    exit(&output, 0);
    assert!(output.stdout.is_empty(), "empty output is valid");
}

#[test]
fn a_nonzero_guest_code_is_a_failure_with_the_result() {
    for code in [1, 5, 9] {
        let s = Scratch::new(&format!("guest-{code}"));
        let toolchain = s.toolchain("toolchain");
        let body = format!(
            "printf '{code}\\n' > \"$drive/clorraine-rc.txt\"\n\
             printf 'partial result\\n' > \"$drive/clorraine-result.txt\"\n"
        );
        let emulator = s.fs_uae("fs-uae", "3.2.35", &body);
        let rom = s.rom("kickstart.rom", 524_299);
        let args = run_args(&toolchain, Some(&emulator), Some(&rom), "hosted-amiga", &[]);
        let output = s.run(&argv(&args), &[]);
        let stderr = exit(&output, 1);
        assert!(
            stderr.contains(&format!("exited with code {code}")),
            "the guest code is named, got:\n{stderr}"
        );
        assert!(
            String::from_utf8_lossy(&output.stdout).contains("partial result"),
            "the guest output is retained"
        );
        let retained = s.retained(&stderr);
        assert!(retained.is_dir(), "the failed run keeps its evidence");
        let _ = fs::remove_dir_all(retained);
    }
}

#[test]
fn an_empty_or_partial_status_is_never_a_success() {
    let s = Scratch::new("partial-status");
    let toolchain = s.toolchain("toolchain");
    let body = "\
        printf '0\\n' > \"$drive/clorraine-rc.txt\"\n\
        printf 'o\\n' > \"$drive/clorraine-status.txt\"\n\
        exec sleep 10\n";
    let emulator = s.fs_uae("fs-uae", "3.2.35", body);
    let rom = s.rom("kickstart.rom", 524_299);
    let args = run_args(&toolchain, Some(&emulator), Some(&rom), "hosted-amiga", &[]);
    let started = Instant::now();
    let output = s.run(&argv(&args), &[]);
    let stderr = exit(&output, 1);
    assert!(
        stderr.contains("did not report success") && stderr.contains("retained at"),
        "a partial status is retained as a failure, got:\n{stderr}"
    );
    assert!(
        started.elapsed() < Duration::from_secs(6),
        "a partial marker must not wait for the full timeout"
    );
    let _ = fs::remove_dir_all(s.retained(&stderr));
}

#[test]
fn a_malformed_rc_is_not_a_success() {
    let s = Scratch::new("malformed-rc");
    let toolchain = s.toolchain("toolchain");
    let body = "\
        printf 'zero\\n' > \"$drive/clorraine-rc.txt\"\n\
        printf 'ok\\n' > \"$drive/clorraine-status.txt\"\n\
        exec sleep 10\n";
    let emulator = s.fs_uae("fs-uae", "3.2.35", body);
    let rom = s.rom("kickstart.rom", 524_299);
    let args = run_args(&toolchain, Some(&emulator), Some(&rom), "hosted-amiga", &[]);
    let stderr = exit(&s.run(&argv(&args), &[]), 1);
    assert!(
        stderr.contains("did not report success"),
        "a malformed rc is not success, got:\n{stderr}"
    );
    let _ = fs::remove_dir_all(s.retained(&stderr));
}

#[test]
fn a_status_without_a_zero_rc_is_not_success() {
    let s = Scratch::new("status-without-rc");
    let toolchain = s.toolchain("toolchain");
    let body = "\
        printf 'ok\\n' > \"$drive/clorraine-status.txt\"\n\
        printf 'partial\\n' > \"$drive/clorraine-result.txt\"\n\
        exec sleep 10\n";
    let emulator = s.fs_uae("fs-uae", "3.2.35", body);
    let rom = s.rom("kickstart.rom", 524_299);
    let args = run_args(&toolchain, Some(&emulator), Some(&rom), "hosted-amiga", &[]);
    let stderr = exit(
        &s.run(
            &argv(&args),
            &[("CLORRAINE_EMULATOR_TIMEOUT_SECONDS", "1".into())],
        ),
        1,
    );
    assert!(
        stderr.contains("did not report success"),
        "a status without a zero rc is not success, got:\n{stderr}"
    );
    let _ = fs::remove_dir_all(s.retained(&stderr));
}

#[test]
fn an_emulator_that_exits_without_markers_fails_promptly() {
    let s = Scratch::new("exited");
    let toolchain = s.toolchain("toolchain");
    let emulator = s.fs_uae("fs-uae", "3.2.35", "exit 0\n");
    let rom = s.rom("kickstart.rom", 524_299);
    let args = run_args(&toolchain, Some(&emulator), Some(&rom), "hosted-amiga", &[]);
    let started = Instant::now();
    let stderr = exit(&s.run(&argv(&args), &[]), 1);
    assert!(
        started.elapsed() < Duration::from_secs(10),
        "an exited emulator must not wait for the default timeout"
    );
    assert!(
        stderr.contains("did not report success"),
        "an exited emulator without markers is a failure, got:\n{stderr}"
    );
    let _ = fs::remove_dir_all(s.retained(&stderr));
}

#[test]
fn a_guest_timeout_is_bounded_and_retains_the_run() {
    let s = Scratch::new("timeout");
    let toolchain = s.toolchain("toolchain");
    let emulator = s.fs_uae("fs-uae", "3.2.35", "exec sleep 10\n");
    let rom = s.rom("kickstart.rom", 524_299);
    let args = run_args(&toolchain, Some(&emulator), Some(&rom), "hosted-amiga", &[]);
    let started = Instant::now();
    let stderr = exit(
        &s.run(
            &argv(&args),
            &[("CLORRAINE_EMULATOR_TIMEOUT_SECONDS", "1".into())],
        ),
        1,
    );
    assert!(
        stderr.contains("timed out after 1 seconds"),
        "the timeout is reported, got:\n{stderr}"
    );
    assert!(
        started.elapsed() < Duration::from_secs(6),
        "the timeout is bounded"
    );
    let retained = s.retained(&stderr);
    assert!(retained.is_dir(), "the timed-out run keeps its evidence");
    let _ = fs::remove_dir_all(retained);
}

#[test]
fn the_emulator_is_killed_and_reaped() {
    let s = Scratch::new("unreaped");
    let toolchain = s.toolchain("toolchain");
    let body = "\
        printf '0\\n' > \"$drive/clorraine-rc.txt\"\n\
        printf 'ok\\n' > \"$drive/clorraine-status.txt\"\n\
        printf 'done\\n' > \"$drive/clorraine-result.txt\"\n\
        printf '%s\\n' \"$$\" > \"$TEST_LOG/emulator.pid\"\n\
        while :; do sleep 1; done\n";
    let emulator = s.fs_uae("fs-uae", "3.2.35", body);
    let rom = s.rom("kickstart.rom", 524_299);
    let args = run_args(&toolchain, Some(&emulator), Some(&rom), "hosted-amiga", &[]);
    let started = Instant::now();
    let output = s.run(&argv(&args), &[]);
    let stderr = exit(&output, 0);
    assert!(stderr.is_empty(), "a completed guest has no diagnostics");
    assert_eq!(String::from_utf8_lossy(&output.stdout), "done\n");
    let elapsed = started.elapsed();
    assert!(
        elapsed < Duration::from_secs(5),
        "termination is bounded, took {elapsed:?}"
    );
    let pid = s.log("emulator.pid");
    let pid = pid.trim();
    assert!(
        !Path::new(&format!("/proc/{pid}")).exists(),
        "the emulator process is gone after the run"
    );
    let private = recorded_private(&s);
    assert!(
        !private.exists(),
        "confirmed termination removes the private directory"
    );
}

#[test]
fn a_result_read_failure_retains_the_private_run() {
    let s = Scratch::new("read-failure");
    let toolchain = s.toolchain("toolchain");
    let body = "\
        mkdir \"$drive/clorraine-result.txt\"\n\
        printf '0\\n' > \"$drive/clorraine-rc.txt\"\n\
        printf 'ok\\n' > \"$drive/clorraine-status.txt\"\n";
    let emulator = s.fs_uae("fs-uae", "3.2.35", body);
    let rom = s.rom("kickstart.rom", 524_299);
    let args = run_args(&toolchain, Some(&emulator), Some(&rom), "hosted-amiga", &[]);
    let stderr = exit(&s.run(&argv(&args), &[]), 1);
    assert!(
        stderr.contains("cannot read") && stderr.contains("retained at"),
        "a read failure retains the run, got:\n{stderr}"
    );
    let retained = s.retained(&stderr);
    assert!(retained.is_dir(), "the unread result keeps its directory");
    let _ = fs::remove_dir_all(retained);
}

#[test]
fn noisy_emulator_output_stays_out_of_the_cli_streams() {
    let s = Scratch::new("noisy");
    let toolchain = s.toolchain("toolchain");
    let body = format!(
        "{SUCCESS}\
         printf 'emulator noise\\n'\n\
         printf 'emulator error\\n' >&2\n"
    );
    let emulator = s.fs_uae("fs-uae", "3.2.35", &body);
    let rom = s.rom("kickstart.rom", 524_299);
    let args = run_args(&toolchain, Some(&emulator), Some(&rom), "hosted-amiga", &[]);
    let output = s.run(&argv(&args), &[]);
    let stderr = exit(&output, 0);
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "Hello, Amiga, from Clorraine!\n",
        "the CLI stdout is only the guest result"
    );
    assert!(stderr.is_empty(), "the CLI stderr stays clean");
    assert!(
        !recorded_private(&s).exists(),
        "the emulator logs live in the private run and leave with it"
    );
}

#[test]
fn capture_writes_the_result_before_cleanup() {
    let s = Scratch::new("capture");
    let toolchain = s.toolchain("toolchain");
    let emulator = successful(&s, "fs-uae");
    let rom = s.rom("kickstart.rom", 524_299);
    let args = run_args(
        &toolchain,
        Some(&emulator),
        Some(&rom),
        "hosted-amiga",
        &["--capture"],
    );
    let output = s.run(&argv(&args), &[]);
    exit(&output, 0);
    let relative = Path::new("build/clorraine/hello/hosted-amiga/out.txt");
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        format!("captured {}\n", relative.display())
    );
    assert_eq!(
        fs::read_to_string(s.work.join(relative)).expect("capture exists"),
        "Hello, Amiga, from Clorraine!\n"
    );
    let private = recorded_private(&s);
    assert!(!private.exists(), "capture is written before cleanup");
}

#[test]
fn a_capture_write_failure_retains_the_private_run() {
    let s = Scratch::new("capture-failure");
    let toolchain = s.toolchain("toolchain");
    let emulator = successful(&s, "fs-uae");
    let rom = s.rom("kickstart.rom", 524_299);
    let args = run_args(
        &toolchain,
        Some(&emulator),
        Some(&rom),
        "hosted-amiga",
        &["--capture"],
    );
    let output = s.run(&argv(&args), &[("FAKE_BLOCK_CAPTURE", "1".into())]);
    let stderr = exit(&output, 1);
    assert!(
        stderr.contains("cannot write") && stderr.contains("retained at"),
        "a capture failure retains the run, got:\n{stderr}"
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("Hello, Amiga, from Clorraine!"),
        "a capture failure still prints the readable guest output"
    );
    let retained = s.retained(&stderr);
    assert!(retained.is_dir(), "the captured result is still readable there");
    let _ = fs::remove_dir_all(retained);
}

#[test]
fn the_a500_profile_also_runs_through_the_verified_runner() {
    let s = Scratch::new("a500");
    let toolchain = s.toolchain("toolchain");
    let emulator = successful(&s, "fs-uae");
    let rom = s.rom("kickstart.rom", 524_299);
    let args = run_args(&toolchain, Some(&emulator), Some(&rom), "hosted-amiga-13", &[]);
    let output = s.run(&argv(&args), &[]);
    exit(&output, 0);
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "Hello, Amiga, from Clorraine!\n"
    );
    let compile = s.read(&toolchain.join("compiler.log"));
    assert!(
        compile.lines().any(|line| line.contains("-mcrt=nix13") && line.contains("hello.c")),
        "the A500 program keeps the nix13 runtime, got:\n{compile}"
    );
    assert!(
        compile.lines().any(|line| line.contains("-mcrt=nix20") && line.contains("drive/runner")),
        "the wrapper always uses the nix20 runtime"
    );
}
