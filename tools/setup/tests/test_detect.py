"""Detection and report behavior for the SDK installer."""

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


class DetectTests(unittest.TestCase):
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

    def report(self) -> dict:
        code, out, err = self.invoke("--json")
        self.assertEqual(code, 0, err)
        return json.loads(out)

    def test_should_report_every_component_ok(self) -> None:
        report = self.report()
        self.assertTrue(report["ready"])
        states = {row["name"]: row["state"] for row in report["components"]}
        self.assertEqual(
            states,
            {
                "docker": "ok",
                "compose": "ok",
                "image": "ok",
                "env": "ok",
                "toolchain": "ok",
                "ndk": "ok",
                "host-cc": "ok",
                "kickstart": "ok",
                "workbench": "ok",
            },
        )

    def test_should_report_a_missing_docker_cli(self) -> None:
        self.runner.responses = [
            (prefix, result)
            for prefix, result in self.runner.responses
            if prefix != ("docker", "version")
        ]
        code, out, err = self.invoke("--json")
        self.assertEqual(code, 1)
        report = json.loads(out)
        self.assertFalse(report["ready"])
        states = {row["name"]: row["state"] for row in report["components"]}
        self.assertEqual(states["docker"], "missing")
        self.assertIn("docker", err.lower())

    def test_should_name_the_image_build_when_the_image_is_missing(self) -> None:
        self.runner.responses = [
            (prefix, result)
            for prefix, result in self.runner.responses
            if prefix != ("docker", "image", "inspect")
        ]
        code, out, err = self.invoke("--json")
        self.assertEqual(code, 1)
        report = json.loads(out)
        states = {row["name"]: row["state"] for row in report["components"]}
        self.assertEqual(states["image"], "missing")
        self.assertTrue(any("make" in row for row in report["plan"]), report["plan"])
        self.assertFalse(
            any(call[:2] == ["docker", "run"] for call in self.runner.calls),
            "the image probe must not run without an image",
        )
        self.assertIn("image", err)

    def test_should_fail_closed_on_a_provenance_mismatch(self) -> None:
        self.runner = support.ready_runner(self.root, commit="deadbeef")
        install_sdk.run_command = self.runner
        code, out, err = self.invoke("--json")
        self.assertEqual(code, 1)
        self.assertIn("deadbeef", out)
        self.assertIn("abc123", err)
        self.assertIn("provenance", err.lower())

    def test_should_treat_missing_media_as_optional_by_default(self) -> None:
        support.write_media(self.root, roms=0, adfs=0, hdfs=0)
        code, out, _ = self.invoke("--json")
        self.assertEqual(code, 0)
        report = json.loads(out)
        states = {row["name"]: row["state"] for row in report["components"]}
        self.assertEqual(states["kickstart"], "optional")
        self.assertEqual(states["workbench"], "optional")

    def test_should_require_media_when_asked(self) -> None:
        support.write_media(self.root, roms=0, adfs=0, hdfs=0)
        code, out, err = self.invoke("--json", "--require-media")
        self.assertEqual(code, 1)
        states = {row["name"]: row["state"] for row in json.loads(out)["components"]}
        self.assertEqual(states["kickstart"], "missing")
        self.assertEqual(states["workbench"], "missing")
        self.assertIn("kickstart", err.lower())

    def test_should_discover_media_without_reading_file_contents(self) -> None:
        media = self.root / "media"
        for path in media.rglob("*"):
            if path.is_file():
                path.chmod(0o000)
        try:
            report = self.report()
        finally:
            for path in media.rglob("*"):
                if path.is_file():
                    path.chmod(0o600)
        states = {row["name"]: row["state"] for row in report["components"]}
        self.assertEqual(states["kickstart"], "ok")
        self.assertEqual(states["workbench"], "ok")
        self.assertFalse(
            any("cat" in call or "file" in call for call in self.runner.calls),
            "media discovery must inspect metadata only",
        )

    def test_should_report_components_in_a_fixed_order(self) -> None:
        report = self.report()
        self.assertEqual(
            [row["name"] for row in report["components"]],
            [
                "docker",
                "compose",
                "image",
                "env",
                "toolchain",
                "ndk",
                "host-cc",
                "kickstart",
                "workbench",
            ],
        )

    def test_should_report_a_missing_compose(self) -> None:
        self.runner.responses = [
            (prefix, result)
            for prefix, result in self.runner.responses
            if prefix != ("docker", "compose", "version")
        ]
        code, out, err = self.invoke("--json")
        self.assertEqual(code, 1)
        states = {row["name"]: row["state"] for row in json.loads(out)["components"]}
        self.assertEqual(states["compose"], "missing")
        self.assertIn("compose", err.lower())

    def test_should_plan_init_when_the_env_file_is_missing(self) -> None:
        (self.root / "docker" / ".env").unlink()
        code, out, err = self.invoke("--json")
        self.assertEqual(code, 1)
        report = json.loads(out)
        states = {row["name"]: row["state"] for row in report["components"]}
        self.assertEqual(states["env"], "missing")
        self.assertTrue(
            any("init" in command for command in report["plan"]), report["plan"]
        )

    def test_should_reject_an_unreadable_env_file(self) -> None:
        env = self.root / "docker" / ".env"
        env.write_bytes(b"\xff\xfe")
        code, out, err = self.invoke("--json")
        self.assertEqual(code, 1)
        states = {row["name"]: row["state"] for row in json.loads(out)["components"]}
        self.assertEqual(states["env"], "missing")
        self.assertNotIn("Traceback", err)

    def test_should_report_a_toolchain_without_a_version(self) -> None:
        self.runner.responses = [
            (prefix, result)
            for prefix, result in self.runner.responses
            if prefix != ("docker", "run")
        ]
        self.runner.on(
            ("docker", "run"),
            support.CommandResult(
                0,
                "tool:m68k-amigaos-gcc=ok\ntool:m68k-amigaos-gdb=ok\n"
                "tool:vc=ok\ntool:vamos=ok\nndk=ok\nprovenance=abc123\n",
            ),
        )
        code, out, err = self.invoke("--json")
        self.assertEqual(code, 1)
        states = {row["name"]: row["state"] for row in json.loads(out)["components"]}
        self.assertEqual(states["toolchain"], "missing")
        self.assertIn("version", err)

    def test_should_report_a_missing_ndk(self) -> None:
        self.runner.responses = [
            (prefix, result)
            for prefix, result in self.runner.responses
            if prefix != ("docker", "run")
        ]
        self.runner.on(
            ("docker", "run"),
            support.CommandResult(0, support.probe_output(ndk=False)),
        )
        code, out, err = self.invoke("--json")
        self.assertEqual(code, 1)
        states = {row["name"]: row["state"] for row in json.loads(out)["components"]}
        self.assertEqual(states["ndk"], "missing")
        self.assertIn("ndk", err.lower())

    def test_json_report_has_stable_keys(self) -> None:
        report = self.report()
        self.assertEqual(
            set(report),
            {"ready", "image", "media_root", "components", "actions", "plan", "downloads"},
        )
        for row in report["components"]:
            self.assertEqual(set(row), {"name", "state", "detail", "hint"})


if __name__ == "__main__":
    unittest.main()
