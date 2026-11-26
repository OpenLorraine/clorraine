//! Host-family rules for the developer driver.
//!
//! Windows and Unix hosts name executables, spell paths, keep configuration,
//! and quote shell words differently. Every such decision lives here as a pure
//! function that takes the host family as a parameter, so the Windows rules
//! are exercised by the same test run that covers Linux. Call sites pass
//! [`HostOs::current`].

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

/// The host family whose naming and directory rules the driver follows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HostOs {
    /// Linux and the other Unix-like hosts.
    Unix,
    /// Microsoft Windows.
    Windows,
}

impl HostOs {
    /// The family of the running host.
    ///
    /// ```
    /// use clc::clorraine::platform::HostOs;
    /// assert_eq!(HostOs::current() == HostOs::Windows, cfg!(windows));
    /// ```
    pub fn current() -> Self {
        if cfg!(windows) {
            HostOs::Windows
        } else {
            HostOs::Unix
        }
    }
}

/// The file name of the executable called `base`.
///
/// Windows programs carry an `.exe` suffix; a name that already ends in
/// `.exe` (in any case) is kept. Unix names are unchanged.
///
/// ```
/// use clc::clorraine::platform::{executable_name, HostOs};
/// assert_eq!(executable_name(HostOs::Windows, "gcc"), "gcc.exe");
/// assert_eq!(executable_name(HostOs::Unix, "gcc"), "gcc");
/// ```
pub fn executable_name(os: HostOs, base: &str) -> String {
    // Compared as bytes: the last four bytes of a non-ASCII name need not
    // start on a character boundary.
    let bytes = base.as_bytes();
    let has_suffix = bytes.len() >= 4 && bytes[bytes.len() - 4..].eq_ignore_ascii_case(b".exe");
    if os == HostOs::Windows && !has_suffix {
        format!("{base}.exe")
    } else {
        base.to_owned()
    }
}

/// A path as a person writes it: without the Windows verbatim prefix.
///
/// Resolving a path on Windows yields `\\?\C:\…` or `\\?\UNC\server\share\…`.
/// Those spellings must not reach the config file or messages, so they become
/// `C:\…` and `\\server\share\…`. Any other text, and every Unix path, is
/// returned unchanged.
///
/// ```
/// use clc::clorraine::platform::{display_path, HostOs};
/// assert_eq!(display_path(HostOs::Windows, r"\\?\C:\rc24"), r"C:\rc24");
/// ```
pub fn display_path(os: HostOs, text: &str) -> String {
    if os == HostOs::Windows {
        if let Some(rest) = text.strip_prefix(r"\\?\UNC\") {
            return format!(r"\\{rest}");
        }
        if let Some(rest) = text.strip_prefix(r"\\?\") {
            if has_drive(rest) {
                return rest.to_owned();
            }
        }
    }
    text.to_owned()
}

/// True when `text` starts with a drive letter and a colon, as in `C:`.
fn has_drive(text: &str) -> bool {
    let bytes = text.as_bytes();
    bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':'
}

/// True when `text` is an absolute Windows path: `C:\…`, `C:/…`, or a UNC
/// `\\server\…` path.
pub(crate) fn is_windows_absolute(text: &str) -> bool {
    let bytes = text.as_bytes();
    (has_drive(text) && bytes.len() >= 3 && matches!(bytes[2], b'\\' | b'/'))
        || text.starts_with(r"\\")
}

/// True when `text` names a path rather than a bare program name.
pub(crate) fn is_path_like(os: HostOs, text: &str) -> bool {
    match os {
        HostOs::Windows => text.contains(['\\', '/']) || has_drive(text),
        HostOs::Unix => text.contains('/'),
    }
}

