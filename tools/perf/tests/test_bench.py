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
import re
import subprocess
import sys
import tempfile
import unittest
from unittest import mock
from decimal import Decimal
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

    def compare(self, base: dict, head: dict, require: list[str] | None = None) -> int:
        """Write both documents, run the comparator, and keep its stdout."""
        base_path = self.write(base)
        head_path = Path(self.temp.name) / "head.json"
        head_path.write_text(json.dumps(head), encoding="utf-8")
        args = argparse.Namespace(
            base=str(base_path),
            head=str(head_path),
            require=require or [],
        )
        self.output = io.StringIO()
        with contextlib.redirect_stdout(self.output):
            return bench.command_compare(args)

    def result(self, base: dict, head: dict, require: list[str] | None = None) -> tuple[int, str]:
        """Return the exit code and the captured comparison output."""
        code = self.compare(base, head, require)
        return code, self.output.getvalue()

    def test_compare_passes_on_flat_metrics(self) -> None:
        base = make_document(label="base", instructions=1000, text=1000, c_bytes=1000)
        head = make_document(label="head", instructions=1000, text=1000, c_bytes=1000)
        self.assertEqual(self.compare(base, head), 0)

    def test_compare_passes_on_met_instruction_requirement(self) -> None:
        base = make_document(label="base", instructions=1000, text=1000, c_bytes=1000)
        head = make_document(label="head", instructions=940, text=1000, c_bytes=900)
        self.assertEqual(
            self.compare(base, head, ["sieve:instructions:5"]),
            0,
        )

    def test_compare_passes_on_met_text_requirement(self) -> None:
        base = make_document(label="base", instructions=1000, text=1000, c_bytes=1000)
        head = make_document(label="head", instructions=1000, text=960, c_bytes=900)
        self.assertEqual(self.compare(base, head, ["sieve:text:3"]), 0)

    def test_compare_fails_on_missed_instruction_requirement(self) -> None:
        base = make_document(label="base", instructions=1000, text=1000, c_bytes=1000)
        head = make_document(label="head", instructions=980, text=1000, c_bytes=900)
        code, output = self.result(base, head, ["matrix:instructions:5"])
        self.assertEqual(code, 1)
        self.assertIn(
            "matrix: instructions -2.0% misses the required -5%",
            output,
        )
        self.assertIn("need <= -5%", output)

    def test_compare_fails_on_missed_text_requirement(self) -> None:
        base = make_document(label="base", instructions=1000, text=1000, c_bytes=1000)
        head = make_document(label="head", instructions=1000, text=990, c_bytes=900)
        code, output = self.result(base, head, ["matrix:text:3"])
        self.assertEqual(code, 1)
        self.assertIn(
            "matrix: text -1.0% misses the required -3%",
            output,
        )
        self.assertIn("need <= -3%", output)

    def test_compare_fails_flat_with_requirement(self) -> None:
        base = make_document(label="base", instructions=1000, text=1000, c_bytes=1000)
        head = make_document(label="head", instructions=1000, text=1000, c_bytes=1000)
        code, output = self.result(base, head, ["matrix:instructions:5"])
        self.assertEqual(code, 1)
        self.assertIn(
            "matrix: instructions +0.0% misses the required -5%",
            output,
        )
        self.assertIn("need <= -5%", output)

    def test_compare_checks_a_duplicate_requirement_twice(self) -> None:
        base = make_document(label="base", instructions=1000, text=1000, c_bytes=1000)
        head = make_document(label="head", instructions=940, text=1000, c_bytes=900)
        code, output = self.result(
            base,
            head,
            ["matrix:instructions:5", "matrix:instructions:5"],
        )
        self.assertEqual(code, 0)
        self.assertEqual(output.count("need <= -5%"), 2)

    def test_compare_accepts_an_exact_boundary(self) -> None:
        base = make_document(label="base", instructions=5, text=1000, c_bytes=1000)
        head = make_document(label="head", instructions=1, text=1000, c_bytes=1000)
        code, output = self.result(base, head, ["matrix:instructions:80"])
        self.assertEqual(code, 0)
        self.assertNotIn("FAIL", output)

    def test_compare_fails_a_tiny_positive_percentage(self) -> None:
        base = make_document(label="base", instructions=1000, text=1000, c_bytes=1000)
        head = make_document(label="head", instructions=1000, text=1000, c_bytes=1000)
        code, output = self.result(base, head, ["matrix:instructions:0.000001"])
        self.assertEqual(code, 1)
        self.assertIn("misses the required -0.000001%", output)

    def test_compare_ignores_c_byte_regression(self) -> None:
        base = make_document(label="base", instructions=1000, text=1000, c_bytes=1000)
        head = make_document(label="head", instructions=1000, text=1000, c_bytes=1200)
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

    def test_compare_rejects_unknown_benchmark(self) -> None:
        base = make_document(label="base", instructions=1000, text=1000, c_bytes=1000)
        head = make_document(label="head", instructions=700, text=800, c_bytes=700)
        with self.assertRaises(bench.HarnessError):
            self.compare(base, head, ["missing:instructions:5"])
        self.assertEqual(self.output.getvalue(), "")

    def test_compare_rejects_unknown_metric(self) -> None:
        base = make_document(label="base", instructions=1000, text=1000, c_bytes=1000)
        head = make_document(label="head", instructions=700, text=800, c_bytes=700)
        with self.assertRaises(bench.HarnessError):
            self.compare(base, head, ["matrix:bogus:5"])
        self.assertEqual(self.output.getvalue(), "")

    def test_compare_rejects_bad_percentage(self) -> None:
        base = make_document(label="base", instructions=1000, text=1000, c_bytes=1000)
        head = make_document(label="head", instructions=700, text=800, c_bytes=700)
        for value in (
            "0",
            "-5",
            "101",
            "1e1000000",
            "1e-7",
            "1e-15",
            "1e-27",
            "nan",
            "inf",
            "nope",
        ):
            with self.assertRaises(bench.HarnessError):
                self.compare(base, head, [f"matrix:instructions:{value}"])
            self.assertEqual(self.output.getvalue(), "")

    def test_compare_rejects_malformed_requirement(self) -> None:
        base = make_document(label="base", instructions=1000, text=1000, c_bytes=1000)
        head = make_document(label="head", instructions=700, text=800, c_bytes=700)
        with self.assertRaises(bench.HarnessError):
            self.compare(base, head, ["matrix:instructions"])
        self.assertEqual(self.output.getvalue(), "")

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


