"""Explicit real-cohort installation and printed-command usability proof.

This module is intentionally outside generic ``test_*.py`` discovery. Its
required candidate context is an error when absent; miniature local fixtures
cannot satisfy it or replace real installed host and Amiga execution.
"""

from __future__ import annotations

import json
import os
import platform
import re
import shlex
import shutil
import stat
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

from native_manifest_data import DOWNLOAD_ROOT, RELEASE, encode, file_digest, release_response
from native_transport_fixture import ShellFixture, limit_files, run_process


_CORE_SHA256 = "eed3a17570ebd6fd312d808f28f724d9f3f4a10f1a2d1e42aa35a4e8f8738fb0"
"""Accepted real current core archive; a synthetic observer cannot replace it."""
_HELLO = b'''module fixture.Hello;
import { println } from std.io;
func main(): i32 {
    println("Hello, Clorraine!");
    return 0;
}
'''
"""Small independently authored program compiled by the newly installed driver."""


def _unique(pairs: list[tuple[str, object]]) -> dict:
    """Reject duplicate keys at every depth in context and candidate JSON."""
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError("duplicate context/candidate key: " + key)
        result[key] = value
    return result


def _json(path: Path) -> dict:
    """Read bounded ordinary UTF-8 JSON rather than following special metadata files."""
    if path.is_symlink() or not path.is_file() or path.stat().st_size > 16 * 1024 * 1024:
        raise ValueError("bounded ordinary JSON file required: " + str(path))
    value = json.loads(path.read_bytes().decode("utf-8"), object_pairs_hook=_unique,
                       parse_constant=_nonfinite)
    if not isinstance(value, dict):
        raise ValueError("JSON object required: " + str(path))
    return value


def _nonfinite(value: str) -> None:
    """Reject JSON extensions that are not finite JSON numbers."""
    raise ValueError("nonfinite JSON value: " + value)


def _ordinary(value: object, *, directory: bool = False) -> Path:
    """Require an absolute ordinary input path and return its canonical spelling."""
    if not isinstance(value, str):
        raise ValueError("absolute input path must be a string")
    path = Path(value)
    if not path.is_absolute() or path.is_symlink():
        raise ValueError("absolute ordinary input required: " + value)
    mode = path.stat().st_mode
    if not (stat.S_ISDIR(mode) if directory else stat.S_ISREG(mode)):
        raise ValueError("wrong input file type: " + value)
    return path.resolve(strict=True)


def _overlap(first: Path, second: Path) -> bool:
    """Detect both ancestor directions without relying on lexical path prefixes."""
    return first == second or first in second.parents or second in first.parents


