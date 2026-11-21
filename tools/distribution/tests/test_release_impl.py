"""Implementation controls for release producer validation and reuse decisions.

These tests protect malformed-input and internal-routing behavior that the
public release specification does not need to enumerate. They never publish,
fetch, build or substitute a miniature archive for real acceptance.
"""

from __future__ import annotations

import io
import json
import os
from pathlib import Path
import sys
import unittest
import zipfile

from bootstrap_package_fixtures import BootstrapFixture
from native_manifest_data import COMPONENTS, SUPPORT, digest, encode
from native_manifest_producer_fixture import ScratchCase
from native_transport_fixture import run_process


ROOT = Path(__file__).resolve().parents[3]
PRODUCER = ROOT / "tools/distribution/package_release.py"
ASSEMBLER = ROOT / "tools/distribution/assemble_support.py"


class ReleaseImplCase(ScratchCase):
    """Share a valid miniature assembly and a no-side-effect environment."""

    def setUp(self) -> None:
        """Fail here, not inside a rejection assertion, when the producer is broken."""
        super().setUp()
        self.fixture = BootstrapFixture(self, self.root)

    def _run(self, executable: Path, arguments: list[str]):
        """Run one public command without build, fetch, install or publication tools."""
        return run_process([sys.executable, str(executable), *arguments],
                           self.root, self.fixture.environment)

    def _verify(self, component: str = COMPONENTS, *, core: Path | None = None,
                vsix: Path | None = None, existing: Path | None = None):
        """Invoke component verification with caller-selected exact inputs."""
        arguments = ["verify-component", "--components", component,
                     "--core", str(core or self.fixture.source.paths[COMPONENTS + "-core"]),
                     "--vsix", str(vsix or self.fixture.source.paths[COMPONENTS + "-vsix"])]
        if existing is not None:
            arguments.extend(["--existing", str(existing)])
        return self._run(PRODUCER, arguments)

    def _assets(self) -> tuple[Path, dict, Path]:
        """Write exact referenced payloads and a matching approval record."""
        manifest = self.fixture.source.manifest
        directory = self.root / "promotion assets"
        directory.mkdir()
        for asset in manifest["assets"]:
            if asset["kind"] == "ndk":
                continue
            name = asset["url"].rsplit("/", 1)[-1]
            (directory / name).write_bytes(self.fixture.source.raw[name])
        approval = self.root / "approval.json"
        approval.write_bytes(encode({
            "schema": 1, "release": manifest["release"],
            "manifest_sha256": digest((self.fixture.output / "manifest.json").read_bytes()),
            "approved": True, "unresolved_major": 0, "st56": "pending"}))
        return directory, manifest, approval


