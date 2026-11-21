"""Runtime-only Greenlet wheel selection; fixtures are ZIP data, not native proof."""

from __future__ import annotations

import base64
import csv
import hashlib
import importlib.util
import io
import os
from pathlib import Path
import shutil
import tempfile
import unittest
import zipfile


class SupportWheelSelectionTests(unittest.TestCase):
    """Keep test-only exclusion separate from byte/metadata preservation guarantees."""

    def setUp(self) -> None:
        """Build real ZIP members with fixed metadata and an independent expected boundary."""
        scratch = tempfile.TemporaryDirectory(
            prefix="wheel-selection-spec-", dir=os.environ.get("CODEOPS_TMPDIR"))
        self.addCleanup(scratch.cleanup)
        self.root = Path(scratch.name)
        self.path = self.root / "greenlet-3.5.6-cp310-cp310-linux_x86_64.whl"
        self.record = "greenlet-3.5.6.dist-info/RECORD"
        self.contents = {
            "greenlet/__init__.py": b"from ._greenlet import greenlet\n",
            "greenlet/_greenlet.cpython-310-x86_64-linux-gnu.so": b"Native-shaped filename only.\n",
            "greenlet/greenlet.h": b"/* Public header fixture. */\n",
            "greenlet/tests/__init__.py": b"import psutil\n",
            "greenlet/tests/_test_extension.cpython-310-x86_64-linux-gnu.so": b"Test fixture.\n",
            "greenlet/tests/nested/input.txt": b"Nested test data.\n",
            "greenlet/tests_extra.py": b"Preserve this non-subtree member.\n",
            "greenlet-3.5.6.dist-info/METADATA":
                b"Metadata-Version: 2.4\nName: greenlet\nVersion: 3.5.6\n"
                b"Requires-Dist: psutil; extra == \"test\"\n\n",
            "greenlet-3.5.6.dist-info/WHEEL":
                b"Wheel-Version: 1.0\nRoot-Is-Purelib: false\nTag: cp310-cp310-linux_x86_64\n",
            "greenlet-3.5.6.dist-info/licenses/LICENSE": b"Original MIT notice fixture.\n",
            "greenlet-3.5.6.dist-info/licenses/LICENSE.PSF": b"Original PSF notice fixture.\n",
            self.record: b"Old RECORD fixture, to be regenerated.\n",
        }
        self._write()
        recipe = Path(__file__).resolve().parents[1] / "inputs/wheel-payload.py"
        spec = importlib.util.spec_from_file_location("wheel_selection_recipe", recipe)
        self.assertIsNotNone(spec)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        self.selector = getattr(module, "select_greenlet_runtime", None)

    def _write(self) -> None:
        """Create predictable ZIP metadata that the selection may not silently discard."""
        with zipfile.ZipFile(self.path, "w") as archive:
            archive.comment = b"Preserved wheel comment."
            for name, content in self.contents.items():
                info = zipfile.ZipInfo(name, date_time=(2020, 3, 4, 5, 6, 8))
                info.compress_type = zipfile.ZIP_DEFLATED
                info.create_system = 3
                info.external_attr = 0o100644 << 16
                info.extra = b"\xfe\xca\x02\x00ok"
                info.comment = b"Member comment."
                archive.writestr(info, content)

    def _select(self, path: Path | None = None) -> None:
        """Invoke the actual narrowly scoped producer operation."""
        self.assertIsNotNone(self.selector, "Missing Greenlet runtime-only selection")
        self.selector(self.path if path is None else path)

    def test_should_exclude_only_the_exact_greenlet_test_subtree(self) -> None:
        """Remove test code/native members recursively, not similarly named runtime files."""
        self._select()
        with zipfile.ZipFile(self.path) as archive:
            self.assertEqual(set(archive.namelist()), {
                name for name in self.contents if not name.startswith("greenlet/tests/")})
            self.assertIn("greenlet/tests_extra.py", archive.namelist())

    def test_should_preserve_all_other_member_bytes_and_zip_metadata(self) -> None:
        """Runtime code, public headers, wheel metadata and both licences remain unchanged."""
        self._select()
        with zipfile.ZipFile(self.path) as archive:
            self.assertEqual(archive.comment, b"Preserved wheel comment.")
            for member in archive.infolist():
                with self.subTest(member=member.filename):
                    if member.filename != self.record:
                        self.assertEqual(archive.read(member), self.contents[member.filename])
                    self.assertEqual(member.date_time, (2020, 3, 4, 5, 6, 8))
                    self.assertEqual(member.compress_type, zipfile.ZIP_DEFLATED)
                    self.assertEqual(member.external_attr, 0o100644 << 16)
                    self.assertEqual(member.extra, b"\xfe\xca\x02\x00ok")
                    self.assertEqual(member.comment, b"Member comment.")

    def test_should_regenerate_complete_record_hashes_and_sizes(self) -> None:
        """The installed-file integrity table covers precisely the surviving wheel files."""
        self._select()
        with zipfile.ZipFile(self.path) as archive:
            rows = list(csv.reader(io.StringIO(archive.read(self.record).decode("utf-8"))))
            self.assertEqual(len(rows), len(archive.namelist()))
            self.assertEqual({row[0] for row in rows}, set(archive.namelist()))
            for name, digest, size in rows:
                if name == self.record:
                    self.assertEqual((digest, size), ("", ""))
                else:
                    data = archive.read(name)
                    expected = base64.urlsafe_b64encode(hashlib.sha256(data).digest()).rstrip(b"=")
                    self.assertEqual(digest, "sha256=" + expected.decode("ascii"))
                    self.assertEqual(size, str(len(data)))

    def test_should_produce_identical_selected_bytes_from_identical_inputs(self) -> None:
        """Output hashes cannot depend on wall time or the private sibling filename."""
        second = self.root / "second.whl"
        shutil.copyfile(self.path, second)
        self._select()
        self._select(second)
        self.assertEqual(self.path.read_bytes(), second.read_bytes())

    def test_should_reject_a_different_distribution_without_rewriting_it(self) -> None:
        """This is not a generic wheel filter or permission to modify another component."""
        self.contents["greenlet-3.5.6.dist-info/METADATA"] = (
            b"Metadata-Version: 2.4\nName: another-package\nVersion: 3.5.6\n\n")
        self._write()
        before = self.path.read_bytes()
        self.assertIsNotNone(self.selector, "Missing Greenlet runtime-only selection")
        with self.assertRaisesRegex(ValueError, "greenlet"):
            self.selector(self.path)
        self.assertEqual(self.path.read_bytes(), before)

    def test_should_reject_a_signed_record_without_discarding_its_signature(self) -> None:
        """A signed wheel requires separate authority; silently invalidating it is forbidden."""
        self.contents[self.record + ".jws"] = b"Signature fixture."
        self._write()
        before = self.path.read_bytes()
        self.assertIsNotNone(self.selector, "Missing Greenlet runtime-only selection")
        with self.assertRaisesRegex(ValueError, "signature"):
            self.selector(self.path)
        self.assertEqual(self.path.read_bytes(), before)

    def test_should_reject_a_missing_runtime_extension_before_rewriting(self) -> None:
        """Removing optional tests must never legitimise a wheel missing its real native API."""
        del self.contents["greenlet/_greenlet.cpython-310-x86_64-linux-gnu.so"]
        self._write()
        before = self.path.read_bytes()
        self.assertIsNotNone(self.selector, "Missing Greenlet runtime-only selection")
        with self.assertRaisesRegex(ValueError, "runtime"):
            self.selector(self.path)
        self.assertEqual(self.path.read_bytes(), before)
