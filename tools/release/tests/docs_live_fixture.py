"""A fake deployed site and fake `gh` answers for the live docs-check tests.

`LiveSite` renders the fixture release's pages as VitePress-like HTML,
answers sitemap, Discord, Marketplace and redirect requests one hop at a time
(the checker follows redirects itself), and answers the `gh` commands the
checker runs. Tests change one answer and check the outcome.
"""

from __future__ import annotations

import json
from dataclasses import dataclass, field
from urllib.parse import urlsplit, urlunsplit

from docs_fixture import LINUX, SITE_URL, VERSION, WINDOWS

import docs_live

HASHES = {
    f"clorraine-{VERSION}-x86_64-linux.tar.gz": "a" * 64,
    f"clorraine-{VERSION}-x86_64-windows.zip": "b" * 64,
    "install.sh": "c" * 64,
    "install.ps1": "d" * 64,
}
DISCORD = "https://discord.gg/7ckPjharmD"
MARKETPLACE = "https://marketplace.visualstudio.com/items?itemName=openlorraine.clorraine"
PACKAGE = (f"https://marketplace.visualstudio.com/_apis/public/gallery/publishers/openlorraine/"
           f"vsextensions/clorraine/{VERSION}/vspackage")
REPO = "https://github.com/OpenLorraine/clorraine"
DEPLOYMENTS = "repos/OpenLorraine/clorraine/deployments?environment=github-pages&per_page=1"
STATUSES = "repos/OpenLorraine/clorraine/deployments/7/statuses?per_page=1"


def api(endpoint: str) -> tuple:
    """The argument tuple of a `gh api` call, pinned to github.com."""
    return ("api", "--hostname", "github.com", endpoint)


def page(title: str, body: str) -> str:
    """A rendered page with the theme's preview bar and VitePress head tags."""
    return f"""<!DOCTYPE html><html><head><title>{title} | Clorraine</title>
<meta name="generator" content="VitePress v1.6.4">
<meta name="description" content="{title} for Clorraine v1.3">
<script>window.__VP_HASH_MAP__ = {{"v0.0.1": 1}}</script><style>.a{{}}</style></head>
<body><div class="preview"><span>v{VERSION} for Linux and Windows</span></div>
<main>{body}</main>
<a href="{REPO}/edit/main/site/x.md">Edit this page</a></body></html>"""


def commands() -> str:
    linux = LINUX.replace("{v}", VERSION).replace("&&", "&amp;&amp;")
    windows = WINDOWS.replace("{v}", VERSION).replace('"', "&quot;")
    return f'<pre><code><span class="line"><span>{linux}</span></span>\n<span>{windows}</span></code></pre>'


def release_body(hashes: dict[str, str] | None = None) -> str:
    hashes = hashes or HASHES
    rows = "\n".join(f"| `{name}` | 1 | `{digest}` |" for name, digest in hashes.items())
    return f"# Clorraine v{VERSION} developer preview\n\n## Assets\n\n| Asset | Bytes | SHA-256 |\n|---|---:|---|\n{rows}\n"


@dataclass
class Response:
    status: int
    headers: dict = field(default_factory=dict)
    body: bytes = b""


