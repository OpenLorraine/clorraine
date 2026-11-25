#!/usr/bin/env python3
"""WS6-05 (static part): handle release discipline in the stdlib source.

For every `handle T freedBy F;` in `stdlib/`:
  no-deleter    F is not declared in an `ffi` block of the module (or a public func)
  deleter-mode  F's handle parameter is not `take`
and for every function of the module that passes a handle of type T to F or to an
`ffi` function whose name says it releases (`free`, `close`, `dispose`, `delete`,
`unlock`, `release`, `remove`, `destroy`, `end`):
  double-close  the handle reaches the release through a parameter that is not
                `take` (the caller's drop would release it again)

Usage: 650-handle-ownership.py [--seed]   (--seed adds a synthetic module whose
`closeX(borrow x: X)` calls the deleter; it must be reported as double-close)
"""
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[4]
RELEASE = re.compile(r"(free|close|dispose|delete|unlock|release|remove|destroy)", re.I)
SEED = """module std.seeded;
export handle X freedBy cl_x_free;
ffi seeded { func cl_x_free(take x: X): void; func cl_x_new(): X?; }
export func closeX(borrow x: X): void { unsafe { cl_x_free(x); } }
"""


def params(text: str) -> list[tuple[str | None, str, str]]:
    out = []
    for p in [p.strip() for p in text.split(",") if p.strip()]:
        words = p.split()
        mode = words[0] if words[0] in ("take", "borrow", "const") else None
        rest = " ".join(words[1:] if mode else words)
        if ":" in rest:
            name, ptype = [x.strip() for x in rest.split(":", 1)]
            out.append((mode, name, ptype.rstrip("?")))
    return out


def check(module: str, src: str, reports: list) -> int:
    handles = dict(re.findall(r"^(?:export\s+)?handle\s+(\w+)\s+freedBy\s+(\w+)\s*;", src, re.M))
    ffi = {}
    for block in re.findall(r"^ffi\s+\w+\s*\{(.*?)^\}", src, re.M | re.S) + \
            re.findall(r"^ffi\s+\w+\s*\{(.*?)\}\s*$", src, re.M):
        for name, ps in re.findall(r"func\s+(\w+)\s*\(([^)]*)\)", block):
            ffi.setdefault(name, params(ps))
    for htype, deleter in handles.items():
        if deleter not in ffi:
            reports.append(("no-deleter", module, f"{htype} freedBy {deleter}"))
            continue
        modes = [m for (m, _, t) in ffi[deleter] if t == htype]
        if modes and modes[0] != "take":
            reports.append(("deleter-mode", module, f"{deleter}({modes[0]} {htype})"))
    for m in re.finditer(r"^(?:export\s+)?func\s+(\w+)\s*(?:<[^>]*>)?\s*\(([^)]*)\)[^{]*\{", src, re.M):
        fname, ps = m.group(1), params(m.group(2))
        start, depth, i = m.end(), 1, m.end()
        while depth and i < len(src):
            depth += {"{": 1, "}": -1}.get(src[i], 0)
            i += 1
        body = src[start:i]
        for mode, pname, ptype in ps:
            if ptype not in handles:
                continue
            for callee, args in re.findall(r"\b(\w+)\s*\(([^()]*)\)", body):
                if callee not in ffi or not re.search(rf"\b{re.escape(pname)}\b", args):
                    continue
                if callee == handles[ptype] or RELEASE.search(callee):
                    cmodes = [cm for (cm, _, ct) in ffi[callee] if ct == ptype]
                    if cmodes and cmodes[0] == "take" and mode != "take":
                        reports.append(("double-close", module, f"{fname}({mode or 'default'} {pname}: {ptype}) -> {callee}(take)"))
                    elif not cmodes or cmodes[0] != "take":
                        if mode != "take" and RELEASE.search(callee):
                            reports.append(("review", module, f"{fname}({mode or 'default'} {pname}: {ptype}) -> {callee}({cmodes[0] if cmodes else '?'})"))
    return len(handles)


def main() -> int:
    reports: list = []
    total = 0
    sources = [(f.stem, f.read_text()) for f in sorted((ROOT / "stdlib").glob("std*.clo"))]
    if "--seed" in sys.argv:
        sources.append(("std.seeded", SEED))
    for module, src in sources:
        total += check(module, src, reports)
    for kind, module, text in reports:
        print(f"{kind}\t{module}\t{text}")
    hard = [r for r in reports if r[0] != "review"]
    print(f"handles: {total}; {len(hard)} report(s); {len(reports) - len(hard)} for review")
    return 1 if hard else 0


if __name__ == "__main__":
    sys.exit(main())
