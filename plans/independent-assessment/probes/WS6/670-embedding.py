#!/usr/bin/env python3
"""WS6-07: stdlib embedding, profile gating, and stdlib/shim version skew.

Checks:
  not-embedded   a stdlib/*.clo file that compiler/src/stdlib.rs does not embed
  no-file        an embedded module name with no file, or a name that differs from the
                 file's `module` declaration
  gate-unknown   a module named in profile.rs's 1.3 allowlist that does not exist
  no-shim        an `ffi` function of a module with no C definition in the shims that
                 the module's profiles link (1.3 modules: shim_hosted_amiga.c; other
                 std.amiga modules: shim_hosted_amiga*.c; core modules: the host shim
                 and shim_hosted_amiga.c; std.io also shim_freestanding.c). Functions
                 the C runtime takes from libnix/libc or the OS (Open, Close, Read,
                 Write) are accepted when the shim declares or defines them.
and lists, for review:
  unused-shim    a non-static `cl_*` function defined in a shim that no stdlib `ffi`
                 block names and the compiler source does not mention

Usage: 670-embedding.py [--seed]   (--seed adds a fake ffi function to std.amiga.dos,
which must be reported as no-shim)
"""
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[4]
RT = ROOT / "compiler" / "runtime"
CORE = {"std.io", "std.mem", "std.text", "std.fmt", "std.math", "std.iff"}


def defined(files: list[Path]) -> set[str]:
    names = set()
    for f in files:
        names |= set(re.findall(r"^(?!static)[A-Za-z][\w \*]*?\b(\w+)\s*\([^;{]*\)\s*\{", f.read_text(), re.M))
        names |= set(re.findall(r"^(?:extern\s+)?[A-Za-z][\w \*]*?\b(\w+)\s*\([^;{]*\)\s*;", f.read_text(), re.M))
    return names


def main() -> int:
    seed = "--seed" in sys.argv
    reports = []
    stdlib_rs = (ROOT / "compiler" / "src" / "stdlib.rs").read_text()
    embedded = dict(re.findall(r'name:\s*"(std[\w.]*)",\s*source:\s*include_str!\("\.\./\.\./stdlib/([\w.]+\.clo)"\)', stdlib_rs))
    files = {f.name: f for f in (ROOT / "stdlib").glob("std*.clo")}
    for fname in sorted(files):
        if fname not in embedded.values():
            reports.append(("not-embedded", fname, ""))
    for name, fname in sorted(embedded.items()):
        f = files.get(fname)
        if f is None:
            reports.append(("no-file", name, fname))
            continue
        declared = re.search(r"^module\s+([\w.]+)\s*;", f.read_text(), re.M)
        if not declared or declared.group(1) != name:
            reports.append(("no-file", name, f"declares {declared.group(1) if declared else 'nothing'}"))
    prof = (ROOT / "compiler" / "src" / "profile.rs").read_text()
    block = prof[prof.index("Profile::HostedAmiga13 => matches!("):]
    on13 = set(re.findall(r'"(std\.[\w.]+)"', block[:block.index(")")]))
    for module in sorted(on13 - set(embedded)):
        reports.append(("gate-unknown", module, ""))
    base = [RT / "shim_hosted_amiga.c"]
    amiga_all = sorted(RT.glob("shim_hosted_amiga*.c"))
    host = [RT / "shim_host.c"]
    free = [RT / "shim_freestanding.c"]
    referenced = set()
    for name, fname in sorted(embedded.items()):
        f = files.get(fname)
        if f is None:
            continue
        src = f.read_text()
        if seed and name == "std.amiga.dos":
            src += "\nffi seeded {\n    func cl_dos_seeded_missing(): u32;\n}\n"
        ffi = set()
        for blk in re.findall(r"^ffi\s+\w+\s*\{(.*?)^\}", src, re.M | re.S):
            ffi |= set(re.findall(r"func\s+(\w+)\s*\(", blk))
        referenced |= ffi
        if name in CORE:
            sets = [("host", host), ("hosted-amiga-13", base)] + ([("freestanding", free)] if name == "std.io" else [])
        elif name in on13:
            sets = [("hosted-amiga-13", base)]
        else:
            sets = [("hosted-amiga", amiga_all)]
        for label, shim in sets:
            have = defined(shim)
            for fn in sorted(ffi - have):
                reports.append(("no-shim", name, f"{fn} not in {label} shims"))
    compiler_src = "".join(p.read_text() for p in (ROOT / "compiler" / "src").rglob("*.rs"))
    unused = []
    for f in sorted(RT.glob("shim_*.c")):
        for fn in re.findall(r"^(?!static)[A-Za-z][\w \*]*?\b(cl_\w+)\s*\([^;{]*\)\s*\{", f.read_text(), re.M):
            if fn not in referenced and fn not in compiler_src:
                unused.append(f"{f.name}:{fn}")
    for kind, module, text in reports:
        print(f"{kind}\t{module}\t{text}")
    print(f"embedding: {len(embedded)} embedded modules, {len(files)} files, {len(on13)} on 1.3; {len(reports)} report(s); {len(unused)} unused shim function(s)")
    for u in unused:
        print(f"unused-shim\t{u}")
    return 1 if reports else 0


if __name__ == "__main__":
    sys.exit(main())
