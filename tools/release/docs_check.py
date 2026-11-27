"""Check a release's documentation before the tag and after the site deploys.

`make release-docs-check VERSION=X.Y.Z` runs the static mode on the working
tree: every release-facing file the manifest lists must name this release
where it should, carry every platform's install command, avoid banned
wording, and mention older versions only in allowlisted, re-reviewed phrases;
the release's review record must have its "Before tag" items ticked.
`LIVE=1` adds the live mode (see `docs_live.py`), which checks the deployed
site, the published release and the project links.

Exit codes: 0 pass, 1 findings, 2 usage or manifest error, 3 a probe was
unreachable, 4 the deployed site is not yet the current commit.
"""

from __future__ import annotations

import argparse
import datetime as dt
import json
import re
import sys
from pathlib import Path
from typing import Callable, TextIO

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

from docs_rules import (  # noqa: E402
    Finding,
    Manifest,
    ManifestError,
    committed_text,
    context,
    date_problem,
    load_manifest,
    normalize,
    parse_day,
    phrase_spans,
    review_findings,
    tokens,
)

REPO = HERE.parents[1]
EXIT_OK, EXIT_FINDINGS, EXIT_USAGE, EXIT_UNREACHABLE, EXIT_NOT_DEPLOYED = 0, 1, 2, 3, 4


class UsageError(Exception):
    """The command line or the release version is invalid."""


def release_version(text: str) -> str:
    """`X.Y.Z` from the command line; one leading `v` is accepted."""
    value = text[1:] if text.startswith("v") else text
    if not re.fullmatch(r"\d+\.\d+\.\d+", value):
        raise UsageError(f"the release version must be X.Y.Z, got {text!r}")
    return value


