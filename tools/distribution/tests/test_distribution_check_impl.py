"""Makefile wiring controls; real archive acceptance remains a separate required context."""

from __future__ import annotations

import os
from pathlib import Path
import subprocess
import sys
import unittest


class DistributionCheckImplementationTests(unittest.TestCase):
    """Inspect real Make dry-runs without launching containers or emulators."""

    def _make(self, target: str) -> str:
        """Require the target to resolve through the actual project Makefile."""
        root = Path(__file__).resolve().parents[3]
        result = subprocess.run(
            ["make", "--no-print-directory", "--dry-run", target], cwd=root,
            capture_output=True, text=True, timeout=30)
        self.assertEqual(result.returncode, 0, result.stderr)
        return result.stdout

    def test_should_select_local_groups_without_claiming_real_acceptance(self) -> None:
        """Local checks name every support group and state their narrower proof boundary."""
        output = self._make("distribution-check")
        for group in (
            "test_support_spec.SupportContractTests", "support_build_spec",
            "support_preparation_spec", "support_wheels_spec", "support_ownership_spec",
            "support_wheel_selection_spec", "test_support_impl", "test_distribution_check_impl",
        ):
            self.assertIn(group, output)
        self.assertIn("Local support tests only", output)
        self.assertNotIn("SupportArtifactAcceptance", output)
        self.assertNotIn("SupportGccRelocationTests", output)

    def test_should_run_distribution_checks_from_the_fast_gate(self) -> None:
        """The normal project check reaches the new target rather than leaving it opt-in."""
        output = self._make("check")
        self.assertIn("distribution-check", output)
        self.assertIn("test_support_spec.SupportContractTests", output)
        self.assertIn("test_support_impl", output)

    def test_should_fail_both_real_groups_when_acceptance_inputs_are_missing(self) -> None:
        """Unavailable licensed/isolated proof must remain a setup error, never a green skip."""
        environment = dict(os.environ, PYTHONDONTWRITEBYTECODE="1")
        environment.pop("CLORRAINE_SUPPORT_ACCEPTANCE", None)
        result = subprocess.run(
            [sys.executable, "-m", "unittest", "-v",
             "support_artifact_spec.SupportArtifactAcceptance",
             "support_gcc_relocation_spec.SupportGccRelocationTests"],
            cwd=Path(__file__).resolve().parent, env=environment,
            capture_output=True, text=True, timeout=30)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("FAILED (errors=2)", result.stderr)
        self.assertEqual(result.stderr.count(
            "AssertionError: CLORRAINE_SUPPORT_ACCEPTANCE must name real acceptance.json inputs"), 2)
        self.assertNotIn("skipped", result.stderr)
