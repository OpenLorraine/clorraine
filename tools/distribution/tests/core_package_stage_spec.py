"""Mandatory public producer checks using actual current core/support inputs."""

from __future__ import annotations

import copy
import hashlib
import io
import os
import shutil
import sys
import tarfile
import tempfile
import unittest
from pathlib import Path

from core_package_artifacts import (
    ActualInputs, archive_file, assembled_layout, audit_inventory, required_environment, tree,
)
from core_package_fixtures import (
    BINARIES, current_version, digest_file, elf_x64, inspect_core, inventory,
    json_bytes, read_json, read_json_bytes, repository,
)


def rewrite_tar(source: Path, target: Path, replacements: dict[str, bytes] | None = None,
                extra: tuple[tarfile.TarInfo, bytes] | None = None) -> None:
    """Copy real archive members, changing only explicit negative-control bytes."""
    replacements = replacements or {}
    with tarfile.open(source, "r:*") as original, tarfile.open(
            target, "w", format=tarfile.PAX_FORMAT) as changed:
        for member in original:
            member = copy.copy(member)
            data = replacements.get(member.name)
            if data is not None:
                member.size = len(data)
                changed.addfile(member, io.BytesIO(data))
            else:
                changed.addfile(member, original.extractfile(member) if member.isfile() else None)
        if extra is not None:
            member, data = extra
            member.size = len(data)
            changed.addfile(member, io.BytesIO(data) if member.isfile() else None)


def mismatched_wheel_abi(source: Path, audit: Path, target: Path, target_audit: Path,
                         python_abi: str) -> None:
    """Write a declared-ABI negative control, not an accepted other-cohort wheel set.

    The source receipt must match the actual Python cohort. Change only its ABI
    declaration and the copied audit's hash for that metadata member. Every
    wheel byte, package record and lock binding stays unchanged, so an audit
    hash mismatch cannot explain rejection of the edited declaration.
    """
    receipt = read_json_bytes(archive_file(source, "wheels.json"))
    if python_abi not in ("cp310", "cp312") or receipt["abi"] != python_abi:
        raise AssertionError("source wheels must match the supported actual Python cohort")
    receipt["abi"] = "cp312" if python_abi == "cp310" else "cp310"
    changed = json_bytes(receipt)
    rewrite_tar(source, target, {"wheels.json": changed})
    changed_audit = read_json(audit)
    changed_audit["files"]["wheels.json"]["sha256"] = hashlib.sha256(changed).hexdigest()
    target_audit.write_bytes(json_bytes(changed_audit))


