"""Unsafe archive specifications through the unchanged shipped installer."""

from __future__ import annotations

import io
import tarfile

from native_manifest_data import RELEASE, tar_bytes
from native_manifest_producer_fixture import NativeProcessCase


class NativeArchiveBoundaryTests(NativeProcessCase):
    """Use the shipped helper rather than inventing a private archive API."""

    def test_should_reject_hostile_members_without_touching_outside_canaries(self) -> None:
        """Unsafe types, paths and link chains remain untrusted despite matching hashes."""
        for attack in ("parent", "long-parent", "absolute", "symlink", "hardlink", "symlink-parent",
                       "duplicate", "file-parent", "fifo", "device", "expansion"):
            with self.subTest(attack=attack):
                fixture, shell = self._new()
                canary = shell.root / "outside-canary"
                canary.write_bytes(b"outside must remain unchanged\n")
                member = tarfile.TarInfo("../escape")
                data = b"changed\n"
                extras = []
                if attack == "absolute":
                    member.name = str(canary)
                elif attack == "long-parent":
                    member.name = "../" * 64 + str(canary).lstrip("/")
                elif attack in ("symlink", "hardlink"):
                    member.name = "outside-link"
                    member.type = tarfile.SYMTYPE if attack == "symlink" else tarfile.LNKTYPE
                    member.linkname = str(canary)
                elif attack == "symlink-parent":
                    link = tarfile.TarInfo("linked-parent")
                    link.type, link.linkname = tarfile.SYMTYPE, str(canary.parent)
                    extras.append((link, b""))
                    member.name = "linked-parent/outside-canary"
                elif attack == "duplicate":
                    member.name = "bin/clc"
                elif attack == "file-parent":
                    member.name = "ordinary/child"
                elif attack in ("fifo", "device"):
                    member.name = "special"
                    member.type = tarfile.FIFOTYPE if attack == "fifo" else tarfile.CHRTYPE
                elif attack == "expansion":
                    member.name, member.size = "huge", 2 ** 62
                if member.isfile() and attack != "expansion":
                    member.size = len(data)
                extras.append((member, data))
                with tarfile.open(fileobj=io.BytesIO(fixture.raw["core.tar"])) as archive:
                    files = {item.name: archive.extractfile(item).read()
                             for item in archive.getmembers() if item.isfile()}
                if attack == "file-parent":
                    files["ordinary"] = b"not a directory"
                executable = ("bin/clc", "bin/clorraine", "bin/clorraine-lsp")
                if attack == "expansion":
                    # Write only a huge *claim*, not huge test data. A bounded
                    # reader must reject it before allocating from that claim.
                    end = sum(512 + ((len(value) + 511) // 512) * 512 for value in files.values())
                    malicious = (tar_bytes(files, executable)[:end] +
                                 member.tobuf(format=tarfile.PAX_FORMAT) + b"\0" * 1024)
                else:
                    malicious = tar_bytes(files, executable, tuple(extras))
                _, url = fixture.hostile_asset(shell, "core", malicious,
                                               measure_expansion=attack != "expansion")
                result = shell.invoke(["--release", RELEASE])
                shell.assert_failed_safely(self, result)
                self.assertTrue(shell.requested(url), "unsafe archive was never requested")
                self.assertEqual(canary.read_bytes(), b"outside must remain unchanged\n")
                self.assertFalse(any(shell.root.rglob("escape")), "parent escape created a file")
