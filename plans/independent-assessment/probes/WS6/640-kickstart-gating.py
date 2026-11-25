#!/usr/bin/env python3
"""WS6-04: is the Kickstart version each module needs declared and enforced?

For every stdlib module the script collects:
  * whether `compiler/src/profile.rs` allows it on `hosted-amiga-13`;
  * the minimum Kickstart in the spec/appendix-amiga.md module table (2.0+ modules);
  * the library/device versions its shim functions request: every `OpenLibrary`,
    `OpenDevice`, and `lib_Version` test in the C bodies of the module's `ffi`
    functions and of the static helpers they call (one level).

It reports:
  13-too-new   a module allowed on 1.3 whose shim requests a version above 34
  13-listed    a module the appendix lists at 2.0+ that profile.rs allows on 1.3
  below-min    a shim request below the appendix minimum (calls may be missing)
  above-min    a shim request above the appendix minimum (the module never works on
               a system the appendix says is supported)

Usage: 640-kickstart-gating.py [--seed]   (--seed pretends std.amiga.workbench and
std.amiga.semaphore are allowed on 1.3, which must produce 13-too-new and 13-listed)
"""
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[4]
KS = {"1.3": 34, "2.0": 36, "2.04": 37, "2.1": 38, "3.0": 39, "3.1": 40}


def shim_functions() -> dict[str, str]:
    bodies: dict[str, str] = {}
    for f in sorted((ROOT / "compiler" / "runtime").glob("shim_hosted_amiga*.c")):
        src = f.read_text()
        for m in re.finditer(r"^[A-Za-z][\w \*]*?\b(\w+)\s*\([^;{]*\)\s*\{", src, re.M):
            start = m.end()
            depth, i = 1, start
            while depth and i < len(src):
                depth += {"{": 1, "}": -1}.get(src[i], 0)
                i += 1
            bodies.setdefault(m.group(1), src[start:i])
    return bodies


def main() -> int:
    seed = "--seed" in sys.argv
    prof = (ROOT / "compiler" / "src" / "profile.rs").read_text()
    block = prof[prof.index("Profile::HostedAmiga13 => matches!("):]
    block = block[:block.index(")")]
    on13 = set(re.findall(r'"(std\.[\w.]+)"', block))
    if seed:
        on13.add("std.amiga.workbench")
        on13.add("std.amiga.semaphore")
    appendix = {}
    for m in re.finditer(r"^\| `(std\.amiga\.\w+)` \| [^|]* \| ([0-9.]+)", (ROOT / "spec" / "appendix-amiga.md").read_text(), re.M):
        appendix[m.group(1)] = m.group(2)
    bodies = shim_functions()
    reports = []
    for f in sorted((ROOT / "stdlib").glob("std*.clo")):
        module = f.stem
        src = f.read_text()
        ffi = set()
        for block_m in re.finditer(r"^ffi\s+\w+\s*\{(.*?)^\}", src, re.M | re.S):
            ffi |= set(re.findall(r"func\s+(\w+)\s*\(", block_m.group(1)))
        text = ""
        for name in sorted(ffi):
            body = bodies.get(name, "")
            text += body
            for helper in set(re.findall(r"\b(cl_\w+)\s*\(", body)) - {name}:
                text += bodies.get(helper, "")
        versions = [int(v) for v in re.findall(r"OpenLibrary\s*\([^,]*,\s*(\d+)\s*\)", text)]
        versions += [int(v) for v in re.findall(r"lib_Version\s*<\s*(\d+)", text)]
        top = max(versions) if versions else None
        if module in on13:
            if top is not None and top > 34:
                reports.append(("13-too-new", module, f"requests V{top} on the 1.3 profile"))
            if module in appendix:
                reports.append(("13-listed", module, f"appendix minimum {appendix[module]}, allowed on 1.3"))
        if module in appendix and top is not None:
            want = KS.get(appendix[module])
            if want is not None and top < want and top < 36:
                reports.append(("below-min", module, f"requests V{top}, appendix minimum {appendix[module]} (V{want})"))
            if want is not None and top > want:
                reports.append(("above-min", module, f"requests V{top}, appendix minimum {appendix[module]} (V{want})"))
        print(f"module\t{module}\t1.3={'yes' if module in on13 else 'no'}\tappendix={appendix.get(module, '-')}\trequests={sorted(set(versions))}")
    for kind, module, text in reports:
        print(f"{kind}\t{module}\t{text}")
    print(f"gating: {len(reports)} report(s)")
    return 1 if reports else 0


if __name__ == "__main__":
    sys.exit(main())
