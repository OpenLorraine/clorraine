//! End-to-end specification tests for the `clorraine` build/run driver.
//!
//! Every case spawns the built `clorraine` binary inside its own private
//! scratch directory and asserts the observable command-line behaviour: what is
//! printed, which artifact is produced, and the process exit code. The `.clo`
//! fixtures are written into that scratch directory at test time, so the
//! repository tree is never touched.
//!
//! The host cases need a C compiler and run on a bare host. The hosted-amiga
//! cases need the SDK toolchain (`m68k-amigaos-gcc` and `vamos`); they skip
//! themselves with a note when those tools are absent, so a bare-host run
//! reports no false failure.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

/// Temporary directory that removes itself when the test finishes.
struct ScratchDir {
    path: PathBuf,
}

impl ScratchDir {
    /// Creates a unique directory for one test process and tag.
    fn new(tag: &str) -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "clorraine-driver-{}-{tag}-{sequence}",
            std::process::id()
        ));
        std::fs::create_dir_all(&path)
            .unwrap_or_else(|err| panic!("cannot create {}: {err}", path.display()));
        Self { path }
    }

    /// Path of the scratch directory.
    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for ScratchDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

/// Writes a source fixture into the scratch directory and returns its path.
fn write_fixture(scratch: &ScratchDir, name: &str, source: &str) -> PathBuf {
    let path = scratch.path().join(name);
    std::fs::write(&path, source)
        .unwrap_or_else(|err| panic!("cannot write fixture {name}: {err}"));
    path
}

/// Runs the built driver with `args`, using `cwd` as its working directory.
fn run_driver(cwd: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_clorraine"))
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("failed to spawn the clorraine driver")
}

/// Runs the built `clc` compiler with `args`, using `cwd` as its working
/// directory.
fn run_clc(cwd: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_clc"))
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("failed to spawn the clc compiler")
}

/// True when `tool` can be spawned, so a case can skip on a host without it.
fn tool_available(tool: &str) -> bool {
    Command::new(tool).arg("--version").output().is_ok()
}

/// The driver's artifact directory for one source stem and profile.
fn artifact_dir(cwd: &Path, stem: &str, profile: &str) -> PathBuf {
    cwd.join("build")
        .join("clorraine")
        .join(stem)
        .join(profile)
}

/// The captured standard output as text.
fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// The captured standard error as text.
fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// Asserts `output` exited with `expected`, showing both streams on mismatch.
fn assert_exit(tag: &str, output: &Output, expected: i32) {
    assert_eq!(
        output.status.code(),
        Some(expected),
        "{tag}: expected exit {expected}; stdout:\n{}\nstderr:\n{}",
        stdout(output),
        stderr(output)
    );
}

/// Emits the same source through `clc` and returns the reference C text.
///
/// `clorraine emit` is defined to print exactly what the compiler writes for
/// the same profile and inputs, so the reference is produced by the compiler
/// itself rather than hardcoded.
fn reference_c(scratch: &ScratchDir, source_name: &str) -> String {
    let compile = run_clc(
        scratch.path(),
        &["--profile", "host", "-o", "reference.c", source_name],
    );
    assert_exit("reference", &compile, 0);
    std::fs::read_to_string(scratch.path().join("reference.c"))
        .expect("the compiler must write the reference C")
}

/// A program that prints one line and returns zero.
const HELLO_SOURCE: &str = r#"module hello;

import { println } from std.io;

func main(): i32 {
    println("Hello, Clorraine!");
    return 0;
}
"#;

/// A program whose `main` returns seven.
const EXIT7_SOURCE: &str = r#"module exit7;

func main(): i32 {
    return 7;
}
"#;

/// A program that fails to parse: `return` has no operand.
const BAD_SOURCE: &str = r#"module bad;

func main(): i32 {
    let = 5;
    return 0;
}
"#;

/// `run` with no profile builds and runs on the host and shows only the
/// program's own output.
///
/// The C compiler is an implementation detail: on success the process writes
/// the program's stdout and nothing on stderr.
#[test]
fn run_default_profile_prints_program_output() {
    let scratch = ScratchDir::new("run-hello");
    write_fixture(&scratch, "hello.clo", HELLO_SOURCE);

    let output = run_driver(scratch.path(), &["run", "hello.clo"]);
    assert_exit("run-hello", &output, 0);
    assert_eq!(
        stdout(&output),
        "Hello, Clorraine!\n",
        "run-hello: the program output is the only stdout"
    );
    assert!(
        stderr(&output).is_empty(),
        "run-hello: no C toolchain text may be shown, got:\n{}",
        stderr(&output)
    );
}

