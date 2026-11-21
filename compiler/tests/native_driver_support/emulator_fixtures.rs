//! External emulator observations without launching an installed FS-UAE.

use super::fixtures::*;
use std::collections::BTreeMap;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};

/// The observer is an executable test double, not an emulator or a validator
/// for actual FS-UAE configuration-loading behavior.
const OBSERVER: &str = include_str!("emulator_observer.py");

/// One native installation, synthetic readable ROM and three distinct emulator
/// executables. All caller-owned files remain under the existing fixture root.
pub(super) struct EmulatorFixture {
    /// Shared compiler fixture, unchanged from the native resolution oracle.
    pub(super) f: Fixture,
    /// Native assembly selected by commands in these tests.
    pub(super) install: PathBuf,
    /// Passive ROM bytes; no real firmware is required by a process double.
    pub(super) rom: PathBuf,
    /// Explicit emulator candidate.
    pub(super) cli: PathBuf,
    /// Lower-priority environment candidate.
    pub(super) environment: PathBuf,
    /// Invocation-local home, never the real user's settings.
    pub(super) home: PathBuf,
    /// Writable temporary parent outside the reset build directory.
    pub(super) temporary: PathBuf,
}

impl EmulatorFixture {
    /// Creates private candidates and replaces only this fixture's existing
    /// PATH emulator guard with the observing helper.
    pub(super) fn new(tag: &str) -> Self {
        let f = Fixture::new(tag);
        let install = f.install("native SDK ; literal", "native", "cp310");
        let rom = f.root.join("synthetic Kickstart ; literal.rom");
        std::fs::write(&rom, b"synthetic ROM fixture\n").unwrap();
        let home = f.root.join("private home");
        let temporary = f.root.join("private temporary");
        std::fs::create_dir_all(&home).unwrap();
        std::fs::create_dir_all(&temporary).unwrap();
        let cli = observer(&f.root.join("cli emulator ; literal"), "cli");
        let environment = observer(&f.root.join("environment emulator"), "environment");
        observer(&f.root.join("path tools/fs-uae"), "path");
        Self { f, install, rom, cli, environment, home, temporary }
    }

    /// Starts from the original command fixture, adding only private emulator
    /// observation settings. Every environment change belongs to this child.
    pub(super) fn command(&self, args: &[&str]) -> Command {
        let mut command = self.f.command(&self.f.driver, args);
        command
            .env("HOME", &self.home)
            .env("XDG_CONFIG_HOME", self.home.join(".config"))
            .env("XDG_CACHE_HOME", self.home.join(".cache"))
            .env("TMPDIR", &self.temporary)
            .env("TEST_EMULATOR_LOG", &self.f.logs)
            .env("TEST_EMULATOR_STATUS", "0")
            .env("TEST_EMULATOR_SIGNAL", "0")
            .env_remove("TEST_EMULATOR_RELEASE");
        command
    }

    /// Builds a public native-emulator invocation with an explicit profile and
    /// source. Additional options precede the source argument.
    pub(super) fn native(&self, profile: &str, extra: &[&str], source: &str) -> Command {
        let sdk = self.install.join("sdk/amiga");
        let mut args = vec!["run", "--profile", profile, "--sdk-root", text(&sdk),
            "--emulator", "--fs-uae", text(&self.cli), "--kickstart", text(&self.rom)];
        args.extend_from_slice(extra);
        args.push(source);
        self.command(&args)
    }

    /// Creates another distinct emulator helper for simultaneous runs.
    pub(super) fn tool(&self, identity: &str) -> PathBuf {
        observer(&self.f.root.join(format!("{identity} emulator")), identity)
    }

    /// Returns one helper's external observation; it survives driver cleanup.
    pub(super) fn record(&self, identity: &str) -> PathBuf {
        self.f.logs.join(format!("{identity}.emulator.json"))
    }

