//! Public contracts for selecting and validating an Amiga toolchain.
//!
//! These tests drive the real driver against small shell compilers, so they
//! exercise command parsing, config persistence, the two-runtime probe, and the
//! absolute compiler path without a real 300 MB toolchain.

#![cfg(target_os = "linux")]

use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

/// One isolated home, config root, work directory, and fake-tool directory.
struct Scratch {
    root: PathBuf,
    home: PathBuf,
    config_home: PathBuf,
    work: PathBuf,
    tools: PathBuf,
}

impl Scratch {
    /// Creates a uniquely named tree under the repository's ignore build area.
    fn new(tag: &str) -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let base = std::env::var_os("CODEOPS_TMPDIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../build/tmp"));
        fs::create_dir_all(&base).expect("create scratch parent");
        let root = loop {
            let candidate = base.join(format!(
                "toolchain-setup-{}-{tag}-{}",
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
        for path in [&home, &config_home, &work, &tools] {
            fs::create_dir_all(path).expect("create scratch directory");
        }
        // The repository test container can be older than the product minimum.
        // A supported getconf keeps setup tests hermetic on every test host.
        let getconf = tools.join("getconf");
        fs::write(&getconf, "#!/bin/sh\nprintf '%s\\n' 'glibc 2.39'\n").expect("write getconf");
        make_executable(&getconf);
        fs::write(
            work.join("hello.clo"),
            "module hello;\nfunc main(): i32 { return 0; }\n",
        )
        .expect("write program");
        Self { root, home, config_home, work, tools }
    }

    /// The config file the CLI owns for this scratch tree.
    fn config_file(&self) -> PathBuf {
        self.config_home.join("clorraine/config")
    }

    /// Runs the driver in this scratch tree with a bounded child process.
    fn run(&self, args: &[&str], env: &[(&str, OsString)]) -> Output {
        let mut path = vec![self.tools.clone()];
        if let Some(value) = std::env::var_os("PATH") {
            path.extend(std::env::split_paths(&value));
        }
        let path = std::env::join_paths(path).expect("join test PATH");
        let mut command = Command::new("/usr/bin/timeout");
        command
            .arg("60s")
            .arg(env!("CARGO_BIN_EXE_clorraine"))
            .args(args)
            .current_dir(&self.work)
            .env("HOME", &self.home)
            .env("XDG_CONFIG_HOME", &self.config_home)
            .env("PATH", path)
            .env_remove("CLORRAINE_TOOLCHAIN")
            .env_remove("CLORRAINE_SDK")
            .env_remove("CLORRAINE_FS_UAE")
            .env_remove("CLORRAINE_KICKSTART");
        for (name, value) in env {
            command.env(name, value);
        }
        command.output().expect("run clorraine")
    }

    /// Creates a fake toolchain root and returns it.
    fn toolchain(&self, name: &str, version: &str, fail_nix13: bool) -> PathBuf {
        let root = self.root.join(name);
        let bin = root.join("bin");
        fs::create_dir_all(&bin).expect("create toolchain bin");
        fs::create_dir_all(root.join("m68k-amigaos/libnix")).expect("create libnix");
        fs::create_dir_all(root.join("m68k-amigaos/ndk-include")).expect("create ndk");
        let compiler = bin.join("m68k-amigaos-gcc");
        let script = format!(
            "#!/bin/sh\n\
             printf '%s %s\\n' \"$0\" \"$*\" >> \"{log}\"\n\
             if [ \"$1\" = \"--version\" ]; then\n\
               printf '%s\\n' '{version}'\n\
               exit 0\n\
             fi\n\
             if [ '{fail}' = 1 ]; then\n\
               case \" $* \" in *' -mcrt=nix13 '*) echo 'probe failure' >&2; exit 1;; esac\n\
             fi\n\
             out=\n\
             prev=\n\
             for arg in \"$@\"; do\n\
               if [ \"$prev\" = \"-o\" ]; then out=$arg; fi\n\
               prev=$arg\n\
             done\n\
             if [ -n \"$out\" ]; then : > \"$out\"; fi\n\
             exit 0\n",
            log = root.join("compiler.log").display(),
            version = version,
            fail = if fail_nix13 { "1" } else { "0" },
        );
        fs::write(&compiler, script).expect("write fake compiler");
        make_executable(&compiler);
        root
    }

    /// Replaces the compiler log with an empty file.
    fn clear_log(&self, root: &Path) {
        let _ = fs::remove_file(root.join("compiler.log"));
    }

    /// Reads the compiler log as lines.
    fn log(&self, root: &Path) -> Vec<String> {
        fs::read_to_string(root.join("compiler.log"))
            .unwrap_or_default()
            .lines()
            .map(str::to_owned)
            .collect()
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

/// Marks a file executable.
fn make_executable(path: &Path) {
    let mut permissions = fs::metadata(path).expect("tool exists").permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).expect("set execute bit");
}

/// Asserts a status and returns stderr text for a readable failure.
fn exit(output: &Output, expected: i32) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    assert_eq!(
        output.status.code(),
        Some(expected),
        "unexpected status; stdout:\n{}\nstderr:\n{stderr}",
        String::from_utf8_lossy(&output.stdout)
    );
    stderr
}

/// The setup command points the config at the canonical toolchain root.
#[test]
fn setup_records_a_valid_toolchain_and_probes_both_runtimes() {
    let s = Scratch::new("valid");
    let root = s.toolchain("amiga-gcc-16.2", "m68k-amigaos-gcc (AmigaDev 16.2-rc24) 16.2.1b", false);
    let output = s.run(&["setup", "--toolchain", root.to_str().unwrap()], &[]);
    exit(&output, 0);
    let config = fs::read_to_string(s.config_file()).expect("config written");
    assert!(
        config.contains(root.to_str().unwrap()),
        "config records the toolchain root, got:\n{config}"
    );
    let log = s.log(&root);
    assert!(
        log.iter().any(|line| line.contains("--version")),
        "setup asks the compiler for its version"
    );
    assert!(
        log.iter().any(|line| line.contains("-mcrt=nix20")),
        "setup probes the nix20 runtime, got:\n{log:#?}"
    );
    assert!(
        log.iter().any(|line| line.contains("-mcrt=nix13")),
        "setup probes the nix13 runtime, got:\n{log:#?}"
    );
}

/// Invalid roots and a wrong version must not replace a good config.
#[test]
fn invalid_setup_leaves_the_previous_config_unchanged() {
    let s = Scratch::new("preserve");
    let good = s.toolchain("good", "m68k-amigaos-gcc (AmigaDev 16.2-rc24) 16.2.1b", false);
    exit(&s.run(&["setup", "--toolchain", good.to_str().unwrap()], &[]), 0);
    let before = fs::read(s.config_file()).expect("config written");

    let missing = s.root.join("not-a-toolchain");
    fs::create_dir_all(missing.join("m68k-amigaos/libnix")).expect("create libnix");
    fs::create_dir_all(missing.join("m68k-amigaos/ndk-include")).expect("create ndk");
    let stderr = exit(&s.run(&["setup", "--toolchain", missing.to_str().unwrap()], &[]), 2);
    assert!(stderr.contains("m68k-amigaos-gcc"), "the error names the missing compiler");
    assert_eq!(fs::read(s.config_file()).expect("config still exists"), before);

    let wrong = s.toolchain("wrong", "m68k-amigaos-gcc (AmigaDev 16.2-rc25) 16.2.1b", false);
    exit(&s.run(&["setup", "--toolchain", wrong.to_str().unwrap()], &[]), 1);
    assert_eq!(fs::read(s.config_file()).expect("config still exists"), before);
}

/// A failing nix13 probe is a setup failure, not a saved toolchain.
#[test]
fn a_failing_runtime_probe_is_rejected_without_replacing_the_config() {
    let s = Scratch::new("probe");
    let good = s.toolchain("good", "m68k-amigaos-gcc (AmigaDev 16.2-rc24) 16.2.1b", false);
    exit(&s.run(&["setup", "--toolchain", good.to_str().unwrap()], &[]), 0);
    let before = fs::read(s.config_file()).expect("good config");
    let root = s.toolchain("probe", "m68k-amigaos-gcc (AmigaDev 16.2-rc24) 16.2.1b", true);
    let stderr = exit(&s.run(&["setup", "--toolchain", root.to_str().unwrap()], &[]), 1);
    assert!(stderr.contains("probe"), "the probe failure is visible, got:\n{stderr}");
    assert_eq!(fs::read(s.config_file()).expect("config preserved"), before);
}

/// A host below the documented glibc floor is rejected before probing.
#[test]
fn an_old_glibc_host_is_rejected() {
    let s = Scratch::new("glibc");
    let getconf = s.tools.join("getconf");
    fs::write(&getconf, "#!/bin/sh\nprintf '%s\\n' 'glibc 2.35'\n").expect("write getconf");
    make_executable(&getconf);
    let root = s.toolchain("selected", "m68k-amigaos-gcc (AmigaDev 16.2-rc24) 16.2.1b", false);
    let stderr = exit(&s.run(&["setup", "--toolchain", root.to_str().unwrap()], &[]), 1);
    assert!(stderr.contains("2.38"), "the floor is named, got:\n{stderr}");
    assert!(!s.config_file().exists(), "old glibc must not save a config");
}

/// Explicit selection wins over the environment, which wins over the config.
#[test]
fn build_precedence_is_flag_then_environment_then_config() {
    let s = Scratch::new("precedence");
    let config = s.toolchain("config", "m68k-amigaos-gcc (AmigaDev 16.2-rc24) 16.2.1b", false);
    let environment = s.toolchain("environment", "m68k-amigaos-gcc (AmigaDev 16.2-rc24) 16.2.1b", false);
    let flag = s.toolchain("flag", "m68k-amigaos-gcc (AmigaDev 16.2-rc24) 16.2.1b", false);
    exit(&s.run(&["setup", "--toolchain", config.to_str().unwrap()], &[]), 0);

    s.clear_log(&flag);
    let out = s.run(&[
        "build", "--profile", "hosted-amiga", "--toolchain", flag.to_str().unwrap(), "hello.clo",
    ], &[]);
    exit(&out, 0);
    let flag_log = s.log(&flag);
    assert!(flag_log.iter().any(|line| line.contains(flag.join("bin").to_str().unwrap())),
        "the flag toolchain is the compiler, got:\n{flag_log:#?}");
    assert!(s.log(&environment).is_empty(), "lower-priority tools must not run");

    s.clear_log(&environment);
    let out = s.run(&["build", "--profile", "hosted-amiga", "hello.clo"],
        &[("CLORRAINE_TOOLCHAIN", environment.as_os_str().to_owned())]);
    exit(&out, 0);
    assert!(s.log(&environment).iter().any(|line| line.contains(environment.join("bin").to_str().unwrap())));

    s.clear_log(&config);
    let out = s.run(&["build", "--profile", "hosted-amiga", "hello.clo"], &[]);
    exit(&out, 0);
    let config_log = s.log(&config);
    assert!(
        config_log.iter().any(|line| line.contains(config.join("bin").to_str().unwrap()) && line.contains("hello.c")),
        "the config compiler ran the build, got:\n{config_log:#?}"
    );
}

/// A selected compiler is invoked by its absolute path, not by a PATH name.
#[test]
fn build_uses_the_absolute_compiler_path() {
    let s = Scratch::new("absolute");
    let root = s.toolchain("a path with spaces", "m68k-amigaos-gcc (AmigaDev 16.2-rc24) 16.2.1b", false);
    exit(&s.run(&["setup", "--toolchain", root.to_str().unwrap()], &[]), 0);
    s.clear_log(&root);
    exit(&s.run(&["build", "--profile", "hosted-amiga", "hello.clo"], &[]), 0);
    let log = s.log(&root);
    let compile = log.last().expect("compiler ran");
    assert!(
        compile.starts_with(&format!("{} ", root.join("bin/m68k-amigaos-gcc").display())),
        "the compiler path is absolute, got:\n{compile}"
    );
}

/// The removed flag and the old environment variable do not select anything.
#[test]
fn removed_sdk_surface_is_not_accepted() {
    let s = Scratch::new("removed");
    let stderr = exit(&s.run(&["build", "--sdk-root", "/tmp", "hello.clo"], &[]), 2);
    assert!(stderr.contains("--sdk-root"), "the old flag is reported unknown, got:\n{stderr}");

    let root = s.toolchain("selected", "m68k-amigaos-gcc (AmigaDev 16.2-rc24) 16.2.1b", false);
    exit(&s.run(&["setup", "--toolchain", root.to_str().unwrap()], &[]), 0);
    s.clear_log(&root);
    let out = s.run(&["build", "--profile", "hosted-amiga", "hello.clo"],
        &[("CLORRAINE_SDK", OsString::from("/no/such/legacy/sdk"))]);
    exit(&out, 0);
    assert!(s.log(&root).iter().any(|line| line.contains(root.join("bin").to_str().unwrap())),
        "the stored toolchain is still selected");
}

/// Setup requires a toolchain and rejects program files.
#[test]
fn setup_requires_a_toolchain_and_no_input_files() {
    let s = Scratch::new("setup-usage");
    let root = s.toolchain("valid", "m68k-amigaos-gcc (AmigaDev 16.2-rc24) 16.2.1b", false);
    exit(&s.run(&["setup"], &[]), 2);
    exit(&s.run(&["setup", "--toolchain", root.to_str().unwrap(), "hello.clo"], &[]), 2);
}

/// The old SDK flag cannot be used on host builds or emit.
#[test]
fn toolchain_is_rejected_for_host_builds_and_emit() {
    let s = Scratch::new("host-usage");
    let root = s.toolchain("valid", "m68k-amigaos-gcc (AmigaDev 16.2-rc24) 16.2.1b", false);
    let stderr = exit(&s.run(&["build", "--toolchain", root.to_str().unwrap(), "hello.clo"], &[]), 2);
    assert!(stderr.contains("--toolchain"), "the error names the flag, got:\n{stderr}");
    exit(&s.run(&["emit", "--toolchain", root.to_str().unwrap(), "hello.clo"], &[]), 2);
}

/// A user host outside the SDK container must select a toolchain first.
#[test]
fn a_host_with_a_path_compiler_but_no_selection_is_told_to_run_setup() {
    if Path::new("/opt/amiga/BUILD-PROVENANCE.txt").exists() {
        return;
    }
    let s = Scratch::new("no-selection");
    let stderr = exit(&s.run(&["build", "--profile", "hosted-amiga", "hello.clo"], &[]), 2);
    assert!(
        stderr.contains("setup --toolchain"),
        "the error points at setup, got:\n{stderr}"
    );
}

/// A native non-emulator run still needs the available `vamos` runner.
#[test]
fn a_native_run_without_vamos_reports_the_missing_runner() {
    let s = Scratch::new("vamos");
    let root = s.toolchain("selected", "m68k-amigaos-gcc (AmigaDev 16.2-rc24) 16.2.1b", false);
    exit(&s.run(&["setup", "--toolchain", root.to_str().unwrap()], &[]), 0);
    let stderr = exit(
        &s.run(
            &["run", "--profile", "hosted-amiga", "--toolchain", root.to_str().unwrap(), "hello.clo"],
            &[("PATH", s.tools.as_os_str().to_owned())],
        ),
        1,
    );
    assert!(stderr.contains("vamos"), "the missing runner is named, got:\n{stderr}");
}

/// An invalid explicit selection must not fall back to the stored good one.
#[test]
fn an_invalid_selection_does_not_fall_back() {
    let s = Scratch::new("fallback");
    let good = s.toolchain("good", "m68k-amigaos-gcc (AmigaDev 16.2-rc24) 16.2.1b", false);
    exit(&s.run(&["setup", "--toolchain", good.to_str().unwrap()], &[]), 0);
    s.clear_log(&good);

    let invalid = s.root.join("invalid");
    fs::create_dir_all(invalid.join("m68k-amigaos/libnix")).expect("libnix");
    fs::create_dir_all(invalid.join("m68k-amigaos/ndk-include")).expect("ndk");
    exit(&s.run(&["build", "--profile", "hosted-amiga", "--toolchain", invalid.to_str().unwrap(), "hello.clo"], &[]), 2);
    assert!(s.log(&good).is_empty(), "the stored toolchain must not run");

    exit(&s.run(&["build", "--profile", "hosted-amiga", "hello.clo"],
        &[("CLORRAINE_TOOLCHAIN", invalid.as_os_str().to_owned())]), 2);
    assert!(s.log(&good).is_empty(), "the stored toolchain must not run after env failure");
}

/// The verified native emulator runner belongs to the next phase; the old
/// native staging path must not be reachable in this build.
#[test]
fn native_emulator_runner_is_explicitly_unavailable() {
    let s = Scratch::new("native-emulator");
    let root = s.toolchain("selected", "m68k-amigaos-gcc (AmigaDev 16.2-rc24) 16.2.1b", false);
    exit(&s.run(&["setup", "--toolchain", root.to_str().unwrap()], &[]), 0);
    let stderr = exit(
        &s.run(
            &["run", "--profile", "hosted-amiga", "--toolchain", root.to_str().unwrap(), "--emulator", "hello.clo"],
            &[],
        ),
        2,
    );
    assert!(
        stderr.contains("verified native FS-UAE runner is not available"),
        "the phase boundary is explicit, got:\n{stderr}"
    );
}