/// `build` produces the host executable under the artifact directory and does
/// not run the program.
#[test]
fn build_writes_executable_without_running() {
    let scratch = ScratchDir::new("build-hello");
    write_fixture(&scratch, "hello.clo", HELLO_SOURCE);

    let output = run_driver(scratch.path(), &["build", "hello.clo"]);
    assert_exit("build-hello", &output, 0);
    let binary = artifact_dir(scratch.path(), "hello", "host").join("hello");
    assert!(
        binary.is_file(),
        "build-hello: the executable must exist at {}",
        binary.display()
    );
    assert!(
        !stdout(&output).contains("Hello, Clorraine!"),
        "build-hello: a build must not run the program"
    );
}

/// `emit` prints, on stdout, the same C translation unit `clc` writes for the
/// same source and profile.
#[test]
fn emit_prints_the_translation_unit_to_stdout() {
    let scratch = ScratchDir::new("emit-hello");
    write_fixture(&scratch, "hello.clo", HELLO_SOURCE);

    let emitted = run_driver(scratch.path(), &["emit", "hello.clo"]);
    assert_exit("emit-hello", &emitted, 0);
    let reference = reference_c(&scratch, "hello.clo");
    assert!(
        !reference.trim().is_empty(),
        "emit-hello: the reference translation unit must not be empty"
    );
    assert_eq!(
        stdout(&emitted),
        reference,
        "emit-hello: emit stdout must match clc's translation unit"
    );
}

/// `emit -o FILE` writes the translation unit to that file instead of stdout.
#[test]
fn emit_output_flag_writes_the_translation_unit() {
    let scratch = ScratchDir::new("emit-out");
    write_fixture(&scratch, "hello.clo", HELLO_SOURCE);

    let emitted = run_driver(scratch.path(), &["emit", "-o", "out.c", "hello.clo"]);
    assert_exit("emit-out", &emitted, 0);
    let reference = reference_c(&scratch, "hello.clo");
    let written = std::fs::read_to_string(scratch.path().join("out.c"))
        .expect("emit-out: -o must write out.c");
    assert_eq!(
        written, reference,
        "emit-out: the written file must hold the translation unit"
    );
    assert!(
        stdout(&emitted).is_empty(),
        "emit-out: with -o the C is written, not printed"
    );
}

/// `run --profile hosted-amiga` builds with the Amiga toolchain and runs the
/// program under `vamos`.
///
/// The SDK container provides `m68k-amigaos-gcc` and `vamos`; on a host without
/// them the case prints a note and returns, because it cannot run for real.
#[test]
fn hosted_amiga_run_prints_program_output_via_vamos() {
    if !tool_available("m68k-amigaos-gcc") || !tool_available("vamos") {
        eprintln!("skipping the hosted-amiga run: the Amiga SDK tools are not on PATH");
        return;
    }
    let scratch = ScratchDir::new("run-amiga");
    write_fixture(&scratch, "hello.clo", HELLO_SOURCE);

    let output = run_driver(
        scratch.path(),
        &["run", "--profile", "hosted-amiga", "hello.clo"],
    );
    assert_exit("run-amiga", &output, 0);
    assert!(
        stdout(&output).contains("Hello, Clorraine!"),
        "run-amiga: vamos must run the program, got:\n{}",
        stdout(&output)
    );
}

/// `run --capture` writes the program output to `out.txt`, prints that path,
/// and does not stream the output.
#[test]
fn run_capture_writes_out_txt_and_prints_its_path() {
    let scratch = ScratchDir::new("run-capture");
    write_fixture(&scratch, "hello.clo", HELLO_SOURCE);

    let output = run_driver(scratch.path(), &["run", "--capture", "hello.clo"]);
    assert_exit("run-capture", &output, 0);

    let out_path = artifact_dir(scratch.path(), "hello", "host").join("out.txt");
    let captured = std::fs::read_to_string(&out_path)
        .unwrap_or_else(|err| panic!("run-capture: out.txt must exist: {err}"));
    assert!(
        captured.contains("Hello, Clorraine!"),
        "run-capture: out.txt holds the program output, got:\n{captured}"
    );

    let printed = stdout(&output);
    assert!(
        printed.contains("build/clorraine/hello/host/out.txt"),
        "run-capture: the captured path is printed, got:\n{printed}"
    );
    assert!(
        !printed.contains("Hello, Clorraine!"),
        "run-capture: captured output must not be streamed"
    );
}

/// `run` exits with the program's own exit code.
#[test]
fn run_propagates_the_program_exit_code() {
    let scratch = ScratchDir::new("run-exit7");
    write_fixture(&scratch, "exit7.clo", EXIT7_SOURCE);

    let output = run_driver(scratch.path(), &["run", "exit7.clo"]);
    assert_exit("run-exit7", &output, 7);
}