    /// Reads fixed record fields through Python's real JSON decoder rather than
    /// adding a JSON crate or an incomplete Rust JSON parser.
    pub(super) fn fields(&self, identity: &str) -> BTreeMap<String, String> {
        let output = Command::new("timeout")
            .args(["10s", "/usr/bin/python3", "-I"])
            .arg(&self.cli)
            .arg("--fixture-fields")
            .arg(self.record(identity))
            .output().expect("decode emulator observation");
        exit(&output, 0);
        let parts: Vec<_> = output.stdout.split(|byte| *byte == 0).collect();
        let mut fields = BTreeMap::new();
        for pair in parts[..parts.len() - 1].chunks_exact(2) {
            fields.insert(
                String::from_utf8(pair[0].to_vec()).unwrap(),
                String::from_utf8(pair[1].to_vec()).unwrap(),
            );
        }
        fields
    }

    /// Asserts the generated settings, staged files and declared private path
    /// scopes. Actual FS-UAE interpretation still needs the real acceptance run.
    pub(super) fn check(&self, identity: &str, rom: &Path) {
        let output = self.check_command(identity, rom)
            .output().expect("check emulator observation");
        exit(&output, 0);
    }

    /// Also compares the recorded guest bytes with a known completed build.
    /// Call before rebuilding that artifact; live mounted copies must retain
    /// their old bytes after a later build changes the source artifact.
    pub(super) fn check_copy(&self, identity: &str, rom: &Path, program: &Path) {
        let output = self.check_command(identity, rom)
            .arg(program).output().expect("check copied executable");
        exit(&output, 0);
    }

    /// Shares the bounded observation-check command without imposing an
    /// internal configuration or staging-directory basename.
    fn check_command(&self, identity: &str, rom: &Path) -> Command {
        let mut command = Command::new("timeout");
        command.args(["10s", "/usr/bin/python3", "-I"])
            .arg(&self.cli).arg("--fixture-check").arg(self.record(identity))
            .arg(rom).arg(&self.f.cwd);
        command
    }

    /// Parses every line of an emulator-error path with the real JSON
    /// decoder, preserving the established diagnostic object shape.
    pub(super) fn check_error_json(&self, output: &Output) {
        let path = self.f.logs.join("emulator-error.ndjson");
        std::fs::write(&path, &output.stderr).unwrap();
        let result = Command::new("timeout")
            .args(["10s", "/usr/bin/python3", "-I"])
            .arg(&self.environment).arg("--fixture-json-error").arg(path)
            .output().expect("check emulator NDJSON error");
        exit(&result, 0);
    }

    /// A rejected command must not have executed any emulator, even the wrong
    /// lower-priority one. Compiler records are checked separately when needed.
    pub(super) fn assert_no_emulator(&self) {
        for entry in std::fs::read_dir(&self.f.logs).unwrap() {
            let name = entry.unwrap().file_name();
            assert!(!name.to_string_lossy().ends_with(".emulator.json"),
                "rejected invocation launched an emulator");
        }
        assert!(!self.f.logs.join("unexpected-emulator-runner.argv").exists());
    }
}

