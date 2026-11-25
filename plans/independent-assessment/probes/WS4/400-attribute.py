#!/usr/bin/env python3
"""WS4-02 instruction attribution by function (expectation in 400-attribute.expected).

Usage (inside the SDK container, from the repository root):
    python3 <plan>/probes/WS4/400-attribute.py <pinned-clc> <plan-folder> <out-dir> <bench> [<bench> ...]
    python3 <plan>/probes/WS4/400-attribute.py --c-file <file.c> <out-dir>      (one C program)

Each program (the Clorraine emission for hosted-amiga-13 and the equivalent C
reference) is built with the A500 shipped -O2 line but without -s, so its symbols
remain; it runs under `vamos -B --instr-trace`, whose lines name the hunk and the
offset of every executed instruction (`prog_0:code +000123`). Offsets are mapped to
the enclosing text symbol from `m68k-amigaos-nm -n`; instructions executed inside
library jump tables and traps are counted under the library label. Output: the top
functions by executed instructions for each side, written to <out-dir>/<name>.tsv.
"""

import bisect
import re
import subprocess
import sys
from collections import Counter
from pathlib import Path

ROOT = Path(subprocess.run(["git", "rev-parse", "--show-toplevel"], capture_output=True,
                           text=True, check=True).stdout.strip())
FLAGS = ["-std=c99", "-mcrt=nix13", "-m68000", "-fwrapv", "-O2", "-ffunction-sections",
         "-fdata-sections", "-Wl,--gc-sections"]
LINE = re.compile(r"instr:\s+\S+:\s+@[0-9a-f]+ \+([0-9a-f]+) (\S+)")


def symbols(binary: Path) -> tuple[list[int], list[str]]:
    out = subprocess.run(["m68k-amigaos-nm", "-n", str(binary)], capture_output=True, text=True).stdout
    addrs, names = [], []
    for line in out.splitlines():
        parts = line.split()
        if len(parts) == 3 and parts[1] in "tT":
            addrs.append(int(parts[0], 16))
            names.append(parts[2])
    return addrs, names


def attribute(binary: Path) -> Counter:
    addrs, names = symbols(binary)
    counts: Counter = Counter()
    proc = subprocess.Popen(["vamos", "-B", "--instr-trace", str(binary)], stdout=subprocess.DEVNULL,
                            stderr=subprocess.PIPE, text=True, errors="replace")
    assert proc.stderr is not None
    for line in proc.stderr:
        m = LINE.search(line)
        if not m:
            continue
        offset, seg = int(m.group(1), 16), m.group(2)
        if seg.endswith(":code"):
            i = bisect.bisect_right(addrs, offset) - 1
            counts[names[i] if i >= 0 else "?"] += 1
        else:
            counts["[" + seg.split("(")[0] + "]"] += 1
    proc.wait(timeout=600)
    return counts


def build(src: Path, binary: Path) -> None:
    subprocess.run(["m68k-amigaos-gcc", *FLAGS, "-o", str(binary), str(src), "-lamiga", "-lm"],
                   check=True, capture_output=True, timeout=600)


def report(label: str, counts: Counter, out: Path) -> None:
    total = sum(counts.values())
    lines = [f"{label}\ttotal\t{total}"] + [f"{label}\t{k}\t{v}" for k, v in counts.most_common(25)]
    out.write_text("\n".join(lines) + "\n")
    print(f"== {label}: {total} instructions")
    for k, v in counts.most_common(12):
        print(f"  {v:>9}  {100.0 * v / total:5.1f}%  {k}")


def main() -> int:
    if sys.argv[1] == "--c-file":
        src, out = Path(sys.argv[2]), ROOT / sys.argv[3]
        out.mkdir(parents=True, exist_ok=True)
        build(src, out / "prog")
        report(src.stem, attribute(out / "prog"), out / f"{src.stem}.tsv")
        return 0
    clc, plan, out = Path(sys.argv[1]).resolve(), ROOT / sys.argv[2], ROOT / sys.argv[3]
    out.mkdir(parents=True, exist_ok=True)
    for name in sys.argv[4:]:
        c_file = out / f"{name}.clo.c"
        subprocess.run([str(clc), "--profile", "hosted-amiga-13", "-o", str(c_file),
                        str(ROOT / "benchmarks" / name / "program.clo")], check=True, capture_output=True)
        build(c_file, out / f"{name}.clo.bin")
        report(f"{name} clorraine", attribute(out / f"{name}.clo.bin"), out / f"{name}.clo.tsv")
        build(plan / "probes" / "WS0" / "ref" / f"{name}.c", out / f"{name}.c.bin")
        report(f"{name} C", attribute(out / f"{name}.c.bin"), out / f"{name}.c.tsv")
    return 0


if __name__ == "__main__":
    sys.exit(main())
