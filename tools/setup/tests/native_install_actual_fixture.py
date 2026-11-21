"""Real installed-output assertions shared by explicit installer acceptance groups."""

from __future__ import annotations

import json
import tarfile
import tempfile
import unittest
from pathlib import Path

from native_install_context import ActualCandidate, load_candidate, read_json, require_update_pair
from native_install_fixture import StateShellFixture, tree_state
from native_manifest_data import encode, file_digest
from native_transport_fixture import run_process


HELLO = b'''module fixture.Hello;
import { println } from std.io;
func main(): i32 {
    println("Hello, Clorraine!");
    return 0;
}
'''
"""Passive source compiled by real candidate tools, not by observation executables."""
PROFILES = ("host", "hosted-amiga", "hosted-amiga-13")
"""Every required native-install smoke profile."""


def private_directory(test: unittest.TestCase, work: Path, prefix: str) -> Path:
    """Register cleanup before creating any files beneath one owned disposable child."""
    temporary = tempfile.TemporaryDirectory(prefix=prefix, dir=work)
    test.addCleanup(temporary.cleanup)
    return Path(temporary.name)


def make_shell(parent: Path, name: str, candidate: ActualCandidate) -> StateShellFixture:
    """Copy exact candidate entry bytes into this proof's private process storage."""
    root = parent / name
    root.mkdir()
    return StateShellFixture(root, candidate.bootstrap, candidate.manifest, candidate.payloads)


def run_hellos(test: unittest.TestCase, driver: Path, cwd: Path, environment: dict[str, str],
               profiles: tuple[str, ...] = PROFILES) -> None:
    """Compile/run source with the selected real driver and compare exact fixture output."""
    source = cwd / "hello ; installed proof.clo"
    source.write_bytes(HELLO)
    for profile in profiles:
        result = run_process([str(driver), "run", str(source), "-p", profile],
                             cwd, environment, timeout=120)
        test.assertEqual(result.returncode, 0, profile + ": " + result.stderr)
        test.assertEqual(result.stdout.replace("\r", ""), "Hello, Clorraine!\n", profile)


def retained_manifest(installation: Path, *, expected: bytes | None = None) -> tuple[dict, Path]:
    """Find retained manifest data without pinning its private transaction filename."""
    matches = []
    for path in installation.rglob("*"):
        if path.is_symlink() or not path.is_file() or path.stat().st_size > 16 * 1024 * 1024:
            continue
        if expected is not None:
            if path.stat().st_size == len(expected) and path.read_bytes() == expected:
                matches.append((json.loads(expected), path))
            continue
        try:
            value = read_json(path)
        except (ValueError, UnicodeError):
            continue
        if {"schema", "release", "assets", "support", "core_version"} <= set(value):
            matches.append((value, path))
    if not matches:
        raise AssertionError("installation does not retain the selected manifest bytes")
    return matches[0]


def assert_working(test: unittest.TestCase, shell: StateShellFixture, root: Path,
                   candidate: ActualCandidate) -> Path:
    """Require selected real binaries, exact manifest/VSIX and all real profile outputs."""
    forbidden = {"sudo", "apt", "apt-get", "code", "make", "cargo", "rustc", "docker", "gh", "wget"}
    test.assertFalse([item for item in shell.events("command") if item["name"] in forbidden],
                     "installer changed the host/editor or compiled an installation dependency")
    test.assertFalse([item for item in shell.events("request")
                      if item["url"] not in shell.configuration["routes"]],
                     "installer attempted an undeclared download, emulator or media fetch")
    current = root / "current"
    test.assertTrue(current.is_symlink(), "successful install did not activate current")
    installation = current.resolve(strict=True)
    test.assertTrue(installation.is_relative_to(root.resolve() / "installations"))
    retained_manifest(installation, expected=(candidate.bootstrap / "manifest.json").read_bytes())
    core_asset = candidate.asset("core")
    with tarfile.open(candidate.payloads[core_asset["identity"]], "r:") as archive:
        metadata = json.loads(archive.extractfile("core.json").read())
    for name in ("clc", "clorraine", "clorraine-lsp"):
        real = installation / "bin" / name
        stable = root / "bin" / name
        test.assertTrue(stable.is_symlink(), name)
        test.assertFalse(real.is_symlink(), "real compiler executable required in assembled bin")
        test.assertEqual(stable.resolve(strict=True), real)
        test.assertEqual(file_digest(real), metadata["files"]["bin/" + name]["sha256"])
        version = run_process([str(stable), "--version"], shell.root, shell.environment)
        test.assertEqual(version.returncode, 0, version.stderr)
        test.assertEqual(version.stdout, name + " " + candidate.manifest["core_version"] + "\n")
    vsix = [path for path in installation.rglob("*.vsix") if path.is_file() and not path.is_symlink()]
    test.assertEqual(len(vsix), 1, "exact downloaded VSIX must be retained")
    test.assertEqual(file_digest(vsix[0]), candidate.asset("vsix")["sha256"])
    run_hellos(test, root / "bin/clorraine", shell.root, shell.environment)
    return installation


