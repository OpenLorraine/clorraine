"""Coherently mutate delivered support bytes only after valid producer assembly."""

from __future__ import annotations

import copy
import io
import json
import tarfile

from native_install_fixture import StateShellFixture
from native_manifest_data import DOWNLOAD_ROOT, digest, encode, make_manifest, tar_bytes


def ordinary_members(data: bytes) -> tuple[dict[str, bytes], tuple[str, ...]]:
    """Read miniature fixture files and their executable bits, never helper source modules."""
    with tarfile.open(fileobj=io.BytesIO(data), mode="r:") as source:
        members = source.getmembers()
        files = {member.name: source.extractfile(member).read()
                 for member in members if member.isfile()}
        executable = tuple(member.name for member in members
                           if member.isfile() and member.mode & 0o111)
    return files, executable


class DeliveredSupport:
    """Keep transport, archive, audit and preparation identities mutually consistent.

    Only miniature support archives are decoded here. Producer inputs and the
    exact shell/helper remain untouched. Valid producer assembly must precede
    construction so negative cases cannot accidentally test producer rejection.
    """

    def __init__(self, fixture) -> None:
        """Retain independent mutable copies of the already accepted fixture's records."""
        self.fixture = fixture
        self.raw = copy.deepcopy(fixture.raw)
        self.files, self.executable = ordinary_members(self.raw["amiga.tar"])
        self.audit = json.loads(self.raw["amiga-audit.json"])
        self.extra_members: tuple[tuple[tarfile.TarInfo, bytes], ...] = ()

    def bind(self, shell: StateShellFixture) -> dict:
        """Route exact mutated payloads and a freshly coherent manifest to the real helper."""
        self.raw["amiga.tar"] = tar_bytes(self.files, self.executable, self.extra_members)
        self.raw["amiga-audit.json"] = encode(self.audit)
        preparation, executable = ordinary_members(self.raw["preparation.tar"])
        descriptor = json.loads(preparation["preparation.json"])
        descriptor["base"] = {
            name: {key: value for key, value in record.items()
                   if key in ("type", "sha256", "target")}
            for name, record in self.audit["files"].items()}
        preparation["preparation.json"] = encode(descriptor)
        preparation_audit = json.loads(self.raw["preparation-audit.json"])
        preparation_audit["files"]["preparation.json"]["sha256"] = digest(
            preparation["preparation.json"])
        self.raw["preparation.tar"] = tar_bytes(preparation, executable)
        self.raw["preparation-audit.json"] = encode(preparation_audit)
        manifest = make_manifest(self.raw, self.fixture.lock)
        for asset in manifest["assets"]:
            filename = asset["url"].rsplit("/", 1)[-1]
            if asset["kind"] == "ndk":
                filename = "ndk"
            shell.route(asset["url"], self.raw[filename])
        shell.route(DOWNLOAD_ROOT + manifest["release"] + "/manifest.json", encode(manifest))
        return manifest


def observed_shell(fixture, *, name: str = "observed-shell") -> StateShellFixture:
    """Observe actual child invocations without replacing any preparation result."""
    root = fixture.root / name
    root.mkdir()
    shell = StateShellFixture(
        root, fixture.bootstrap, fixture.manifest, fixture.paths,
        probes={"ubuntu": "22.04", "architecture": "x86_64", "python": [3, 10, 12]})
    # The miniature preparation really fails. Enabling the existing final-rename
    # barrier records subprocesses without substituting even that negative result.
    shell.barrier = "before-current"
    return shell
