//! Windows CLI specification tests.
//!
//! These cases pin the behaviour the Windows command line depends on and that
//! can be proven on any host: source paths inside `#line` directives are valid
//! C string literals, the freestanding runtime only uses raw Linux system
//! calls on Linux, and the driver's Windows rules are pure functions that take
//! the host family as a parameter, so a Linux test run exercises the Windows
//! branches too. The language server spells a Windows file one way for every
//! URI it receives or sends.

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

use clc::clorraine::platform::{
    config_file, display_path, executable_name, host_artifact_name, host_compilers,
    missing_compiler_hint, pe_file_version, shell_word, HostOs,
};
use clc::lsp::uri::{from_path_for, normalize_for, parse_for, DocumentUri};

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
            "clc-windows-cli-{}-{tag}-{sequence}",
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

/// Runs the built compiler with the given arguments.
fn run_clc(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_clc"))
        .args(args)
        .output()
        .expect("failed to spawn the compiler binary")
}

/// A complete program on one line, so its statements map to source line 1.
const ONE_LINE_PROGRAM: &str = "module demo.Paths; func main(): i32 { return 0; }\n";

/// Writes the one-line program under `file_name`, emits it for `profile`, and
/// returns the source path as passed to the compiler and the emitted C.
fn emit_named(scratch: &ScratchDir, file_name: &str, profile: &str) -> (String, String) {
    let source = scratch.path().join(file_name);
    std::fs::write(&source, ONE_LINE_PROGRAM).expect("write the source");
    let c_path = scratch.path().join("out.c");
    let source_text = source.to_str().expect("UTF-8 scratch path").to_owned();
    let output = run_clc(&[
        "--profile",
        profile,
        "-o",
        c_path.to_str().expect("UTF-8 scratch path"),
        &source_text,
    ]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "the compiler rejected {file_name:?}; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let emitted = std::fs::read_to_string(&c_path).expect("read the emitted C");
    (source_text, emitted)
}

/// Builds host-profile C with the strict flags and asserts the build passes.
fn assert_strict_host_build(scratch: &ScratchDir, emitted: &str) {
    let c_path = scratch.path().join("strict.c");
    std::fs::write(&c_path, emitted).expect("write the C");
    let program = scratch.path().join("prog");
    let built = Command::new("cc")
        .args(["-std=c99", "-Wall", "-Wextra", "-Werror", "-o"])
        .arg(&program)
        .arg(&c_path)
        .output()
        .expect("failed to spawn the host C compiler");
    assert!(
        built.status.success(),
        "the host C compiler rejected the emitted C; stderr:\n{}",
        String::from_utf8_lossy(&built.stderr)
    );
}

/// The `#line` directives of emitted C, in order.
fn line_directives(emitted: &str) -> Vec<&str> {
    emitted.lines().filter(|line| line.starts_with("#line ")).collect()
}

/// The source directory of a scratch file, as the compiler was given it.
fn directory_of(source: &str) -> &str {
    &source[..source.rfind('/').expect("an absolute scratch path")]
}

#[test]
fn should_escape_a_backslash_in_the_line_directive_path_when_the_file_name_contains_one() {
    let scratch = ScratchDir::new("backslash");
    let (source, emitted) = emit_named(&scratch, "back\\slash.clo", "host");
    let expected = format!("#line 1 \"{}/back\\\\slash.clo\"", directory_of(&source));
    assert!(
        line_directives(&emitted).contains(&expected.as_str()),
        "expected {expected:?} among the directives {:?}",
        line_directives(&emitted)
    );
    assert_strict_host_build(&scratch, &emitted);
}

#[test]
fn should_escape_a_double_quote_in_the_line_directive_path_when_the_file_name_contains_one() {
    let scratch = ScratchDir::new("quote");
    let (source, emitted) = emit_named(&scratch, "quo\"te.clo", "host");
    let expected = format!("#line 1 \"{}/quo\\\"te.clo\"", directory_of(&source));
    assert!(
        line_directives(&emitted).contains(&expected.as_str()),
        "expected {expected:?} among the directives {:?}",
        line_directives(&emitted)
    );
    assert_strict_host_build(&scratch, &emitted);
}

#[test]
fn should_write_a_tab_as_an_octal_escape_when_the_file_name_contains_one() {
    let scratch = ScratchDir::new("tab");
    let (source, emitted) = emit_named(&scratch, "tab\there.clo", "host");
    let expected = format!("#line 1 \"{}/tab\\011here.clo\"", directory_of(&source));
    assert!(
        line_directives(&emitted).contains(&expected.as_str()),
        "expected {expected:?} among the directives {:?}",
        line_directives(&emitted)
    );
    assert_strict_host_build(&scratch, &emitted);
}

#[test]
fn should_keep_every_line_directive_unchanged_when_the_path_has_no_special_characters() {
    // A plain path is written exactly as the compiler was given it, so the
    // emitted directives are the same bytes they were before escaping existed.
    let hello = Path::new(env!("CARGO_MANIFEST_DIR")).join("../examples/hello/hello.clo");
    let hello_text = hello.to_str().expect("UTF-8 repository path");
    assert!(
        !hello_text.contains(['\\', '"']) && hello_text.bytes().all(|b| b >= 0x20 && b != 0x7f),
        "the repository path must be plain for this case: {hello_text:?}"
    );
    let scratch = ScratchDir::new("plain");
    let c_path = scratch.path().join("hello.c");
    let output = run_clc(&[
        "--profile",
        "host",
        "-o",
        c_path.to_str().expect("UTF-8 scratch path"),
        hello_text,
    ]);
    assert_eq!(output.status.code(), Some(0), "{}", String::from_utf8_lossy(&output.stderr));
    let emitted = std::fs::read_to_string(&c_path).expect("read the emitted C");
    let directives = line_directives(&emitted);
    assert!(!directives.is_empty(), "hello emits no #line directives");
    let mut names_hello = false;
    for directive in directives {
        let rest = directive.strip_prefix("#line ").expect("a #line directive");
        let (number, quoted) = rest.split_once(' ').expect("a line number and a path");
        assert!(number.bytes().all(|b| b.is_ascii_digit()), "{directive:?}");
        let path = quoted
            .strip_prefix('"')
            .and_then(|rest| rest.strip_suffix('"'))
            .unwrap_or_else(|| panic!("an unquoted path in {directive:?}"));
        // Hello's own lines name the path as given; the imported standard
        // library keeps its plain logical path. Neither gains an escape.
        assert!(
            path == hello_text || (path.starts_with("stdlib/std.") && path.ends_with(".clo")),
            "{directive:?}"
        );
        assert!(!path.contains(['\\', '"']), "{directive:?}");
        names_hello |= path == hello_text;
    }
    assert!(names_hello, "no directive names hello.clo itself");
}

/// Emits a small freestanding program and returns its C text.
fn emit_freestanding(scratch: &ScratchDir) -> PathBuf {
    let source = scratch.path().join("main.clo");
    std::fs::write(&source, ONE_LINE_PROGRAM).expect("write the source");
    let c_path = scratch.path().join("free.c");
    let output = run_clc(&[
        "--profile",
        "freestanding",
        "-o",
        c_path.to_str().expect("UTF-8 scratch path"),
        source.to_str().expect("UTF-8 scratch path"),
    ]);
    assert_eq!(output.status.code(), Some(0), "{}", String::from_utf8_lossy(&output.stderr));
    c_path
}

/// Preprocesses freestanding C as the host compiler would see it with the
/// given extra macro flags, and returns the preprocessed text.
fn preprocess(c_path: &Path, flags: &[&str]) -> String {
    let output = Command::new("cc")
        .args(["-std=c99", "-ffreestanding", "-E", "-P"])
        .args(flags)
        .arg(c_path)
        .output()
        .expect("failed to spawn the host C compiler");
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    String::from_utf8(output.stdout).expect("UTF-8 preprocessor output")
}

/// Macros that tell C code it is being compiled for Linux.
const NOT_LINUX: [&str; 4] = ["-U__linux__", "-U__linux", "-Ulinux", "-U__gnu_linux__"];

/// The raw system-call instruction the shim emits for this host's CPU.
const SYSCALL_MARKER: &str = if cfg!(target_arch = "aarch64") { "\"svc #0\"" } else { "\"syscall\"" };

#[test]
fn should_use_raw_system_calls_only_on_linux_when_building_the_freestanding_runtime() {
    let scratch = ScratchDir::new("free-syscalls");
    let c_path = emit_freestanding(&scratch);

    let linux = preprocess(&c_path, &[]);
    assert!(linux.contains(SYSCALL_MARKER), "Linux keeps the raw system calls");

    let mut windows = NOT_LINUX.to_vec();
    windows.push("-D_WIN64");
    let other = preprocess(&c_path, &windows);
    assert!(
        !other.contains(SYSCALL_MARKER),
        "a non-Linux host must take the C-runtime fallback, not raw Linux system calls"
    );
    let plain_other = preprocess(&c_path, &NOT_LINUX);
    assert!(!plain_other.contains(SYSCALL_MARKER), "any non-Linux host takes the fallback");
}

#[test]
fn should_make_the_pointer_sized_integer_64_bit_only_under_win64_when_building_freestanding() {
    let scratch = ScratchDir::new("free-uintptr");
    let c_path = emit_freestanding(&scratch);

    let linux = preprocess(&c_path, &[]);
    assert!(linux.contains("typedef unsigned long uintptr_t;"), "Linux keeps unsigned long");
    assert!(!linux.contains("typedef unsigned long long uintptr_t;"));

    let mut windows = NOT_LINUX.to_vec();
    windows.push("-D_WIN64");
    let win64 = preprocess(&c_path, &windows);
    assert!(
        win64.contains("typedef unsigned long long uintptr_t;"),
        "64-bit Windows has a 32-bit long, so the pointer-sized integer is unsigned long long"
    );
    assert!(!win64.contains("typedef unsigned long uintptr_t;"));
}

#[test]
fn should_select_native_system_calls_only_for_linux_x86_64_and_aarch64() {
    use clc::clorraine::native_syscalls_for;
    assert!(native_syscalls_for("linux", "x86_64"));
    assert!(native_syscalls_for("linux", "aarch64"));
    assert!(!native_syscalls_for("windows", "x86_64"));
    assert!(!native_syscalls_for("macos", "aarch64"));
    assert!(!native_syscalls_for("linux", "m68k"));
}

// --- The driver's host-family rules -----------------------------------------

/// An environment lookup over a fixed list of variables.
fn vars(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<OsString> {
    let pairs: Vec<(String, String)> =
        pairs.iter().map(|(k, v)| ((*k).to_owned(), (*v).to_owned())).collect();
    move |name: &str| {
        pairs.iter().find(|(key, _)| key == name).map(|(_, value)| OsString::from(value))
    }
}

#[test]
fn should_add_the_exe_suffix_only_on_windows_and_only_when_missing() {
    assert_eq!(executable_name(HostOs::Windows, "m68k-amigaos-gcc"), "m68k-amigaos-gcc.exe");
    assert_eq!(executable_name(HostOs::Windows, "x.EXE"), "x.EXE");
    assert_eq!(executable_name(HostOs::Unix, "cc"), "cc");
}

#[test]
fn should_drop_the_verbatim_prefix_from_a_windows_drive_path_when_displaying_it() {
    assert_eq!(display_path(HostOs::Windows, r"\\?\C:\Users\a b\rc24"), r"C:\Users\a b\rc24");
}

#[test]
fn should_show_a_verbatim_unc_path_as_a_plain_share_path_and_leave_unix_text_alone() {
    assert_eq!(display_path(HostOs::Windows, r"\\?\UNC\srv\share\x"), r"\\srv\share\x");
    assert_eq!(display_path(HostOs::Unix, r"\\?\C:\x"), r"\\?\C:\x");
}

#[test]
fn should_keep_the_config_in_appdata_and_ignore_home_and_xdg_when_on_windows() {
    let lookup = vars(&[
        ("APPDATA", r"C:\Users\u\AppData\Roaming"),
        ("HOME", "/c/Users/u"),
        ("XDG_CONFIG_HOME", "/x"),
    ]);
    let file = config_file(HostOs::Windows, lookup).expect("a config file");
    assert_eq!(file.to_str(), Some(r"C:\Users\u\AppData\Roaming\clorraine\config"));
}

#[test]
fn should_ask_for_appdata_when_it_is_unset_or_relative_on_windows() {
    let unset = config_file(HostOs::Windows, vars(&[("HOME", "/h")])).unwrap_err();
    assert!(unset.contains("set APPDATA"), "{unset}");
    let relative = config_file(HostOs::Windows, vars(&[("APPDATA", r"rel\dir")])).unwrap_err();
    assert!(relative.contains("set APPDATA"), "{relative}");
}

#[test]
fn should_keep_the_xdg_then_home_config_rule_when_on_unix() {
    let xdg = config_file(HostOs::Unix, vars(&[("XDG_CONFIG_HOME", "/x"), ("HOME", "/h")]))
        .expect("a config file");
    assert_eq!(xdg, PathBuf::from("/x/clorraine/config"));
    let home = config_file(HostOs::Unix, vars(&[("HOME", "/h")])).expect("a config file");
    assert_eq!(home, PathBuf::from("/h/.config/clorraine/config"));
}

#[test]
fn should_try_gcc_cc_then_clang_on_windows_cc_on_unix_and_only_the_override_when_set() {
    let names = |list: Vec<OsString>| -> Vec<String> {
        list.into_iter().map(|name| name.to_string_lossy().into_owned()).collect()
    };
    assert_eq!(names(host_compilers(HostOs::Windows, None)), ["gcc", "cc", "clang"]);
    assert_eq!(names(host_compilers(HostOs::Unix, None)), ["cc"]);
    let chosen = OsStr::new(r"C:\gcc\bin\gcc.exe");
    assert_eq!(names(host_compilers(HostOs::Windows, Some(chosen))), [r"C:\gcc\bin\gcc.exe"]);
}

#[test]
fn should_name_a_gcc_distribution_or_the_override_when_no_compiler_is_found_on_windows() {
    assert_eq!(
        missing_compiler_hint(HostOs::Windows),
        "install GCC (for example w64devkit or MSYS2 UCRT64) or set CLORRAINE_CC"
    );
    assert_eq!(missing_compiler_hint(HostOs::Unix), "install a C compiler (cc)");
}

/// Writes a fake C compiler that records its arguments and creates the file
/// named after `-o`, and returns its path.
#[cfg(unix)]
fn fake_compiler(scratch: &ScratchDir) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let path = scratch.path().join("fake-cc");
    std::fs::write(
        &path,
        "#!/bin/sh\n\
         printf '%s\\n' \"$@\" > \"$FAKE_CC_LOG\"\n\
         out=''; prev=''\n\
         for arg in \"$@\"; do [ \"$prev\" = -o ] && out=\"$arg\"; prev=\"$arg\"; done\n\
         [ -n \"$out\" ] && : > \"$out\" && chmod +x \"$out\"\n\
         exit 0\n",
    )
    .expect("write the fake compiler");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))
        .expect("make the fake compiler executable");
    path
}

