#!/usr/bin/env python3
"""WS1-09 profile-gating matrix probe.

Usage (inside the SDK container, from the repository root):
    python3 <plan>/probes/WS1/057-gating-matrix.py <pinned-clc> [<out-dir>]

For every embedded standard-library module and every profile, a one-import
program is emitted; the observed acceptance is compared with the expectation
in 057-gating-matrix.expected (one line per module: ``<module> <host>
<hosted-amiga> <hosted-amiga-13> <freestanding>``, each ``A`` accept, ``R``
reject with E11200, or ``-`` unspecified). Also checks that an unknown
profile name is E11201. Prints one line per cell and a summary; exit 1 on any
mismatch with a specified expectation.
"""

import subprocess
import sys
from pathlib import Path

ROOT = Path(subprocess.run(["git", "rev-parse", "--show-toplevel"], capture_output=True,
                           text=True, check=True).stdout.strip())
HERE = Path(__file__).resolve().parent
PROFILES = ["host", "hosted-amiga", "hosted-amiga-13", "freestanding"]


def first_export(module: str) -> str:
    for line in (ROOT / "stdlib" / f"{module}.clo").read_text().splitlines():
        if line.startswith("export func "):
            return line.split()[2].split("(")[0].split("<")[0]
    raise SystemExit(f"no exported function in {module}")


def main() -> int:
    clc = Path(sys.argv[1]).resolve()
    out = ROOT / (sys.argv[2] if len(sys.argv) > 2 else "build/tmp/assessment/WS1/gating")
    out.mkdir(parents=True, exist_ok=True)
    expected = {}
    for line in (HERE / "057-gating-matrix.expected").read_text().splitlines():
        if line.startswith("std."):
            parts = line.split()
            expected[parts[0]] = dict(zip(PROFILES, parts[1:5]))
    modules = sorted(p.stem for p in (ROOT / "stdlib").glob("std.*.clo"))
    failures = 0
    for module in modules:
        name = first_export(module)
        src = out / f"{module}.clo"
        src.write_text(f"module Probe.Gate;\n\nimport {{ {name} as probe }} from {module};\n\n"
                       "func main(): i32 {\n    return 0;\n}\n")
        row = []
        for profile in PROFILES:
            done = subprocess.run([str(clc), "--profile", profile, "-o", str(out / "g.c"), str(src)],
                                  capture_output=True, text=True, timeout=120)
            text = done.stderr + done.stdout
            observed = "A" if done.returncode == 0 else ("R" if "E11200" in text else "X")
            want = expected.get(module, {}).get(profile, "?")
            ok = want == "-" or observed == want
            failures += not ok
            row.append(f"{profile}={observed}{'' if ok else '!=' + want}")
            if observed == "X":
                (out / f"{module}.{profile}.log").write_text(text)
        print(f"{'PASS' if all('!=' not in c for c in row) else 'FAIL'} {module}: {' '.join(row)}")
    done = subprocess.run([str(clc), "--profile", "amiga-os4", "-o", str(out / "g.c"),
                           str(out / f"{modules[0]}.clo")], capture_output=True, text=True)
    code_ok = done.returncode != 0 and "E11201" in done.stderr + done.stdout
    failures += not code_ok
    print(f"{'PASS' if code_ok else 'FAIL'} unknown-profile: rc={done.returncode} "
          f"{(done.stderr + done.stdout).strip().splitlines()[:1]}")
    print(f"gating: {failures} mismatch(es)")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