/// `--emulator` prints the exact FS-UAE host handoff and exits zero; with
/// `--capture` the printed command carries the `RUN_CAPTURE=1` prefix.
///
/// The driver builds the hosted-amiga program before it prints the handoff, so
/// a host without the Amiga toolchain prints a note and returns instead of
/// reporting a false failure.
#[test]
fn hosted_amiga_emulator_prints_the_fs_uae_handoff() {
    if !tool_available("m68k-amigaos-gcc") {
        eprintln!("emulator-handoff: skipped (m68k-amigaos-gcc is not installed)");
        return;
    }
    let scratch = ScratchDir::new("emulator-handoff");
    write_fixture(&scratch, "hello.clo", HELLO_SOURCE);

    let plain = run_driver(
        scratch.path(),
        &[
            "run",
            "--profile",
            "hosted-amiga",
            "--emulator",
            "hello.clo",
        ],
    );
    assert_exit("emulator-handoff", &plain, 0);
    assert_eq!(
        stdout(&plain),
        "FS-UAE must run on the host. From the repository root:\n  \
         make -C docker run PROG='build/clorraine/hello/hosted-amiga/hello'\n",
        "emulator-handoff: the printed command must be exact"
    );

    let captured = run_driver(
        scratch.path(),
        &[
            "run",
            "--profile",
            "hosted-amiga",
            "--emulator",
            "--capture",
            "hello.clo",
        ],
    );
    assert_exit("emulator-handoff-capture", &captured, 0);
    assert_eq!(
        stdout(&captured),
        "FS-UAE must run on the host. From the repository root:\n  \
         RUN_CAPTURE=1 make -C docker run PROG='build/clorraine/hello/hosted-amiga/hello'\n",
        "emulator-handoff-capture: capture prefixes the printed command"
    );
}

/// A Clorraine parse error is reported on stderr, the driver exits one, and no
/// artifact is produced.
#[test]
fn a_syntax_error_is_reported_and_leaves_no_artifact() {
    let scratch = ScratchDir::new("run-bad");
    write_fixture(&scratch, "bad.clo", BAD_SOURCE);

    let output = run_driver(scratch.path(), &["run", "bad.clo"]);
    assert_exit("run-bad", &output, 1);
    assert!(
        stderr(&output).contains("error E"),
        "run-bad: the compiler diagnostic is shown, got:\n{}",
        stderr(&output)
    );

    let dir = artifact_dir(scratch.path(), "bad", "host");
    assert!(
        !dir.join("bad.c").exists(),
        "run-bad: no emitted C on a diagnostic"
    );
    assert!(
        !dir.join("bad").exists(),
        "run-bad: no executable on a diagnostic"
    );
}

/// Every malformed invocation is a usage error on stderr and exits two.
#[test]
fn usage_errors_exit_two() {
    let scratch = ScratchDir::new("usage");
    write_fixture(&scratch, "hello.clo", HELLO_SOURCE);

    let cases: [&[&str]; 5] = [
        &["frobnicate", "hello.clo"],
        &["run", "--nope", "hello.clo"],
        &["run"],
        &["run", "-o", "x.c", "hello.clo"],
        &["run", "--emulator", "hello.clo"],
    ];
    for args in cases {
        let output = run_driver(scratch.path(), args);
        assert_exit("usage", &output, 2);
        assert!(
            !stderr(&output).is_empty(),
            "usage: a usage error is printed for {args:?}"
        );
    }
}

/// `--help` and `--version` print their text to stdout and exit zero.
#[test]
fn help_and_version_print_to_stdout() {
    let scratch = ScratchDir::new("help-version");

    let help = run_driver(scratch.path(), &["--help"]);
    assert_exit("help", &help, 0);
    let help_text = stdout(&help);
    for command in ["run", "build", "emit"] {
        assert!(
            help_text.contains(command),
            "help: the {command} command must be listed, got:\n{help_text}"
        );
    }

    let version = run_driver(scratch.path(), &["--version"]);
    assert_exit("version", &version, 0);
    let version_text = stdout(&version);
    assert!(
        !version_text.trim().is_empty(),
        "version: a version line is printed"
    );
    assert!(
        version_text.contains("clorraine"),
        "version: the version line names the tool, got:\n{version_text}"
    );
}

