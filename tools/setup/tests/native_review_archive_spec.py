"""Component-aware archive containment before any destination member is written."""

from __future__ import annotations

from pathlib import Path
import sys
import tarfile

from native_manifest_data import RELEASE, digest, measure, tar_bytes
from native_manifest_producer_fixture import NativeProcessCase, ScratchCase
from native_review_payload_fixture import DeliveredSupport, observed_shell


_REPOSITORY = Path(__file__).resolve().parents[3]
"""Import root used only by local direct extraction contracts."""
_ORIGINAL_PATH = sys.path[:]
try:
    sys.path.insert(0, str(_REPOSITORY))
    from tools.setup import native_archive as archive
finally:
    sys.path[:] = _ORIGINAL_PATH


def symbolic_link(name: str, target: str) -> tuple[tarfile.TarInfo, bytes]:
    """Build a passive relative-link tar header without creating a filesystem link."""
    member = tarfile.TarInfo(name)
    member.type, member.linkname, member.mode = tarfile.SYMTYPE, target, 0o777
    return member, b""


def directory(name: str) -> tuple[tarfile.TarInfo, bytes]:
    """Declare directory targets explicitly so internal directory links are unambiguous."""
    member = tarfile.TarInfo(name)
    member.type, member.mode = tarfile.DIRTYPE, 0o755
    return member, b""


def link_escape(files: dict[str, bytes]) -> bytes:
    """Place a link before later parent components that escape after POSIX resolution."""
    return tar_bytes({**files, "shallow/data": b"internal\n", "outside": b"archive member\n"},
                     tuple(name for name in files if name.startswith("bin/")), (
                         symbolic_link("dir/link", "../shallow"),
                         symbolic_link("escape", "dir/link/../../outside")))


class NativeReviewArchiveTests(ScratchCase):
    """Authenticated tar bytes remain untrusted until all link chains are contained."""

    def extract(self, data: bytes, name: str) -> Path:
        """Supply authentic lengths/hashes and exact expansion rather than bypassing checks."""
        payload = self.root / (name + ".tar")
        payload.write_bytes(data)
        count, expanded = measure(data, "core")
        asset = {"kind": "core", "bytes": len(data), "sha256": digest(data),
                 "entries": count, "expanded_bytes": expanded}
        destination = self.root / name
        archive.extract_tar(payload, destination, asset)
        return destination

    def test_should_reject_link_then_parent_escape_before_any_member_write(self) -> None:
        """Later '..' applies to the followed link target, not its lexical spelling."""
        canary = self.root / "outside"
        canary.write_bytes(b"outside private canary\n")
        data = link_escape({"first-ordinary-member": b"must not be extracted\n"})
        destination = self.root / "candidate"
        try:
            with self.assertRaises(ValueError):
                self.extract(data, "candidate")
            self.assertFalse(destination.exists(), "archive validation must precede every write")
        finally:
            self.assertEqual(canary.read_bytes(), b"outside private canary\n")

    def test_should_keep_legitimate_internal_links_and_parent_chains_usable(self) -> None:
        """Component-aware checks must retain contained links required by SDK archives."""
        data = tar_bytes({"tree/deep/item": b"deep\n", "tree/peer": b"peer\n"}, extras=(
            directory("tree"), directory("tree/deep"),
            symbolic_link("tree/alias", "deep"),
            symbolic_link("through-parent", "tree/alias/../peer"),
            symbolic_link("second", "through-parent"),
            symbolic_link("tree/local", "deep/../peer")))
        destination = self.extract(data, "contained")
        for name in ("through-parent", "second", "tree/local"):
            self.assertEqual((destination / name).read_bytes(), b"peer\n")
        self.assertEqual((destination / "tree/alias/item").read_bytes(), b"deep\n")

    def test_should_reject_cycles_and_dangling_links_before_writing(self) -> None:
        """Bounded resolution rejects missing targets and cycles rather than guessing a file."""
        for name, links in (
                ("dangling", (symbolic_link("alias", "missing"),)),
                ("cycle", (symbolic_link("first", "second"), symbolic_link("second", "first")))):
            with self.subTest(kind=name):
                data = tar_bytes({"ordinary": b"not written"}, extras=links)
                with self.assertRaises(ValueError):
                    self.extract(data, name)
                self.assertFalse((self.root / name).exists())


class NativeReviewArchiveShellTests(NativeProcessCase):
    """The shipped helper must reject component-aware escapes before preparation."""

    def test_should_reject_delivered_link_escape_before_preparation(self) -> None:
        """Coherent transport hashes cannot make an escaping support archive safe to use."""
        fixture, _shell = self._new()
        shell = observed_shell(fixture)
        delivered = DeliveredSupport(fixture)
        for name, content in (("shallow/data", b"internal\n"), ("outside", b"archive member\n")):
            delivered.files[name] = content
            delivered.audit["files"][name] = {
                "type": "file", "sha256": digest(content), "origins": ["lha"], "role": "source"}
        delivered.extra_members = (
            symbolic_link("dir/link", "../shallow"),
            symbolic_link("escape", "dir/link/../../outside"))
        for member, _data in delivered.extra_members:
            delivered.audit["files"][member.name] = {
                "type": "symlink", "target": member.linkname, "origins": ["lha"], "role": "link"}
        manifest = delivered.bind(shell)
        data = delivered.raw["amiga.tar"]
        url = next(asset["url"] for asset in manifest["assets"] if asset["kind"] == "amiga")
        canary = shell.root / "outside"
        canary.write_bytes(b"outside shell canary\n")
        result = shell.invoke(["--release", RELEASE])
        shell.assert_failed_safely(self, result)
        self.assertTrue(shell.requested(url))
        self.assertIn(digest(data), [event["sha256"] for event in shell.events("response-bytes")
                                    if event["url"] == url])
        self.assertEqual(canary.read_bytes(), b"outside shell canary\n")
        self.assertFalse([event for event in shell.events("subprocess")
                          if event["operation"] == "preparation"],
                         "unsafe archive reached preparation")
