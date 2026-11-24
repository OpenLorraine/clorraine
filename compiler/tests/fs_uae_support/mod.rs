//! Fixtures for the verified FS-UAE runner specification tests.
//!
//! The tests drive the real `clorraine` binary, so the emulator and the Amiga
//! compiler are small shell helpers. The emulator helper honours `--version`,
//! discovers the mounted `CNEXT` drive from `--hard_drive_0=`, and then runs a
//! test-supplied body that writes the guest markers. The compiler helper records
//! its command line, copies the wrapper source for inspection, and creates the
//! requested output file.
//!
//! Every helper and every run lives under the repository's ignored `build/tmp`
//! tree, so a failed assertion leaves its evidence nearby without touching the
//! user's files.

use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

/// The exact version line a pinned rc24 compiler reports.
pub const RC24_VERSION: &str = "m68k-amigaos-gcc (AmigaDev 16.2-rc24) 16.2.1b";

/// One isolated home, config root, work directory, and helper directory.
pub struct Scratch {
    /// Root of the private test tree.
    pub root: PathBuf,
    /// Isolated user home for the driver process.
    pub home: PathBuf,
    /// Isolated XDG config root for the driver process.
    pub config_home: PathBuf,
    /// Working directory holding the `.clo` source and build output.
    pub work: PathBuf,
    /// Directory that stands in for `PATH`.
    pub tools: PathBuf,
    /// Directory where helpers write observations and emulator noise.
    pub logs: PathBuf,
}

