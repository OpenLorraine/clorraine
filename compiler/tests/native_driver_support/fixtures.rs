//! Private installed-layout fixtures and bounded external-process observations.

use std::ffi::OsString;
use std::io::Write;
use std::os::unix::fs::{symlink, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

/// Program output shared by the host and synthetic runner fixtures.
pub(super) const HELLO: &str = "Hello, Clorraine!\n";

/// A real source program that prints before returning a controllable status.
pub(super) fn source(status: i32) -> String {
    format!(
        "module hello;\nimport {{ println }} from std.io;\n\
         func main(): i32 {{ println(\"Hello, Clorraine!\"); return {status}; }}\n"
    )
}

/// Fixed isolated entry-point invocation; no installed metadata supplies code.
pub(super) const ENTRY: &str = r#"import importlib.metadata
import sys
entries = [entry for entry in importlib.metadata.distribution("amitools").entry_points
           if entry.group == "console_scripts" and entry.name == "vamos"]
if len(entries) != 1:
    raise SystemExit("clorraine: expected exactly one installed vamos entry point")
sys.argv = ["vamos", *sys.argv[1:]]
raise SystemExit(entries[0].load()())"#;

/// Ordinary-file descriptor with intentionally synthetic, mutually linked hashes.
pub(super) fn descriptor(abi: &str) -> String {
    format!(
        "{{\"schema\":1,\"core_version\":\"1.1.2\",\"support_lock_sha256\":\"{}\",\
         \"ndk_sha256\":\"{}\",\"prepared_inventory_sha256\":\"{}\",\"python_abi\":\"{abi}\"}}\n",
        "a".repeat(64), "b".repeat(64), "c".repeat(64)
    )
}

/// Complete miniature wheel receipt; wheel bytes are not stored inside the venv.
fn wheels(abi: &str) -> String {
    let hash = "a".repeat(64);
    let packages = ["amitools", "machine68k", "greenlet", "pysdl2", "pip"]
        .iter()
        .map(|name| {
            let purpose = if *name == "pip" { "bootstrap" } else { "runtime" };
            format!(
                "\"{name}\":{{\"version\":\"1.0\",\"source\":\"{name}\",\"purpose\":\"{purpose}\",\
                 \"filename\":\"wheels/{name}-1.0-py3-none-any.whl\",\"bytes\":1,\
                 \"sha256\":\"{hash}\",\"tags\":[\"py3-none-any\"]}}"
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{\"schema\":1,\"lock_sha256\":\"{hash}\",\"abi\":\"{abi}\",\"packages\":{{{packages}}}}}\n"
    )
}

/// One owned test tree; dropping it never removes another invocation's files.
pub(super) struct Fixture {
    /// Canonical root containing every file this fixture owns.
    pub(super) root: PathBuf,
    /// Unrelated working directory used for driver commands.
    pub(super) cwd: PathBuf,
    /// Per-helper argument records, outside directories reset by builds.
    pub(super) logs: PathBuf,
    /// Development-style driver with no adjacent SDK candidate.
    pub(super) driver: PathBuf,
    path: OsString,
}

impl Fixture {
    /// Creates an isolated tree and lower-priority PATH tools, without changing
    /// the test process's environment.
    pub(super) fn new(tag: &str) -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let base = std::env::var_os("CODEOPS_TMPDIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                Path::new(env!("CARGO_MANIFEST_DIR")).join("../build/tmp")
            });
        std::fs::create_dir_all(&base).expect("create scratch parent");
        // Container processes can reuse the same PID while sharing the mounted
        // scratch parent. Exclusive creation prevents adopting another run.
        let root = loop {
            let candidate = base.join(format!(
                "native-driver-{}-{tag}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            match std::fs::create_dir(&candidate) {
                Ok(()) => break candidate,
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => panic!("create fixture root {}: {error}", candidate.display()),
            }
        };
        let root = root.canonicalize().expect("canonical fixture root");
        let cwd = root.join("unrelated workspace ; literal");
        let logs = root.join("process logs");
        let bin = root.join("path tools");
        for path in [&cwd, &logs, &bin] {
            std::fs::create_dir_all(path).expect("create fixture directory");
        }
        let driver = root.join("development/bin/clorraine");
        copy_driver(&driver);
        executable(&bin.join("m68k-amigaos-gcc"), &helper("legacy", "compiler"));
        executable(&bin.join("vamos"), &helper("legacy", "runner"));
        executable(&bin.join("cc"), &helper("host", "cc"));
        // A wrong mode selection must never open a real inherited emulator.
        executable(&bin.join("fs-uae"), &helper("unexpected-emulator", "runner"));
        let mut search = vec![bin];
        if let Some(path) = std::env::var_os("PATH") {
            search.extend(std::env::split_paths(&path));
        }
        let path = std::env::join_paths(search).expect("join fixture PATH");
        let fixture = Self { root, cwd, logs, driver, path };
        fixture.write_source(&source(0));
        fixture
    }

    /// Creates the full miniature installation, not an installer completion
    /// receipt. No transaction state or publication data is needed by a driver.
    pub(super) fn install(&self, relative: &str, identity: &str, abi: &str) -> PathBuf {
        let root = self.root.join(relative);
        let sdk = root.join("sdk/amiga");
        let venv = root.join("sdk/vamos");
        std::fs::create_dir_all(sdk.join("bin")).expect("create SDK");
        std::fs::create_dir_all(venv.join("bin")).expect("create companion");
        copy_driver(&root.join("bin/clorraine"));
        std::fs::write(root.join("installation.json"), descriptor(abi))
            .expect("write descriptor");
        std::fs::write(
            sdk.join(".ndk-prepared.json"),
            format!(
                "{{\"schema\":1,\"lock_sha256\":\"{}\",\"ndk_sha256\":\"{}\",\
                 \"inventory_sha256\":\"{}\"}}\n",
                "a".repeat(64), "b".repeat(64), "c".repeat(64)
            ),
        ).expect("write preparation receipt");
        std::fs::write(venv.join("wheels.json"), wheels(abi)).expect("write wheel receipt");
        let version = if abi == "cp310" { "3.10.12" } else { "3.12.16" };
        std::fs::write(venv.join("pyvenv.cfg"), format!("version = {version}\n"))
            .expect("write venv config");
        executable(&sdk.join("bin/m68k-amigaos-gcc"), &helper(identity, "compiler"));
        executable(&venv.join("bin/python"), &helper(identity, "runner"));
        root
    }

    /// Writes the entry program without involving the source checkout.
    pub(super) fn write_source(&self, contents: &str) {
        std::fs::write(self.cwd.join("hello.clo"), contents).expect("write source");
    }

    /// Constructs a bounded driver command with invocation-local environment.
    /// PATH tools are decoys for native mode and genuine subprocess boundaries
    /// for the unchanged legacy lookup.
    pub(super) fn command(&self, driver: &Path, args: &[&str]) -> Command {
        let mut command = Command::new("timeout");
        command
            .arg("30s")
            .arg(driver)
            .args(args)
            .current_dir(&self.cwd)
            .env("PATH", &self.path)
            .env("TEST_NATIVE_LOG", &self.logs)
            .env("TEST_NATIVE_STATUS", "0")
            .env("TEST_NATIVE_TEXT", HELLO)
            .env_remove("TEST_NATIVE_BARRIER")
            .env_remove("TEST_NATIVE_COMPILER_ERROR")
            .env_remove("CLORRAINE_SDK")
            .env_remove("CLORRAINE_FS_UAE")
            .env_remove("CLORRAINE_KICKSTART");
        command
    }

    /// Runs through a development binary, optionally selecting a native SDK by
    /// its public command-line option rather than a test-only hook.
    pub(super) fn native(&self, install: &Path, action: &str, extra: &[&str]) -> Output {
        let mut args = vec![action, "--profile", "hosted-amiga", "--sdk-root"];
        let sdk = install.join("sdk/amiga");
        args.push(text(&sdk));
        args.extend_from_slice(extra);
        args.push("hello.clo");
        run(&mut self.command(&self.driver, &args))
    }

    /// Returns the actual executable spelling and literal argv from one helper.
    /// NUL separators preserve spaces, newlines and the Python code argument.
    pub(super) fn argv(&self, identity: &str, role: &str) -> Vec<String> {
        let path = self.logs.join(format!("{identity}-{role}.argv"));
        let bytes = std::fs::read(&path)
            .unwrap_or_else(|err| panic!("missing subprocess record {}: {err}", path.display()));
        bytes.split(|byte| *byte == 0)
            .filter(|part| !part.is_empty())
            .map(|part| String::from_utf8(part.to_vec()).expect("UTF-8 fixture argv"))
            .collect()
    }

    /// Ensures rejected selections cannot secretly use a lower-priority tool.
    pub(super) fn assert_no_processes(&self) {
        assert_eq!(std::fs::read_dir(&self.logs).unwrap().count(), 0,
            "rejected invocation must not compile or launch a runner");
    }

    /// Removes old observations, never tools or build artifacts.
    pub(super) fn clear_logs(&self) {
        for entry in std::fs::read_dir(&self.logs).unwrap() {
            std::fs::remove_file(entry.unwrap().path()).expect("clear owned log");
        }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

/// Converts owned UTF-8 fixture paths to literal command arguments.
pub(super) fn text(path: &Path) -> &str {
    path.to_str().expect("fixture path must be UTF-8")
}

/// Copies the actual integration-test binary, keeping SDK discovery independent
/// of both the build tree and the caller's working directory.
fn copy_driver(path: &Path) {
    std::fs::create_dir_all(path.parent().unwrap()).expect("create binary directory");
    let output = external_copy(Path::new(env!("CARGO_BIN_EXE_clorraine")), path)
        .expect("start driver copy");
    assert!(output.status.success(), "copy tested driver: {}",
        String::from_utf8_lossy(&output.stderr));
}

/// Creates a tiny external executable with no shell-expanded fixture data.
///
/// The harness writes only a never-executed input file. A separate process
/// copies it into place, so another test's fork cannot inherit a writable
/// descriptor for the executable and make its launch fail with ETXTBSY.
pub(super) fn executable(path: &Path, contents: &str) {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let input = path.parent().unwrap().join(format!(
        ".helper-input-{}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    {
        let mut file = std::fs::OpenOptions::new().write(true).create_new(true)
            .open(&input).expect("create private helper input");
        file.write_all(contents.as_bytes()).expect("write private helper input");
    }
    let copied = external_copy(&input, path);
    std::fs::remove_file(&input).expect("remove private helper input");
    let output = copied.expect("start helper copy");
    assert!(output.status.success(), "copy executable helper: {}",
        String::from_utf8_lossy(&output.stderr));
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))
        .expect("mark helper executable");
}

/// Keeps writable destination descriptors in a child, not in the multithreaded
/// harness. Waiting for that child closes them before any target is executed.
fn external_copy(source: &Path, destination: &Path) -> std::io::Result<Output> {
    Command::new("/bin/cp").args(["--preserve=mode", "--"])
        .arg(source).arg(destination).output()
}

/// Installs relative stable links whose meaning survives moving the whole root.
pub(super) fn stable_links(root: &Path, version: &str) -> PathBuf {
    std::fs::create_dir_all(root.join("bin")).expect("create stable bin");
    symlink(format!("installations/{version}"), root.join("current"))
        .expect("link active installation");
    let driver = root.join("bin/clorraine");
    symlink("../current/bin/clorraine", &driver).expect("link stable driver");
    driver
}

/// Collects a bounded command result, leaving assertion messages to the caller.
pub(super) fn run(command: &mut Command) -> Output {
    command.output().expect("spawn bounded driver process")
}

/// Asserts an observable status, showing both streams on failure.
pub(super) fn exit(output: &Output, expected: i32) {
    assert_eq!(
        output.status.code(), Some(expected),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr)
    );
}

/// Checks both selected programs, the isolated Python vector and absence of
/// PATH fallback. A preflight-only success cannot satisfy these observations.
pub(super) fn selected(f: &Fixture, root: &Path, identity: &str) {
    selected_profile(f, root, identity, "hosted-amiga");
}

/// Checks the same dispatch contract for either existing Amiga runtime profile.
pub(super) fn selected_profile(f: &Fixture, root: &Path, identity: &str, profile: &str) {
    let compiler = f.argv(identity, "compiler");
    assert_eq!(compiler[0], text(&root.join("sdk/amiga/bin/m68k-amigaos-gcc")));
    let runner = f.argv(identity, "runner");
    assert_eq!(runner.len(), 5, "Python invocation has exactly five elements");
    assert_eq!(runner[0], text(&root.join("sdk/vamos/bin/python")));
    assert_eq!(&runner[1..4], &["-I", "-c", ENTRY]);
    let program = f.cwd.join("build/clorraine/hello").join(profile).join("hello");
    assert_eq!(runner[4], text(&program));
    assert!(program.is_absolute());
    assert!(!f.logs.join("legacy-compiler.argv").exists(), "no PATH compiler");
    assert!(!f.logs.join("legacy-runner.argv").exists(), "no bare vamos");
}

/// Changes one exact JSON fragment, failing authoring immediately if a fixture
/// mutation no longer targets the intended field.
pub(super) fn replace(path: &Path, from: &str, to: &str) {
    let original = std::fs::read_to_string(path).expect("read fixture metadata");
    assert!(original.contains(from), "mutation target missing: {from}");
    std::fs::write(path, original.replacen(from, to, 1)).expect("mutate fixture metadata");
}

/// Generates external compiler/runner helpers. Only literal trusted identity
/// tokens enter the script; dynamic paths and program settings use environment
/// data, not generated shell code. Probe invocations do not count as builds.
fn helper(identity: &str, role: &str) -> String {
    assert!(identity.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'));
    assert!(["compiler", "runner", "cc"].contains(&role));
    format!(
        r##"#!/usr/bin/python3
"""External process fixture; not a real compiler or Amiga interpreter."""
import os
from pathlib import Path
import sys
import time

identity = "{identity}"
role = "{role}"
args = sys.argv[1:]
if role in ("compiler", "cc") and "-o" not in args:
    raise SystemExit(0)
if role == "runner" and args == ["--version"]:
    raise SystemExit(0)
record = Path(os.environ["TEST_NATIVE_LOG"]) / (identity + "-" + role + ".argv")
record.write_bytes(b"\0".join(os.fsencode(arg) for arg in sys.argv) + b"\0")
if role == "cc":
    os.execv("/usr/bin/cc", ["/usr/bin/cc", *args])
if role == "compiler":
    barrier = os.environ.get("TEST_NATIVE_BARRIER")
    if barrier:
        ready = Path(barrier + ".ready")
        release = Path(barrier + ".release")
        ready.write_text("ready\n")
        deadline = time.monotonic() + 8
        while not release.exists():
            if time.monotonic() >= deadline:
                raise SystemExit("compiler fixture barrier timed out")
            time.sleep(0.01)
    error = os.environ.get("TEST_NATIVE_COMPILER_ERROR")
    if error:
        sys.stderr.write(error)
        raise SystemExit(19)
    output = Path(args[args.index("-o") + 1])
    output.write_text("#!/bin/sh\nprintf 'cross-should-not-run\\n'\nexit 0\n")
    output.chmod(0o755)
    raise SystemExit(0)
sys.stdout.write(os.environ["TEST_NATIVE_TEXT"])
raise SystemExit(int(os.environ["TEST_NATIVE_STATUS"]))
"##
    )
}
