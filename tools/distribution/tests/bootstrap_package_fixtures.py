"""Small public-command and exact-ZIP fixtures for bootstrap packaging specifications."""

from __future__ import annotations

import copy
import os
import shutil
import stat
import sys
import zipfile
from pathlib import Path

from bootstrap_package_probe import EMPTY_INITIALIZERS, FORBIDDEN_MARKER, REQUIRED_MEMBERS


SETUP_TESTS = Path(__file__).resolve().parents[2] / "setup/tests"
"""Parallel test-fixture directory, never an application import path for a child."""
sys.path.insert(0, str(SETUP_TESTS))

from native_manifest_data import COMPONENTS, RELEASE, SUPPORT, file_digest  # noqa: E402
from native_manifest_producer_fixture import ProducerFixture, ScratchCase  # noqa: E402
from native_transport_fixture import run_process  # noqa: E402


class BootstrapFixture:
    """Require valid production assembly before testing any malformed input or ZIP.

    ``source`` exposes the existing independent input fixture. ``output`` and
    ``temporary`` belong to this test alone. Commands always use the documented
    producer entry and literal argv, never an implementation Python function.
    """

    def __init__(self, test, root: Path) -> None:
        """Build the already verified miniature inputs through their frozen public fixture."""
        self.root = root
        inputs = root / "baseline"
        inputs.mkdir()
        self.source = ProducerFixture(test, inputs)
        self.output = self.source.bootstrap
        self.producer = Path(__file__).resolve().parents[3] / "tools/distribution/package_release.py"
        self.temporary = root / "packager temporary"
        self.cwd = root / "packager cwd"
        self.outputs = root / "fresh outputs"
        self.guards = root / "packager guards"
        self.guard_log = root / "forbidden-command.log"
        for directory in (self.temporary, self.cwd, self.outputs, self.guards):
            directory.mkdir()
        source = (
            "#!" + sys.executable + "\nimport sys\nfrom pathlib import Path\n"
            "with Path(" + repr(str(self.guard_log)) + ").open('a') as output:\n"
            "    output.write(Path(sys.argv[0]).name + '\\n')\n"
            "raise SystemExit('unexpected build/acquisition/install/publication command')\n")
        for name in ("cc", "gcc", "clang", "cargo", "rustc", "npm", "npx", "make", "cmake",
                     "curl", "wget", "git", "gh", "docker", "sudo", "apt", "apt-get",
                     "clc", "clorraine", "clorraine-lsp", "code"):
            path = self.guards / name
            path.write_text(source, encoding="utf-8")
            path.chmod(0o755)
        self.environment = {key: value for key, value in os.environ.items()
                            if not key.startswith(("PYTHON", "CLORRAINE", "LD_", "GIT_",
                                                   "GH_", "GITHUB_", "SSH_"))}
        home = root / "packager home"
        home.mkdir()
        self.environment.update(PATH=str(self.guards) + ":/usr/bin:/bin", HOME=str(home),
                                TMPDIR=str(self.temporary), CODEOPS_TMPDIR=str(self.temporary),
                                PYTHONDONTWRITEBYTECODE="1")

    def arguments(self, output: Path, **overrides: object) -> list[str]:
        """Construct only the approved producer options from public input bindings."""
        values = {
            "release": RELEASE, "components": COMPONENTS, "support": SUPPORT,
            "core": self.source.paths[COMPONENTS + "-core"],
            "vsix": self.source.paths[COMPONENTS + "-vsix"],
            "support-inputs": self.source.paths[SUPPORT + "-lock"].parent, "output": output,
        }
        values.update(overrides)
        result = ["assemble"]
        for name, value in values.items():
            result.extend(["--" + name, str(value)])
        return result

    def invoke(self, test, arguments: list[str]):
        """Run the public command, requiring untouched caller bytes and no hidden side effects."""
        before = input_snapshot(self.source.paths)
        result = run_process([sys.executable, str(self.producer), *arguments],
                             self.cwd, self.environment)
        test.assertEqual(input_snapshot(self.source.paths), before, "producer changed caller inputs")
        test.assertFalse(self.guard_log.exists(), "producer built, fetched, installed or published")
        test.assertEqual(list(self.temporary.iterdir()), [], "producer abandoned private work")
        return result


class BootstrapCase(ScratchCase):
    """Give each packaging/closure method its own valid production assembly control."""

    def setUp(self) -> None:
        """Missing producer code fails here rather than passing an invalid-input assertion."""
        super().setUp()
        self.fixture = BootstrapFixture(self, self.root)


