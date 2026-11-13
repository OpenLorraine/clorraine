import { readFileSync } from "node:fs";

import { defineConfig } from "vitepress";

/**
 * The canonical editor grammar is the single source for Clorraine syntax
 * highlighting. Reading it here keeps the site and the VS Code extension in
 * step instead of maintaining a second grammar.
 */
const clorraineGrammar = JSON.parse(
  readFileSync(
    new URL("../../editors/clorraine.tmLanguage.json", import.meta.url),
    "utf8",
  ),
) as Record<string, unknown>;

// Shiki derives the primary language id from the grammar name (`Clorraine`).
// The lowercase aliases are what fenced code blocks and editors use.
clorraineGrammar.aliases = ["clorraine", "clo"];

/**
 * Public documentation site for Clorraine.
 *
 * The site is deployed as a GitHub Pages project site, so every internal link
 * and public asset URL must include the `/clorraine/` base path. Change `base`
 * and the absolute metadata URLs together if a custom domain is adopted.
 */
export default defineConfig({
  title: "Clorraine",
  titleTemplate: ":title · Clorraine",
  description:
    "A safe systems language for the Amiga. Rust-style ownership, plain C99 output, native 68K.",
  lang: "en",
  base: "/clorraine/",
  cleanUrls: true,
  // `README.md` documents the site package; it is not a public page.
  srcExclude: ["README.md"],
  lastUpdated: true,
  sitemap: {
    hostname: "https://openlorraine.github.io/clorraine/",
  },
  transformHead({ pageData }) {
    // Clean URLs drop the `.md` suffix and the `index` segment. The canonical
    // and Open Graph URL must match the deployed path, not the source path.
    const relative = pageData.relativePath
      .replace(/(^|\/)index\.md$/, "")
      .replace(/\.md$/, "");
    const pageUrl = `https://openlorraine.github.io/clorraine/${relative}`;
    return [
      ["link", { rel: "canonical", href: pageUrl }],
      ["meta", { property: "og:url", content: pageUrl }],
    ];
  },
  head: [
    ["link", { rel: "icon", href: "/clorraine/favicon.svg", type: "image/svg+xml" }],
    ["meta", { name: "theme-color", content: "#0b0f14" }],
    ["meta", { name: "author", content: "OpenLorraine" }],
    ["meta", { property: "og:type", content: "website" }],
    ["meta", { property: "og:site_name", content: "Clorraine" }],
    [
      "meta",
      {
        property: "og:title",
        content: "Clorraine — a safe systems language for the Amiga",
      },
    ],
    [
      "meta",
      {
        property: "og:description",
        content:
          "Rust-style ownership and modern tooling for 68K Amiga programming, with plain C99 output for the toolchains and emulators you already use.",
      },
    ],
    [
      "meta",
      {
        property: "og:image",
        content: "https://openlorraine.github.io/clorraine/og.png",
      },
    ],
    ["meta", { property: "og:image:width", content: "1200" }],
    ["meta", { property: "og:image:height", content: "630" }],
    [
      "meta",
      {
        property: "og:image:alt",
        content:
          "Clorraine wordmark on a dark panel with the tagline and a Hello, Amiga code sample.",
      },
    ],
    ["meta", { name: "twitter:card", content: "summary_large_image" }],
    [
      "meta",
      {
        name: "twitter:title",
        content: "Clorraine — a safe systems language for the Amiga",
      },
    ],
    [
      "meta",
      {
        name: "twitter:description",
        content:
          "Rust-style ownership, plain C99 output, native 68K. Developer preview.",
      },
    ],
    [
      "meta",
      {
        name: "twitter:image",
        content: "https://openlorraine.github.io/clorraine/og.png",
      },
    ],
  ],
  markdown: {
    lineNumbers: false,
    languages: [clorraineGrammar],
    theme: {
      light: "vitesse-light",
      dark: "vitesse-dark",
    },
  },
  themeConfig: {
    logo: "/logo.svg",
    siteTitle: "Clorraine",
    nav: [
      { text: "Guide", link: "/guide/quick-start", activeMatch: "/guide/" },
      { text: "Language", link: "/language/", activeMatch: "/language/" },
      { text: "Amiga", link: "/amiga/", activeMatch: "/amiga/" },
      { text: "Examples", link: "/examples/", activeMatch: "/examples/" },
      { text: "Project", link: "/project/brief", activeMatch: "/project/" },
      { text: "Reference", link: "/reference/cli", activeMatch: "/reference/" },
      {
        text: "GitHub",
        link: "https://github.com/OpenLorraine/clorraine",
      },
    ],
    sidebar: {
      "/guide/": [
        {
          text: "Start here",
          items: [
            { text: "Quick start", link: "/guide/quick-start" },
            { text: "Status", link: "/status" },
            { text: "Roadmap", link: "/guide/roadmap" },
            { text: "Known limitations", link: "/guide/known-limitations" },
          ],
        },
      ],
      "/language/": [
        {
          text: "Language tour",
          items: [
            { text: "Overview", link: "/language/" },
            { text: "Ownership", link: "/language/ownership" },
            { text: "Types and modules", link: "/language/types" },
            { text: "Generics", link: "/language/generics" },
            { text: "Lambdas and closures", link: "/language/lambdas" },
            { text: "Volatile and MMIO", link: "/language/mmio" },
            { text: "Profiles", link: "/language/profiles" },
            { text: "From C to Clorraine", link: "/language/from-c" },
          ],
        },
      ],
      "/amiga/": [
        {
          text: "Amiga",
          items: [
            { text: "Overview", link: "/amiga/" },
            { text: "Profiles", link: "/amiga/profiles" },
            { text: "SDK coverage", link: "/amiga/sdk-coverage" },
            { text: "Emulator workflow", link: "/amiga/emulator" },
            { text: "Screenshots", link: "/amiga/screenshots" },
          ],
        },
      ],
      "/examples/": [
        {
          text: "Examples",
          items: [
            { text: "Overview", link: "/examples/" },
            { text: "Hello, Amiga", link: "/examples/hello" },
            { text: "String parser", link: "/examples/string-parser" },
            { text: "Boing graphics", link: "/examples/graphics" },
            { text: "Intuition GUI", link: "/examples/gui" },
          ],
        },
      ],
      "/project/": [
        {
          text: "Project",
          items: [
            { text: "Project brief", link: "/project/brief" },
            { text: "Licensing and provenance", link: "/project/licensing" },
            { text: "Verification", link: "/project/verification" },
            { text: "Contributing", link: "/project/contributing" },
            { text: "About", link: "/project/about" },
          ],
        },
      ],
      "/reference/": [
        {
          text: "Reference",
          items: [
            { text: "Command line", link: "/reference/cli" },
            { text: "Diagnostics", link: "/reference/diagnostics" },
            { text: "Standard library", link: "/reference/stdlib" },
          ],
        },
      ],
    },
    search: {
      provider: "local",
    },
    outline: {
      level: [2, 3],
    },
    editLink: {
      pattern: "https://github.com/OpenLorraine/clorraine/edit/main/site/:path",
      text: "Edit this page",
    },
    lastUpdated: {
      text: "Last updated",
      formatOptions: { dateStyle: "medium" },
    },
    docFooter: {
      prev: "Previous",
      next: "Next",
    },
    socialLinks: [
      { icon: "github", link: "https://github.com/OpenLorraine/clorraine" },
    ],
    footer: {
      message:
        "Clorraine is an independent project. Amiga, AmigaOS, Kickstart, and Workbench are trademarks of Amiga Corporation. Not affiliated with or endorsed by Amiga Corporation or Hyperion Entertainment.",
      copyright: "Apache 2.0 · © 2026 OpenLorraine",
    },
  },
});
