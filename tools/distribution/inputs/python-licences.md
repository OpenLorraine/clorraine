# Python source notices and packaging obligations

This review uses the exact source archives in the input catalog, not current
branch tips or package-name guesses. Downloaded lengths and SHA-256 values
were rechecked before reading archive members. Package metadata and notice
contents were inspected separately.

This records source declarations and the notice/source work required during
packaging. It does not replace the actual wheel-content audit, prove ABI
compatibility or authorise publication.

## Selected source declarations

| Source | Role | Observed terms and notice paths |
|---|---|---|
| machine68k 0.4.1 | Native vamos dependency | Package metadata declares GPL-2.0-or-later. `README.md` and wrapper source headers identify GPL version 2. Bundled `src/musashi/m68kcpu.c` and `.h` carry Karl Stenerud's MIT-style permission and warranty notice. Retain both the wrapper's GNU terms and Musashi notices. |
| greenlet 3.5.6 | Native vamos dependency | `LICENSE` gives MIT terms except the identified Stackless-derived platform files; `LICENSE.PSF` supplies their PSF version 2 terms. Retain both, not just MIT. |
| PySDL2 0.9.17 | Recorded SDK package; include only if selected by the measured runtime closure | `COPYING.txt` points to `doc/copying.rst`: CC0 public-domain dedication with a zlib fallback. That document also identifies separate font/documentation exceptions. Preserve the exceptions if those files are included. |
| pip 26.2.1 | Offline bootstrap wheel | Project `LICENSE.txt` is MIT; `src/pip/_vendor/` has additional licences, including certifi's Mozilla notice, distlib's Python-derived terms, Apache notices and BSD/MIT notices. The root MIT label does not replace these. |
| setuptools 77.0.3 | Build backend; not automatically a shipped runtime dependency | Root `LICENSE` is MIT. Preserve the actual `_vendor/*dist-info/` licence files, including packaging, typing-extensions and other bundled code, if conveying those files. |
| setuptools-scm 8.3.1 | Build backend | Root `LICENSE` contains MIT permission/warranty text. Retain it with any source or wheel that is conveyed. |
| Cython 3.0.12 | Build tool | `COPYING.txt` records the original Pyrex permission and Apache-2.0 licensing for Cython changes; `LICENSE.txt` contains Apache terms. `COPYING.txt` explicitly states that embedded generated-code snippets do not impose Cython's licence on generated output. Preserve this statement with the tool/source notices. |
| wheel 0.45.1 | Build backend | `LICENSE.txt` is MIT; `src/wheel/vendored/packaging/` retains packaging's dual licence files. |
| packaging 24.2 | Build dependency | Root `LICENSE` offers either `LICENSE.APACHE` or `LICENSE.BSD`; the latter is a two-clause BSD notice. Preserve the choice statement and both original texts rather than silently relabelling it. |
| flit-core 3.12.0 | Build backend | Root `LICENSE` is BSD-3-Clause. Bundled `flit_core/vendor/tomli-1.2.3.dist-info/LICENSE` is a separate MIT notice. Test-fixture licence-shaped files are not evidence for the runtime's licence. |
| tomli 2.0.2 | Python 3.10 build dependency | Root `LICENSE` contains MIT terms. This is the selected standalone source, distinct from vendored tomli copies within other packages. |

The collection found 66 named licence/copyright candidates across these
archives. That number includes vendored files and some source/test documents;
it is not a claim that 66 files are all required in every runtime archive.
Machine68k also has inline source/header notices despite having no separately
named root licence file in this source distribution.

The wheel recipe now preserves machine68k's `README.md`, `src/cpu.c`,
`src/musashi/m68kcpu.c` and `src/musashi/m68kcpu.h` as original notice-bearing
files, plus PySDL2's `doc/copying.rst`. Amitools likewise declares its GNU terms
in `README.md` rather than a root licence-named file. Original archives/raw
sources remain alongside those notices. Their bytes were checked in both
manual ABI outputs; complete conveyed-content compliance remains a later gate.

## Machine68k source completeness

Do not treat an absent `LICENSE` filename as absence of obligations. Its
GPL-labelled Python/native wrapper and the bundled Musashi implementation
must retain their actual notices. Include the matching source and required
build information for the distributed native extension.

The inspected Musashi headers grant copying, modification, distribution and
sale subject to preserving the copyright/permission notice and disclaimer.
That grant does not remove the separate wrapper's GNU obligations.
Copying only a generic GPL text would omit the Musashi notice.

## Runtime and build-only boundaries

Determine the runtime set from the built amitools/vamos metadata and actual
dependency closure. An old SDK's installed package list is not, by itself,
proof that every package belongs in the native runtime payload.
Keep selected versions pinned; do not replace or upgrade them during this
classification.

