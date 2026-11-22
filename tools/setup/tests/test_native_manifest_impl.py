"""Implementation boundaries for native metadata, archives and transfer budgets."""

from __future__ import annotations

import hashlib
import io
import os
from pathlib import Path
import signal
import sys
import tarfile
import tempfile
import time
import unittest
from unittest.mock import patch
import zipfile

from native_manifest_data import tar_bytes, zip_bytes
from native_transport_fixture import run_process


_ROOT = Path(__file__).resolve().parents[3]
"""Repository location used only by this implementation-test interpreter."""
_ORIGINAL_PATH = sys.path[:]
try:
    sys.path.insert(0, str(_ROOT))
    from tools.setup import native_archive as archive
finally:
    sys.path[:] = _ORIGINAL_PATH


class _Response:
    """Small external response protocol with an optional genuinely blocking read."""

    def __init__(self, delay: float = 0) -> None:
        """Retain a delay without interpreting or accepting any installer data."""
        self.delay = delay
        self.closed = False

    def getcode(self) -> int:
        """Expose a successful HTTP status so the body boundary is reachable."""
        return 200

    def geturl(self) -> str:
        """Return the same literal safe URL the downloader requested."""
        return "https://example.invalid/probe"

    def info(self) -> dict:
        """No Content-Length header can rescue the body-timeout control."""
        return {}

    def read(self, _size: int) -> bytes:
        """Block inside one read, then report EOF unless a real deadline interrupts it."""
        time.sleep(self.delay)
        return b""

    def __enter__(self):
        """Expose the normal response context protocol."""
        return self

    def __exit__(self, *_error) -> None:
        """Record cleanup even when the downloader interrupts a blocking read."""
        self.closed = True


class _Opener:
    """External open boundary that can block before returning response headers."""

    def __init__(self, response: _Response, delay: float = 0) -> None:
        """Retain one response and an optional header/open delay."""
        self.response = response
        self.delay = delay

    def open(self, _request, *, timeout):
        """Accept only a finite deadline, then model a blocking open/header operation."""
        if not 0 < timeout < float("inf"):
            raise AssertionError("HTTP open needs a finite timeout")
        time.sleep(self.delay)
        return self.response


