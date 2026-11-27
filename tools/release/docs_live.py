"""The live half of the release docs check.

After the site deploys, this module confirms that the deployed site is the
current commit, reads every page in the sitemap as rendered text and applies
the text rules to it, probes every project link (GitHub through the
maintainer's authenticated `gh`, Discord through its invite API, the
Marketplace through the versioned package URL), and reads the published
release body. Plain HTTPS requests go only to a fixed set of hosts, follow
redirects one checked hop at a time, and retry only transient failures; a
probe that never answers is reported as unreachable, never as a pass.
"""

from __future__ import annotations

import datetime as dt
import json
import re
import subprocess
import time
import urllib.error
import urllib.request
import xml.etree.ElementTree as ET
from dataclasses import dataclass, field
from html.parser import HTMLParser
from pathlib import Path
from typing import Callable
from urllib.parse import quote, urljoin, urlsplit

from docs_rules import Finding, Manifest, glob_regex, normalize, tokens

USER_AGENT = "clorraine-release-docs-check/1"
REQUEST_TIMEOUT = 20
RUN_BUDGET = 600
BODY_LIMIT = 5 * 1024 * 1024
RETRIES = 2
MAX_REDIRECTS = 5
MAX_RETRY_AFTER = 60
PLAIN_HOSTS = ("openlorraine.github.io", "discord.com", "marketplace.visualstudio.com")
CDN_SUFFIX = ".vsassets.io"
SITE_PATHS = ("site", "docs/amiga-sdk", "demo/poster.png", ".github/workflows/docs.yml")
OWNER_REPO = "OpenLorraine/clorraine"
GITHUB_PREFIX = f"https://github.com/{OWNER_REPO}"
ASSETS = ("clorraine-{version}-x86_64-linux.tar.gz", "clorraine-{version}-x86_64-windows.zip",
          "install.sh", "install.ps1")
RELEASE_URL = re.compile(r"https://github\.com/OpenLorraine/clorraine/releases/(?:download|tag)/[^\s\"'<>)`]+")

# Tests replace this so retries do not wait.
sleep = time.sleep


class Unreachable(Exception):
    """A request that did not get an answer worth judging."""


@dataclass
class Response:
    status: int
    headers: dict = field(default_factory=dict)
    body: bytes = b""


@dataclass
class Outcome:
    findings: list[Finding] = field(default_factory=list)
    unreachable: list[Finding] = field(default_factory=list)
    not_deployed: Finding | None = None


# --- Transport ---------------------------------------------------------------


class _NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, *args, **kwargs):  # noqa: D401 - urllib hook
        return None


_OPENER = urllib.request.build_opener(_NoRedirect())


def http_transport(url: str, read_body: bool = True) -> Response:
    """One HTTPS GET hop: no redirect is followed, no cookie or token is sent."""
    request = urllib.request.Request(url, headers={"User-Agent": USER_AGENT, "Accept": "*/*"})
    try:
        with _OPENER.open(request, timeout=REQUEST_TIMEOUT) as answer:
            body = answer.read(BODY_LIMIT + 1) if read_body else b""
            if len(body) > BODY_LIMIT:
                raise Unreachable(f"{url}: the answer is larger than {BODY_LIMIT} bytes")
            return Response(answer.status, dict(answer.headers), body)
    except urllib.error.HTTPError as error:
        return Response(error.code, dict(error.headers or {}), b"")


def gh_runner(args: list[str]) -> tuple[int, str, str]:
    """Runs `gh` with an argument list; it uses the maintainer's own login."""
    try:
        result = subprocess.run(["gh", *args], capture_output=True, text=True, timeout=REQUEST_TIMEOUT * 3)
    except FileNotFoundError:
        raise Unreachable("the GitHub CLI `gh` is not installed") from None
    except subprocess.TimeoutExpired:
        raise Unreachable(f"gh {' '.join(args)} timed out") from None
    return result.returncode, result.stdout, result.stderr


def _host_allowed(host: str) -> bool:
    return host in PLAIN_HOSTS or host.endswith(CDN_SUFFIX)


