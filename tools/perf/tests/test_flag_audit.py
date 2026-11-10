"""Unit tests for the old-vs-shipped build-flag audit.

The tests use synthetic matrices; the real matrix is the ``make bench-flags``
gate, which needs the pinned SDK toolchain.
"""

from __future__ import annotations

import copy
import re
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import flag_audit  # noqa: E402

ROOT = Path(__file__).resolve().parents[3]


def entry(
    profile: str,
    kind: str,
    *,
    old_text: int = 1000,
    new_text: int = 500,
    old_bin: int = 1000,
    new_bin: int = 500,
    old_raw: int = 100,
    new_raw: int = 90,
    old_net: int = 1000,
    new_net: int = 1000,
    output: str = "same\n",
    host_symtab: bool = False,
) -> dict:
    """Build one program record with the audit matrix shape."""
    return {
        "profile": profile,
        "kind": kind,
        "host_symtab": host_symtab,
        "old": {
            "text": old_text,
            "bin": old_bin,
            "instructions_raw": old_raw,
            "instructions_net": old_net,
            "output": output,
        },
        "gc": {
            "text": new_text,
            "bin": new_bin,
            "instructions_raw": new_raw,
            "instructions_net": new_net,
            "output": output,
        },
        "new": {
            "text": new_text,
            "bin": new_bin,
            "instructions_raw": new_raw,
            "instructions_net": new_net,
            "output": output,
        },
    }


def matrix(entries: dict[str, dict]) -> dict:
    """Wrap program records in the audit document shape."""
    return {"schema": 1, "programs": entries}


def clean_matrix() -> dict:
    """A matrix where every floor passes."""
    return matrix(
        {
            "hosted-amiga/calibration": entry(
                "hosted-amiga", "calibration", old_raw=100, new_raw=90
            ),
            "hosted-amiga/lookup": entry("hosted-amiga", "scored"),
            "host/matrix": entry("host", "scored", old_bin=1000, new_bin=800),
            "hosted-amiga-13/file-copier": entry(
                "hosted-amiga-13", "corpus", old_text=1000, new_text=600
            ),
            "freestanding/program": entry(
                "freestanding",
                "freestanding",
                old_text=208,
                new_text=208,
                old_bin=1552,
                new_bin=1552,
            ),
        }
    )


