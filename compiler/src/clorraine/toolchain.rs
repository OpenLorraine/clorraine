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
    /// Release-only flags; the debug line is the same for every profile.
    pub release: &'static [&'static str],
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

/// A500/Kickstart 1.3 base flags: the same CPU and ABI, with the 1.3 libnix.
const AMIGA_13_BASE: &[&str] = &["-std=c99", "-mcrt=nix13", "-m68000", "-fwrapv"];

/// Debug optimization flags.
const DEBUG_FLAGS: &[&str] = &["-O0", "-g"];

/// Release flags for a Linux host; the seam starts at the current behavior.
const HOST_RELEASE_GNU: &[&str] = &["-O2"];

/// Release flags for a host without the pinned GNU linker.
const HOST_RELEASE_FALLBACK: &[&str] = &["-O2"];

/// Release flags shared by the two Amiga profiles.
const M68K_RELEASE_FLAGS: &[&str] = &["-O2"];

/// Release flags for the freestanding m68k and host-run tools.
const FREESTANDING_RELEASE_FLAGS: &[&str] = &["-O2"];

/// Warning flags for a debug build; warnings are errors.
///
/// The release line of the compile matrix omits warnings, so these are
/// appended in debug only.
const WARNINGS: &[&str] = &["-Wall", "-Wextra", "-Werror"];

/// The Amiga link libraries appended on every hosted-amiga build.
const AMIGA_LIBS: &[&str] = &["-lamiga", "-lm"];

/// Freestanding m68k build flags: no C runtime, its own start-up, soft floats.
const FREESTANDING_BUILD_BASE: &[&str] = &[
    "-std=c99",
    "-ffreestanding",
    "-fno-builtin",
    "-nostdlib",
    "-m68000",
    "-fwrapv",
];

/// The freestanding host-run flags: the same no-libc stance on the host.
const FREESTANDING_HOST_BASE: &[&str] = &["-std=c99", "-ffreestanding", "-fno-builtin", "-nostdlib"];

/// The fallback host-run flags for a CPU with no shim syscall layer: the host
/// C runtime supplies the console write and exit, so `-nostdlib` is dropped.
const FREESTANDING_HOST_FALLBACK_BASE: &[&str] =
    &["-std=c99", "-ffreestanding", "-fno-builtin"];

/// The freestanding m68k link libraries (the self-contained soft-float runtime).
const FREESTANDING_LIBS: &[&str] = &["-lg"];

/// Selects the host release flags by linker family.
///
/// The seam is a pure function so both outcomes run on the Linux test host;
/// the value is filled in when the shipped flags are wired.
fn host_release_flags(gnu_linker: bool) -> &'static [&'static str] {
    if gnu_linker {
        HOST_RELEASE_GNU
    } else {
        HOST_RELEASE_FALLBACK
    }
}

/// True on the Linux host, whose pinned linker accepts `--gc-sections`.
fn host_is_linux() -> bool {
    cfg!(target_os = "linux")
}

/// Returns the C compiler for a profile.
pub fn tool(profile: Profile) -> Tool {
    match profile {
        Profile::Host => Tool {
            program: "cc",
            base: HOST_BASE,
            release: host_release_flags(host_is_linux()),
            libs: &[],
        },
        Profile::HostedAmiga => Tool {
            program: "m68k-amigaos-gcc",
            base: AMIGA_BASE,
            release: M68K_RELEASE_FLAGS,
            libs: AMIGA_LIBS,
        },
        Profile::HostedAmiga13 => Tool {
            program: "m68k-amigaos-gcc",
            base: AMIGA_13_BASE,
            release: M68K_RELEASE_FLAGS,
            libs: AMIGA_LIBS,
        },
        Profile::Freestanding => Tool {
            program: "m68k-amigaos-gcc",
            base: FREESTANDING_BUILD_BASE,
            release: FREESTANDING_RELEASE_FLAGS,
            libs: FREESTANDING_LIBS,
        },
    }
}

/// True when the runtime shim has a hand-written syscall layer for this host.
///
/// The freestanding host run uses raw `write`/`exit` syscalls only on the
/// architectures the shim covers; every other host falls back to the platform
/// C runtime. Keeping the answer in one function makes the fallback a single
/// branch instead of a scatter of `cfg!` checks.
pub fn native_syscalls() -> bool {
    cfg!(any(target_arch = "x86_64", target_arch = "aarch64"))
}

