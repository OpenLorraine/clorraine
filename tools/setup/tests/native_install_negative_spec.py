"""Actual-context update failures; every old installation is created by the real bootstrap."""

from __future__ import annotations

import os

from native_install_context import read_json
from native_install_fixture import tree_state
from native_manifest_data import DOWNLOAD_ROOT
from native_install_actual_fixture import assert_working


class UpdateFailureCases:
    """Mixin keeping targeted negative acceptance in the mandatory actual class."""

    def test_should_preserve_old_installation_after_failed_update_download_and_retry(self) -> None:
        """A stalled selected manifest cannot activate or turn partial data into success."""
        first, second, root, old, snapshot, settings = self.baseline()
        url = DOWNLOAD_ROOT + self.update.manifest["release"] + "/manifest.json"
        second.route(url, path=self.update.bootstrap / "manifest.json", read_error="timeout")
        for attempt in range(2):
            with self.subTest(attempt=attempt):
                count = len(second.events("read-timeout"))
                result = second.invoke(["--release", self.update.manifest["release"]], root=root)
                self.assert_failed_update(second, result, root)
                self.assertTrue(second.requested(url))
                self.assertEqual(len(second.events("read-timeout")), count + 1)
                self.assert_old_unchanged(first, root, old, snapshot, settings)
        second.route(url, path=self.update.bootstrap / "manifest.json")
        repaired = second.invoke(["--release", self.update.manifest["release"]], root=root)
        self.assertEqual(repaired.returncode, 0, repaired.stderr)
        self.assertNotEqual(assert_working(self, second, root, self.update), old)
        self.assertEqual(tree_state(old), snapshot)

    def test_should_preserve_old_installation_for_every_failed_required_update_child(self) -> None:
        """Preparation, wheels and each required smoke must fail closed on both retries."""
        operations = ("preparation", "wheels", "smoke-host",
                      "smoke-hosted-amiga", "smoke-hosted-amiga-13")
        for operation in operations:
            with self.subTest(operation=operation):
                first, second, root, old, snapshot, settings = self.baseline()
                second.fault = {"operation": operation,
                                "diagnostic": "fixture failed " + operation}
                for attempt in range(2):
                    with self.subTest(attempt=attempt):
                        result = second.invoke(["--release", self.update.manifest["release"]],
                                               root=root)
                        second.assert_fault_reached(self, operation)
                        self.assert_failed_update(second, result, root)
                        words = ("ndk", "prepar") if operation == "preparation" else (
                            ("pip", "wheel", "vamos") if operation == "wheels" else ("smoke", "run", "host"))
                        self.assertTrue(any(word in result.stderr.lower() for word in words))
                        self.assert_old_unchanged(first, root, old, snapshot, settings)
                # A second failure must have reached the same operation again:
                # an incomplete first candidate cannot have been reused as complete.
                second.fault = None
                result = second.invoke(["--release", self.update.manifest["release"]], root=root)
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertNotEqual(assert_working(self, second, root, self.update), old)
                self.assertEqual(tree_state(old), snapshot)
                self.assertEqual(settings.read_bytes(), b'{"preserve":"user configuration"}\n')

    def test_should_report_real_venv_and_declared_runtime_failures_without_mutating_old_state(self) -> None:
        """Later negative stages use real cohorts and real inputs, never miniature success."""
        lock_asset = self.update.asset("support-lock")
        lock = read_json(self.update.payloads[lock_asset["identity"]])
        packages = lock["output"]["runtime_packages"][self.update.ubuntu]
        declared = sorted((soname, package) for package, record in packages.items()
                          for soname in record["sonames"])
        if not declared:
            raise RuntimeError("actual runtime-library negative requires a declared system SONAME")
        soname, provider = declared[0]
        for operation, remedy in (("venv", "python3-venv"), ("runtime", provider)):
            with self.subTest(operation=operation):
                first, second, root, old, snapshot, settings = self.baseline()
                second.fault = {"operation": operation,
                                "diagnostic": "fixture missing " + remedy}
                if operation == "runtime":
                    second.fault["soname"] = soname
                result = second.invoke(["--release", self.update.manifest["release"]], root=root)
                second.assert_fault_reached(self, operation)
                self.assert_failed_update(second, result, root)
                guidance = (result.stdout + result.stderr).lower()
                self.assertIn("apt", guidance)
                self.assertIn(remedy.lower(), guidance)
                self.assert_old_unchanged(first, root, old, snapshot, settings)
                second.fault = None
                result = second.invoke(["--release", self.update.manifest["release"]], root=root)
                self.assertEqual(result.returncode, 0, result.stderr)
                assert_working(self, second, root, self.update)
                self.assertEqual(tree_state(old), snapshot)

    def test_should_reject_managed_alias_substitution_in_a_real_owned_installation(self) -> None:
        """Recognized installer ownership does not authorize escaping path substitutions."""
        for name in ("bin", "components", "installations", "current"):
            with self.subTest(path=name):
                first, second, root, old, snapshot, settings = self.baseline()
                outside = first.root / "outside target"
                outside.mkdir()
                (outside / "canary").write_bytes(b"outside must remain unchanged\n")
                before_outside = tree_state(outside)
                managed = root / name
                backup = first.root / ("saved-" + name)
                managed.rename(backup)
                managed.symlink_to(outside, target_is_directory=True)
                selected_before = os.readlink(root / "current")
                try:
                    result = second.invoke(["--release", self.update.manifest["release"]], root=root)
                    self.assert_failed_update(second, result, root)
                    self.assertEqual(os.readlink(root / "current"), selected_before)
                    self.assertEqual(tree_state(outside), before_outside)
                finally:
                    managed.unlink()
                    backup.rename(managed)
                self.assert_old_unchanged(first, root, old, snapshot, settings)
