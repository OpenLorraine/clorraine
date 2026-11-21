//! Display and authentication precedence through copied emulator entry points.
//!
//! Docker is replaced by a logging process. These tests check the Compose
//! invocation and its host environment using private configuration and program
//! fixtures. The logger does not resolve Compose configuration: container-side
//! XAUTHORITY, the authentication-file bind and real display authentication must
//! be checked separately with actual Docker and the emulator.

use std::fs;
use std::io::ErrorKind;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

/// Deliberately conflicting display stored in the fixture's configuration.
const DESKTOP_DISPLAY: &str = ":990";
/// Display explicitly selected by a caller.
const PRIVATE_DISPLAY: &str = ":991";
/// Synthetic image configured in the fixture's SDK environment.
const IMAGE: &str = "clorraine-display-fixture:local";

/// Logs each external Docker call without executing any container or its command.
const FAKE_DOCKER: &str = r#"#!/bin/bash
set -euo pipefail
log=$(mktemp "${NATIVE_GATE_LOG_DIR}/docker.XXXXXX")
printf '%s\0' "${DISPLAY-}" "${XAUTHORITY-}" "$@" > "$log"
"#;

/// Prevents an accidental direct display-server or emulator launch through PATH.
const FORBIDDEN_DESKTOP_TOOL: &str = r#"#!/bin/bash
printf '%s\0' "$0" "$@" >> "$NATIVE_GATE_DENIED_LOG"
printf 'unexpected direct desktop process: %s\n' "$0" >&2
exit 97
"#;

/// Owns one collision-resistant scratch directory, including failure cleanup.
struct ScratchDir {
    /// Only this directory is removed; its caller-provided parent is retained.
    path: PathBuf,
}

impl ScratchDir {
    /// Allocates a unique directory beneath the configured temporary root.
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let base = std::env::var_os("CODEOPS_TMPDIR")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir);
        fs::create_dir_all(&base).expect("create temporary parent");
        loop {
            let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
            let path = base.join(format!("native-gate-{}-{sequence}", std::process::id()));
            match fs::create_dir(&path) {
                Ok(()) => return Self { path },
                Err(error) if error.kind() == ErrorKind::AlreadyExists => continue,
                Err(error) => panic!("cannot create {}: {error}", path.display()),
            }
        }
    }
}

impl Drop for ScratchDir {
    /// Cleans only this test's owned tree, including copied wrappers and logs.
    fn drop(&mut self) {
        if let Err(error) = fs::remove_dir_all(&self.path) {
            eprintln!("cannot clean {}: {error}", self.path.display());
        }
    }
}

/// Writes controlled fixture bytes and optionally makes a process executable.
fn write_fixture(root: &Path, name: &str, bytes: &[u8], executable: bool) -> PathBuf {
    let path = root.join(name);
    fs::create_dir_all(path.parent().expect("fixture parent")).expect("create fixture parent");
    fs::write(&path, bytes).unwrap_or_else(|error| panic!("write {}: {error}", path.display()));
    if executable {
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755))
            .expect("make fixture executable");
    }
    path
}

/// Quotes a literal value for the shell-readable configuration fixture.
fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

/// Selects one of the two documented public wrapper invocations.
#[derive(Clone, Copy, Debug)]
enum Route {
    /// Invoke the emulator wrapper directly.
    Direct,
    /// Invoke the program wrapper and follow its emulator handoff.
    Wrapped,
}

/// Independent configuration, auth files and external process log for one case.
struct Fixture {
    /// Keeps every generated file alive until assertions finish.
    scratch: ScratchDir,
    /// Authentication selected when the caller supplies no override.
    desktop_authority: PathBuf,
    /// Distinct caller authentication path, intentionally containing spaces.
    private_authority: PathBuf,
    /// Configuration bytes must remain unchanged after either invocation.
    configuration: String,
}