class ReleaseInputValidationTests(ReleaseImplCase):
    """Literal dispatch values and mode combinations fail closed with clean errors."""

    def test_should_accept_full_and_support_only_validation_modes(self) -> None:
        """Support-only dispatch still validates its identity and commit."""
        full = self._run(PRODUCER, ["validate", "--release", "v1.2.0",
                                    "--components", COMPONENTS, "--support", SUPPORT,
                                    "--commit", "a" * 40])
        self.assertEqual(full.returncode, 0, full.stderr)
        self.assertEqual(json.loads(full.stdout),
                         {"schema": 1, "release": "v1.2.0", "components": COMPONENTS,
                          "support": SUPPORT, "commit": "a" * 40})
        support = self._run(PRODUCER, ["validate", "--support", SUPPORT,
                                       "--commit", "b" * 40])
        self.assertEqual(support.returncode, 0, support.stderr)
        self.assertEqual(json.loads(support.stdout),
                         {"schema": 1, "support": SUPPORT, "commit": "b" * 40})

    def test_should_reject_partial_or_unsafe_validation_values(self) -> None:
        """A release without components, bad tokens and bad commits exit nonzero."""
        cases = [
            ["--release", "v1.2.0", "--support", SUPPORT, "--commit", "a" * 40],
            ["--support", SUPPORT, "--commit", "a" * 39],
            ["--support", "../escape", "--commit", "a" * 40],
            ["--components", "Upper", "--support", SUPPORT, "--commit", "a" * 40],
            ["--release", "v1.2.0\n", "--components", COMPONENTS,
             "--support", SUPPORT, "--commit", "a" * 40],
            ["--support", SUPPORT, "--commit", "a" * 40,
             "--previous-core", "../escape"],
            ["--support", SUPPORT, "--commit", "a" * 40,
             "--previous-vsix", "Upper"],
        ]
        for arguments in cases:
            with self.subTest(arguments=arguments):
                result = self._run(PRODUCER, ["validate", *arguments])
                self.assertNotEqual(result.returncode, 0)
                self.assertNotIn("Traceback", result.stderr)
                self.assertTrue(result.stderr.strip())
        accepted = self._run(PRODUCER, ["validate", "--support", SUPPORT,
                                        "--commit", "a" * 40,
                                        "--previous-core", "none",
                                        "--previous-vsix", "prior-vsix"])
        self.assertEqual(accepted.returncode, 0, accepted.stderr)
        self.assertEqual(json.loads(accepted.stdout)["previous_vsix"], "prior-vsix")

    def test_should_keep_assemble_support_modes_narrow_and_non_root(self) -> None:
        """Preparation needs a baseline; other kinds cannot accept one; unknown kinds fail."""
        base = ["assemble", "--lock", "lock", "--sources", "sources",
                "--producer", "producer", "--output", "output"]
        missing = self._run(ASSEMBLER, [*base, "--kind", "preparation"])
        self.assertEqual(missing.returncode, 1)
        self.assertIn("sdk-base", missing.stderr)
        extra = self._run(ASSEMBLER, [*base, "--kind", "sdk", "--sdk-base", "sdk"])
        self.assertEqual(extra.returncode, 1)
        self.assertIn("only valid", extra.stderr)
        unknown = self._run(ASSEMBLER, [*base, "--kind", "other"])
        self.assertEqual(unknown.returncode, 2)
        wheel = self.root / "wheel-producer"
        wheel.mkdir()
        (wheel / "wheels.json").write_text("{}\n", encoding="utf-8")
        mismatched = self._run(ASSEMBLER, ["assemble", "--kind", "sdk",
                                           "--lock", "lock", "--sources", "sources",
                                           "--producer", str(wheel), "--output", "output"])
        self.assertEqual(mismatched.returncode, 1)
        self.assertIn("wheel payload", mismatched.stderr)

    def test_should_build_the_preparation_bundle_from_locked_inputs(self) -> None:
        """The bundle contains the locked script/inputs and exact SDK helpers."""
        payload = self.root / "sdk-payload"
        (payload / "bin").mkdir(parents=True)
        for identity in ("lha", "fd2sfd", "sfdc"):
            (payload / "bin" / identity).write_bytes(b"helper " + identity.encode())
            (payload / "bin" / identity).chmod(0o755)
        output = self.root / "preparation-bundle"
        result = self._run(ASSEMBLER, ["bundle-preparation",
                                       "--lock", str(ROOT / "tools/distribution/support-lock.json"),
                                       "--sdk-payload", str(payload),
                                       "--output", str(output)])
        self.assertEqual(result.returncode, 0, result.stderr)
        metadata = json.loads((output / "preparation.json").read_bytes())
        self.assertEqual(set(metadata), {"schema", "lock_sha256", "archive", "helpers",
                                         "inputs", "inventory", "notices"})
        self.assertEqual(set(metadata["helpers"]), {"lha", "fd2sfd", "sfdc"})
        self.assertTrue((output / "prepare_ndk.py").is_file())
        self.assertTrue((output / "ndk-inputs/helper-headers/alib.h").is_file())
        second = self._run(ASSEMBLER, ["bundle-preparation",
                                       "--lock", str(ROOT / "tools/distribution/support-lock.json"),
                                       "--sdk-payload", str(payload),
                                       "--output", str(output)])
        self.assertEqual(second.returncode, 1)
        self.assertIn("existing output", second.stderr)
        self.assertTrue((output / "preparation.json").is_file())


class ReleasePayloadValidationTests(ReleaseImplCase):
    """Core/VSIX metadata and caller-owned paths are checked before classification."""

    def test_should_classify_core_reuse_independently_from_vsix(self) -> None:
        """One unchanged payload can be reused while the other is still new."""
        existing = self.root / "existing component"
        existing.mkdir()
        (existing / "core.tar").write_bytes(self.fixture.source.raw["core.tar"])
        result = self._verify(existing=existing)
        self.assertEqual(result.returncode, 0, result.stderr)
        record = json.loads(result.stdout)
        self.assertEqual(record["core"]["decision"], "reuse")
        self.assertEqual(record["vsix"]["decision"], "publish")

    def test_should_reject_changed_vsix_metadata(self) -> None:
        """A VSIX with a different version cannot pass as the current editor payload."""
        original = self.fixture.source.paths[COMPONENTS + "-vsix"].read_bytes()
        changed = io.BytesIO()
        with zipfile.ZipFile(io.BytesIO(original)) as source, zipfile.ZipFile(changed, "w") as out:
            for item in source.infolist():
                data = source.read(item.filename)
                if item.filename == "extension/package.json":
                    metadata = json.loads(data)
                    metadata["version"] = "9.9.9"
                    data = encode(metadata)
                out.writestr(item, data)
        path = self.root / "changed.vsix"
        path.write_bytes(changed.getvalue())
        result = self._verify(vsix=path)
        self.assertEqual(result.returncode, 1)
        self.assertIn("VSIX", result.stderr)
        self.assertNotIn("Traceback", result.stderr)

    def test_should_reject_alias_or_nonordinary_existing_assets(self) -> None:
        """A symlinked directory, symlinked asset or FIFO is not immutable history."""
        target = self.root / "real existing"
        target.mkdir()
        (target / "core.tar").write_bytes(self.fixture.source.raw["core.tar"])
        alias = self.root / "existing alias"
        alias.symlink_to(target, target_is_directory=True)
        for existing in (alias,):
            with self.subTest(kind="directory alias"):
                result = self._verify(existing=existing)
                self.assertEqual(result.returncode, 1)
                self.assertIn("ordinary directory", result.stderr)
        for kind in ("symlink", "fifo"):
            with self.subTest(kind=kind):
                directory = self.root / ("nonordinary-" + kind)
                directory.mkdir()
                path = directory / "core.tar"
                if kind == "symlink":
                    path.symlink_to(target / "core.tar")
                else:
                    os.mkfifo(path)
                try:
                    result = self._verify(existing=directory)
                    self.assertEqual(result.returncode, 1)
                    self.assertNotIn("Traceback", result.stderr)
                finally:
                    path.unlink()

    def test_should_reject_identity_traversal_in_verify_component(self) -> None:
        """A component identity remains a token, not an output path."""
        result = self._verify("../escape")
        self.assertEqual(result.returncode, 1)
        self.assertIn("identity", result.stderr)
        self.assertNotIn("Traceback", result.stderr)


