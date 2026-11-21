"""Explicit supported-cohort install/update, ABI transition and foreign-root proofs.

Missing real contexts are errors. This module is not imported into generic
local discovery, and no successful runtime is replaced by a test process.
"""

from __future__ import annotations

import json
import os
import re
import shutil
import stat
import tarfile
import tempfile
import unittest
import uuid
from pathlib import Path

from native_install_activation_spec import ActivationCases
from native_install_actual_fixture import (
    ActualInstallCase, assert_working, make_shell, private_directory, retained_manifest, run_hellos,
)
from native_install_context import load_candidate, ordinary_path, overlap, read_json
from native_install_fixture import tree_state
from native_install_negative_spec import UpdateFailureCases
from native_manifest_data import file_digest


class NativeInstallActualAcceptance(UpdateFailureCases, ActivationCases, ActualInstallCase):
    """Mandatory real initial/update and targeted negative proofs on both supported cohorts."""

    def test_should_install_all_profiles_and_revalidate_same_release_without_duplicate_state(self) -> None:
        """The HOME default works; a same-release rerun retains its selection and settings."""
        if shutil.which("fs-uae", path="/usr/bin:/bin") is not None:
            raise RuntimeError("initial compiler/host/vamos proof requires FS-UAE absent")
        first, _, root, old, snapshot, settings = self.baseline(default_root=True)
        self.assertEqual(root, first.home / ".clorraine")
        self.assertEqual(first.events("helper-execution")[0]["argv"][2:],
                         ["--release", self.initial.manifest["release"]])
        before_targets = {path.name for path in (root / "installations").iterdir()}
        before_pointer = os.readlink(root / "current")
        result = first.invoke(["--release", self.initial.manifest["release"]])
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(os.readlink(root / "current"), before_pointer)
        self.assertEqual({path.name for path in (root / "installations").iterdir()}, before_targets)
        self.assertEqual(tree_state(old), snapshot)
        self.assertEqual(settings.read_bytes(), b'{"preserve":"user configuration"}\n')
        assert_working(self, first, root, self.initial)
        self.assertTrue(any(line.startswith("export PATH=") for line in result.stdout.splitlines()))
        self.assertTrue(any(line.startswith("code --install-extension ")
                            for line in result.stdout.splitlines()))
        self.assertFalse([item for item in first.events("command")
                          if item["name"] in ("code", "sudo", "apt", "apt-get")])

    def test_should_activate_revised_support_retain_old_bytes_and_relocate_the_whole_root(self) -> None:
        """A support-only update uses the same real core and keeps old runnable versions."""
        first, second, root, old, snapshot, settings = self.baseline()
        old_binaries = {name: file_digest(old / "bin" / name)
                        for name in ("clc", "clorraine", "clorraine-lsp")}
        result = second.invoke(["--release", self.update.manifest["release"]], root=root)
        self.assertEqual(result.returncode, 0, result.stderr)
        new = assert_working(self, second, root, self.update)
        self.assertNotEqual(new, old)
        self.assertEqual({name: file_digest(new / "bin" / name) for name in old_binaries},
                         old_binaries)
        self.assertEqual(tree_state(old), snapshot)
        self.assertEqual(settings.read_bytes(), b'{"preserve":"user configuration"}\n')
        run_hellos(self, old / "bin/clorraine", first.root, first.environment)
        moved = self.initial_work / "relocated whole installation ; literal"
        root.rename(moved)
        self.assertFalse(root.exists(), "original prefix must not remain available")
        assert_working(self, second, moved, self.update)
        self.assertEqual(tree_state(moved / old.relative_to(root)), snapshot)
        self.assertEqual((moved / settings.relative_to(root)).read_bytes(),
                         b'{"preserve":"user configuration"}\n')


