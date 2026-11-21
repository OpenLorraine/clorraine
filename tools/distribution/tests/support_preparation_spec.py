"""Local preparation and wheel-build rejection contracts; no real transformation is faked."""

from __future__ import annotations

import copy
import importlib.util
import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

from support_fixtures import SupportFixture, digest, json_bytes


def member(path: str, kind: str = "file", target: str | None = None) -> dict:
    """Describe one archive member through the public preparation boundary."""
    return {"path": path, "type": kind, "target": target}


class SupportPreparationTests(unittest.TestCase):
    """Use real files/processes, replacing only external failing helper/container calls."""

    def setUp(self) -> None:
        """Own the complete fixture lifetime, including cleanup after setup failures."""
        scratch = tempfile.TemporaryDirectory(
            prefix="support-preparation-", dir=os.environ.get("CODEOPS_TMPDIR"))
        self.addCleanup(scratch.cleanup)
        self.fixture = SupportFixture(Path(scratch.name))

    def _entry(self, name: str) -> Path:
        """Require the public entry rather than interpreting its absence as rejection."""
        path = Path(__file__).resolve().parents[1] / name
        self.assertTrue(path.is_file(), f"Missing public command: {path}")
        return path

    def _validator(self):
        """Load only the documented callable when the test actually runs."""
        path = self._entry("prepare_ndk.py")
        spec = importlib.util.spec_from_file_location("_preparation_contract_subject", path)
        self.assertIsNotNone(spec.loader)
        module = importlib.util.module_from_spec(spec)
        sys.modules[spec.name] = module
        self.addCleanup(sys.modules.pop, spec.name, None)
        spec.loader.exec_module(module)
        validator = getattr(module, "validate_archive_members", None)
        self.assertTrue(callable(validator), "prepare_ndk.validate_archive_members is required")
        return validator

    def _reject_members(self, members: list[dict], identity: str) -> None:
        """Require a named value error, not an unrelated import or process failure."""
        validator = self._validator()
        with self.assertRaises(ValueError) as caught:
            validator(members)
        self.assertIn(identity, str(caught.exception))

    def _preparation(self) -> tuple[Path, dict]:
        """Copy the standalone entry beside hashed, deliberately non-transforming helpers."""
        bundle = self.fixture.root / "preparation resources"
        bundle.mkdir()
        shutil.copy2(self._entry("prepare_ndk.py"), bundle / "prepare_ndk.py")
        helper = (
            "#!" + sys.executable + "\n"
            '"""Record an unexpected external helper invocation, then fail."""\n'
            "import os\nfrom pathlib import Path\n"
            "Path(os.environ['SUPPORT_HELPER_LOG']).write_text('helper executed\\n')\n"
            "raise SystemExit(91)\n").encode()
        helpers = {}
        (bundle / "helpers").mkdir()
        for name in ("lha", "sfdc", "fd2sfd", "fd2pragma"):
            path = "helpers/" + name
            (bundle / path).write_bytes(helper)
            (bundle / path).chmod(0o755)
            helpers[name] = {"path": path, "sha256": digest(helper)}
        patch = b"Synthetic preparation input.\n"
        (bundle / "input.patch").write_bytes(patch)
        self.fixture.lock["inputs"]["input.patch"] = digest(patch)
        (self.fixture.sources / "inputs/input.patch").write_bytes(patch)
        self.fixture.lock["preparation"]["patches"] = ["input.patch"]
        self.fixture.lock["preparation"]["helpers"] = {
            name: {"source": "lha", "path": record["path"]} for name, record in helpers.items()}
        inventory = dict(self.fixture.lock["preparation"]["inventory"])
        inventory["include/NOTICE"] = digest(b"Synthetic retained notice.\n")
        self.fixture.lock["preparation"]["inventory"] = inventory
        self.fixture.save()
        metadata = {
            "schema": 1, "lock_sha256": digest(self.fixture.lock_path.read_bytes()),
            "archive": {"source": "seed", "bytes": self.fixture.lock["sources"]["seed"]["bytes"],
                        "sha256": self.fixture.lock["sources"]["seed"]["sha256"]},
            "base": {path: {"type": "file", "sha256": record["sha256"]}
                     for path, record in self.fixture.audit["files"].items()},
            "helpers": helpers, "inputs": {"input.patch": digest(patch)},
            "inventory": inventory, "notices": ["include/NOTICE"]}
        return bundle, metadata

    def _run_preparation(self, bundle: Path, metadata: dict, identity: str,
                         encoded: bytes | None = None) -> None:
        """A failed precondition must precede helper execution and completion publication."""
        (bundle / "preparation.json").write_bytes(
            json_bytes(metadata) if encoded is None else encoded)
        log = self.fixture.root / "helper.log"
        outside = self.fixture.root / "outside-sentinel"
        outside.write_bytes(b"outside remains unchanged\n")
        environment = dict(self.fixture.environment, SUPPORT_HELPER_LOG=str(log))
        result = subprocess.run(
            [sys.executable, str(bundle / "prepare_ndk.py"), "--archive",
             str(self.fixture.sources / "archives/seed"), "--destination", str(self.fixture.payload)],
            cwd=self.fixture.root, env=environment, capture_output=True, text=True, timeout=30)
        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn(identity, result.stderr)
        self.assertFalse(log.exists(), "a helper ran before preconditions were established")
        self.assertFalse((self.fixture.payload / ".ndk-prepared.json").exists())
        self.assertEqual(outside.read_bytes(), b"outside remains unchanged\n")

    def _wheel_build(self, abi: str, image_id: str | None = None) -> subprocess.CompletedProcess[str]:
        """Invoke the real wheel command with an explicit prepared-image identity."""
        entry = self._entry("build-wheels.sh")
        self.assertTrue(os.access(entry, os.X_OK), f"Not executable: {entry}")
        self.fixture.save()
        if image_id is None:
            image_id = self.fixture.image_id
        return subprocess.run(
            [str(entry), "--lock", str(self.fixture.lock_path), "--sources", str(self.fixture.sources),
             "--image-id", image_id, "--abi", abi, "--output", str(self.fixture.build_output)],
            cwd=self.fixture.root, env=self.fixture.environment,
            capture_output=True, text=True, timeout=30)

    def _failed_wheel_recipe(self, abi: str) -> None:
        """A real recipe-boundary failure must preserve isolation and discard private work."""
        self.assertNotEqual(os.getuid(), 0, "wheel builds require an ordinary user")
        result = self._wheel_build(abi)
        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("recipe.sh", result.stderr)
        expected = [*self.fixture.lock["build"]["argv"], "--wheels", abi]
        calls = [call for call in self.fixture.docker_calls()
                 if call[:1] == ["run"] and call[-len(expected):] == expected]
        self.assertTrue(calls, "the fixed wheel recipe suffix never reached Docker")
        for call in calls:
            self.assertEqual(call[-len(expected) - 1], self.fixture.image_id)
            for name, value in (("--network", "none"), ("--pull", "never")):
                self.assertTrue(name + "=" + value in call or any(
                    call[index:index + 2] == [name, value] for index in range(len(call))))
        output = self.fixture.build_output
        self.assertFalse((output / "payload").exists())
        self.assertFalse((output / "provenance.json").exists())
        if output.exists():
            self.assertEqual(list(output.iterdir()), [], "failed wheel build left private staging")

    def test_should_accept_a_safe_complete_member_listing(self) -> None:
        """Directories, files and internal relative aliases are legitimate archive entries."""
        listing = [member("header-alias", "symlink", "include/header.h"),
                   member("include/header.h"), member("include", "directory")]
        original = copy.deepcopy(listing)
        self.assertIsNone(self._validator()(listing))
        self.assertEqual(listing, original, "pure validation must not rewrite its inputs")

    def test_should_reject_unsafe_member_paths(self) -> None:
        """Traversal, absolute names, alternate separators and control bytes are rejected."""
        for path, identity in (
                ("../escape", "escape"), ("/absolute", "absolute"), ("dir\\escape", "escape"),
                ("control\nname", "control"), ("control\x00name", "control"),
                ("dir//header", "header"), ("dir/./header", "header")):
            with self.subTest(path=repr(path)):
                self._reject_members([member(path)], identity)

    def test_should_reject_duplicate_member_destinations(self) -> None:
        """The same destination cannot be written twice under conflicting archive records."""
        self._reject_members([member("duplicate"), member("duplicate")], "duplicate")

    def test_should_reject_extraction_through_a_symlink_parent(self) -> None:
        """Even an internal alias cannot be a parent of an extracted member."""
        self._reject_members(
            [member("real", "directory"), member("alias", "symlink", "real"),
             member("alias/header.h")], "alias")

    def test_should_reject_an_escaping_link(self) -> None:
        """A relative link cannot resolve outside the extraction root."""
        self._reject_members([member("escape-link", "symlink", "../outside")], "escape-link")

    def test_should_reject_a_dangling_link(self) -> None:
        """An internal-looking target must actually exist in the complete listing."""
        self._reject_members([member("dangling", "symlink", "missing")], "dangling")

    def test_should_reject_link_cycles(self) -> None:
        """Following an archive alias must terminate at an actual file or directory."""
        self._reject_members(
            [member("cycle-a", "symlink", "cycle-b"), member("cycle-b", "symlink", "cycle-a")],
            "cycle-")

    def test_should_reject_special_member_types(self) -> None:
        """Device nodes, pipes and unrecognized entry types are never extracted."""
        for kind in ("fifo", "block-device", "character-device", "socket", "hardlink"):
            with self.subTest(kind=kind):
                self._reject_members([member("special", kind)], "special")

    def test_should_check_corrupt_archive_bytes_before_running_helpers(self) -> None:
        """A one-byte change cannot reach an external archive tool."""
        bundle, metadata = self._preparation()
        archive = self.fixture.sources / "archives/seed"
        content = archive.read_bytes()
        archive.write_bytes(content[:-1] + bytes([content[-1] ^ 1]))
        self._run_preparation(bundle, metadata, "seed")

    def test_should_check_archive_length_before_running_helpers(self) -> None:
        """A declared byte length must match before any archive tool runs."""
        bundle, metadata = self._preparation()
        metadata["archive"]["bytes"] += 1
        self._run_preparation(bundle, metadata, "seed")

    def test_should_reject_a_missing_preparation_helper(self) -> None:
        """A declared helper must exist before any preparation process is launched."""
        bundle, metadata = self._preparation()
        (bundle / metadata["helpers"]["lha"]["path"]).unlink()
        self._run_preparation(bundle, metadata, "lha")

    def test_should_reject_changed_helper_bytes(self) -> None:
        """A correctly named helper cannot execute after its bytes change."""
        bundle, metadata = self._preparation()
        with (bundle / metadata["helpers"]["lha"]["path"]).open("ab") as stream:
            stream.write(b"\n# changed\n")
        self._run_preparation(bundle, metadata, "lha")

    def test_should_reject_a_mismatching_sdk_baseline(self) -> None:
        """Preparation is restricted to the exact fresh SDK tree recorded in metadata."""
        bundle, metadata = self._preparation()
        (self.fixture.payload / "bin/lha").write_bytes(b"changed baseline\n")
        self._run_preparation(bundle, metadata, "bin/lha")

    def test_should_reject_invalid_preparation_metadata_before_helpers(self) -> None:
        """Malformed metadata cannot authorize archive or helper execution."""
        bundle, metadata = self._preparation()
        for mutation, identity in (("unknown", "unexpected"), ("boolean", "bytes"),
                                   ("duplicate", "schema"), ("unsafe", "../outside")):
            with self.subTest(mutation=mutation):
                changed = copy.deepcopy(metadata)
                if mutation == "unknown":
                    changed["unexpected"] = True
                elif mutation == "boolean":
                    changed["archive"]["bytes"] = True
                elif mutation == "unsafe":
                    changed["helpers"]["lha"]["path"] = "../outside"
                    shutil.copy2(bundle / "helpers/lha", self.fixture.root / "outside")
                encoded = json_bytes(changed)
                if mutation == "duplicate":
                    encoded = b'{"schema":1,' + encoded[1:]
                self._run_preparation(bundle, changed, identity, encoded)

    def test_should_reject_an_unsupported_wheel_abi_before_docker(self) -> None:
        """An unsupported interpreter cannot select a recipe or create outputs."""
        result = self._wheel_build("cp311")
        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("cp311", result.stderr)
        self.assertEqual(self.fixture.docker_calls(), [])
        self.assertFalse(self.fixture.build_output.exists())

    def test_should_reject_a_mutable_wheel_image_before_docker(self) -> None:
        """Wheel builds require an immutable image ID before any external operation."""
        image = "clorraine-prepared:latest"
        result = self._wheel_build("cp310", image)
        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn(image, result.stderr)
        self.assertEqual(self.fixture.docker_calls(), [])
        self.assertFalse(self.fixture.build_output.exists())

    def test_should_reject_wheel_builds_with_unverified_prepared_image(self) -> None:
        """A wrong environment-recipe label prevents wheel execution and publication."""
        self.fixture.environment["SUPPORT_DOCKER_RECIPE_LABEL"] = "e" * 64
        result = self._wheel_build("cp310")
        self.assertNotEqual(result.returncode, 0, result.stdout)
        identities = (self.fixture.image_id, self.fixture.lock["build"]["environment_recipe"],
                      "org.clorraine.environment-recipe-sha256")
        self.assertTrue(any(identity in result.stderr for identity in identities), result.stderr)
        calls = self.fixture.docker_calls()
        self.assertTrue(any(
            self.fixture.image_id in call
            and (call[:2] == ["image", "inspect"] or call[:1] == ["inspect"])
            for call in calls), "the prepared image was not inspected")
        expected = [*self.fixture.lock["build"]["argv"], "--wheels", "cp310"]
        self.assertFalse(any(
            call[:1] == ["run"] and call[-len(expected):] == expected for call in calls),
            "unverified image reached the wheel recipe")
        output = self.fixture.build_output
        self.assertFalse((output / "payload").exists())
        self.assertFalse((output / "provenance.json").exists())
        if output.exists():
            self.assertEqual(list(output.iterdir()), [], "rejected image left private staging")

    def test_should_reject_wheel_builds_with_an_incomplete_source_lock(self) -> None:
        """Wheel recipes consume the same exact, complete input lock as support builds."""
        del self.fixture.lock["sources"]["seed"]["sha256"]
        result = self._wheel_build("cp310")
        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("seed", result.stderr)
        self.assertFalse(self.fixture.build_output.exists())

    def test_should_discard_a_failed_python310_wheel_build(self) -> None:
        """The Python 3.10 recipe uses its fixed suffix and cannot publish failed work."""
        self._failed_wheel_recipe("cp310")

    def test_should_discard_a_failed_python312_wheel_build(self) -> None:
        """The Python 3.12 recipe uses its fixed suffix and cannot publish failed work."""
        self._failed_wheel_recipe("cp312")
