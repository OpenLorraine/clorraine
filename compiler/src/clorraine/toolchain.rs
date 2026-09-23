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

/// Warning flags shared by every build; warnings are errors.
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
/// The shape is `<compiler> <base> <mode> <warnings> -o <bin> <c> <libs>`,
/// which is exactly the compile matrix line with the artifact paths filled in.
pub fn command(tool: &Tool, release: bool, c_path: &Path, bin_path: &Path) -> Vec<String> {
    let mut args = vec![tool.program.to_owned()];
    args.extend(tool.base.iter().map(|flag| (*flag).to_owned()));
    let mode = if release { RELEASE_FLAGS } else { DEBUG_FLAGS };
    args.extend(mode.iter().map(|flag| (*flag).to_owned()));
    args.extend(WARNINGS.iter().map(|flag| (*flag).to_owned()));
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
pub fn fs_uae_handoff(bin: &Path, capture: bool) -> String {
    let prefix = if capture { "RUN_CAPTURE=1 " } else { "" };
    format!(
        "FS-UAE must run on the host. From the repository root:\n  {prefix}make -C docker run PROG={}\n",
        bin.display()
    )
}
