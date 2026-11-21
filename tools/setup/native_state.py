"""Owned-root locking and immutable component storage for native installation."""

from __future__ import annotations

from contextlib import contextmanager
import ctypes
import fcntl
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import stat
import sys
import tempfile
import uuid

from tools.distribution.manifest import MAX_MANIFEST_BYTES
from .native_archive import ordinary, read_json, verify_file


_OWNER = ".clorraine-root-v1"
"""Empty ownership-marker directory, created in one atomic filesystem operation."""
_TOOLS = ("clc", "clorraine", "clorraine-lsp")
"""Stable executable entries that must always follow the selected installation."""
_MANAGED = ("bin", "components", "installations")
"""Real, user-owned managed directories; none may redirect into another tree."""
_NAME = re.compile(r"[a-z0-9][a-z0-9._-]*")
"""Installer-generated directory identifiers, never paths."""
_COMPLETE = "complete.json"
"""Separate smoke-tested installation receipt; installation.json is only assembly readiness."""


def encoded(value: object) -> bytes:
    """Encode deterministic metadata without accepting nonfinite JSON numbers."""
    return (json.dumps(value, sort_keys=True, separators=(",", ":"),
                       ensure_ascii=False, allow_nan=False) + "\n").encode("utf-8")


def durable_file(path: Path, data: bytes) -> None:
    """Create and flush a new private ordinary file without replacing an existing entry."""
    descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
    with os.fdopen(descriptor, "wb") as output:
        output.write(data)
        output.flush()
        os.fsync(output.fileno())


def _sync_directory(path: Path) -> None:
    """Flush directory entries without following a substituted directory alias."""
    descriptor = os.open(path, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC)
    try:
        os.fsync(descriptor)
    finally:
        os.close(descriptor)


def publish_directory(source: Path, destination: Path) -> None:
    """Atomically publish a fresh Linux entry without clobbering even an empty directory.

    A plain rename may replace an existing empty directory. The supported
    Ubuntu libc exposes renameat2, whose no-replace flag closes that race.
    The same operation publishes a fully flushed ordinary completion file.
    """
    library = ctypes.CDLL(None, use_errno=True)
    rename = library.renameat2
    rename.argtypes = [ctypes.c_int, ctypes.c_char_p, ctypes.c_int, ctypes.c_char_p, ctypes.c_uint]
    rename.restype = ctypes.c_int
    if rename(-100, os.fsencode(source), -100, os.fsencode(destination), 1) != 0:
        code = ctypes.get_errno()
        raise OSError(code, os.strerror(code), str(destination))


def _owned_directory(metadata) -> bool:
    """Require a usable directory that another account/group cannot modify."""
    return (stat.S_ISDIR(metadata.st_mode) and metadata.st_uid == os.geteuid()
            and metadata.st_mode & 0o700 == 0o700 and not metadata.st_mode & 0o022)


def _clear_workspace(descriptor: int) -> None:
    """Remove children through a verified workspace descriptor, never its absolute alias.

    Python 3.10's rmtree has no dir_fd argument. Descriptor-relative traversal
    keeps cleanup inside the original workspace even if its root is renamed.
    Never follow child links or continue through a replaced directory inode.
    """
    for name in os.listdir(descriptor):
        metadata = os.stat(name, dir_fd=descriptor, follow_symlinks=False)
        if stat.S_ISDIR(metadata.st_mode):
            child = os.open(name, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC,
                            dir_fd=descriptor)
            try:
                opened = os.fstat(child)
                if (opened.st_dev, opened.st_ino) != (metadata.st_dev, metadata.st_ino):
                    raise ValueError("workspace: child directory changed during cleanup")
                _clear_workspace(child)
                current = os.stat(name, dir_fd=descriptor, follow_symlinks=False)
                if (current.st_dev, current.st_ino) != (opened.st_dev, opened.st_ino):
                    raise ValueError("workspace: child directory was replaced during cleanup")
                os.rmdir(name, dir_fd=descriptor)
            finally:
                os.close(child)
        else:
            os.unlink(name, dir_fd=descriptor)