class Client:
    """HTTPS GETs under the network policy, within one run's time budget."""

    def __init__(self, transport: Callable) -> None:
        self.transport = transport
        self.deadline = time.monotonic() + RUN_BUDGET

    def get(self, url: str, read_body: bool = True) -> Response:
        last = "no answer"
        for attempt in range(RETRIES + 1):
            if time.monotonic() > self.deadline:
                raise Unreachable(f"{url}: the run's time budget is spent")
            try:
                answer = self._follow(url, read_body)
            except Unreachable:
                raise
            except (OSError, TimeoutError, urllib.error.URLError) as error:
                last = str(error) or type(error).__name__
            else:
                if answer.status == 429 or answer.status >= 500:
                    last = f"HTTP {answer.status}"
                    if attempt < RETRIES:
                        sleep(self._retry_after(answer, attempt))
                    continue
                return answer
            if attempt < RETRIES:
                sleep(2 ** (attempt + 1))
        raise Unreachable(f"{url}: {last} after {RETRIES + 1} attempts")

    def _follow(self, url: str, read_body: bool) -> Response:
        current = url
        for _ in range(MAX_REDIRECTS + 1):
            parts = urlsplit(current)
            if parts.scheme != "https" or not _host_allowed(parts.hostname or ""):
                raise Unreachable(f"{url}: refused a request to {parts.hostname or current} (not an allowed host)")
            answer = self.transport(current, read_body)
            if answer.status in (301, 302, 303, 307, 308):
                location = answer.headers.get("Location") or answer.headers.get("location")
                if not location:
                    return answer
                current = urljoin(current, location)
                continue
            return answer
        raise Unreachable(f"{url}: more than {MAX_REDIRECTS} redirects")

    @staticmethod
    def _retry_after(answer: Response, attempt: int) -> float:
        value = answer.headers.get("Retry-After") or answer.headers.get("retry-after")
        if value and str(value).isdigit():
            return min(int(value), MAX_RETRY_AFTER)
        return 2 ** (attempt + 1)


# --- Rendered text -----------------------------------------------------------

_INLINE = {"a", "abbr", "b", "code", "em", "i", "kbd", "mark", "small", "span", "strong", "sub", "sup", "time"}
_SKIP = {"script", "style", "template"}


class _PageText(HTMLParser):
    """Body text, the title, the descriptions, and every href of one page."""

    def __init__(self) -> None:
        super().__init__(convert_charrefs=True)
        self.parts: list[str] = []
        self.hrefs: list[str] = []
        self.skip = 0
        self.in_title = False

    def handle_starttag(self, tag: str, attrs) -> None:
        values = dict(attrs)
        if tag in _SKIP:
            self.skip += 1
            return
        if tag == "title":
            self.in_title = True
        if tag == "meta":
            name = values.get("name") or values.get("property")
            if name in ("description", "og:description") and values.get("content"):
                self.parts.append("\n" + values["content"] + "\n")
            return
        if tag == "a" and values.get("href"):
            self.hrefs.append(values["href"])
        if tag not in _INLINE:
            self.parts.append("\n")

    def handle_endtag(self, tag: str) -> None:
        if tag in _SKIP:
            self.skip = max(0, self.skip - 1)
            return
        if tag == "title":
            self.in_title = False
        if tag not in _INLINE:
            self.parts.append("\n")

    def handle_data(self, data: str) -> None:
        if not self.skip:
            self.parts.append(data)


def page_text(html: str) -> tuple[str, list[str]]:
    """The normalized rendered text of a page and its hrefs."""
    parser = _PageText()
    parser.feed(html)
    parser.close()
    return normalize("".join(parser.parts)).text, parser.hrefs


# --- The live check ----------------------------------------------------------