impl Fixture {
    /// Copies wrapper/configuration bytes intact and supplies local synthetic resources.
    fn new() -> Self {
        let scratch = ScratchDir::new();
        let root = &scratch.path;
        for directory in ["docker/scripts", "logs", "fakebin", "home", "tmp"] {
            fs::create_dir_all(root.join(directory)).expect("create fixture directory");
        }
        let repository = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("compiler must have a repository parent");
        for name in ["run.sh", "emulator.sh"] {
            let relative = Path::new("docker/scripts").join(name);
            let source = repository.join(&relative);
            let target = root.join(&relative);
            fs::copy(&source, &target)
                .unwrap_or_else(|error| panic!("copy {}: {error}", source.display()));
            fs::set_permissions(target, fs::Permissions::from_mode(0o755))
                .expect("make copied wrapper executable");
        }
        fs::copy(
            repository.join("docker/compose.yaml"),
            root.join("docker/compose.yaml"),
        )
        .expect("copy Compose configuration intact");
        write_fixture(root, "fakebin/docker", FAKE_DOCKER.as_bytes(), true);
        for name in ["fs-uae", "fs-uae-launcher", "Xvfb", "xvfb-run"] {
            write_fixture(
                root,
                &format!("fakebin/{name}"),
                FORBIDDEN_DESKTOP_TOOL.as_bytes(),
                true,
            );
        }
        let desktop_authority =
            write_fixture(root, "desktop.xauth", b"desktop auth fixture\n", false);
        let private_authority = write_fixture(
            root,
            "private authority with spaces.xauth",
            b"private auth fixture\n",
            false,
        );
        write_fixture(root, "kickstart.rom", &vec![0; 512 * 1024], false);
        // A one-hunk Amiga program returning zero; the logging Docker never executes it.
        let program: Vec<u8> = [0x3f3_u32, 0, 1, 0, 0, 1, 0x3e9, 1, 0x7000_4e75, 0x3f2]
            .into_iter()
            .flat_map(u32::to_be_bytes)
            .collect();
        write_fixture(root, "build/fixture-program", &program, true);
        let configuration = [
            ("DISPLAY", DESKTOP_DISPLAY),
            (
                "XAUTHORITY",
                desktop_authority.to_str().expect("UTF-8 auth fixture path"),
            ),
            ("KICKSTART", "kickstart.rom"),
            ("AMIGA_MODEL", "A1200"),
            ("AMIGA_DATA", root.to_str().expect("UTF-8 fixture root")),
            ("IMAGE", IMAGE),
            ("RUNNER", "dir"),
            ("RUN_CAPTURE", "0"),
            ("FS_UAE_ARGS", "--fullscreen=0"),
        ]
        .iter()
        .map(|(name, value)| format!("export {name}={}\n", shell_quote(value)))
        .collect::<String>();
        write_fixture(root, "docker/.env", configuration.as_bytes(), false);
        Self {
            scratch,
            desktop_authority,
            private_authority,
            configuration,
        }
    }

    /// Runs a copied entry with explicit caller overrides and no inherited display state.
    fn launch(&self, route: Route, display: Option<&str>, authority: Option<&Path>) -> Output {
        let root = &self.scratch.path;
        let name = match route {
            Route::Direct => "emulator.sh",
            Route::Wrapped => "run.sh",
        };
        let mut command = Command::new("/bin/bash");
        command
            .arg(root.join("docker/scripts").join(name))
            .current_dir(root)
            .env_clear()
            .env(
                "PATH",
                std::env::join_paths([root.join("fakebin"), "/usr/bin".into(), "/bin".into()])
                    .expect("valid fixture PATH"),
            )
            .env("HOME", root.join("home"))
            .env(
                "USER",
                std::env::var_os("USER").unwrap_or_else(|| "fixture".into()),
            )
            .env("SHELL", "/bin/bash")
            .env("LC_ALL", "C")
            .env("TMPDIR", root.join("tmp"))
            .env("CODEOPS_TMPDIR", root.join("tmp"))
            .env("NATIVE_GATE_LOG_DIR", root.join("logs"))
            .env(
                "NATIVE_GATE_DENIED_LOG",
                root.join("unexpected-desktop.log"),
            )
            .env_remove("DISPLAY")
            .env_remove("XAUTHORITY");
        if matches!(route, Route::Wrapped) {
            command.arg(root.join("build/fixture-program"));
        }
        command.arg("--fullscreen=0");
        if let Some(value) = display {
            command.env("DISPLAY", value);
        }
        if let Some(value) = authority {
            command.env("XAUTHORITY", value);
        }
        command
            .output()
            .expect("run copied wrapper with fake Docker")
    }

    /// Reads NUL-framed external observations, preserving spaces inside each argument.
    fn calls(&self) -> Vec<DockerCall> {
        let mut paths: Vec<PathBuf> = fs::read_dir(self.scratch.path.join("logs"))
            .expect("read Docker logs")
            .map(|entry| entry.expect("Docker log entry").path())
            .collect();
        paths.sort();
        paths
            .into_iter()
            .map(|path| {
                let bytes = fs::read(&path).expect("read external Docker observation");
                assert_eq!(bytes.last(), Some(&0), "truncated Docker log");
                let fields: Vec<String> = bytes[..bytes.len() - 1]
                    .split(|byte| *byte == 0)
                    .map(|field| String::from_utf8(field.to_vec()).expect("UTF-8 fixture argument"))
                    .collect();
                assert!(
                    fields.len() >= 2,
                    "Docker log must include both environment values"
                );
                DockerCall {
                    display: fields[0].clone(),
                    authority: fields[1].clone(),
                    arguments: fields[2..].to_vec(),
                }
            })
            .collect()
    }
}

