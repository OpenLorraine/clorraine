"""Shared rules for the release docs check.

This module owns the data model: loading and validating the manifest,
normalizing Markdown, Vue and HTML text into comparable prose, finding
Clorraine version tokens, matching allowlist phrases, parsing dates, and
reading review records. It performs no network access.
"""

from __future__ import annotations

import bisect
import datetime as dt
import html
import json
import re
import subprocess
from dataclasses import dataclass
from pathlib import Path

VERSION_RE = re.compile(r"^\d+\.\d+\.\d+$")
# A `v`-prefixed Clorraine-style version: `v1.3` or `v1.3.0`, with an optional
# prerelease suffix. The boundaries keep `v1.3.1` from matching inside
# `v1.3.10` and skip words such as `nv1.2`.
V_TOKEN = re.compile(
    r"(?<![\w.\-])v(?P<num>\d+\.\d+(?:\.\d+)?)(?:-(?P<pre>[0-9A-Za-z]+(?:\.[0-9A-Za-z]+)*))?(?![\w]|\.\d)"
)
# A release asset name such as `clorraine-1.3.0-x86_64-linux.tar.gz`.
ASSET_TOKEN = re.compile(r"(?<![\w.\-])clorraine-(?P<num>\d+\.\d+\.\d+)(?![\w]|\.\d)")
EMAIL = re.compile(r"[\w.+-]+@[\w-]+(?:\.[\w-]+)+")
MONTHS = {
    name: index
    for index, name in enumerate(
        ("January", "February", "March", "April", "May", "June", "July",
         "August", "September", "October", "November", "December"),
        start=1,
    )
}
DATE_WINDOW_DAYS = 14

MANIFEST_KEYS = {
    "site_url", "scan", "excluded", "platforms", "install_surfaces", "navigation",
    "required", "banned", "placeholder", "dated", "changelogs", "theme_files",
    "review", "allowlist",
}


class ManifestError(Exception):
    """The manifest is malformed or names a file the repository does not track."""


@dataclass(frozen=True)
class Finding:
    """One problem: where it is (`path:line` or a URL), its rule, and why."""

    where: str
    rule: str
    message: str
    extra: str = ""

    def sort_key(self) -> tuple:
        path, _, line = self.where.rpartition(":")
        if path and line.isdigit():
            return (path, int(line), self.rule, self.message)
        return (self.where, 0, self.rule, self.message)

    def render(self) -> str:
        text = f"{self.where} [{self.rule}] {self.message}"
        if self.extra:
            text += f"\n    allowlist entry: {self.extra}"
        return text


# --- Repository access -------------------------------------------------------


def tracked_files(repo: Path) -> list[str]:
    """The paths git tracks in `repo`, with `/` separators."""
    result = subprocess.run(
        ["git", "-C", str(repo), "ls-files", "-z"], capture_output=True, check=True
    )
    return [name for name in result.stdout.decode("utf-8").split("\0") if name]


def committed_text(repo: Path, path: str) -> str | None:
    """The text of `path` at HEAD, or None when HEAD does not hold it."""
    result = subprocess.run(
        ["git", "-C", str(repo), "show", f"HEAD:{path}"], capture_output=True
    )
    if result.returncode != 0:
        return None
    return result.stdout.decode("utf-8")


def glob_regex(pattern: str) -> re.Pattern[str]:
    """A regex for a path glob where `**/` spans directories and `*` does not."""
    out = []
    index = 0
    while index < len(pattern):
        if pattern.startswith("**/", index):
            out.append("(?:.*/)?")
            index += 3
        elif pattern.startswith("**", index):
            out.append(".*")
            index += 2
        elif pattern[index] == "*":
            out.append("[^/]*")
            index += 1
        elif pattern[index] == "?":
            out.append("[^/]")
            index += 1
        else:
            out.append(re.escape(pattern[index]))
            index += 1
    return re.compile("^" + "".join(out) + "$")


# --- Manifest ----------------------------------------------------------------


@dataclass
class Manifest:
    data: dict
    tracked: set[str]
    scan_set: list[str]

    def __getitem__(self, key: str):
        return self.data[key]


