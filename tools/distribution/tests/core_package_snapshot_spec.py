"""Executable snapshot integrity through the public local archive command.

Real ELF tools and file barriers change caller bytes at version-query boundaries.
No producer function, private filename or internal process hook is used.
"""

from __future__ import annotations

import json
import os
import shutil
import signal
import subprocess
import sys
import tarfile
import tempfile
import time
import unittest
from pathlib import Path

from core_package_fixtures import (
    CoreFixture, digest_file, elf_x64, inspect_core, inventory, repository, scratch_parent,
)


_BARRIER_SOURCE = r'''
/* Ordinary ELF version tools expose only their own public process boundary. */
#define _POSIX_C_SOURCE 200809L
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <unistd.h>

/** Wait for a bounded test-side file release, never for producer internals. */
static int barrier(void) {
    const char *root = getenv("SNAPSHOT_BARRIERS");
    char ready[4096], pending[4096], release[4096];
    struct timespec pause = {0, 10000000};
    if (root == NULL || BARRIER[0] == '\0') return 0;
    if (snprintf(ready, sizeof(ready), "%s/%s.ready", root, BARRIER) >= (int)sizeof(ready)
        || snprintf(pending, sizeof(pending), "%s/%s.writing", root, BARRIER)
           >= (int)sizeof(pending)
        || snprintf(release, sizeof(release), "%s/%s.release", root, BARRIER)
           >= (int)sizeof(release)) return 82;
    FILE *stream = fopen(pending, "wb");
    if (stream == NULL) return 83;
    fputs("ready\n", stream);
    if (fclose(stream) != 0) return 84;
    if (rename(pending, ready) != 0) return 84;
    for (int attempt = 0; attempt < 800; ++attempt) {
        if (access(release, F_OK) == 0) return 0;
        nanosleep(&pause, NULL);
    }
    return 85;
}

/** The printed version is compiled into these exact executable bytes. */
int main(int argc, char **argv) {
    if (argc != 2 || strcmp(argv[1], "--version") != 0) return 81;
    int status = barrier();
    if (status != 0) return status;
    printf("%s %s\n", TOOL_NAME, TOOL_VERSION);
    return 0;
}
'''
"""C99 fixture data; version changes require different actual executable bytes."""


