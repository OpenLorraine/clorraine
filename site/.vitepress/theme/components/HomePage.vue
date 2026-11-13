<script setup lang="ts">
import { withBase } from "vitepress";
</script>

<template>
  <div class="cl-home">
    <section class="cl-hero">
      <div class="cl-hero__glow" aria-hidden="true"></div>
      <div class="cl-wrap cl-hero__grid">
        <div class="cl-hero__copy">
          <p class="cl-kicker">
            <span class="cl-kicker__dot" aria-hidden="true"></span>
            Developer preview · v1.1.2
          </p>
          <h1>
            A safe systems language
            <span class="cl-hero__accent">for the Amiga.</span>
          </h1>
          <p class="cl-hero__lead">
            Clorraine brings Rust-style ownership and modern tooling to 68K
            Amiga programming. It emits plain C99, so it works with the
            toolchains and emulators you already use.
          </p>
          <div class="cl-actions">
            <a class="cl-btn cl-btn--primary" :href="withBase('/language/')">
              Explore the language
              <span aria-hidden="true">→</span>
            </a>
            <a
              class="cl-btn cl-btn--ghost"
              :href="withBase('/guide/quick-start')"
            >
              Quick start
            </a>
          </div>
          <p class="cl-hero__note">
            Apache 2.0 · No ROMs or Workbench media · Linux x86_64 release
            today; installers in progress.
          </p>
        </div>

        <div class="cl-terminal" aria-label="Hello world source and output">
          <div class="cl-terminal__bar">
            <span class="cl-dots" aria-hidden="true">
              <i></i><i></i><i></i>
            </span>
            <span class="cl-terminal__file">hello.clo</span>
            <span class="cl-terminal__tag">host</span>
          </div>
          <pre class="cl-code"><code><span class="tok-kw">module</span> demo.Hello;

<span class="tok-kw">import</span> { println } <span class="tok-kw">from</span> std.io;

<span class="tok-kw">func</span> <span class="tok-fn">main</span>(): <span class="tok-type">i32</span> {
    <span class="tok-kw">let</span> who: <span class="tok-type">str</span> = <span class="tok-str">"Amiga"</span>;
    println(<span class="tok-str">$"Hello, {who}, from Clorraine!"</span>);
    <span class="tok-kw">return</span> <span class="tok-num">0</span>;
}</code></pre>
          <div class="cl-terminal__output">
            <div class="cl-terminal__cmd">
              <span class="cl-prompt" aria-hidden="true">$</span>
              clorraine run hello.clo
            </div>
            <div class="cl-terminal__result">
              Hello, Amiga, from Clorraine!
            </div>
          </div>
          <div class="cl-terminal__footer">
            <span>One C99 translation unit</span>
            <span class="cl-terminal__ok">exit 0</span>
          </div>
        </div>
      </div>
    </section>

    <section class="cl-stats" aria-label="Project facts">
      <div class="cl-wrap cl-stats__grid">
        <div class="cl-stat">
          <span class="cl-stat__value">4</span>
          <span class="cl-stat__label">target profiles</span>
        </div>
        <div class="cl-stat">
          <span class="cl-stat__value">81</span>
          <span class="cl-stat__label">tracked SDK capabilities</span>
        </div>
        <div class="cl-stat">
          <span class="cl-stat__value">43</span>
          <span class="cl-stat__label">standard-library modules</span>
        </div>
        <div class="cl-stat">
          <span class="cl-stat__value">0</span>
          <span class="cl-stat__label">external Rust crates</span>
        </div>
      </div>
    </section>

    <section class="cl-section">
      <div class="cl-wrap">
        <header class="cl-section__head">
          <p class="cl-eyebrow">See it run</p>
          <h2>Real code. Real Amiga screens.</h2>
          <p>
            Every capture below comes from the project's FS-UAE acceptance
            runs, not from a mockup. The programs are built from the same source
            tree you can clone today.
          </p>
        </header>
        <div class="cl-shots">
          <ScreenshotCard
            class="cl-shot--wide"
            src="/screenshots/a500-gui.webp"
            alt="A Clorraine Intuition window on an Amiga screen, with a name field, a Say hello button, and a ready status line."
            caption="Intuition: windows, gadgets, requesters, and keyboard shortcuts."
            tag="A500 · KS 1.3"
          />
          <ScreenshotCard
            src="/screenshots/a500-boing.webp"
            alt="A chequered Amiga screen with a shaded sphere moving across it, drawn by the Clorraine Boing demo."
            caption="Boing: chip-memory bitmaps, a copper list, and a masked blit."
            tag="A500 · KS 1.3"
          />
          <ScreenshotCard
            src="/screenshots/a500-assets.webp"
            alt="The Clorraine ILBM viewer showing a checkerboard and a diamond image on a dark blue screen."
            caption="DOS and assets: an ILBM image decoded and displayed."
            tag="A500 · KS 1.3"
          />
        </div>
        <p class="cl-section__more">
          <a :href="withBase('/examples/')">Browse the examples →</a>
        </p>
      </div>
    </section>

    <section class="cl-section cl-section--panel">
      <div class="cl-wrap cl-split">
        <div class="cl-split__copy">
          <p class="cl-eyebrow">Ownership without a garbage collector</p>
          <h2>The compiler catches what C cannot.</h2>
          <p>
            Moves, borrows, and deterministic drops are checked before the C
            compiler sees the program. A value that has been moved cannot be
            used again, and the compiler says exactly where.
          </p>
          <ul class="cl-checks">
            <li>One owner per value; drops run once at scope end.</li>
            <li>Read-only lends, writable borrows, and explicit moves.</li>
            <li>No collector, no hidden ownership runtime, no tracing pauses.</li>
          </ul>
          <a class="cl-text-link" :href="withBase('/language/ownership')">
            How ownership works
            <span aria-hidden="true">→</span>
          </a>
        </div>

        <div class="cl-diagnostic">
          <div class="cl-diagnostic__bar">
            <span class="cl-terminal__file">ownership-fail.clo</span>
            <span class="cl-diagnostic__state">rejected</span>
          </div>
          <pre class="cl-code cl-code--compact"><code><span class="tok-kw">let</span> n: <span class="tok-type">u32</span> = <span class="tok-num">1</span>;