def load_manifest(path: Path, repo: Path) -> Manifest:
    """Read and validate the manifest against the repository's tracked files."""
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except FileNotFoundError:
        raise ManifestError(f"manifest not found: {path}") from None
    except json.JSONDecodeError as error:
        raise ManifestError(f"manifest is not valid JSON: {error}") from None
    if not isinstance(data, dict):
        raise ManifestError("manifest must be a JSON object")
    unknown = set(data) - MANIFEST_KEYS
    missing = MANIFEST_KEYS - set(data)
    if unknown:
        raise ManifestError(f"manifest has unknown keys {sorted(unknown)}")
    if missing:
        raise ManifestError(f"manifest is missing {sorted(missing)}")
    tracked = set(tracked_files(repo))

    def need_tracked(file: object, context: str) -> str:
        if not isinstance(file, str) or file not in tracked:
            raise ManifestError(f"{context}: {file!r} is not a tracked file")
        return file

    def need_text(value: object, context: str) -> str:
        if not isinstance(value, str) or not value.strip():
            raise ManifestError(f"{context} must be a non-empty string")
        return value

    def need_regex(value: object, context: str, groups: set[str]) -> None:
        need_text(value, context)
        try:
            compiled = re.compile(value, re.M)
        except re.error as error:
            raise ManifestError(f"{context} is not a valid regex: {error}") from None
        absent = groups - set(compiled.groupindex)
        if absent:
            raise ManifestError(f"{context} needs the groups {sorted(absent)}")

    need_text(data["site_url"], "site_url")
    if not data["site_url"].startswith("https://") or not data["site_url"].endswith("/"):
        raise ManifestError("site_url must be an https URL ending in /")
    scan = data["scan"]
    if set(scan) != {"files", "globs", "exclude"}:
        raise ManifestError("scan needs exactly files, globs, exclude")
    for file in scan["files"]:
        need_tracked(file, "scan.files")
    for entry in data["excluded"]:
        need_tracked(entry.get("path"), "excluded")
        need_text(entry.get("reason"), f"excluded {entry.get('path')} reason")
    for platform in data["platforms"]:
        for key in ("name", "command", "guide", "sidebar_text", "anchor", "heading"):
            need_text(platform.get(key), f"platform {platform.get('name')} {key}")
        if "{version}" not in platform["command"]:
            raise ManifestError(f"platform {platform['name']} command needs {{version}}")
        need_tracked(platform["guide"], f"platform {platform['name']} guide")
    for file in data["install_surfaces"]:
        need_tracked(file, "install_surfaces")
    navigation = data["navigation"]
    need_tracked(navigation.get("sidebar"), "navigation.sidebar")
    need_tracked(navigation.get("quick_start"), "navigation.quick_start")
    need_text(navigation.get("quick_start_url"), "navigation.quick_start_url")
    for entry in data["required"]:
        need_tracked(entry.get("file"), "required")
        need_text(entry.get("pattern"), f"required {entry.get('file')} pattern")
        need_text(entry.get("why"), f"required {entry.get('file')} why")
    for entry in data["banned"]:
        need_text(entry.get("pattern"), "banned pattern")
        need_text(entry.get("message"), f"banned {entry.get('pattern')} message")
    need_tracked(data["placeholder"].get("file"), "placeholder")
    need_text(data["placeholder"].get("line_prefix"), "placeholder.line_prefix")
    for entry in data["dated"]:
        need_tracked(entry.get("file"), "dated")
        need_regex(entry.get("pattern"), f"dated {entry.get('file')} pattern", {"day", "month", "year"})
    for entry in data["changelogs"]:
        need_tracked(entry.get("file"), "changelogs")
        need_regex(entry.get("heading"), f"changelog {entry.get('file')} heading", {"version"})
        if not isinstance(entry.get("dated"), bool):
            raise ManifestError(f"changelog {entry.get('file')} dated must be true or false")
        if entry["dated"]:
            need_regex(entry["heading"], f"changelog {entry['file']} heading", {"version", "date"})
    for entry in data["theme_files"]:
        need_tracked(entry.get("file"), "theme_files")
        if entry.get("pages") not in ("all", "home"):
            raise ManifestError(f"theme file {entry.get('file')} pages must be all or home")
    review = data["review"]
    need_tracked(review.get("template"), "review.template")
    need_text(review.get("dir"), "review.dir")
    for entry in data["allowlist"]:
        file = need_tracked(entry.get("file"), "allowlist")
        phrase = need_text(entry.get("phrase"), f"allowlist {file} phrase")
        need_text(entry.get("reason"), f"allowlist {file} {phrase!r} reason")
        if not VERSION_RE.match(str(entry.get("reviewed_for", ""))):
            raise ManifestError(f"allowlist {file} {phrase!r} reviewed_for must be X.Y.Z")
        if not tokens(normalize(phrase).text):
            raise ManifestError(f"allowlist {file} {phrase!r} names no version token")
        if set(entry) != {"file", "phrase", "reason", "reviewed_for"}:
            raise ManifestError(f"allowlist {file} {phrase!r} has unknown keys")

    includes = [glob_regex(pattern) for pattern in scan["globs"]]
    excludes = [glob_regex(pattern) for pattern in scan["exclude"]]
    scan_set = set(scan["files"])
    for name in tracked:
        if any(rx.match(name) for rx in includes) and not any(rx.match(name) for rx in excludes):
            scan_set.add(name)
    return Manifest(data=data, tracked=tracked, scan_set=sorted(scan_set))


