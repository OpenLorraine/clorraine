"""Command-line and .env parsing behavior for the SDK installer."""

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


class CliTests(unittest.TestCase):
    def setUp(self) -> None:
        self.scratch = tempfile.TemporaryDirectory()
        self.root = Path(self.scratch.name)
        support.write_root(self.root)
        support.write_media(self.root)
        self.runner = support.ready_runner(self.root)
        self.patches = [
            mock.patch.object(install_sdk, "run_command", self.runner),
            mock.patch.object(install_sdk, "which", return_value="/usr/bin/cc"),
            mock.patch.object(install_sdk, "is_interactive", return_value=False),
        ]
        for patch in self.patches:
            patch.start()

    def tearDown(self) -> None:
        for patch in reversed(self.patches):
            patch.stop()
        self.scratch.cleanup()

    def invoke(self, *extra: str) -> tuple[int, str, str]:
        out, err = StringIO(), StringIO()
        with redirect_stdout(out), redirect_stderr(err):
            code = install_sdk.main(["--root", str(self.root), *extra])
        return code, out.getvalue(), err.getvalue()

    def test_should_reject_an_unknown_flag(self) -> None:
        code, _, err = self.invoke("--nope")
        self.assertEqual(code, 2)
        self.assertIn("--nope", err)

    def test_should_not_evaluate_shell_text_in_the_env_file(self) -> None:
        marker = self.root / "evaluated"
        env = self.root / "docker" / ".env"
        env.write_text(
            f"IMAGE=clorraine-sdk:dev\nAMIGA_DATA=$(touch {marker})\n", encoding="utf-8"
        )
        code, out, err = self.invoke("--json")
        self.assertIn(code, (0, 1), err)
        self.assertFalse(marker.exists(), "the installer must not evaluate .env as shell")
        report = json.loads(out)
        self.assertIn("$(touch", report["media_root"])

    def test_should_strip_one_pair_of_quotes_from_an_env_value(self) -> None:
        env = self.root / "docker" / ".env"
        env.write_text(
            'IMAGE="clorraine-sdk:dev"\nAMIGA_DATA=\'/tmp/a b\'\n', encoding="utf-8"
        )
        values = install_sdk.read_env(env)
        self.assertEqual(values["IMAGE"], "clorraine-sdk:dev")
        self.assertEqual(values["AMIGA_DATA"], "/tmp/a b")

    def test_should_print_a_human_table_by_default(self) -> None:
        code, out, err = self.invoke()
        self.assertEqual(code, 0, err)
        self.assertIn("Clorraine SDK", out)
        self.assertIn("docker", out)
        self.assertNotIn('"components"', out)


if __name__ == "__main__":
    unittest.main()
