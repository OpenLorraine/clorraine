//! Public contracts for the first-run experience: `clorraine new`, the emulator
//! paths remembered by `setup`, and the two shipped Amiga programs.
//!
//! Every case drives the built driver in a private scratch tree with its own
//! home and config directory. The toolchain and FS-UAE are small shell
//! stand-ins, so no real toolchain, emulator, or ROM is needed.

#![cfg(target_os = "linux")]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

/// The supported toolchain's version line.
const RC24: &str = "m68k-amigaos-gcc (AmigaDev 16.2-rc24) 16.2.1b";

/// One isolated home, config root, and work directory.
struct Scratch {
    root: PathBuf,
    home: PathBuf,
    config_home: PathBuf,
    work: PathBuf,
    tools: PathBuf,
}

impl Scratch {
    /// Creates a uniquely named tree under the repository's build area.
    fn new(tag: &str) -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let base = std::env::var_os("CODEOPS_TMPDIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../build/tmp"));
        fs::create_dir_all(&base).expect("create scratch parent");
        let root = loop {
            let candidate = base.join(format!(
                "first-run-{}-{tag}-{}",
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
        let tools = root.join("tools");
        for path in [&home, &config_home, &work, &tools] {
            fs::create_dir_all(path).expect("create scratch directory");
        }
        let getconf = tools.join("getconf");
        fs::write(&getconf, "#!/bin/sh\nprintf '%s\\n' 'glibc 2.39'\n").expect("write getconf");
        make_executable(&getconf);
        Self { root, home, config_home, work, tools }
    }

    /// The config file the CLI owns for this scratch tree.
    fn config_file(&self) -> PathBuf {
        self.config_home.join("clorraine/config")
    }

    /// Runs the driver by its absolute path in the work directory.
    fn run(&self, args: &[&str]) -> Output {
        self.run_in(&self.work, args)
    }

    /// Runs the driver in the work directory with extra environment.
    fn run_env(&self, args: &[&str], env: &[(&str, &str)]) -> Output {
        self.command(&self.work, args, env)
    }

    /// Runs the driver by its absolute path in `cwd`.
    fn run_in(&self, cwd: &Path, args: &[&str]) -> Output {
        self.command(cwd, args, &[])
    }

    /// Runs the driver by its absolute path in `cwd` with extra environment.
    fn command(&self, cwd: &Path, args: &[&str], env: &[(&str, &str)]) -> Output {
        let mut path = vec![self.tools.clone()];
        if let Some(value) = std::env::var_os("PATH") {
            path.extend(std::env::split_paths(&value));
        }
        let path = std::env::join_paths(path).expect("join test PATH");
        Command::new("/usr/bin/timeout")
            .arg("60s")
            .arg(env!("CARGO_BIN_EXE_clorraine"))
            .args(args)
            .current_dir(cwd)
            .env("HOME", &self.home)
            .env("XDG_CONFIG_HOME", &self.config_home)
            .env("PATH", path)
            .env_remove("CLORRAINE_TOOLCHAIN")
            .env_remove("CLORRAINE_FS_UAE")
            .env_remove("CLORRAINE_KICKSTART")
            .env("CLORRAINE_EMULATOR_TIMEOUT_SECONDS", "5")
            .envs(env.iter().copied())
            .output()
            .expect("run clorraine")
    }

    /// Creates a fake rc24 toolchain root whose compiler accepts any build.
    fn toolchain(&self) -> PathBuf {
        let root = self.root.join("rc24");
        let bin = root.join("bin");
        fs::create_dir_all(&bin).expect("create toolchain bin");
        fs::create_dir_all(root.join("m68k-amigaos/libnix")).expect("create libnix");
        fs::create_dir_all(root.join("m68k-amigaos/ndk-include")).expect("create ndk");
        let compiler = bin.join("m68k-amigaos-gcc");
        let script = format!(
            "#!/bin/sh\n\
             if [ \"$1\" = \"--version\" ]; then printf '%s\\n' '{RC24}'; exit 0; fi\n\
             out=\n\
             prev=\n\
             for arg in \"$@\"; do\n\
               if [ \"$prev\" = \"-o\" ]; then out=$arg; fi\n\
               prev=$arg\n\
             done\n\
             if [ -n \"$out\" ]; then : > \"$out\"; fi\n\
             exit 0\n"
        );
        fs::write(&compiler, script).expect("write fake compiler");
        make_executable(&compiler);
        root
    }

    /// Creates a fake FS-UAE that reports `version`.
    fn fs_uae(&self, name: &str, version: &str) -> PathBuf {
        let path = self.root.join(name);
        fs::write(&path, format!("#!/bin/sh\nprintf '%s\\n' '{version}'\n")).expect("write fs-uae");
        make_executable(&path);
        path
    }

    /// Creates a Kickstart stand-in of `bytes` bytes.
    fn rom(&self, name: &str, bytes: usize) -> PathBuf {
        let path = self.root.join(name);
        fs::write(&path, vec![0u8; bytes]).expect("write rom");
        path
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

/// Asserts a status and returns stdout for further checks.
fn exit(output: &Output, expected: i32) -> String {
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    assert_eq!(
        output.status.code(),
        Some(expected),
        "unexpected status; stdout:\n{stdout}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    stdout
}

/// The stderr text of a finished command.
fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// The committed starter template.
fn template(name: &str) -> String {
    fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("templates/starter").join(name))
        .expect("read template")
}

/// `new DIR` writes the program, the editor setup, and a README, and says
/// what to do next.
#[test]
fn new_creates_a_runnable_starter_project() {
    let s = Scratch::new("new");
    let stdout = exit(&s.run(&["new", "my-app"]), 0);
    let project = s.work.join("my-app");
    assert_eq!(fs::read_to_string(project.join("main.clo")).expect("main.clo"), template("main.clo"));
    assert_eq!(fs::read_to_string(project.join("README.md")).expect("README"), template("README.md"));
    assert!(fs::read_to_string(project.join(".gitignore")).expect(".gitignore").contains("build/"));
    let settings = fs::read_to_string(project.join(".vscode/settings.json")).expect("settings");
    assert!(settings.contains("\"clorraine.profile\": \"hosted-amiga\""), "{settings}");
    let extensions = fs::read_to_string(project.join(".vscode/extensions.json")).expect("extensions");
    assert!(extensions.contains("openlorraine.clorraine"), "{extensions}");
    let tasks = fs::read_to_string(project.join(".vscode/tasks.json")).expect("tasks");
    assert!(tasks.contains("\"run\", \"-p\", \"hosted-amiga\", \"--emulator\", \"main.clo\""), "{tasks}");
    assert!(tasks.contains("\"isDefault\": true"), "the run task is the default build task: {tasks}");
    assert!(!tasks.contains("@CLORRAINE@"), "no launcher placeholder: {tasks}");
    assert!(stdout.contains("cd my-app"), "next steps name the folder:\n{stdout}");
    assert!(stdout.contains("Run in FS-UAE"), "next steps name the editor action:\n{stdout}");
}

/// The tasks ask the Clorraine extension for the driver instead of naming a
/// path, so the project works on any machine with Clorraine installed, even
/// when the editor's PATH lacks the install directory.
#[test]
fn new_points_the_tasks_at_the_extension_driver_command() {
    let s = Scratch::new("launcher");
    exit(&s.run(&["new", "app"]), 0);
    let tasks = fs::read_to_string(s.work.join("app/.vscode/tasks.json")).expect("tasks");
    let command = "\"command\": \"${command:clorraine.driverPath}\"";
    assert_eq!(tasks.matches(command).count(), 2, "both tasks use the driver command: {tasks}");
    assert_eq!(tasks.matches("\"command\":").count(), 2, "no other task command: {tasks}");
    let launcher = env!("CARGO_BIN_EXE_clorraine");
    assert!(!tasks.contains(launcher), "no machine-specific launcher path: {tasks}");
    assert!(!tasks.contains("@CLORRAINE@"), "{tasks}");
}

/// Without a name the project is `amiga-starter` in the current directory.
#[test]
fn new_defaults_to_amiga_starter() {
    let s = Scratch::new("default");
    exit(&s.run(&["new"]), 0);
    assert!(s.work.join("amiga-starter/main.clo").is_file());
    assert!(s.work.join("amiga-starter/.vscode/tasks.json").is_file());
}

/// An existing empty directory is used; a non-empty one is never touched.
#[test]
fn new_refuses_a_non_empty_directory() {
    let s = Scratch::new("refuse");
    fs::create_dir_all(s.work.join("empty")).expect("empty dir");
    exit(&s.run(&["new", "empty"]), 0);
    assert!(s.work.join("empty/main.clo").is_file());

    let busy = s.work.join("busy");
    fs::create_dir_all(&busy).expect("busy dir");
    fs::write(busy.join("notes.txt"), "keep me").expect("existing file");
    let output = s.run(&["new", "busy"]);
    exit(&output, 2);
    assert!(stderr(&output).contains("not empty"), "{}", stderr(&output));
    assert_eq!(fs::read_to_string(busy.join("notes.txt")).expect("kept"), "keep me");
    assert!(!busy.join("main.clo").exists(), "nothing is written into a busy folder");

    fs::write(s.work.join("file"), "x").expect("plain file");
    exit(&s.run(&["new", "file"]), 2);
}

/// `new` takes at most one folder and no build options.
#[test]
fn new_rejects_extra_arguments() {
    let s = Scratch::new("args");
    for line in [
        vec!["new", "a", "b"],
        vec!["new", "--profile", "hosted-amiga", "a"],
        vec!["new", "--emulator", "a"],
    ] {
        let output = s.run(&line);
        exit(&output, 2);
        assert!(stderr(&output).contains("'new' accepts"), "{line:?}: {}", stderr(&output));
    }
    assert!(!s.work.join("a").exists(), "a rejected line writes nothing");
}

/// The help text lists the new command.
#[test]
fn help_lists_new() {
    let s = Scratch::new("help");
    let stdout = exit(&s.run(&["--help"]), 0);
    assert!(stdout.contains("new "), "{stdout}");
}

/// `setup` stores optional Kickstart and FS-UAE paths beside the toolchain.
#[test]
fn setup_remembers_the_emulator_paths() {
    let s = Scratch::new("setup");
    let root = s.toolchain();
    let fs_uae = s.fs_uae("fs-uae", "3.2.35");
    let rom = s.rom("kick31.rom", 524_288);
    let stdout = exit(&s.run(&[
        "setup", "--toolchain", root.to_str().unwrap(),
        "--kickstart", rom.to_str().unwrap(), "--fs-uae", fs_uae.to_str().unwrap(),
    ]), 0);
    let config = fs::read_to_string(s.config_file()).expect("config");
    assert!(config.contains(&format!("toolchain={}", root.display())), "{config}");
    assert!(config.contains(&format!("kickstart={}", rom.display())), "{config}");
    assert!(config.contains(&format!("fs-uae={}", fs_uae.display())), "{config}");
    assert!(stdout.contains("Kickstart"), "{stdout}");

    // A later toolchain-only setup keeps the remembered emulator paths.
    exit(&s.run(&["setup", "--toolchain", root.to_str().unwrap()]), 0);
    let config = fs::read_to_string(s.config_file()).expect("config");
    assert!(config.contains("kickstart="), "{config}");
    assert!(config.contains("fs-uae="), "{config}");
}

/// Invalid emulator media are rejected and the old config is kept.
#[test]
fn setup_rejects_invalid_emulator_paths() {
    let s = Scratch::new("invalid");
    let root = s.toolchain();
    exit(&s.run(&["setup", "--toolchain", root.to_str().unwrap()]), 0);
    let before = fs::read_to_string(s.config_file()).expect("config");

    let small = s.rom("kick13.rom", 262_144);
    let output = s.run(&["setup", "--toolchain", root.to_str().unwrap(), "--kickstart", small.to_str().unwrap()]);
    exit(&output, 2);
    assert!(stderr(&output).contains("--kickstart"), "{}", stderr(&output));

    let old = s.fs_uae("old-fs-uae", "3.1.66");
    let output = s.run(&["setup", "--toolchain", root.to_str().unwrap(), "--fs-uae", old.to_str().unwrap()]);
    exit(&output, 2);
    assert!(stderr(&output).contains("3.2.0"), "{}", stderr(&output));

    assert_eq!(fs::read_to_string(s.config_file()).expect("config"), before);
}

/// `setup` repairs a config it cannot read instead of refusing to run.
#[test]
fn setup_replaces_a_malformed_config() {
    let s = Scratch::new("repair");
    let root = s.toolchain();
    fs::create_dir_all(s.config_file().parent().unwrap()).expect("config dir");
    fs::write(s.config_file(), "toolchain=/old\nkickstart=\n").expect("malformed config");
    exit(&s.run(&["setup", "--toolchain", root.to_str().unwrap()]), 0);
    let config = fs::read_to_string(s.config_file()).expect("config");
    assert_eq!(config, format!("toolchain={}\n", root.display()));
}

/// `run --emulator` uses the remembered paths when no flag or environment
/// variable names them.
#[test]
fn run_uses_the_remembered_emulator_paths() {
    let s = Scratch::new("remembered");
    let root = s.toolchain();
    let fs_uae = s.fs_uae("fs-uae", "3.2.35");
    let rom = s.rom("kick31.rom", 524_288);
    exit(&s.run(&[
        "setup", "--toolchain", root.to_str().unwrap(),
        "--kickstart", rom.to_str().unwrap(), "--fs-uae", fs_uae.to_str().unwrap(),
    ]), 0);
    fs::write(s.work.join("hello.clo"), "module hello;\nfunc main(): i32 { return 0; }\n").expect("program");

    // Shrinking the remembered ROM proves that run read it from the config.
    fs::write(&rom, vec![0u8; 262_144]).expect("shrink rom");
    let output = s.run(&["run", "-p", "hosted-amiga", "--emulator", "hello.clo"]);
    exit(&output, 2);
    let text = stderr(&output);
    assert!(text.contains("256 KB"), "the remembered ROM was checked: {text}");

    // An old remembered FS-UAE is reported before the ROM.
    fs::write(&fs_uae, "#!/bin/sh\nprintf '%s\\n' '3.1.0'\n").expect("downgrade fs-uae");
    let output = s.run(&["run", "-p", "hosted-amiga", "--emulator", "hello.clo"]);
    exit(&output, 2);
    assert!(stderr(&output).contains("3.2.0"), "the remembered FS-UAE was checked: {}", stderr(&output));
}

/// A flag or the environment still wins over a remembered path, and a stale
/// remembered path names the config and the way to change it.
#[test]
fn flags_and_the_environment_beat_remembered_paths() {
    let s = Scratch::new("precedence");
    let root = s.toolchain();
    let fs_uae = s.fs_uae("fs-uae", "3.2.35");
    let good = s.rom("kick31.rom", 524_288);
    exit(&s.run(&[
        "setup", "--toolchain", root.to_str().unwrap(),
        "--kickstart", good.to_str().unwrap(), "--fs-uae", fs_uae.to_str().unwrap(),
    ]), 0);
    fs::write(s.work.join("hello.clo"), "module hello;\nfunc main(): i32 { return 0; }\n").expect("program");
    // Make the remembered ROM invalid, then name a valid one another way.
    fs::write(&good, vec![0u8; 262_144]).expect("shrink remembered rom");
    let other = s.rom("other.rom", 524_288);

    let output = s.run(&["run", "-p", "hosted-amiga", "--emulator", "hello.clo"]);
    exit(&output, 2);
    let text = stderr(&output);
    assert!(text.contains("remembered in"), "a stale remembered path names its source: {text}");
    assert!(text.contains("clorraine setup"), "and how to change it: {text}");

    // A valid ROM named by the flag or the environment is used instead, so
    // the stale remembered ROM is never checked. (The stand-in FS-UAE then
    // fails the run itself, which is not what this case is about.)
    let output = s.run(&["run", "-p", "hosted-amiga", "--emulator", "--kickstart", other.to_str().unwrap(), "hello.clo"]);
    let text = stderr(&output);
    assert!(!text.contains("256 KB"), "the flag ROM wins: {text}");
    let output = s.run_env(&["run", "-p", "hosted-amiga", "--emulator", "hello.clo"],
        &[("CLORRAINE_KICKSTART", other.to_str().unwrap())]);
    let text = stderr(&output);
    assert!(!text.contains("256 KB"), "the environment ROM wins: {text}");
}

/// The starter and the installer's Amiga hello compile for the Amiga with no
/// diagnostics at all.
#[test]
fn the_shipped_programs_compile_without_warnings() {
    let s = Scratch::new("programs");
    let repository = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    for program in [
        repository.join("compiler/templates/starter/main.clo"),
        repository.join("examples/amiga/hello-intro.clo"),
    ] {
        let output = s.run(&["emit", "-p", "hosted-amiga", "-o", "out.c", program.to_str().unwrap()]);
        exit(&output, 0);
        assert!(stderr(&output).is_empty(), "{} has diagnostics:\n{}", program.display(), stderr(&output));
    }
}