def interpreter_identity(abi: str) -> dict:
    """Record the actual system interpreter, not a mutable environment's launcher spelling."""
    python = Path(sys.executable).resolve()
    digest = hashlib.sha256()
    with ordinary(python) as source:
        for block in iter(lambda: source.read(65536), b""):
            digest.update(block)
    return {"abi": abi, "executable": str(python), "sha256": digest.hexdigest(),
            "version": sys.version}


def _installation_inventory(directory: Path, *, flush: bool = False) -> dict:
    """Measure every retained file/link, forbidding unexpected external aliases.

    A venv legitimately points its Python executables at the system interpreter.
    All other links must remain within this installation after root relocation.
    The completion record is excluded because it authenticates this inventory.
    """
    inventory = {}
    for parent, directories, files in os.walk(directory, followlinks=False):
        for name in [*directories, *files]:
            path = Path(parent) / name
            relative = path.relative_to(directory).as_posix()
            if relative == _COMPLETE:
                continue
            metadata = path.lstat()
            if metadata.st_uid != os.geteuid():
                raise ValueError("installation: foreign-owned retained member")
            if stat.S_ISLNK(metadata.st_mode):
                target = os.readlink(path)
                resolved = path.resolve(strict=True)
                internal = resolved == directory or directory in resolved.parents
                if not internal and not (
                        relative.startswith("sdk/vamos/bin/python")
                        and resolved == Path(sys.executable).resolve()):
                    raise ValueError("installation: unexpected external symbolic link")
                inventory[relative] = {"type": "symlink", "target": target}
            elif stat.S_ISREG(metadata.st_mode):
                digest = hashlib.sha256()
                with ordinary(path) as source:
                    for block in iter(lambda: source.read(65536), b""):
                        digest.update(block)
                    if flush:
                        os.fsync(source.fileno())
                inventory[relative] = {"type": "file", "bytes": metadata.st_size,
                                       "mode": stat.S_IMODE(metadata.st_mode),
                                       "sha256": digest.hexdigest()}
            elif not stat.S_ISDIR(metadata.st_mode):
                raise ValueError("installation: unexpected special file")
        if flush:
            _sync_directory(Path(parent))
    return inventory


def _verified_completion(directory: Path, record: dict | None = None,
                         expected_manifest: bytes | None = None) -> dict:
    """Revalidate the durable receipt and exact retained bytes before selection.

    Reuse and final activation share this check: a descriptor, empty receipt,
    alias or plausible filename must never be sufficient to select a target.
    Read the manifest without following its final entry or allocating from an
    unchecked local-state file length. Reuse additionally supplies the selected
    document bytes, which must match literally as well as by digest.
    """
    if record is None:
        record = read_json(directory / _COMPLETE)
    if (set(record) != {"schema", "manifest_sha256", "interpreter", "inventory"}
            or type(record["schema"]) is not int or record["schema"] != 1
            or not isinstance(record["inventory"], dict)
            or not isinstance(record["interpreter"], dict)
            or not isinstance(record["manifest_sha256"], str)
            or re.fullmatch("[0-9a-f]{64}", record["manifest_sha256"]) is None):
        raise ValueError("installation: invalid completed-installation receipt")
    identity = record["interpreter"]
    if (set(identity) != {"abi", "executable", "sha256", "version"}
            or not all(isinstance(value, str) and value for value in identity.values())
            or identity["abi"] not in ("cp310", "cp312")
            or not Path(identity["executable"]).is_absolute()
            or re.fullmatch("[0-9a-f]{64}", identity["sha256"]) is None):
        raise ValueError("installation: invalid completed interpreter identity")
    with ordinary(directory / "manifest.json") as source:
        if os.fstat(source.fileno()).st_size > MAX_MANIFEST_BYTES:
            raise ValueError("installation: retained manifest exceeds the safe bound")
        manifest = source.read(MAX_MANIFEST_BYTES + 1)
    if (len(manifest) > MAX_MANIFEST_BYTES
            or (expected_manifest is not None and manifest != expected_manifest)
            or hashlib.sha256(manifest).hexdigest() != record["manifest_sha256"]
            or record["inventory"] != _installation_inventory(directory)):
        raise ValueError("installation: completed state failed revalidation")
    return record


