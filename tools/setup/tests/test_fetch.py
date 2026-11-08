"""Fetch planning, confirmation, and verification for the SDK installer."""

from __future__ import annotations

import json
import tempfile
import unittest
from contextlib import redirect_stderr, redirect_stdout
from io import StringIO
from pathlib import Path
from unittest import mock

import support

import install_sdk


class FetchTests(unittest.TestCase):
    def setUp(self) -> None:
        self.scratch = tempfile.TemporaryDirectory()
        self.root = Path(self.scratch.name)
        support.write_root(self.root)
        support.write_media(self.root)
        self.image_present = False
        self.runner = self.build_runner()
        self.interactive = False
        self.patches = [
            mock.patch.object(install_sdk, "run_command", self.runner),
            mock.patch.object(install_sdk, "which", return_value="/usr/bin/cc"),
            mock.patch.object(
                install_sdk, "is_interactive", side_effect=lambda: self.interactive
            ),
        ]
        for patch in self.patches:
            patch.start()

    def tearDown(self) -> None:
        for patch in reversed(self.patches):
            patch.stop()
        self.scratch.cleanup()

    def build_runner(self) -> support.FakeRunner:
        runner = support.FakeRunner()
        runner.on(("docker", "version"), support.CommandResult(0, "27.5.1\n"))
        runner.on(("docker", "compose", "version"), support.CommandResult(0, "2.30.0\n"))
        runner.on(("make",), support.CommandResult(0, "built\n"))
        runner.hook(
            ("docker", "image", "inspect"),
            lambda argv: support.CommandResult(0, "sha256:abc\n")
            if self.image_present
            else support.CommandResult(1, "", "no image"),
        )
        runner.hook(
            ("make",),
            lambda argv: self.mark_image()
            if "image" in argv
            else support.CommandResult(0, "ok\n"),
        )
        runner.on(("docker", "run"), support.CommandResult(0, support.probe_output()))
        return runner

    def mark_image(self) -> support.CommandResult:
        self.image_present = True
        return support.CommandResult(0, "built\n")

    def invoke(self, *extra: str) -> tuple[int, str, str]:
        out, err = StringIO(), StringIO()
        with redirect_stdout(out), redirect_stderr(err):
            code = install_sdk.main(
                [
                    "--root",
                    str(self.root),
                    "--amiga-data",
                    str(self.root / "media"),
                    *extra,
                ]
            )
        return code, out.getvalue(), err.getvalue()

    def test_should_build_the_image_and_reprobe(self) -> None:
        code, out, err = self.invoke("--fetch", "--yes")
        self.assertEqual(code, 0, err)
        make_calls = [call for call in self.runner.calls if call[:1] == ["make"]]
        self.assertTrue(make_calls, self.runner.calls)
        self.assertIn("image", make_calls[0])
        self.assertTrue(self.image_present)
        self.assertIn("image", out.lower())
        inspects = [
            call for call in self.runner.calls if call[:3] == ["docker", "image", "inspect"]
        ]
        self.assertGreaterEqual(len(inspects), 2, "the fetch must re-probe the image")

    def test_should_refuse_a_non_interactive_fetch_without_yes(self) -> None:
        code, out, err = self.invoke("--fetch")
        self.assertEqual(code, 2)
        self.assertIn("--yes", err)
        self.assertIn("--dry-run", err)
        self.assertFalse(any(call[:1] == ["make"] for call in self.runner.calls))

    def test_dry_run_makes_no_changes(self) -> None:
        code, out, err = self.invoke("--fetch", "--dry-run")
        self.assertEqual(code, 0, err)
        self.assertFalse(any(call[:1] == ["make"] for call in self.runner.calls))
        self.assertIn("make", out)
        self.assertFalse(self.image_present)

    def test_should_report_a_failed_build(self) -> None:
        self.runner.hooks = []
        self.runner.responses = [
            (prefix, result)
            for prefix, result in self.runner.responses
            if prefix != ("make",)
        ]
        self.runner.on(("make",), support.CommandResult(7, "", "compiler failed\n"))
        code, out, err = self.invoke("--fetch", "--yes")
        self.assertEqual(code, 1)
        self.assertIn("image", err.lower())
        self.assertIn("make", err)
        self.assertIn("compiler failed", err)

    def test_should_verify_provenance_after_a_fetch(self) -> None:
        self.runner.responses = [
            (prefix, result)
            for prefix, result in self.runner.responses
            if prefix != ("docker", "run")
        ]
        self.runner.on(
            ("docker", "run"),
            support.CommandResult(0, support.probe_output(commit="deadbeef")),
        )
        code, out, err = self.invoke("--fetch", "--yes")
        self.assertEqual(code, 1)
        self.assertIn("provenance", err.lower())
        self.assertIn("deadbeef", err)
        self.assertIn("abc123", err)
        self.assertIn("recovery", err.lower())

    def test_should_report_provenance_when_no_plan_can_fix_it(self) -> None:
        self.image_present = True
        self.runner.responses = [
            (prefix, result)
            for prefix, result in self.runner.responses
            if prefix != ("docker", "run")
        ]
        self.runner.on(
            ("docker", "run"),
            support.CommandResult(0, support.probe_output(commit="deadbeef")),
        )
        code, out, err = self.invoke("--fetch", "--yes")
        self.assertEqual(code, 1)
        self.assertIn("provenance", err.lower())
        self.assertIn("recovery", err.lower())

    def test_should_emit_one_json_object_for_a_fetch(self) -> None:
        code, out, err = self.invoke("--fetch", "--yes", "--json")
        self.assertEqual(code, 0, err)
        report = json.loads(out)
        self.assertTrue(report["ready"])

    def test_should_emit_one_json_object_for_a_dry_run(self) -> None:
        code, out, err = self.invoke("--fetch", "--dry-run", "--json")
        self.assertEqual(code, 0, err)
        report = json.loads(out)
        self.assertTrue(report["plan"])
        self.assertIn("Dry run", err)

    def test_should_confirm_on_a_terminal(self) -> None:
        self.interactive = True
        with mock.patch("builtins.input", return_value="y"):
            code, out, err = self.invoke("--fetch")
        self.assertEqual(code, 0, err)
        self.assertIn("Proceed?", err)


if __name__ == "__main__":
    unittest.main()
