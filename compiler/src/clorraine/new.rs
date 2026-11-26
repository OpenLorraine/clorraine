//! Starter projects for `clorraine new`.
//!
//! A starter is a folder with one hackable Amiga program, a README, and a VS
//! Code setup: the Amiga profile, a recommendation for the Clorraine
//! extension, and a default build task that runs the program in FS-UAE. The
//! templates are compiled into the driver, so a project can be created without
//! a network or a source checkout.
//!
//! The command never overwrites anything: an existing folder must be empty.

use std::ffi::{OsStr, OsString};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

use super::cli::Options;
use super::display;
use super::environment::{self, EnvironmentError};
use super::platform::{self, HostOs};

/// The folder used when the command names none.
const DEFAULT_FOLDER: &str = "amiga-starter";

/// The starter program.
const MAIN: &str = include_str!("../../templates/starter/main.clo");
/// The project README.
const README: &str = include_str!("../../templates/starter/README.md");
/// Workspace settings: the Amiga profile.
const SETTINGS: &str = include_str!("../../templates/starter/settings.json");
/// The extension recommendation.
const EXTENSIONS: &str = include_str!("../../templates/starter/extensions.json");
/// Build and run tasks; `@CLORRAINE@` becomes the launcher path.
const TASKS: &str = include_str!("../../templates/starter/tasks.json");

/// Creates the starter project named by `options` (or the default folder).
///
/// `search_path` is the caller's `PATH`, used to find the launcher the tasks
/// should run when the driver was started by a bare name.
pub fn run(options: &Options, search_path: &OsStr) -> Result<(), EnvironmentError> {
    let name = options.files.first().map(String::as_str).unwrap_or(DEFAULT_FOLDER);
    let folder = Path::new(name);
    prepare_folder(folder)?;
    let tasks = TASKS.replace("@CLORRAINE@", &json_string(&launcher(search_path)));
    for (relative, text) in [
        ("main.clo", MAIN),
        ("README.md", README),
        (".gitignore", "build/\n"),
        (".vscode/settings.json", SETTINGS),
        (".vscode/extensions.json", EXTENSIONS),
        (".vscode/tasks.json", tasks.as_str()),
    ] {
        write_new(&folder.join(relative), text)?;
    }
    print!("{}", summary(name.trim_end_matches('/')));
    Ok(())
}

/// Creates `folder`, or accepts it when it already exists and is empty.
fn prepare_folder(folder: &Path) -> Result<(), EnvironmentError> {
    match fs::symlink_metadata(folder) {
        Ok(metadata) if metadata.is_dir() => {
            let mut entries = fs::read_dir(folder).map_err(|cause| failure(folder, &cause))?;
            if entries.next().is_some() {
                return Err(usage(&format!(
                    "'{}' already exists and is not empty; choose another name",
                    shown(folder)
                )));
            }
        }
        Ok(_) => {
            return Err(usage(&format!(
                "'{}' already exists and is not a folder; choose another name",
                shown(folder)
            )));
        }
        Err(cause) if cause.kind() == std::io::ErrorKind::NotFound => {
            fs::create_dir_all(folder).map_err(|cause| failure(folder, &cause))?;
        }
        Err(cause) => return Err(failure(folder, &cause)),
    }
    fs::create_dir(folder.join(".vscode")).map_err(|cause| failure(folder, &cause))
}

/// Writes one new file; an existing file is never replaced.
fn write_new(path: &Path, text: &str) -> Result<(), EnvironmentError> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|cause| failure(path, &cause))?;
    file.write_all(text.as_bytes()).map_err(|cause| failure(path, &cause))
}

/// The `clorraine` launcher the editor tasks should run.
fn launcher(search_path: &OsStr) -> String {
    let invoked = std::env::args_os().next().unwrap_or_else(|| OsString::from("clorraine"));
    launcher_for(HostOs::current(), &invoked, search_path)
}

/// The launcher for a driver started as `invoked`, with `search_path` as
/// `PATH`, on the given host family.
///
/// A launcher started by path is used as written, made absolute but not
/// resolved, so an installed link (`bin/clorraine`, or the Windows
/// `current\bin\clorraine.exe` junction path) keeps following upgrades. A
/// bare name is looked up on `PATH` as the host's executable name; failing
/// both, the tasks use the bare name and rely on the editor's own `PATH`.
///
/// ```
/// use clc::clorraine::{launcher_for, platform::HostOs};
/// use std::ffi::OsStr;
/// let path = r"C:\P\current\bin\clorraine.exe";
/// assert_eq!(launcher_for(HostOs::Windows, OsStr::new(path), OsStr::new("")), path);
/// ```
pub fn launcher_for(os: HostOs, invoked: &OsStr, search_path: &OsStr) -> String {
    let invoked_text = invoked.to_string_lossy();
    let found = if platform::is_path_like(os, &invoked_text) {
        Some(platform::absolute_for(os, Path::new(invoked)))
    } else {
        let name = platform::executable_name(os, &invoked_text);
        std::env::split_paths(search_path)
            .map(|dir| platform::absolute_for(os, &dir.join(&name)))
            .find(|candidate| environment::is_executable_file(candidate))
    };
    found
        .and_then(|path| path.to_str().map(str::to_owned))
        .unwrap_or_else(|| "clorraine".to_owned())
}

/// Escapes `text` for use inside a JSON string literal.
fn json_string(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            ch if (ch as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", ch as u32)),
            ch => out.push(ch),
        }
    }
    out
}

/// The success message: what was created and how to run it.
fn summary(name: &str) -> String {
    let folder = platform::shell_word(HostOs::current(), name);
    format!(
        "Created your Amiga project in {shown}/\n\
         \n\
         \x20 main.clo    your program: change MESSAGE and the colours at the top\n\
         \x20 .vscode/    Run in FS-UAE as the default build task (Ctrl+Shift+B)\n\
         \x20 README.md   how to run and change it\n\
         \n\
         Next:\n\
         \x20 cd {folder}\n\
         \x20 code .      then open main.clo and press Run in FS-UAE\n\
         \n\
         Or from a terminal in that folder:\n\
         \x20 clorraine run -p hosted-amiga --emulator main.clo\n",
        shown = display::sanitize(name),
    )
}

/// A path for a message, made terminal-safe.
fn shown(path: &Path) -> String {
    display::sanitize(&path.display().to_string()).into_owned()
}

/// A usage-class error (exit 2).
fn usage(detail: &str) -> EnvironmentError {
    EnvironmentError { code: 2, message: format!("clorraine: new: {detail}\n") }
}

/// A filesystem failure (exit 1).
fn failure(path: &Path, cause: &std::io::Error) -> EnvironmentError {
    EnvironmentError {
        code: 1,
        message: format!("clorraine: new: cannot write '{}': {cause}\n", shown(path)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_strings_escape_quotes_backslashes_and_controls() {
        assert_eq!(json_string(r#"/a "b"\c"#), r#"/a \"b\"\\c"#);
        assert_eq!(json_string("a\tb"), "a\\u0009b");
    }

    #[test]
    fn shell_words_are_quoted_only_when_needed() {
        let unix = |word| platform::shell_word(HostOs::Unix, word);
        assert_eq!(unix("my-app"), "my-app");
        assert_eq!(unix("my app"), "'my app'");
        assert_eq!(unix("it's"), "'it'\\''s'");
    }

    #[test]
    fn the_templates_are_complete() {
        assert!(MAIN.contains("const MESSAGE: str"));
        assert!(TASKS.contains("@CLORRAINE@"));
        assert!(EXTENSIONS.contains("openlorraine.clorraine"));
    }
}