# --- Normalization -----------------------------------------------------------


@dataclass
class Normalized:
    """Prose with markup removed, plus the source line of every character."""

    text: str
    lines: list[int]

    def line_at(self, offset: int) -> int:
        if not self.lines:
            return 1
        return self.lines[min(offset, len(self.lines) - 1)]


_NORMALIZE = re.compile(
    r"(?P<link>\[(?P<label>[^\]\n]*)\]\((?P<url>[^)\s]*)\))"
    r"|(?P<entity>&(?:#\d+|#x[0-9A-Fa-f]+|[A-Za-z][A-Za-z0-9]*);)"
    r"|(?P<mark>\*\*|__|`)"
    r"|(?P<zwsp>​)"
    r"|(?P<space>\s+)"
)


def normalize(raw: str) -> Normalized:
    """Prose for comparison: links become `label url`, entities are decoded,
    emphasis and code markers and zero-width spaces are dropped, and every
    whitespace run becomes one space."""
    starts = [0] + [m.end() for m in re.finditer("\n", raw)]

    def line(offset: int) -> int:
        return bisect.bisect_right(starts, offset)

    chars: list[str] = []
    lines: list[int] = []

    def emit(text: str, at: int) -> None:
        number = line(at)
        for ch in text:
            if ch.isspace():
                if chars and chars[-1] == " ":
                    continue
                ch = " "
            chars.append(ch)
            lines.append(number)

    position = 0
    for match in _NORMALIZE.finditer(raw):
        for offset in range(position, match.start()):
            emit(raw[offset], offset)
        kind = match.lastgroup
        if kind == "link":
            label = normalize(match.group("label")).text
            emit(f"{label} {match.group('url')}", match.start())
        elif kind == "entity":
            emit(html.unescape(match.group()), match.start())
        elif kind == "space":
            emit(" ", match.start())
        position = match.end()
    for offset in range(position, len(raw)):
        emit(raw[offset], offset)
    while chars and chars[0] == " ":
        chars.pop(0)
        lines.pop(0)
    while chars and chars[-1] == " ":
        chars.pop()
        lines.pop()
    return Normalized("".join(chars), lines)


# --- Version tokens ----------------------------------------------------------


@dataclass(frozen=True)
class Token:
    text: str
    start: int
    end: int
    number: str
    prerelease: str | None
    asset: bool

    def is_current(self, version: str) -> bool:
        if self.prerelease:
            return False
        if self.asset or self.number.count(".") == 2:
            return self.number == version
        return self.number == ".".join(version.split(".")[:2])


def tokens(text: str) -> list[Token]:
    """Every Clorraine-style version token in normalized text."""
    found = [
        Token(m.group(), m.start(), m.end(), m.group("num"), m.group("pre"), False)
        for m in V_TOKEN.finditer(text)
    ]
    found += [
        Token(m.group(), m.start(), m.end(), m.group("num"), None, True)
        for m in ASSET_TOKEN.finditer(text)
    ]
    return sorted(found, key=lambda token: token.start)


def phrase_spans(text: str, phrase: str) -> list[tuple[int, int]]:
    """Every occurrence of a normalized phrase in normalized text."""
    needle = normalize(phrase).text
    spans = []
    start = text.find(needle)
    while start >= 0:
        spans.append((start, start + len(needle)))
        start = text.find(needle, start + 1)
    return spans


