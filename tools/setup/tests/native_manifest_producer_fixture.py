"""Public producer invocation and independent inputs for local fault specifications."""

from __future__ import annotations

import copy
import importlib
import os
import sys
import tempfile
import unittest
from pathlib import Path

from native_manifest_data import (
    COMPONENTS, RELEASE, SUPPORT, asset_rows, digest, encode, make_manifest, measure,
)
from native_manifest_payloads import build_inputs
from native_transport_fixture import ShellFixture, run_process


def parser_boundary(test: unittest.TestCase):
    """Require the documented parser inside each test, not during discovery."""
    original_path = sys.path[:]
    try:
        sys.path.insert(0, str(Path(__file__).resolve().parents[3]))
        module = importlib.import_module("tools.distribution.manifest")
    except ModuleNotFoundError as error:
        test.fail("Missing public distribution manifest boundary: " + str(error))
    finally:
        sys.path[:] = original_path
    test.assertTrue(callable(getattr(module, "parse_manifest", None)),
                    "missing parse_manifest(data: bytes)")
    error = getattr(module, "ManifestError", None)
    test.assertTrue(isinstance(error, type) and issubclass(error, ValueError),
                    "ManifestError must derive from ValueError")
    return module.parse_manifest, error


class ScratchCase(unittest.TestCase):
    """Allocate and clean only each method's owned child in the approved scratch root."""

    def setUp(self) -> None:
        """Register cleanup before creating any archive, process log or fixture input."""
        parent = Path(os.environ.get(
            "CODEOPS_TMPDIR", str(Path(__file__).resolve().parents[3] / "build/tmp")))
        parent.mkdir(parents=True, exist_ok=True)
        scratch = tempfile.TemporaryDirectory(prefix="native-manifest-", dir=parent)
        self.addCleanup(scratch.cleanup)
        self.root = Path(scratch.name)


class ProducerFixture:
    """Require valid production assembly before any hostile-input assertion.

    Missing production code is a deliberate test failure, not an import error
    and never a reason for an invalid-input test to pass.
    """

    def __init__(self, test: unittest.TestCase, root: Path) -> None:
        """Build miniature authenticated inputs and call only the public assembler."""
        repository = Path(__file__).resolve().parents[3]
        producer = repository / "tools/distribution/package_release.py"
        test.assertTrue(producer.is_file(), "Missing public bootstrap producer command")
        self.root = root
        inputs = root / "inputs"
        inputs.mkdir()
        self.raw, self.lock = build_inputs(inputs)
        support_inputs = inputs / "support"
        support_inputs.mkdir()
        self.paths = {}
        for identity, _, filename, _, _, _ in asset_rows():
            parent = support_inputs if filename not in ("core.tar", "clorraine.vsix", "ndk") else inputs
            path = parent / filename
            path.write_bytes(self.raw[filename])
            self.paths[identity] = path
        self.bootstrap = root / "bootstrap"
        temporary = root / "producer-temporary"
        temporary.mkdir()
        guards = root / "producer-guards"
        guards.mkdir()
        guard_log = root / "producer-forbidden-command.log"
        guard_source = (
            "#!" + sys.executable + "\nimport sys\nfrom pathlib import Path\n"
            "with Path(" + repr(str(guard_log)) + ").open('a') as output:\n"
            "    output.write(Path(sys.argv[0]).name + '\\n')\n"
            "raise SystemExit('forbidden acquisition/build process')\n")
        for name in ("curl", "wget", "git", "gh", "docker", "npm", "make", "cargo", "rustc",
                     "cc", "gcc", "clang", "sudo", "apt", "apt-get"):
            path = guards / name
            path.write_text(guard_source, encoding="utf-8")
            path.chmod(0o755)
        environment = {key: value for key, value in os.environ.items()
                       if not key.startswith(("PYTHON", "CLORRAINE", "LD_", "GIT_", "GH_", "SSH_"))}
        environment.update(TMPDIR=str(temporary), CODEOPS_TMPDIR=str(temporary),
                           PYTHONDONTWRITEBYTECODE="1", PATH=str(guards) + ":/usr/bin:/bin",
                           HOME=str(root))
        result = run_process([
            sys.executable, str(producer), "assemble", "--release", RELEASE,
            "--components", COMPONENTS, "--support", SUPPORT,
            "--core", str(inputs / "core.tar"), "--vsix", str(inputs / "clorraine.vsix"),
            "--support-inputs", str(support_inputs), "--output", str(self.bootstrap),
        ], root, environment)
        test.assertEqual(result.returncode, 0, "valid assembly failed: " + result.stderr)
        test.assertFalse(guard_log.exists(), "assembler fetched, built or changed host packages")
        test.assertEqual({path.name for path in self.bootstrap.iterdir()},
                         {"install.sh", "install.pyz", "manifest.json"})
        self.manifest = make_manifest(self.raw, self.lock)
        test.assertEqual((self.bootstrap / "manifest.json").read_bytes(), encode(self.manifest),
                         "producer disagrees with independently constructed input identities")

    def shell(self, *, probes: dict | None = None) -> ShellFixture:
        """Bind exact candidate bytes to a fresh copied shell and isolated HTTP runner."""
        private = self.root / "shell"
        private.mkdir()
        identities = {"ubuntu": "22.04", "architecture": "x86_64", "python": [3, 10, 12]}
        if probes:
            identities.update(probes)
        return ShellFixture(private, self.bootstrap, self.manifest, self.paths, probes=identities)

    def hostile_asset(self, shell: ShellFixture, kind: str, data: bytes,
                      *, measure_expansion: bool = False) -> tuple[dict, str]:
        """Create untrusted release bytes before routing, with no transport rewriting.

        The helper/shell remain the real producer outputs. The hostile publisher
        input has a matching transport hash so rejection cannot merely be a
        download-corruption result. Inventory declarations stay bounded by the
        independently measured original payload unless explicitly requested.
        """
        manifest = copy.deepcopy(self.manifest)
        asset = next(item for item in manifest["assets"] if item["kind"] == kind)
        asset.update(bytes=len(data), sha256=digest(data))
        if measure_expansion:
            asset["entries"], asset["expanded_bytes"] = measure(data, kind)
        shell.route(asset["url"], data)
        shell.route(
            "https://github.com/OpenLorraine/clorraine/releases/download/" +
            manifest["release"] + "/manifest.json", encode(manifest))
        return asset, asset["url"]


class NativeProcessCase(ScratchCase):
    """Share private storage and valid assembly without inheriting test methods."""

    def _new(self, **probes) -> tuple[ProducerFixture, ShellFixture]:
        """Require successful production assembly before one external-failure scenario."""
        root = self.root / ("scenario-" + str(len(list(self.root.iterdir()))))
        root.mkdir()
        fixture = ProducerFixture(self, root)
        return fixture, fixture.shell(probes=probes)
