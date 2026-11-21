"""Explicit real-cohort proof of the shipped bootstrap and packaged helper closure.

Invoke ``BootstrapPackageActualAcceptance`` directly on both supported cohorts.
Missing candidate context is an error, not a local-fixture success or a skip.
"""

from __future__ import annotations

import json
import unittest

from bootstrap_package_fixtures import assert_zip_contract, copy_without_member, probe
from native_install_actual_fixture import assert_working, private_directory
from native_install_context import load_candidate
from native_manifest_data import DOWNLOAD_ROOT
from native_transport_fixture import ShellFixture


class BootstrapPackageActualAcceptance(unittest.TestCase):
    """Mandatory actual bootstrap installation with real core, support, NDK and current VSIX."""

    def test_should_install_exact_packaged_bytes_with_closed_imports_and_shared_validation(self) -> None:
        """Isolation/import controls do not replace the real supported-runtime smoke path."""
        candidate = load_candidate("CLORRAINE_INSTALL_ACCEPTANCE", "v1.2.0-r1")
        private = private_directory(self, candidate.work_root, "real-bootstrap-package-")
        shell_root = private / "shipped-shell"
        shell_root.mkdir()
        shell = ShellFixture(shell_root, candidate.bootstrap, candidate.manifest, candidate.payloads)
        self.assertEqual(shell.configuration["probes"], {})
        helper = shell.shipped / "install.pyz"
        assert_zip_contract(self, helper)
        closure = probe(self, private, helper)
        self.assertEqual(closure.returncode, 0, closure.stderr)
        self.assertFalse(json.loads(closure.stdout)["pythonpath_present"])
        validator = probe(self, private, helper, "validate", (str(shell.shipped / "manifest.json"),))
        self.assertEqual(validator.returncode, 0, validator.stderr)
        damaged = private / "missing-state-module.pyz"
        copy_without_member(helper, damaged, "tools/setup/native_state.py")
        rejected_closure = probe(self, private, damaged)
        self.assertNotEqual(rejected_closure.returncode, 0)
        # The damaged private helper is never delivered to a re-pinned shell.
        result = shell.invoke(["--release", candidate.manifest["release"]], timeout=600)
        self.assertEqual(result.returncode, 0, result.stderr)
        assert_working(self, shell, shell.install_root, candidate)
        self.assertTrue(any(line.startswith("export PATH=") for line in result.stdout.splitlines()))
        self.assertTrue(any(line.startswith("code --install-extension ")
                            for line in result.stdout.splitlines()))
        executions = shell.events("helper-execution")
        self.assertEqual(len(executions), 1)
        self.assertEqual(executions[0]["sha256"], shell.original_hashes["install.pyz"])
        responses = {item["url"]: item["sha256"] for item in shell.events("response-bytes")}
        self.assertEqual(responses[DOWNLOAD_ROOT + candidate.manifest["release"] + "/manifest.json"],
                         shell.original_hashes["manifest.json"])
        for asset in candidate.manifest["assets"]:
            if asset["url"] in responses:
                self.assertEqual(responses[asset["url"]], asset["sha256"])
        bad_root = private / "schema rejection"
        bad_root.mkdir()
        bad = ShellFixture(bad_root, candidate.bootstrap, candidate.manifest, candidate.payloads)
        url = DOWNLOAD_ROOT + candidate.manifest["release"] + "/manifest.json"
        bad.route(url, b'{"schema":2}\n')
        failure = bad.invoke(["--release", candidate.manifest["release"]], timeout=120)
        bad.assert_failed_safely(self, failure)
        self.assertTrue(bad.requested(url), "invalid metadata never reached the real helper")
        self.assertFalse(any(bad.requested(asset["url"]) for asset in candidate.manifest["assets"]),
                         "helper used payloads before validating the selected metadata")