def context(text: str, token: Token, width: int = 40) -> str:
    """The normalized text around a token, trimmed to whole words."""
    left = max(0, token.start - width)
    right = min(len(text), token.end + width)
    snippet = text[left:right]
    if left > 0 and " " in snippet[: token.start - left]:
        snippet = snippet[snippet.index(" ") + 1:]
    if right < len(text) and " " in snippet[-(right - token.end):]:
        snippet = snippet[: snippet.rindex(" ")]
    return snippet.strip(" .,;:")


# --- Dates -------------------------------------------------------------------


def parse_day(day: str, month: str, year: str) -> dt.date | None:
    """A date from English parts such as `27`, `November`, `2026`."""
    number = MONTHS.get(month)
    if number is None:
        return None
    try:
        return dt.date(int(year), number, int(day))
    except ValueError:
        return None


def date_problem(date: dt.date, today: dt.date) -> str | None:
    """Why a release date is not acceptable today, or None."""
    age = (today - date).days
    if age < 0:
        return f"{date.isoformat()} is in the future"
    if age > DATE_WINDOW_DAYS:
        return f"{date.isoformat()} is {age} days old (at most {DATE_WINDOW_DAYS})"
    return None


# --- Review records ----------------------------------------------------------

SECTIONS = ("Before tag", "After deploy")
_ITEM = re.compile(r"^- \[(?P<mark>[ xX])\] (?P<text>.+?)\s*$")


def checklist(text: str) -> dict[str, list[tuple[bool, str]]]:
    """The checkbox items of each `## ` section, in order."""
    sections: dict[str, list[tuple[bool, str]]] = {}
    current: str | None = None
    for line in text.splitlines():
        if line.startswith("## "):
            current = line[3:].strip()
            sections.setdefault(current, [])
            continue
        match = _ITEM.match(line)
        if match and current is not None:
            sections[current].append((match.group("mark") != " ", match.group("text")))
    return sections


def review_findings(record_path: str, record: str | None, template: str, version: str,
                    sections: tuple[str, ...]) -> list[Finding]:
    """Problems with a release review record; `sections` must be fully ticked."""
    where = record_path
    if record is None:
        return [Finding(where, "review", f"the review record for v{version} is missing; copy the template")]
    found: list[Finding] = []
    wanted = checklist(template)
    have = checklist(record)
    first = next((line for line in record.splitlines() if line.strip()), "")
    if first.strip() != f"# Release review: v{version}":
        found.append(Finding(where, "review", f"the first line must be '# Release review: v{version}'"))
    fields = {}
    for line in record.splitlines():
        for name in ("Prepared by", "Reviewer", "Date"):
            if line.startswith(f"{name}:"):
                fields[name] = line.split(":", 1)[1].strip()
    if not fields.get("Prepared by"):
        found.append(Finding(where, "review", "'Prepared by:' is missing"))
    reviewer = fields.get("Reviewer", "")
    if not re.fullmatch(r"@[A-Za-z0-9](?:[A-Za-z0-9-]*[A-Za-z0-9])?", reviewer):
        found.append(Finding(where, "review", "'Reviewer:' must be the maintainer's @handle"))
    try:
        dt.date.fromisoformat(fields.get("Date", ""))
    except ValueError:
        found.append(Finding(where, "review", "'Date:' must be YYYY-MM-DD"))
    if EMAIL.search(record):
        found.append(Finding(where, "review", "the record must not contain an e-mail address"))
    for section in SECTIONS:
        expected = [text for _, text in wanted.get(section, [])]
        items = have.get(section)
        if items is None:
            found.append(Finding(where, "review", f"section '## {section}' is missing"))
            continue
        actual = [text for _, text in items]
        if actual != expected:
            missing = [text for text in expected if text not in actual]
            extra = [text for text in actual if text not in expected]
            detail = "; ".join(
                part for part in (
                    f"missing {missing}" if missing else "",
                    f"not in the template {extra}" if extra else "",
                    "items are out of order" if not missing and not extra else "",
                ) if part
            )
            found.append(Finding(where, "review", f"'{section}' items differ from the template: {detail}"))
        if section in sections:
            open_items = [text for ticked, text in items if not ticked]
            for text in open_items:
                found.append(Finding(where, "review", f"'{section}' item not ticked: {text}"))
    return found