/// The CLI's config file, from the host's environment variables.
///
/// `var` looks one variable up, so callers pass the process environment and
/// tests pass a fixed table. Unix keeps the XDG rule: an absolute
/// `XDG_CONFIG_HOME`, else `HOME/.config`, then `clorraine/config`. Windows
/// always uses `%APPDATA%\clorraine\config` and ignores `HOME` and
/// `XDG_CONFIG_HOME`, which MSYS2 and Git Bash export with Unix spellings.
///
/// # Errors
///
/// The message to print when no absolute base directory is available.
///
/// ```
/// use clc::clorraine::platform::{config_file, HostOs};
/// let file = config_file(HostOs::Unix, |name| (name == "HOME").then(|| "/h".into()));
/// assert_eq!(file.unwrap(), std::path::PathBuf::from("/h/.config/clorraine/config"));
/// ```
pub fn config_file(os: HostOs, var: impl Fn(&str) -> Option<OsString>) -> Result<PathBuf, String> {
    match os {
        HostOs::Windows => {
            let appdata = var("APPDATA")
                .and_then(|value| value.into_string().ok())
                .filter(|value| is_windows_absolute(value))
                .ok_or_else(|| "clorraine: cannot determine a config directory; set APPDATA".to_owned())?;
            // Built as text so the separators are Windows ones on every host.
            Ok(PathBuf::from(format!(
                r"{}\clorraine\config",
                appdata.trim_end_matches(['\\', '/'])
            )))
        }
        HostOs::Unix => {
            if let Some(xdg) = var("XDG_CONFIG_HOME").map(PathBuf::from).filter(|path| path.is_absolute()) {
                return Ok(xdg.join("clorraine").join("config"));
            }
            let home = var("HOME").map(PathBuf::from).filter(|path| path.is_absolute()).ok_or_else(|| {
                "clorraine: cannot determine a config directory; set HOME or XDG_CONFIG_HOME".to_owned()
            })?;
            Ok(home.join(".config").join("clorraine").join("config"))
        }
    }
}

/// The host C compilers to try, in order.
///
/// A `CLORRAINE_CC` value is the only candidate. Otherwise Unix uses `cc`;
/// Windows tries `gcc`, `cc`, and `clang`, since a Windows GCC rarely
/// provides a `cc` name.
pub fn host_compilers(os: HostOs, override_cc: Option<&OsStr>) -> Vec<OsString> {
    if let Some(chosen) = override_cc {
        return vec![chosen.to_owned()];
    }
    let names: &[&str] = match os {
        HostOs::Unix => &["cc"],
        HostOs::Windows => &["gcc", "cc", "clang"],
    };
    names.iter().map(OsString::from).collect()
}

/// The `PATH` a toolchain program runs with, or `None` to keep the caller's.
///
/// A Windows compiler driver starts helper programs from other folders (the
/// rc24 `cc1`, `as`, and `collect2`), and those load DLLs that only the
/// driver's own folder holds. Windows looks for a program's DLLs in its own
/// folder and on `PATH`, so a program named by a path gets its folder first on
/// `PATH`. A bare name such as `gcc`, and every Unix command, keeps the
/// caller's `PATH`.
///
/// ```
/// use clc::clorraine::platform::{tool_search_path, HostOs};
/// use std::ffi::{OsStr, OsString};
/// assert_eq!(
///     tool_search_path(HostOs::Windows, OsStr::new(r"C:\tc\bin\gcc.exe"), Some(OsStr::new(r"C:\Windows"))),
///     Some(OsString::from(r"C:\tc\bin;C:\Windows"))
/// );
/// assert_eq!(tool_search_path(HostOs::Unix, OsStr::new("/tc/bin/gcc"), None), None);
/// ```
pub fn tool_search_path(os: HostOs, program: &OsStr, current: Option<&OsStr>) -> Option<OsString> {
    if !matches!(os, HostOs::Windows) {
        return None;
    }
    let text = program.to_string_lossy();
    let folder = &text[..text.rfind(['\\', '/'])?];
    if folder.is_empty() {
        return None;
    }
    let mut path = OsString::from(folder);
    if let Some(current) = current.filter(|current| !current.is_empty()) {
        path.push(";");
        path.push(current);
    }
    Some(path)
}

/// Puts a Windows toolchain program's folder first on its `PATH`.
pub fn prepare_tool_command(command: &mut std::process::Command, program: &OsStr) {
    if let Some(path) = tool_search_path(HostOs::current(), program, std::env::var_os("PATH").as_deref()) {
        command.env("PATH", path);
    }
}

/// The hint printed when no host C compiler is found.
pub fn missing_compiler_hint(os: HostOs) -> &'static str {
    match os {
        HostOs::Unix => "install a C compiler (cc)",
        HostOs::Windows => "install GCC (for example w64devkit or MSYS2 UCRT64) or set CLORRAINE_CC",
    }
}

