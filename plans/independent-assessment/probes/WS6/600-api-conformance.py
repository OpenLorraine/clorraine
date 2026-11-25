#!/usr/bin/env python3
"""WS6-01: compare the stdlib API tables in the appendices with the stdlib source.

For every table row whose first cell names a call (`name(...)`, `xs.name(...)`, or a
bare backticked identifier), the script checks against the module the section names:

  missing   the name is not a public function of that module (nor of any module)
  arity     the documented parameter count differs (dot-calls count the receiver)
  mode      a documented `take`/`borrow`/`const` mode differs at the same position
  type      a documented parameter type differs at the same position
  returns   the Returns cell is a single type and differs from the declared result

and, in reverse, lists public functions that no appendix mentions (`undocumented`).

Usage: 600-api-conformance.py [--appendix FILE ...]   (default: the two appendices)
Exit 0 when nothing is reported in the first five classes, 1 otherwise.
"""
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[4]
STDLIB = ROOT / "stdlib"
SECTION_MODULE = {  # appendix-stdlib.md sections without a module in the header
    "2": "std.mem", "3": "std.mem", "3.1": "std.mem", "5": "std.fmt", "6": "std.amiga.custom",
    "7": "std.amiga.interrupt", "10": "std.mem", "11": "std.text", "13": "std.amiga.exec",
}
SKIP_SECTIONS = {"1", "4", "12", "26"}
MODES = {"take", "borrow", "const"}


def split_top(text: str) -> list[str]:
    parts, depth, cur = [], 0, ""
    for ch in text:
        if ch in "<([":
            depth += 1
        elif ch in ">)]":
            depth -= 1
        if ch == "," and depth == 0:
            parts.append(cur.strip())
            cur = ""
        else:
            cur += ch
    if cur.strip():
        parts.append(cur.strip())
    return parts


def parse_param(text: str) -> tuple[str | None, str | None]:
    words = text.split()
    mode = words[0] if words and words[0] in MODES else None
    rest = " ".join(words[1:] if mode else words)
    ptype = rest.split(":", 1)[1].strip() if ":" in rest else None
    return mode, ptype


def norm_type(t: str | None) -> str | None:
    if t is None:
        return None
    t = t.replace("`", "").strip()
    t = re.sub(r"\s+", "", t)
    return "void" if t in ("", "-", "—", "void") else t


def index_source() -> dict[str, dict[str, dict]]:
    table: dict[str, dict[str, dict]] = {}
    for f in sorted(STDLIB.glob("std*.clo")):
        module = f.stem
        src = f.read_text()
        public = {}
        for m in re.finditer(r"^(export\s+)?(?:intrinsic\s+)?func\s+(\w+)\s*(<[^>]*>)?\s*\(([^)]*)\)\s*(?::\s*([^{;]+))?", src, re.M):
            exported, name, _, params, ret = m.groups()
            if not exported and module != "std.mem":
                continue  # std.mem's Array/Box surface is importable without `export`
            plist = [parse_param(p) for p in split_top(params)]
            public[name] = {"params": plist, "ret": norm_type(ret or "void")}
        table[module] = public
    return table


def doc_rows(files: list[Path]):
    for f in files:
        section, module = None, None
        for lineno, line in enumerate(f.read_text().splitlines(), 1):
            h = re.match(r"^(#{2,3})\s+(\d+(?:\.\d+)?)\.?\s+(.*)$", line)
            if h:
                section = h.group(2)
                named = re.search(r"\(`(std\.[a-z0-9.]+)`\)", h.group(3))
                if named:
                    module = named.group(1)
                elif section in SECTION_MODULE:
                    module = SECTION_MODULE[section]
                elif h.group(1) == "##":
                    module = None  # a top-level section without a module name
                # a ### subsection without a name keeps its parent's module
                if f.name != "appendix-stdlib.md" and not named:
                    module = None  # other appendices: resolve by name only
                continue
            if not line.startswith("| `") or (f.name == "appendix-stdlib.md" and section in SKIP_SECTIONS):
                continue
            cells = [c.strip() for c in line.strip().strip("|").split("|")]
            yield f.name, lineno, module, cells


def calls_in(cell: str):
    for span in re.findall(r"`([^`]+)`", cell):
        found = False
        for m in re.finditer(r"(\.)?\b([a-z][A-Za-z0-9_]*)\s*(<[^<>()]*>)?\s*\(([^()]*)\)", span):
            found = True
            yield m.group(2), m.group(1) == ".", m.group(4)
        if not found and re.fullmatch(r"[a-z][A-Za-z0-9_]*", span.strip()):
            yield span.strip(), False, None


def main() -> int:
    args = sys.argv[1:]
    files = [Path(a) for a in args[args.index("--appendix") + 1:]] if "--appendix" in args else [
        ROOT / "spec" / "appendix-stdlib.md", ROOT / "spec" / "appendix-amiga.md"]
    source = index_source()
    every = {n: m for m, fs in source.items() for n in fs}
    reports, mentioned = [], set()
    for fname, lineno, module, cells in doc_rows(files):
        ret_cell = cells[1] if len(cells) > 1 else ""
        names = list(calls_in(cells[0]))
        for name, dot, params in names:
            mentioned.add(name)
            where = f"{fname}:{lineno}"
            mod = module if module in source and name in source[module] else every.get(name)
            if mod is None:
                reports.append(("missing", where, f"{name} (section module {module})"))
                continue
            if module and mod != module and module in source:
                reports.append(("module", where, f"{name} documented under {module}, defined in {mod}"))
            sig = source[mod][name]
            if params is not None and "..." not in params:
                doc = [parse_param(p) for p in split_top(params)]
                if dot:
                    doc = [(None, None)] + doc
                bare_modes = all(p.strip() in MODES for p in split_top(params)) if params.strip() else False
                if len(doc) != len(sig["params"]) and not (bare_modes and dot):
                    reports.append(("arity", where, f"{name}: documented {len(doc)}, declared {len(sig['params'])}"))
                else:
                    for i, ((dm, dt), (sm, st)) in enumerate(zip(doc, sig["params"])):
                        if dm is not None and dm != sm:
                            reports.append(("mode", where, f"{name} param {i}: documented {dm}, declared {sm}"))
                        if dt is not None and st is not None and norm_type(dt) != norm_type(st):
                            reports.append(("type", where, f"{name} param {i}: documented {dt}, declared {st}"))
            if len(names) == 1:
                rt = ret_cell.replace("`", "").strip()
                if re.fullmatch(r"[A-Za-z_][\w<>\[\]?,]*|—|-", rt):
                    if norm_type(rt) != sig["ret"]:
                        reports.append(("returns", where, f"{name}: documented {rt}, declared {sig['ret']}"))
    undocumented = sorted(f"{m}.{n}" for m, fs in source.items() for n in fs
                          if n not in mentioned and not (m == "std.mem" and n in ("boxPtr", "len", "rawSlice", "rawStr", "boxAlloc")))
    for kind, where, text in reports:
        print(f"{kind}\t{where}\t{text}")
    print(f"conformance: {len(reports)} report(s); {len(undocumented)} undocumented public function(s)")
    for u in undocumented:
        print(f"undocumented\t{u}")
    return 1 if reports else 0


if __name__ == "__main__":
    sys.exit(main())