@dataclass
class LiveSite:
    """Answers for every request and `gh` call of a clean live run."""

    pages: dict[str, str] = field(default_factory=dict)
    http: dict[str, object] = field(default_factory=dict)
    gh_answers: dict[tuple, tuple[int, str, str]] = field(default_factory=dict)
    requests: list[str] = field(default_factory=list)
    gh_calls: list[tuple] = field(default_factory=list)
    head: str = "0" * 40
    extra_sitemap: list[str] = field(default_factory=list)

    def __post_init__(self) -> None:
        self.pages = {
            "": page("Home", f"<h1>Clorraine</h1>{commands()}"
                             f'<a href="{REPO}/releases/tag/v{VERSION}">Release</a>'
                             f'<a href="{DISCORD}">Discord</a><a href="{MARKETPLACE}">Extension</a>'),
            "status": page("Status", "<p><strong>v1.3.0</strong> is published as a developer preview. "
                                     "The latest stable release is unchanged: v1.1.2 remains the latest stable release.</p>"
                                     f'<a href="{REPO}/blob/main/docs/known-limitations.md">limits</a>'),
            "project/brief": page("Brief", "<p>Developer preview · v1.3.0 · Updated 27 November 2026</p>"),
            "guide/quick-start": page("Quick start", f'<h2 id="install-on-linux">Install on Linux<a href="#install-on-linux">​</a></h2>'
                                                     f"{commands()}<h2>Install on Windows</h2>"),
        }
        for path, html in self.pages.items():
            self.http[SITE_URL + path] = Response(200, {"content-type": "text/html"}, html.encode())
        self.http[SITE_URL + "sitemap.xml"] = None  # rendered on demand from pages and lastmod
        self.http["https://discord.gg/7ckPjharmD"] = Response(301, {"Location": "https://discord.com/invite/7ckPjharmD"})
        self.http["https://discord.com/api/v10/invites/7ckPjharmD"] = Response(200, {}, b'{"code": "7ckPjharmD"}')
        self.http[MARKETPLACE] = Response(200, {}, b"<html>Clorraine</html>")
        self.http[PACKAGE] = Response(302, {"Location": "https://openlorraine.gallerycdn.vsassets.io/x/clorraine.vsix"})
        self.http["https://openlorraine.gallerycdn.vsassets.io/x/clorraine.vsix"] = Response(200, {}, b"PK")
        release = {"tag_name": f"v{VERSION}", "prerelease": True, "body": release_body(),
                   "assets": [{"name": name} for name in [*HASHES, "SHA256SUMS"]]}
        sums = "".join(f"{digest}  {name}\n" for name, digest in HASHES.items())
        self.gh_answers = {
            api("repos/OpenLorraine/clorraine"): (0, "{}", ""),
            api(f"repos/OpenLorraine/clorraine/releases/tags/v{VERSION}"): (0, json.dumps(release), ""),
            api("repos/OpenLorraine/clorraine/contents/docs/known-limitations.md?ref=main"): (0, "{}", ""),
            ("release", "download", f"v{VERSION}", "--repo", "github.com/OpenLorraine/clorraine", "--pattern",
             "SHA256SUMS", "--output", "-"): (0, sums, ""),
        }
        self.deploy(self.head, "success")

    def deploy(self, sha: str, state: str) -> None:
        """The newest GitHub Pages deployment and its latest status."""
        self.gh_answers[api(DEPLOYMENTS)] = (0, json.dumps([{"id": 7, "sha": sha}]), "")
        self.gh_answers[api(STATUSES)] = (0, json.dumps([{"state": state}]), "")

    def sitemap(self) -> bytes:
        locations = [SITE_URL + path for path in self.pages] + self.extra_sitemap
        urls = "".join(f"<url><loc>{loc}</loc><lastmod>2026-11-27T11:00:00.000Z</lastmod></url>" for loc in locations)
        return f'<?xml version="1.0"?><urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">{urls}</urlset>'.encode()

    def set_release(self, **changes) -> None:
        key = api(f"repos/OpenLorraine/clorraine/releases/tags/v{VERSION}")
        release = json.loads(self.gh_answers[key][1])
        release.update(changes)
        self.gh_answers[key] = (0, json.dumps(release), "")

    def transport(self, url: str, read_body: bool = True) -> Response:
        """One hop of an HTTPS GET; the cache-busting query is ignored."""
        parts = urlsplit(url)
        query = "&".join(q for q in parts.query.split("&") if q and not q.startswith("release-docs-check="))
        key = urlunsplit((parts.scheme, parts.netloc, parts.path, query, ""))
        self.requests.append(url)
        if key == SITE_URL + "sitemap.xml":
            return Response(200, {"content-type": "application/xml"}, self.sitemap())
        answer = self.http.get(key, Response(404))
        if isinstance(answer, Exception):
            raise answer
        if callable(answer):
            return answer()
        return answer

    def gh(self, args: list[str]) -> tuple[int, str, str]:
        key = tuple(args)
        self.gh_calls.append(key)
        return self.gh_answers.get(key, (1, "", "gh: Not Found (HTTP 404)"))


def no_sleep(_seconds: float) -> None:
    """Retries in tests do not wait."""


docs_live.sleep = no_sleep
