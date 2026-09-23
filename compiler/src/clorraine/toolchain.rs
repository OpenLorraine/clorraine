//! The profile toolchain table for the `clorraine` developer driver.
//!
//! One place owns the exact C-builder command lines from the specification's
//! compile matrix and the runner each profile uses. Every command is returned
//! as an argument vector and spawned directly, so no shell ever sees a path:
//! a file name with spaces or metacharacters reaches the compiler as one
//! literal argument.
//!
//! The `host` flags mirror the project's established host build; the
//! `hosted-amiga` flags are the GCC development and release lines.

use std::path::Path;

use super::display;
use crate::profile::Profile;

/// A C compiler with its fixed flags for one profile.
pub struct Tool {
    /// The compiler executable name.
    pub program: &'static str,
    /// Flags that are the same in debug and release.
    pub base: &'static [&'static str],
    /// Link libraries; empty for the host profile.
    pub libs: &'static [&'static str],
}

/// The way a built program is executed for a profile.
pub enum Runner {
    /// Execute the binary on the host.
    Native,
    /// Run the Amiga binary under `vamos`.
    Vamos,
}

/// Host compiler base flags.
const HOST_BASE: &[&str] = &["-std=c99"];

/// Amiga compiler base flags from the compile matrix.
const AMIGA_BASE: &[&str] = &["-std=c99", "-mcrt=nix20", "-m68000", "-fwrapv"];

/// Debug optimization flags.
const DEBUG_FLAGS: &[&str] = &["-O0", "-g"];

/// Release optimization flags.
const RELEASE_FLAGS: &[&str] = &["-O2"];

/// Warning flags for a debug build; warnings are errors.
///
/// The release line of the compile matrix omits warnings, so these are
/// appended in debug only.
const WARNINGS: &[&str] = &["-Wall", "-Wextra", "-Werror"];

/// The Amiga link libraries appended on every hosted-amiga build.
const AMIGA_LIBS: &[&str] = &["-lamiga", "-lm"];

/// Returns the C compiler for a profile.
pub fn tool(profile: Profile) -> Tool {
    match profile {
        Profile::Host => Tool {
            program: "cc",
            base: HOST_BASE,
            libs: &[],
        },
        Profile::HostedAmiga => Tool {
            program: "m68k-amigaos-gcc",
            base: AMIGA_BASE,
            libs: AMIGA_LIBS,
        },
    }
}

/// Assembles the C compiler argument vector for one build.
///
/// The shape is `<compiler> <base> <mode> [warnings] -o <bin> <c> <libs>`,
/// which is exactly the compile matrix line with the artifact paths filled in.
/// The warning flags appear in debug only, because the matrix's release line
/// has none.
pub fn command(tool: &Tool, release: bool, c_path: &Path, bin_path: &Path) -> Vec<String> {
    let mut args = vec![tool.program.to_owned()];
    args.extend(tool.base.iter().map(|flag| (*flag).to_owned()));
    let mode = if release { RELEASE_FLAGS } else { DEBUG_FLAGS };
    args.extend(mode.iter().map(|flag| (*flag).to_owned()));
    if !release {
        args.extend(WARNINGS.iter().map(|flag| (*flag).to_owned()));
    }
    args.push("-o".to_owned());
    args.push(bin_path.display().to_string());
    args.push(c_path.display().to_string());
    args.extend(tool.libs.iter().map(|flag| (*flag).to_owned()));
    args
}

/// Returns the default runner for a profile.
pub fn runner(profile: Profile) -> Runner {
    match profile {
        Profile::Host => Runner::Native,
        Profile::HostedAmiga => Runner::Vamos,
    }
}

/// Assembles the runner argument vector for a built program.
pub fn runner_command(runner: &Runner, bin: &Path) -> Vec<String> {
    match runner {
        Runner::Native => vec![bin.display().to_string()],
        Runner::Vamos => vec!["vamos".to_owned(), bin.display().to_string()],
    }
}