class ReleasePromotionValidationTests(ReleaseImplCase):
    """Approval shape, ordinary assets and measured inventories fail closed."""

    def test_should_reject_unknown_duplicate_or_mistyped_approval_records(self) -> None:
        """The approval document has an exact schema and bounded field types."""
        assets, manifest, approval = self._assets()
        base = json.loads(approval.read_bytes())
        cases = {
            "unknown": {**base, "extra": True},
            "missing": {key: value for key, value in base.items() if key != "st56"},
            "boolean-major": {**base, "unresolved_major": True},
            "schema": {**base, "schema": 2},
            "st56": {**base, "st56": "later"},
        }
        for label, value in cases.items():
            with self.subTest(label=label):
                approval.write_bytes(encode(value))
                result = self._run(PRODUCER, ["promotion",
                                              "--manifest", str(self.fixture.output / "manifest.json"),
                                              "--assets", str(assets),
                                              "--approval", str(approval)])
                self.assertEqual(result.returncode, 1)
                self.assertNotIn("Traceback", result.stderr)
        duplicate = ('{"schema":1,"release":"%s","manifest_sha256":"%s",'
                     '"approved":true,"approved":false,"unresolved_major":0,'
                     '"st56":"pending"}\n' % (manifest["release"], "f" * 64))
        approval.write_text(duplicate, encoding="utf-8")
        result = self._run(PRODUCER, ["promotion",
                                      "--manifest", str(self.fixture.output / "manifest.json"),
                                      "--assets", str(assets),
                                      "--approval", str(approval)])
        self.assertEqual(result.returncode, 1)
        self.assertIn("duplicate", result.stderr.lower())

    def test_should_block_unreadable_asset_and_inventory_mismatch(self) -> None:
        """Private payloads and a false tar inventory never become ready evidence."""
        assets, manifest, approval = self._assets()
        core = next(asset for asset in manifest["assets"] if asset["kind"] == "core")
        name = core["url"].rsplit("/", 1)[-1]
        path = assets / name
        original = path.read_bytes()
        path.chmod(0)
        try:
            result = self._run(PRODUCER, ["promotion",
                                          "--manifest", str(self.fixture.output / "manifest.json"),
                                          "--assets", str(assets), "--approval", str(approval)])
            self.assertEqual(result.returncode, 1)
            record = json.loads(result.stdout)
            self.assertTrue(any("unreadable" in blocker or "hash" in blocker
                                for blocker in record["blockers"]))
        finally:
            path.chmod(0o600)
            path.write_bytes(original)
        changed = json.loads((self.fixture.output / "manifest.json").read_bytes())
        changed_core = next(asset for asset in changed["assets"] if asset["kind"] == "core")
        changed_core["entries"] += 1
        manifest_path = self.root / "changed-manifest.json"
        manifest_path.write_bytes(encode(changed))
        approval.write_bytes(encode({**json.loads(approval.read_bytes()),
                                     "manifest_sha256": digest(encode(changed))}))
        result = self._run(PRODUCER, ["promotion", "--manifest", str(manifest_path),
                                      "--assets", str(assets), "--approval", str(approval)])
        self.assertEqual(result.returncode, 1)
        self.assertIn("inventory mismatch", result.stdout)

    def test_should_reject_assets_directory_alias_and_extra_output(self) -> None:
        """The assets directory is ordinary and no command writes a partial result."""
        assets, _manifest, approval = self._assets()
        alias = self.root / "assets alias"
        alias.symlink_to(assets, target_is_directory=True)
        before = {path.name for path in assets.iterdir()}
        result = self._run(PRODUCER, ["promotion",
                                      "--manifest", str(self.fixture.output / "manifest.json"),
                                      "--assets", str(alias), "--approval", str(approval)])
        self.assertEqual(result.returncode, 1)
        self.assertIn("ordinary directory", result.stderr)
        self.assertEqual({path.name for path in assets.iterdir()}, before)


if __name__ == "__main__":
    unittest.main()