def actual_context() -> tuple[dict, Path, dict[str, Path]]:
    """Validate the explicit real candidate, current cohort and system prerequisites.

    This function never creates or changes candidate inputs. Missing context and
    unsuitable environments deliberately raise errors rather than test skips.
    """
    selected = os.environ.get("CLORRAINE_INSTALL_ACCEPTANCE")
    if not selected:
        raise RuntimeError("CLORRAINE_INSTALL_ACCEPTANCE is required for actual installer proof")
    context_file = _ordinary(str(Path(selected).absolute()))
    context = _json(context_file)
    fields = {"schema", "ubuntu", "abi", "bootstrap", "bootstrap_sha256", "payloads", "work_root"}
    if set(context) != fields or type(context["schema"]) is not int or context["schema"] != 1:
        raise ValueError("actual installer context has unknown/missing fields or invalid schema")
    cohorts = {"22.04": "cp310", "24.04": "cp312"}
    ubuntu, abi = context["ubuntu"], context["abi"]
    if not isinstance(ubuntu, str) or cohorts.get(ubuntu) != abi:
        raise ValueError("actual installer context requires a supported Ubuntu/ABI pair")
    actual_os = platform.freedesktop_os_release()
    actual_abi = "cp" + str(sys.version_info.major) + str(sys.version_info.minor)
    if (actual_os.get("ID") != "ubuntu" or actual_os.get("VERSION_ID") != ubuntu or
            actual_abi != abi or platform.machine() != "x86_64" or
            sys.implementation.name != "cpython"):
        raise RuntimeError("actual Ubuntu, architecture and CPython ABI must match; no simulations")
    bootstrap = _ordinary(context["bootstrap"], directory=True)
    names = {"install.sh", "install.pyz", "manifest.json"}
    expected = context["bootstrap_sha256"]
    if not isinstance(expected, dict) or set(expected) != names:
        raise ValueError("bootstrap_sha256 must name exactly the three producer outputs")
    if {path.name for path in bootstrap.iterdir()} != names:
        raise ValueError("bootstrap directory must contain exactly the three producer outputs")
    for name in names:
        path = _ordinary(str(bootstrap / name))
        if not isinstance(expected[name], str) or re.fullmatch("[0-9a-f]{64}", expected[name]) is None:
            raise ValueError("invalid bootstrap SHA-256: " + name)
        if file_digest(path) != expected[name]:
            raise ValueError("bootstrap hash mismatch: " + name)
    manifest = _json(bootstrap / "manifest.json")
    if manifest.get("release") != RELEASE:
        raise ValueError("actual root/guidance proof requires distribution " + RELEASE)
    assets = manifest.get("assets")
    if not isinstance(assets, list) or len(assets) != 12:
        raise ValueError("actual candidate must reference exactly twelve inputs")
    identities = [item["identity"] for item in assets]
    supplied = context["payloads"]
    if not isinstance(supplied, dict) or len(set(identities)) != 12 or set(supplied) != set(identities):
        raise ValueError("payload paths must cover every unique candidate asset identity")
    paths = {}
    for asset in assets:
        identity = asset["identity"]
        path = _ordinary(supplied[identity])
        if (type(asset["bytes"]) is not int or path.stat().st_size != asset["bytes"] or
                file_digest(path) != asset["sha256"]):
            raise ValueError("actual candidate input length/hash mismatch: " + identity)
        if asset["kind"] == "core" and asset["sha256"] != _CORE_SHA256:
            raise ValueError("actual candidate must use the accepted current real core archive")
        paths[identity] = path
    work = _ordinary(context["work_root"], directory=True)
    if work.stat().st_uid != os.getuid() or os.getuid() == 0 or any(work.iterdir()):
        raise ValueError("work_root must be empty, disposable and owned by an ordinary user")
    inputs = [bootstrap, context_file, *paths.values()]
    if any(_overlap(work, path) for path in inputs):
        raise ValueError("work_root must be disjoint from every candidate/context input")
    if any(os.access(path, os.W_OK) for path in [bootstrap, *paths.values()]):
        raise RuntimeError("actual candidate inputs must be bound read-only")
    packages = manifest["prerequisites"][ubuntu]
    if not isinstance(packages, list) or not packages or any(
            not isinstance(name, str) or re.fullmatch("[a-z0-9][a-z0-9+.-]+", name) is None
            for name in packages):
        raise ValueError("invalid prerequisite package set in actual manifest")
    system_path = "/usr/bin:/bin"
    for name in ("cc", "gcc", "python3", "curl", "sha256sum", "dpkg-query"):
        if shutil.which(name, path=system_path) is None:
            raise RuntimeError("actual declared prerequisite tool unavailable: " + name)
    query = subprocess.run(
        ["/usr/bin/dpkg-query", "-W", "-f=${Package} ${db:Status-Status}\n", *packages],
        env={"PATH": system_path, "LC_ALL": "C"}, capture_output=True, text=True,
        timeout=30, check=False, preexec_fn=limit_files)
    installed = {line.split()[0] for line in query.stdout.splitlines()
                 if line.endswith(" installed")}
    if query.returncode or set(packages) - installed:
        raise RuntimeError("actual Ubuntu prerequisite packages unavailable: " +
                           repr(sorted(set(packages) - installed)))
    return manifest, work, paths