<span class="tok-kw">let</span> s: <span class="tok-type">String</span> = <span class="tok-str">$"n={n}"</span>;
<span class="tok-kw">let</span> t: <span class="tok-type">String</span> = s;

observe(s);</code></pre>
          <div class="cl-diagnostic__error">
            <div class="cl-diagnostic__code">error E10020</div>
            <p>'s' was moved and cannot be used</p>
            <p class="cl-diagnostic__help">
              help: re-initialize the binding or use a view
            </p>
          </div>
        </div>
      </div>
    </section>

    <section class="cl-section">
      <div class="cl-wrap">
        <header class="cl-section__head cl-section__head--center">
          <p class="cl-eyebrow">From source to silicon</p>
          <h2>One language. Four targets. Plain C99.</h2>
          <p>
            The compiler lowers the whole program to one readable C translation
            unit. The toolchain you already trust does the rest.
          </p>
        </header>
        <ol class="cl-pipeline">
          <li class="cl-pipeline__step">
            <span class="cl-pipeline__index">01</span>
            <strong>.clo source</strong>
            <span>Modules, ownership, generics, and hardware access.</span>
          </li>
          <li class="cl-pipeline__step">
            <span class="cl-pipeline__index">02</span>
            <strong>Clorraine checks</strong>
            <span>Types, moves, borrows, drops, and profile rules.</span>
          </li>
          <li class="cl-pipeline__step">
            <span class="cl-pipeline__index">03</span>
            <strong>C99 translation unit</strong>
            <span>Inspectable output with no hidden source language runtime.</span>
          </li>
          <li class="cl-pipeline__step">
            <span class="cl-pipeline__index">04</span>
            <strong>m68k GCC</strong>
            <span>AmigaOS hunk executables, or a host binary for tests.</span>
          </li>
        </ol>
        <p class="cl-pipeline__note">
          The generated C includes the small runtime shim the target needs. It
          contains no ownership logic; the checks already happened in the
          compiler.
        </p>
      </div>
    </section>

    <section class="cl-section cl-section--panel">
      <div class="cl-wrap">
        <header class="cl-section__head">
          <p class="cl-eyebrow">Built in the open</p>
          <h2>Evidence, not adjectives.</h2>
          <p>
            The specification is frozen. The compiler is guarded by
            specification tests, sanitizers, fuzzing, and host-versus-m68k
            differential runs.
          </p>
        </header>
        <div class="cl-evidence">
          <a
            class="cl-evidence__card"
            href="https://github.com/OpenLorraine/clorraine/tree/main/spec"
          >
            <span class="cl-evidence__tag">Specification</span>
            <strong>Frozen language and grammar</strong>
            <span>
              Chapters, the EBNF grammar, and a diagnostic registry versioned
              with the language.
            </span>
          </a>
          <a
            class="cl-evidence__card"
            href="https://github.com/OpenLorraine/clorraine/tree/main/compiler/tests"
          >
            <span class="cl-evidence__tag">Tests</span>
            <strong>Specification tests as the oracle</strong>
            <span>
              Compile-fail, run, staged, and corpus suites derive from the
              specification, not from the implementation.
            </span>
          </a>
          <a class="cl-evidence__card" :href="withBase('/project/verification')">
            <span class="cl-evidence__tag">Verification</span>
            <strong>Sanitizers, fuzzing, and differentials</strong>
            <span>
              ASan and UBSan legs, a deterministic fuzzer, and host-versus-m68k
              output comparisons.
            </span>
          </a>
          <a class="cl-evidence__card" :href="withBase('/project/licensing')">
            <span class="cl-evidence__tag">Licence</span>
            <strong>Apache 2.0, output stays yours</strong>
            <span>
              Fork it, bundle it, sell it, or ship proprietary programs built
              with it. The trademark policy protects the name.
            </span>
          </a>
        </div>
      </div>
    </section>

    <section class="cl-section cl-section--last">
      <div class="cl-wrap">
        <PartnerCTA />
      </div>
    </section>
  </div>
</template>
