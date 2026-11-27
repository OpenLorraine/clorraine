//! Starter projects for `clorraine new`.
//!
//! A starter is a folder with one hackable Amiga program, a README, and a VS
//! Code setup: the Amiga profile, a recommendation for the Clorraine
//! extension, and a default build task that runs the program in FS-UAE. The
//! templates are compiled into the driver, so a project can be created without
//! a network or a source checkout.
//!
//! The command never overwrites anything: an existing folder must be empty.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

use super::cli::Options;
use super::display;
use super::environment::EnvironmentError;
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
/// Build and run tasks. They ask the Clorraine extension for the driver
/// (`${command:clorraine.driverPath}`) instead of naming a path, so the project
/// works on any machine with Clorraine installed and survives upgrades.
const TASKS: &str = include_str!("../../templates/starter/tasks.json");

/// Creates the starter project named by `options` (or the default folder).
pub fn run(options: &Options) -> Result<(), EnvironmentError> {
    let name = options.files.first().map(String::as_str).unwrap_or(DEFAULT_FOLDER);
    let folder = Path::new(name);
    prepare_folder(folder)?;
    for (relative, text) in [
        ("main.clo", MAIN),
        ("README.md", README),
        (".gitignore", "build/\n"),
        (".vscode/settings.json", SETTINGS),
        (".vscode/extensions.json", EXTENSIONS),
        (".vscode/tasks.json", TASKS),
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
    fn shell_words_are_quoted_only_when_needed() {
        let unix = |word| platform::shell_word(HostOs::Unix, word);
        assert_eq!(unix("my-app"), "my-app");
        assert_eq!(unix("my app"), "'my app'");
        assert_eq!(unix("it's"), "'it'\\''s'");
    }

    #[test]
    fn the_templates_are_complete() {
        assert!(MAIN.contains("const MESSAGE: str"));
        assert!(TASKS.contains("${command:clorraine.driverPath}"));
        assert!(EXTENSIONS.contains("openlorraine.clorraine"));
    }
}
