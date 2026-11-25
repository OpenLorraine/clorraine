#!/usr/bin/env python3
"""WS4-08 flag variants (expectation in 420-flags.expected).

Usage (inside the SDK container, from the repository root):
    python3 <plan>/probes/WS4/420-flags.py <pinned-clc> <out-dir> [<bench> ...]

Builds each benchmark's Clorraine emission and the calibration program with flag
variants on the A500 lens (hosted-amiga-13, nix13, -m68000) and the 68020 probe lens
(hosted-amiga emission, nix20, -m68020, vamos -C 68020), traces each under vamos, and
prints raw and net instructions and .text per variant. Variants: shipped -O2 (base),
shipped -Os, -O2 -fomit-frame-pointer, -O2 -flto, -O2 without -fwrapv, -O3. Every
variant keeps section GC and strip. Output identity is checked against the
benchmark golden; a variant whose output differs is reported as FAIL.
"""

import os
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(subprocess.run(["git", "rev-parse", "--show-toplevel"], capture_output=True,
                           text=True, check=True).stdout.strip())
sys.path.insert(0, str(ROOT / "tools" / "perf"))
import bench  # noqa: E402

GC = ["-ffunction-sections", "-fdata-sections", "-Wl,--gc-sections", "-s"]
VARIANTS = {
    "O2": ["-fwrapv", "-O2"],
    "Os": ["-fwrapv", "-Os"],
    "O2-fomit-frame-pointer": ["-fwrapv", "-O2", "-fomit-frame-pointer"],
    "O2-flto": ["-fwrapv", "-O2", "-flto"],
    "O2-no-fwrapv": ["-O2"],
    "O3": ["-fwrapv", "-O3"],
}
LENSES = {
    "a500": ("hosted-amiga-13", ["-std=c99", "-mcrt=nix13", "-m68000"], "68000"),
    "68020-probe": ("hosted-amiga", ["-std=c99", "-mcrt=nix20", "-m68020"], "68020"),
}


def main() -> int:
    clc = Path(sys.argv[1]).resolve()
    out = ROOT / sys.argv[2]
    names = sys.argv[3:] or ["container", "local-table", "lookup", "matrix", "moves", "sieve",
                             "strings", "text-scan"]
    out.mkdir(parents=True, exist_ok=True)
    failures = 0
    saved = os.environ.get("HOME")
    for lens, (profile, base, cpu) in LENSES.items():
        with tempfile.TemporaryDirectory(dir=out) as home:
            Path(home, ".vamosrc").write_text(f"[vamos]\ncpu={cpu}\n")
            os.environ["HOME"] = home
            for variant, flags in VARIANTS.items():
                cal = None
                for name in ["calibration"] + names:
                    work = out / lens / variant / name
                    work.mkdir(parents=True, exist_ok=True)
                    c_file = work / "prog.c"
                    bench.emit(clc, profile, ROOT / "benchmarks" / name / "program.clo", c_file)
                    binary = work / "prog"
                    try:
                        bench.build_step(["m68k-amigaos-gcc", *base, *flags, *GC, "-o", str(binary),
                                          str(c_file), "-lamiga", "-lm"], bench.M68K_BUILD_TIMEOUT)
                        observed, count = bench.trace_vamos(binary)
                        text = bench.size_sections(binary)["text"]
                    except bench.HarnessError as error:
                        print(f"FAIL {lens} {variant} {name}: {str(error)[:160]}")
                        failures += 1
                        continue
                    ok = observed == bench.expected_output(name)
                    failures += not ok
                    if name == "calibration":
                        cal = (count, text)
                        print(f"{'PASS' if ok else 'FAIL'} {lens} {variant} calibration: raw={count} text={text}")
                        continue
                    print(f"{'PASS' if ok else 'FAIL'} {lens} {variant} {name}: raw={count} "
                          f"net={count - cal[0]} text={text} text_net={text - cal[1]}")
    if saved is not None:
        os.environ["HOME"] = saved
    print(f"flags: {failures} failure(s)")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
