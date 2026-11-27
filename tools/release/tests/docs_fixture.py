"""A small git repository holding a clean release, for the docs-check tests.

`Fixture` writes the release-facing files of a fictional 1.3.0 release, a
manifest that covers them, and a fully ticked review record, then commits
them. A test changes one file (or the manifest) and runs the checker through
`run()`, which calls `docs_check.main` with an injected clock, fetcher and
`gh` runner so no case touches the network.
"""

from __future__ import annotations

import datetime as dt
import io
import json
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))

import docs_check  # noqa: E402

TODAY = dt.date(2026, 11, 27)
VERSION = "1.3.0"
LINUX = "curl -fsSL https://github.com/OpenLorraine/clorraine/releases/download/v{v}/install.sh -o clorraine-install.sh && sh clorraine-install.sh"
WINDOWS = 'powershell -ExecutionPolicy ByPass -c "irm https://github.com/OpenLorraine/clorraine/releases/download/v{v}/install.ps1 | iex"'

TEMPLATE = """# Release review template

Copy this file to `vX.Y.Z.md`. An agent may prepare it; the maintainer confirms
before any box is ticked.

## Before tag

- [ ] Status page matches this release
- [ ] Project brief matches this release

## After deploy

- [ ] The live check passed
- [ ] The published release body was read
"""

SITE_URL = "https://openlorraine.github.io/clorraine/"


def manifest() -> dict:
    """The manifest for the fixture tree."""
    return {
        "site_url": SITE_URL,
        "scan": {
            "files": [
                "README.md", "RELEASE-NOTES.md", "CHANGELOG.md", "docs/README.md",
                "docs/linux-cli.md", "docs/windows-cli.md", "tools/vscode/CHANGELOG.md",
            ],
            "globs": ["site/**/*.md", "site/**/*.vue", "site/**/*.ts"],
            "exclude": ["site/public/**", "site/.vitepress/dist/**", "site/.vitepress/cache/**"],
        },
        "excluded": [{"path": "docs/findings.md", "reason": "historical design notes"}],
        "platforms": [
            {
                "name": "Linux",
                "command": LINUX.replace("{v}", "{version}"),
                "guide": "docs/linux-cli.md",
                "sidebar_text": "Install on Linux",
                "anchor": "install-on-linux",
                "heading": "## Install on Linux",
            },
            {
                "name": "Windows",
                "command": WINDOWS.replace("{v}", "{version}"),
                "guide": "docs/windows-cli.md",
                "sidebar_text": "Install on Windows",
                "anchor": "install-on-windows",
                "heading": "## Install on Windows",
            },
        ],
        "install_surfaces": [
            "README.md", "RELEASE-NOTES.md", "site/guide/quick-start.md",
            "site/.vitepress/theme/components/HomePage.vue",
        ],
        "navigation": {
            "sidebar": "site/.vitepress/config.ts",
            "quick_start": "site/guide/quick-start.md",
            "quick_start_url": "/guide/quick-start",
        },
        "required": [
            {"file": "site/status.md", "pattern": "v{version} is published", "why": "release state"},
            {"file": "site/project/brief.md", "pattern": "Developer preview · v{version}", "why": "brief header"},
            {"file": "site/.vitepress/theme/Layout.vue", "pattern": "v{version} for", "why": "preview bar"},
            {"file": "docs/README.md", "pattern": "v{minor} limitations", "why": "limitations index"},
        ],
        "banned": [
            {"pattern": "next release", "message": "name the release instead"},
            {"pattern": "vX.Y.Z", "message": "placeholder version"},
            {"pattern": "not in a marketplace", "message": "the extension is on the Marketplace"},
            {"pattern": "PENDING-", "message": "unfinished release text"},
        ],
        "placeholder": {"file": "RELEASE-NOTES.md", "line_prefix": "PENDING-CHECKSUMS:"},
        "dated": [
            {
                "file": "site/project/brief.md",
                "pattern": r"Updated (?P<day>\d{1,2}) (?P<month>[A-Z][a-z]+) (?P<year>\d{4})",
            }
        ],
        "changelogs": [
            {
                "file": "CHANGELOG.md",
                "heading": r"^## \[(?P<version>\d+\.\d+\.\d+)\](?: - (?P<date>\d{4}-\d{2}-\d{2}))?\s*$",
                "dated": True,
            },
            {"file": "tools/vscode/CHANGELOG.md", "heading": r"^## (?P<version>\d+\.\d+\.\d+)\s*$", "dated": False},
        ],
        "theme_files": [
            {"file": "site/.vitepress/theme/Layout.vue", "pages": "all"},
            {"file": "site/.vitepress/config.ts", "pages": "all"},
            {"file": "site/.vitepress/theme/components/HomePage.vue", "pages": "home"},
        ],
        "review": {"template": "docs/release-reviews/TEMPLATE.md", "dir": "docs/release-reviews"},
        "allowlist": [
            {
                "file": "site/status.md",
                "phrase": "v1.1.2 remains the latest stable release",
                "reason": "the last stable release is still v1.1.2",
                "reviewed_for": VERSION,
            }
        ],
    }