class NativeInstallAbiTransitionAcceptance(unittest.TestCase):
    """Explicit Ubuntu24 repair of a parent-owned disposable real Ubuntu22 seed."""

    def test_should_create_a_fresh_cp312_environment_preserving_cp310_bytes_and_settings(self) -> None:
        """The prior venv is retained, never mutated to run with an incompatible Python."""
        update = load_candidate("CLORRAINE_INSTALL_UPDATE_ACCEPTANCE", "v1.2.0-r2")
        if update.ubuntu != "24.04" or update.abi != "cp312":
            raise RuntimeError("ABI transition proof requires real Ubuntu24 and CPython3.12")
        selected = os.environ.get("CLORRAINE_INSTALL_PREVIOUS_ROOT")
        if not selected:
            raise RuntimeError("CLORRAINE_INSTALL_PREVIOUS_ROOT is required for the real cp310 seed")
        seed = ordinary_path(selected, directory=True)
        if (seed.stat().st_uid != os.geteuid() or overlap(seed, update.work_root) or
                seed.stat().st_dev != update.work_root.stat().st_dev):
            raise ValueError("seed must be owned, outside work_root and on the installation filesystem")
        current = seed / "current"
        self.assertTrue(current.is_symlink())
        old = current.resolve(strict=True)
        self.assertTrue(old.is_relative_to(seed / "installations"))
        descriptor = read_json(old / "installation.json")
        self.assertEqual(set(descriptor), {"schema", "core_version", "support_lock_sha256",
                                          "ndk_sha256", "prepared_inventory_sha256", "python_abi"})
        self.assertIs(type(descriptor["schema"]), int)
        self.assertEqual(descriptor["schema"], 1)
        self.assertEqual(descriptor["python_abi"], "cp310")
        self.assertEqual(descriptor["core_version"], update.manifest["core_version"])
        for name in ("support_lock_sha256", "ndk_sha256", "prepared_inventory_sha256"):
            self.assertRegex(descriptor[name], "^[0-9a-f]{64}$")
        previous, _ = retained_manifest(old)
        self.assertEqual(previous["release"], "v1.2.0-r1")
        old_core = [item for item in previous["assets"] if item["kind"] == "core"]
        self.assertEqual(len(old_core), 1)
        core = update.asset("core")
        self.assertEqual((old_core[0]["identity"], old_core[0]["sha256"]),
                         (core["identity"], core["sha256"]))
        with tarfile.open(update.payloads[core["identity"]], "r:") as archive:
            core_metadata = json.loads(archive.extractfile("core.json").read())
        for name in ("clc", "clorraine", "clorraine-lsp"):
            self.assertEqual(file_digest(old / "bin" / name),
                             core_metadata["files"]["bin/" + name]["sha256"])
        # One cleanup owns both restoration and scratch removal. Separate
        # TemporaryDirectory cleanup would still run after a restore error and
        # could delete the borrowed seed while it was inside that directory.
        private = Path(tempfile.mkdtemp(prefix="real-abi-transition-", dir=update.work_root))
        moved = private / "relocated ABI root ; literal"

        def restore_parent_seed() -> None:
            """Restore the borrowed seed or leave it intact at a named recovery path."""
            if moved.exists():
                if seed.exists():
                    raise RuntimeError("cannot restore seed over an unexpected directory; retained at " +
                                       str(moved))
                moved.rename(seed)

        def cleanup_private_work() -> None:
            """Never remove scratch until the borrowed seed is safely outside it."""
            restore_parent_seed()
            shutil.rmtree(private)

        self.addCleanup(cleanup_private_work)
        shell = make_shell(private, "update-shell", update)
        run_hellos(self, old / "bin/clorraine", shell.root, shell.environment, profiles=("host",))
        snapshot = tree_state(old)
        settings = seed / ("user-settings-" + uuid.uuid4().hex + ".json")
        with settings.open("xb") as output:
            output.write(b'{"preserve":"cp310 user settings"}\n')
        result = shell.invoke(["--release", update.manifest["release"]], root=seed)
        self.assertEqual(result.returncode, 0, result.stderr)
        new = assert_working(self, shell, seed, update)
        self.assertNotEqual(new, old)
        self.assertEqual(read_json(new / "installation.json")["python_abi"], "cp312")
        self.assertFalse((new / "sdk/vamos").is_symlink())
        self.assertFalse(os.path.samefile(old / "sdk/vamos", new / "sdk/vamos"))
        self.assertEqual(tree_state(old), snapshot)
        self.assertEqual(settings.read_bytes(), b'{"preserve":"cp310 user settings"}\n')
        seed.rename(moved)
        self.assertFalse(seed.exists())
        assert_working(self, shell, moved, update)
        self.assertEqual(tree_state(moved / old.relative_to(seed)), snapshot)
        self.assertEqual((moved / settings.relative_to(seed)).read_bytes(),
                         b'{"preserve":"cp310 user settings"}\n')
        restore_parent_seed()
        self.assertTrue(seed.is_dir(), "parent retains ownership of eventual seed cleanup")