class Static:
    """The static rules over the working tree of one repository."""

    def __init__(self, repo: Path, manifest: Manifest, version: str, today: dt.date,
                 manifest_path: str = "tools/release/docs-manifest.json") -> None:
        self.repo = repo
        self.manifest_path = manifest_path
        self.manifest = manifest
        self.version = version
        self.minor = ".".join(version.split(".")[:2])
        self.today = today
        self.findings: list[Finding] = []
        self._cache: dict[str, str] = {}

    # Files ---------------------------------------------------------------

    def raw(self, path: str) -> str:
        if path not in self._cache:
            try:
                self._cache[path] = (self.repo / path).read_text(encoding="utf-8")
            except FileNotFoundError:
                self._cache[path] = ""
                self.add(path, "unreadable", "git tracks this file but it is missing from the working tree")
            except UnicodeDecodeError as error:
                self._cache[path] = ""
                self.add(path, "unreadable", f"not UTF-8 text ({error.reason} at byte {error.start})")
        return self._cache[path]

    def add(self, where: str, rule: str, message: str, extra: str = "") -> None:
        self.findings.append(Finding(where, rule, message, extra))

    def fill(self, pattern: str) -> str:
        return pattern.replace("{version}", self.version).replace("{minor}", self.minor)

    # Rules ---------------------------------------------------------------

    def run(self, record_sections: tuple[str, ...] = ("Before tag",)) -> list[Finding]:
        self.version_input()
        self.unclassified()
        self.versions()
        self.required()
        self.platforms()
        self.banned()
        self.placeholder()
        self.dated()
        self.changelogs()
        self.review(record_sections)
        return self.findings

    def version_input(self) -> None:
        cargo = self.raw("compiler/Cargo.toml") if "compiler/Cargo.toml" in self.manifest.tracked else ""
        section = re.search(r"^\[package\]\s*$(?P<body>.*?)(?=^\[|\Z)", cargo, re.M | re.S)
        found = re.search(r'^version\s*=\s*"([^"]+)"', section.group("body"), re.M) if section else None
        if not found or found.group(1) != self.version:
            have = found.group(1) if found else "nothing"
            self.add("compiler/Cargo.toml", "version-input", f"the compiler version is {have}, not {self.version}")
        package = "tools/vscode/package.json"
        try:
            have = json.loads(self.raw(package)).get("version") if package in self.manifest.tracked else None
        except json.JSONDecodeError:
            have = None
        if have != self.version:
            self.add(package, "version-input", f"the extension version is {have}, not {self.version}")
        for entry in self.manifest["changelogs"]:
            top = self._top_heading(entry)
            if top is None or top.group("version") != self.version:
                have = top.group("version") if top else "no released heading"
                self.add(entry["file"], "version-input", f"the newest released section is {have}, not {self.version}")
        notes = self.manifest["placeholder"]["file"]
        title = next((line for line in self.raw(notes).splitlines() if line.startswith("# ")), "")
        if f"v{self.version}" not in title:
            self.add(f"{notes}:1", "version-input", f"the release-notes title does not name v{self.version}")

    def unclassified(self) -> None:
        known = set(self.manifest.scan_set) | {entry["path"] for entry in self.manifest["excluded"]}
        for name in sorted(self.manifest.tracked):
            markdown = name.lower().endswith(".md")
            top_level = "/" not in name and markdown
            in_docs = name.startswith("docs/") and name.count("/") == 1 and markdown
            if (top_level or in_docs) and name not in known:
                self.add(name, "unclassified",
                         "this Markdown file is neither scanned nor excluded in the docs manifest")

    def versions(self) -> None:
        entries = self.manifest["allowlist"]
        used = [False] * len(entries)
        for index, entry in enumerate(entries):
            if entry["reviewed_for"] != self.version:
                self.add(self.manifest_path, "allowlist-review",
                         f"re-confirm the entry for {entry['file']} ({entry['phrase']!r}) and set reviewed_for to {self.version}")
        for path in self.manifest.scan_set:
            text = normalize(self.raw(path))
            spans = []
            for index, entry in enumerate(entries):
                if entry["file"] != path:
                    continue
                found = phrase_spans(text.text, entry["phrase"])
                if found:
                    used[index] = True
                spans += found
            exempt_from = self._exempt_line(path)
            for token in tokens(text.text):
                if token.is_current(self.version):
                    continue
                line = text.line_at(token.start)
                if exempt_from is not None and line >= exempt_from:
                    continue
                if any(start <= token.start and token.end <= end for start, end in spans):
                    continue
                entry = json.dumps({"file": path, "phrase": context(text.text, token), "reason": "",
                                    "reviewed_for": self.version})
                expected = self.version if token.asset or token.number.count(".") == 2 else self.minor
                self.add(f"{path}:{line}", "stale-version",
                         f"{token.text} is not v{expected}; update it or allowlist it as history", entry)
        for index, entry in enumerate(entries):
            if not used[index]:
                self.add(self.manifest_path, "allowlist-unused",
                         f"the entry for {entry['file']} no longer matches: {entry['phrase']!r}")

    def required(self) -> None:
        for entry in self.manifest["required"]:
            wanted = self.fill(entry["pattern"])
            if normalize(wanted).text not in normalize(self.raw(entry["file"])).text:
                self.add(entry["file"], "required", f"{entry['why']}: expected {wanted!r}")

    def platforms(self) -> None:
        navigation = self.manifest["navigation"]
        sidebar = self.raw(navigation["sidebar"])
        quick_start = self.raw(navigation["quick_start"]).splitlines()
        for platform in self.manifest["platforms"]:
            command = normalize(self.fill(platform["command"])).text
            name = platform["name"]
            for surface in [*self.manifest["install_surfaces"], platform["guide"]]:
                if command not in normalize(self.raw(surface)).text:
                    self.add(surface, "platform", f"the {name} install command for v{self.version} is missing")
            link = f"{navigation['quick_start_url']}#{platform['anchor']}"
            item = re.compile(r"\{\s*text:\s*([\"'])" + re.escape(platform["sidebar_text"]) + r"\1\s*,\s*link:\s*([\"'])"
                              + re.escape(link) + r"\2\s*,?\s*\}")
            if not item.search(sidebar):
                self.add(navigation["sidebar"], "platform",
                         f"the sidebar needs '{platform['sidebar_text']}' linking to {link} for {name}")
            if platform["heading"] not in (line.rstrip() for line in quick_start):
                self.add(navigation["quick_start"], "platform",
                         f"the quick start needs the heading '{platform['heading']}' for {name}")

    def banned(self) -> None:
        placeholder = self.manifest["placeholder"]
        prefix = placeholder["line_prefix"].lower()
        for path in self.manifest.scan_set:
            raw = self.raw(path)
            # Only the one placeholder line, and only for the pattern it starts with, is exempt.
            exempt = None
            if path == placeholder["file"]:
                exempt = next((number for number, line in enumerate(raw.splitlines(), 1)
                               if line.startswith(placeholder["line_prefix"])), None)
            text = normalize(raw)
            lowered = text.text.lower()
            for entry in self.manifest["banned"]:
                needle = normalize(entry["pattern"]).text.lower()
                start = lowered.find(needle)
                while start >= 0:
                    line = text.line_at(start)
                    if not (line == exempt and prefix.startswith(needle)):
                        self.add(f"{path}:{line}", "banned", f"'{entry['pattern']}': {entry['message']}")
                    start = lowered.find(needle, start + 1)

    def placeholder(self) -> None:
        entry = self.manifest["placeholder"]
        committed = committed_text(self.repo, entry["file"])
        count = sum(1 for line in (committed or "").splitlines() if line.startswith(entry["line_prefix"]))
        if count != 1:
            self.add(entry["file"], "placeholder",
                     f"the committed file must keep exactly one '{entry['line_prefix']}' line (found {count})")
        # The release steps replace the line in an uncommitted copy, so the
        # working tree may hold none; it must never hold more than one.
        count = sum(1 for line in self.raw(entry["file"]).splitlines() if line.startswith(entry["line_prefix"]))
        if count > 1:
            self.add(entry["file"], "placeholder",
                     f"the working-tree file has {count} '{entry['line_prefix']}' lines")

    def dated(self) -> None:
        for entry in self.manifest["dated"]:
            text = normalize(self.raw(entry["file"]))
            match = re.search(entry["pattern"], text.text)
            if not match:
                self.add(entry["file"], "dated", "the dated line is missing")
                continue
            where = f"{entry['file']}:{text.line_at(match.start())}"
            date = parse_day(match.group("day"), match.group("month"), match.group("year"))
            if date is None:
                self.add(where, "dated", f"cannot read the date {match.group()!r}")
                continue
            problem = date_problem(date, self.today)
            if problem:
                self.add(where, "dated", problem)

    def changelogs(self) -> None:
        for entry in self.manifest["changelogs"]:
            top = self._top_heading(entry)
            if top is None:
                self.add(entry["file"], "changelog", "no released section heading")
                continue
            where = f"{entry['file']}:{self.raw(entry['file']).count(chr(10), 0, top.start()) + 1}"
            if not entry["dated"]:
                continue
            raw_date = top.groupdict().get("date")
            if not raw_date:
                self.add(where, "changelog", f"the newest released heading needs a date: [{self.version}] - YYYY-MM-DD")
                continue
            try:
                problem = date_problem(dt.date.fromisoformat(raw_date), self.today)
            except ValueError:
                problem = f"cannot read the date {raw_date!r}"
            if problem:
                self.add(where, "changelog", problem)

    def review(self, sections: tuple[str, ...]) -> None:
        review = self.manifest["review"]
        path = f"{review['dir'].rstrip('/')}/v{self.version}.md"
        record = self.raw(path) if (self.repo / path).is_file() else None
        self.findings += review_findings(path, record, self.raw(review["template"]), self.version, sections,
                                         tracked=path in self.manifest.tracked)

    # Helpers -------------------------------------------------------------

    def _changelog(self, path: str) -> dict | None:
        return next((entry for entry in self.manifest["changelogs"] if entry["file"] == path), None)

    def _top_heading(self, entry: dict):
        return next(re.finditer(entry["heading"], self.raw(entry["file"]), re.M), None)

    def _exempt_line(self, path: str) -> int | None:
        """The first line of a changelog's older sections, which are history."""
        entry = self._changelog(path)
        if entry is None:
            return None
        headings = list(re.finditer(entry["heading"], self.raw(path), re.M))
        if len(headings) < 2:
            return None
        return self.raw(path).count("\n", 0, headings[1].start()) + 1


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(prog="release-docs-check", description=__doc__.splitlines()[0])
    parser.add_argument("--version", required=True, help="the release version, X.Y.Z")
    parser.add_argument("--live", action="store_true", help="also check the deployed site and the published release")
    parser.add_argument("--today", help="the date to check against, YYYY-MM-DD (default: today)")
    parser.add_argument("--repo", default=str(REPO), help=argparse.SUPPRESS)
    parser.add_argument("--manifest", help=argparse.SUPPRESS)
    return parser.parse_args(argv)