/// Several source files form one program, and the artifact directory uses the
/// first file's stem.
///
/// `a.clo` imports a function defined in `b.clo`; the run only succeeds when
/// both files are passed to the compiler, and the artifact lands under `a`.
#[test]
fn multiple_sources_use_the_first_file_stem() {
    let scratch = ScratchDir::new("multi-file");
    write_fixture(
        &scratch,
        "a.clo",
        "module prog.a;\n\nimport { pick } from prog.b;\n\nfunc main(): i32 {\n    return pick();\n}\n",
    );
    write_fixture(
        &scratch,
        "b.clo",
        "module prog.b;\n\nexport func pick(): i32 {\n    return 0;\n}\n",
    );

    let output = run_driver(scratch.path(), &["run", "a.clo", "b.clo"]);
    assert_exit("multi-file", &output, 0);
    assert!(
        artifact_dir(scratch.path(), "a", "host").is_dir(),
        "multi-file: the artifact stem is the first file's"
    );
    assert!(
        !artifact_dir(scratch.path(), "b", "host").exists(),
        "multi-file: the second file does not name the artifact"
    );
}

/// A source path containing spaces and shell metacharacters is passed to the
/// compiler as one literal argument, with no shell interpretation.
#[test]
fn a_path_with_shell_metacharacters_is_passed_literally() {
    let scratch = ScratchDir::new("weird-name");
    let name = "weird ; name.clo";
    write_fixture(&scratch, name, HELLO_SOURCE);

    let output = run_driver(scratch.path(), &["emit", name]);
    assert_exit("weird-name", &output, 0);
    assert!(
        !stdout(&output).trim().is_empty(),
        "weird-name: the translation unit is emitted for the literal path"
    );
}

/// The two GCC release rows match the shipped flags; the other six rows are
/// pinned unchanged, with the freestanding row's pre-existing wording recorded
/// in the findings.
#[test]
fn the_two_gcc_release_rows_match_the_shipped_flags() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../spec/17-codegen.md");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("cannot read {}: {err}", path.display()));
    let rows: Vec<&str> = text
        .lines()
        .filter(|line| {
            (line.starts_with("| GCC ")
                || line.starts_with("| vbcc ")
                || line.starts_with("| freestanding "))
                && (line.contains("demo.c") || line.starts_with("| freestanding release "))
        })
        .collect();
    let expected = vec![
        "| GCC debug/development | `m68k-amigaos-gcc -std=c99 -mcrt=nix20 -m68000 -fwrapv -O0 -g -Wall -Wextra -Werror -o demo demo.c -lamiga -lm` |",
        "| GCC release | `m68k-amigaos-gcc -std=c99 -mcrt=nix20 -m68000 -fwrapv -O2 -ffunction-sections -fdata-sections -Wl,--gc-sections -s -o demo demo.c -lamiga -lm` |",
        "| GCC 1.3 debug/development | `m68k-amigaos-gcc -std=c99 -mcrt=nix13 -m68000 -fwrapv -O0 -g -Wall -Wextra -Werror -o demo demo.c -lamiga -lm` |",
        "| GCC 1.3 release | `m68k-amigaos-gcc -std=c99 -mcrt=nix13 -m68000 -fwrapv -O2 -ffunction-sections -fdata-sections -Wl,--gc-sections -s -o demo demo.c -lamiga -lm` |",
        "| vbcc release | `vc +aos68k -c99 -O2 -cpu=68000 -I/opt/amiga/m68k-amigaos/ndk-include -o demo demo.c -lamiga -lmieee` |",
        "| freestanding build/link (m68k) | `m68k-amigaos-gcc -std=c99 -ffreestanding -fno-builtin -nostdlib -m68000 -fwrapv -O0 -g -Wall -Wextra -Werror -o demo demo.c -lg` |",
        "| freestanding release build/link (m68k) | the same line with `-O2` in place of `-O0 -g` |",
        "| freestanding host run | `cc -std=c99 -ffreestanding -fno-builtin -nostdlib -O0 -g -Wall -Wextra -Werror -o demo demo.c` |",
    ];
    assert_eq!(rows, expected, "the compile matrix must match the shipped rows");
}

/// The release help names the profiles that are stripped and the ones that
/// keep symbols.
#[test]
fn the_release_help_names_the_stripped_and_symbol_profiles() {
    let scratch = ScratchDir::new("release-help");
    let output = run_driver(scratch.path(), &["--help"]);
    assert_exit("release-help", &output, 0);
    let help = stdout(&output);
    assert!(
        help.contains("strip + section-GC on Linux/Amiga"),
        "release-help: the stripped profiles must be named, got:\n{help}"
    );
    assert!(
        help.contains("freestanding/non-Linux and debug keep symbols"),
        "release-help: the symbol-bearing profiles must be named, got:\n{help}"
    );
}
