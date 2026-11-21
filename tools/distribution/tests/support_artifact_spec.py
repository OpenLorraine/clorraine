"""Mandatory real-archive proof on each supported, isolated Ubuntu/Python host."""

from __future__ import annotations

import email.parser
import io
import json
import os
import re
import shutil
import sys
import tarfile
import unittest
import zipfile
from pathlib import Path

from support_artifact_fixtures import (
    ArtifactFixture, CASES, PROFILES, contained, file_hash, read_json,
)
from support_fixtures import digest


_VAMOS = """
import importlib.metadata
import sys
entries = [entry for entry in importlib.metadata.distribution("amitools").entry_points
           if entry.group == "console_scripts" and entry.name == "vamos"]
assert len(entries) == 1, "exactly one installed vamos entry is required"
sys.argv = ["vamos", *sys.argv[1:]]
raise SystemExit(entries[0].load()())
"""

# Run inside the installed interpreter so native imports exercise its ABI and
# private site-packages, rather than whichever Python launched unittest.
_INSTALLED = """
import hashlib
import importlib
import importlib.machinery
import importlib.metadata
import json
import pathlib
import site
import sys
native, imported, versions = [], set(), {}
for name in json.loads(sys.argv[1]):
    distribution = importlib.metadata.distribution(name)
    versions[name] = distribution.version
    for relative in distribution.files or []:
        path = pathlib.Path(distribution.locate_file(relative))
        if not path.is_file():
            continue
        with path.open("rb") as stream:
            signature = stream.read(8)
        if not (signature.startswith(b"\\x7fELF") or signature == b"!<arch>\\n"):
            continue
        content = path.read_bytes()
        native.append({"path": str(path), "sha256": hashlib.sha256(content).hexdigest()})
        for suffix in importlib.machinery.EXTENSION_SUFFIXES:
            if path.name.endswith(suffix):
                stem = path.name[:-len(suffix)]
                parts = [*relative.parts[:-1], stem]
                if (all(part.isidentifier() for part in parts)
                        and b"PyInit_" + stem.encode() + b"\\0" in content):
                    importlib.import_module(".".join(parts))
                    imported.add(name)
                break
importlib.import_module("machine68k")
importlib.import_module("greenlet")
print(json.dumps({"prefix": sys.prefix, "isolated": sys.flags.isolated,
                  "user_site": site.ENABLE_USER_SITE, "versions": versions,
                  "native": native, "imported": sorted(imported)}))
"""


def loaded_file_owners(path: Path, query) -> set[str]:
    """Return genuine dpkg owners of the actual loaded file using the supplied runner.

    `query` has ArtifactFixture.run's interface and returns bytes. Resolve the
    file before asking dpkg: an alias query can exit successfully while reporting
    only a package's file redirection (a diversion), not ownership. A real wrong
    owner must be returned to the caller, never hidden by trying other names.
    Missing files raise OSError; absent ownership returns an empty set.

    Example: owners = loaded_file_owners(loader_path, fixture.run)
    """
    canonical = path.resolve(strict=True)
    package = r"[a-z0-9][a-z0-9+.-]+(?::[a-z0-9][a-z0-9-]*)?"
    record = re.compile(r"^(" + package + r"(?:, " + package + r")*): (.+)$")
    candidates = [canonical]
    if str(canonical).startswith("/usr/lib/"):
        alias = Path(str(canonical)[4:])
        try:
            if alias.resolve(strict=True) == canonical:
                candidates.append(alias)
        except OSError:
            pass
    for candidate in candidates:
        result = query(["dpkg-query", "-S", candidate], check=False)
        if result.returncode:
            continue
        owners = set()
        for line in result.stdout.decode("utf-8").splitlines():
            match = record.fullmatch(line)
            if match is not None and match[2] == str(candidate):
                owners.update(name.partition(":")[0] for name in match[1].split(", "))
        if owners:
            return owners
    return set()


