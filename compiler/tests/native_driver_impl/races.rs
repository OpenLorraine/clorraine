//! External process barriers for activation and private-storage collision checks.

use super::emulator_fixtures::*;
use super::fixtures::*;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// Release an external compiler before any waiting driver/helper is joined.
struct Release(PathBuf);

impl Release {
    /// File-based signals belong to the process fixture, never production code.
    fn now(&self) {
        std::fs::write(&self.0, b"continue\n").unwrap();
    }
}

impl Drop for Release {
    fn drop(&mut self) {
        let _ = std::fs::write(&self.0, b"continue\n");
    }
}

/// Keep a bounded driver joined even when an intermediate assertion fails.
struct PausedBuild {
    release: Release,
    worker: Option<JoinHandle<Output>>,
}

impl PausedBuild {
    /// Start one independent command with the existing compiler's wait barrier.
    fn start(mut command: Command, barrier: &Path) -> Self {
        command.env("TEST_NATIVE_BARRIER", barrier);
        Self {
            release: Release(barrier.with_extension("release")),
            worker: Some(std::thread::spawn(move || run(&mut command))),
        }
    }

    /// Release and reap the entire command before the fixture can be removed.
    fn finish(mut self) -> Output {
        self.release.now();
        self.worker.take().unwrap().join().unwrap()
    }
}

impl Drop for PausedBuild {
    fn drop(&mut self) {
        let _ = std::fs::write(&self.release.0, b"continue\n");
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

/// Wait at most ten seconds for an external compiler's publication boundary.
fn compiler_ready(barrier: &Path) {
    let ready = barrier.with_extension("ready");
    let deadline = Instant::now() + Duration::from_secs(10);
    while !ready.exists() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(ready.is_file(), "compiler never reached its external barrier");
}

/// Environment-selected current aliases are resolved once for either Amiga
/// profile. Even invalid replacement metadata cannot affect the pinned runner.
#[test]
fn environment_activation_cannot_reselect_metadata_or_runner_after_compilation_starts() {
    for profile in ["hosted-amiga", "hosted-amiga-13"] {
        let f = Fixture::new("environment-activation");
        let original = f.install("managed/installations/a", "original", "cp310");
        let replacement = f.install("managed/installations/b", "replacement", "cp312");
        std::fs::write(replacement.join("installation.json"), b"invalid replacement\n").unwrap();
        let managed = f.root.join("managed");
        let driver = stable_links(&managed, "a");
        let barrier = f.root.join("compiler barrier");
        let mut command = f.command(&driver, &["run", "--profile", profile, "hello.clo"]);
        command.env("CLORRAINE_SDK", managed.join("current/sdk/amiga"))
            .env("TEST_NATIVE_STATUS", "7");
        let paused = PausedBuild::start(command, &barrier);
        compiler_ready(&barrier);
        symlink("installations/b", managed.join("next")).unwrap();
        std::fs::rename(managed.join("next"), managed.join("current")).unwrap();
        let output = paused.finish();
        exit(&output, 7);
        selected_profile(&f, &original, "original", profile);
        assert!(!f.logs.join("replacement-compiler.argv").exists());
        assert!(!f.logs.join("replacement-runner.argv").exists());
    }
}

/// Publish the driver's PID from an external compiler, then delegate unchanged.
/// This gives the test a deterministic allocation collision without a driver hook.
fn publish_driver_pid(e: &EmulatorFixture) {
    let compiler = e.install.join("sdk/amiga/bin/m68k-amigaos-gcc");
    std::fs::rename(&compiler, compiler.with_file_name("compiler-delegate")).unwrap();
    executable(&compiler, r#"#!/usr/bin/python3
"""Expose the parent process identity before the unchanged compiler barrier."""
import os
from pathlib import Path
import sys
barrier = Path(os.environ["TEST_NATIVE_BARRIER"])
barrier.with_suffix(".pid").write_text(str(os.getppid()))
delegate = str(Path(__file__).with_name("compiler-delegate"))
os.execv(delegate, [delegate, *sys.argv[1:]])
"#);
}

/// Occupied candidate paths are never adopted or followed, even if a restarted
/// process reuses an earlier PID. Cleanup removes only the newly allocated root.
#[test]
fn emulator_allocation_neither_adopts_nor_cleans_stale_directory_or_alias_candidates() {
    for alias in [false, true] {
        let e = EmulatorFixture::new("stale-storage-collision");
        publish_driver_pid(&e);
        let barrier = e.f.root.join("compiler barrier");
        let mut command = e.native("hosted-amiga", &[], "hello.clo");
        command.env("TEST_NATIVE_BARRIER", &barrier);
        let mut waiting = WaitingRun::start(&e, command, "cli");
        // This guard drops before WaitingRun on failure, releasing its compiler
        // before the driver/emulator join and any fixture cleanup.
        let release = Release(barrier.with_extension("release"));
        compiler_ready(&barrier);
        let pid = std::fs::read_to_string(barrier.with_extension("pid")).unwrap();
        assert!(pid.bytes().all(|byte| byte.is_ascii_digit()));
        let stale = e.temporary.join(format!("clorraine-emulator-{pid}-0"));
        let protected = if alias {
            let target = e.f.root.join("protected other storage");
            std::fs::create_dir(&target).unwrap();
            symlink(&target, &stale).unwrap();
            target
        } else {
            std::fs::create_dir(&stale).unwrap();
            stale.clone()
        };
        let sentinel = protected.join("do-not-adopt");
        std::fs::write(&sentinel, b"existing private storage\n").unwrap();
        release.now();
        waiting.ready();
        waiting.assert_waiting();
        e.check("cli", &e.rom);
        let fields = e.fields("cli");
        let allocated = Path::new(&fields["drive"]).parent().unwrap();
        assert_ne!(allocated, stale);
        assert_ne!(allocated, protected);
        assert_eq!(std::fs::read(&sentinel).unwrap(), b"existing private storage\n");
        let output = waiting.finish();
        exit(&output, 0);
        cleaned(&fields);
        assert_eq!(std::fs::read(&sentinel).unwrap(), b"existing private storage\n");
        if alias {
            assert_eq!(std::fs::read_link(&stale).unwrap(), protected);
        } else {
            assert!(stale.is_dir());
        }
    }
}
