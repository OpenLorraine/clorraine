"""Mechanical state, cache, lock and activation boundaries; not runtime acceptance."""

from __future__ import annotations

import fcntl
import hashlib
import json
import os
from pathlib import Path
import select
import signal
import stat
import subprocess
import sys
import tempfile
from types import SimpleNamespace
import unittest

from native_transport_fixture import limit_files


_ROOT = Path(__file__).resolve().parents[3]
"""Source location used by this implementation-test interpreter and its owned lock child."""
_ORIGINAL_PATH = sys.path[:]
try:
    sys.path.insert(0, str(_ROOT))
    from tools.setup import native_state as state
finally:
    sys.path[:] = _ORIGINAL_PATH

_LOCK_CHILD = """
import sys
from pathlib import Path
sys.path.insert(0, sys.argv[1])
from tools.setup.native_state import OwnedRoot
with OwnedRoot(Path(sys.argv[2])):
    print("ready", flush=True)
    sys.stdin.buffer.read(1)
"""
"""Own a real directory-inode flock until the parent terminates this specific session."""


class NativeStateImplTests(unittest.TestCase):
    """Use real private filesystem/process objects without inventing successful compilers."""

    def setUp(self) -> None:
        """Register cleanup for exactly this test's child directory before using it."""
        parent = Path(os.environ.get("CODEOPS_TMPDIR") or os.environ.get("TMPDIR")
                      or str(_ROOT / "build/tmp"))
        parent.mkdir(parents=True, exist_ok=True)
        self.temporary = tempfile.TemporaryDirectory(prefix="native-state-", dir=parent)
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)

    def asset(self, data: bytes = b"x") -> dict:
        """Describe a raw component cache input, never an assembled/runnable installation."""
        return {"identity": "unit-component", "kind": "support-lock",
                "bytes": len(data), "sha256": hashlib.sha256(data).hexdigest()}

    def cache_directory(self, owner: state.OwnedRoot) -> Path:
        """Inspect the single private component directory created by the cache implementation."""
        entries = list((owner.root / "components").iterdir())
        self.assertEqual(len(entries), 1)
        return entries[0]

    def test_should_require_private_owned_directory_metadata(self) -> None:
        """The pure ownership predicate does not substitute for actual foreign-root proof."""
        for mode, uid, expected in (
                (stat.S_IFDIR | 0o700, os.geteuid(), True),
                (stat.S_IFDIR | 0o777, os.geteuid(), False),
                (stat.S_IFDIR | 0o500, os.geteuid(), False),
                (stat.S_IFREG | 0o700, os.geteuid(), False),
                (stat.S_IFDIR | 0o700, os.geteuid() + 1, False)):
            with self.subTest(mode=mode, uid=uid):
                self.assertEqual(state._owned_directory(
                    SimpleNamespace(st_mode=mode, st_uid=uid)), expected)

    def test_should_release_real_alias_exclusion_after_owned_process_death(self) -> None:
        """Only the owned lock-holder session is killed; an arbitrary leftover file is inert."""
        selected = self.root / "selected root"
        alias = self.root / "same inode alias"
        alias.symlink_to(selected, target_is_directory=True)
        error_path = self.root / "lock-child.stderr"
        with error_path.open("wb") as errors:
            process = subprocess.Popen(
                [sys.executable, "-I", "-B", "-c", _LOCK_CHILD, str(_ROOT), str(selected)],
                stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=errors,
                start_new_session=True, preexec_fn=limit_files)
            try:
                ready, _, _ = select.select([process.stdout], [], [], 10)
                self.assertTrue(ready, "owned child did not reach the real lock")
                self.assertEqual(process.stdout.readline(), b"ready\n", error_path.read_text())
                (selected / "leftover-user-file").write_bytes(b"preserve this\n")
                with self.assertRaisesRegex(ValueError, "lock|running"):
                    with state.OwnedRoot(alias):
                        self.fail("a second descriptor acquired the held directory inode")
                os.killpg(process.pid, signal.SIGKILL)
                process.wait(timeout=5)
                with state.OwnedRoot(alias) as owner:
                    owner.validate()
                    self.assertEqual(owner.root, selected.resolve())
                    self.assertEqual((selected / "leftover-user-file").read_bytes(),
                                     b"preserve this\n")
            finally:
                if process.poll() is None:
                    os.killpg(process.pid, signal.SIGKILL)
                    process.wait(timeout=5)
                process.stdin.close()
                process.stdout.close()

    def test_should_mark_the_lock_descriptor_close_on_exec(self) -> None:
        """Compiler/interpreter children must not keep the parent's installation lock alive."""
        with state.OwnedRoot(self.root / "installation") as owner:
            self.assertIsNotNone(owner._descriptor)
            self.assertTrue(fcntl.fcntl(owner._descriptor, fcntl.F_GETFD) & fcntl.FD_CLOEXEC)

    def test_should_clean_only_its_workspace_when_an_operation_raises(self) -> None:
        """An exception removes this run's scratch, not user files or unrelated incomplete data."""
        with state.OwnedRoot(self.root / "installation") as owner:
            retained = owner.root / "retained-user-data"
            retained.write_bytes(b"unchanged\n")
            with self.assertRaisesRegex(RuntimeError, "deliberate"):
                with owner.work() as workspace:
                    created = workspace
                    (workspace / "partial").write_bytes(b"owned temporary bytes")
                    raise RuntimeError("deliberate")
            self.assertFalse(created.exists())
            self.assertEqual(retained.read_bytes(), b"unchanged\n")

    def test_should_never_treat_readiness_as_completion_or_activate_it(self) -> None:
        """Even a syntactically correct assembly descriptor does not certify passed smokes."""
        with state.OwnedRoot(self.root / "installation") as owner:
            incomplete = owner.root / "installations/incomplete"
            incomplete.mkdir(mode=0o700)
            (incomplete / "installation.json").write_bytes(state.encoded({
                "schema": 1, "core_version": "1.1.2", "python_abi": "cp310",
                "support_lock_sha256": "0" * 64, "ndk_sha256": "1" * 64,
                "prepared_inventory_sha256": "2" * 64}))
            self.assertIsNone(owner.completed(b'{"not":"accepted metadata"}',
                                              state.interpreter_identity("cp310")))
            with self.assertRaises((OSError, ValueError)):
                owner.activate(incomplete)
            self.assertFalse(os.path.lexists(owner.root / "current"))

    def test_should_reject_malformed_or_aliased_completion_before_stable_links(self) -> None:
        """A filename claiming completion is not proof of a durable verified installation."""
        for kind in ("empty-object", "plausible-but-false", "bad-json", "external-alias"):
            with self.subTest(kind=kind), state.OwnedRoot(self.root / kind) as owner:
                candidate = owner.root / "installations/incomplete"
                candidate.mkdir(mode=0o700)
                receipt = candidate / "complete.json"
                outside = self.root / (kind + "-outside")
                outside.write_bytes(b"{}\n")
                if kind == "external-alias":
                    receipt.symlink_to(outside)
                elif kind == "plausible-but-false":
                    receipt.write_bytes(state.encoded({
                        "schema": 1, "manifest_sha256": "0" * 64, "inventory": {},
                        "interpreter": {"abi": "cp310", "executable": sys.executable,
                                        "sha256": "1" * 64, "version": "fixture"}}))
                else:
                    receipt.write_bytes(b"{}\n" if kind == "empty-object" else b"{invalid")
                with self.assertRaises((OSError, ValueError)):
                    owner.activate(candidate)
                self.assertFalse(os.path.lexists(owner.root / "current"))
                self.assertFalse(list((owner.root / "bin").iterdir()))
                self.assertEqual(outside.read_bytes(), b"{}\n")

    def test_should_copy_cache_snapshots_without_refetching_or_hardlinking(self) -> None:
        """Preparation may change its private copy without mutating verified shared bytes."""
        fetched = []

        def fetch(asset, destination):
            """Provide deterministic bytes to the cache's real verification boundary."""
            fetched.append(asset["identity"])
            destination.write_bytes(b"x")

        with state.OwnedRoot(self.root / "installation") as owner:
            with owner.work() as work:
                snapshot = owner.input(self.asset(), work, fetch)
                payload = self.cache_directory(owner) / "payload"
                self.assertFalse(os.path.samefile(snapshot, payload))
                snapshot.write_bytes(b"changed private stage")
                self.assertEqual(payload.read_bytes(), b"x")
            with owner.work() as work:
                self.assertEqual(owner.input(self.asset(), work, fetch).read_bytes(), b"x")
        self.assertEqual(fetched, ["unit-component"])

    def test_should_refuse_changed_bytes_for_an_immutable_cached_identity(self) -> None:
        """A revision cannot silently retarget an existing component identity."""
        with state.OwnedRoot(self.root / "installation") as owner:
            with owner.work() as work:
                owner.input(self.asset(), work, lambda _asset, path: path.write_bytes(b"x"))
            with owner.work() as work, self.assertRaisesRegex(ValueError, "immutable|receipt"):
                owner.input(self.asset(b"y"), work,
                            lambda _asset, _path: self.fail("must not download a retargeted identity"))
            self.assertEqual((self.cache_directory(owner) / "payload").read_bytes(), b"x")

    def test_should_reject_boolean_integer_fields_in_cache_completion(self) -> None:
        """Python equality must not turn JSON true into schema one or byte length one."""
        for field in ("schema", "bytes"):
            with self.subTest(field=field), state.OwnedRoot(self.root / field) as owner:
                with owner.work() as work:
                    owner.input(self.asset(), work, lambda _asset, path: path.write_bytes(b"x"))
                receipt = self.cache_directory(owner) / "complete.json"
                record = json.loads(receipt.read_bytes())
                record[field] = True
                receipt.write_bytes(state.encoded(record))
                with owner.work() as work, self.assertRaisesRegex(ValueError, "receipt|immutable"):
                    owner.input(self.asset(), work,
                                lambda _asset, _path: self.fail("malformed state must not refetch"))

    def test_should_refuse_corrupt_cached_payload_without_overwriting_it(self) -> None:
        """Corrupt complete data fails revalidation rather than masquerading as a reusable input."""
        with state.OwnedRoot(self.root / "installation") as owner:
            with owner.work() as work:
                owner.input(self.asset(), work, lambda _asset, path: path.write_bytes(b"x"))
            directory = self.cache_directory(owner)
            original_receipt = (directory / "complete.json").read_bytes()
            (directory / "payload").write_bytes(b"y")
            with owner.work() as work, self.assertRaises(ValueError):
                owner.input(self.asset(), work,
                            lambda _asset, _path: self.fail("corrupt complete cache cannot refetch"))
            self.assertEqual((directory / "payload").read_bytes(), b"y")
            self.assertEqual((directory / "complete.json").read_bytes(), original_receipt)

    def test_should_ignore_incomplete_cache_without_erasing_its_contents(self) -> None:
        """A partial directory is not reusable; a verified fresh snapshot does not rewrite it."""
        asset = self.asset()
        with state.OwnedRoot(self.root / "installation") as owner:
            partial = owner.root / "components" / hashlib.sha256(
                asset["identity"].encode("ascii")).hexdigest()
            partial.mkdir(mode=0o700)
            (partial / "unrelated-partial").write_bytes(b"leave alone")
            for _attempt in range(2):
                with owner.work() as work:
                    self.assertEqual(owner.input(
                        asset, work, lambda _asset, path: path.write_bytes(b"x")).read_bytes(), b"x")
                self.assertEqual({path.name for path in partial.iterdir()}, {"unrelated-partial"})
                self.assertEqual((partial / "unrelated-partial").read_bytes(), b"leave alone")

    def test_should_publish_without_clobbering_any_existing_destination_type(self) -> None:
        """RENAME_NOREPLACE protects empty directories too, unlike ordinary rename."""
        for kind in ("file", "empty-directory", "alias"):
            with self.subTest(kind=kind):
                source = self.root / (kind + "-source")
                source.mkdir()
                (source / "data").write_bytes(b"new")
                destination = self.root / kind
                outside = self.root / (kind + "-outside")
                outside.write_bytes(b"preserve")
                if kind == "file":
                    destination.write_bytes(b"old")
                elif kind == "empty-directory":
                    destination.mkdir()
                else:
                    destination.symlink_to(outside)
                with self.assertRaises(OSError):
                    state.publish_directory(source, destination)
                self.assertEqual((source / "data").read_bytes(), b"new")
                self.assertEqual(outside.read_bytes(), b"preserve")
                if kind == "file":
                    self.assertEqual(destination.read_bytes(), b"old")
                elif kind == "empty-directory":
                    self.assertFalse(list(destination.iterdir()))
                else:
                    self.assertTrue(destination.is_symlink())

    def test_should_reject_special_files_and_external_links_in_installation_inventory(self) -> None:
        """Inventory inspection never follows an unexpected alias or waits on a FIFO."""
        for kind in ("fifo", "external-link"):
            with self.subTest(kind=kind):
                candidate = self.root / kind
                candidate.mkdir()
                outside = self.root / (kind + "-outside")
                outside.write_bytes(b"unchanged")
                if kind == "fifo":
                    os.mkfifo(candidate / "special")
                else:
                    (candidate / "outside").symlink_to(outside)
                with self.assertRaises(ValueError):
                    state._installation_inventory(candidate)
                self.assertEqual(outside.read_bytes(), b"unchanged")