class NativeManifestActualAcceptance(unittest.TestCase):
    """Mandatory positive proof on each real supported Ubuntu and Python cohort."""

    def test_should_install_exact_revision_and_print_usable_quoted_commands(self) -> None:
        """Install real bytes, run all profiles and preserve existing shell/editor state."""
        manifest, work, payloads = actual_context()
        scratch = tempfile.TemporaryDirectory(prefix="actual-root-guidance-", dir=work)
        self.addCleanup(scratch.cleanup)
        private = Path(scratch.name)
        context_path = _ordinary(str(Path(os.environ["CLORRAINE_INSTALL_ACCEPTANCE"]).absolute()))
        bootstrap = _ordinary(_json(context_path)["bootstrap"], directory=True)
        shell = ShellFixture(private, bootstrap, manifest, payloads)
        self.assertEqual(shell.configuration["probes"], {}, "actual proof must not override identity")
        original_inputs = {identity: file_digest(path) for identity, path in payloads.items()}
        protected = {}
        for name in (".profile", ".bashrc", ".config/Code/User/settings.json",
                     ".vscode/extensions/sentinel"):
            path = shell.home / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(b"user state must not change\n")
            protected[path] = path.read_bytes()
        result = shell.invoke(["--release", RELEASE], timeout=600)
        self.assertEqual(result.returncode, 0, result.stderr)
        current = shell.install_root / "current"
        self.assertTrue(current.is_symlink())
        installation = current.resolve(strict=True)
        self.assertTrue(installation.is_relative_to(shell.install_root / "installations"))
        for name in ("clc", "clorraine", "clorraine-lsp"):
            stable = shell.install_root / "bin" / name
            real = installation / "bin" / name
            self.assertTrue(stable.is_symlink(), name)
            self.assertFalse(real.is_symlink(), "installed compiler must be a real file")
            self.assertEqual(stable.resolve(strict=True), real)
            version = run_process([str(stable), "--version"], private, shell.environment)
            self.assertEqual(version.returncode, 0, version.stderr)
            self.assertEqual(version.stdout, name + " " + manifest["core_version"] + "\n")
        retained = [path for path in installation.rglob("*.vsix")
                    if path.is_file() and not path.is_symlink()]
        self.assertEqual(len(retained), 1, "retain the downloaded VSIX inside this installation")
        vsix = next(item for item in manifest["assets"] if item["kind"] == "vsix")
        self.assertEqual(file_digest(retained[0]), vsix["sha256"])
        original_manifest = (bootstrap / "manifest.json").read_bytes()
        self.assertTrue(any(path.is_file() and not path.is_symlink() and
                            path.stat().st_size == len(original_manifest) and
                            path.read_bytes() == original_manifest
                            for path in installation.rglob("*")),
                        "installation did not retain the exact selected manifest bytes")
        source = private / "hello ; literal.clo"
        source.write_bytes(_HELLO)
        driver = shell.install_root / "bin/clorraine"
        for profile in ("host", "hosted-amiga", "hosted-amiga-13"):
            smoke = run_process([str(driver), "run", str(source), "-p", profile],
                                private, shell.environment)
            self.assertEqual(smoke.returncode, 0, profile + ": " + smoke.stderr)
            self.assertEqual(smoke.stdout.replace("\r", ""), "Hello, Clorraine!\n", profile)
        self.assertFalse([item for item in shell.events("command") if item["name"] == "code"],
                         "installer automatically installed the extension")
        for path, contents in protected.items():
            self.assertEqual(path.read_bytes(), contents)
        path_lines = [line for line in result.stdout.splitlines() if line.startswith("export PATH=")]
        code_lines = [line for line in result.stdout.splitlines()
                      if line.startswith("code --install-extension ")]
        self.assertEqual(len(path_lines), 1)
        self.assertEqual(len(code_lines), 1)
        unrelated = private / "unrelated existing bin"
        unrelated.mkdir()
        environment = dict(shell.environment, PATH=str(unrelated) + ":/usr/bin:/bin")
        command = path_lines[0] + '\ncommand -v clorraine\nprintf \'%s\\n\' "$PATH"\n'
        selected = run_process(["/bin/sh", "-c", command], private, environment)
        self.assertEqual(selected.returncode, 0, selected.stderr)
        discovered, resulting_path = selected.stdout.splitlines()
        self.assertEqual(discovered, str(driver))
        self.assertIn(str(unrelated), resulting_path.split(os.pathsep))
        self.assertEqual(resulting_path, str(shell.install_root / "bin") + ":" + environment["PATH"])
        self.assertEqual(shlex.split(code_lines[0]), ["code", "--install-extension", str(retained[0])])
        manual = run_process(["/bin/sh", "-c", code_lines[0]], private, shell.environment)
        self.assertEqual(manual.returncode, 0, manual.stderr)
        code_calls = [item for item in shell.events("command") if item["name"] == "code"]
        self.assertEqual([item["argv"] for item in code_calls],
                         [["--install-extension", str(retained[0])]])
        # A bad subsequent distribution must not change an actual working
        # installation. No handwritten private receipt is used as an old install.
        old_target = os.readlink(current)
        bad_release = "v1.2.0-r2"
        shell.route("https://api.github.com/repos/OpenLorraine/clorraine/releases/tags/" + bad_release,
                    encode(release_response(bad_release)))
        shell.route(DOWNLOAD_ROOT + bad_release + "/manifest.json", b'{"schema":2}\n')
        rejected = shell.invoke(["--release", bad_release])
        self.assertNotEqual(rejected.returncode, 0)
        self.assertTrue(shell.requested(DOWNLOAD_ROOT + bad_release + "/manifest.json"))
        self.assertEqual(os.readlink(current), old_target)
        still_working = run_process([str(driver), "run", str(source), "-p", "host"],
                                    private, shell.environment)
        self.assertEqual(still_working.returncode, 0, still_working.stderr)
        self.assertEqual(still_working.stdout.replace("\r", ""), "Hello, Clorraine!\n")
        for identity, path in payloads.items():
            self.assertEqual(file_digest(path), original_inputs[identity])
        for path, contents in protected.items():
            self.assertEqual(path.read_bytes(), contents)
