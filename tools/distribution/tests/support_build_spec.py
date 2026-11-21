"""Support-build success, output containment and post-recipe inspection contracts."""

from __future__ import annotations

import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

from support_fixtures import SupportFixture, digest


class SupportBuildTests(unittest.TestCase):
    """Replace only Docker while exercising real source checks and output files."""

    def setUp(self) -> None:
        """Own each independent fixture and enable a successful external container."""
        scratch = tempfile.TemporaryDirectory(
            prefix="support-build-spec-", dir=os.environ.get("CODEOPS_TMPDIR"))
        self.addCleanup(scratch.cleanup)
        self.fixture = SupportFixture(Path(scratch.name))
        self.fixture.environment["SUPPORT_DOCKER_SUCCESS"] = "1"

    def _build(self, output: Path | None = None) -> subprocess.CompletedProcess[str]:
        """Invoke the public script without replacing filesystem or Git behaviour."""
        fixture = self.fixture
        fixture.save()
        script = Path(__file__).resolve().parents[1] / "build-support.sh"
        self.assertTrue(script.is_file(), f"Missing public command: {script}")
        self.assertTrue(os.access(script, os.X_OK), f"Command is not executable: {script}")
        return subprocess.run(
            [str(script), "--lock", str(fixture.lock_path),
             "--sources", str(fixture.sources), "--image-id", fixture.image_id,
             "--output", str(output if output is not None else fixture.build_output)],
            cwd=fixture.root, env=fixture.environment,
            capture_output=True, text=True, timeout=30)

    def _reject(self, identity: str, output: Path | None = None) -> None:
        """Require a named rejection and no successful or abandoned output."""
        result = self._build(output)
        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn(identity, result.stderr)
        destination = output if output is not None else self.fixture.build_output
        if destination.exists():
            self.assertEqual(list(destination.iterdir()), [], "failed build left staging")

    def test_should_publish_exact_provenance_after_a_successful_recipe(self) -> None:
        """Only inspected recipe outputs receive complete exact-input provenance."""
        fixture = self.fixture
        result = self._build()
        self.assertEqual(result.returncode, 0, result.stderr)
        output = fixture.build_output
        self.assertEqual({path.name for path in output.iterdir()}, {"payload", "provenance.json"})
        actual = json.loads((output / "provenance.json").read_text(encoding="utf-8"))
        expected = {
            "schema": 1, "lock_sha256": digest(fixture.lock_path.read_bytes()),
            "image": fixture.lock["build"]["image"], "image_id": fixture.image_id,
            "environment_recipe_sha256": fixture.lock["inputs"]["environment.Dockerfile"],
            "argv": fixture.lock["build"]["argv"], "exit_code": 0,
            "sources": {name: source.get("commit", source.get("sha256"))
                        for name, source in fixture.lock["sources"].items()},
            "inputs": fixture.lock["inputs"],
            "files": {name: {"type": "file", "sha256": record["sha256"]}
                      for name, record in fixture.audit["files"].items()},
        }
        self.assertEqual(actual, expected)
        for path in fixture.payload.rglob("*"):
            if path.is_file():
                self.assertEqual((output / "payload" / path.relative_to(fixture.payload)).read_bytes(),
                                 path.read_bytes())

    def test_should_accept_an_existing_empty_output_directory(self) -> None:
        """A caller-owned empty destination is a permitted new-build location."""
        self.fixture.build_output.mkdir()
        result = self._build()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertTrue((self.fixture.build_output / "provenance.json").is_file())

    def test_should_accept_dockers_normalised_base_repository_spelling(self) -> None:
        """Docker's short registry display name does not change the inspected base layers."""
        self.fixture.environment["SUPPORT_DOCKER_SHORT_BASE_NAME"] = "1"
        result = self._build()
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_should_record_safe_relative_payload_links(self) -> None:
        """Inspection retains internal relative links without dereferencing them."""
        (self.fixture.payload / "bin/lha-link").symlink_to("lha")
        result = self._build()
        self.assertEqual(result.returncode, 0, result.stderr)
        output = self.fixture.build_output
        metadata = json.loads((output / "provenance.json").read_text(encoding="utf-8"))
        self.assertEqual(metadata["files"]["bin/lha-link"],
                         {"type": "symlink", "target": "lha"})
        self.assertEqual(os.readlink(output / "payload/bin/lha-link"), "lha")

    def test_should_reject_output_inside_source_staging_before_docker(self) -> None:
        """Output creation cannot modify the verified source closure."""
        output = self.fixture.sources / "build-output"
        self._reject("output", output)
        self.assertEqual(self.fixture.docker_calls(), [])
        self.assertFalse(output.exists())

    def test_should_reject_an_output_symlink_without_touching_its_target(self) -> None:
        """An existing destination link is not an empty output directory."""
        target = self.fixture.root / "untouched"
        target.mkdir()
        self.fixture.build_output.symlink_to(target, target_is_directory=True)
        result = self._build()
        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("output", result.stderr)
        self.assertTrue(self.fixture.build_output.is_symlink())
        self.assertEqual(list(target.iterdir()), [])
        self.assertEqual(self.fixture.docker_calls(), [])

    def test_should_reject_recipe_payload_links_to_outside_files(self) -> None:
        """Successful execution does not make escaping payload links acceptable."""
        (self.fixture.payload / "bin/escape").symlink_to("../../outside")
        self._reject("bin/escape")

    def test_should_reject_empty_recipe_payload_directories(self) -> None:
        """The file-based output inventory cannot silently omit an empty directory."""
        (self.fixture.payload / "unaccounted-empty").mkdir()
        self._reject("unaccounted-empty")

    def test_should_reject_special_recipe_payload_files(self) -> None:
        """A FIFO must be rejected without opening it or publishing success."""
        self.fixture.environment["SUPPORT_DOCKER_PAYLOAD_FIFO"] = "1"
        self._reject("channel")