class RequirementParsingTests(unittest.TestCase):
    """The --require syntax and the Makefile expansion."""

    def test_parse_requirement_reads_from_the_right(self) -> None:
        self.assertEqual(
            bench.parse_requirement("matrix:instructions:5"),
            ("matrix", "instructions", Decimal("5")),
        )
        self.assertEqual(
            bench.parse_requirement("a:b:text:2.5"),
            ("a:b", "text", Decimal("2.5")),
        )

    def test_parse_args_preserves_a_colon_in_the_name(self) -> None:
        args = bench.parse_args(
            [
                "compare",
                "--require",
                "a:b:instructions:5",
                "base.json",
                "head.json",
            ]
        )
        self.assertEqual(args.require, ["a:b:instructions:5"])

    def test_parse_args_collects_repeated_require(self) -> None:
        args = bench.parse_args(
            [
                "compare",
                "--require",
                "matrix:instructions:5",
                "--require",
                "matrix:text:3",
                "base.json",
                "head.json",
            ]
        )
        self.assertEqual(
            args.require,
            ["matrix:instructions:5", "matrix:text:3"],
        )

    def test_parse_args_defaults_to_no_requirement(self) -> None:
        args = bench.parse_args(["compare", "base.json", "head.json"])
        self.assertEqual(args.require, [])

    def test_makefile_expands_two_requirements(self) -> None:
        completed = subprocess.run(
            ["make", "-n", "bench-compare",
             "REQUIRE=matrix:instructions:5 matrix:text:3"],
            cwd=str(bench.ROOT),
            capture_output=True,
            text=True,
            timeout=30,
        )
        self.assertEqual(completed.returncode, 0, completed.stderr)
        self.assertIn(
            "--require matrix:instructions:5 --require matrix:text:3",
            completed.stdout,
        )

    def test_makefile_expands_empty_requirement(self) -> None:
        completed = subprocess.run(
            ["make", "-n", "bench-compare"],
            cwd=str(bench.ROOT),
            capture_output=True,
            text=True,
            timeout=30,
        )
        self.assertEqual(completed.returncode, 0, completed.stderr)
        self.assertNotIn("--require", completed.stdout)


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