class FloorTests(unittest.TestCase):
    """The size and runtime floors are exact."""

    def test_a_clean_matrix_passes(self) -> None:
        self.assertEqual(flag_audit.check_floors(clean_matrix()), [])

    def test_an_m68k_text_miss_is_reported(self) -> None:
        document = clean_matrix()
        document["programs"]["hosted-amiga/lookup"]["new"]["text"] = 801
        errors = flag_audit.check_floors(document)
        self.assertTrue(any("lookup" in error for error in errors), errors)

    def test_an_m68k_binary_miss_is_reported(self) -> None:
        document = clean_matrix()
        document["programs"]["hosted-amiga/lookup"]["new"]["bin"] = 801
        errors = flag_audit.check_floors(document)
        self.assertTrue(any("lookup" in error for error in errors), errors)

    def test_a_host_binary_miss_is_reported(self) -> None:
        document = clean_matrix()
        document["programs"]["host/matrix"]["new"]["bin"] = 851
        errors = flag_audit.check_floors(document)
        self.assertTrue(any("matrix" in error for error in errors), errors)

    def test_a_corpus_text_miss_is_reported(self) -> None:
        document = clean_matrix()
        document["programs"]["hosted-amiga-13/file-copier"]["new"]["text"] = 601
        errors = flag_audit.check_floors(document)
        self.assertTrue(any("file-copier" in error for error in errors), errors)

    def test_a_calibration_miss_is_reported(self) -> None:
        document = clean_matrix()
        document["programs"]["hosted-amiga/calibration"]["new"]["instructions_raw"] = 91
        errors = flag_audit.check_floors(document)
        self.assertTrue(any("calibration" in error for error in errors), errors)

    def test_a_net_instruction_miss_is_reported(self) -> None:
        document = clean_matrix()
        document["programs"]["hosted-amiga/lookup"]["new"]["instructions_net"] = 1011
        errors = flag_audit.check_floors(document)
        self.assertTrue(any("lookup" in error for error in errors), errors)

    def test_an_output_miss_is_reported(self) -> None:
        document = clean_matrix()
        document["programs"]["hosted-amiga/lookup"]["new"]["output"] = "different\n"
        errors = flag_audit.check_floors(document)
        self.assertTrue(any("lookup" in error for error in errors), errors)

    def test_a_freestanding_growth_is_reported(self) -> None:
        document = clean_matrix()
        document["programs"]["freestanding/program"]["new"]["text"] = 232
        errors = flag_audit.check_floors(document)
        self.assertTrue(any("freestanding" in error for error in errors), errors)

    def test_a_freestanding_binary_change_is_reported(self) -> None:
        document = clean_matrix()
        document["programs"]["freestanding/program"]["new"]["bin"] = 1553
        errors = flag_audit.check_floors(document)
        self.assertTrue(any("freestanding" in error for error in errors), errors)

    def test_a_host_symbol_table_is_reported(self) -> None:
        document = clean_matrix()
        document["programs"]["host/matrix"]["host_symtab"] = True
        errors = flag_audit.check_floors(document)
        self.assertTrue(any("matrix" in error for error in errors), errors)

    def test_the_exact_boundary_passes_and_one_byte_more_fails(self) -> None:
        document = clean_matrix()
        document["programs"]["hosted-amiga/lookup"]["new"]["text"] = 800
        self.assertEqual(flag_audit.check_floors(document), [])
        document["programs"]["hosted-amiga/lookup"]["new"]["text"] = 801
        self.assertTrue(flag_audit.check_floors(document))


class VectorTests(unittest.TestCase):
    """The audit vectors are the driver and harness vectors, not a copy."""

    def test_audit_derives_the_shipped_vectors(self) -> None:
        host = flag_audit.profile_spec("host")
        self.assertEqual(host["new"], flag_audit.bench.HOST_RELEASE)
        self.assertEqual(host["old"], ["-std=c99", "-O2"])
        self.assertEqual(host["gc"], host["new"][:-1])

        m20 = flag_audit.profile_spec("hosted-amiga")
        self.assertEqual(m20["new"], flag_audit.bench.M68K_RELEASE)
        self.assertEqual(
            m20["old"],
            ["-std=c99", "-mcrt=nix20", "-m68000", "-fwrapv", "-O2"],
        )
        self.assertEqual(m20["gc"], m20["new"][:-1])

        m13 = flag_audit.profile_spec("hosted-amiga-13")
        self.assertEqual(
            m13["new"],
            [flag.replace("-mcrt=nix20", "-mcrt=nix13") for flag in flag_audit.bench.M68K_RELEASE],
        )
        self.assertIn("-mcrt=nix13", m13["old"])

        free = flag_audit.profile_spec("freestanding")
        self.assertEqual(free["old"], free["new"])
        self.assertEqual(free["gc"], free["old"] + flag_audit.GC_FLAGS)

    def test_driver_base_constants_match_the_audit(self) -> None:
        host = flag_audit.profile_spec("host")
        self.assertEqual(
            host["new"],
            rust_const("HOST_BASE") + rust_const("HOST_RELEASE_GNU"),
        )
        m20 = flag_audit.profile_spec("hosted-amiga")
        self.assertEqual(
            m20["new"],
            rust_const("AMIGA_BASE") + rust_const("M68K_RELEASE_FLAGS"),
        )
        m13 = flag_audit.profile_spec("hosted-amiga-13")
        self.assertEqual(
            m13["new"],
            rust_const("AMIGA_13_BASE") + rust_const("M68K_RELEASE_FLAGS"),
        )
        free = flag_audit.profile_spec("freestanding")
        self.assertEqual(
            free["new"],
            rust_const("FREESTANDING_BUILD_BASE")
            + rust_const("FREESTANDING_RELEASE_FLAGS"),
        )