/// Runs the driver in `cwd` with extra environment variables.
fn run_driver_with(cwd: &Path, args: &[&str], env: &[(&str, &OsStr)]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_clorraine"));
    command.args(args).current_dir(cwd);
    for (name, value) in env {
        command.env(name, value);
    }
    command.output().expect("failed to spawn the clorraine driver")
}

/// The hello program the driver cases build.
const HELLO: &str = "module demo.Hello;\n\nimport { println } from std.io;\n\n\
func main(): i32 {\n    println(\"Hello\");\n    return 0;\n}\n";

#[cfg(unix)]
#[test]
fn should_build_with_the_compiler_named_by_clorraine_cc_when_it_is_set() {
    let scratch = ScratchDir::new("cc-override");
    std::fs::write(scratch.path().join("hello.clo"), HELLO).expect("write hello");
    let fake = fake_compiler(&scratch);
    let log = scratch.path().join("argv.txt");
    let output = run_driver_with(
        scratch.path(),
        &["build", "hello.clo"],
        &[("CLORRAINE_CC", fake.as_os_str()), ("FAKE_CC_LOG", log.as_os_str())],
    );
    assert_eq!(
        output.status.code(),
        Some(0),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let argv = std::fs::read_to_string(&log).expect("the fake compiler was invoked");
    let argv: Vec<&str> = argv.lines().collect();
    assert!(argv.contains(&"-std=c99"), "the host flags are passed: {argv:?}");
    assert!(argv.contains(&"-o"), "the output is named: {argv:?}");
}

#[test]
fn should_fail_with_the_override_name_when_clorraine_cc_does_not_exist() {
    let scratch = ScratchDir::new("cc-missing");
    std::fs::write(scratch.path().join("hello.clo"), HELLO).expect("write hello");
    let output = run_driver_with(
        scratch.path(),
        &["build", "hello.clo"],
        &[("CLORRAINE_CC", OsStr::new("/no/such/cc"))],
    );
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("clorraine: CLORRAINE_CC '/no/such/cc' was not found"),
        "stderr:\n{stderr}"
    );
}

