#!/usr/bin/env python3
"""Probe runner for the assessment workstreams.

Usage (inside the SDK container, from the repository root):

    python3 <plan>/probes/WS1/probe.py <probe-dir> --clc <pinned-clc>
        [--out DIR] [--only ID ...] [--contrast] [--jobs N]

A probe is ``<NNN>-<slug>.clo`` (or a folder ``<NNN>-<slug>/`` whose ``.clo``
files are compiled together) plus ``<NNN>-<slug>.expected``. The expectation
file is written and committed before the probe runs. Lines starting with
``#`` cite the expectation source. Keys:

    profiles: host hosted-amiga hosted-amiga-13 freestanding
    expect: accept | reject <CODE> | output | fault <CODE> | companion | nocrash
            (companion-differs: the output must differ from the companion's;
             build-fails: clc accepts and the debug C build must fail)
    companion: <file.c>          C program whose output the probe must match
    limit-ms: <N>                wall-time limit for ``nocrash``
    with: <a.clo> [<b.clo> ...]  extra sources (in the probe folder) compiled with the probe
    contrast: <from> ==> <to>    literal source rewrite for the contrasting input
    contrast-expect: <kind>      what the contrasting input must do
    --- output                   expected stdout follows (for ``output``)
    --- contrast-output          expected stdout of the contrasting input

``accept``, ``output``, ``fault``, and ``companion`` build every accepted
emission with the profile's debug line (warnings as errors) and its shipped
release line; ``output`` and ``fault`` run both builds (host natively, Amiga
under ``vamos``). ``freestanding`` is compiled (``-c``) but not run.
``reject`` requires the first ``error Exxxxx`` code to match. With
``--sanitize`` the host profile also builds and runs an AddressSanitizer line
(``-fsanitize=address -no-pie``, ``ASAN_OPTIONS=detect_leaks=1``) and an
UndefinedBehaviorSanitizer line (``-fsanitize=undefined
-fno-sanitize-recover=all -no-pie``); any sanitizer report fails the check. ``nocrash``
requires ``clc`` to end with exit 0 or 1 (no panic, signal, or timeout) within
``limit-ms``.

With ``--contrast`` only the contrasting inputs run (the falsifiability
check). Results go to ``<out>/results.json``; each line printed is
``PASS|FAIL <probe> <profile> <mode>: <detail>``. Exit 1 if any check fails.
"""

from __future__ import annotations

import argparse
import concurrent.futures
import json
import re
import shutil
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(subprocess.run(["git", "rev-parse", "--show-toplevel"], capture_output=True,
                           text=True, check=True).stdout.strip())
sys.path.insert(0, str(ROOT / "tools" / "perf"))
import bench  # noqa: E402

SCRATCH = ROOT / "build" / "tmp" / "assessment"
WARN = ["-Wall", "-Wextra", "-Werror"]
GC = ["-ffunction-sections", "-fdata-sections", "-Wl,--gc-sections", "-s"]
LIBS = ["-lamiga", "-lm"]
BUILDS = {
    "host": [("debug", "cc", ["-std=c99", "-O0", "-g", *WARN], []),
             ("release", "cc", ["-std=c99", "-O2", *GC], [])],
    "hosted-amiga": [
        ("debug", "m68k-amigaos-gcc",
         ["-std=c99", "-mcrt=nix20", "-m68000", "-fwrapv", "-O0", "-g", *WARN], LIBS),
        ("release", "m68k-amigaos-gcc",
         ["-std=c99", "-mcrt=nix20", "-m68000", "-fwrapv", "-O2", *GC], LIBS)],
    "hosted-amiga-13": [
        ("debug", "m68k-amigaos-gcc",
         ["-std=c99", "-mcrt=nix13", "-m68000", "-fwrapv", "-O0", "-g", *WARN], LIBS),
        ("release", "m68k-amigaos-gcc",
         ["-std=c99", "-mcrt=nix13", "-m68000", "-fwrapv", "-O2", *GC], LIBS)],
    "freestanding": [
        ("debug", "m68k-amigaos-gcc",
         ["-std=c99", "-ffreestanding", "-fno-builtin", "-nostdlib", "-m68000", "-fwrapv",
          "-O0", "-g", *WARN, "-c"], []),
        ("release", "m68k-amigaos-gcc",
         ["-std=c99", "-ffreestanding", "-fno-builtin", "-nostdlib", "-m68000", "-fwrapv",
          "-O2", "-c"], [])],
}
SANITIZE = [
    ("asan", "cc", ["-std=c99", "-O0", "-g", "-fsanitize=address", "-fno-omit-frame-pointer",
                    "-no-pie"], []),
    ("ubsan", "cc", ["-std=c99", "-O0", "-g", "-fsanitize=undefined",
                     "-fno-sanitize-recover=all", "-no-pie"], []),
]
SAN_ENV = {"ASAN_OPTIONS": "detect_leaks=1", "UBSAN_OPTIONS": "print_stacktrace=1"}
CODE_RE = re.compile(r"error (E\d{5})")


