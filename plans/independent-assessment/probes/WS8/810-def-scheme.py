#!/usr/bin/env python3
"""WS8-07: classify every DEF row of plans/00-roadmap.md by class and component.

Rows: the tracker's `| ↳ DEF-n |` rows (columns: ID, title, -, fixing plan, stage,
icon, date, notes; the parent is the preceding PLAN row) and the backlog ledger's
`| DEF-n |` rows (ID, title, found, fixed, notes).

Class — the first rule whose pattern matches the title, then the notes:
  memory-safety  double free, use after free, dangling, leak, freed twice, ASan report
  invalid-c      the emitted C fails a compile-matrix leg (invalid C, strict build,
                 conflicting types, incompatible type, -Werror, C compiler rejects)
  miscompile     an accepted program does the wrong thing (wrong value/order, evaluates
                 twice, re-evaluates, drops a step, skips, loses, stale, miscompiles)
  false-reject   a valid program is rejected (rejects a valid, wrongly rejected,
                 spurious diagnostic)
  false-accept   an invalid program is accepted (is accepted, accepts, not reported,
                 bypasses a guard, missing diagnostic)
  platform       Amiga/m68k-only behavior (Amiga, m68k, vbcc, Kickstart, FS-UAE, vamos,
                 shim, libnix, NDK)
  performance    cost only (slow, instructions, .text, size, quadratic, time)
  other          none of the above (reported for reading)

Component — the first matching group: ownership (own, drop, move, borrow, loan, lend,
release, take), codegen (emit, lowering, temporary, materializ, C text), sema (type,
generic, instance, infer, const, fold, diagnostic), front-end (parse, lex, grammar,
token), runtime (shim, runtime, `cl_`), stdlib (`std.`, stdlib, Array, Buffer), tooling
(LSP, driver, docgen, dist, install, VS Code, site), else other.

A class is **recurring** with at least 3 rows across at least 2 plans (03-03 WS8-07);
the plan of a row is its fixing-plan link, else its parent PLAN row.

Usage: 810-def-scheme.py [--rows] [--only DEF-n ...]
"""
import re
import sys
from collections import Counter, defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parents[4]
CLASSES = [
    ("memory-safety", r"double[- ]free|use[- ]after[- ]free|dangling|\bleak|freed twice|asan"),
    ("invalid-c", r"invalid c\b|strict (host )?build|conflicting types|incompatible type|-werror|c compiler (rejects|fails)|fails to compile|not valid c"),
    ("miscompile", r"wrong (value|result|order|element|slot)|evaluates? .*twice|re-?evaluat|drops the|miscompil|skips|loses|stale|out of order|silently|returns the wrong"),
    ("false-reject", r"rejects? (a |the )?valid|wrongly rejected|spurious|false (error|rejection)|rejects .* that the spec"),
    ("false-accept", r"is accepted|are accepted|\baccepts\b|not reported|bypass|missing (e\d|diagnostic)|without (a |an )?(error|diagnostic)"),
    ("platform", r"amiga|m68k|vbcc|kickstart|fs-uae|vamos|\bshim|libnix|\bndk\b|a500|a1200"),
    ("performance", r"\bslow|instructions|\.text|code size|quadratic|compile time|footprint"),
]
COMPONENTS = [
    ("ownership", r"\bown|drop|\bmove|borrow|loan|\blend|release|\btake\b"),
    ("codegen", r"emit|lowering|lowers|temporar|materializ|\bc text|codegen"),
    ("sema", r"\btype|generic|instance|infer|\bconst\b|fold|diagnostic|\be1\d{4}"),
    ("front-end", r"pars|\blex|grammar|token"),
    ("runtime", r"shim|runtime|\bcl_"),
    ("stdlib", r"\bstd\.|stdlib|\barray\b|\bbuffer\b"),
    ("tooling", r"\blsp\b|driver|docgen|\bdist\b|install|vs ?code|\bsite\b"),
]


def first(rules, text: str) -> str:
    for name, pattern in rules:
        if re.search(pattern, text, re.I):
            return name
    return "other"


def rows():
    parent = None
    for line in (ROOT / "plans" / "00-roadmap.md").read_text().splitlines():
        m = re.match(r"^\| (PLAN-\d+) \|[^|]*\|[^|]*\| \[([^\]]+)\]", line)
        if m:
            parent = m.group(2)
            continue
        cells = [c.strip() for c in line.strip().strip("|").split("|")]
        if line.startswith("| ↳ DEF-"):
            did = cells[0].replace("↳", "").strip()
            link = re.search(r"\[([^\]]+)\]\(", line)
            plan = link.group(1) if link else parent
            date = next((c for c in cells if re.match(r"\d{4}-\d{2}-\d{2}", c)), "")
            yield did, cells[1], cells[-1], plan or "-", date[:10]
        elif re.match(r"^\| DEF-\d+ \|", line):
            yield cells[0], cells[1], cells[-1], "backlog-ledger", cells[2][:10]


def main() -> int:
    only = set(sys.argv[sys.argv.index("--only") + 1:]) if "--only" in sys.argv else None
    table = []
    for did, title, notes, plan, date in rows():
        if only and did not in only:
            continue
        cls = first(CLASSES, title)
        if cls == "other":
            cls = first(CLASSES, notes)
        comp = first(COMPONENTS, title + " " + notes)
        table.append((did, date, plan, comp, cls, title))
    if "--rows" in sys.argv or only:
        for r in table:
            print("\t".join(r))
    by_class = Counter(r[4] for r in table)
    plans = defaultdict(set)
    months = defaultdict(Counter)
    for did, date, plan, comp, cls, _ in table:
        plans[cls].add(plan)
        months[date[:7]][cls] += 1
    print(f"rows {len(table)}")
    for cls, n in by_class.most_common():
        flag = "recurring" if n >= 3 and len(plans[cls]) >= 2 else "-"
        print(f"class\t{cls}\t{n}\tplans {len(plans[cls])}\t{flag}")
    for comp, n in Counter(r[3] for r in table).most_common():
        print(f"component\t{comp}\t{n}")
    for month in sorted(months):
        print(f"month\t{month}\t" + ", ".join(f"{k} {v}" for k, v in months[month].most_common()))
    return 0


if __name__ == "__main__":
    sys.exit(main())
