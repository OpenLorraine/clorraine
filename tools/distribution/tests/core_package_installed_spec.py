"""Mandatory checkout-free observations of the actual assembled core candidate."""

from __future__ import annotations

import os
import re
import shutil
import unittest
from pathlib import Path

from core_package_artifacts import ActualInputs, assembled_layout, required_environment
from core_package_fixtures import BINARIES, digest_file, inspect_core


HELLO = "Hello, Clorraine!\n"
"""Known visible program output, independent of a tool's version banner."""


class CoreInstalledAcceptance(unittest.TestCase):
    """Require real core bytes and a movable, unactivated actual staged candidate.

    CLORRAINE_CORE_ARCHIVE and CLORRAINE_CORE_ARCHIVE_SHA256 identify the actual
    archive verified by the producer group. CLORRAINE_CORE_INSTALLATION names
    its disposable stage output. The relocation test moves this directory and
    restores it; it must be user-owned, separate from work_root, unactivated,
    and on work_root's filesystem.
    CLORRAINE_SUPPORT_ACCEPTANCE remains the existing actual-support record.
    Run only in the isolated runtime image with the original checkout absent.
    """

    @classmethod
    def setUpClass(cls) -> None:
        """Refuse missing evidence, unsupported cohorts and replacement core bytes."""
        cls.inputs = ActualInputs(runtime=True)
        cls.addClassCleanup(cls.inputs.cleanup)
        cls.archive = required_environment("CLORRAINE_CORE_ARCHIVE")
        identity = os.environ.get("CLORRAINE_CORE_ARCHIVE_SHA256", "")
        if re.fullmatch(r"[0-9a-f]{64}", identity) is None or digest_file(cls.archive) != identity:
            raise AssertionError("CLORRAINE_CORE_ARCHIVE_SHA256 must pin the verified actual core bytes")
        cls.core = inspect_core(cls.archive)
        cls.installation = required_environment("CLORRAINE_CORE_INSTALLATION")
        if (cls.installation.is_symlink() or not cls.installation.is_dir()
                or cls.installation.stat().st_uid != os.getuid()
                or cls.installation.stat().st_dev != cls.inputs.root.stat().st_dev):
            raise AssertionError("actual staged candidate must be owned, movable and on work_root's filesystem")
        original = cls.installation.resolve()
        work = cls.inputs.root.resolve()
        if original.is_relative_to(work) or work.is_relative_to(original):
            raise AssertionError("movable candidate and proof work must be disjoint")
        for parent in original.parents:
            current = parent / "current"
            if current.is_symlink() and current.resolve() == original:
                raise AssertionError("refusing to relocate an activated installation")
        for path in (cls.installation, cls.archive):
            for parent in (path, *path.parents):
                if (parent / ".git").exists() or (parent / "compiler/Cargo.toml").exists():
                    raise AssertionError("actual runtime candidate/archive cannot live in a checkout")
        for name in ("m68k-amigaos-gcc", "vamos"):
            if shutil.which(name, path=cls.inputs.environment["PATH"]) is not None:
                raise AssertionError(f"runtime proof must not have an alternate PATH {name}")
        assembled_layout(cls.installation, cls.archive, cls.inputs)

    def _versions(self, installation: Path) -> None:
        """Execute all three packaged binaries, never development/PATH replacements."""
        for name in BINARIES:
            result = self.inputs.run([installation / "bin" / name, "--version"])
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertIn(name, result.stdout)
            self.assertIn(self.core["core_version"], result.stdout)

    def _programs(self, driver: Path, cwd: Path) -> None:
        """Observe real host and both Amiga-profile output/status/capture vectors."""
        cwd.mkdir(parents=True, exist_ok=True)
        source = cwd / "hello.clo"
        for status in (0, 7):
            source.write_text(
                "module hello;\nimport { println } from std.io;\n"
                "func main(): i32 {\n"
                f'    println("Hello, Clorraine!");\n    return {status};\n'
                "}\n", encoding="utf-8")
            for profile in ("host", "hosted-amiga", "hosted-amiga-13"):
                for capture in (False, True):
                    with self.subTest(status=status, profile=profile, capture=capture):
                        args = [driver, "run", "--profile", profile]
                        if capture:
                            args.append("--capture")
                        args.append(source)
                        result = self.inputs.run(args, cwd)
                        self.assertEqual(result.returncode, status, result.stderr)
                        if capture:
                            relative = f"build/clorraine/hello/{profile}/out.txt"
                            self.assertEqual((cwd / relative).read_text().replace("\r", ""), HELLO)
                            self.assertIn(relative, result.stdout)
                            self.assertNotIn(HELLO.strip(), result.stdout)
                        else:
                            self.assertEqual(result.stdout.replace("\r", ""), HELLO)

    def test_should_expose_actual_canonical_adjacent_tools_without_checkout(self) -> None:
        """Real versions and real program output require more than layout-shaped files."""
        self._versions(self.installation)
        self._programs(self.installation / "bin/clorraine", self.inputs.root / "unrelated workspace")
        assembled_layout(self.installation, self.archive, self.inputs)
        self.inputs.unchanged()

    def test_should_run_actual_archive_through_relocated_stable_links(self) -> None:
        """Move the whole root with its real driver, SDK and venv, then restore input.

        Moving rather than copying keeps every previous candidate prefix
        unavailable during observation. Only the explicitly disposable input
        is moved; it is restored even when a program assertion fails.
        """
        root = self.inputs.root / "stable root"
        (root / "installations").mkdir(parents=True)
        (root / "bin").mkdir()
        live = root / "installations/a"
        self.installation.rename(live)
        try:
            (root / "current").symlink_to("installations/a")
            (root / "bin/clorraine").symlink_to("../current/bin/clorraine")
            self.assertFalse(self.installation.exists(), "original stage prefix is unavailable")
            self._versions(live)
            self._programs(root / "bin/clorraine", self.inputs.root / "first unrelated workspace")
            moved = self.inputs.root / "relocated root ; literal"
            root.rename(moved)
            live = moved / "installations/a"
            self.assertFalse(root.exists(), "first stable root is unavailable")
            self.assertFalse(self.installation.exists(), "original stage prefix remains unavailable")
            self._versions(live)
            self._programs(moved / "bin/clorraine", self.inputs.root / "second unrelated workspace")
            assembled_layout(live, self.archive, self.inputs)
            self.inputs.unchanged()
        finally:
            try:
                if self.installation.exists():
                    raise AssertionError("refusing to overwrite a newly created original candidate path")
                live.rename(self.installation)
            except BaseException:
                self.inputs.keep_reason = f"candidate restoration failed; user input remains at {live}"
                raise