def foreign_tmpfs(value: str) -> Path:
    """Require the dedicated empty, writable foreign-UID tmpfs, not a system directory."""
    root = ordinary_path(value, directory=True)
    metadata = root.stat()
    if (os.geteuid() in (0, 65534) or metadata.st_uid != 65534 or metadata.st_gid != 65534 or
            stat.S_IMODE(metadata.st_mode) != 0o1777 or any(root.iterdir())):
        raise ValueError("foreign root must be fresh UID/GID65534 mode1777 tmpfs")
    mounts = []
    for line in Path("/proc/self/mountinfo").read_text().splitlines():
        fields, _, filesystem = line.partition(" - ")
        values = fields.split()
        mountpoint = re.sub(r"\\([0-7]{3})", lambda match: chr(int(match[1], 8)), values[4])
        if Path(mountpoint).resolve() == root:
            mounts.append(filesystem.split()[0])
    if mounts != ["tmpfs"] or not os.path.ismount(root) or not os.access(root, os.W_OK | os.X_OK):
        raise RuntimeError("foreign root requires a dedicated writable tmpfs mount")
    probe = root / ("owned-write-probe-" + uuid.uuid4().hex)
    created = False
    try:
        with probe.open("xb") as output:
            created = True
            output.write(b"own probe only\n")
        if probe.stat().st_uid != os.geteuid():
            raise RuntimeError("foreign-root probe child is not owned by this process")
    finally:
        if created:
            probe.unlink()
    if any(root.iterdir()):
        raise RuntimeError("foreign-root fixture did not remain empty after its own write probe")
    return root


class NativeInstallForeignRootAcceptance(unittest.TestCase):
    """Mandatory actual rejection of writable foreign ownership without privileged test code."""

    def test_should_refuse_a_writable_foreign_owned_empty_root_without_creating_state(self) -> None:
        """Ownership rejection must not pass merely because ordinary writes were impossible."""
        candidate = load_candidate("CLORRAINE_INSTALL_ACCEPTANCE", "v1.2.0-r1")
        selected = os.environ.get("CLORRAINE_INSTALL_FOREIGN_ROOT")
        if not selected:
            raise RuntimeError("CLORRAINE_INSTALL_FOREIGN_ROOT is required for foreign-owner proof")
        root = foreign_tmpfs(selected)
        if overlap(root, candidate.work_root):
            raise ValueError("foreign root and owned actual work_root must be disjoint")
        private = private_directory(self, candidate.work_root, "real-foreign-root-")
        canary = private / "owned-outside-canary"
        canary.write_bytes(b"ordinary owned work must remain unchanged\n")
        shell = make_shell(private, "foreign-shell", candidate)
        result = shell.invoke(["--release", candidate.manifest["release"]], root=root)
        shell.assert_failed_safely(self, result)
        self.assertTrue(any(word in result.stderr.lower() for word in
                            ("own", "uid", "user", "account")))
        self.assertEqual(list(root.iterdir()), [], "installer wrote into a foreign-owned root")
        self.assertEqual(canary.read_bytes(), b"ordinary owned work must remain unchanged\n")
        self.assertFalse(any(line.startswith(("export PATH=", "code --install-extension "))
                             for line in result.stdout.splitlines()))