class SupportArtifactAcceptance(unittest.TestCase):
    """Require genuine archives and isolation; missing acceptance inputs are failures."""

    @classmethod
    def setUpClass(cls) -> None:
        """Stage one verified candidate and clean its owned work even if auditing fails."""
        value = os.environ.get("CLORRAINE_SUPPORT_ACCEPTANCE")
        if not value:
            raise AssertionError("CLORRAINE_SUPPORT_ACCEPTANCE must name real acceptance.json inputs")
        path = Path(value)
        if not path.is_absolute() or not path.is_file():
            raise AssertionError(f"required acceptance.json is unavailable: {value}")
        cls.candidate = ArtifactFixture(path)
        cls.addClassCleanup(cls.candidate.scratch.cleanup)
        cls.candidate.stage()

    def _root(self) -> Path:
        """Give every acceptance scenario a distinct mutable copy of the candidate."""
        return self.candidate.assembled(self._testMethodName)

    def _wheel_metadata(self, root: Path) -> dict:
        """Cross-check recorded wheel identities against their actual ZIP metadata."""
        metadata = read_json(root / "wheelset/wheels.json")
        self.assertEqual(set(metadata), {"schema", "lock_sha256", "abi", "packages"})
        self.assertEqual(metadata["schema"], 1)
        self.assertEqual(metadata["lock_sha256"], self.candidate.inputs["lock"]["sha256"])
        self.assertEqual(metadata["abi"], self.candidate.inputs["python_abi"])
        self.assertTrue({"amitools", "machine68k", "greenlet", "pip"} <= set(metadata["packages"]))
        filenames = set()
        for name, record in metadata["packages"].items():
            self.assertRegex(name, r"^[a-z0-9]+(?:-[a-z0-9]+)*$")
            self.assertRegex(record["version"], r"^[0-9][a-zA-Z0-9.!+_-]*$")
            self.assertEqual(set(record), {"version", "source", "purpose", "filename",
                                          "bytes", "sha256", "tags"})
            self.assertIn(record["purpose"], ("runtime", "bootstrap"))
            if name in ("amitools", "machine68k", "greenlet", "pip"):
                self.assertEqual(record["purpose"], "bootstrap" if name == "pip" else "runtime")
            source = self.candidate.lock["sources"][record["source"]]
            if source["kind"] == "archive":
                self.assertEqual(record["version"], source["version"])
            path = contained(root / "wheelset", record["filename"])
            self.assertEqual(path.suffix, ".whl")
            self.assertNotIn(record["filename"], filenames)
            filenames.add(record["filename"])
            self.assertIs(type(record["bytes"]), int)
            self.assertEqual(path.stat().st_size, record["bytes"])
            self.assertEqual(file_hash(path), record["sha256"])
            with zipfile.ZipFile(path) as wheel:
                for suffix in (".dist-info/METADATA", ".dist-info/WHEEL"):
                    entries = [item for item in wheel.namelist() if item.endswith(suffix)]
                    self.assertEqual(len(entries), 1, f"{path}: ambiguous {suffix}")
                    headers = email.parser.BytesParser().parsebytes(wheel.read(entries[0]))
                    if suffix.endswith("/METADATA"):
                        self.assertEqual(re.sub(r"[-_.]+", "-", headers["Name"]).lower(), name)
                        self.assertEqual(headers["Version"], record["version"])
                    else:
                        self.assertTrue(record["tags"], f"{path}: missing wheel tags")
                        self.assertEqual(set(headers.get_all("Tag", [])), set(record["tags"]))
        return metadata

    def _install(self, root: Path, metadata: dict, check: bool = True):
        """Use real offline pip with every dependency explicitly pinned and hashed."""
        fixture = self.candidate
        fixture.run([sys.executable, "-I", "-m", "venv", root / "venv"])
        python = root / "venv/bin/python"
        fixture.run([python, "-I", "-m", "pip", "--version"])
        requirements = root / "wheel-pins.txt"
        requirements.write_text("".join(
            f"{name}=={record['version']} --hash=sha256:{record['sha256']}\n"
            for name, record in sorted(metadata["packages"].items())), encoding="utf-8")
        result = fixture.run(
            [python, "-I", "-m", "pip", "--isolated", "--no-cache-dir", "install", "--no-index",
             "--find-links", root / "wheelset/wheels", "--only-binary=:all:", "--no-deps",
             "--require-hashes", "--force-reinstall", "-r", requirements], check=check)
        if check:
            fixture.run([python, "-I", "-m", "pip", "--isolated", "check"])
        return result

    def _installed(self, root: Path, metadata: dict) -> dict:
        """Import every identifiable native extension through the relocated private venv."""
        result = self.candidate.run(
            [root / "venv/bin/python", "-I", "-c", _INSTALLED,
             json.dumps(sorted(metadata["packages"]))])
        observed = json.loads(result.stdout.decode().splitlines()[-1])
        self.assertEqual(Path(observed["prefix"]).resolve(), (root / "venv").resolve())
        self.assertEqual(observed["isolated"], 1)
        self.assertFalse(observed["user_site"])
        self.assertEqual(observed["versions"], {
            name: record["version"] for name, record in metadata["packages"].items()})
        self.assertTrue({"machine68k", "greenlet"} <= set(observed["imported"]))
        return observed

    def _native_path(self, payload: Path, key: str, installed: dict) -> Path:
        """Locate audited wheel-native bytes after installation without guessing package layout."""
        if "!" not in key:
            return contained(payload, key)
        wheel_name, member = key.split("!", 1)
        with zipfile.ZipFile(contained(payload, wheel_name)) as wheel:
            expected = digest(wheel.read(member))
        matches = [Path(record["path"]) for record in installed["native"]
                   if record["sha256"] == expected]
        self.assertTrue(matches, f"{key}: audited native bytes were not installed")
        return matches[0]

    def _native_closure(self, root: Path, installed: dict) -> None:
        """Inspect actual native dependencies and load host tools, beyond version-only evidence."""
        fixture = self.candidate
        packages = fixture.lock["output"]["runtime_packages"][fixture.inputs["ubuntu"]]
        for name, record in packages.items():
            result = fixture.run(["dpkg-query", "-W", "-f=${Version}", name])
            self.assertEqual(result.stdout.decode(), record["version"], name)
        for kind, directory in (("amiga", "sdk"), ("preparation", "preparation"), ("wheels", "wheelset")):
            payload, audit = root / directory, fixture.audits[kind]
            for key, record in audit["native"].items():
                path = self._native_path(payload, key, installed)
                if record["format"] in ("ar", "amiga-object") and record["machine"] == "m68k":
                    # Target archives and standalone HUNK objects require the
                    # cross-tool's actual format report, not host ELF parsing.
                    objdump = root / "sdk/bin/m68k-amigaos-objdump"
                    self.assertTrue(objdump.resolve(strict=True).is_relative_to((root / "sdk").resolve()))
                    report = fixture.run([objdump, "-f", path]).stdout.decode()
                    architectures = re.findall(r"^\s*architecture:\s*([^,\s]+)\s*,", report, re.M)
                    members = re.findall(r"^[^\n]*\bfile format\s+\S+\s*$", report, re.M)
                    self.assertTrue(architectures, f"{key}: no actual target object architecture")
                    self.assertEqual(len(architectures), len(members), f"{key}: uninspected target object")
                    self.assertTrue(all(value.split(":", 1)[0] == "m68k" for value in architectures), key)
                    is_archive = re.search(r"^\s*In archive\s+", report, re.M) is not None
                    self.assertEqual(is_archive, record["format"] == "ar", key)
                    if record["format"] == "amiga-object":
                        self.assertEqual(len(members), 1, f"{key}: not a standalone object")
                        self.assertRegex(report, r"\bfile format\s+amiga\b")
                    self.assertEqual(record["needed"], {}, key)
                    self.assertEqual(record["glibc"], [], key)
                    self.assertIsNone(record["soname"], key)
                    continue
                header = fixture.run(["readelf", "-hW", path]).stdout.decode()
                machines = re.findall(r"Machine:\s*(.+)", header)
                self.assertTrue(machines, f"{key}: no actual ELF machine")
                expected = "Advanced Micro Devices X86-64" if record["machine"] == "x86_64" else "MC68000"
                self.assertTrue(all(machine.strip() == expected for machine in machines), key)
                dynamic = fixture.run(["readelf", "-dW", path]).stdout.decode()
                needed = set(re.findall(r"\(NEEDED\).*?\[(.*?)\]", dynamic))
                self.assertEqual(needed, set(record["needed"]), key)
                sonames = re.findall(r"\(SONAME\).*?\[(.*?)\]", dynamic)
                self.assertEqual(sonames, [] if record["soname"] is None else [record["soname"]], key)
                versions = fixture.run(["readelf", "--version-info", "-W", path]).stdout.decode()
                glibc = set(re.findall(r"Name: GLIBC_([0-9.]+)", versions))
                self.assertEqual(glibc, set(record["glibc"]), key)
                self.assertTrue(all(tuple(map(int, version.split("."))) <= (2, 35) for version in glibc), key)
                if record["machine"] != "x86_64" or not needed:
                    continue
                loaded = fixture.run(["ldd", path]).stdout.decode()
                self.assertNotIn("not found", loaded, key)
                # The load-address suffix ends the path; internal spaces belong to the filename.
                address_suffix = r"[ \t]+\(0x[0-9a-fA-F]+\)[ \t]*$"
                for soname, provider in record["needed"].items():
                    match = re.search(
                        r"^[ \t]*" + re.escape(soname) + r"[ \t]+=>[ \t]+(/[^\n]+?)" + address_suffix,
                        loaded, re.M)
                    if match is None:
                        match = re.search(
                            r"^[ \t]*(/[^\n]*/" + re.escape(soname) + r")" + address_suffix, loaded, re.M)
                    self.assertIsNotNone(match, f"{key}: unresolved {soname}")
                    resolved = Path(match.group(1))
                    if provider.startswith("file:"):
                        expected_path = self._native_path(payload, provider[5:], installed)
                        self.assertEqual(resolved.resolve(), expected_path.resolve())
                    else:
                        self.assertTrue(provider.startswith("package:"), provider)
                        package = provider[8:]
                        self.assertIn(soname, packages[package]["sonames"])
                        owners = loaded_file_owners(resolved, fixture.run)
                        self.assertIn(package, owners, str(resolved))
            for name, record in audit["files"].items():
                path = contained(payload, name)
                if record["type"] != "file" or not os.access(path, os.X_OK):
                    continue
                native = audit["native"].get(name, {})
                with path.open("rb") as stream:
                    header = stream.read(20)
                executable = header.startswith(b"#!")
                if native.get("machine") == "x86_64" and native["format"] == "elf":
                    kind_number = int.from_bytes(header[16:18], "little" if header[5] == 1 else "big")
                    executable = kind_number == 2 or b"INTERP" in fixture.run(
                        ["readelf", "-lW", path]).stdout
                if record["role"] not in ("source", "notice", "review") and executable:
                    result = fixture.run([path, "--help"], check=False)
                    self.assertGreaterEqual(result.returncode, 0, f"{path}: loader signal")
                    self.assertNotIn(b"error while loading shared libraries", result.stderr)
                    self.assertIsNone(re.search(rb"version .+ not found", result.stderr))

    def _corpus(self, root: Path, label: str) -> None:
        """Compile all fixed units with strict debug flags and compare real program output."""
        fixture = self.candidate
        for profile in PROFILES:
            for case in CASES:
                if profile == "host" and case == "volatile-mmio":
                    continue
                with self.subTest(location=label, profile=profile, case=case):
                    work = root / label / profile / case
                    work.mkdir(parents=True)
                    source = fixture.fixtures / "units" / profile / (case + ".c")
                    binary = work / "program"
                    compiler = "cc" if profile == "host" else root / "sdk/bin/m68k-amigaos-gcc"
                    flags = ["-std=c99"]
                    if profile != "host":
                        self.assertTrue(Path(compiler).resolve().is_relative_to((root / "sdk").resolve()))
                        flags += ["-mcrt=nix13" if profile == "hosted-amiga-13" else "-mcrt=nix20",
                                  "-m68000", "-fwrapv"]
                    flags += ["-O0", "-g", "-Wall", "-Wextra", "-Werror"]
                    libraries = [] if profile == "host" else ["-lamiga", "-lm"]
                    fixture.run([compiler, *flags, source, *libraries, "-o", binary], cwd=work)
                    command = ([binary] if profile == "host" else
                               [root / "venv/bin/python", "-I", "-c", _VAMOS, binary])
                    result = fixture.run(command, cwd=work)
                    self.assertEqual(result.stdout.replace(b"\r", b""),
                                     (fixture.fixtures / "goldens" / (case + ".txt")).read_bytes())

    def test_should_prepare_the_complete_reference_tree_and_notices(self) -> None:
        """The actual helpers must reproduce all ordinary and baseline-era header bytes."""
        self.candidate.prepare(self._root())

    def test_should_reject_corrupt_real_ndk_bytes_and_missing_helpers(self) -> None:
        """The real preparation payload must reject corrupted inputs without completion."""
        for failure in ("archive", "helper"):
            with self.subTest(failure=failure):
                root = self.candidate.assembled(self._testMethodName + "-" + failure)
                metadata = read_json(root / "preparation/preparation.json")
                archive = Path(self.candidate.inputs["ndk_archive"])
                if failure == "archive":
                    corrupt = root / "corrupt-ndk-archive"
                    shutil.copyfile(archive, corrupt)
                    with corrupt.open("r+b") as stream:
                        stream.seek(-1, os.SEEK_END)
                        final = stream.read(1)
                        stream.seek(-1, os.SEEK_END)
                        stream.write(bytes([final[0] ^ 1]))
                    archive, identity = corrupt, metadata["archive"]["source"]
                else:
                    contained(root / "preparation", metadata["helpers"]["lha"]["path"]).unlink()
                    identity = "lha"
                result = self.candidate.run(
                    [sys.executable, root / "preparation/prepare_ndk.py", "--archive",
                     archive, "--destination", root / "sdk"], check=False)
                self.assertNotEqual(result.returncode, 0, result.stdout.decode())
                self.assertIn(identity, result.stderr.decode())
                self.assertFalse((root / "sdk/.ndk-prepared.json").exists())

    def test_should_load_declared_host_tools_and_native_wheels(self) -> None:
        """Both native ABI compatibility and real loader dependency ownership are required."""
        root = self._root()
        self.candidate.prepare(root)
        metadata = self._wheel_metadata(root)
        self._install(root, metadata)
        self._native_closure(root, self._installed(root, metadata))

    def test_should_audit_standalone_amiga_startup_objects(self) -> None:
        """Required startup objects retain an accurate non-archive m68k native record."""
        fixture = self.candidate
        root = self._root()
        sdk = root / "sdk"
        objdump = sdk / "bin/m68k-amigaos-objdump"
        self.assertTrue(objdump.resolve(strict=True).is_relative_to(sdk.resolve()))
        required = "m68k-amigaos/libnix/lib/ncrt0.o"
        self.assertTrue(contained(sdk, required).is_file(), required)
        objects = sorted((sdk / "m68k-amigaos/libnix/lib").rglob("*.o"))
        self.assertTrue(objects, "the SDK must contain its target startup objects")
        for path in objects:
            with self.subTest(path=path.relative_to(sdk).as_posix()):
                key = path.relative_to(sdk).as_posix()
                record = fixture.audits["amiga"]["native"].get(key)
                self.assertEqual(record, {
                    "format": "amiga-object", "machine": "m68k", "soname": None,
                    "needed": {}, "glibc": []}, key)
                report = fixture.run([objdump, "-f", path]).stdout.decode()
                self.assertIsNone(re.search(r"^\s*In archive\s+", report, re.M), key)
                formats = re.findall(r"\bfile format\s+(\S+)", report)
                self.assertEqual(formats, ["amiga"], key)
                architectures = re.findall(r"^\s*architecture:\s*([^,\s]+)\s*,", report, re.M)
                self.assertEqual(len(architectures), 1, key)
                self.assertEqual(architectures[0].split(":", 1)[0], "m68k", key)

    def test_should_run_all_profiles_before_and_after_relocation(self) -> None:
        """A moved SDK/venv must prepare, compile and execute without its original prefix."""
        root = self._root()
        self.candidate.prepare(root)
        metadata = self._wheel_metadata(root)
        self._install(root, metadata)
        self._installed(root, metadata)
        self._corpus(root, "original executions")
        relocated = root.with_name(root.name + " relocated with spaces")
        root.rename(relocated)
        self.assertFalse(root.exists(), "the old assembled prefix must be unavailable")
        fresh = relocated / "fresh unprepared sdk"
        shutil.copytree(self.candidate.payloads["amiga"], fresh, symlinks=True)
        self.candidate.prepare(relocated, fresh)
        shutil.rmtree(relocated / "sdk")
        fresh.rename(relocated / "sdk")
        self._native_closure(relocated, self._installed(relocated, metadata))
        self._corpus(relocated, "relocated executions")

    def test_should_reject_incomplete_or_incompatible_offline_wheel_sets(self) -> None:
        """Real pip must reject missing native wheels, wrong ABI and source fallback."""
        for variant in ("machine68k", "greenlet", "wrong-abi", "source-distribution"):
            with self.subTest(variant=variant):
                root = self.candidate.assembled(self._testMethodName + "-" + variant)
                metadata = self._wheel_metadata(root)
                name = variant if variant in ("machine68k", "greenlet") else "machine68k"
                record = metadata["packages"][name]
                path = contained(root / "wheelset", record["filename"])
                if variant in ("machine68k", "greenlet"):
                    path.unlink()
                elif variant == "wrong-abi":
                    other = "cp312" if metadata["abi"] == "cp310" else "cp310"
                    wrong = path.with_name(path.name.rsplit("-", 3)[0] + f"-{other}-{other}-linux_x86_64.whl")
                    path.rename(wrong)
                else:
                    path.unlink()
                    source = path.with_name(f"{name}-{record['version']}.tar.gz")
                    backend = (
                        "import os\nfrom pathlib import Path\n"
                        "Path(os.environ['SUPPORT_COMPILATION_SENTINEL']).write_text('backend ran')\n"
                        "raise RuntimeError('source builds are forbidden')\n").encode()
                    # No bootstrap dependency can mask a forbidden attempt to run this backend.
                    project = (b"[build-system]\nrequires = []\n"
                               b'build-backend = "sentinel_backend"\nbackend-path = ["."]\n')
                    with tarfile.open(source, "w:gz") as archive:
                        for filename, content in (("pyproject.toml", project),
                                                  ("sentinel_backend.py", backend)):
                            item = tarfile.TarInfo(f"{name}-{record['version']}/{filename}")
                            item.size = len(content)
                            archive.addfile(item, io.BytesIO(content))
                    record["sha256"] = file_hash(source)
                    record["bytes"] = source.stat().st_size
                    record["filename"] = source.relative_to(root / "wheelset").as_posix()
                sentinel = root / "compilation-attempt"
                original = self.candidate.environment.copy()
                self.candidate.environment["SUPPORT_COMPILATION_SENTINEL"] = str(sentinel)
                try:
                    result = self._install(root, metadata, check=False)
                finally:
                    self.candidate.environment = original
                self.assertNotEqual(result.returncode, 0, result.stdout.decode())
                self.assertIn(name, result.stderr.decode().lower())
                self.assertFalse(sentinel.exists(), "pip executed a source-build backend")
