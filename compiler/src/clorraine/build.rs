//! Emitting the C and compiling it for the `clorraine` developer driver.
//!
//! A build has two steps. First the compiler runs in process (no second
//! process, no `clc` binary on `PATH`) and writes the translation unit into the
//! program's artifact directory. Then the profile's C compiler turns that C
//! into a program. A Clorraine diagnostic stops before the C compiler runs; a C
//! compiler rejection is an internal compiler error, because the driver owns
//! the emitted C.
//!
//! The C compiler is spawned with an argument vector, never a shell, so a file
//! name is a single literal argument.

use std::fmt;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::artifacts;
use super::display;
use super::toolchain::{self, Tool};
use crate::diag::DiagnosticFormat;
use crate::profile::Profile;

/// A failure whose cause is a defect in the compiler, not in the program.
///
/// The emitted C is kept on disk so the failure can be inspected; the message
/// names that path and the target C compiler's first error line.
pub struct IceError {
    /// The emitted translation unit that the C compiler rejected.
    pub c_path: PathBuf,
    /// The first non-empty line the target C compiler wrote to stderr.
    pub first_error: String,
}

impl fmt::Display for IceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "error: internal compiler error: the emitted C failed to compile\n  note: emitted C kept at {}\n  first error: {}\n",
            display::sanitize(&self.c_path.display().to_string()),
            display::sanitize(&self.first_error),
        )
    }
}

/// The result of a successful build.
pub struct BuildOutput {
    /// The emitted C translation unit.
    pub c: PathBuf,
    /// The built program.
    pub bin: PathBuf,
}

/// Why a build could not finish.
pub enum BuildFailure {
    /// The compiler reported diagnostics, already written to stderr.
    Diagnostics(i32),
    /// The driver could not finish the build; the message is ready to print.
    Error(String),
}

/// Emits the program's C to `c_path` by calling the compiler in process.
///
/// Returns the compiler's exit code: zero on success, or the diagnostic exit
/// code, in which case no C is written. Diagnostics already went to stderr.
pub fn emit_c(
    profile: Profile,
    files: &[String],
    c_path: &Path,
    format: DiagnosticFormat,
) -> i32 {
    let mut args = vec![
        "--profile".to_owned(),
        profile.name().to_owned(),
        "-o".to_owned(),
        c_path.display().to_string(),
    ];
    if format == DiagnosticFormat::Json {
        args.push("--diagnostics=json".to_owned());
    }
    args.extend(files.iter().cloned());
    crate::driver::run(args)
}

/// Compiles `c_path` into `bin_path` with `tool`.
///
/// A non-zero compiler exit becomes an [`IceError`] carrying the emitted-C path
/// and the compiler's first error line.
pub fn compile(
    tool: &Tool,
    release: bool,
    c_path: &Path,
    bin_path: &Path,
) -> Result<(), IceError> {
    let argv = toolchain::command(tool, release, c_path, bin_path);
    let output = Command::new(&argv[0]).args(&argv[1..]).output();
    match output {
        Ok(output) if output.status.success() => Ok(()),
        Ok(output) => Err(IceError {
            c_path: c_path.to_path_buf(),
            first_error: first_error_line(&output.stderr),
        }),
        Err(error) => Err(IceError {
            c_path: c_path.to_path_buf(),
            first_error: format!("could not run '{}': {error}", argv[0]),
        }),
    }
}

/// Emits and compiles one program, returning the built artifact paths.
///
/// The program directory is recreated first, so a failed build never leaves a
/// stale binary behind.
pub fn build(
    profile: Profile,
    release: bool,
    files: &[String],
    format: DiagnosticFormat,
) -> Result<BuildOutput, BuildFailure> {
    let stem = artifacts::program_stem(files).ok_or_else(|| {
        BuildFailure::Error("clorraine: an input file name has no file stem\n".to_owned())
    })?;
    artifacts::reset_program_dir(profile, &stem).map_err(|error| {
        BuildFailure::Error(format!(
            "clorraine: cannot prepare the artifact directory: {error}\n"
        ))
    })?;
    let c_path = artifacts::c_path(profile, &stem);
    let code = emit_c(profile, files, &c_path, format);
    if code != 0 {
        return Err(BuildFailure::Diagnostics(code));
    }
    let bin_path = artifacts::bin_path(profile, &stem);
    let tool = toolchain::tool(profile);
    compile(&tool, release, &c_path, &bin_path)
        .map_err(|ice| BuildFailure::Error(ice.to_string()))?;
    Ok(BuildOutput {
        c: c_path,
        bin: bin_path,
    })
}

/// The first non-empty line of a compiler's stderr, as lossy UTF-8.
fn first_error_line(stderr: &[u8]) -> String {
    String::from_utf8_lossy(stderr)
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("(the C compiler produced no error output)")
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_failing_c_compiler_is_an_internal_compiler_error() {
        // `false` always exits non-zero, so it stands in for a rejected build.
        let tool = Tool {
            program: "false",
            base: &[],
            libs: &[],
        };
        let error = compile(&tool, false, Path::new("emitted.c"), Path::new("prog"))
            .expect_err("a failing compiler must not succeed");

        assert_eq!(error.c_path, Path::new("emitted.c"));
        let message = error.to_string();
        assert!(
            message.contains("internal compiler error"),
            "the report names the failure, got:\n{message}"
        );
        assert!(
            message.contains("emitted.c"),
            "the report names the emitted C, got:\n{message}"
        );
    }

    #[test]
    fn a_missing_compiler_is_reported_by_name() {
        let tool = Tool {
            program: "clorraine-no-such-compiler",
            base: &[],
            libs: &[],
        };
        let error = compile(&tool, false, Path::new("emitted.c"), Path::new("prog"))
            .expect_err("a missing compiler must not succeed");
        assert!(
            error.first_error.contains("clorraine-no-such-compiler"),
            "the report names the tool, got:\n{}",
            error.first_error
        );
    }
}
