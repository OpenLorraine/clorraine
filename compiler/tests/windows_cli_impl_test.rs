//! Windows CLI implementation tests.
//!
//! Edge cases behind the Windows specification: the escaping of every byte
//! class a source path can carry into a `#line` directive, the boundaries of
//! the native-syscall predicate, and a MinGW build of freestanding C that
//! uses `offsetof`, the one shim macro that converts a pointer to an integer.

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
            "clc-windows-cli-impl-{}-{tag}-{sequence}",
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

/// Emits the one-line program saved as `file_name` and returns the quoted
/// path of its first `#line 1` directive, without the quotes, after checking
/// that the C builds with the strict host flags.
fn line_one_path(tag: &str, file_name: &str) -> (String, String) {
    let scratch = ScratchDir::new(tag);
    let source = scratch.path().join(file_name);
    std::fs::write(&source, ONE_LINE_PROGRAM).expect("write the source");
    let c_path = scratch.path().join("out.c");
    let source_text = source.to_str().expect("UTF-8 scratch path").to_owned();
    let output = run_clc(&["--profile", "host", "-o", c_path.to_str().expect("UTF-8"), &source_text]);
    assert_eq!(output.status.code(), Some(0), "{}", String::from_utf8_lossy(&output.stderr));
    let emitted = std::fs::read_to_string(&c_path).expect("read the emitted C");
    let directive = emitted
        .lines()
        .find(|line| line.starts_with("#line 1 \""))
        .unwrap_or_else(|| panic!("{tag}: no #line 1 directive"))
        .to_owned();
    let built = Command::new("cc")
        .args(["-std=c99", "-Wall", "-Wextra", "-Werror", "-o"])
        .arg(scratch.path().join("prog"))
        .arg(&c_path)
        .output()
        .expect("failed to spawn the host C compiler");
    assert!(built.status.success(), "{tag}: {}", String::from_utf8_lossy(&built.stderr));
    let dir = source_text[..source_text.rfind('/').expect("absolute")].to_owned();
    let body = directive["#line 1 \"".len()..directive.len() - 1].to_owned();
    (dir, body)
}

#[test]
fn should_octal_escape_every_control_byte_and_delete_in_a_line_directive_path() {
    let name: String = (1u8..0x20).chain([0x7f]).map(char::from).collect::<String>() + ".clo";
    let (dir, body) = line_one_path("controls", &name);
    let expected: String = (1u32..0x20).chain([0x7f]).map(|b| format!("\\{b:03o}")).collect();
    assert_eq!(body, format!("{dir}/{expected}.clo"));
}

#[test]
fn should_keep_a_digit_after_an_octal_escape_as_its_own_character() {
    // A C octal escape takes at most three digits, so `\0017` is byte 1 and '7'.
    let (dir, body) = line_one_path("octal-digit", "a\u{1}7.clo");
    assert_eq!(body, format!("{dir}/a\\0017.clo"));
}

#[test]
fn should_keep_spaces_and_utf8_letters_unescaped_in_a_line_directive_path() {
    let (dir, body) = line_one_path("utf8", "caf\u{e9} au lait.clo");
    assert_eq!(body, format!("{dir}/caf\u{e9} au lait.clo"));
}

#[test]
fn should_escape_backslashes_and_quotes_together_in_a_line_directive_path() {
    let (dir, body) = line_one_path("mixed", "a\\\"b\\\\.clo");
    assert_eq!(body, format!("{dir}/a\\\\\\\"b\\\\\\\\.clo"));
}

#[test]
fn should_break_every_trigraph_in_a_line_directive_path() {
    // The C compilers run with -std=c99, which turns trigraphs on; the strict
    // build inside the helper fails on any trigraph that survives.
    let (dir, body) = line_one_path("trigraph", "tri??=x.clo");
    assert_eq!(body, format!("{dir}/tri?\\?=x.clo"));
    let (dir, body) = line_one_path("trigraph-run", "a???=b??-c.clo");
    assert_eq!(body, format!("{dir}/a?\\?\\?=b?\\?-c.clo"));
}

#[test]
fn should_keep_a_single_question_mark_unescaped_in_a_line_directive_path() {
    let (dir, body) = line_one_path("question", "why?.clo");
    assert_eq!(body, format!("{dir}/why?.clo"));
}

