"""Actual lock and syscall-boundary interruption cases, without private state schemas."""

from __future__ import annotations

import os
from pathlib import Path

from native_install_actual_fixture import PROFILES, assert_working, make_shell
from native_install_fixture import tree_state


class ActivationCases:
    """Mixin exercising genuine OS-held exclusion and the real current replacement."""

    def test_should_survive_death_at_both_exact_current_boundaries_and_retry(self) -> None:
        """A recorded before/after rename signal must select only runnable complete state."""
        for boundary in ("before-current", "after-current"):
            with self.subTest(boundary=boundary):
                first, second, root, old, snapshot, settings = self.baseline()
                second.barrier = boundary
                running = second.start(["--release", self.update.manifest["release"]], root=root)
                self.addCleanup(running.close)
                running.wait_ready(boundary, timeout=600)
                records = [item for item in second.events()
                           if item.get("invocation") == running.invocation]
                boundary_index = next(index for index, item in enumerate(records)
                                      if item["kind"] == boundary)
                attempts = [item for item in records[:boundary_index]
                            if item["kind"] == "subprocess" and
                            item.get("operation", "").startswith("smoke-")]
                self.assertTrue({"smoke-" + profile for profile in PROFILES} <=
                                {item["operation"] for item in attempts},
                                "activation occurred before all candidate smoke attempts")
                for attempt in attempts:
                    self.assertTrue(attempt["candidate_sdk"], "smoke did not use an adjacent SDK")
                    candidate = Path(attempt["argv"][0]).parent.parent.resolve()
                    self.assertNotEqual(candidate, old, "update smoked the old installation")
                running.close()
                selected = (root / "current").resolve(strict=True)
                if boundary == "before-current":
                    self.assertEqual(selected, old)
                    assert_working(self, first, root, self.initial)
                else:
                    self.assertNotEqual(selected, old)
                    assert_working(self, second, root, self.update)
                self.assertEqual(tree_state(old), snapshot)
                self.assertEqual(settings.read_bytes(), b'{"preserve":"user configuration"}\n')
                second.barrier = None
                retry = second.invoke(["--release", self.update.manifest["release"]], root=root)
                self.assertEqual(retry.returncode, 0, retry.stderr)
                self.assertNotEqual(assert_working(self, second, root, self.update), old)
                self.assertEqual(tree_state(old), snapshot)

    def test_should_exclude_canonical_aliases_then_install_after_real_lock_holder_death(self) -> None:
        """Fresh-root retry succeeds despite any dead process's leftover private files."""
        first = make_shell(self.initial_work, "lock-holder", self.initial)
        second = make_shell(self.update_work, "lock-peer", self.initial)
        first.install_root.mkdir()
        core = self.initial.asset("core")
        first.http_url = core["url"]
        running = first.start(["--release", self.initial.manifest["release"]])
        self.addCleanup(running.close)
        running.wait_ready("http-ready", timeout=600)
        before = tree_state(first.install_root)
        alias = self.update_work / "canonical alias ; same root"
        alias.symlink_to(first.install_root, target_is_directory=True)
        rejected = second.invoke(["--release", self.initial.manifest["release"]],
                                 root=alias, timeout=10)
        self.assertNotEqual(rejected.returncode, 0)
        self.assertTrue(any(word in rejected.stderr.lower() for word in
                            ("lock", "another installer", "running", "busy", "concurrent")))
        self.assertIsNone(running.process.poll())
        self.assertEqual(tree_state(first.install_root), before)
        running.close()
        self.assertFalse((first.install_root / "current").exists())
        result = second.invoke(["--release", self.initial.manifest["release"]], root=alias)
        self.assertEqual(result.returncode, 0, result.stderr)
        assert_working(self, second, first.install_root, self.initial)