def main(argv: list[str] | None = None, stdout: TextIO = sys.stdout, stderr: TextIO = sys.stderr,
         fetcher: Callable | None = None, gh: Callable | None = None) -> int:
    try:
        args = parse_args(list(sys.argv[1:] if argv is None else argv))
    except SystemExit as exit_:
        return EXIT_USAGE if exit_.code else EXIT_OK
    try:
        version = release_version(args.version)
        today = dt.date.fromisoformat(args.today) if args.today else dt.date.today()
    except (UsageError, ValueError) as error:
        print(f"release-docs-check: {error}", file=stderr)
        return EXIT_USAGE
    repo = Path(args.repo).resolve()
    manifest_path = Path(args.manifest) if args.manifest else repo / "tools/release/docs-manifest.json"
    try:
        manifest = load_manifest(manifest_path, repo)
    except ManifestError as error:
        print(f"release-docs-check: {error}", file=stderr)
        return EXIT_USAGE
    sections = ("Before tag", "After deploy") if args.live else ("Before tag",)
    try:
        shown = str(manifest_path.resolve().relative_to(repo))
    except ValueError:
        shown = str(manifest_path)
    findings = Static(repo, manifest, version, today, shown).run(sections)
    unreachable: list[Finding] = []
    if args.live:
        import docs_live

        outcome = docs_live.check(repo, manifest, version, fetcher=fetcher, gh=gh)
        if outcome.not_deployed is not None:
            for finding in sorted(set(findings), key=Finding.sort_key):
                print(finding.render(), file=stdout)
            print(outcome.not_deployed.render(), file=stdout)
            print("release-docs-check: the deployed site is not the current commit yet; "
                  "wait for the docs deploy and retry", file=stdout)
            return EXIT_NOT_DEPLOYED
        findings += outcome.findings
        unreachable = outcome.unreachable
        if outcome.prerelease is not None:
            release_kind = "prerelease" if outcome.prerelease else "release"
    unique = sorted(set(findings), key=Finding.sort_key)
    for finding in unique:
        print(finding.render(), file=stdout)
    for finding in sorted(set(unreachable), key=Finding.sort_key):
        print(finding.render(), file=stdout)
    mode = "static and live" if args.live else "static"
    if args.live and outcome.prerelease is not None:
        mode += f", published as a {release_kind}"
    print(f"release-docs-check: {len(unique)} findings, {len(set(unreachable))} unreachable "
          f"(v{version}, {mode})", file=stdout)
    if unique:
        return EXIT_FINDINGS
    if unreachable:
        return EXIT_UNREACHABLE
    return EXIT_OK


if __name__ == "__main__":
    sys.exit(main())
