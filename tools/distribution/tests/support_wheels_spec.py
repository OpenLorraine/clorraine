"""Wheel transport/metadata contracts; tiny ZIP fixtures are not native runtime proof."""

from __future__ import annotations

import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
import zipfile

from support_fixtures import SupportFixture, digest, json_bytes


class SupportWheelTests(unittest.TestCase):
    """Replace only Docker and validate actual wheel ZIP/manifest bytes at the boundary."""

    def setUp(self) -> None:
        """Own metadata-only wheel fixtures independently of real artifact acceptance."""
        scratch = tempfile.TemporaryDirectory(
            prefix="support-wheel-spec-", dir=os.environ.get("CODEOPS_TMPDIR"))
        self.addCleanup(scratch.cleanup)
        self.fixture = SupportFixture(Path(scratch.name))
        self.fixture.environment["SUPPORT_DOCKER_SUCCESS"] = "1"
        (self.fixture.payload / "wheels").mkdir()
        self.metadata = {
            "schema": 1, "lock_sha256": digest(self.fixture.lock_path.read_bytes()),
            "abi": "cp310", "packages": {},
        }
        for name in ("amitools", "machine68k", "greenlet", "pysdl2", "pip"):
            tag = "cp310-cp310-linux_x86_64" if name in ("machine68k", "greenlet") else "py3-none-any"
            self.metadata["packages"][name] = self._wheel(name, tag)

    def _wheel(self, name: str, tag: str) -> dict:
        """Create a valid metadata ZIP, deliberately without claiming compiled extension code."""
        filename = f"wheels/{name}-1-{tag}.whl"
        path = self.fixture.payload / filename
        with zipfile.ZipFile(path, "w") as archive:
            archive.writestr(f"{name}/__init__.py", "# Metadata-only protocol fixture.\n")
            archive.writestr(f"{name}-1.dist-info/METADATA",
                             f"Metadata-Version: 2.4\nName: {name}\nVersion: 1\n\n")
            archive.writestr(f"{name}-1.dist-info/WHEEL",
                             "Wheel-Version: 1.0\nGenerator: protocol fixture\n"
                             f"Root-Is-Purelib: {'true' if tag.endswith('-any') else 'false'}\n"
                             f"Tag: {tag}\n\n")
        return {
            "version": "1", "source": "lha" if name == "amitools" else "seed",
            "purpose": "bootstrap" if name == "pip" else "runtime",
            "filename": filename, "bytes": path.stat().st_size,
            "sha256": digest(path.read_bytes()), "tags": [tag],
        }

    def _build(self, encoded: bytes | None = None) -> subprocess.CompletedProcess[str]:
        """Run the public wheel command with real Git/files and the external Docker fixture."""
        fixture = self.fixture
        fixture.save()
        (fixture.payload / "wheels.json").write_bytes(
            json_bytes(self.metadata) if encoded is None else encoded)
        script = Path(__file__).resolve().parents[1] / "build-wheels.sh"
        self.assertTrue(script.is_file(), f"Missing public command: {script}")
        self.assertTrue(os.access(script, os.X_OK), f"Not executable: {script}")
        return subprocess.run(
            [str(script), "--lock", str(fixture.lock_path), "--sources", str(fixture.sources),
             "--image-id", fixture.image_id, "--abi", "cp310",
             "--output", str(fixture.build_output)],
            cwd=fixture.root, env=fixture.environment,
            capture_output=True, text=True, timeout=30)

    def _rejected(self, identity: str, encoded: bytes | None = None) -> None:
        """Reject invalid wheels without publishing artifacts or abandoning private staging."""
        result = self._build(encoded)
        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn(identity, result.stderr)
        output = self.fixture.build_output
        if output.exists():
            self.assertEqual(list(output.iterdir()), [])

    def test_should_publish_verified_wheel_metadata_and_exact_recipe_suffix(self) -> None:
        """Successful wheel output binds the exact lock and fixed ABI recipe suffix."""
        result = self._build()
        self.assertEqual(result.returncode, 0, result.stderr)
        output = self.fixture.build_output
        metadata = json.loads((output / "payload/wheels.json").read_text(encoding="utf-8"))
        self.assertEqual(metadata, self.metadata)
        provenance = json.loads((output / "provenance.json").read_text(encoding="utf-8"))
        expected = [*self.fixture.lock["build"]["argv"], "--wheels", "cp310"]
        self.assertEqual(provenance["argv"], expected)
        self.assertEqual(provenance["lock_sha256"], self.metadata["lock_sha256"])
        calls = self.fixture.docker_calls()
        self.assertTrue(any(call[:1] == ["run"] and call[-len(expected):] == expected
                            for call in calls))

    def test_should_reject_a_missing_runtime_package(self) -> None:
        """The measured mandatory runtime closure cannot silently lose machine68k."""
        del self.metadata["packages"]["machine68k"]
        self._rejected("machine68k")

    def test_should_reject_wheels_for_a_different_interpreter(self) -> None:
        """Correct hashes and metadata cannot make a cp312 native wheel a cp310 artifact."""
        old = self.metadata["packages"]["machine68k"]["filename"]
        (self.fixture.payload / old).unlink()
        self.metadata["packages"]["machine68k"] = self._wheel(
            "machine68k", "cp312-cp312-linux_x86_64")
        self._rejected("machine68k")

    def test_should_reject_changed_wheel_bytes(self) -> None:
        """A recorded wheel digest is checked against the actual file after execution."""
        record = self.metadata["packages"]["greenlet"]
        with (self.fixture.payload / record["filename"]).open("ab") as stream:
            stream.write(b"changed")
        self._rejected("greenlet")

    def test_should_reject_a_source_distribution_in_the_wheel_directory(self) -> None:
        """A successful recipe may not introduce an installation-time source fallback."""
        (self.fixture.payload / "wheels/hidden.tar.gz").write_bytes(b"source distribution")
        self._rejected("hidden.tar.gz")

    def test_should_reject_wheel_metadata_for_different_lock_bytes(self) -> None:
        """Wheel metadata must bind this exact lock, not another structurally valid lock."""
        self.metadata["lock_sha256"] = "f" * 64
        self._rejected("lock_sha256")

    def test_should_reject_unknown_wheel_metadata_fields(self) -> None:
        """Unknown fields cannot silently change the interpreted wheel schema."""
        self.metadata["unexpected"] = True
        self._rejected("unexpected")

    def test_should_reject_duplicate_wheel_metadata_keys(self) -> None:
        """Even repeated identical JSON keys remain ambiguous and invalid."""
        encoded = b'{"schema":1,' + json_bytes(self.metadata)[1:]
        self._rejected("schema", encoded)

    def _package(self) -> subprocess.CompletedProcess[str]:
        """Exercise archive publication with the same metadata-only protocol wheels."""
        fixture = self.fixture
        for record in self.metadata["packages"].values():
            # One synthetic Git component owns these protocol fixture bytes.
            # This is not a real wheel/source correspondence claim.
            record["source"] = "lha"
            path = fixture.payload / record["filename"]
            fixture.add_file(record["filename"], path.read_bytes(), "wheel")
        fixture.add_file("wheels.json", json_bytes(self.metadata), "preparation")
        fixture.save()
        script = Path(__file__).resolve().parents[1] / "package_support.py"
        self.assertTrue(script.is_file(), f"Missing public command: {script}")
        return subprocess.run(
            [sys.executable, str(script), "--lock", str(fixture.lock_path),
             "--payload", str(fixture.payload), "--audit", str(fixture.audit_path),
             "--output", str(fixture.archive_output)],
            cwd=fixture.root, env=fixture.environment,
            capture_output=True, text=True, timeout=30)

    def test_should_package_a_wheel_set_bound_to_the_actual_lock(self) -> None:
        """Packaging preserves the wheel producer's exact-input metadata boundary."""
        result = self._package()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertTrue(self.fixture.archive_output.is_file())

    def test_should_reject_packaging_wheel_metadata_for_different_lock_bytes(self) -> None:
        """A consistent file audit cannot legitimise a wheel manifest for another lock."""
        self.metadata["lock_sha256"] = "f" * 64
        result = self._package()
        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("lock_sha256", result.stderr)
        self.assertFalse(self.fixture.archive_output.exists())
