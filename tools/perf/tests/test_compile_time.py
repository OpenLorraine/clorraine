"""Unit tests for the compiler-throughput harness.

The tests use synthetic documents and generated source text; they do not
compile the corpus or need the SDK toolchain. The real measurement run is the
``make bench-compile`` target.
"""

from __future__ import annotations

import json
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import compile_time  # noqa: E402


def document(workloads: dict) -> dict:
    """Build a schema-complete synthetic result document."""
    return {
        "schema": 1,
        "label": "test",
        "compiler": {"git": "test", "sha256": "test", "path": "test", "version": "test"},
        "toolchain": {"host_cc": "test", "python": "3"},
        "runs": 3,
        "workloads": workloads,
    }


def workload(median_ns: int, rss: int = 100) -> dict:
    """Build one synthetic workload record."""
    return {
        "median_ns": median_ns,
        "min_ns": median_ns,
        "max_ns": median_ns,
        "median_cpu_ns": median_ns,
        "min_cpu_ns": median_ns,
        "max_cpu_ns": median_ns,
        "max_rss_kb": rss,
        "exit_codes": [0],
    }


class GeneratedInputTests(unittest.TestCase):
    """The generated shapes stay deterministic and match their names."""

    def test_function_farm_has_the_requested_function_count(self) -> None:
        source = compile_time.function_farm(250)
        self.assertEqual(source.count("func work"), 250)
        self.assertIn("func main(): i32", source)

    def test_call_farm_has_the_requested_call_count(self) -> None:
        source = compile_time.call_farm(40)
        self.assertEqual(source.count("target("), 41)

    def test_statement_and_local_farms_scale_with_their_arguments(self) -> None:
        statements = compile_time.statement_farm(40)
        self.assertEqual(statements.count("total = total +"), 40)
        locals_source = compile_time.local_farm(40)
        self.assertEqual(locals_source.count("let local"), 40)
        self.assertEqual(locals_source.count("total = total + local"), 40)

    def test_generated_sources_are_deterministic(self) -> None:
        first = compile_time.generated_sources()
        second = compile_time.generated_sources()
        self.assertEqual(first, second)
        self.assertEqual(
            set(first),
            {
                "functions-1000",
                "functions-2000",
                "functions-4000",
                "functions-8000",
                "calls-4000",
                "statements-4000",
                "locals-4000",
                "mixed-4000",
            },
        )

    def test_mixed_farm_calls_every_function(self) -> None:
        source = compile_time.mixed_farm(30)
        self.assertEqual(source.count("func work"), 30)
        self.assertEqual(source.count("total = total + work"), 30)

    def test_generated_names_avoid_the_float_keywords(self) -> None:
        for source in compile_time.generated_sources().values():
            self.assertNotIn("func f32", source)
            self.assertNotIn("func f64", source)

    def test_write_generated_inputs_round_trips(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            paths = compile_time.write_generated_inputs(Path(temporary))
            self.assertEqual(len(paths), 8)
            for name, path in paths.items():
                self.assertTrue(path.is_file(), name)
                self.assertEqual(path.name, f"{name}.clo")


class SummaryTests(unittest.TestCase):
    """One workload record reports the center and the bounds."""

    def test_summarize_uses_the_median_and_keeps_the_bounds(self) -> None:
        record = compile_time.summarize(
            [300, 100, 200], [30, 10, 20], max_rss_kb=12, codes=[0, 0, 1]
        )
        self.assertEqual(record["median_ns"], 200)
        self.assertEqual(record["min_ns"], 100)
        self.assertEqual(record["max_ns"], 300)
        self.assertEqual(record["median_cpu_ns"], 20)
        self.assertEqual(record["max_rss_kb"], 12)
        self.assertEqual(record["exit_codes"], [0, 1])

    def test_summarize_rejects_an_empty_sample_set(self) -> None:
        with self.assertRaises(compile_time.ThroughputError):
            compile_time.summarize([], [], max_rss_kb=0, codes=[])

    def test_percent_change_is_signed(self) -> None:
        self.assertEqual(compile_time.percent_change(100, 50), -50.0)
        self.assertEqual(compile_time.percent_change(100, 150), 50.0)
        self.assertEqual(compile_time.percent_change(0, 150), 0.0)


class MeasurementTests(unittest.TestCase):
    """The fork/wait4 path returns exit codes, timing, and peak memory."""

    def test_measure_reports_success_and_memory(self) -> None:
        record = compile_time.measure(
            [sys.executable, "-c", "pass"], runs=1, timeout=30
        )
        self.assertEqual(record["exit_codes"], [0])
        self.assertGreater(record["median_ns"], 0)
        self.assertGreater(record["median_cpu_ns"], 0)
        self.assertGreater(record["max_rss_kb"], 0)

    def test_measure_reports_a_failing_exit_code(self) -> None:
        record = compile_time.measure(
            [sys.executable, "-c", "raise SystemExit(3)"], runs=1, timeout=30
        )
        self.assertEqual(record["exit_codes"], [3])


class CompareTests(unittest.TestCase):
    """Base/head comparison computes deltas and enforces an optional limit."""

    def test_compare_rows_cover_every_workload(self) -> None:
        base = document({"a": workload(100), "b": workload(200, rss=10)})
        head = document({"a": workload(50), "b": workload(250, rss=20)})
        rows = compile_time.compare_rows(base, head)
        self.assertEqual([row[0] for row in rows], ["a", "b"])
        self.assertEqual(rows[0][3], -50.0)
        self.assertEqual(rows[1][3], 25.0)
        self.assertEqual((rows[1][4], rows[1][5]), (10, 20))

    def test_compare_rows_can_use_the_minimum(self) -> None:
        base = document({"a": workload(100)})
        head = document({"a": workload(200)})
        base["workloads"]["a"]["min_ns"] = 80
        head["workloads"]["a"]["min_ns"] = 60
        rows = compile_time.compare_rows(base, head, metric="min_ns")
        self.assertEqual(rows[0][1:3], (80.0, 60.0))
        self.assertEqual(rows[0][3], -25.0)

    def test_compare_rows_can_use_cpu_time(self) -> None:
        base = document({"a": workload(100)})
        head = document({"a": workload(200)})
        base["workloads"]["a"]["median_cpu_ns"] = 70
        head["workloads"]["a"]["median_cpu_ns"] = 35
        rows = compile_time.compare_rows(base, head, metric="median_cpu_ns")
        self.assertEqual(rows[0][1:3], (70.0, 35.0))
        self.assertEqual(rows[0][3], -50.0)

    def test_compare_rows_rejects_an_unknown_metric(self) -> None:
        base = document({"a": workload(100)})
        with self.assertRaises(compile_time.ThroughputError):
            compile_time.compare_rows(base, base, metric="max_ns")

    def test_compare_rows_rejects_different_workload_sets(self) -> None:
        base = document({"a": workload(100)})
        head = document({"b": workload(100)})
        with self.assertRaises(compile_time.ThroughputError):
            compile_time.compare_rows(base, head)

    def test_compare_rejects_a_failed_fast_head(self) -> None:
        base = document({"a": workload(100)})
        head = document({"a": workload(1)})
        head["workloads"]["a"]["exit_codes"] = [1]
        with tempfile.TemporaryDirectory() as temporary:
            base_path = Path(temporary) / "base.json"
            head_path = Path(temporary) / "head.json"
            base_path.write_text(json.dumps(base), encoding="utf-8")
            head_path.write_text(json.dumps(head), encoding="utf-8")
            args = compile_time.parse_args(
                ["compare", str(base_path), str(head_path)]
            )
            with self.assertRaises(compile_time.ThroughputError):
                compile_time.command_compare(args)

    def test_compare_rejects_a_changed_corpus_split(self) -> None:
        base = document(
            {
                "corpus": {
                    "files": 1,
                    "ok": 1,
                    "failed": 0,
                    "median_ns": 100,
                    "min_ns": 100,
                    "max_ns": 100,
                    "median_cpu_ns": 90,
                    "min_cpu_ns": 90,
                    "max_cpu_ns": 90,
                    "max_rss_kb": 10,
                    "exit_codes": [0, 1],
                    "items": {"a.clo": {"exit_codes": [0]}},
                }
            }
        )
        head = document(
            {
                "corpus": {
                    "files": 1,
                    "ok": 0,
                    "failed": 1,
                    "median_ns": 100,
                    "min_ns": 100,
                    "max_ns": 100,
                    "median_cpu_ns": 90,
                    "min_cpu_ns": 90,
                    "max_cpu_ns": 90,
                    "max_rss_kb": 10,
                    "exit_codes": [0, 1],
                    "items": {"a.clo": {"exit_codes": [1]}},
                }
            }
        )
        with tempfile.TemporaryDirectory() as temporary:
            base_path = Path(temporary) / "base.json"
            head_path = Path(temporary) / "head.json"
            base_path.write_text(json.dumps(base), encoding="utf-8")
            head_path.write_text(json.dumps(head), encoding="utf-8")
            args = compile_time.parse_args(
                ["compare", str(base_path), str(head_path)]
            )
            with self.assertRaises(compile_time.ThroughputError):
                compile_time.command_compare(args)

    def test_compare_rejects_different_toolchains(self) -> None:
        base = document({"a": workload(100)})
        head = document({"a": workload(100)})
        head["toolchain"] = {"host_cc": "other", "python": "3"}
        with tempfile.TemporaryDirectory() as temporary:
            base_path = Path(temporary) / "base.json"
            head_path = Path(temporary) / "head.json"
            base_path.write_text(json.dumps(base), encoding="utf-8")
            head_path.write_text(json.dumps(head), encoding="utf-8")
            args = compile_time.parse_args(
                ["compare", str(base_path), str(head_path)]
            )
            with self.assertRaises(compile_time.ThroughputError):
                compile_time.command_compare(args)

    def test_compare_command_fails_only_past_the_limit(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            base_path = Path(temporary) / "base.json"
            head_path = Path(temporary) / "head.json"
            base_path.write_text(
                json.dumps(document({"a": workload(100)})), encoding="utf-8"
            )
            head_path.write_text(
                json.dumps(document({"a": workload(110)})), encoding="utf-8"
            )
            args = compile_time.parse_args(
                ["compare", "--limit", str(5), str(base_path), str(head_path)]
            )
            self.assertEqual(compile_time.command_compare(args), 1)
            args = compile_time.parse_args(
                ["compare", "--limit", str(20), str(base_path), str(head_path)]
            )
            self.assertEqual(compile_time.command_compare(args), 0)


class DocumentTests(unittest.TestCase):
    """Result documents are validated before comparison."""

    def write(self, value: dict) -> Path:
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        path = Path(temporary.name) / "result.json"
        path.write_text(json.dumps(value), encoding="utf-8")
        return path

    def test_load_document_accepts_a_complete_document(self) -> None:
        path = self.write(document({"a": workload(1)}))
        self.assertEqual(compile_time.load_document(path)["schema"], 1)

    def test_load_document_rejects_a_missing_field(self) -> None:
        value = document({"a": workload(1)})
        del value["workloads"]
        with self.assertRaises(compile_time.ThroughputError):
            compile_time.load_document(self.write(value))

    def test_load_document_rejects_an_unsupported_schema(self) -> None:
        value = document({"a": workload(1)})
        value["schema"] = 99
        with self.assertRaises(compile_time.ThroughputError):
            compile_time.load_document(self.write(value))


if __name__ == "__main__":
    unittest.main()
