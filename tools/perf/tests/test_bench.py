"""Unit tests for the benchmark harness.

The tests use synthetic result documents and a temporary fake compiler. They
do not invoke the SDK toolchain, so they run in the fast check.
"""

from __future__ import annotations

import argparse
import contextlib
import io
import json
import os
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import bench  # noqa: E402


def make_document(
    *,
    label: str,
    instructions: int,
    text: int,
    c_bytes: int,
    output: str = "sieve: 1\n",
    toolchain: str = "same",
    runs: int = 1,
) -> dict:
    """Build a schema-complete synthetic result document."""
    return {
        "schema": 1,
        "label": label,
        "compiler": {"git": "test", "sha256": "test", "path": "test", "version": "test"},
        "harness_git": "test",
        "toolchain": {"host_cc": toolchain, "m68k_gcc": toolchain, "vamos": toolchain, "python": "3"},
        "runs": runs,
        "profiles": {"host": "host", "m68k": "hosted-amiga"},
        "flags": {"host_debug": "x", "host_release": "x", "m68k_debug": "x", "m68k_release": "x"},
        "calibration": {"instructions": 10, "host_release_ns": 1},
        "benchmarks": {
            "sieve": {
                "c_bytes": c_bytes,
                "c_lines": 1,
                "host_c_bytes": c_bytes,
                "host_c_lines": 1,
                "host_debug_bytes": 1,
                "host_release_bytes": 1,
                "host_debug_ns": 1,
                "host_release_ns": 1,
                "host_release_median_ns": 1,
                "host_release_net_ns": 1,
                "m68k_debug": {"text": text, "data": 1, "bss": 1},
                "m68k_release": {"text": text, "data": 1, "bss": 1},
                "instructions_raw": instructions,
                "instructions_net": instructions,
                "output_sha256": output,
            },
            "matrix": {
                "c_bytes": c_bytes,
                "c_lines": 1,
                "host_c_bytes": c_bytes,
                "host_c_lines": 1,
                "host_debug_bytes": 1,
                "host_release_bytes": 1,
                "host_debug_ns": 1,
                "host_release_ns": 1,
                "host_release_median_ns": 1,
                "host_release_net_ns": 1,
                "m68k_debug": {"text": text, "data": 1, "bss": 1},
                "m68k_release": {"text": text, "data": 1, "bss": 1},
                "instructions_raw": instructions,
                "instructions_net": instructions,
                "output_sha256": output,
            },
        },
    }


class TempFiles(unittest.TestCase):
    """Helpers for writing result documents."""

    def write(self, document: dict) -> Path:
        """Write one document under a temporary directory."""
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        path = Path(self.temp.name) / "result.json"
        path.write_text(json.dumps(document), encoding="utf-8")
        return path


class NormalizationTests(unittest.TestCase):
    """stdout normalization and path safety."""

    def test_normalize_converts_crlf_and_lone_cr(self) -> None:
        self.assertEqual(bench.normalize_output(b"a\r\nb\rc\n\n"), "a\nb\nc\n")

    def test_normalize_keeps_one_trailing_newline(self) -> None:
        self.assertEqual(bench.normalize_output(b"a\n"), "a\n")
        self.assertEqual(bench.normalize_output(b"a"), "a\n")

    def test_label_rejects_traversal(self) -> None:
        for label in ("..", ".", "a/b", "a\\b", ""):
            with self.assertRaises(bench.HarnessError):
                bench.validate_label(label)

    def test_label_accepts_safe_component(self) -> None:
        bench.validate_label("head-2026.11.07_1")

    def test_confined_rejects_outside_path(self) -> None:
        with self.assertRaises(bench.HarnessError):
            bench.confined(Path("/tmp/elsewhere.json"), bench.BUILD)


class ParsingTests(unittest.TestCase):
    """Size and delta parsing."""

    def test_parse_size_row(self) -> None:
        output = "   text\t   data\t    bss\t    dec\t    hex\tfilename\n 101396\t    304\t    600\t  102300\t   18f9c\tbuild/x\n"
        self.assertEqual(
            bench.parse_size_output(output),
            {"text": 101396, "data": 304, "bss": 600},
        )

    def test_parse_size_rejects_unknown(self) -> None:
        with self.assertRaises(bench.HarnessError):
            bench.parse_size_output("no table here")

    def test_format_delta(self) -> None:
        self.assertEqual(bench.format_delta(100, 80), "-20.0%")
        self.assertEqual(bench.format_delta(100, 120), "+20.0%")
        self.assertEqual(bench.format_delta(None, 10), "n/a")


