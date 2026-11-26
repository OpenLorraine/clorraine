#!/usr/bin/env python3
"""WS8-03 / WS8-06: static assertion-strength classification of the Rust tests.

A test's class is the strongest kind of check its body makes (no test is run or
edited). Strength order and the evidence each class needs in the test body:

  value       the program's output or a computed result is compared: assert_output,
              assert_stdout, assert_run_exit, assert_follows, assert_before,
              assert_release_after, assert_eq!/assert_ne! on stdout/output/result
              text, assert_exit with a non-zero expected code, or assert_exit(…, 0)
              when the embedded program itself checks values (`assert(`, `panic(`,
              `return 1`/`return 2` on a mismatch)
  diagnostic  a compile-time diagnostic code or a runtime fault code is required:
              assert_rejected*, assert_reports*, assert_trap*, assert_fault, or an
              `E1xxxx`/`E2xxxx`/`W1xxxx` literal inside an assertion
  text        emitted C text is required or forbidden: assert_contains/_all,
              assert_absent, assert_emitted, assert_on_line_of, assert_keyed_instance,
              assert_no_template on a C/emission variable (pinned-text, WS8-06)
  sanitizer   only a clean ASan/UBSan/leak run: assert_no_asan, assert_no_sanitizer*,
              assert_leak_free
  compiles    only acceptance or a zero exit of a program that checks nothing:
              assert_accepts, assert_accepted, assert_front_end_accepts,
              assert_run_ok, status.success(), assert_exit(…, 0)
  no-panic    only the absence of a compiler panic or crash
  other       none of the above (reported for reading)

Usage:
  800-assertion-scheme.py sample [--per-area N]   deterministic sample per area
  800-assertion-scheme.py classify FILE::TEST ...  classify named tests
  800-assertion-scheme.py census                   count every test by class
"""
import hashlib
import re
import sys
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parents[4]
TESTS = ROOT / "compiler" / "tests"
AREAS = {
    "lexical-literals": r"lex|literal|escape|token",
    "evaluation-order": r"eval|order|sequenc",
    "constants": r"const|fold",
    "profiles-gating": r"profile|gating|a500|e11200",
    "ownership-drops": r"own|drop|move",
    "borrows": r"borrow|loan|nll",
    "generics": r"generic|instance|mono",
    "closures": r"lambda|closure|capture",
    "c-validity": r"strict|emission|invalid|integrity",
    "arithmetic-ub": r"overflow|wrap|ubsan|division|shift",
    "names": r"mangl|collision|symbol|rename",
    "determinism": r"determinis",
    "runtime-faults": r"fault|trap|robust",
    "stdlib": r"stdlib|std_|buffer|string|array|iff",
    "performance": r"perf|bench|throughput|size",
}
ORDER = ["value", "diagnostic", "text", "sanitizer", "compiles", "no-panic", "other"]


def tests() -> dict[str, str]:
    """Map FILE::TEST to its text: from the test's `fn` to the next `#[test]` (or the
    end of the file). Trailing helper functions may be included; they only add
    assertion kinds the test can reach."""
    out = {}
    for f in sorted(TESTS.glob("*.rs")):
        src = f.read_text()
        heads = list(re.finditer(r"#\[test\][^\n]*\n(?:\s*#\[[^\n]*\n)*\s*fn\s+(\w+)\s*\(\)", src))
        for i, m in enumerate(heads):
            stop = heads[i + 1].start() if i + 1 < len(heads) else len(src)
            out[f"{f.name}::{m.group(1)}"] = src[m.end():stop]
    return out


def classify(body: str) -> str:
    b = body
    program_checks = re.search(r"\bassert\(|\bpanic\(|return [1-9]\d*;", b) is not None
    if re.search(r"assert_(output|stdout|run_exit|follows|before|release_after)\(", b) \
            or re.search(r"assert_(eq|ne)!\([^;]*(stdout|output|result|value|text)", b) \
            or re.search(r"assert_exit\([^;]*,\s*[1-9]\d*\s*\)", b) \
            or (re.search(r"assert_exit\([^;]*,\s*0\s*\)", b) and program_checks):
        return "value"
    if re.search(r"assert_(rejected\w*|reports\w*|traps?|fault)\(", b) \
            or re.search(r"assert\w*!?\([^;]*\"[EW][12]\d{4}", b):
        return "diagnostic"
    if re.search(r"assert_(contains(_all)?|absent|emitted|on_line_of|keyed_instance|no_template)\(", b):
        return "text"
    if re.search(r"assert_(no_asan|no_sanitizer\w*|leak_free)\(", b):
        return "sanitizer"
    if re.search(r"assert_(accepts|accepted|front_end_accepts|run_ok)\(|status\.success\(\)|assert_exit\([^;]*,\s*0\s*\)", b):
        return "compiles"
    if re.search(r"panic|crash", b) and re.search(r"assert", b):
        return "no-panic"
    return "other"


def main() -> int:
    mode = sys.argv[1] if len(sys.argv) > 1 else "sample"
    every = tests()
    if mode == "classify":
        for name in sys.argv[2:]:
            print(f"{classify(every[name])}\t{name}")
        return 0
    if mode == "census":
        counts = Counter(classify(b) for b in every.values())
        print(f"tests {len(every)}: " + ", ".join(f"{k} {counts[k]}" for k in ORDER))
        return 0
    per = int(sys.argv[sys.argv.index("--per-area") + 1]) if "--per-area" in sys.argv else 5
    total = Counter()
    for area, pattern in AREAS.items():
        names = sorted((n for n in every if re.search(pattern, n, re.I)),
                       key=lambda n: hashlib.sha256(n.encode()).hexdigest())[:per]
        for n in names:
            c = classify(every[n])
            total[c] += 1
            print(f"{area}\t{c}\t{n}")
    print("sample: " + ", ".join(f"{k} {total[k]}" for k in ORDER))
    return 0


if __name__ == "__main__":
    sys.exit(main())