def parse_expected(path: Path) -> dict:
    """Read an expectation file into a dictionary."""
    spec: dict = {"profiles": ["host", "hosted-amiga", "hosted-amiga-13"], "output": [],
                  "contrast-output": []}
    section = None
    for raw in path.read_text(encoding="utf-8").splitlines():
        if raw.startswith("--- "):
            section = raw[4:].strip()
            continue
        if section is not None:
            spec[section].append(raw)
            continue
        if raw.startswith("#") or not raw.strip():
            continue
        key, _, value = raw.partition(":")
        key, value = key.strip(), value.strip()
        if key == "profiles":
            spec["profiles"] = value.split()
        elif key == "contrast":
            src, _, dst = value.partition("==>")
            spec["contrast"] = (src.strip().replace("\\n", "\n"), dst.strip().replace("\\n", "\n"))
        else:
            spec[key] = value
    for key in ("output", "contrast-output"):
        spec[key] = bench.normalize_output("\n".join(spec[key]).encode()) if spec[key] else ""
    return spec


def run(argv: list[str], timeout: int, cwd: Path | None = None,
        env: dict | None = None) -> tuple[int, str, str, float]:
    """Run a command; return (exit code, stdout, stderr, seconds). Timeout is code -9."""
    start = time.monotonic()
    full_env = None
    if env:
        import os
        full_env = dict(os.environ, **env)
    try:
        done = subprocess.run(argv, capture_output=True, timeout=timeout, cwd=cwd or ROOT,
                              env=full_env)
        return (done.returncode, done.stdout.decode("utf-8", "replace"),
                done.stderr.decode("utf-8", "replace"), time.monotonic() - start)
    except subprocess.TimeoutExpired:
        return -9, "", f"timeout after {timeout}s", time.monotonic() - start


def first_code(text: str) -> str | None:
    match = CODE_RE.search(text)
    return match.group(1) if match else None