class NativeMetadataArchiveImplTests(unittest.TestCase):
    """Check internals without changing the independent public-shell oracle."""

    def setUp(self) -> None:
        """Own one temporary child and register cleanup before constructing any data."""
        parent = Path(os.environ.get("CODEOPS_TMPDIR") or os.environ.get("TMPDIR")
                      or str(_ROOT / "build/tmp"))
        parent.mkdir(parents=True, exist_ok=True)
        self.temporary = tempfile.TemporaryDirectory(prefix="native-boundary-", dir=parent)
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)

    def asset(self, data: bytes, entries: int, expanded: int) -> dict:
        """Supply an authenticated private-boundary record rather than bypass its digest."""
        return {"kind": "core", "bytes": len(data), "sha256": hashlib.sha256(data).hexdigest(),
                "entries": entries, "expanded_bytes": expanded}

    def test_should_reject_duplicate_nonfinite_and_excessive_json_integers(self) -> None:
        """The auxiliary API/receipt decoder is bounded independently of manifest parsing."""
        for data in (b'{"nested":{"x":1,"x":2}}', b'{"x":NaN}', b'{"x":1e999}',
                     b'{"x":123456789012345678901234567890}'):
            with self.subTest(data=data), self.assertRaises(ValueError):
                archive.decode_json(data, "probe")
        self.assertEqual(archive.decode_json(b'[{"unused":1.25}]', "API"), [{"unused": 1.25}])

    def test_should_reject_metadata_aliases_and_nonblocking_fifo_inputs(self) -> None:
        """A metadata filename must not redirect to another file or wait on a pipe writer."""
        target = self.root / "target.json"
        target.write_bytes(b"{}")
        alias = self.root / "alias.json"
        alias.symlink_to(target.name)
        fifo = self.root / "fifo.json"
        os.mkfifo(fifo)
        for path in (alias, fifo):
            with self.subTest(path=path.name), self.assertRaises((OSError, ValueError)):
                archive.read_json(path)
        self.assertEqual(archive.read_json(target), {})

    def test_should_bound_tar_metadata_reads_and_seek_offsets_before_allocating(self) -> None:
        """Unchecked PAX lengths and seeks cannot turn an input declaration into allocation."""
        source = self.root / "bounded"
        source.write_bytes(b"x" * 512)
        with source.open("rb") as stream:
            reader = archive._TarReader(stream)
            for size in (-1, archive.MAX_MANIFEST_BYTES + 1):
                with self.subTest(size=size), self.assertRaises(ValueError):
                    reader.read(size)
            for offset in (-1, 513):
                with self.subTest(offset=offset), self.assertRaises(ValueError):
                    reader.seek(offset)

    def test_should_accept_internal_links_but_reject_unsafe_modes_before_writing(self) -> None:
        """Required SDK-relative links remain usable without granting privileged file modes."""
        link = tarfile.TarInfo("relative")
        link.type, link.linkname, link.mode = tarfile.SYMTYPE, "nested/file", 0o777
        data = tar_bytes({"nested/file": b"abc"}, extras=((link, b""),))
        path = self.root / "safe.tar"
        path.write_bytes(data)
        destination = self.root / "safe"
        archive.extract_tar(path, destination, self.asset(data, 2, 3))
        self.assertEqual((destination / "relative").read_bytes(), b"abc")
        for mode in (0o666, 0o4755, 0o2755):
            with self.subTest(mode=mode):
                member = tarfile.TarInfo("unsafe")
                member.mode, member.size = mode, 1
                data = tar_bytes({"first": b"a"}, extras=((member, b"b"),))
                path.write_bytes(data)
                output = self.root / ("unsafe-" + str(mode))
                with self.assertRaises(ValueError):
                    archive.extract_tar(path, output, self.asset(data, 2, 2))
                self.assertFalse(output.exists(), "validation must precede every destination write")

    def test_should_reject_inconsistent_inventory_without_creating_staging(self) -> None:
        """Header count and expanded-byte claims are checked against actual authenticated bytes."""
        data = tar_bytes({"one": b"abc"})
        path = self.root / "payload.tar"
        path.write_bytes(data)
        for count, expanded in ((2, 3), (1, 2), (True, 3), (1, 2 ** 62)):
            with self.subTest(count=count, expanded=expanded):
                output = self.root / "uncreated"
                with self.assertRaises(ValueError):
                    archive.extract_tar(path, output, self.asset(data, count, expanded))
                self.assertFalse(output.exists())

    def test_should_reject_vsix_special_members_and_crc_corruption(self) -> None:
        """Reading rather than extracting a ZIP still requires safe types and valid bodies."""
        path = self.root / "extension.vsix"
        data = zip_bytes({"extension/package.json": b'{"name":"clorraine"}',
                          "extension/body.txt": b"body"})
        path.write_bytes(data)
        package, count, expanded = archive.inspect_vsix(path)
        self.assertEqual(package, {"name": "clorraine"})
        self.assertEqual((count, expanded), (2, len(b'{"name":"clorraine"}') + 4))
        malicious = io.BytesIO()
        with zipfile.ZipFile(malicious, "w") as output:
            member = zipfile.ZipInfo("extension/socket")
            member.create_system = 3
            member.external_attr = 0o140644 << 16
            output.writestr(member, b"x")
        path.write_bytes(malicious.getvalue())
        with self.assertRaises(ValueError):
            archive.inspect_vsix(path)
        corrupted = bytearray(data)
        with zipfile.ZipFile(io.BytesIO(data)) as original:
            member = original.getinfo("extension/body.txt")
        header = member.header_offset
        offset = (header + 30 + int.from_bytes(data[header + 26:header + 28], "little")
                  + int.from_bytes(data[header + 28:header + 30], "little"))
        corrupted[offset] ^= 1
        path.write_bytes(corrupted)
        with self.assertRaises((ValueError, zipfile.BadZipFile)):
            archive.inspect_vsix(path)

    def timed_transfer(self, *, open_delay: float = 0, read_delay: float = 0) -> None:
        """Require a real wall-clock bound around a single blocking external operation."""
        response = _Response(read_delay)
        opener = _Opener(response, open_delay)
        handler = signal.getsignal(signal.SIGALRM)
        self.assertEqual(signal.getitimer(signal.ITIMER_REAL)[0], 0)
        started = time.monotonic()
        with patch.object(archive, "_SECONDS", 0.05), \
                patch.object(archive.ssl, "create_default_context", return_value=object()), \
                patch.object(archive, "build_opener", return_value=opener):
            with self.assertRaisesRegex(ValueError, "deadline"):
                archive._download("https://example.invalid/probe", io.BytesIO(), "probe", 1024)
        self.assertLess(time.monotonic() - started, 0.2)
        self.assertEqual(signal.getsignal(signal.SIGALRM), handler)
        self.assertEqual(signal.getitimer(signal.ITIMER_REAL)[0], 0)
        if read_delay:
            self.assertTrue(response.closed)

    def test_should_enforce_total_budget_inside_a_blocking_open(self) -> None:
        """A socket's per-read timeout alone is not an overall HTTP-header deadline."""
        self.timed_transfer(open_delay=0.3)

    def test_should_enforce_total_budget_inside_a_blocking_body_read(self) -> None:
        """A trickled read ending at EOF cannot bypass the transfer's total time budget."""
        self.timed_transfer(read_delay=0.3)

    def test_should_not_disarm_an_existing_process_timer(self) -> None:
        """A library boundary must fail closed rather than steal another caller's alarm."""
        previous = signal.getsignal(signal.SIGALRM)
        self.assertEqual(signal.getitimer(signal.ITIMER_REAL)[0], 0)
        try:
            signal.signal(signal.SIGALRM, lambda *_event: None)
            signal.setitimer(signal.ITIMER_REAL, 2)
            with patch.object(archive.ssl, "create_default_context", return_value=object()), \
                    patch.object(archive, "build_opener", return_value=_Opener(_Response())):
                with self.assertRaisesRegex(ValueError, "timer|alarm"):
                    archive._download("https://example.invalid/probe", io.BytesIO(), "probe", 1024)
            self.assertGreater(signal.getitimer(signal.ITIMER_REAL)[0], 0)
        finally:
            signal.setitimer(signal.ITIMER_REAL, 0)
            signal.signal(signal.SIGALRM, previous)

    def test_should_clear_its_timer_after_success_or_transport_failure(self) -> None:
        """A completed transfer must leave no alarm that can interrupt later installer work."""
        previous = signal.getsignal(signal.SIGALRM)
        self.assertEqual(signal.getitimer(signal.ITIMER_REAL)[0], 0)
        for failure in (False, True):
            with self.subTest(failure=failure), \
                    patch.object(archive.ssl, "create_default_context", return_value=object()), \
                    patch.object(archive, "build_opener") as factory:
                factory.return_value = _Opener(_Response())
                if failure:
                    with patch.object(factory.return_value, "open", side_effect=OSError("probe")):
                        with self.assertRaises(ValueError):
                            archive._download("https://example.invalid/probe", io.BytesIO(),
                                              "probe", 1024)
                else:
                    self.assertEqual(archive._download(
                        "https://example.invalid/probe", io.BytesIO(), "probe", 1024), {})
            self.assertEqual(signal.getitimer(signal.ITIMER_REAL)[0], 0)
            self.assertEqual(signal.getsignal(signal.SIGALRM), previous)

    def test_should_sanitize_producer_usage_diagnostics_before_argument_parsing(self) -> None:
        """Even unknown options must not inject terminal controls through argparse errors."""
        arguments = ["assemble", "--release", "v1.2.0", "--components", "a", "--support", "b",
                     "--core", "missing", "--vsix", "missing", "--support-inputs", "missing",
                     "--output", str(self.root / "uncreated")]
        for control in ("\x1b", "\x07", "\x7f", "\u202e"):
            with self.subTest(control=repr(control)):
                result = run_process(
                    [sys.executable, str(_ROOT / "tools/distribution/package_release.py"),
                     *arguments, "--unknown=" + control + "unsafe"], self.root,
                    {**os.environ, "PYTHONDONTWRITEBYTECODE": "1"}, timeout=15)
                self.assertNotEqual(result.returncode, 0)
                self.assertNotIn(control, result.stdout + result.stderr)
                self.assertFalse((self.root / "uncreated").exists())