class CorePackageArtifactAcceptance(unittest.TestCase):
    """Require actual archive/stage inputs on each supported Python/Ubuntu cohort.

    CLORRAINE_CORE_BINARIES names the actual current three-binary directory.
    CLORRAINE_SUPPORT_ACCEPTANCE names the existing complete support input
    record. The producer checkout is available here because archive reads its
    own version/notices. Checkout-free runtime proof is a separate real group.
    """

    @classmethod
    def setUpClass(cls) -> None:
        """Check explicit actual inputs; absent data is an error, never a skip."""
        cls.inputs = ActualInputs()
        cls.addClassCleanup(cls.inputs.cleanup)
        cls.binaries = required_environment("CLORRAINE_CORE_BINARIES")
        if not cls.binaries.is_dir():
            raise AssertionError("CLORRAINE_CORE_BINARIES must name the actual binary directory")
        for name in BINARIES:
            elf_x64(cls.binaries / name)
        cls.version = current_version()
        cls.expected = inventory(cls.binaries, repository())
        cls.entry = repository() / "tools/distribution/package_core.py"
        if not cls.entry.is_file():
            raise AssertionError(f"Missing public command: {cls.entry}")
        cls.core = cls.inputs.root / "actual core.tar"
        result = cls.inputs.run([
            sys.executable, cls.entry, "archive", "--binaries", cls.binaries, "--output", cls.core])
        if result.returncode != 0:
            raise AssertionError(f"actual core archive failed: {result.stderr}")
        inspect_core(cls.core, cls.expected, cls.version)
        cls.core_identity = digest_file(cls.core)
        print(f"actual core sha256={cls.core_identity} version={cls.version}", flush=True)

    def setUp(self) -> None:
        """Use separate output parents so failed private siblings remain observable."""
        scratch = tempfile.TemporaryDirectory(prefix="core-stage-case-", dir=self.inputs.root)
        self.addCleanup(scratch.cleanup)
        self.root = Path(scratch.name)
        self.output_parent = self.root / "published"
        self.output_parent.mkdir()
        self.output = self.output_parent / "candidate ; literal"
        self.managed = Path(self.inputs.environment["HOME"]) / ".clorraine"
        if not self.managed.exists():
            (self.managed / "installations/old").mkdir(parents=True)
            (self.managed / "installations/old/keep").write_bytes(b"existing user state\n")
            (self.managed / "current").symlink_to("installations/old")

    def _stage(self, replacements: dict | None = None):
        """Call the exact public stage signature with genuine matching inputs."""
        inputs = self.inputs
        values = {
            "--core": self.core, "--lock": inputs.paths["lock"],
            "--sdk-archive": inputs.paths["amiga"], "--sdk-audit": inputs.paths["amiga_audit"],
            "--preparation-archive": inputs.paths["preparation"],
            "--preparation-audit": inputs.paths["preparation_audit"],
            "--wheels-archive": inputs.paths["wheels"], "--wheels-audit": inputs.paths["wheels_audit"],
            "--ndk": inputs.paths["ndk"], "--python": sys.executable, "--output": self.output,
        }
        values.update(replacements or {})
        arguments = [sys.executable, self.entry, "stage"]
        for name, value in values.items():
            arguments.extend([name, value])
        user_state = (tree(self.managed), sorted(
            path.relative_to(self.managed).as_posix()
            for path in self.managed.rglob("*") if path.is_dir() and not path.is_symlink()))
        result = inputs.run(arguments, self.root)
        self.assertEqual((tree(self.managed), sorted(
            path.relative_to(self.managed).as_posix()
            for path in self.managed.rglob("*") if path.is_dir() and not path.is_symlink())),
            user_state, "local staging changed existing user activation/state")
        inputs.unchanged()
        self.assertEqual(digest_file(self.core), self.core_identity)
        self.assertEqual(inventory(self.binaries, repository()), self.expected)
        return result

    def _failed(self, replacements: dict, identity: str) -> None:
        """Require a named content failure with no published or incomplete candidate."""
        result = self._stage(replacements)
        self.assertEqual(result.returncode, 1, result.stderr)
        named = identity.lower() in result.stderr.lower()
        if "--core" in replacements:
            named = named or "core" in result.stderr.lower()
        self.assertTrue(named, result.stderr)
        self.assertEqual(list(self.output_parent.iterdir()), [],
                         "failed staging published or abandoned a candidate sibling")

    def test_should_archive_actual_current_binaries_without_rebuilding(self) -> None:
        """Real core bytes/notices and the recorded development version survive packaging."""
        inspect_core(self.core, self.expected, self.version)
        self.assertEqual(digest_file(self.core), self.core_identity)

    def test_should_stage_real_prepared_sdk_and_matching_private_venv(self) -> None:
        """The published assembly has canonical adjacent tools and exact linked receipts."""
        result = self._stage()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn(str(self.output), result.stdout)
        assembled_layout(self.output, self.core, self.inputs)
        self.assertEqual(list(self.output_parent.iterdir()), [self.output])
        python = self.output / "sdk/vamos/bin/python"
        probe = self.inputs.run([python, "-I", "-c",
            "import importlib.metadata, json, site, sys\n"
            "import machine68k, greenlet\n"
            "names = ['amitools', 'machine68k', 'greenlet', 'pysdl2', 'pip']\n"
            "print(json.dumps({'prefix': sys.prefix, 'isolated': sys.flags.isolated,\n"
            " 'user_site': site.ENABLE_USER_SITE,\n"
            " 'versions': {name: importlib.metadata.version(name) for name in names}}))\n"])
        self.assertEqual(probe.returncode, 0, probe.stderr)
        observed = read_json_bytes(probe.stdout.encode("utf-8"))
        self.assertEqual(Path(observed["prefix"]).resolve(), python.parent.parent.resolve())
        self.assertEqual(observed["isolated"], 1)
        self.assertFalse(observed["user_site"])
        wheels = read_json_bytes(archive_file(self.inputs.paths["wheels"], "wheels.json"))
        self.assertEqual(observed["versions"], {
            name: wheels["packages"][name]["version"] for name in observed["versions"]})

    def test_should_reject_invalid_core_metadata_types_and_fields(self) -> None:
        """Core metadata is a strict input schema, not a best-effort hint."""
        original = read_json_bytes(archive_file(self.core, "core.json"))
        for index, variant in enumerate(("boolean-schema", "unknown", "missing", "invalid-version")):
            with self.subTest(variant=variant):
                changed = copy.deepcopy(original)
                if variant == "boolean-schema":
                    changed["schema"] = True
                elif variant == "unknown":
                    changed["extra"] = "not permitted"
                elif variant == "missing":
                    del changed["files"]["bin/clc"]
                else:
                    changed["core_version"] = "1.1"
                path = self.root / f"bad-core-{index}.tar"
                rewrite_tar(self.core, path, {"core.json": json_bytes(changed)})
                self._failed({"--core": path}, "core")

    def test_should_reject_duplicate_core_json_keys_and_archive_members(self) -> None:
        """Neither repeated JSON keys nor duplicate archive members may choose a winner."""
        original = archive_file(self.core, "core.json")
        duplicate_keys = b'{"schema":1,' + original.lstrip()[1:]
        path = self.root / "duplicate-keys.tar"
        rewrite_tar(self.core, path, {"core.json": duplicate_keys})
        self._failed({"--core": path}, "core")
        member = tarfile.TarInfo("core.json")
        member.mode = 0o644
        other = self.root / "duplicate-member.tar"
        rewrite_tar(self.core, other, extra=(member, original))
        self._failed({"--core": other}, "core")

    def test_should_reject_core_byte_hash_mismatch(self) -> None:
        """A plausible manifest never authorizes different executable bytes."""
        path = self.root / "changed-core.tar"
        rewrite_tar(self.core, path, {"bin/clc": b"changed executable bytes\n"})
        self._failed({"--core": path}, "clc")

    def test_should_reject_escaping_core_member_before_outside_write(self) -> None:
        """Unaccounted absolute paths must not escape private extraction."""
        outside = self.root / "protected-outside"
        outside.write_bytes(b"protected outside contents\n")
        member = tarfile.TarInfo(str(outside))
        member.mode = 0o644
        path = self.root / "escaping-core.tar"
        rewrite_tar(self.core, path, extra=(member, b"unexpected overwrite\n"))
        self._failed({"--core": path}, "core")
        self.assertEqual(outside.read_bytes(), b"protected outside contents\n")

    def test_should_reject_support_audit_binding_mismatches(self) -> None:
        """Every supplied audit must bind the same exact lock bytes."""
        for name in ("amiga", "preparation", "wheels"):
            with self.subTest(name=name):
                audit = read_json(self.inputs.paths[name + "_audit"])
                audit["lock_sha256"] = "0" * 64
                path = self.root / (name + "-bad-audit.json")
                path.write_bytes(json_bytes(audit))
                option = "--sdk-audit" if name == "amiga" else f"--{name}-audit"
                result = self._stage({option: path})
                self.assertEqual(result.returncode, 1, result.stderr)
                self.assertTrue(any(word in result.stderr.lower()
                                    for word in (name, "sdk", "audit", "lock")), result.stderr)
                self.assertEqual(list(self.output_parent.iterdir()), [])

    def test_should_not_execute_tampered_preparation_payload(self) -> None:
        """Verify the full preparation audit before any altered helper can execute."""
        sentinel = self.root / "unexpected-preparation-execution"
        body = (
            "#!/usr/bin/env python3\n"
            "from pathlib import Path\n"
            f"Path({str(sentinel)!r}).write_bytes(b'unexpected execution\\n')\n"
            "raise SystemExit(67)\n"
        ).encode("utf-8")
        path = self.root / "changed-preparation.tar"
        rewrite_tar(self.inputs.paths["preparation"], path, {"prepare_ndk.py": body})
        result = self._stage({"--preparation-archive": path})
        self.assertEqual(result.returncode, 1, result.stderr)
        self.assertTrue(any(word in result.stderr.lower()
                            for word in ("preparation", "prepare_ndk", "hash", "audit")), result.stderr)
        self.assertFalse(sentinel.exists(), "unverified preparation code was executed")
        self.assertEqual(list(self.output_parent.iterdir()), [])

    def test_should_reject_changed_ndk_bytes(self) -> None:
        """Staging must not prepare against an archive different from the exact lock."""
        path = self.root / "changed-ndk"
        with self.inputs.paths["ndk"].open("rb") as source, path.open("wb") as target:
            first = source.read(1)
            target.write(bytes([first[0] ^ 1]))
            shutil.copyfileobj(source, target)
        self._failed({"--ndk": path}, "ndk")

    def test_should_reject_missing_or_non_executable_selected_python(self) -> None:
        """Name the Python prerequisite/remedy without repairing paths or publishing."""
        missing = self.root / "missing-python"
        non_executable = self.root / "non-executable-python"
        non_executable.write_bytes(b"Passive non-executable prerequisite control.\n")
        non_executable.chmod(0o644)
        for selected in (missing, non_executable):
            with self.subTest(python=selected.name):
                before = None if not selected.exists() else (
                    digest_file(selected), selected.stat().st_mode)
                self.assertFalse(os.access(selected, os.X_OK))
                result = self._stage({"--python": selected})
                self.assertEqual(result.returncode, 1, result.stderr)
                diagnostic = result.stderr.lower()
                self.assertIn("python", diagnostic, result.stderr)
                self.assertTrue(any(word in diagnostic
                                    for word in ("--python", "executable", "3.10", "3.12")),
                                result.stderr)
                self.assertEqual(list(self.output_parent.iterdir()), [],
                                 "invalid Python left a candidate or private sibling")
                if before is None:
                    self.assertFalse(selected.exists() or selected.is_symlink())
                else:
                    self.assertFalse(selected.is_symlink())
                    self.assertEqual((digest_file(selected), selected.stat().st_mode), before)

    def test_should_reject_other_declared_wheel_abi_with_matching_audit(self) -> None:
        """Reject a declared wheel/Python mismatch, not merely altered audit bytes."""
        inputs = self.inputs
        python_abi = inputs.inputs["python_abi"]
        other_abi = "cp312" if python_abi == "cp310" else "cp310"
        changed = self.root / "other-declared-abi.tar"
        changed_audit = self.root / "other-declared-abi-audit.json"
        mismatched_wheel_abi(inputs.paths["wheels"], inputs.paths["wheels_audit"],
                             changed, changed_audit, python_abi)
        expected_receipt = read_json_bytes(archive_file(inputs.paths["wheels"], "wheels.json"))
        expected_receipt["abi"] = other_abi
        self.assertEqual(read_json_bytes(archive_file(changed, "wheels.json")), expected_receipt)
        expected_inventory = copy.deepcopy(inputs.inventories["wheels"])
        metadata_hash = hashlib.sha256(archive_file(changed, "wheels.json")).hexdigest()
        expected_inventory["wheels.json"]["sha256"] = metadata_hash
        self.assertEqual(audit_inventory(changed, changed_audit, inputs.inputs["lock"]["sha256"]),
                         expected_inventory)
        expected_audit = read_json(inputs.paths["wheels_audit"])
        expected_audit["files"]["wheels.json"]["sha256"] = metadata_hash
        self.assertEqual(read_json(changed_audit), expected_audit)
        identities = (digest_file(changed), digest_file(changed_audit))
        result = self._stage({"--wheels-archive": changed, "--wheels-audit": changed_audit,
                              "--python": sys.executable})
        self.assertEqual(result.returncode, 1, result.stderr)
        self.assertTrue(any(word in result.stderr.lower() for word in ("python", "abi", "wheel")),
                        result.stderr)
        self.assertEqual(list(self.output_parent.iterdir()), [],
                         "mismatched declared ABI left a candidate or private sibling")
        self.assertEqual((digest_file(changed), digest_file(changed_audit)), identities)

    def test_should_refuse_existing_output_without_activation_or_input_changes(self) -> None:
        """Existing user state is never disposable candidate work."""
        self.output.mkdir()
        keep = self.output / "keep"
        keep.write_bytes(b"user candidate contents\n")
        current = self.root / "current"
        current.symlink_to("old-complete")
        result = self._stage()
        self.assertEqual(result.returncode, 1, result.stderr)
        self.assertEqual(keep.read_bytes(), b"user candidate contents\n")
        self.assertEqual(os.readlink(current), "old-complete")
        self.assertEqual(list(self.output_parent.iterdir()), [self.output])