#[test]
fn should_name_host_programs_with_exe_on_windows_and_without_on_unix() {
    assert_eq!(host_artifact_name(HostOs::Windows, "hello", false), "hello.exe");
    assert_eq!(host_artifact_name(HostOs::Windows, "hello", true), "hello.host.exe");
    assert_eq!(host_artifact_name(HostOs::Unix, "hello", false), "hello");
    assert_eq!(host_artifact_name(HostOs::Unix, "hello", true), "hello.host");
}

/// A minimal PE32+ image whose fixed version resource reports
/// `major.minor.patch.0`: an `MZ` header pointing at a `PE\0\0` signature for
/// x86-64, and a `VS_FIXEDFILEINFO` block later in the file.
fn pe_fixture(major: u16, minor: u16, patch: u16) -> Vec<u8> {
    let mut bytes = vec![0u8; 0x400];
    bytes[0..2].copy_from_slice(b"MZ");
    bytes[0x3c..0x40].copy_from_slice(&0x80u32.to_le_bytes());
    bytes[0x80..0x84].copy_from_slice(b"PE\0\0");
    bytes[0x84..0x86].copy_from_slice(&0x8664u16.to_le_bytes());
    let ms = (u32::from(major) << 16) | u32::from(minor);
    let ls = u32::from(patch) << 16;
    let info = [0xFEEF_04BDu32, 0x0001_0000, ms, ls, ms, ls];
    for (index, word) in info.iter().enumerate() {
        let at = 0x200 + index * 4;
        bytes[at..at + 4].copy_from_slice(&word.to_le_bytes());
    }
    bytes
}