impl Scratch {
    /// Creates a uniquely named tree under the repository's ignored build area.
    pub fn new(tag: &str) -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let base = std::env::var_os("CODEOPS_TMPDIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../build/tmp"));
        fs::create_dir_all(&base).expect("create scratch parent");
        let root = loop {
            let candidate = base.join(format!(
                "fs-uae-{}-{tag}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            match fs::create_dir(&candidate) {
                Ok(()) => break candidate,
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => panic!("create scratch root {}: {error}", candidate.display()),
            }
        };
        let root = root.canonicalize().expect("canonical scratch root");
        let home = root.join("home");
        let config_home = root.join("config");
        let work = root.join("work");
        let tools = root.join("path tools");
        let logs = root.join("logs");
        for path in [&home, &config_home, &work, &tools, &logs] {
            fs::create_dir_all(path).expect("create scratch directory");
        }
        fs::write(
            work.join("hello.clo"),
            "module hello;\nfunc main(): i32 { return 0; }\n",
        )
        .expect("write hello program");
        Self { root, home, config_home, work, tools, logs }
    }

    /// Runs the driver in this scratch tree with a bounded child process.
    ///
    /// `PATH` contains only the helper directory, so a test never finds the
    /// host's real FS-UAE by accident. A test that needs the host tools passes
    /// its own `PATH` value in `env`.
    pub fn run(&self, args: &[&str], env: &[(&str, OsString)]) -> Output {
        let mut command = Command::new("/usr/bin/timeout");
        command
            .arg("60s")
            .arg(env!("CARGO_BIN_EXE_clorraine"))
            .args(args)
            .current_dir(&self.work)
            .env("HOME", &self.home)
            .env("XDG_CONFIG_HOME", &self.config_home)
            .env("PATH", &self.tools)
            .env("TEST_FS_UAE_LOG", self.logs.join("fs-uae.log"))
            .env("TEST_PRIVATE_DIR", self.logs.join("private-dir.txt"))
            .env("TEST_LOG", &self.logs)
            .env_remove("CLORRAINE_TOOLCHAIN")
            .env_remove("CLORRAINE_FS_UAE")
            .env_remove("CLORRAINE_KICKSTART")
            .env_remove("CLORRAINE_EMULATOR_TIMEOUT_SECONDS");
        for (name, value) in env {
            command.env(name, value);
        }
        command.output().expect("run clorraine")
    }

    /// Creates a fake upstream toolchain and returns its root.
    ///
    /// The compiler accepts `--version`, any probe, and any compile. It creates
    /// the `-o` output and, when `FAKE_BLOCK_CAPTURE` is set, a directory at
    /// `out.txt` so a capture-write failure can be exercised without changing
    /// the product.
    pub fn toolchain(&self, name: &str) -> PathBuf {
        let root = self.root.join(name);
        let bin = root.join("bin");
        fs::create_dir_all(&bin).expect("create toolchain bin");
        fs::create_dir_all(root.join("m68k-amigaos/libnix")).expect("create libnix");
        fs::create_dir_all(root.join("m68k-amigaos/ndk-include")).expect("create ndk");
        let compiler = bin.join("m68k-amigaos-gcc");
        let script = format!(
            "#!/bin/sh\n\
             printf '%s\\n' \"$0 $*\" >> \"{log}\"\n\
             if [ \"$1\" = \"--version\" ]; then\n\
             \tprintf '%s\\n' '{version}'\n\
             \texit 0\n\
             fi\n\
             out=\n\
             prev=\n\
             for arg in \"$@\"; do\n\
             \tif [ \"$prev\" = \"-o\" ]; then out=$arg; fi\n\
             \tcase \"$arg\" in *emulator_runner.c) cp \"$arg\" \"$TEST_LOG/wrapper.c\" ;; esac\n\
             \tprev=$arg\n\
             done\n\
             if [ -n \"$out\" ]; then\n\
             \t: > \"$out\"\n\
             \tif [ \"${{FAKE_BLOCK_CAPTURE:-0}}\" = 1 ]; then mkdir -p \"$(dirname \"$out\")/out.txt\"; fi\n\
             fi\n\
             exit 0\n",
            log = root.join("compiler.log").display(),
            version = RC24_VERSION,
        );
        fs::write(&compiler, script).expect("write fake compiler");
        make_executable(&compiler);
        root
    }

    /// Creates a fake FS-UAE and returns its executable path.
    ///
    /// `body` runs after the helper has logged its arguments and located the
    /// drive in `$drive`. It can write `clorraine-rc.txt`, the result, and the
    /// status marker.
    pub fn fs_uae(&self, name: &str, version: &str, body: &str) -> PathBuf {
        let path = self.root.join(name);
        let script = format!(
            "#!/bin/sh\n\
             if [ \"$1\" = \"--version\" ]; then\n\
             \tprintf '%s\\n' '@@VERSION@@'\n\
             \texit 0\n\
             fi\n\
             printf '%s\\n' \"$*\" >> \"${{TEST_FS_UAE_LOG:?}}\"\n\
             printf '%s\\n' \"$PWD\" > \"${{TEST_PRIVATE_DIR:?}}\"\n\
             drive=\n\
             for arg in \"$@\"; do\n\
             \tcase \"$arg\" in --hard_drive_0=*) drive=${{arg#--hard_drive_0=}} ;; esac\n\
             done\n\
             @@BODY@@\n"
        )
        .replace("@@VERSION@@", version)
        .replace("@@BODY@@", body);
        fs::write(&path, script).expect("write fake fs-uae");
        make_executable(&path);
        path
    }

    /// Writes a ROM-shaped file of `size` zero bytes.
    pub fn rom(&self, name: &str, size: usize) -> PathBuf {
        let path = self.root.join(name);
        fs::write(&path, vec![0u8; size]).expect("write ROM fixture");
        path
    }

    /// Reads a helper log, returning an empty string when it does not exist.
    pub fn log(&self, name: &str) -> String {
        fs::read_to_string(self.logs.join(name)).unwrap_or_default()
    }

    /// Reads the driver-visible helper log at `path`.
    pub fn read(&self, path: &Path) -> String {
        fs::read_to_string(path).unwrap_or_default()
    }

    /// The private run directory named by a retained-run diagnostic.
    ///
    /// The caller asserts the directory exists; this helper only extracts the
    /// path between `retained at '` and the closing quote.
    pub fn retained(&self, stderr: &str) -> PathBuf {
        let marker = "retained at '";
        let start = stderr
            .find(marker)
            .unwrap_or_else(|| panic!("no retained run in stderr:\n{stderr}"))
            + marker.len();
        let rest = &stderr[start..];
        let end = rest
            .find('\'')
            .unwrap_or_else(|| panic!("unterminated retained path:\n{stderr}"));
        PathBuf::from(&rest[..end])
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

/// Marks a helper executable.
pub fn make_executable(path: &Path) {
    let mut permissions = fs::metadata(path).expect("helper exists").permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).expect("set execute bit");
}

/// Asserts a status and returns stderr text for a readable failure.
pub fn exit(output: &Output, expected: i32) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    assert_eq!(
        output.status.code(),
        Some(expected),
        "unexpected status; stdout:\n{}\nstderr:\n{stderr}",
        String::from_utf8_lossy(&output.stdout)
    );
    stderr
}

/// A guest body that records a successful run and one result line.
pub const SUCCESS: &str = "\
printf '0\\n' > \"$drive/clorraine-rc.txt\"\n\
printf 'ok\\n' > \"$drive/clorraine-status.txt\"\n\
printf 'Hello, Amiga, from Clorraine!\\r\\n' > \"$drive/clorraine-result.txt\"\n";