class MeasureTests(unittest.TestCase):
    """The result document records the linked release artifact size."""

    def test_measure_records_the_m68k_release_file_size(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)

            def fake_emit(clc, profile, source, output: Path) -> None:
                output.write_text("/* c */\n", encoding="utf-8")

            def fake_build(argv: list[str], timeout: int, cwd=None) -> None:
                binary = Path(argv[argv.index("-o") + 1])
                binary.write_bytes(b"0123456789")

            expected = "x\n"
            with mock.patch.object(bench, "emit", side_effect=fake_emit), \
                 mock.patch.object(bench, "build_step", side_effect=fake_build), \
                 mock.patch.object(bench, "run_host", return_value=expected), \
                 mock.patch.object(bench, "run_vamos", return_value=expected), \
                 mock.patch.object(bench, "time_host", return_value=(1, 1)), \
                 mock.patch.object(
                     bench,
                     "size_sections",
                     return_value={"text": 1, "data": 2, "bss": 3},
                 ), \
                 mock.patch.object(bench, "trace_vamos", return_value=(expected, 10)), \
                 mock.patch.object(bench, "expected_output", return_value=expected):
                result = bench.measure(Path("clc"), "calibration", root, 1, True)
        self.assertEqual(result["m68k_release_bytes"], 10)

    def test_the_calibration_summary_keeps_two_fields(self) -> None:
        self.assertEqual(
            bench.calibration_summary(
                {
                    "instructions_raw": 3,
                    "host_release_ns": 4,
                    "m68k_release_bytes": 5,
                }
            ),
            {"instructions": 3, "host_release_ns": 4},
        )


class ReleaseFlagTests(unittest.TestCase):
    """The scored harness and the driver describe the same shipped build."""

    def test_harness_release_vectors_are_the_shipped_set(self) -> None:
        shipped = [
            "-O2",
            "-ffunction-sections",
            "-fdata-sections",
            "-Wl,--gc-sections",
            "-s",
        ]
        self.assertEqual(bench.HOST_RELEASE, ["-std=c99", *shipped])
        self.assertEqual(bench.M68K_RELEASE, bench.M68K_BASE + shipped)

    def test_driver_constants_match_the_harness(self) -> None:
        shipped = [
            "-O2",
            "-ffunction-sections",
            "-fdata-sections",
            "-Wl,--gc-sections",
            "-s",
        ]
        self.assertEqual(
            ["-std=c99", *rust_release("HOST_RELEASE_GNU")], bench.HOST_RELEASE
        )
        self.assertEqual(
            bench.M68K_BASE + rust_release("M68K_RELEASE_FLAGS"), bench.M68K_RELEASE
        )
        self.assertEqual(rust_release("HOST_RELEASE_FALLBACK"), ["-O2"])
        self.assertEqual(rust_release("FREESTANDING_RELEASE_FLAGS"), ["-O2"])
        self.assertEqual(rust_release("M68K_RELEASE_FLAGS"), shipped)

    def test_binary_bytes_reads_and_wraps_missing(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "artifact"
            path.write_bytes(b"abc")
            self.assertEqual(bench.binary_bytes(path), 3)
            with self.assertRaises(bench.HarnessError):
                bench.binary_bytes(Path(temporary) / "missing")


def rust_release(name: str) -> list[str]:
    """Read one release-constant string slice from the driver source."""
    source = (bench.ROOT / "compiler/src/clorraine/toolchain.rs").read_text(
        encoding="utf-8"
    )
    match = re.search(rf"const {name}: &\[&str\] = &\[(.*?)\];", source, re.S)
    if match is None:
        raise AssertionError(f"missing Rust constant: {name}")
    return re.findall(r'"([^"]+)"', match.group(1))


if __name__ == "__main__":
    unittest.main()