#[test]
fn should_read_the_file_version_from_a_pe_version_resource() {
    assert_eq!(pe_file_version(&pe_fixture(3, 2, 35)), Some((3, 2, 35)));
}

#[test]
fn should_find_no_version_in_an_elf_file_a_pe_without_the_resource_or_a_truncated_pe() {
    let mut elf = vec![0u8; 0x400];
    elf[0..4].copy_from_slice(b"\x7fELF");
    assert_eq!(pe_file_version(&elf), None);

    let mut unsigned = pe_fixture(3, 2, 35);
    unsigned[0x200..0x204].copy_from_slice(&[0, 0, 0, 0]);
    assert_eq!(pe_file_version(&unsigned), None);

    let fixture = pe_fixture(3, 2, 35);
    assert_eq!(pe_file_version(&fixture[..100]), None);
}

#[test]
fn should_read_the_fs_uae_version_from_the_file_without_running_it_on_windows() {
    let scratch = ScratchDir::new("pe-version");
    let path = scratch.path().join("fs-uae.exe");
    std::fs::write(&path, pe_fixture(3, 2, 35)).expect("write the fixture");
    assert_eq!(clc::clorraine::fs_uae_version_for(HostOs::Windows, &path), Ok((3, 2, 35)));
}

#[test]
fn should_reject_an_old_or_oversized_fs_uae_executable_on_windows() {
    let scratch = ScratchDir::new("pe-reject");
    let old = scratch.path().join("old.exe");
    std::fs::write(&old, pe_fixture(3, 1, 66)).expect("write the fixture");
    let error = clc::clorraine::fs_uae_version_for(HostOs::Windows, &old).unwrap_err();
    assert!(error.contains("older than the required 3.2.0"), "{error}");

    let huge = scratch.path().join("huge.exe");
    let file = std::fs::File::create(&huge).expect("create the large file");
    file.set_len(256 * 1024 * 1024 + 1).expect("grow the large file");
    let error = clc::clorraine::fs_uae_version_for(HostOs::Windows, &huge).unwrap_err();
    let expected = format!("cannot read the FS-UAE version from '{}'", huge.display());
    assert!(error.contains(&expected), "{error}");
}