def check(clc: Path, pid: str, sources: list[Path], kind: str, spec: dict, expected_out: str,
          profile: str, work: Path, sanitize: bool = False) -> list[tuple[bool, str, str]]:
    """Check one probe (or its contrast) on one profile; return (ok, mode, detail) rows."""
    work.mkdir(parents=True, exist_ok=True)
    c_file = work / "prog.c"
    word, _, arg = kind.partition(" ")
    limit = int(spec.get("limit-ms", "0") or 0)
    rc, out, err, secs = run([str(clc), "--profile", profile, "-o", str(c_file),
                              *map(str, sources)], timeout=max(120, limit // 1000 * 3 + 10))
    (work / "clc.log").write_text(out + err, encoding="utf-8")
    code = first_code(err + out)
    if word == "nocrash":
        ok = rc in (0, 1) and "panicked" not in err and (limit == 0 or secs * 1000 <= limit)
        return [(ok, "clc", f"rc={rc} {secs*1000:.0f}ms code={code} limit={limit}ms")]
    if word == "reject":
        ok = rc != 0 and code == arg and "panicked" not in err
        return [(ok, "clc", f"rc={rc} first={code} expected={arg}")]
    if rc != 0:
        return [(False, "clc", f"rejected rc={rc} first={code}")]
    if word == "build-fails":
        _, cc, flags, libs = BUILDS[profile][0]
        brc, bout, berr, _ = run([cc, *flags, "-o", str(work / "prog-debug"), str(c_file), *libs], 600)
        (work / "build-debug.log").write_text(bout + berr, encoding="utf-8")
        first = next((ln for ln in berr.splitlines() if "error" in ln), "")
        return [(brc != 0, "debug", f"C build rc={brc} {first[:160]}")]
    rows = []
    builds = BUILDS[profile] + (SANITIZE if sanitize and profile == "host" else [])
    for mode, cc, flags, libs in builds:
        binary = work / f"prog-{mode}"
        brc, bout, berr, _ = run([cc, *flags, "-o", str(binary), str(c_file), *libs], 600)
        (work / f"build-{mode}.log").write_text(bout + berr, encoding="utf-8")
        if brc != 0:
            rows.append((False, mode, "C build failed: " + (berr.strip().splitlines() or [""])[0][:200]))
            continue
        if word == "accept" or profile == "freestanding":
            rows.append((True, mode, "built"))
            continue
        runner = [str(binary)] if profile == "host" else ["vamos", str(binary)]
        xrc, xout, xerr, _ = run(runner, 120, env=SAN_ENV if mode in ("asan", "ubsan") else None)
        (work / f"run-{mode}.log").write_text(f"rc={xrc}\n--- stdout\n{xout}\n--- stderr\n{xerr}",
                                              encoding="utf-8")
        got = bench.normalize_output(xout.encode())
        if word == "output":
            report = next((ln for ln in xerr.splitlines()
                           if "Sanitizer" in ln or "runtime error" in ln), "")
            ok = xrc == 0 and got == expected_out and not report
            rows.append((ok, mode, f"rc={xrc}" + ("" if ok else f" got={got!r} want={expected_out!r}")
                         + (f" SANITIZER: {report[:160]}" if report else "")))
        elif word == "fault":
            text = xerr if profile == "host" else xout
            ok = xrc != 0 and f"error {arg}:" in text
            rows.append((ok, mode, f"rc={xrc} first={first_code(text)} expected={arg}"))
        elif word in ("companion", "companion-differs"):
            comp = (sources[0].parent / spec["companion"])
            cbin = work / f"companion-{mode}"
            crc, _, cerr, _ = run([cc, *[f for f in flags if f not in WARN], "-o", str(cbin),
                                   str(comp), *libs], 600)
            if crc != 0:
                rows.append((False, mode, "companion build failed: " + cerr[:200]))
                continue
            crunner = [str(cbin)] if profile == "host" else ["vamos", str(cbin)]
            _, cout, _, _ = run(crunner, 120)
            want = bench.normalize_output(cout.encode())
            same = xrc == 0 and got == want
            ok = same if word == "companion" else (xrc == 0 and not same)
            rows.append((ok, mode, ("matches companion" if same else "differs from companion")
                         + ("" if ok else f" got={got!r} companion={want!r}")))
    return rows


def main() -> int:
    parser = argparse.ArgumentParser(description="assessment probe runner")
    parser.add_argument("probe_dir")
    parser.add_argument("--clc", required=True)
    parser.add_argument("--out", default=None)
    parser.add_argument("--only", nargs="+", default=None)
    parser.add_argument("--contrast", action="store_true")
    parser.add_argument("--jobs", type=int, default=6)
    parser.add_argument("--sanitize", action="store_true",
                        help="also build and run the host ASan and UBSan lines")
    args = parser.parse_args()
    pdir = (ROOT / args.probe_dir).resolve()
    out = (ROOT / (args.out or f"build/tmp/assessment/{pdir.name}/probes")).resolve()
    out.relative_to(SCRATCH.resolve())
    clc = (ROOT / args.clc).resolve()
    jobs = []
    for exp in sorted(pdir.glob("*.expected")):
        pid = exp.stem
        if args.only and not any(pid.startswith(o) for o in args.only):
            continue
        spec = parse_expected(exp)
        if "expect" not in spec or spec["expect"] == "script":
            continue
        folder = pdir / pid
        sources = sorted(folder.glob("*.clo")) if folder.is_dir() else [pdir / f"{pid}.clo"]
        sources += [pdir / extra for extra in spec.get("with", "").split()]
        if not all(s.is_file() for s in sources) or not sources:
            print(f"FAIL {pid} - setup: missing source")
            continue
        if args.contrast:
            if "contrast" not in spec:
                continue
            src, dst = spec["contrast"]
            cdir = out / pid / "contrast-src"
            cdir.mkdir(parents=True, exist_ok=True)
            new_sources = []
            hits = 0
            for s in sources:
                if s.parent == pdir and s.name in spec.get("with", "").split():
                    shutil.copy(s, cdir / s.name)
                    new_sources.append(cdir / s.name)
                    continue
                text = s.read_text(encoding="utf-8")
                hits += text.count(src)
                (cdir / s.name).write_text(text.replace(src, dst), encoding="utf-8")
                new_sources.append(cdir / s.name)
            if "companion" in spec:
                shutil.copy(pdir / spec["companion"], cdir / spec["companion"])
            for extra in pdir.glob("*.bin"):
                shutil.copy(extra, cdir / extra.name)
            for extra in pdir.glob("data*"):
                if extra.is_dir():
                    shutil.copytree(extra, cdir / extra.name, dirs_exist_ok=True)
            if hits == 0:
                print(f"FAIL {pid} - contrast: pattern not found")
                continue
            for profile in spec["profiles"]:
                jobs.append((pid, new_sources, spec["contrast-expect"], spec,
                             spec["contrast-output"], profile, out / pid / "contrast" / profile))
        else:
            for profile in spec["profiles"]:
                jobs.append((pid, sources, spec["expect"], spec, spec["output"], profile,
                             out / pid / profile))
    results = []
    with concurrent.futures.ThreadPoolExecutor(max_workers=args.jobs) as pool:
        futures = {pool.submit(check, clc, *job, sanitize=args.sanitize): job for job in jobs}
        for future in concurrent.futures.as_completed(futures):
            pid, _, kind, _, _, profile, _ = futures[future]
            for ok, mode, detail in future.result():
                results.append({"probe": pid, "profile": profile, "mode": mode, "kind": kind,
                                "ok": ok, "detail": detail, "contrast": args.contrast})
    results.sort(key=lambda r: (r["probe"], r["profile"], r["mode"]))
    for r in results:
        print(f"{'PASS' if r['ok'] else 'FAIL'} {r['probe']} {r['profile']} {r['mode']}: "
              f"[{r['kind']}] {r['detail']}")
    out.mkdir(parents=True, exist_ok=True)
    name = "contrast.json" if args.contrast else "results.json"
    (out / name).write_text(json.dumps(results, indent=2) + "\n", encoding="utf-8")
    failed = sum(not r["ok"] for r in results)
    print(f"probe: {len(results) - failed} pass, {failed} fail; wrote {out / name}")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
