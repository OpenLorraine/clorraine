"""Mandatory real-cohort smoke isolation; invoke explicitly, never as a local substitute."""

from __future__ import annotations

from pathlib import Path
import shutil
import sys
import unittest

from native_install_actual_fixture import assert_working, make_shell, private_directory
from native_install_context import load_candidate
from native_transport_fixture import run_process


_SMOKE_CHILD = r'''
import json
from pathlib import Path
import sys
helper, candidate, manifest, work = sys.argv[1:]
sys.path.insert(0, helper)
from tools.setup.install_native import _smoke
_smoke(Path(candidate), json.loads(Path(manifest).read_bytes()), Path(work))
'''
"""Call the exact shipped smoke boundary; no child result, compiler or runner is replaced."""


class NativeReviewSmokeActualTests(unittest.TestCase):
    """Real candidate tools must not be rescued or displaced by an inherited SDK override.

    Invoke with ``python3 -m unittest native_review_smoke_spec.NativeReviewSmokeActualTests -v``
    in the existing isolated supported-cohort context. Missing context, packages,
    actual binaries or actual wheel environments are errors, never skips.
    """

    def setUp(self) -> None:
        """Validate immutable inputs and build an actual installation through its shipped shell."""
        self.candidate = load_candidate("CLORRAINE_INSTALL_ACCEPTANCE", "v1.2.0-r1")
        self.work = private_directory(self, self.candidate.work_root, "real-smoke-isolation-")
        self.shell = make_shell(self.work, "bootstrap", self.candidate)
        result = self.shell.invoke(["--release", self.candidate.manifest["release"]])
        self.assertEqual(result.returncode, 0, result.stderr)
        self.installation = assert_working(
            self, self.shell, self.shell.install_root, self.candidate)
        self.assertFalse(self.shell.configuration["probes"])

    def broken_candidate(self) -> Path:
        """Copy only accepted real binaries and descriptor, deliberately omitting every SDK byte."""
        root = self.work / "candidate-without-sdk"
        root.mkdir(mode=0o700)
        shutil.copytree(self.installation / "bin", root / "bin")
        shutil.copyfile(self.installation / "installation.json", root / "installation.json")
        self.assertFalse((root / "sdk").exists())
        return root

    def smoke(self, candidate: Path, override: Path | None):
        """Run real shipped host/both-vamos smoke logic with one explicit environment condition."""
        work = self.work / "smoke-work"
        work.mkdir(mode=0o700)
        environment = self.shell.environment.copy()
        environment["PATH"] = "/usr/bin:/bin"
        environment.pop("CLORRAINE_SDK", None)
        if override is not None:
            environment["CLORRAINE_SDK"] = str(override)
        return run_process(
            [sys.executable, "-I", "-B", "-c", _SMOKE_CHILD,
             str(self.candidate.bootstrap / "install.pyz"), str(candidate),
             str(self.candidate.bootstrap / "manifest.json"), str(work)],
            work, environment, timeout=180)

    def test_should_reject_sdkless_candidate_despite_valid_external_sdk_override(self) -> None:
        """All real profiles passing against another installation cannot certify this candidate."""
        broken = self.broken_candidate()
        sdk = self.installation / "sdk/amiga"
        self.assertTrue((sdk / "bin/m68k-amigaos-gcc").is_file())
        self.assertTrue((self.installation / "sdk/vamos/bin/python").exists())
        result = self.smoke(broken, sdk)
        self.assertNotEqual(result.returncode, 0,
                            "SDK-less candidate passed real smokes using external CLORRAINE_SDK")
        self.assertFalse((broken / "sdk").exists(), "test must not accidentally repair the candidate")

    def test_should_smoke_valid_candidate_despite_stale_external_sdk_override(self) -> None:
        """The candidate's own SDK must win without passing the host-inapplicable sdk-root flag."""
        stale = self.work / "nonexistent-external-sdk"
        self.assertFalse(stale.exists())
        result = self.smoke(self.installation, stale)
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_should_smoke_valid_candidate_without_override(self) -> None:
        """A valid ordinary candidate passes all three real profile smokes as a control."""
        result = self.smoke(self.installation, None)
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_should_reject_sdkless_candidate_without_override(self) -> None:
        """The broken candidate truly lacks usable adjacent SDK tools before any override."""
        broken = self.broken_candidate()
        result = self.smoke(broken, None)
        self.assertNotEqual(result.returncode, 0, "broken-candidate control unexpectedly passed")