def rust_const(name: str) -> list[str]:
    """Read one Rust string-slice constant from the driver source."""
    source = (ROOT / "compiler/src/clorraine/toolchain.rs").read_text(
        encoding="utf-8"
    )
    match = re.search(rf"const {name}: &\[&str\] = &\[(.*?)\];", source, re.S)
    if match is None:
        raise AssertionError(f"missing Rust constant: {name}")
    return re.findall(r'"([^"]+)"', match.group(1))


class ConstructorTests(unittest.TestCase):
    """The remaining init-list constructors must be proven to run."""

    def _proof(self, **overrides: object) -> dict:
        constructors = {
            name: 1 for name in flag_audit.REQUIRED_CONSTRUCTORS
        }
        constructors.update(overrides)
        return {
            "constructors": {
                profile: {"init_loop": 1, "constructors": dict(constructors)}
                for profile in flag_audit.M68K_PROFILES
            }
        }

    def test_a_clean_proof_passes(self) -> None:
        self.assertEqual(flag_audit.check_constructor(self._proof()), [])

    def test_a_missing_profile_is_reported(self) -> None:
        proof = self._proof()
        del proof["constructors"]["hosted-amiga-13"]
        failures = flag_audit.check_constructor(proof)
        self.assertTrue(any("hosted-amiga-13" in failure for failure in failures), failures)

    def test_a_missing_constructor_is_reported(self) -> None:
        proof = self._proof()
        del proof["constructors"]["hosted-amiga"]["constructors"]["___initcpp"]
        failures = flag_audit.check_constructor(proof)
        self.assertTrue(any("___initcpp" in failure for failure in failures), failures)

    def test_a_zero_hit_is_reported(self) -> None:
        proof = self._proof(___initmalloc=0)
        failures = flag_audit.check_constructor(proof)
        self.assertTrue(any("___initmalloc" in failure for failure in failures), failures)

    def test_the_proof_maps_symbols_through_the_trace_base(self) -> None:
        nm = (
            "00000000 T _start\n"
            "0000007c T __Exit\n"
            "0000025a T _main\n"
            "00000802 T ___initstdio\n"
            "00000950 T ___next\n"
        )
        with mock.patch.object(flag_audit.bench, "run_command", return_value=nm), \
             mock.patch.object(flag_audit, "trace_pcs", return_value=[0x260C, 0x2688, 0x2E0E]):
            proof = flag_audit.constructor_proof("hosted-amiga", Path("x"))
        self.assertEqual(proof["base"], 0x260C)
        self.assertGreater(proof["init_loop"], 0)
        self.assertGreater(proof["constructors"]["___initstdio"], 0)

    def test_a_missing_readelf_is_an_audit_error(self) -> None:
        with mock.patch.object(
            flag_audit.bench,
            "run_command",
            side_effect=flag_audit.bench.HarnessError("missing readelf"),
        ):
            with self.assertRaises(flag_audit.AuditError):
                flag_audit.host_symbols(Path("x"))