Build-backend packages are installed before the offline wheel build.
Their use does not automatically make every backend, development extra or
test resource a user runtime dependency. Conversely, any backend/source files
that are conveyed still need their own notices.

The runtime wheels deliberately omit only `greenlet/tests/`. Upstream includes
native test extensions there and declares psutil/objgraph/setuptools as optional
test dependencies; they are not required by vamos. Runtime extension/header,
metadata and licence member bytes remain unchanged. Wheel RECORD is regenerated
after selection. The full original Greenlet source archive and both licences
remain conveyed, including the omitted tests' source. This selection does not
authorise changes to other wheel subtrees or weaken native-runtime checks.

Ubuntu's venv bootstrap wheels and the CPython source-bundled ensurepip wheel
remain separate recorded inputs. Do not describe them as the selected
pip 26.2.1 output or infer their notices from another pip version.

## Required packaging checks

1. Match every wheel to its recorded source identity, version, ABI and hash.
2. Preserve the applicable root notices and notices for actual vendored files.
   Do not rely solely on the package's top-level metadata licence label.
3. Include the matching source/build information required for the conveyed
   GNU-licensed components. Keep original inline notices in that source.
4. Compare actual wheel contents and external notice/source files with the
   file-based component audit. A missing notice must fail packaging.
5. Preserve original source licences in any corresponding-source bundle.
   Do not claim that Clorraine's Apache licence or Output Exception relicenses
   these packages or their bundled third-party files.

## Selected notice identities

Paths below omit each archive's top directory. Hashes identify inspected
bytes, not a replacement licence text or publication approval.

| Source / path | SHA-256 |
|---|---|
| machine68k `README.md` | `87d30a98f15da807470a361c9c8a4717a16c8d6625f950a13e0c5cd201a6bec5` |
| machine68k `src/cpu.c` | `de135392ef971171abba10faf921d4f2b687ecb829ca9454f6e5eb5d6307e58a` |
| machine68k `src/musashi/m68kcpu.c` | `7f339c4c883572d1ea68632d716d5e1a899a6d658f04d44545f4fbd5441fddcf` |
| greenlet `LICENSE` | `769831d6e5dfaf2c20802faccff1fafb4c2025dd8f6253dfa47fcad59d4d0979` |
| greenlet `LICENSE.PSF` | `e5ff3c23c110e494cd7d736c10fd96d462457bafeca310840db6527298c7d46b` |
| PySDL2 `COPYING.txt` | `0ef0582440f89e3c3fba5db67f3492bc91a6702c0248cc937b547be08027c80d` |
| PySDL2 `doc/copying.rst` | `91f56c44f50f234c4ab4984be04e1345779d550c19b1bba349f465c41b19c20c` |
| pip `LICENSE.txt` | `634300a669d49aeae65b12c6c48c924c51a4cdf3d1ff086dc3456dc8bcaa2104` |
| pip `src/pip/_vendor/certifi/LICENSE` | `e93716da6b9c0d5a4a1df60fe695b370f0695603d21f6f83f053e42cfc10caf7` |
| setuptools `LICENSE` | `86da0f01aeae46348a3c3d465195dc1ceccde79f79e87769a64b8da04b2a4741` |
| setuptools-scm `LICENSE` | `89807acf2309bd285f033404ee78581602f3cd9b819a16ac2f0e5f60ff4a473e` |
| Cython `COPYING.txt` | `e1eb1c49a8508e8173dac30157e4a6439a44ad8846194746c424fbc3fc2b95d7` |
| Cython `LICENSE.txt` | `9568a2b155e66ac3e0ba1fd80b52b827b9460e6cf6f233125e7cbca8e206ddc3` |
| wheel `LICENSE.txt` | `30c23618679108f3e8ea1d2a658c7ca417bdfc891c98ef1a89fa4ff0c9828654` |
| packaging `LICENSE` | `cad1ef5bd340d73e074ba614d26f7deaca5c7940c3d8c34852e65c4909686c48` |
| packaging `LICENSE.APACHE` | `0d542e0c8804e39aa7f37eb00da5a762149dc682d7829451287e11b938e94594` |
| packaging `LICENSE.BSD` | `b70e7e9b742f1cc6f948b34c16aa39ffece94196364bc88ff0d2180f0028fac5` |
| flit-core `LICENSE` | `35b4f1dec512e617077fd6980dbb43ef8c2887adc5d0185edb4c04da175dd816` |
| flit-core `flit_core/vendor/tomli-1.2.3.dist-info/LICENSE` | `b80816b0d530b8accb4c2211783790984a6e3b61922c2b5ee92f3372ab2742fe` |
| tomli `LICENSE` | `b80816b0d530b8accb4c2211783790984a6e3b61922c2b5ee92f3372ab2742fe` |