/// The C compiler that links a profile's host-run artifact, if it has one.
///
/// The host profile runs its build artifact directly. The hosted-amiga profile
/// runs its build artifact under `vamos`, so it has no separate run tool. The
/// freestanding profile builds an m68k artifact that cannot run here, so its
/// run leg compiles the same translation unit for the host with `cc`.
pub fn run_tool(profile: Profile, native_syscalls: bool) -> Option<Tool> {
    match profile {
        Profile::Host => Some(tool(Profile::Host)),
        Profile::HostedAmiga | Profile::HostedAmiga13 => None,
        Profile::Freestanding => Some(Tool {
            program: "cc",
            base: if native_syscalls {
                FREESTANDING_HOST_BASE
            } else {
                FREESTANDING_HOST_FALLBACK_BASE
            },
            release: FREESTANDING_RELEASE_FLAGS,
            libs: &[],
        }),
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
    let mode = if release { tool.release } else { DEBUG_FLAGS };
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
        Profile::HostedAmiga | Profile::HostedAmiga13 => Runner::Vamos,
        Profile::Freestanding => Runner::Native,
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
pub fn fs_uae_handoff(bin: &Path, capture: bool, profile: Profile) -> String {
    let prefix = if capture { "RUN_CAPTURE=1 " } else { "" };
    let target = match profile {
        Profile::HostedAmiga13 => {
            "AMIGA_MODEL=A500 KICKSTART=/amiga/roms/amiga-os-130.rom RUNNER=adf "
        }
        _ => "",
    };
    let raw = bin.display().to_string();
    let path = display::sanitize(&raw);
    format!(
        "FS-UAE must run on the host. From the repository root:\n  {target}{prefix}make -C docker run PROG={}\n",
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
    fn the_a500_command_is_the_nix13_matrix_line() {
        let tool = tool(Profile::HostedAmiga13);
        assert_eq!(
            command(&tool, false, Path::new("out.c"), Path::new("prog")).join(" "),
            "m68k-amigaos-gcc -std=c99 -mcrt=nix13 -m68000 -fwrapv -O0 -g -Wall -Wextra -Werror -o prog out.c -lamiga -lm"
        );
        assert_eq!(
            command(&tool, true, Path::new("out.c"), Path::new("prog")).join(" "),
            "m68k-amigaos-gcc -std=c99 -mcrt=nix13 -m68000 -fwrapv -O2 -o prog out.c -lamiga -lm"
        );
    }

    #[test]
    fn each_profile_has_its_runner() {
        assert!(matches!(runner(Profile::Host), Runner::Native));
        assert!(matches!(runner(Profile::HostedAmiga), Runner::Vamos));
        assert!(matches!(runner(Profile::HostedAmiga13), Runner::Vamos));
        assert_eq!(runner_command(&Runner::Native, Path::new("p")).join(" "), "p");
        assert_eq!(runner_command(&Runner::Vamos, Path::new("p")).join(" "), "vamos p");
    }

    #[test]
    fn the_handoff_prints_the_host_command_and_capture_prefix() {
        let bin = Path::new("build/clorraine/hello/hosted-amiga/hello");
        assert_eq!(
            fs_uae_handoff(bin, false, Profile::HostedAmiga),
            "FS-UAE must run on the host. From the repository root:\n  \
             make -C docker run PROG='build/clorraine/hello/hosted-amiga/hello'\n"
        );
        assert!(fs_uae_handoff(bin, true, Profile::HostedAmiga).contains(
            "  RUN_CAPTURE=1 make -C docker run PROG='build/clorraine/hello/hosted-amiga/hello'\n"
        ));
    }

    #[test]
    fn the_a500_handoff_selects_the_a500_rom_and_adf() {
        let bin = Path::new("build/clorraine/hello/hosted-amiga-13/hello");
        let text = fs_uae_handoff(bin, true, Profile::HostedAmiga13);
        assert!(text.contains("AMIGA_MODEL=A500"));
        assert!(text.contains("KICKSTART=/amiga/roms/amiga-os-130.rom"));
        assert!(text.contains("RUNNER=adf"));
        assert!(text.contains("RUN_CAPTURE=1"));
    }

    #[test]
    fn the_handoff_quotes_a_path_so_a_pasted_command_cannot_inject() {
        // A stem with a quote, a substitution, and a control byte must be one
        // inert shell word: the quote is escaped and the control byte removed.
        let bin = Path::new("build/clorraine/a$(touch PWNED)'b\x1b/hosted-amiga/x");
        let text = fs_uae_handoff(bin, false, Profile::HostedAmiga);
        assert!(
            text.contains("PROG='build/clorraine/a$(touch PWNED)'\\''b\u{FFFD}/hosted-amiga/x'"),
            "the path must be one single-quoted word, got:\n{text}"
        );
        assert!(
            !text.contains('\x1b'),
            "a control byte must not survive into the printed command, got:\n{text}"
        );
    }

    #[test]
    fn the_freestanding_build_command_is_the_no_libc_matrix_line() {
        let tool = tool(Profile::Freestanding);
        assert_eq!(
            command(&tool, false, Path::new("out.c"), Path::new("prog")).join(" "),
            "m68k-amigaos-gcc -std=c99 -ffreestanding -fno-builtin -nostdlib -m68000 -fwrapv -O0 -g -Wall -Wextra -Werror -o prog out.c -lg"
        );
    }

    #[test]
    fn the_freestanding_host_run_command_keeps_or_drops_nostdlib() {
        let syscalls = run_tool(Profile::Freestanding, true).expect("freestanding has a run tool");
        assert_eq!(syscalls.program, "cc");
        assert_eq!(
            command(&syscalls, false, Path::new("out.c"), Path::new("prog")).join(" "),
            "cc -std=c99 -ffreestanding -fno-builtin -nostdlib -O0 -g -Wall -Wextra -Werror -o prog out.c"
        );
        let fallback = run_tool(Profile::Freestanding, false).expect("freestanding has a run tool");
        assert_eq!(
            command(&fallback, false, Path::new("out.c"), Path::new("prog")).join(" "),
            "cc -std=c99 -ffreestanding -fno-builtin -O0 -g -Wall -Wextra -Werror -o prog out.c"
        );
    }

    #[test]
    fn only_profiles_with_a_host_run_have_a_run_tool() {
        assert!(run_tool(Profile::HostedAmiga, true).is_none());
        assert!(run_tool(Profile::HostedAmiga13, true).is_none());
        assert!(run_tool(Profile::Host, true).is_some());
        assert!(matches!(runner(Profile::Freestanding), Runner::Native));
    }
}
