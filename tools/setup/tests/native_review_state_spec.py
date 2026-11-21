"""Filesystem recovery contracts using real private trees, not runtime acceptance."""

from __future__ import annotations

import json
import os
from pathlib import Path
import signal
import subprocess
import sys

from native_manifest_producer_fixture import ScratchCase
from native_manifest_data import encode, make_manifest
from native_transport_fixture import limit_files


_REPOSITORY = Path(__file__).resolve().parents[3]
"""Import location for local mechanical checks; never mounted by actual acceptance."""
_ORIGINAL_PATH = sys.path[:]
try:
    sys.path.insert(0, str(_REPOSITORY))
    from tools.setup import native_state as state
finally:
    sys.path[:] = _ORIGINAL_PATH

_INTERRUPTED_COMPLETION = r'''
import errno
from pathlib import Path
import resource
import signal
import sys
sys.path.insert(0, sys.argv[1])
from tools.setup.native_state import OwnedRoot, interpreter_identity
root, directory, manifest, disposition = sys.argv[2:]
identity = interpreter_identity("cp310")
with OwnedRoot(Path(root)) as owner:
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
    signal.signal(signal.SIGXFSZ, signal.SIG_DFL if disposition == "death" else signal.SIG_IGN)
    resource.setrlimit(resource.RLIMIT_FSIZE, (128, 1024 ** 3))
    try:
        owner.complete(Path(directory), manifest.encode(), identity)
    except OSError as error:
        if error.errno != errno.EFBIG:
            raise
        raise SystemExit(73)
raise SystemExit("completion unexpectedly fit within the forced write limit")
'''
"""A real file-size limit interrupts receipt writes without a production test hook."""


class NativeReviewCompletionTests(ScratchCase):
    """Exercise completion publication and retry without claiming compiler success."""

    def stage(self, owner, name: str, manifest: bytes) -> Path:
        """Stage passive inventory bytes using the existing filesystem boundary."""
        with owner.work() as work:
            candidate = work / "candidate"
            candidate.mkdir(mode=0o700)
            (candidate / "passive-observer").write_bytes(name.encode() * 256)
            metadata = json.loads(manifest)
            assets = {item["kind"]: item for item in metadata["assets"]}
            (candidate / "manifest.json").write_bytes(manifest)
            (candidate / "installation.json").write_bytes(encode({
                "schema": 1, "core_version": metadata["core_version"], "python_abi": "cp310",
                "support_lock_sha256": assets["support-lock"]["sha256"],
                "ndk_sha256": assets["ndk"]["sha256"],
                "prepared_inventory_sha256": metadata["support"]["prepared_inventory_sha256"]}))
            return owner.stage(candidate, metadata["release"], "cp310")

    def completed_control(self, owner, manifest: bytes) -> Path:
        """Publish a mechanical receipt; these passive bytes are not runnable tools."""
        directory = self.stage(owner, "ordinary-control", manifest)
        identity = state.interpreter_identity("cp310")
        owner.complete(directory, manifest, identity)
        self.assertEqual(owner.completed(manifest, identity), directory)
        owner.activate(directory)
        self.assertEqual((owner.root / "current").resolve(strict=True), directory)
        return directory

    def interruption(self, disposition: str) -> None:
        """Preserve the old selection and retry after an actual partial write or death."""
        root = self.root / "installation"
        metadata = make_manifest()
        old_manifest = encode(metadata)
        metadata["release"] = "v1.2.0-r2"
        new_manifest = encode(metadata)
        identity = state.interpreter_identity("cp310")
        with state.OwnedRoot(root) as owner:
            old = self.completed_control(owner, old_manifest)
            old_receipt = (old / "complete.json").read_bytes()
            incomplete = self.stage(owner, "interrupted-control", new_manifest)
            selected = os.readlink(root / "current")
        result = subprocess.run(
            [sys.executable, "-I", "-B", "-c", _INTERRUPTED_COMPLETION,
             str(_REPOSITORY), str(root), str(incomplete),
             new_manifest.decode(), disposition],
            stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
            cwd=self.root, timeout=20, check=False, preexec_fn=limit_files)
        expected = -signal.SIGXFSZ if disposition == "death" else 73
        self.assertEqual(result.returncode, expected, result.stderr.decode(errors="replace"))
        self.assertEqual(os.readlink(root / "current"), selected)
        self.assertEqual((old / "complete.json").read_bytes(), old_receipt)
        with state.OwnedRoot(root) as owner:
            self.assertEqual(owner.completed(old_manifest, identity), old)
            try:
                reusable = owner.completed(new_manifest, identity)
            except (OSError, ValueError) as error:
                self.fail("interrupted completion must be ignored on retry: " + str(error))
            self.assertIsNone(reusable, "an interrupted receipt became eligible for reuse")
            fresh = self.stage(owner, "retry-control", new_manifest)
            owner.complete(fresh, new_manifest, identity)
            self.assertEqual(owner.completed(new_manifest, identity), fresh)
            owner.activate(fresh)
            self.assertEqual((root / "current").resolve(strict=True), fresh)
            self.assertEqual((old / "complete.json").read_bytes(), old_receipt)

    def test_should_retry_after_a_real_partial_completion_write(self) -> None:
        """An EFBIG failure cannot leave a final receipt that poisons later reuse checks."""
        self.interruption("error")

    def test_should_retry_after_process_death_during_completion_write(self) -> None:
        """SIGXFSZ releases the root lock and leaves no eligible partial completion."""
        self.interruption("death")

    def test_should_reuse_normal_completion_and_reject_later_receipt_corruption(self) -> None:
        """Ignoring interrupted publication must not weaken genuinely completed integrity."""
        manifest = encode(make_manifest())
        with state.OwnedRoot(self.root / "installation") as owner:
            directory = self.completed_control(owner, manifest)
            receipt = directory / "complete.json"
            receipt.write_bytes(receipt.read_bytes()[:128])
            with self.assertRaises((OSError, ValueError)):
                owner.completed(manifest, state.interpreter_identity("cp310"))
            with self.assertRaises((OSError, ValueError)):
                owner.activate(directory)