/// Creates a file and marks it executable where the host has mode bits.
fn executable_file(path: &Path) {
    std::fs::create_dir_all(path.parent().expect("a parent")).expect("create the parent");
    std::fs::write(path, "").expect("write the file");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))
            .expect("make it executable");
    }
}

#[test]
fn should_select_the_exe_compiler_in_a_toolchain_root_on_windows() {
    use clc::clorraine::{validate_for, Validation};
    let scratch = ScratchDir::new("validate-windows");
    let root = scratch.path().join("rc24");
    executable_file(&root.join("bin").join("m68k-amigaos-gcc.exe"));
    std::fs::create_dir_all(root.join("m68k-amigaos").join("libnix")).expect("libnix");
    std::fs::create_dir_all(root.join("m68k-amigaos").join("ndk-include")).expect("ndk-include");
    let toolchain = validate_for(HostOs::Windows, &root, Validation::Selection)
        .unwrap_or_else(|error| panic!("{}", error.message));
    let compiler = toolchain.compiler.to_string_lossy().into_owned();
    assert!(compiler.ends_with("m68k-amigaos-gcc.exe"), "{compiler}");

    let bare = scratch.path().join("bare");
    executable_file(&bare.join("bin").join("m68k-amigaos-gcc"));
    std::fs::create_dir_all(bare.join("m68k-amigaos").join("libnix")).expect("libnix");
    std::fs::create_dir_all(bare.join("m68k-amigaos").join("ndk-include")).expect("ndk-include");
    let error = validate_for(HostOs::Windows, &bare, Validation::Selection)
        .err()
        .expect("a root without the .exe compiler is rejected");
    assert!(error.message.contains("has no bin/m68k-amigaos-gcc.exe"), "{}", error.message);
}