/// Environment and literal argv observed at the external Docker boundary.
struct DockerCall {
    /// Display visible to the actual Docker process.
    display: String,
    /// Host authentication file visible to the actual Docker process.
    authority: String,
    /// Compose options, service and command, without the Docker executable itself.
    arguments: Vec<String>,
}

impl DockerCall {
    /// Recognizes the final Compose service/command handoff rather than prerequisite probes.
    fn is_emulator_launch(&self) -> bool {
        self.arguments.first().map(String::as_str) == Some("compose")
            && self
                .arguments
                .windows(4)
                .any(|arguments| arguments == ["run", "--rm", "emulator", "fs-uae"])
    }
}

/// Requires the selected pair and an existing host authority file at the Compose handoff.
fn assert_routing(route: Route, override_display: bool, override_authority: bool) {
    let fixture = Fixture::new();
    let display = override_display.then_some(PRIVATE_DISPLAY);
    let authority = override_authority.then_some(fixture.private_authority.as_path());
    let expected_display = display.unwrap_or(DESKTOP_DISPLAY);
    let expected_authority = authority.unwrap_or(&fixture.desktop_authority);
    let output = fixture.launch(route, display, authority);
    assert!(
        output.status.success(),
        "{route:?}: stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!fixture.scratch.path.join("unexpected-desktop.log").exists());
    let calls = fixture.calls();
    let launches: Vec<_> = calls
        .iter()
        .filter(|call| call.is_emulator_launch())
        .collect();
    assert!(
        !launches.is_empty(),
        "{route:?}: no Compose run --rm emulator fs-uae command reached Docker"
    );
    for call in launches {
        for option in [
            ["-p", "clorraine"],
            ["--env-file", ".env"],
            ["-f", "compose.yaml"],
        ] {
            assert!(
                call.arguments
                    .windows(2)
                    .any(|arguments| arguments == option),
                "{route:?}: missing Compose option {option:?}; argv: {:?}",
                call.arguments
            );
        }
        assert_eq!(
            call.display, expected_display,
            "{route:?}: effective DISPLAY"
        );
        assert_eq!(
            call.authority,
            expected_authority
                .to_str()
                .expect("UTF-8 auth fixture path"),
            "{route:?}: effective XAUTHORITY"
        );
        assert!(
            Path::new(&call.authority).is_file(),
            "{route:?}: effective XAUTHORITY does not name an existing host file"
        );
    }
    assert_eq!(
        fs::read_to_string(fixture.scratch.path.join("docker/.env")).expect("read fixture config"),
        fixture.configuration,
        "caller overrides must not rewrite stored defaults"
    );
}

/// Direct launches retain both nonempty caller-selected display values.
#[test]
fn direct_launch_preserves_the_private_pair() {
    assert_routing(Route::Direct, true, true);
}

/// The complete program-to-emulator chain retains both caller-selected values.
#[test]
fn wrapped_launch_preserves_the_private_pair() {
    assert_routing(Route::Wrapped, true, true);
}

/// Direct launches use configured values when both caller variables are absent.
#[test]
fn direct_launch_uses_defaults_when_callers_are_unset() {
    assert_routing(Route::Direct, false, false);
}

/// The full wrapper chain retains configured defaults without caller overrides.
#[test]
fn wrapped_launch_uses_defaults_when_callers_are_unset() {
    assert_routing(Route::Wrapped, false, false);
}

/// A direct display override must not discard the configured authentication file.
#[test]
fn direct_launch_preserves_a_display_only_override() {
    assert_routing(Route::Direct, true, false);
}

/// A direct authentication override must not discard the configured display.
#[test]
fn direct_launch_preserves_an_authority_only_override() {
    assert_routing(Route::Direct, false, true);
}

/// Both wrapper loads independently preserve a caller's display-only choice.
#[test]
fn wrapped_launch_preserves_a_display_only_override() {
    assert_routing(Route::Wrapped, true, false);
}

/// Both wrapper loads independently preserve a caller's authentication-only choice.
#[test]
fn wrapped_launch_preserves_an_authority_only_override() {
    assert_routing(Route::Wrapped, false, true);
}