#[test]
fn should_reject_native_system_calls_for_other_linux_architectures_and_other_systems() {
    use clc::clorraine::native_syscalls_for;
    for (os, arch) in [
        ("linux", "x86"),
        ("linux", "arm"),
        ("linux", "riscv64"),
        ("android", "aarch64"),
        ("freebsd", "x86_64"),
        ("macos", "x86_64"),
        ("", "x86_64"),
        ("Linux", "x86_64"),
    ] {
        assert!(!native_syscalls_for(os, arch), "{os}/{arch}");
    }
}

#[test]
fn should_build_freestanding_offsetof_without_narrowing_a_pointer_when_compiled_with_mingw() {
    if Command::new("x86_64-w64-mingw32-gcc").arg("--version").output().is_err() {
        println!("skipped: x86_64-w64-mingw32-gcc is not installed");
        return;
    }
    let scratch = ScratchDir::new("mingw-offsetof");
    let source = scratch.path().join("off.clo");
    std::fs::write(
        &source,
        "module demo.Off;\n\nstruct Pair { a: i32; b: u32; }\n\n\
         const OFF: usize = offsetof(Pair, b);\n\n\
         func main(): i32 {\n    if (OFF == 4) {\n        return 0;\n    }\n    return 1;\n}\n",
    )
    .expect("write the source");
    let c_path = scratch.path().join("off.c");
    let output = run_clc(&[
        "--profile",
        "freestanding",
        "-o",
        c_path.to_str().expect("UTF-8"),
        source.to_str().expect("UTF-8"),
    ]);
    assert_eq!(output.status.code(), Some(0), "{}", String::from_utf8_lossy(&output.stderr));
    // The driver's flags for a host without the raw syscall layer.
    let built = Command::new("x86_64-w64-mingw32-gcc")
        .args(["-std=c99", "-ffreestanding", "-fno-builtin", "-O0", "-g"])
        .args(["-Wall", "-Wextra", "-Werror", "-o"])
        .arg(scratch.path().join("off.exe"))
        .arg(&c_path)
        .output()
        .expect("failed to spawn MinGW");
    assert!(built.status.success(), "{}", String::from_utf8_lossy(&built.stderr));
}

// --- The driver's host-family rules -----------------------------------------

use std::ffi::{OsStr, OsString};

use clc::clorraine::platform::{
    config_file, display_path, executable_name, pe_file_version, shell_word, HostOs,
};

/// A minimal PE32+ image with a fixed version block at `info_at`.
fn pe_with_info(info_at: usize, structure_version: u32) -> Vec<u8> {
    let mut bytes = vec![0u8; 0x400];
    bytes[0..2].copy_from_slice(b"MZ");
    bytes[0x3c..0x40].copy_from_slice(&0x80u32.to_le_bytes());
    bytes[0x80..0x84].copy_from_slice(b"PE\0\0");
    let info = [0xFEEF_04BDu32, structure_version, (3 << 16) | 2, 35 << 16];
    for (index, word) in info.iter().enumerate() {
        let at = info_at + index * 4;
        bytes[at..at + 4].copy_from_slice(&word.to_le_bytes());
    }
    bytes
}

#[test]
fn should_find_no_version_when_the_pe_header_offset_points_past_the_end_or_overflows() {
    for offset in [0x3fe_u32, 0x400, u32::MAX] {
        let mut bytes = pe_with_info(0x200, 0x0001_0000);
        bytes[0x3c..0x40].copy_from_slice(&offset.to_le_bytes());
        assert_eq!(pe_file_version(&bytes), None, "offset {offset:#x}");
    }
    assert_eq!(pe_file_version(b"MZ"), None);
    assert_eq!(pe_file_version(&[]), None);
}

#[test]
fn should_ignore_a_version_block_that_is_unaligned_has_another_structure_version_or_is_cut_off() {
    assert_eq!(pe_file_version(&pe_with_info(0x202, 0x0001_0000)), None);
    assert_eq!(pe_file_version(&pe_with_info(0x200, 0x0002_0000)), None);
    // The block needs all 52 bytes; one that starts 16 bytes before the end
    // of the file is not read.
    let mut short = pe_with_info(0x3f0, 0x0001_0000);
    short.truncate(0x400);
    assert_eq!(pe_file_version(&short), None);
    assert_eq!(pe_file_version(&pe_with_info(0x200, 0x0001_0000)), Some((3, 2, 35)));
}

