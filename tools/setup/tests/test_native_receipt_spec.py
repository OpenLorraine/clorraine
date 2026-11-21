"""Receipt cleanup must preserve replacement-root data after validation fails."""

from __future__ import annotations

import os
from pathlib import Path
import stat
from unittest.mock import patch

from native_manifest_data import encode, make_manifest
from native_manifest_producer_fixture import ScratchCase
import native_review_state_spec as recovery


class NativeReceiptCleanupTests(ScratchCase):
    """Use passive real filesystem objects, not simulated installed-runtime success."""

    def stage(self, owner, name: str, manifest: bytes) -> Path:
        """Reuse the established passive candidate fixture without altering its expectations."""
        return recovery.NativeReviewCompletionTests.stage(self, owner, name, manifest)

    def completed_control(self, owner, manifest: bytes) -> Path:
        """Create an ordinary mechanical completion control through the existing fixture."""
        return recovery.NativeReviewCompletionTests.completed_control(self, owner, manifest)

    def test_should_preserve_replacement_receipt_after_root_changes_during_flush(self) -> None:
        """A completed file flush does not authorise deleting through a replaced root path."""
        selected = self.root / "installation"
        moved = self.root / "original-owned-root"
        metadata = make_manifest()
        old_manifest = encode(metadata)
        metadata["release"] = "v1.2.0-r2"
        manifest = encode(metadata)
        identity = recovery.state.interpreter_identity("cp310")
        observations = []
        real_fsync = os.fsync
        with recovery.state.OwnedRoot(selected) as owner:
            old = self.completed_control(owner, old_manifest)
            old_relative = old.relative_to(selected)
            old_receipt = (old / "complete.json").read_bytes()
            old_pointer = os.readlink(selected / "current")
            candidate = self.stage(owner, "receipt-candidate", manifest)
            candidate_relative = candidate.relative_to(selected)
            before = {(item.stat().st_dev, item.stat().st_ino)
                      for item in candidate.iterdir() if item.is_file()}

            def observe_flush(descriptor: int) -> None:
                """Forward real fsync, then replace only this test's root after a new file flush."""
                real_fsync(descriptor)
                opened = os.fstat(descriptor)
                key = (opened.st_dev, opened.st_ino)
                if observations or not stat.S_ISREG(opened.st_mode) or key in before:
                    return
                # Identify the newly flushed file by its actual inode rather
                # than assuming the installer's private temporary-name format.
                matches = [item for item in candidate.iterdir()
                           if (item.lstat().st_dev, item.lstat().st_ino) == key]
                self.assertEqual(len(matches), 1, "flush must name a new candidate file")
                pending = matches[0]
                relative = pending.relative_to(selected)
                flushed = pending.read_bytes()
                self.assertTrue(flushed, "observer must run after actual file bytes are flushed")
                selected.rename(moved)
                selected.mkdir(mode=0o700)
                canary = selected / relative
                canary.parent.mkdir(mode=0o700, parents=True)
                canary.write_bytes(b"replacement data does not belong to this installation\n")
                observations.append((relative, flushed, canary))

            with patch.object(os, "fsync", side_effect=observe_flush):
                with self.assertRaises((OSError, ValueError)):
                    owner.complete(candidate, manifest, identity)

        self.assertEqual(len(observations), 1, "receipt flush boundary was not reached")
        relative, flushed, canary = observations[0]
        self.assertTrue(canary.is_file(), "failure cleanup deleted replacement-root data")
        self.assertEqual(canary.read_bytes(),
                         b"replacement data does not belong to this installation\n")
        self.assertEqual((moved / relative).read_bytes(), flushed)
        self.assertFalse((selected / "current").exists())
        self.assertFalse((moved / candidate_relative / "complete.json").exists())
        self.assertEqual(os.readlink(moved / "current"), old_pointer)
        self.assertEqual((moved / old_relative / "complete.json").read_bytes(), old_receipt)
        with recovery.state.OwnedRoot(moved) as owner:
            self.assertIsNone(owner.completed(manifest, identity),
                              "retained incomplete receipt became eligible for reuse")
            fresh = self.stage(owner, "retry-after-root-change", manifest)
            owner.complete(fresh, manifest, identity)
            self.assertEqual(owner.completed(manifest, identity), fresh)
            owner.activate(fresh)
            self.assertEqual((moved / "current").resolve(strict=True), fresh)
            self.assertEqual((moved / old_relative / "complete.json").read_bytes(), old_receipt)

    def test_should_publish_normal_completion_without_a_leftover_temporary_entry(self) -> None:
        """Successful publication consumes its private filename and remains reusable."""
        manifest = encode(make_manifest())
        identity = recovery.state.interpreter_identity("cp310")
        with recovery.state.OwnedRoot(self.root / "installation") as owner:
            candidate = self.stage(owner, "ordinary-receipt", manifest)
            before = {item.name for item in candidate.iterdir()}
            owner.complete(candidate, manifest, identity)
            self.assertEqual({item.name for item in candidate.iterdir()},
                             before | {"complete.json"})
            self.assertEqual(owner.completed(manifest, identity), candidate)
            owner.activate(candidate)
            self.assertEqual((owner.root / "current").resolve(strict=True), candidate)

    def test_should_retry_after_known_interrupted_completion_without_editing_old_state(self) -> None:
        """Retaining unselected files must preserve the established partial-write recovery contract."""
        recovery.NativeReviewCompletionTests.interruption(self, "error")
