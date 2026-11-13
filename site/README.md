# Clorraine documentation site

The public site is a VitePress package. It is separate from `docs/`, which holds
internal findings and review records. Do not point VitePress at `docs/`.

## Local development

From the repository root:

```sh
make site-dev
```

Or from this directory:

```sh
npm ci
npm run docs:dev
```

The dev server prints a local URL. The production build is:

```sh
make site-check
```

`site-check` performs a clean install and a production build. VitePress fails
the build on a dead internal link, so a green build is also a link check.

## Deployment

`.github/workflows/docs.yml` builds the site on every pull request that touches
it and deploys from `main` with GitHub Pages. The repository setting must use
**GitHub Actions** as the Pages source.

The production URL is <https://openlorraine.github.io/clorraine/>. The `base` in
`.vitepress/config.ts` is `/clorraine/`; change it only with a custom domain.

The README links to the site only after a deployment is verified at the
production URL.

## Content rules

- The current license is **Apache 2.0 with the Clorraine Output Exception**.
  v1.1.2 and earlier were published under MIT. Do not describe the current
  source as MIT.
- The compiler is a **developer preview**. Do not claim production readiness.
- Use only real acceptance captures. Do not present a mockup as an Amiga
  screenshot.
- Do not use Workbench or Kickstart screenshots as marketing images.
- Do not publish ROMs, Workbench media, the NDK archive, private email
  addresses, plans, or internal findings.
- Keep the trademark footer on every page; it is configured in
  `.vitepress/config.ts`.
- Test code samples before publishing. The complete runnable samples on the
  types, generics, lambdas, and MMIO pages were compiled and run with the
  driver; the ownership page's rejection is the documented compile-fail case.

## Assets

- Screenshots are derived from `docs/amiga-sdk/*.png` and the First Light
  Copperline capture; they live in `public/screenshots/`.
- The social preview is `public/og.png` (1200×630).
- The logo and favicon are `public/logo.svg` and `public/favicon.svg`.
- Inter and JetBrains Mono are served locally from
  `.vitepress/theme/fonts/`; their OFL texts are in `public/licenses/`.
- The canonical editor grammar in `editors/clorraine.tmLanguage.json` is loaded
  for syntax highlighting, so there is one grammar for the site and the VS Code
  extension.

## Live demo assets

The First Light player is a pinned, self-hosted Copperline 1.1.0 bundle:

| Path | Contents |
|---|---|
| `public/copperline/pkg/` | Copperline browser JavaScript and WebAssembly |
| `public/copperline/aros/` | AROS ROMs, license, acknowledgements |
| `public/demos/first-light.clstate` | Saved Copperline machine state |
| `public/demos/first-light.adf` | Bootable A500 ADF of the same build |

The state is tied to the bundle. If you update `public/copperline/`, regenerate
the state and the ADF from the same Copperline build; otherwise the player may
refuse the state or run a different demo. The provenance and regeneration steps
are in `public/copperline/NOTICE.md`. The ADF is built from
`examples/amiga/first-light.clo` with the `--release --profile hosted-amiga`
options.