/// Writes a distinct process helper with no interpolated filesystem paths.
fn observer(path: &Path, identity: &str) -> PathBuf {
    assert!(identity.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'));
    executable(path, &OBSERVER.replace("IDENTITY = \"fixture\"", &format!("IDENTITY = \"{identity}\"")));
    path.to_owned()
}

/// A bounded driver process with file-backed output. Unlike `Command::output`,
/// this lets a test detect driver exit even if its surviving child still holds
/// inherited output descriptors open.
pub(super) struct WaitingRun {
    child: Option<Child>,
    release: PathBuf,
    started: PathBuf,
    pid: PathBuf,
    stdout: PathBuf,
    stderr: PathBuf,
}

impl WaitingRun {
    /// Starts a driver whose emulator helper waits for this run's release file.
    pub(super) fn start(e: &EmulatorFixture, mut command: Command, identity: &str) -> Self {
        let release = e.f.logs.join(format!("{identity}.release"));
        let stdout = e.f.logs.join(format!("{identity}.driver.stdout"));
        let stderr = e.f.logs.join(format!("{identity}.driver.stderr"));
        command
            .env("TEST_EMULATOR_RELEASE", &release)
            .stdout(Stdio::from(File::create(&stdout).unwrap()))
            .stderr(Stdio::from(File::create(&stderr).unwrap()));
        Self {
            child: Some(command.spawn().expect("start bounded driver")),
            release,
            started: e.f.logs.join(format!("{identity}.started")),
            pid: e.f.logs.join(format!("{identity}.pid")),
            stdout,
            stderr,
        }
    }

    /// Waits for a snapshot or an early driver failure. On failure, release and
    /// join before asserting, so no fixture files disappear under a live child.
    pub(super) fn ready(&mut self) {
        let deadline = Instant::now() + Duration::from_secs(10);
        while !self.started.exists() && Instant::now() < deadline {
            if self.child.as_mut().unwrap().try_wait().unwrap().is_some() {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        if !self.started.exists() {
            let output = self.collect();
            exit(&output, 0);
            panic!("emulator never reached its external wait boundary");
        }
    }

    /// Checks the actual driver process status while the child is held. File
    /// output prevents an orphaned child's pipe from masking an early return.
    pub(super) fn assert_waiting(&mut self) {
        std::thread::sleep(Duration::from_millis(80));
        assert!(self.child.as_mut().unwrap().try_wait().unwrap().is_none(),
            "driver must not return before its emulator exits");
    }

    /// Releases the helper and joins the driver before reading its output.
    pub(super) fn finish(mut self) -> Output {
        self.collect()
    }

    /// Joins owned processes on success and failure paths. The outer timeout is
    /// thirty seconds and the emulator's independent wait limit is ten seconds.
    fn collect(&mut self) -> Output {
        std::fs::write(&self.release, b"continue\n").unwrap();
        let status = self.child.take().expect("collect one driver").wait().unwrap();
        self.wait_for_helper();
        Output {
            status,
            stdout: std::fs::read(&self.stdout).unwrap(),
            stderr: std::fs::read(&self.stderr).unwrap(),
        }
    }

    /// If a broken driver abandoned its child, still wait for the released
    /// helper to stop. Linux zombies have stopped using the staged files.
    fn wait_for_helper(&self) {
        // A driver can return before its child finishes loading Python. Allow
        // that bounded startup window before deciding no helper was launched.
        let startup_deadline = Instant::now() + Duration::from_secs(1);
        let pid = loop {
            if let Ok(pid) = std::fs::read_to_string(&self.pid) {
                break pid;
            }
            if Instant::now() >= startup_deadline {
                return;
            }
            std::thread::sleep(Duration::from_millis(10));
        };
        let process = PathBuf::from(format!("/proc/{}/stat", pid.trim()));
        let deadline = Instant::now() + Duration::from_secs(11);
        while Instant::now() < deadline {
            match std::fs::read_to_string(&process) {
                Err(_) => return,
                Ok(stat) if stat.rsplit_once(") ").map(|(_, tail)| tail.starts_with('Z'))
                    .unwrap_or(false) => return,
                _ => std::thread::sleep(Duration::from_millis(10)),
            }
        }
        panic!("owned emulator helper survived its bounded wait");
    }
}

impl Drop for WaitingRun {
    fn drop(&mut self) {
        if self.child.is_some() {
            let _ = self.collect();
        }
    }
}

/// Checks an option rejection is semantic, not merely an old parser's unknown
/// option error, and that neither building nor launching happened.
pub(super) fn usage_rejection(e: &EmulatorFixture, output: &Output, flag: &str) {
    exit(output, 2);
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains(flag), "diagnostic identifies {flag}: {error}");
    assert!(!error.contains("unknown option"), "known option needs applicability/value validation");
    e.f.assert_no_processes();
}

/// Confirms all files observed by the emulator have been cleaned only after
/// process completion, without assuming a private run-directory basename.
pub(super) fn cleaned(fields: &BTreeMap<String, String>) {
    for key in ["config", "drive", "cwd", "base", "cache", "state", "logs"] {
        let path = Path::new(fields.get(key).unwrap_or_else(|| panic!("missing field {key}")));
        assert!(!path.exists(), "private {key} must be cleaned after emulator exit: {}", path.display());
    }
}