class Live:
    def __init__(self, repo: Path, manifest: Manifest, version: str, transport: Callable, gh: Callable) -> None:
        self.repo = repo
        self.manifest = manifest
        self.version = version
        self.minor = ".".join(version.split(".")[:2])
        self.client = Client(transport)
        self.gh = gh
        self.outcome = Outcome()
        self.site_url = manifest["site_url"]
        self._release: dict | None = None
        self._release_error: Finding | None = None

    def finding(self, where: str, rule: str, message: str) -> None:
        self.outcome.findings.append(Finding(where, rule, message))

    def unreachable(self, where: str, message: str) -> None:
        self.outcome.unreachable.append(Finding(where, "unreachable", message))

    def run(self) -> Outcome:
        pages = self.fresh_pages()
        if pages is None:
            return self.outcome
        links: set[str] = set()
        for path in pages:
            links |= self.check_page(path)
        for link in sorted(links):
            self.probe(link)
        self.check_release_body()
        return self.outcome

    # Freshness -----------------------------------------------------------

    def fresh_pages(self) -> list[str] | None:
        url = self.site_url + "sitemap.xml"
        try:
            answer = self.client.get(url)
        except Unreachable as error:
            self.unreachable(url, str(error))
            return None
        if answer.status != 200:
            self.unreachable(url, f"HTTP {answer.status}")
            return None
        try:
            root = ET.fromstring(answer.body)
        except ET.ParseError as error:
            self.finding(url, "sitemap", f"not valid XML: {error}")
            return None
        ns = {"s": "http://www.sitemaps.org/schemas/sitemap/0.9"}
        locations, stamps = [], []
        for entry in root.findall("s:url", ns):
            loc = (entry.findtext("s:loc", default="", namespaces=ns) or "").strip()
            stamp = (entry.findtext("s:lastmod", default="", namespaces=ns) or "").strip()
            if not loc.startswith(self.site_url):
                self.finding(url, "sitemap", f"{loc} is outside {self.site_url}")
                continue
            locations.append(loc[len(self.site_url):])
            if stamp:
                stamps.append(_instant(stamp))
        expected = self.commit_time()
        newest = max((stamp for stamp in stamps if stamp), default=None)
        tracked = self.tracked_pages()
        if newest != expected or set(locations) != tracked:
            detail = []
            if newest != expected:
                detail.append(f"its newest lastmod is {newest} but the last site commit is {expected}")
            if set(locations) != tracked:
                detail.append(f"pages differ (missing {sorted(tracked - set(locations))}, "
                              f"extra {sorted(set(locations) - tracked)})")
            self.outcome.not_deployed = Finding(url, "deploy-not-current", "the sitemap " + "; ".join(detail))
            return None
        return sorted(set(locations))

    def commit_time(self) -> dt.datetime | None:
        result = subprocess.run(["git", "-C", str(self.repo), "log", "-1", "--format=%cI", "--", *SITE_PATHS],
                                capture_output=True, text=True)
        return _instant(result.stdout.strip()) if result.returncode == 0 and result.stdout.strip() else None

    def tracked_pages(self) -> set[str]:
        excludes = [glob_regex(pattern) for pattern in self.manifest["scan"]["exclude"]]
        pages = set()
        for name in self.manifest.tracked:
            if not (name.startswith("site/") and name.endswith(".md")) or name.endswith("/README.md"):
                continue
            if any(rx.match(name) for rx in excludes):
                continue
            stem = name[len("site/"):-len(".md")]
            pages.add(stem[: -len("index")] if stem == "index" or stem.endswith("/index") else stem)
        return pages

    # Pages ---------------------------------------------------------------

    def sources(self, path: str) -> list[str]:
        base = "site/" + (path + "index" if path == "" or path.endswith("/") else path) + ".md"
        files = [base]
        for entry in self.manifest["theme_files"]:
            if entry["pages"] == "all" or (entry["pages"] == "home" and path == ""):
                files.append(entry["file"])
        return files

    def check_page(self, path: str) -> set[str]:
        url = self.site_url + path
        try:
            answer = self.client.get(f"{url}?release-docs-check={int(time.time())}")
        except Unreachable as error:
            self.unreachable(url, str(error))
            return set()
        if answer.status != 200:
            self.finding(url, "page", f"HTTP {answer.status}")
            return set()
        text, hrefs = page_text(answer.body.decode("utf-8", "replace"))
        where = urlsplit(url).path
        allowed = set()
        sources = self.sources(path)
        for entry in self.manifest["allowlist"]:
            if entry["file"] in sources:
                allowed |= {token.text for token in tokens(normalize(entry["phrase"]).text)}
        for token in tokens(text):
            if not token.is_current(self.version) and token.text not in allowed:
                expected = self.version if token.asset or token.number.count(".") == 2 else self.minor
                self.finding(where, "stale-version", f"{token.text} is not v{expected} on the deployed page")
        lowered = text.lower()
        for entry in self.manifest["banned"]:
            if normalize(entry["pattern"]).text.lower() in lowered:
                self.finding(where, "banned", f"'{entry['pattern']}': {entry['message']}")
        if path in ("", self.manifest["navigation"]["quick_start_url"].strip("/")):
            for platform in self.manifest["platforms"]:
                command = normalize(platform["command"].replace("{version}", self.version)).text
                if command not in text:
                    self.finding(where, "platform", f"the deployed page lacks the {platform['name']} install command")
        links = {urljoin(url, href) for href in hrefs}
        links |= set(RELEASE_URL.findall(text))
        return {link.split("#")[0] for link in links if link.startswith("https://") and "/edit/" not in link}

    # Probes --------------------------------------------------------------

    def probe(self, link: str) -> None:
        parts = urlsplit(link)
        host = parts.hostname or ""
        try:
            if link == GITHUB_PREFIX or link.startswith(GITHUB_PREFIX + "/"):
                self.probe_github(link)
            elif host in ("discord.gg", "discord.com") and (parts.path.startswith("/invite/") or host == "discord.gg"):
                code = parts.path.rsplit("/", 1)[-1]
                self.expect_ok(link, f"https://discord.com/api/v10/invites/{quote(code)}")
            elif host == "marketplace.visualstudio.com" and parts.path == "/items":
                self.expect_ok(link, link)
                self.expect_ok(link, "https://marketplace.visualstudio.com/_apis/public/gallery/publishers/openlorraine/"
                                     f"vsextensions/clorraine/{self.version}/vspackage", read_body=False)
        except Unreachable as error:
            self.unreachable(link, str(error))

    def expect_ok(self, link: str, url: str, read_body: bool = True) -> None:
        answer = self.client.get(url, read_body)
        if answer.status != 200:
            self.finding(link, "link", f"{url} answered HTTP {answer.status}")

    def probe_github(self, link: str) -> None:
        rest = link[len(GITHUB_PREFIX):].strip("/")
        segments = [segment for segment in rest.split("/") if segment]
        if segments[:2] == ["releases", "tag"] and len(segments) == 3:
            self.release(segments[2], link)
        elif segments[:2] == ["releases", "download"] and len(segments) == 4:
            release = self.release(segments[2], link)
            if release is not None and segments[3] not in {asset.get("name") for asset in release.get("assets", [])}:
                self.finding(link, "link", f"release {segments[2]} has no asset {segments[3]}")
        elif segments[:1] == ["releases"]:
            self.gh_ok(link, ["api", f"repos/{OWNER_REPO}/releases?per_page=1"])
        elif segments[:1] in (["blob"], ["tree"]) and len(segments) >= 2:
            path = "/".join(segments[2:])
            self.gh_ok(link, ["api", f"repos/{OWNER_REPO}/contents/{quote(path)}?ref={quote(segments[1])}"])
        elif segments[:1] in (["issues"], ["pull"]) and len(segments) == 2 and segments[1].isdigit():
            self.gh_ok(link, ["api", f"repos/{OWNER_REPO}/issues/{segments[1]}"])
        else:
            self.gh_ok(link, ["api", f"repos/{OWNER_REPO}"])

    def gh_call(self, args: list[str]) -> tuple[int, str, str]:
        code, out, err = self.gh(args)
        if code != 0 and "HTTP 404" not in err:
            raise Unreachable(f"gh {' '.join(args)} failed: {(err or out).strip()[:200]}")
        return code, out, err

    def gh_ok(self, link: str, args: list[str]) -> bool:
        code, _, err = self.gh_call(args)
        if code != 0:
            self.finding(link, "link", f"GitHub answered: {err.strip()[:120]}")
            return False
        return True

    def release(self, tag: str, link: str) -> dict | None:
        cache = getattr(self, "_releases", {})
        self._releases = cache
        if tag not in cache:
            code, out, err = self.gh_call(["api", f"repos/{OWNER_REPO}/releases/tags/{tag}"])
            cache[tag] = json.loads(out) if code == 0 else None
        if cache[tag] is None:
            self.finding(link, "link", f"there is no published release {tag}")
        return cache[tag]

    # Published release ---------------------------------------------------

    def check_release_body(self) -> None:
        tag = f"v{self.version}"
        where = f"{GITHUB_PREFIX}/releases/tag/{tag}"
        try:
            release = self.release(tag, where)
            if release is None:
                return
            code, sums, err = self.gh_call(["release", "download", tag, "--repo", OWNER_REPO,
                                            "--pattern", "SHA256SUMS", "--output", "-"])
        except Unreachable as error:
            self.unreachable(where, str(error))
            return
        body = release.get("body") or ""
        if "PENDING-" in body:
            self.finding(where, "release-body", "the published body still contains PENDING-")
        names = [name.replace("{version}", self.version) for name in ASSETS]
        for name in names:
            if name not in body:
                self.finding(where, "release-body", f"the published body does not name {name}")
        if code != 0:
            self.finding(where, "release-body", "the release has no SHA256SUMS asset")
            return
        expected = {}
        for line in sums.splitlines():
            parts = line.split()
            if len(parts) == 2:
                expected[parts[1].lstrip("*")] = parts[0].lower()
        for line in body.splitlines():
            if not line.lstrip().startswith("|"):
                continue
            name = next((candidate for candidate in names if candidate in line), None)
            digest = re.search(r"\b[0-9a-f]{64}\b", line)
            if name and digest and expected.get(name) != digest.group():
                self.finding(where, "release-body",
                             f"the hash for {name} in the body differs from SHA256SUMS")


def _instant(text: str) -> dt.datetime | None:
    try:
        value = dt.datetime.fromisoformat(text.replace("Z", "+00:00"))
    except ValueError:
        return None
    if value.tzinfo is None:
        value = value.replace(tzinfo=dt.timezone.utc)
    return value.astimezone(dt.timezone.utc).replace(microsecond=0)


def check(repo: Path, manifest: Manifest, version: str, fetcher: Callable | None = None,
          gh: Callable | None = None) -> Outcome:
    """Runs the live check; `fetcher` and `gh` default to the real network."""
    return Live(repo, manifest, version, fetcher or http_transport, gh or gh_runner).run()