class MainTests(unittest.TestCase):
    """The gate exits nonzero and writes the JSON before failing."""

    def test_a_floor_failure_writes_json_and_exits_one(self) -> None:
        written: list[dict] = []
        with mock.patch.object(flag_audit, "run_matrix", return_value=clean_matrix()), \
             mock.patch.object(flag_audit, "check_floors", return_value=["bad floor"]), \
             mock.patch.object(flag_audit.bench, "write_json", side_effect=lambda doc, path: written.append(doc)):
            status = flag_audit.main(
                ["run", "--repeat", "1", "--check", "--out", "build/perf/test.json"]
            )
        self.assertEqual(status, 1)
        self.assertEqual(len(written), 1)
        self.assertFalse(written[0]["checks"][0]["passed"])

    def test_a_repeat_difference_exits_one(self) -> None:
        first = clean_matrix()
        second = copy.deepcopy(first)
        second["programs"]["hosted-amiga/lookup"]["new"]["text"] = 501
        with mock.patch.object(flag_audit, "run_matrix", side_effect=[first, second]), \
             mock.patch.object(flag_audit, "check_floors", return_value=[]), \
             mock.patch.object(flag_audit.bench, "write_json"):
            status = flag_audit.main(
                ["run", "--repeat", "2", "--check", "--out", "build/perf/test.json"]
            )
        self.assertEqual(status, 1)

    def test_the_default_repeat_is_two(self) -> None:
        written: list[dict] = []
        with mock.patch.object(flag_audit, "run_matrix", return_value=clean_matrix()) as run, \
             mock.patch.object(
                 flag_audit.bench,
                 "write_json",
                 side_effect=lambda doc, path: written.append(doc),
             ):
            status = flag_audit.main(
                ["run", "--check", "--out", "build/perf/test.json"]
            )
        self.assertEqual(status, 0)
        self.assertEqual(run.call_count, 2)
        self.assertTrue(written[0]["checks"][-1]["passed"])


class ErrorTests(unittest.TestCase):
    """Malformed audit inputs fail with a clear audit error."""

    def test_unknown_profile(self) -> None:
        with self.assertRaises(flag_audit.AuditError):
            flag_audit.profile_spec("nope")

    def test_unknown_program_kind(self) -> None:
        with self.assertRaises(flag_audit.AuditError):
            flag_audit.program_source("x", "nope")

    def test_malformed_entry(self) -> None:
        with self.assertRaises(flag_audit.AuditError):
            flag_audit.check_floors({"programs": {"hosted-amiga/x": {}}})


class OutputTests(unittest.TestCase):
    """Every run is checked against the committed expected output."""

    def _measure(self, output: str) -> dict:
        spec = {"compiler": "cc", "profile": "host", "libs": []}
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            c_path = root / "program.c"
            c_path.write_text("/* c */\n", encoding="utf-8")
            with mock.patch.object(flag_audit.bench, "build_step"), \
                 mock.patch.object(flag_audit.bench, "binary_bytes", return_value=1), \
                 mock.patch.object(
                     flag_audit.bench,
                     "run_command",
                     return_value="   text    data     bss     dec     hex filename\n"
                     "      1       2       3       6       6 x\n",
                 ), \
                 mock.patch.object(flag_audit, "run_in", return_value=output):
                return flag_audit.measure_variant(
                    spec, c_path, root / "program", [], root / "run", "expected\n", False, True
                )

    def test_the_expected_output_passes(self) -> None:
        self.assertEqual(self._measure("expected\n")["output"], "expected\n")

    def test_a_wrong_output_fails_even_when_both_variants_match(self) -> None:
        with self.assertRaises(flag_audit.AuditError):
            self._measure("wrong\n")


class RepeatTests(unittest.TestCase):
    """Two audit runs must agree except for timing fields."""

    def test_identical_matrices_have_no_differences(self) -> None:
        first = clean_matrix()
        second = copy.deepcopy(first)
        second["programs"]["hosted-amiga/lookup"]["duration_ms"] = 123
        self.assertEqual(flag_audit.compare_repeats(first, second), [])

    def test_a_changed_measurement_is_reported(self) -> None:
        first = clean_matrix()
        second = copy.deepcopy(first)
        second["programs"]["hosted-amiga/lookup"]["new"]["text"] = 501
        differences = flag_audit.compare_repeats(first, second)
        self.assertTrue(any("lookup" in difference for difference in differences))


if __name__ == "__main__":
    unittest.main()