def record(before: bool = True, after: bool = True, reviewer: str = "@maintainer") -> str:
    """A review record for the fixture release."""
    b = "x" if before else " "
    a = "x" if after else " "
    return f"""# Release review: v{VERSION}

Prepared by: agent
Reviewer: {reviewer}
Date: {TODAY.isoformat()}

## Before tag

- [{b}] Status page matches this release
- [{b}] Project brief matches this release
  - checked the status table rows

## After deploy

- [{a}] The live check passed
- [{a}] The published release body was read
"""


def files(version: str = VERSION) -> dict[str, str]:
    """The release-facing files of the fixture tree."""
    linux, windows = LINUX.replace("{v}", version), WINDOWS.replace("{v}", version)
    minor = ".".join(version.split(".")[:2])
    day = "27 November 2026"
    return {
        "compiler/Cargo.toml": f'[package]\nname = "clc"\nversion = "{version}"\n',
        "tools/vscode/package.json": json.dumps({"name": "clorraine", "version": version}) + "\n",
        "CHANGELOG.md": (
            f"# Changelog\n\n## [Unreleased]\n\nNo changes since v{version}.\n\n"
            f"## [{version}] - {TODAY.isoformat()}\n\n- Windows installer.\n\n"
            "## [1.2.0] - 2026-11-24\n\n- The v1.2.0 Linux installer replaced v1.1.2.\n"
        ),
        "tools/vscode/CHANGELOG.md": f"# Changelog\n\n## {version}\n\n- Marketplace.\n\n## 1.2.0\n\n- First v1.2.0 build.\n",
        "RELEASE-NOTES.md": (
            f"# Clorraine v{version} developer preview\n\n```sh\n{linux}\n```\n\n```powershell\n{windows}\n```\n\n"
            "## Assets\n\nPENDING-CHECKSUMS: the release steps replace this line with the asset table.\n"
        ),
        "README.md": f"# Clorraine\n\n> Developer preview v{version}.\n\n```sh\n{linux}\n```\n\n```powershell\n{windows}\n```\n",
        "docs/README.md": f"# Docs\n\n- [Known limitations](known-limitations.md): v{minor} limitations\n- [Release reviews](release-reviews/)\n",
        "docs/linux-cli.md": f"# Linux CLI\n\n```sh\n{linux}\n```\n",
        "docs/windows-cli.md": f"# Windows CLI\n\n```powershell\n{windows}\n```\n",
        "docs/findings.md": "# Old notes\n\nWritten for v0.9.0.\n",
        "site/status.md": (
            f"# Status\n\n**v{version}** is published as a developer preview.\n"
            "The latest stable release is unchanged: v1.1.2 remains the latest stable release.\n"
            "FS-UAE 3.2.35 is the tested emulator.\n"
        ),
        "site/project/brief.md": f"# Brief\n\n**Developer preview · v{version} · Updated {day}**\n",
        "site/guide/quick-start.md": (
            f"# Quick start\n\n## Install on Linux\n\n```sh\n{linux}\n```\n\n"
            f"## Install on Windows\n\n```powershell\n{windows}\n```\n"
        ),
        "site/index.md": "---\nlayout: home\n---\n",
        "site/.vitepress/config.ts": (
            "export default {\n  themeConfig: {\n    sidebar: [\n"
            '      { text: "Quick start", link: "/guide/quick-start" },\n'
            '      { text: "Install on Linux", link: "/guide/quick-start#install-on-linux" },\n'
            '      { text: "Install on Windows", link: "/guide/quick-start#install-on-windows" },\n'
            "    ],\n  },\n};\n"
        ),
        "site/.vitepress/theme/Layout.vue": f"<template><span>v{version} for Linux and Windows</span></template>\n",
        "site/.vitepress/theme/components/HomePage.vue": (
            f"<template><pre><code>{linux.replace('&&', '&amp;&amp;')}\n{windows}</code></pre></template>\n"
        ),
        "site/public/copperline/NOTICE.md": "Copperline v1.1.0, a third-party build.\n",
        "docs/release-reviews/TEMPLATE.md": TEMPLATE,
        f"docs/release-reviews/v{version}.md": record(),
    }