class OwnedRoot:
    """Hold one nonblocking OS lock for a canonical ordinary-user installation root.

    The lock is on the directory inode itself, so it needs no stale lock-file
    heuristic and survives alternate path spellings. Closing the descriptor or
    process death releases it; executable children do not inherit it.
    """

    def __init__(self, root: Path) -> None:
        """Retain a canonical selection without touching unrelated existing contents."""
        self.root = root.resolve()
        self._descriptor = None

    def __enter__(self):
        """Validate ownership, acquire the real lock, then initialize only an empty root."""
        if os.geteuid() == 0:
            raise ValueError("root: run the user installer without sudo")
        self.root.mkdir(mode=0o700, parents=True, exist_ok=True)
        descriptor = os.open(self.root, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC)
        try:
            metadata = os.fstat(descriptor)
            if not stat.S_ISDIR(metadata.st_mode) or metadata.st_uid != os.geteuid():
                raise ValueError("root: use a private directory owned by your user account")
            try:
                fcntl.flock(descriptor, fcntl.LOCK_EX | fcntl.LOCK_NB)
            except BlockingIOError as error:
                raise ValueError("root lock: another installer is running for this root") from error
            self._descriptor = descriptor
            # A caller may create an empty destination under a group-writable
            # umask. Secure that empty owned directory only after exclusion;
            # never chmod an unrelated nonempty or foreign-owned destination.
            if not os.listdir(descriptor):
                os.fchmod(descriptor, 0o700)
            self._check_root()
            entries = set(os.listdir(descriptor))
            if not entries:
                # An empty directory is the entire marker. Creation is atomic:
                # interruption cannot leave a half-written ownership JSON file.
                os.mkdir(_OWNER, 0o700, dir_fd=descriptor)
                os.fsync(descriptor)
            elif _OWNER not in entries:
                raise ValueError("root: refusing an unrelated nonempty destination")
            marker = self.root / _OWNER
            metadata = marker.lstat()
            if (not _owned_directory(metadata) or stat.S_IMODE(metadata.st_mode) != 0o700
                    or any(marker.iterdir())):
                raise ValueError("root: invalid installer ownership marker")
            self.validate()
            for name in _MANAGED:
                with self._directory(name, create=True):
                    pass
            return self
        except BaseException:
            self._descriptor = None
            os.close(descriptor)
            raise

    def __exit__(self, *_exception) -> None:
        """Release only this invocation's directory lock and descriptor."""
        if self._descriptor is not None:
            os.close(self._descriptor)
            self._descriptor = None

    def _check_root(self) -> None:
        """Detect replacement of the selected root instead of following its new spelling."""
        if self._descriptor is None:
            raise ValueError("root: installation lock is not held")
        opened, current = os.fstat(self._descriptor), self.root.lstat()
        if ((opened.st_dev, opened.st_ino) != (current.st_dev, current.st_ino)
                or not _owned_directory(current)):
            raise ValueError("root: selected directory was replaced or ownership changed")

    @contextmanager
    def _directory(self, name: str, *, create: bool = False):
        """Open a fixed managed child through the locked root descriptor without aliases."""
        self._check_root()
        if name not in _MANAGED:
            raise ValueError("root: invalid managed directory name")
        try:
            metadata = os.stat(name, dir_fd=self._descriptor, follow_symlinks=False)
        except FileNotFoundError:
            if not create:
                yield None
                return
            os.mkdir(name, 0o700, dir_fd=self._descriptor)
            metadata = os.stat(name, dir_fd=self._descriptor, follow_symlinks=False)
        if not _owned_directory(metadata):
            raise ValueError("root: managed directory is a symlink or not safely user-owned")
        descriptor = os.open(name, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC,
                             dir_fd=self._descriptor)
        try:
            opened = os.fstat(descriptor)
            if (opened.st_dev, opened.st_ino) != (metadata.st_dev, metadata.st_ino):
                raise ValueError("root: managed directory changed while opening it")
            yield descriptor
        finally:
            os.close(descriptor)

    def validate(self) -> None:
        """Reject managed path substitutions before downloads, reuse or publication."""
        self._check_root()
        for name in _MANAGED:
            with self._directory(name):
                pass
        current = self.root / "current"
        if os.path.lexists(current):
            metadata = current.lstat()
            if not stat.S_ISLNK(metadata.st_mode) or metadata.st_uid != os.geteuid():
                raise ValueError("root: current must be an installer-owned internal symlink")
            parts = Path(os.readlink(current)).parts
            if (len(parts) != 2 or parts[0] != "installations" or _NAME.fullmatch(parts[1]) is None
                    or not _owned_directory((self.root / parts[0] / parts[1]).lstat())):
                raise ValueError("root: current escapes or names an incomplete directory")
        with self._directory("bin") as descriptor:
            if descriptor is not None:
                for name in _TOOLS:
                    try:
                        metadata = os.stat(name, dir_fd=descriptor, follow_symlinks=False)
                    except FileNotFoundError:
                        continue
                    if (not stat.S_ISLNK(metadata.st_mode) or metadata.st_uid != os.geteuid()
                            or os.readlink(name, dir_fd=descriptor) != "../current/bin/" + name):
                        raise ValueError("root: unexpected file or escaping stable bin symlink")

    @contextmanager
    def work(self):
        """Create one private incomplete workspace and clean only that invocation's child."""
        self.validate()
        directory = Path(tempfile.mkdtemp(prefix=".incomplete-", dir=self.root))
        descriptor = os.open(directory, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC)
        try:
            identity = os.fstat(descriptor)
            self._check_root()
            yield directory
        finally:
            try:
                # An unverifiable name is not cleanup authority. Leave original
                # scratch for inspection rather than delete a replacement tree.
                try:
                    self._check_root()
                    current = os.stat(directory.name, dir_fd=self._descriptor,
                                      follow_symlinks=False)
                    unchanged = (identity.st_dev, identity.st_ino) == (current.st_dev, current.st_ino)
                except (OSError, ValueError):
                    unchanged = False
                if unchanged:
                    _clear_workspace(descriptor)
                    current = os.stat(directory.name, dir_fd=self._descriptor,
                                      follow_symlinks=False)
                    if (identity.st_dev, identity.st_ino) == (current.st_dev, current.st_ino):
                        os.rmdir(directory.name, dir_fd=self._descriptor)
            finally:
                os.close(descriptor)

    def input(self, asset: dict, work: Path, fetch) -> Path:
        """Revalidate a completed immutable component, or fetch a fresh private copy.

        A private snapshot is always used by consumers. Incomplete cache
        directories are ignored, never mistaken for authenticated components
        or erased to make room for an update.
        """
        self.validate()
        name = hashlib.sha256(asset["identity"].encode("ascii")).hexdigest()
        directory = self.root / "components" / name
        record = {
            "schema": 1, "identity": asset["identity"], "kind": asset["kind"],
            "bytes": asset["bytes"], "sha256": asset["sha256"],
        }
        snapshot = work / ("input-" + name)
        with self._directory("components") as descriptor:
            try:
                metadata = os.stat(name, dir_fd=descriptor, follow_symlinks=False)
            except FileNotFoundError:
                metadata = None
            if metadata is not None:
                if not _owned_directory(metadata):
                    raise ValueError("component cache: unsafe directory substitution")
                receipt = directory / "complete.json"
                if os.path.lexists(receipt):
                    stored = read_json(receipt)
                    if (type(stored.get("schema")) is not int
                            or type(stored.get("bytes")) is not int or stored != record):
                        raise ValueError("component cache: invalid receipt or changed immutable identity")
                    verify_file(directory / "payload", asset)
                    with (directory / "payload").open("rb") as source, snapshot.open("xb") as target:
                        shutil.copyfileobj(source, target, length=64 * 1024)
                    verify_file(snapshot, asset)
                    return snapshot
        fetch(asset, snapshot)
        verify_file(snapshot, asset)
        if metadata is not None:
            return snapshot
        # This stage is not reusable until payload and receipt are both durable.
        temporary = work / ("cache-" + uuid.uuid4().hex)
        temporary.mkdir(mode=0o700)
        with snapshot.open("rb") as source, (temporary / "payload").open("xb") as target:
            shutil.copyfileobj(source, target, length=64 * 1024)
            target.flush()
            os.fsync(target.fileno())
        verify_file(temporary / "payload", asset)
        durable_file(temporary / "complete.json", encoded(record))
        _sync_directory(temporary)
        self.validate()
        with self._directory("components"):
            publish_directory(temporary, directory)
        _sync_directory(self.root / "components")
        return snapshot

    def completed(self, manifest_bytes: bytes, interpreter: dict) -> Path | None:
        """Find a revalidated smoke-tested installation of these exact bytes/interpreter.

        Incomplete directories do not become eligible just because an assembly
        descriptor exists. Corrupt completed state fails instead of activating
        it or silently editing a previous version to repair the update.
        """
        self.validate()
        digest = hashlib.sha256(manifest_bytes).hexdigest()
        with self._directory("installations"):
            for directory in sorted((self.root / "installations").iterdir()):
                if not _owned_directory(directory.lstat()):
                    raise ValueError("installations: unsafe directory substitution")
                receipt = directory / _COMPLETE
                if not os.path.lexists(receipt):
                    continue
                record = read_json(receipt)
                if (record.get("manifest_sha256") != digest
                        or record.get("interpreter") != interpreter):
                    continue
                _verified_completion(directory, record, manifest_bytes)
                return directory
        return None

    def stage(self, candidate: Path, release: str, abi: str) -> Path:
        """Move assembled bytes to their permanent unique path before smoke/completion."""
        self.validate()
        name = release + "-" + abi + "-" + uuid.uuid4().hex
        if _NAME.fullmatch(name) is None:
            raise ValueError("installation: invalid generated version name")
        destination = self.root / "installations" / name
        with self._directory("installations"):
            publish_directory(candidate, destination)
        _sync_directory(destination.parent)
        return destination

    def complete(self, directory: Path, manifest_bytes: bytes, interpreter: dict) -> None:
        """Flush a separate completion receipt only after the caller's required smokes pass."""
        self.validate()
        if directory.parent != self.root / "installations" or not _owned_directory(directory.lstat()):
            raise ValueError("installation: completion target is not private owned staging")
        inventory = _installation_inventory(directory, flush=True)
        pending = directory / (".complete-" + uuid.uuid4().hex)
        durable_file(pending, encoded({
            "schema": 1, "manifest_sha256": hashlib.sha256(manifest_bytes).hexdigest(),
            "interpreter": interpreter, "inventory": inventory,
        }))
        self.validate()
        # Successful publication consumes the private name without replacement.
        # On failure, retain it in incomplete state: an absolute-path cleanup
        # could name unrelated data after a root change, and retry already
        # ignores versions without the final completion receipt.
        publish_directory(pending, directory / _COMPLETE)
        _sync_directory(directory)

    def activate(self, directory: Path) -> None:
        """Select a durable completed version through one same-filesystem atomic rename."""
        self.validate()
        if (directory.parent != self.root / "installations"
                or not _owned_directory(directory.lstat())
                or not (directory / _COMPLETE).is_file()):
            raise ValueError("activation: missing completed-installation state")
        _verified_completion(directory)
        # Stable links always follow current. Creating them before selection
        # cannot mix component versions in a running canonicalized driver.
        with self._directory("bin", create=True) as descriptor:
            for name in _TOOLS:
                try:
                    os.symlink("../current/bin/" + name, name, dir_fd=descriptor)
                except FileExistsError:
                    if os.readlink(name, dir_fd=descriptor) != "../current/bin/" + name:
                        raise ValueError("activation: unexpected stable executable entry")
            os.fsync(descriptor)
        target = "installations/" + directory.name
        if os.path.lexists(self.root / "current") and os.readlink(self.root / "current") == target:
            return
        pending = ".current-" + uuid.uuid4().hex
        os.symlink(target, pending, dir_fd=self._descriptor)
        try:
            os.fsync(self._descriptor)
            self.validate()
            os.replace(pending, "current", src_dir_fd=self._descriptor, dst_dir_fd=self._descriptor)
            os.fsync(self._descriptor)
        finally:
            try:
                os.unlink(pending, dir_fd=self._descriptor)
            except FileNotFoundError:
                pass
