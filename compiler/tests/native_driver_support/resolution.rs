//! SDK selection is observed at the subprocess boundary, not just preflight.

use super::fixtures::*;
use std::os::unix::fs::symlink;
use std::time::{Duration, Instant};

/// An explicit SDK outranks a different environment, adjacent SDK and PATH tools.
#[test]
fn cli_sdk_wins_over_environment_adjacent_and_legacy_tools() {
    let f = Fixture::new("cli-priority");
    let adjacent = f.install("adjacent", "adjacent", "cp310");
    let environment = f.install("environment", "environment", "cp312");
    let cli = f.install("cli ; selected", "cli", "cp310");
    let output = run(f.command(
        &adjacent.join("bin/clorraine"),
        &["run", "--profile", "hosted-amiga", "--sdk-root",
          text(&cli.join("sdk/amiga")), "hello.clo"],
    ).env("CLORRAINE_SDK", environment.join("sdk/amiga")));
    exit(&output, 0);
    selected(&f, &cli, "cli");
    assert_eq!(std::fs::read_dir(&f.logs).unwrap().count(), 2);
}

/// Environment selection outranks adjacent discovery and legacy PATH tools.
#[test]
fn environment_sdk_wins_over_adjacent_and_legacy_tools() {
    let f = Fixture::new("environment-priority");
    let adjacent = f.install("adjacent", "adjacent", "cp310");
    let environment = f.install("environment", "environment", "cp312");
    let output = run(f.command(
        &adjacent.join("bin/clorraine"),
        &["run", "--profile", "hosted-amiga", "hello.clo"],
    ).env("CLORRAINE_SDK", environment.join("sdk/amiga")));
    exit(&output, 0);
    selected(&f, &environment, "environment");
}

/// An adjacent candidate supplies both absolute tool paths despite PATH decoys.
#[test]
fn adjacent_sdk_wins_over_legacy_tools() {
    let f = Fixture::new("adjacent-priority");
    let adjacent = f.install("adjacent", "adjacent", "cp310");
    let output = run(&mut f.command(
        &adjacent.join("bin/clorraine"),
        &["run", "--profile", "hosted-amiga", "hello.clo"],
    ));
    exit(&output, 0);
    selected(&f, &adjacent, "adjacent");
}

/// With no native candidate, the existing container marker still selects the
/// bare compiler and bare vamos from PATH.
#[test]
fn legacy_marker_keeps_existing_path_tool_lookup() {
    assert!(std::path::Path::new("/opt/amiga/BUILD-PROVENANCE.txt").is_file(),
        "this compatibility control requires the pinned SDK container");
    let f = Fixture::new("legacy-priority");
    let output = run(&mut f.command(
        &f.driver, &["run", "--profile", "hosted-amiga", "hello.clo"],
    ));
    exit(&output, 0);
    assert!(f.argv("legacy", "compiler")[0].ends_with("m68k-amigaos-gcc"));
    let runner = f.argv("legacy", "runner");
    assert_eq!(runner.len(), 2, "legacy vamos still receives only the program");
    assert!(runner[0].ends_with("vamos"));
}

/// Stable links must follow the canonical executable, including after the
/// whole installation root moves to another path with spaces and punctuation.
#[test]
fn stable_link_discovers_adjacent_sdk_before_and_after_root_relocation() {
    let f = Fixture::new("relocation");
    let original = f.root.join("original root");
    let install = f.install("original root/installations/a", "relocated", "cp310");
    let driver = stable_links(&original, "a");
    for action in ["build", "run"] {
        let output = run(&mut f.command(
            &driver, &[action, "--profile", "hosted-amiga", "hello.clo"],
        ));
        exit(&output, 0);
    }
    selected(&f, &install, "relocated");
    f.clear_logs();
    let moved = f.root.join("moved root ; literal");
    std::fs::rename(&original, &moved).expect("relocate the whole root");
    assert!(!original.exists(), "old prefix must no longer be usable");
    for action in ["build", "run"] {
        let output = run(&mut f.command(
            &moved.join("bin/clorraine"),
            &[action, "--profile", "hosted-amiga", "hello.clo"],
        ));
        exit(&output, 0);
    }
    selected(&f, &moved.join("installations/a"), "relocated");
}

/// Activation after tool resolution cannot redirect the waiting compiler's
/// path or the later runner. The external compiler waits on bounded file
/// signals; no production pause hook is introduced.
#[test]
fn activation_during_build_keeps_original_compiler_and_companion() {
    let f = Fixture::new("activation");
    let original = f.install("root/installations/a", "original", "cp310");
    f.install("root/installations/b", "replacement", "cp312");
    let root = f.root.join("root");
    let driver = stable_links(&root, "a");
    let barrier = f.root.join("compiler gate");
    let ready = barrier.with_extension("ready");
    let release = barrier.with_extension("release");
    let mut command = f.command(
        &driver,
        &["run", "--profile", "hosted-amiga", "--sdk-root",
          text(&root.join("current/sdk/amiga")), "hello.clo"],
    );
    command.env("TEST_NATIVE_BARRIER", &barrier);
    let worker = std::thread::spawn(move || run(&mut command));
    let deadline = Instant::now() + Duration::from_secs(10);
    while !ready.exists() && !worker.is_finished() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
    }
    let reached = ready.exists();
    // Always release before joining, even if the observation failed, so the
    // fixture cannot delete files still in use by its own compiler helper.
    if reached {
        symlink("installations/b", root.join("next")).expect("prepare activation");
        std::fs::rename(root.join("next"), root.join("current")).expect("activate");
    }
    std::fs::write(&release, "continue\n").expect("release compiler");
    let output = worker.join().expect("join bounded driver");
    exit(&output, 0);
    assert!(reached, "compiler must reach the external pause boundary");
    selected(&f, &original, "original");
    assert!(!f.logs.join("replacement-compiler.argv").exists());
    assert!(!f.logs.join("replacement-runner.argv").exists());
}

/// An interpreter alias remains venv-spelled when executed. Resolving it to a
/// final interpreter would change Python's environment selection.
#[test]
fn companion_interpreter_alias_keeps_venv_spelling() {
    let f = Fixture::new("python-alias");
    let install = f.install("candidate", "venv", "cp312");
    let python = install.join("sdk/vamos/bin/python");
    std::fs::rename(&python, python.with_file_name("system-python-helper")).unwrap();
    symlink("system-python-helper", &python).expect("make interpreter alias");
    let output = f.native(&install, "run", &[]);
    exit(&output, 0);
    selected(&f, &install, "venv");
}

/// An explicit SDK may have a different core-version record from the driver;
/// the descriptor describes the selected assembly, not a core-version lock.
#[test]
fn explicit_sdk_does_not_require_matching_driver_core_version() {
    let f = Fixture::new("different-core");
    let install = f.install("candidate", "different", "cp310");
    replace(&install.join("installation.json"), "\"1.1.2\"", "\"8.7.6\"");
    let output = f.native(&install, "run", &[]);
    exit(&output, 0);
    selected(&f, &install, "different");
}