class Fixture:
    """A temporary git repository holding the clean fixture release."""

    def __init__(self) -> None:
        self.root = Path(tempfile.mkdtemp(prefix="docs-check-", dir=_scratch()))
        self.manifest = manifest()
        for path, text in files().items():
            self.write(path, text, commit=False)
        self._git("init", "-q")
        self._git("config", "user.email", "fixture@example.invalid")
        self._git("config", "user.name", "Fixture")
        self.save_manifest(commit=False)
        self.commit("fixture")

    def write(self, path: str, text: str, commit: bool = True) -> None:
        target = self.root / path
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(text, encoding="utf-8")
        if commit:
            self.commit(f"edit {path}")

    def read(self, path: str) -> str:
        return (self.root / path).read_text(encoding="utf-8")

    def replace(self, path: str, old: str, new: str) -> None:
        text = self.read(path)
        assert old in text, f"{old!r} not in {path}"
        self.write(path, text.replace(old, new))

    def save_manifest(self, commit: bool = True) -> None:
        self.write("tools/release/docs-manifest.json", json.dumps(self.manifest, indent=2) + "\n", commit=commit)

    def commit(self, message: str) -> None:
        self._git("add", "-A")
        self._git("commit", "-q", "--allow-empty", "-m", message, "--date", "2026-11-27T12:00:00+01:00",
                  env_date="2026-11-27T12:00:00+01:00")

    def run(self, *args: str, today: dt.date = TODAY, fetcher=None, gh=None) -> tuple[int, str]:
        out = io.StringIO()
        argv = ["--repo", str(self.root), "--today", today.isoformat(), *args]
        if "--version" not in args:
            argv += ["--version", VERSION]
        code = docs_check.main(argv, stdout=out, stderr=out, fetcher=fetcher, gh=gh)
        return code, out.getvalue()

    def cleanup(self) -> None:
        shutil.rmtree(self.root, ignore_errors=True)

    def _git(self, *args: str, env_date: str | None = None) -> None:
        env = None
        if env_date:
            import os
            env = dict(os.environ, GIT_COMMITTER_DATE=env_date, GIT_AUTHOR_DATE=env_date)
        subprocess.run(["git", *args], cwd=self.root, check=True, capture_output=True, env=env)


def _scratch() -> str | None:
    """Scratch space under build/tmp when run from the repository."""
    repo = HERE.parents[2]
    scratch = repo / "build" / "tmp"
    if (repo / ".git").exists():
        scratch.mkdir(parents=True, exist_ok=True)
        return str(scratch)
    return None