class DocumentTests(TempFiles):
    """Schema validation and structural checks."""

    def test_load_accepts_complete_document(self) -> None:
        path = self.write(make_document(label="head", instructions=1, text=1, c_bytes=1))
        self.assertEqual(bench.load_document(path)["schema"], 1)

    def test_load_rejects_missing_field(self) -> None:
        document = make_document(label="head", instructions=1, text=1, c_bytes=1)
        del document["toolchain"]
        path = self.write(document)
        with self.assertRaises(bench.HarnessError):
            bench.load_document(path)

    def test_load_rejects_wrong_schema(self) -> None:
        document = make_document(label="head", instructions=1, text=1, c_bytes=1)
        document["schema"] = 99
        path = self.write(document)
        with self.assertRaises(bench.HarnessError):
            bench.load_document(path)

    def test_require_same_rejects_different_toolchain(self) -> None:
        base = make_document(label="base", instructions=1, text=1, c_bytes=1, toolchain="a")
        head = make_document(label="head", instructions=1, text=1, c_bytes=1, toolchain="b")
        with self.assertRaises(bench.HarnessError):
            bench.require_same(base, head, "toolchain")


class CompareTests(TempFiles):
    """The comparison gate."""

    def compare(self, base: dict, head: dict) -> int:
        """Write both documents and run the comparator."""
        base_path = self.write(base)
        head_path = Path(self.temp.name) / "head.json"
        head_path.write_text(json.dumps(head), encoding="utf-8")
        args = argparse.Namespace(base=str(base_path), head=str(head_path))
        with contextlib.redirect_stdout(io.StringIO()):
            return bench.command_compare(args)

    def test_compare_passes_on_targets(self) -> None:
        base = make_document(label="base", instructions=1000, text=1000, c_bytes=1000)
        head = make_document(label="head", instructions=700, text=800, c_bytes=700)
        self.assertEqual(self.compare(base, head), 0)

    def test_compare_fails_on_output(self) -> None:
        base = make_document(label="base", instructions=1000, text=1000, c_bytes=1000)
        head = make_document(label="head", instructions=700, text=800, c_bytes=700, output="other\n")
        self.assertEqual(self.compare(base, head), 1)

    def test_compare_fails_on_instruction_regression(self) -> None:
        base = make_document(label="base", instructions=1000, text=1000, c_bytes=1000)
        head = make_document(label="head", instructions=1100, text=800, c_bytes=700)
        self.assertEqual(self.compare(base, head), 1)

    def test_compare_fails_on_text_regression(self) -> None:
        base = make_document(label="base", instructions=1000, text=1000, c_bytes=1000)
        head = make_document(label="head", instructions=700, text=1100, c_bytes=700)
        self.assertEqual(self.compare(base, head), 1)

    def test_compare_fails_on_c_byte_target(self) -> None:
        base = make_document(label="base", instructions=1000, text=1000, c_bytes=1000)
        head = make_document(label="head", instructions=700, text=800, c_bytes=900)
        self.assertEqual(self.compare(base, head), 1)

    def test_compare_fails_on_missing_trace(self) -> None:
        base = make_document(label="base", instructions=1000, text=1000, c_bytes=1000)
        head = make_document(label="head", instructions=700, text=800, c_bytes=700)
        head["benchmarks"]["sieve"]["instructions_net"] = None
        self.assertEqual(self.compare(base, head), 1)

    def test_compare_fails_on_structural_mismatch(self) -> None:
        base = make_document(label="base", instructions=1000, text=1000, c_bytes=1000)
        head = make_document(label="head", instructions=700, text=800, c_bytes=700, runs=2)
        with self.assertRaises(bench.HarnessError):
            self.compare(base, head)


class FakeCompilerTests(unittest.TestCase):
    """A compiler that fails must stop the run with its exact command."""

    def test_fake_compiler_failure(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            fake = Path(temporary) / "clc"
            fake.write_text("#!/bin/sh\necho 'fake compiler failure' >&2\nexit 1\n", encoding="utf-8")
            fake.chmod(0o755)
            with self.assertRaises(bench.HarnessError) as caught:
                bench.emit(fake, "host", Path("program.clo"), Path(temporary) / "out.c")
            self.assertIn("fake compiler failure", str(caught.exception))

    def test_host_timeout(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            slow = Path(temporary) / "slow"
            slow.write_text("#!/bin/sh\nsleep 5\n", encoding="utf-8")
            slow.chmod(0o755)
            with self.assertRaises(bench.HarnessError):
                bench.run_host(slow, timeout=0.2)

    def test_trace_timeout_kills_a_hung_vamos(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            fake = Path(temporary) / "vamos"
            fake.write_text("#!/bin/sh\nexec sleep 5\n", encoding="utf-8")
            fake.chmod(0o755)
            previous_path = os.environ.get("PATH", "")
            previous_timeout = bench.TRACE_TIMEOUT
            os.environ["PATH"] = temporary + os.pathsep + previous_path
            bench.TRACE_TIMEOUT = 0.2
            try:
                with self.assertRaises(bench.HarnessError):
                    bench.trace_vamos(Path(temporary) / "program")
            finally:
                bench.TRACE_TIMEOUT = previous_timeout
                os.environ["PATH"] = previous_path


if __name__ == "__main__":
    unittest.main()