class ActualInstallCase(unittest.TestCase):
    """Require both independently assembled actual candidates before creating any work."""

    def setUp(self) -> None:
        """Unavailable actual inputs are errors, not local fixture substitutes or skips."""
        self.initial = load_candidate("CLORRAINE_INSTALL_ACCEPTANCE", "v1.2.0-r1")
        self.update = load_candidate("CLORRAINE_INSTALL_UPDATE_ACCEPTANCE", "v1.2.0-r2")
        require_update_pair(self.initial, self.update)
        self.initial_work = private_directory(self, self.initial.work_root, "real-initial-")
        self.update_work = private_directory(self, self.update.work_root, "real-update-")
        self.sequence = 0

    def baseline(
        self, *, default_root: bool = False,
    ) -> tuple[StateShellFixture, StateShellFixture, Path, Path, dict, Path]:
        """Install real initial bytes, optionally exercising the literal HOME default."""
        self.sequence += 1
        first = make_shell(self.initial_work, "initial-" + str(self.sequence), self.initial)
        second = make_shell(self.update_work, "update-" + str(self.sequence), self.update)
        arguments = ["--release", self.initial.manifest["release"]]
        if default_root:
            first.install_root = first.home / ".clorraine"
            first.configuration["bootstrap_arguments"] = arguments
            first.config_path.write_bytes(encode(first.configuration))
            result = run_process(["/bin/sh", str(first.shipped / "install.sh"), *arguments],
                                 first.root, first.environment, timeout=600)
        else:
            result = first.invoke(arguments)
        self.assertEqual(result.returncode, 0, result.stderr)
        root = first.install_root
        installation = assert_working(self, first, root, self.initial)
        settings = root / "user-settings.json"
        settings.write_bytes(b'{"preserve":"user configuration"}\n')
        return first, second, root, installation, tree_state(installation), settings

    def assert_old_unchanged(self, shell: StateShellFixture, root: Path,
                             installation: Path, snapshot: dict, settings: Path) -> None:
        """Check old bytes/selection, then run the real old tools without fault observers."""
        self.assertEqual((root / "current").resolve(strict=True), installation)
        self.assertEqual(tree_state(installation), snapshot)
        self.assertEqual(settings.read_bytes(), b'{"preserve":"user configuration"}\n')
        assert_working(self, shell, root, self.initial)

    def assert_failed_update(self, shell: StateShellFixture, result, root: Path) -> None:
        """Require failure diagnostics and prohibit automatic host/editor changes."""
        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertTrue(result.stderr.strip(), "failed update must name its step and remedy")
        self.assertFalse(any(line.startswith(("export PATH=", "code --install-extension "))
                             for line in result.stdout.splitlines()))
        forbidden = {"sudo", "apt", "apt-get", "code", "make", "cargo", "rustc"}
        self.assertFalse([item for item in shell.events("command") if item["name"] in forbidden])
        self.assertTrue((root / "current").is_symlink(), "failed update lost the old selection")