/// The exact host-side command that boots FS-UAE for a built program.
///
/// The SDK development container cannot launch a sibling container, so an
/// emulator run is handed off: the driver prints this command and exits zero.
/// With `capture` the host wrapper is asked to record the output instead of
/// opening a display.
///
/// The built path is derived from the source file's name, so it is printed
/// single-quoted: a developer who pastes the line cannot have a byte in the
/// name interpreted by the shell. Control bytes are stripped first, so a name
/// cannot drive terminal escapes either.
pub fn fs_uae_handoff(bin: &Path, capture: bool) -> String {
    let prefix = if capture { "RUN_CAPTURE=1 " } else { "" };
    let raw = bin.display().to_string();
    let path = display::sanitize(&raw);
    format!(
        "FS-UAE must run on the host. From the repository root:\n  {prefix}make -C docker run PROG={}\n",
        shell_single_quote(&path)
    )
}

/// Wraps `text` in single quotes for a POSIX shell.
///
/// Inside single quotes every byte is literal, so only the quote character
/// needs escaping: `'\''` closes the quote, emits a literal quote, and opens a
/// new one. The result is safe to paste as one shell word.
fn shell_single_quote(text: &str) -> String {
    let mut quoted = String::with_capacity(text.len() + 2);
    quoted.push('\'');
    for character in text.chars() {
        if character == '\'' {
            quoted.push_str("'\\''");
        } else {
            quoted.push(character);
        }
    }
    quoted.push('\'');
    quoted
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_host_command_is_the_cc_line_with_artifact_paths() {
        let tool = tool(Profile::Host);
        assert_eq!(
            command(&tool, false, Path::new("out.c"), Path::new("prog")).join(" "),
            "cc -std=c99 -O0 -g -Wall -Wextra -Werror -o prog out.c"
        );
        assert_eq!(
            command(&tool, true, Path::new("out.c"), Path::new("prog")).join(" "),
            "cc -std=c99 -O2 -o prog out.c"
        );
    }

    #[test]
    fn the_amiga_command_is_the_gcc_matrix_line_with_float_libraries() {
        let tool = tool(Profile::HostedAmiga);
        assert_eq!(
            command(&tool, false, Path::new("out.c"), Path::new("prog")).join(" "),
            "m68k-amigaos-gcc -std=c99 -mcrt=nix20 -m68000 -fwrapv -O0 -g -Wall -Wextra -Werror -o prog out.c -lamiga -lm"
        );
    }

    #[test]
    fn the_amiga_release_command_omits_the_warning_flags() {
        let tool = tool(Profile::HostedAmiga);
        assert_eq!(
            command(&tool, true, Path::new("out.c"), Path::new("prog")).join(" "),
            "m68k-amigaos-gcc -std=c99 -mcrt=nix20 -m68000 -fwrapv -O2 -o prog out.c -lamiga -lm"
        );
    }

    #[test]
    fn each_profile_has_its_runner() {
        assert!(matches!(runner(Profile::Host), Runner::Native));
        assert!(matches!(runner(Profile::HostedAmiga), Runner::Vamos));
        assert_eq!(runner_command(&Runner::Native, Path::new("p")).join(" "), "p");
        assert_eq!(runner_command(&Runner::Vamos, Path::new("p")).join(" "), "vamos p");
    }

    #[test]
    fn the_handoff_prints_the_host_command_and_capture_prefix() {
        let bin = Path::new("build/clorraine/hello/hosted-amiga/hello");
        assert_eq!(
            fs_uae_handoff(bin, false),
            "FS-UAE must run on the host. From the repository root:\n  \
             make -C docker run PROG='build/clorraine/hello/hosted-amiga/hello'\n"
        );
        assert!(fs_uae_handoff(bin, true).contains(
            "  RUN_CAPTURE=1 make -C docker run PROG='build/clorraine/hello/hosted-amiga/hello'\n"
        ));
    }

    #[test]
    fn the_handoff_quotes_a_path_so_a_pasted_command_cannot_inject() {
        // A stem with a quote, a substitution, and a control byte must be one
        // inert shell word: the quote is escaped and the control byte removed.
        let bin = Path::new("build/clorraine/a$(touch PWNED)'b\x1b/hosted-amiga/x");
        let text = fs_uae_handoff(bin, false);
        assert!(
            text.contains("PROG='build/clorraine/a$(touch PWNED)'\\''b\u{FFFD}/hosted-amiga/x'"),
            "the path must be one single-quoted word, got:\n{text}"
        );
        assert!(
            !text.contains('\x1b'),
            "a control byte must not survive into the printed command, got:\n{text}"
        );
    }
}
