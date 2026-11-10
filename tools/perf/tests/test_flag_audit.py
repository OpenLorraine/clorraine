"""Unit tests for the old-vs-shipped build-flag audit.

The tests use synthetic matrices; the real matrix is the ``make bench-flags``
gate, which needs the pinned SDK toolchain.
"""

from __future__ import annotations

import copy
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import flag_audit  # noqa: E402


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
