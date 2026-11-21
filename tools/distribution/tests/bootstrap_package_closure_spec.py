"""Isolated ZIP closure and unchanged-shell helper-integrity specifications."""

from __future__ import annotations

import json

from bootstrap_package_fixtures import BootstrapCase, copy_without_member, probe
from bootstrap_package_probe import MODULE_MEMBERS
from native_manifest_data import RELEASE, file_digest
from native_transport_fixture import ShellFixture


class BootstrapClosureTests(BootstrapCase):
    """Keep module/import controls distinct from actual supported-runtime acceptance."""

    def test_should_import_complete_helper_closure_from_exact_zip_in_isolated_python(self) -> None:
        """Local imports prove packaging closure only, never successful runtime installation."""
        helper = self.fixture.output / "install.pyz"
        before = file_digest(helper)
        result = probe(self, self.root, helper)
        self.assertEqual(result.returncode, 0, result.stderr)
        report = json.loads(result.stdout)
        self.assertEqual(report["isolated"], 1)
        self.assertEqual(report["no_site"], 1)
        self.assertFalse(report["pythonpath_present"])
        self.assertEqual(report["origins"],
                         {name: str(helper) + "/" + member for name, member in MODULE_MEMBERS.items()})
        self.assertEqual(file_digest(helper), before)

    def test_should_use_the_packaged_public_validator_for_valid_and_invalid_metadata(self) -> None:
        """The actual ZIP's parser validates without loading a checkout implementation."""
        result = probe(self, self.root, self.fixture.output / "install.pyz", "validate",
                       (str(self.fixture.output / "manifest.json"),))
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_should_fail_direct_closure_when_any_required_module_is_removed(self) -> None:
        """Private damaged copies cannot fall back to checkout, CWD or PYTHONPATH packages."""
        helper = self.fixture.output / "install.pyz"
        before = file_digest(helper)
        valid = probe(self, self.root, helper, poisoned=True)
        self.assertEqual(valid.returncode, 0, valid.stderr)
        for name, member in MODULE_MEMBERS.items():
            if name in ("tools", "tools.setup", "tools.distribution"):
                continue
            with self.subTest(module=name):
                damaged = self.root / (name.replace(".", "-") + ".pyz")
                copy_without_member(helper, damaged, member)
                result = probe(self, self.root, damaged, poisoned=True)
                self.assertNotEqual(result.returncode, 0, "missing module did not fail closure proof")
                self.assertTrue(result.stderr.strip())
                self.assertEqual(file_digest(helper), before, "original helper changed")

    def test_should_forward_main_status_and_require_the_real_zip_entry(self) -> None:
        """No richer help/test CLI or fabricated entry script is used for status proof."""
        helper = self.fixture.output / "install.pyz"
        valid = probe(self, self.root, helper)
        self.assertEqual(valid.returncode, 0, valid.stderr)
        arguments = ("--not-a-bootstrap-option",)
        direct = probe(self, self.root, helper, "api-status", arguments)
        entry = probe(self, self.root, helper, "entry-status", arguments)
        self.assertNotEqual(direct.returncode, 0)
        self.assertEqual(entry.returncode, direct.returncode)
        damaged = self.root / "no-entry.pyz"
        copy_without_member(helper, damaged, "__main__.py")
        missing = probe(self, self.root, damaged, "entry-status", arguments)
        self.assertNotEqual(missing.returncode, 0)
        self.assertIn("__main__", missing.stderr)

    def test_should_reject_corrupt_or_short_helper_delivery_before_any_python_execution(self) -> None:
        """The real unchanged shipped shell enforces its own pinned bytes, size and digest."""
        for fault in ("corrupt", "truncate"):
            with self.subTest(fault=fault):
                private = self.root / ("shell-" + fault)
                private.mkdir()
                shell = ShellFixture(private, self.fixture.output, self.fixture.source.manifest,
                                     self.fixture.source.paths)
                shell.configuration["curl_fault"] = fault
                result = shell.invoke(["--release", RELEASE])
                shell.assert_failed_safely(self, result)
                self.assertEqual(len(shell.events("helper-delivery")), 1)
                self.assertFalse(shell.events("helper-execution"))
                self.assertFalse(shell.events("runner"))
                self.assertFalse([item for item in shell.events("command") if item["name"] == "python3"])
                self.assertEqual(shell.events("request"), [])