/// The file name of a host-built program.
///
/// `host_run` names the freestanding profile's host-run artifact, which sits
/// beside the m68k one as `<stem>.host`. Windows adds `.exe` to both, since
/// the C compiler writes that name.
pub fn host_artifact_name(os: HostOs, stem: &str, host_run: bool) -> String {
    let base = if host_run { format!("{stem}.host") } else { stem.to_owned() };
    match os {
        HostOs::Unix => base,
        HostOs::Windows => format!("{base}.exe"),
    }
}

/// The signature that opens a `VS_FIXEDFILEINFO` block.
const FIXED_FILE_INFO_SIGNATURE: u32 = 0xFEEF_04BD;

/// The only `VS_FIXEDFILEINFO` structure version in use.
const FIXED_FILE_INFO_VERSION: u32 = 0x0001_0000;

/// The size of a `VS_FIXEDFILEINFO` block: thirteen 32-bit fields.
const FIXED_FILE_INFO_SIZE: usize = 52;

/// The file version recorded in a Windows executable's version resource.
///
/// FS-UAE for Windows is a GUI program that prints nothing to a pipe, so its
/// version is read from the file instead of by running it. The bytes must be
/// a PE image (an `MZ` header whose `e_lfanew` points at `PE\0\0`); the
/// result is the major, minor, and patch of the first `VS_FIXEDFILEINFO`
/// block, found at a 4-byte-aligned offset with its signature and structure
/// version. Anything else yields `None`.
pub fn pe_file_version(bytes: &[u8]) -> Option<(u64, u64, u64)> {
    if bytes.len() < 0x40 || &bytes[0..2] != b"MZ" {
        return None;
    }
    let header = read_u32(bytes, 0x3c)? as usize;
    if bytes.get(header..header.checked_add(4)?)? != b"PE\0\0" {
        return None;
    }
    let mut at = 0;
    while at + FIXED_FILE_INFO_SIZE <= bytes.len() {
        if read_u32(bytes, at) == Some(FIXED_FILE_INFO_SIGNATURE)
            && read_u32(bytes, at + 4) == Some(FIXED_FILE_INFO_VERSION)
        {
            let most = read_u32(bytes, at + 8)?;
            let least = read_u32(bytes, at + 12)?;
            return Some((u64::from(most >> 16), u64::from(most & 0xffff), u64::from(least >> 16)));
        }
        at += 4;
    }
    None
}

/// The little-endian 32-bit value at `at`, if the bytes reach that far.
fn read_u32(bytes: &[u8], at: usize) -> Option<u32> {
    let word = bytes.get(at..at.checked_add(4)?)?;
    Some(u32::from_le_bytes([word[0], word[1], word[2], word[3]]))
}

/// Quotes `word` for the host's shell when it is not a plain word.
///
/// Unix uses POSIX single quotes; Windows wraps the word in double quotes,
/// which `cmd.exe` and PowerShell both accept for a folder name with spaces
/// (PowerShell still expands `$` and a backtick inside them). Control
/// characters are made visible first, as in every driver message.
pub fn shell_word(os: HostOs, word: &str) -> String {
    let plain = !word.is_empty()
        && word
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || "-_./+@%".contains(ch));
    if plain {
        return word.to_owned();
    }
    let shown = super::display::sanitize(word);
    match os {
        HostOs::Unix => format!("'{}'", shown.replace('\'', "'\\''")),
        HostOs::Windows => format!("\"{shown}\""),
    }
}

/// `path` made absolute against the current directory, without resolving
/// links, so an installed launcher link keeps its own spelling.
///
/// A Windows path is absolute when it has a drive and a separator or is a
/// UNC path; the check is textual so it also holds on a Unix test host.
pub(crate) fn absolute_for(os: HostOs, path: &Path) -> PathBuf {
    let absolute = match os {
        HostOs::Windows => path.to_str().is_some_and(is_windows_absolute),
        HostOs::Unix => path.is_absolute(),
    };
    if absolute {
        return path.to_owned();
    }
    std::env::current_dir()
        .map(|dir| dir.join(path))
        .unwrap_or_else(|_| path.to_owned())
}