class NativeReviewWorkspaceTests(ScratchCase):
    """Cleanup authority follows owned directory identity, not a reusable pathname."""

    def test_should_preserve_replacement_workspace_after_root_substitution(self) -> None:
        """Root validation failure never authorizes deleting the replacement tree."""
        selected = self.root / "installation"
        moved = self.root / "original-owned-root"
        with state.OwnedRoot(selected) as owner:
            with owner.work() as workspace:
                (workspace / "owned-partial").write_bytes(b"original run scratch")
                relative = workspace.relative_to(selected)
                selected.rename(moved)
                selected.mkdir(mode=0o700)
                replacement = selected / relative
                replacement.mkdir(mode=0o700, parents=True)
                canary = replacement / "replacement-canary"
                canary.write_bytes(b"not this run's workspace\n")
                with self.assertRaises((OSError, ValueError)):
                    owner.validate()
        self.assertTrue(canary.is_file(), "cleanup deleted a substituted root's workspace")
        self.assertEqual(canary.read_bytes(), b"not this run's workspace\n")

    def test_should_preserve_replacement_when_only_workspace_is_substituted(self) -> None:
        """An unchanged root does not prove that its workspace entry is still owned."""
        with state.OwnedRoot(self.root / "installation") as owner:
            with owner.work() as workspace:
                (workspace / "owned-partial").write_bytes(b"original run scratch")
                saved = workspace.with_name(workspace.name + "-original")
                workspace.rename(saved)
                workspace.mkdir(mode=0o700)
                canary = workspace / "replacement-canary"
                canary.write_bytes(b"replacement must survive\n")
            self.assertTrue(canary.is_file(), "cleanup deleted a different workspace inode")
            self.assertEqual(canary.read_bytes(), b"replacement must survive\n")

    def test_should_clean_verified_workspace_on_normal_and_exceptional_exit(self) -> None:
        """Ordinary cleanup still removes this run's scratch and preserves unrelated bytes."""
        with state.OwnedRoot(self.root / "installation") as owner:
            retained = owner.root / "retained-user-file"
            retained.write_bytes(b"preserve\n")
            with owner.work() as workspace:
                normal = workspace
                (workspace / "partial").write_bytes(b"scratch")
            self.assertFalse(normal.exists())
            with self.assertRaisesRegex(RuntimeError, "owned test failure"):
                with owner.work() as workspace:
                    exceptional = workspace
                    (workspace / "partial").write_bytes(b"scratch")
                    raise RuntimeError("owned test failure")
            self.assertFalse(exceptional.exists())
            self.assertEqual(retained.read_bytes(), b"preserve\n")