class CoreSnapshotTests(unittest.TestCase):
    """Inventory, queried versions and archived executables describe one snapshot."""

    def setUp(self) -> None:
        """Own all fixtures before compiling; no producer-side compilation is allowed."""
        scratch = tempfile.TemporaryDirectory(prefix="core-snapshot-", dir=scratch_parent())
        self.addCleanup(scratch.cleanup)
        self.fixture = CoreFixture(Path(scratch.name))
        self.barriers = self.fixture.root / "barriers"
        self.barriers.mkdir()
        self.fixture.environment["SNAPSHOT_BARRIERS"] = str(self.barriers)
        self.source = self.fixture.root / "snapshot-version.c"
        self.source.write_text(_BARRIER_SOURCE, encoding="utf-8")
        self.compiler = shutil.which("cc")
        self.assertIsNotNone(self.compiler, "existing cc is required for ELF test data")
        self._compile(self.fixture.binaries / "clc", "clc", self.fixture.version, "clc")
        self.wrong = self.fixture.root / "wrong-clorraine"
        self.valid = self.fixture.root / "valid-clorraine"
        self._compile(self.wrong, "clorraine", "9.9.9", "")
        self._compile(self.valid, "clorraine", self.fixture.version, "clorraine")
        self.assertNotEqual(self.wrong.read_bytes(), self.valid.read_bytes())

    def _compile(self, output: Path, name: str, version: str, barrier: str) -> None:
        """Compile fixed banners before invoking the guarded packaging environment."""
        subprocess.run([
            self.compiler, "-std=c99", "-Wall", "-Wextra", "-Werror",
            f"-DTOOL_NAME={json.dumps(name)}", f"-DTOOL_VERSION={json.dumps(version)}",
            f"-DBARRIER={json.dumps(barrier)}", "-o", str(output), str(self.source),
        ], check=True, capture_output=True, text=True, timeout=30)
        elf_x64(output)

    def _replace(self, original: Path, source: Path) -> None:
        """Atomically change a path without rewriting a currently running ELF inode."""
        replacement = original.with_name(original.name + ".replacement")
        shutil.copy2(source, replacement)
        replacement.replace(original)
        self.assertEqual(original.read_bytes(), source.read_bytes())

    def _release(self, name: str) -> None:
        """Allow one actual version child to finish after the caller mutation."""
        (self.barriers / (name + ".release")).write_bytes(b"continue\n")

    def _ready_or_exited(self, child: subprocess.Popen, name: str) -> bool:
        """Wait for an observable barrier or valid early rejection, with a deadline."""
        ready = self.barriers / (name + ".ready")
        deadline = time.monotonic() + 5
        while time.monotonic() < deadline:
            if ready.exists():
                self.assertEqual(ready.read_bytes(), b"ready\n")
                return True
            if child.poll() is not None:
                return False
            time.sleep(0.01)
        self.fail(f"version child did not reach {name} barrier or exit")

    def _join(self, child: subprocess.Popen) -> tuple[str, str]:
        """Release and reap every owned child, including after an assertion failure."""
        for name in ("clc", "clorraine"):
            self._release(name)
        try:
            return child.communicate(timeout=10)
        except subprocess.TimeoutExpired:
            os.killpg(child.pid, signal.SIGKILL)
            child.communicate(timeout=5)
            raise

    def _start(self, command: list[str]) -> subprocess.Popen:
        """Use a private process group so failed controls cannot abandon descendants."""
        return subprocess.Popen(
            command, cwd=self.fixture.root, env=self.fixture.environment,
            stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
            text=True, start_new_session=True)

    def _producer(self) -> subprocess.Popen:
        """Run only the documented archive interface, without importing its module."""
        return self._start([
            sys.executable, str(repository() / "tools/distribution/package_core.py"),
            "archive", "--binaries", str(self.fixture.binaries),
            "--output", str(self.fixture.output),
        ])

    def test_should_reject_mutating_original_that_hides_wrong_snapshotted_version(self) -> None:
        """A temporarily valid original must not authenticate archived wrong-version bytes."""
        fixture = self.fixture
        original = fixture.binaries / "clorraine"
        self._replace(original, self.wrong)
        before = fixture.binary_state()
        expected = inventory(fixture.binaries, repository())
        child = self._producer()
        boundaries = []
        try:
            if self._ready_or_exited(child, "clc"):
                boundaries.append("clc")
                self._replace(original, self.valid)
                self._release("clc")
                if self._ready_or_exited(child, "clorraine"):
                    boundaries.append("clorraine")
                    self._replace(original, self.wrong)
                    self._release("clorraine")
            stdout, stderr = child.communicate(timeout=10)
        finally:
            self._join(child)
            self._replace(original, self.wrong)

        self.assertEqual(fixture.binary_state(), before, "caller inputs were not restored exactly")
        fixture.expected = expected
        fixture.assert_no_side_effects(self)
        contradiction = ""
        if fixture.output.exists():
            core = inspect_core(fixture.output, expected, fixture.version)
            with tarfile.open(fixture.output, "r:") as archive:
                extracted = fixture.root / "extracted-clorraine"
                extracted.write_bytes(archive.extractfile("bin/clorraine").read())
            extracted.chmod(0o755)
            self.assertEqual(extracted.read_bytes(), self.wrong.read_bytes())
            version = subprocess.run([str(extracted), "--version"], capture_output=True,
                                     text=True, timeout=5, check=True).stdout.strip()
            contradiction = f"; core.json={core['core_version']}, extracted={version}"
            self.assertEqual(boundaries, ["clc", "clorraine"],
                             "the baseline exploit must cross both real query barriers")
            self.assertEqual(version, "clorraine 9.9.9")
        self.assertEqual(child.returncode, 1,
                         f"wrong snapshot accepted after barriers {boundaries}: "
                         f"{stdout!r} {stderr!r}{contradiction}")
        self.assertIn("clorraine", stderr.lower(), stderr)
        self.assertFalse(fixture.output.exists(), "failed version validation published output")
        self.assertEqual(list(fixture.published.iterdir()), [])

    def test_should_archive_stable_snapshot_deterministically_without_changing_inputs(self) -> None:
        """Stable current-version ELF bytes retain exact contents and deterministic output."""
        fixture = self.fixture
        self._replace(fixture.binaries / "clorraine", self.valid)
        for name in ("clc", "clorraine"):
            self._release(name)
        fixture.expected = inventory(fixture.binaries, repository())
        before = fixture.binary_state()
        first = fixture.archive(self)
        self.assertEqual(first.returncode, 0, first.stderr)
        inspect_core(fixture.output, fixture.expected, fixture.version)
        second_output = fixture.published / "second.tar"
        second = fixture.archive(self, second_output)
        self.assertEqual(second.returncode, 0, second.stderr)
        self.assertEqual(digest_file(fixture.output), digest_file(second_output))
        self.assertEqual(fixture.binary_state(), before)
        fixture.assert_no_side_effects(self)
        existing = fixture.output.read_bytes()
        refused = fixture.archive(self)
        self.assertEqual(refused.returncode, 1, refused.stderr)
        self.assertEqual(fixture.output.read_bytes(), existing, "existing output was clobbered")
        self.assertEqual(fixture.binary_state(), before)
        self.assertEqual(set(fixture.published.iterdir()), {fixture.output, second_output})
        fixture.assert_no_side_effects(self)

    def test_should_expose_real_query_barriers_and_byte_bound_version_controls(self) -> None:
        """Sanity controls prove both synchronization points and the actual ELF banners."""
        for name, tool, banner in (
            ("clc", self.fixture.binaries / "clc", f"clc {self.fixture.version}"),
            ("clorraine", self.valid, f"clorraine {self.fixture.version}"),
        ):
            with self.subTest(name=name):
                (self.barriers / (name + ".ready")).unlink(missing_ok=True)
                (self.barriers / (name + ".release")).unlink(missing_ok=True)
                child = self._start([str(tool), "--version"])
                try:
                    self.assertTrue(self._ready_or_exited(child, name))
                    self.assertIsNone(child.poll(), "barrier did not keep its ELF child alive")
                    self._release(name)
                    stdout, stderr = child.communicate(timeout=5)
                finally:
                    self._join(child)
                self.assertEqual(child.returncode, 0, stderr)
                self.assertEqual(stdout, banner + "\n")
        wrong = subprocess.run([str(self.wrong), "--version"], capture_output=True,
                               text=True, timeout=5, check=True)
        self.assertEqual(wrong.stdout, "clorraine 9.9.9\n")