/// The rc24 compiler cannot run from a folder whose path contains a space on
/// Windows: its own library path is split at the space. Setup refuses such a
/// root with that reason; the same root is still accepted on Linux.
#[test]
fn should_refuse_a_toolchain_root_with_a_space_on_windows_only() {
    use clc::clorraine::{validate_for, Validation};
    let scratch = ScratchDir::new("validate-space");
    let root = scratch.path().join("amiga gcc");
    executable_file(&root.join("bin").join("m68k-amigaos-gcc.exe"));
    executable_file(&root.join("bin").join("m68k-amigaos-gcc"));
    std::fs::create_dir_all(root.join("m68k-amigaos").join("libnix")).expect("libnix");
    std::fs::create_dir_all(root.join("m68k-amigaos").join("ndk-include")).expect("ndk-include");
    let error = validate_for(HostOs::Windows, &root, Validation::Selection)
        .err()
        .expect("a Windows root with a space is rejected");
    assert_eq!(error.code, 2, "{}", error.message);
    assert!(
        error.message.contains("contains a space"),
        "the reason names the space: {}",
        error.message
    );
    validate_for(HostOs::Unix, &root, Validation::Selection)
        .unwrap_or_else(|error| panic!("Linux keeps accepting the root: {}", error.message));
}

/// The rc24 compiler's helper programs (`cc1`, `as`, `collect2`) live outside
/// its `bin` folder but load a DLL that only `bin` holds, so on Windows every
/// toolchain program starts with its own folder first on `PATH`. A bare
/// program name and every Linux command keep the caller's `PATH` unchanged.
#[test]
fn should_put_a_windows_tool_folder_first_on_path_and_leave_linux_alone() {
    use clc::clorraine::platform::tool_search_path;
    let gcc = r"C:\ProgramData\Clorraine\amiga-gcc-16.2-rc24\bin\m68k-amigaos-gcc.exe";
    assert_eq!(
        tool_search_path(HostOs::Windows, OsStr::new(gcc), Some(OsStr::new(r"C:\Windows;C:\Tools"))),
        Some(OsString::from(r"C:\ProgramData\Clorraine\amiga-gcc-16.2-rc24\bin;C:\Windows;C:\Tools"))
    );
    assert_eq!(
        tool_search_path(HostOs::Windows, OsStr::new("C:/tc/bin/m68k-amigaos-gcc.exe"), None),
        Some(OsString::from("C:/tc/bin"))
    );
    assert_eq!(tool_search_path(HostOs::Windows, OsStr::new("gcc"), Some(OsStr::new(r"C:\Windows"))), None);
    assert_eq!(
        tool_search_path(HostOs::Unix, OsStr::new("/opt/rc24/bin/m68k-amigaos-gcc"), Some(OsStr::new("/usr/bin"))),
        None
    );
}