#[test]
fn should_keep_windows_paths_without_a_drive_verbatim_prefix_and_plain_paths_unchanged() {
    let volume = r"\\?\Volume{0b1c}\x";
    assert_eq!(display_path(HostOs::Windows, volume), volume);
    assert_eq!(display_path(HostOs::Windows, r"\\?\d:\x"), r"d:\x");
    assert_eq!(display_path(HostOs::Windows, r"C:\x"), r"C:\x");
    assert_eq!(display_path(HostOs::Windows, r"\\srv\share"), r"\\srv\share");
}

#[test]
fn should_name_non_ascii_executables_without_splitting_a_character() {
    assert_eq!(executable_name(HostOs::Windows, "\u{e9}x\u{e9}"), "\u{e9}x\u{e9}.exe");
    assert_eq!(executable_name(HostOs::Windows, "\u{20ac}\u{20ac}"), "\u{20ac}\u{20ac}.exe");
    assert_eq!(executable_name(HostOs::Unix, "\u{20ac}\u{20ac}"), "\u{20ac}\u{20ac}");
}

#[test]
fn should_read_the_version_of_an_fs_uae_executable_larger_than_64_mib_on_windows() {
    // The FS-UAE 3.2.35 executable is about 66 MiB; the version block sits
    // near the start, and the rest of the file is padding here.
    let scratch = ScratchDir::new("pe-large");
    let path = scratch.path().join("fs-uae.exe");
    std::fs::write(&path, pe_with_info(0x200, 0x0001_0000)).expect("write the fixture");
    let file = std::fs::OpenOptions::new().write(true).open(&path).expect("open the fixture");
    file.set_len(70 * 1024 * 1024).expect("grow the fixture");
    assert_eq!(clc::clorraine::fs_uae_version_for(HostOs::Windows, &path), Ok((3, 2, 35)));
}

#[test]
fn should_treat_any_case_of_the_exe_suffix_as_present() {
    assert_eq!(executable_name(HostOs::Windows, "fs-uae.Exe"), "fs-uae.Exe");
    assert_eq!(executable_name(HostOs::Windows, "exe"), "exe.exe");
    assert_eq!(executable_name(HostOs::Windows, ".exe"), ".exe");
}