def input_snapshot(paths: dict[str, Path]) -> dict[str, tuple]:
    """Record the exact selected input bytes/types/modes without following aliases."""
    result = {}
    for name, path in paths.items():
        if path.is_symlink():
            result[name] = ("symlink", os.readlink(path))
        elif path.is_file():
            result[name] = ("file", path.stat().st_mode & 0o7777, file_digest(path))
        elif path.exists():
            result[name] = ("other", path.lstat().st_mode)
        else:
            result[name] = ("missing",)
    return result


def output_bytes(directory: Path) -> dict[str, bytes]:
    """Capture the exact three entry assets, without interpreting installer source."""
    return {name: (directory / name).read_bytes()
            for name in ("install.sh", "install.pyz", "manifest.json")}


def assert_zip_contract(test, archive: Path) -> None:
    """Check required members and deterministic safe ZIP metadata, allowing runtime data."""
    with zipfile.ZipFile(archive) as helper:
        entries = helper.infolist()
        names = [item.filename for item in entries]
        test.assertEqual(names, sorted(names), "helper paths must be sorted")
        test.assertEqual(len(names), len(set(names)), "duplicate helper destinations")
        test.assertTrue(REQUIRED_MEMBERS <= set(names), "required package/module member missing")
        for item in entries:
            parts = item.filename.rstrip("/").split("/")
            test.assertFalse(item.filename.startswith(("/", "\\")))
            test.assertFalse("\\" in item.filename)
            test.assertTrue(all(part not in ("", ".", "..") for part in parts))
            test.assertFalse(any(ord(character) < 32 or ord(character) == 127
                                 for character in item.filename))
            test.assertEqual(item.date_time, (1980, 1, 1, 0, 0, 0))
            mode = item.external_attr >> 16
            if item.is_dir():
                test.assertIn(stat.S_IFMT(mode), (0, stat.S_IFDIR))
                test.assertEqual(item.file_size, 0)
            else:
                test.assertIn(stat.S_IFMT(mode), (0, stat.S_IFREG))
            test.assertEqual(mode & 0o7022, 0, "unsafe helper file mode")
            test.assertTrue(mode & stat.S_IRUSR, "helper data must be readable")
        for name in EMPTY_INITIALIZERS:
            test.assertEqual(helper.read(name), b"", "package initializers must be empty")


def copy_without_member(source: Path, destination: Path, omitted: str) -> None:
    """Remove one member from a private opaque ZIP copy without re-pinning the shell."""
    with zipfile.ZipFile(source) as original, zipfile.ZipFile(destination, "w") as changed:
        if omitted not in original.namelist():
            raise AssertionError("negative control member was not present: " + omitted)
        changed.comment = original.comment
        for item in original.infolist():
            if item.filename != omitted:
                changed.writestr(copy.copy(item), original.read(item.filename))


def probe(test, root: Path, archive: Path, mode: str = "closure",
          arguments: tuple[str, ...] = (), *, poisoned: bool = False):
    """Execute a copied standalone probe with only exact-ZIP/stdlib imports in Python -I."""
    private = root / ("probe-" + str(len(list(root.glob("probe-*")))))
    private.mkdir()
    script = private / "probe.py"
    shutil.copyfile(Path(__file__).with_name("bootstrap_package_probe.py"), script)
    home = private / "home"
    home.mkdir()
    environment = {key: value for key, value in os.environ.items()
                   if not key.startswith(("PYTHON", "CLORRAINE", "LD_", "GIT_", "GH_", "SSH_"))}
    environment.update(HOME=str(home), PATH="/usr/bin:/bin", TMPDIR=str(private),
                       CODEOPS_TMPDIR=str(private), PYTHONDONTWRITEBYTECODE="1")
    if poisoned:
        # Both locations are untrusted decoys. No test directory or source
        # checkout is added to the child; -I must ignore these possible fallbacks.
        poison = private / "poison"
        for parent in (private, poison):
            for package in ("tools", "tools/setup", "tools/distribution"):
                directory = parent / package
                directory.mkdir(parents=True, exist_ok=True)
                (directory / "__init__.py").write_text(
                    'raise RuntimeError("BOOTSTRAP_SOURCE_DECOY_IMPORTED")\n', encoding="utf-8")
        environment["PYTHONPATH"] = str(poison)
    result = run_process([sys.executable, "-I", "-S", str(script), mode, str(archive), *arguments],
                         private, environment)
    test.assertNotIn("BOOTSTRAP_SOURCE_DECOY_IMPORTED", result.stderr)
    test.assertNotIn(FORBIDDEN_MARKER, result.stderr, "probe attempted network/process execution")
    return result