/// FS-UAE decrypts a Cloanto ROM (header `AMIROMTYPE1`) with the `rom.key`
/// beside it; without the key the Amiga never boots and the run only times
/// out. On Windows that ROM is refused up front; a plain ROM, a ROM with its
/// key, and every Linux ROM are accepted as before.
#[test]
fn should_refuse_an_encrypted_rom_without_its_key_on_windows_only() {
    use clc::clorraine::kickstart_key_missing_for;
    let scratch = ScratchDir::new("rom-key");
    let rom = scratch.path().join("amiga-os-310-a1200.rom");
    let header = b"AMIROMTYPE1";
    std::fs::write(&rom, header).expect("write the ROM header");
    assert!(kickstart_key_missing_for(HostOs::Windows, &rom, header));
    assert!(!kickstart_key_missing_for(HostOs::Unix, &rom, header));
    assert!(!kickstart_key_missing_for(HostOs::Windows, &rom, &[0x11, 0x14, 0x4E, 0xF9]));
    std::fs::write(scratch.path().join("rom.key"), b"key").expect("write the key");
    assert!(!kickstart_key_missing_for(HostOs::Windows, &rom, header));
}

#[test]
fn should_quote_words_for_the_host_shell() {
    assert_eq!(shell_word(HostOs::Windows, "my app"), "\"my app\"");
    assert_eq!(shell_word(HostOs::Windows, "my-app"), "my-app");
    assert_eq!(shell_word(HostOs::Unix, "it's"), "'it'\\''s'");
}

#[test]
fn should_keep_an_invoked_launcher_path_or_find_the_exe_on_path_on_windows() {
    use clc::clorraine::launcher_for;
    let invoked = r"C:\P\current\bin\clorraine.exe";
    assert_eq!(launcher_for(HostOs::Windows, OsStr::new(invoked), OsStr::new("")), invoked);

    let scratch = ScratchDir::new("launcher");
    let dir = scratch.path().join("bin");
    executable_file(&dir.join("clorraine.exe"));
    let found = launcher_for(HostOs::Windows, OsStr::new("clorraine"), dir.as_os_str());
    assert_eq!(found, format!("{}/clorraine.exe", dir.display()));
}

// --- Language server paths -------------------------------------------------

/// The file path a Windows document URI parses to.
fn windows_file(uri: &str) -> String {
    match parse_for(HostOs::Windows, uri) {
        Ok(DocumentUri::File(path)) => path.to_string_lossy().into_owned(),
        other => panic!("{uri} did not parse to a file: {other:?}"),
    }
}

#[test]
fn should_parse_a_vscode_windows_uri_to_a_drive_path_with_backslashes() {
    assert_eq!(windows_file("file:///c%3A/Users/a%20b/x.clo"), r"C:\Users\a b\x.clo");
}

#[test]
fn should_parse_an_unescaped_drive_uri_to_the_same_windows_path() {
    assert_eq!(windows_file("file:///C:/x/y.clo"), r"C:\x\y.clo");
}

#[test]
fn should_reject_a_network_share_uri_on_windows() {
    let error = parse_for(HostOs::Windows, "file://server/share/x.clo").unwrap_err();
    assert!(error.contains("network-share paths are not supported"), "{error}");
}

#[test]
fn should_format_a_windows_path_as_the_uri_vscode_sends() {
    assert_eq!(from_path_for(HostOs::Windows, r"C:\a b\x.clo"), "file:///c%3A/a%20b/x.clo");
}

#[test]
fn should_spell_one_windows_file_one_way_and_leave_unix_paths_alone() {
    assert_eq!(normalize_for(HostOs::Windows, r"\\?\c:\x\y.clo"), r"C:\x\y.clo");
    assert_eq!(normalize_for(HostOs::Unix, "/x/y.clo"), "/x/y.clo");
}