#[test]
fn should_accept_a_unc_or_slash_terminated_appdata_and_reject_a_drive_without_a_separator() {
    let config = |value: &str| {
        let value = value.to_owned();
        config_file(HostOs::Windows, move |name| (name == "APPDATA").then(|| OsString::from(&value)))
    };
    assert_eq!(
        config(r"C:\Users\u\AppData\Roaming\").unwrap().to_str(),
        Some(r"C:\Users\u\AppData\Roaming\clorraine\config")
    );
    assert_eq!(
        config(r"\\srv\home\u").unwrap().to_str(),
        Some(r"\\srv\home\u\clorraine\config")
    );
    assert!(config("C:relative").is_err());
    assert!(config("").is_err());
}

#[test]
fn should_quote_an_empty_word_and_make_control_characters_visible() {
    assert_eq!(shell_word(HostOs::Unix, ""), "''");
    assert_eq!(shell_word(HostOs::Windows, ""), "\"\"");
    let shown = shell_word(HostOs::Windows, "a\u{1b}b");
    assert!(!shown.contains('\u{1b}'), "{shown:?}");
}

// --- Language server paths -------------------------------------------------

use clc::lsp::uri::{from_path_for, normalize_for, parse_for, DocumentUri};

/// The Windows file path a URI parses to, or the parse error.
fn windows_parse(uri: &str) -> Result<String, String> {
    match parse_for(HostOs::Windows, uri)? {
        DocumentUri::File(path) => Ok(path.to_string_lossy().into_owned()),
        DocumentUri::Untitled(other) => Err(format!("untitled: {other}")),
    }
}

#[test]
fn should_accept_localhost_and_lowercase_drives_in_windows_uris() {
    assert_eq!(windows_parse("file://localhost/d:/src/a.clo"), Ok(r"D:\src\a.clo".to_owned()));
    assert_eq!(windows_parse("file:///z%3a/x.clo"), Ok(r"Z:\x.clo".to_owned()));
}

#[test]
fn should_reject_a_windows_uri_without_a_drive_or_with_a_bad_escape() {
    assert!(windows_parse("file:///home/u/x.clo").unwrap_err().contains("no drive letter"));
    assert!(windows_parse("file:///c%3/x.clo").is_err());
    assert!(windows_parse("https://example.com/x.clo").is_err());
}

#[test]
fn should_keep_untitled_buffers_opaque_on_windows() {
    assert_eq!(
        parse_for(HostOs::Windows, "untitled:Untitled-1"),
        Ok(DocumentUri::Untitled("untitled:Untitled-1".to_owned()))
    );
}

#[test]
fn should_round_trip_a_windows_path_with_spaces_and_unicode() {
    let path = "C:\\Users\\a b\\caf\u{e9}\\x.clo";
    let uri = from_path_for(HostOs::Windows, path);
    assert!(uri.starts_with("file:///c%3A/Users/a%20b/caf%C3%A9/"), "{uri}");
    assert_eq!(windows_parse(&uri), Ok(path.to_owned()));
}

#[test]
fn should_format_a_verbatim_or_lowercase_windows_path_like_the_editor() {
    assert_eq!(from_path_for(HostOs::Windows, r"\\?\c:\x.clo"), "file:///c%3A/x.clo");
    assert_eq!(normalize_for(HostOs::Windows, r"d:\x"), r"D:\x");
    assert_eq!(normalize_for(HostOs::Windows, r"\\?\UNC\srv\s\x"), r"\\srv\s\x");
}

#[test]
fn should_keep_unix_uri_formatting_unchanged() {
    assert_eq!(from_path_for(HostOs::Unix, "/tmp/a b/x.clo"), "file:///tmp/a%20b/x.clo");
    assert_eq!(
        parse_for(HostOs::Unix, "file:///tmp/a%20b/x.clo"),
        Ok(DocumentUri::File("/tmp/a b/x.clo".into()))
    );
}

#[test]
fn should_keep_the_drive_root_skip_a_semicolon_folder_and_drop_the_verbatim_prefix() {
    use clc::clorraine::platform::tool_search_path;
    assert_eq!(
        tool_search_path(HostOs::Windows, OsStr::new(r"C:\gcc.exe"), None),
        Some(OsString::from(r"C:\"))
    );
    assert_eq!(tool_search_path(HostOs::Windows, OsStr::new(r"C:\tc;x\bin\gcc.exe"), None), None);
    assert_eq!(
        tool_search_path(HostOs::Windows, OsStr::new(r"\\?\C:\tc\bin\gcc.exe"), Some(OsStr::new(r"C:\Windows"))),
        Some(OsString::from(r"C:\tc\bin;C:\Windows"))
    );
    assert_eq!(
        tool_search_path(HostOs::Windows, OsStr::new(r"\\server\share\tc\bin\gcc.exe"), None),
        Some(OsString::from(r"\\server\share\tc\bin"))
    );
}

#[test]
fn should_give_a_windows_toolchain_command_its_folder_on_path_and_a_unix_one_none() {
    use clc::clorraine::platform::tool_command_for;
    let windows = tool_command_for(HostOs::Windows, Path::new(r"C:\tc\bin\gcc.exe"), Some(OsStr::new(r"C:\Windows")));
    let path: Vec<_> = windows
        .get_envs()
        .filter(|(name, _)| *name == OsStr::new("PATH"))
        .map(|(_, value)| value.map(OsStr::to_owned))
        .collect();
    assert_eq!(path, vec![Some(OsString::from(r"C:\tc\bin;C:\Windows"))]);
    let unix = tool_command_for(HostOs::Unix, Path::new("/opt/tc/bin/gcc"), Some(OsStr::new("/usr/bin")));
    assert_eq!(unix.get_envs().count(), 0, "a Unix command keeps the caller's environment");
}

#[test]
fn should_refuse_a_windows_toolchain_root_with_a_semicolon() {
    use clc::clorraine::{validate_for, Validation};
    let scratch = std::env::temp_dir().join(format!("clc-windows-impl-semicolon-{}", std::process::id()));
    let root = scratch.join("tc;x");
    std::fs::create_dir_all(root.join("bin")).expect("bin");
    std::fs::write(root.join("bin").join("m68k-amigaos-gcc.exe"), "").expect("compiler");
    let error = validate_for(HostOs::Windows, &root, Validation::Selection)
        .err()
        .expect("a root with ';' is rejected");
    let _ = std::fs::remove_dir_all(&scratch);
    assert_eq!(error.code, 2);
    assert!(error.message.contains("contains a ';'"), "{}", error.message);
}
