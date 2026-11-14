<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import { withBase } from "vitepress";

/**
 * Minimal typing for the pinned Copperline browser bundle. The bundle is
 * loaded from a static path at runtime, so VitePress never bundles the
 * emulator and a page stays a normal document until the visitor presses play.
 */
interface CopperlineWasm {
  memory: WebAssembly.Memory;
}

interface CopperlineEmu {
  run(now: number, maxFrames: number): void;
  present_width(): number;
  present_rows(): number;
  present_ptr(): number;
  present_content_rect(): ArrayLike<number>;
  resync_clock(): void;
  load_rom(rom: Uint8Array, ext?: Uint8Array): void;
  load_state(state: Uint8Array): void;
  set_floppy_speed(speed: number): void;
  set_volume_percent(percent: number): void;
  set_port_device(port: number, device: string): void;
  set_joystick_port(
    port: number,
    up: boolean,
    down: boolean,
    left: boolean,
    right: boolean,
    fire: boolean,
    button2: boolean,
  ): void;
  free(): void;
}

interface CopperlineModule {
  default: () => Promise<CopperlineWasm>;
  WebEmu: new (model: string, video: string, drives: number) => CopperlineEmu;
}

type Phase = "idle" | "loading" | "running" | "paused" | "error";

const props = withDefaults(
  defineProps<{
    /** Poster shown before the emulator loads. */
    poster: string;
    /** Copperline save state for this demo. */
    state: string;
    /** Bootable ADF, used as the download and the hosted fallback. */
    adf: string;
    /** Small label over the picture. */
    badge?: string;
    /** Caption under the picture while the demo is idle. */
    caption?: string;
    /** Whether the demo reads a joystick in port 2. */
    interactive?: boolean;
    /** Hint shown on the idle screen. */
    hint?: string;
  }>(),
  {
    badge: "A500 · OCS",
    caption: "Compiled with clc · live in Copperline",
    interactive: false,
    hint: "Runs in your browser with Copperline and AROS. First load takes a few seconds.",
  },
);

const phase = ref<Phase>("idle");
const message = ref("");
const progress = ref<number | null>(null);
const stageRef = ref<HTMLElement | null>(null);
const canvasRef = ref<HTMLCanvasElement | null>(null);

let emulator: CopperlineEmu | null = null;
let wasm: CopperlineWasm | null = null;
let stateBytes: Uint8Array | null = null;
let offscreen: HTMLCanvasElement | null = null;
let offscreenContext: CanvasRenderingContext2D | null = null;
let animation = 0;
let looping = false;
let clockStarted = false;
let lastFrameAt = 0;
let disposed = false;
let leftHeld = false;
let rightHeld = false;
let fireHeld = false;

const posterUrl = computed(() => withBase(props.poster));
const adfUrl = computed(() => withBase(props.adf));
const fallbackUrl = computed(() => {
  if (typeof window === "undefined") return "https://copperline.dev/try/";
  const disk = new URL(adfUrl.value, window.location.href).href;
  return `https://copperline.dev/try/?machine=A500&df0=${encodeURIComponent(disk)}`;
});

/** Fetches a binary asset and reports byte progress when the server allows it. */
async function fetchBytes(
  url: string,
  onProgress?: (loaded: number, total: number) => void,
): Promise<Uint8Array> {
  const response = await fetch(url);
  if (!response.ok) {
    throw new Error(`${url}: HTTP ${response.status}`);
  }
  const total = Number(response.headers.get("content-length") ?? 0);
  if (!response.body || !onProgress || total <= 0) {
    return new Uint8Array(await response.arrayBuffer());
  }
  const reader = response.body.getReader();
  const chunks: Uint8Array[] = [];
  let loaded = 0;
  for (;;) {
    const { done, value } = await reader.read();
    if (done) break;
    if (value) {
      chunks.push(value);
      loaded += value.byteLength;
      onProgress(loaded, total);
    }
  }
  const result = new Uint8Array(loaded);
  let offset = 0;
  for (const chunk of chunks) {
    result.set(chunk, offset);
    offset += chunk.byteLength;
  }
  return result;
}

/** Draws the presentation buffer, trimming the small screen-border corner. */
function draw(): void {
  const canvas = canvasRef.value;
  if (!canvas || !emulator || !wasm) return;

  const width = emulator.present_width();
  const height = emulator.present_rows();
  if (width < 1 || height < 1) return;

  if (!offscreen) {
    offscreen = document.createElement("canvas");
    offscreenContext = offscreen.getContext("2d", { alpha: false });
  }
  if (!offscreen || !offscreenContext) return;
  if (offscreen.width !== width || offscreen.height !== height) {
    offscreen.width = width;
    offscreen.height = height;
    offscreenContext.imageSmoothingEnabled = false;
  }

  const pixels = new Uint8ClampedArray(
    wasm.memory.buffer,
    emulator.present_ptr(),
    width * height * 4,
  );
  offscreenContext.putImageData(new ImageData(pixels, width, height), 0, 0);

  const content = emulator.present_content_rect();
  let sourceX = 0;
  let sourceY = 0;
  let sourceWidth = width;
  let sourceHeight = height;
  if (content && content.length >= 4) {
    const x = Number(content[0]);
    const y = Number(content[1]);
    const w = Number(content[2]);
    const h = Number(content[3]);
    if (w > 8 && h > 8) {
      sourceX = x;
      sourceY = y;
      sourceWidth = w;
      sourceHeight = h;
    }
  }

  // The AROS screen border leaves a green wedge in the top-left corner. A
  // sixteen-pixel trim removes it without touching the demo picture.
  const trim = Math.min(16, sourceWidth - 32, sourceHeight - 32);
  const cropX = sourceX + Math.max(0, trim);
  const cropY = sourceY + Math.max(0, trim);
  const cropWidth = sourceWidth - Math.max(0, trim);
  const cropHeight = sourceHeight - Math.max(0, trim);

  if (canvas.width !== cropWidth || canvas.height !== cropHeight) {
    canvas.width = cropWidth;
    canvas.height = cropHeight;
  }
  const context = canvas.getContext("2d", { alpha: false });
  if (!context) return;
  context.imageSmoothingEnabled = false;
  context.drawImage(
    offscreen,
    cropX,
    cropY,
    cropWidth,
    cropHeight,
    0,
    0,
    cropWidth,
    cropHeight,
  );
}

/**
 * Runs one animation frame.
 *
 * `run` steps the emulated machine up to the wall clock, so a host that
 * cannot emulate a PAL frame in 20 ms starts each callback further behind.
 * Left alone, Copperline answers by stepping its full 20-frame budget on every
 * callback, which takes even longer and collapses the page to a slideshow.
 * Re-anchoring drops that debt and keeps the demo at whatever rate the host can
 * actually sustain.
 */
function loop(now: number): void {
  if (!looping || !emulator) return;
  if (clockStarted && now - lastFrameAt > 100) {
    emulator.resync_clock();
  }
  clockStarted = true;
  lastFrameAt = now;
  emulator.run(now, 20);
  draw();
  animation = requestAnimationFrame(loop);
}

function startLoop(): void {
  if (looping) return;
  looping = true;
  animation = requestAnimationFrame(loop);
}

function stopLoop(): void {
  looping = false;
  cancelAnimationFrame(animation);
}

/** Sends the current keyboard/touch stick state to Copperline. */
function pushStick(): void {
  emulator?.set_joystick_port(
    2,
    false,
    false,
    leftHeld,
    rightHeld,
    fireHeld,
    false,
  );
}

function onKeyDown(event: KeyboardEvent): void {
  if (!props.interactive || !emulator) return;
  if (phase.value !== "running" && phase.value !== "paused") return;
  const code = event.code;
  if (code === "ArrowLeft" || code === "KeyA") {
    leftHeld = true;
  } else if (code === "ArrowRight" || code === "KeyD") {
    rightHeld = true;
  } else if (code === "Space" || code === "Enter") {
    fireHeld = true;
  } else {
    return;
  }
  event.preventDefault();
  pushStick();
}

function onKeyUp(event: KeyboardEvent): void {
  if (!props.interactive || !emulator) return;
  const code = event.code;
  if (code === "ArrowLeft" || code === "KeyA") {
    leftHeld = false;
  } else if (code === "ArrowRight" || code === "KeyD") {
    rightHeld = false;
  } else if (code === "Space" || code === "Enter") {
    fireHeld = false;
  } else {
    return;
  }
  event.preventDefault();
  pushStick();
}

function releaseStick(): void {
  leftHeld = false;
  rightHeld = false;
  fireHeld = false;
  pushStick();
}

/** Loads the emulator, AROS, and the saved machine state. */
async function start(): Promise<void> {
  if (phase.value === "loading" || phase.value === "running") return;
  phase.value = "loading";
  message.value = "Loading the Copperline emulator…";
  progress.value = null;

  try {
    const moduleUrl = withBase("/copperline/pkg/copperline_web.js");
    const module = (await import(
      /* @vite-ignore */ moduleUrl
    )) as unknown as CopperlineModule;
    wasm = await module.default();
    if (disposed) return;

    message.value = "Loading AROS and the saved machine…";
    let loaded = 0;
    const report = (bytes: number, total: number): void => {
      loaded += bytes;
      progress.value = Math.min(99, Math.round((loaded / (3 * 1024 * 1024)) * 100));
    };
    const [rom, ext, state] = await Promise.all([
      fetchBytes(withBase("/copperline/aros/aros-amiga-m68k-rom.bin"), report),
      fetchBytes(withBase("/copperline/aros/aros-amiga-m68k-ext.bin"), report),
      fetchBytes(withBase(props.state), report),
    ]);
    if (disposed) return;

    const machine = new module.WebEmu("A500", "PAL", 2);
    machine.set_floppy_speed(0);
    machine.set_volume_percent(0);
    machine.load_rom(rom, ext);
    machine.load_state(state);
    emulator = machine;
    stateBytes = state;

    progress.value = 100;
    message.value = "Running in Copperline with AROS";
    phase.value = "running";
    startLoop();
  } catch (error) {
    phase.value = "error";
    message.value =
      error instanceof Error ? error.message : "The live demo could not start.";
  }
}

function pause(): void {
  if (phase.value !== "running") return;
  stopLoop();
  releaseStick();
  phase.value = "paused";
}

function resume(): void {
  if (phase.value !== "paused" || !emulator) return;
  clockStarted = false;
  phase.value = "running";
  startLoop();
}

function toggle(): void {
  if (phase.value === "running") pause();
  else if (phase.value === "paused") resume();
}

function restart(): void {
  if (!emulator || !stateBytes) return;
  emulator.load_state(stateBytes);
  leftHeld = false;
  rightHeld = false;
  fireHeld = false;
  clockStarted = false;
  if (phase.value === "paused") {
    phase.value = "running";
    startLoop();
  }
}

async function fullscreen(): Promise<void> {
  const stage = stageRef.value;
  if (!stage) return;
  try {
    if (document.fullscreenElement) await document.exitFullscreen();
    else await stage.requestFullscreen();
  } catch {
    // A browser that refuses fullscreen simply leaves the inline view.
  }
}

function onVisibility(): void {
  if (!emulator) return;
  if (document.hidden) {
    stopLoop();
    if (props.interactive) releaseStick();
  } else if (phase.value === "running") {
    clockStarted = false;
    startLoop();
  }
}

function touch(code: "left" | "right" | "fire", pressed: boolean): void {
  if (code === "left") leftHeld = pressed;
  if (code === "right") rightHeld = pressed;
  if (code === "fire") fireHeld = pressed;
  pushStick();
}

onMounted(() => {
  document.addEventListener("visibilitychange", onVisibility);
  window.addEventListener("keydown", onKeyDown);
  window.addEventListener("keyup", onKeyUp);
  window.addEventListener("blur", releaseStick);
});

onBeforeUnmount(() => {
  disposed = true;
  document.removeEventListener("visibilitychange", onVisibility);
  window.removeEventListener("keydown", onKeyDown);
  window.removeEventListener("keyup", onKeyUp);
  window.removeEventListener("blur", releaseStick);
  stopLoop();
  emulator?.free();
  emulator = null;
  stateBytes = null;
  offscreen = null;
  offscreenContext = null;
});
</script>

<template>
  <figure class="ap-player">
    <div ref="stageRef" class="ap-player__stage">
      <img
        v-if="phase === 'idle' || phase === 'loading' || phase === 'error'"
        class="ap-player__poster"
        :src="posterUrl"
        :alt="caption"
      />
      <canvas
        v-show="phase === 'running' || phase === 'paused'"
        ref="canvasRef"
        class="ap-player__canvas"
        role="img"
        :aria-label="caption"
      ></canvas>

      <div v-if="phase === 'idle'" class="ap-player__veil">
        <button class="ap-player__play" type="button" @click="start">
          <span class="ap-player__play-icon" aria-hidden="true">▶</span>
          {{ interactive ? "Play now" : "Run the demo" }}
        </button>
        <p class="ap-player__hint">{{ hint }}</p>
        <p v-if="interactive" class="ap-player__keys">
          ← → move · Space fires
        </p>
      </div>

      <div v-else-if="phase === 'loading'" class="ap-player__veil">
        <p class="ap-player__loading">{{ message }}</p>
        <div class="ap-player__progress" aria-hidden="true">
          <span :style="{ width: `${progress ?? 8}%` }"></span>
        </div>
      </div>

      <div v-else-if="phase === 'error'" class="ap-player__veil">
        <p class="ap-player__loading">The live demo could not start.</p>
        <p class="ap-player__hint">{{ message }}</p>
        <p class="ap-player__links">
          <a :href="fallbackUrl" target="_blank" rel="noopener">
            Open it in Copperline
          </a>
          <a :href="adfUrl" download>Download the ADF</a>
        </p>
      </div>

      <span class="ap-player__badge">{{ badge }}</span>
    </div>

    <figcaption class="ap-player__bar">
      <div class="ap-player__controls">
        <button
          type="button"
          class="ap-player__button"
          :disabled="phase === 'idle' || phase === 'loading' || phase === 'error'"
          @click="toggle"
        >
          {{ phase === "paused" ? "Resume" : "Pause" }}
        </button>
        <button
          type="button"
          class="ap-player__button"
          :disabled="phase !== 'running' && phase !== 'paused'"
          @click="restart"
        >
          Restart
        </button>
        <button
          type="button"
          class="ap-player__button"
          :disabled="phase !== 'running' && phase !== 'paused'"
          @click="fullscreen"
        >
          Fullscreen
        </button>
      </div>
      <div v-if="interactive" class="ap-player__touch" aria-label="Touch controls">
        <button
          type="button"
          class="ap-player__touch-button"
          aria-label="Move left"
          @pointerdown.prevent="touch('left', true)"
          @pointerup.prevent="touch('left', false)"
          @pointerleave="touch('left', false)"
        >
          ◀
        </button>
        <button
          type="button"
          class="ap-player__touch-button"
          aria-label="Move right"
          @pointerdown.prevent="touch('right', true)"
          @pointerup.prevent="touch('right', false)"
          @pointerleave="touch('right', false)"
        >
          ▶
        </button>
        <button
          type="button"
          class="ap-player__touch-button ap-player__touch-button--fire"
          aria-label="Fire"
          @pointerdown.prevent="touch('fire', true)"
          @pointerup.prevent="touch('fire', false)"
          @pointerleave="touch('fire', false)"
        >
          Fire
        </button>
      </div>
      <span class="ap-player__caption">{{
        phase === "running" || phase === "paused"
          ? "Running in Copperline with AROS"
          : caption
      }}</span>
    </figcaption>
    <p class="ap-player__sr" aria-live="polite">{{ message }}</p>
  </figure>
</template>

<style scoped>
.ap-player {
  margin: 28px 0;
  overflow: hidden;
  border: 1px solid var(--vp-c-divider);
  border-radius: 16px;
  background: #05070a;
  box-shadow: 0 30px 80px rgba(0, 0, 0, 0.28);
}

.ap-player__stage {
  position: relative;
  aspect-ratio: 4 / 3;
  overflow: hidden;
  background: #05070a;
}

.ap-player__poster,
.ap-player__canvas {
  position: absolute;
  inset: 0;
  display: block;
  width: 100%;
  height: 100%;
  object-fit: fill;
  image-rendering: pixelated;
}

.ap-player__canvas {
  background: #05070a;
}

.ap-player__veil {
  position: absolute;
  inset: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 12px;
  padding: 28px;
  text-align: center;
  background:
    radial-gradient(60% 60% at 50% 40%, rgba(4, 6, 10, 0.15), rgba(4, 6, 10, 0.72)),
    linear-gradient(180deg, rgba(4, 6, 10, 0.1), rgba(4, 6, 10, 0.55));
}

.ap-player__play {
  display: inline-flex;
  align-items: center;
  gap: 12px;
  min-height: 52px;
  padding: 0 24px;
  border: 1px solid rgba(245, 165, 36, 0.7);
  border-radius: 999px;
  background: rgba(10, 13, 18, 0.78);
  color: #ffe9b0;
  font-family: var(--vp-font-family-base);
  font-size: 15px;
  font-weight: 650;
  cursor: pointer;
  backdrop-filter: blur(8px);
  transition:
    background-color 0.15s ease,
    transform 0.15s ease,
    border-color 0.15s ease;
}

.ap-player__play:hover {
  background: rgba(245, 165, 36, 0.16);
  border-color: #f5a524;
  transform: translateY(-1px);
}

.ap-player__play-icon {
  display: grid;
  place-items: center;
  width: 26px;
  height: 26px;
  border-radius: 50%;
  background: #f5a524;
  color: #171106;
  font-size: 11px;
  padding-left: 2px;
}

.ap-player__hint {
  max-width: 40ch;
  margin: 0;
  color: #c4cfdb;
  font-size: 12.5px;
  line-height: 1.5;
}

.ap-player__keys {
  margin: 0;
  color: #f5c77e;
  font-family: var(--vp-font-family-mono);
  font-size: 11px;
  letter-spacing: 0.04em;
}

.ap-player__loading {
  margin: 0;
  color: #f4e8ce;
  font-size: 14px;
  font-weight: 560;
}

.ap-player__progress {
  width: min(280px, 70%);
  height: 3px;
  overflow: hidden;
  border-radius: 999px;
  background: rgba(255, 255, 255, 0.14);
}

.ap-player__progress span {
  display: block;
  height: 100%;
  border-radius: inherit;
  background: #f5a524;
  transition: width 0.2s ease;
}

.ap-player__links {
  display: flex;
  flex-wrap: wrap;
  justify-content: center;
  gap: 10px 18px;
  margin: 4px 0 0;
  font-size: 12.5px;
}

.ap-player__links a {
  color: #ffd68a;
}

.ap-player__badge {
  position: absolute;
  top: 14px;
  left: 14px;
  padding: 5px 9px;
  border: 1px solid rgba(245, 165, 36, 0.3);
  border-radius: 999px;
  background: rgba(5, 7, 10, 0.68);
  color: #f5c77e;
  font-family: var(--vp-font-family-mono);
  font-size: 9px;
  letter-spacing: 0.1em;
  text-transform: uppercase;
  backdrop-filter: blur(6px);
}

.ap-player__bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 10px 14px;
  border-top: 1px solid rgba(255, 255, 255, 0.08);
  background: #090d12;
}

.ap-player__controls {
  display: flex;
  gap: 6px;
}

.ap-player__button,
.ap-player__touch-button {
  min-height: 30px;
  padding: 0 11px;
  border: 1px solid rgba(255, 255, 255, 0.16);
  border-radius: 7px;
  background: rgba(255, 255, 255, 0.03);
  color: #d5dee8;
  font-family: var(--vp-font-family-base);
  font-size: 11.5px;
  font-weight: 560;
  cursor: pointer;
  touch-action: none;
}

.ap-player__button:hover:not(:disabled),
.ap-player__touch-button:hover {
  border-color: rgba(245, 165, 36, 0.55);
  color: #ffe1a6;
}

.ap-player__button:disabled {
  opacity: 0.38;
  cursor: default;
}

.ap-player__touch {
  display: flex;
  gap: 6px;
}

.ap-player__touch-button {
  min-width: 38px;
  color: #ffe1a6;
}

.ap-player__touch-button--fire {
  min-width: 52px;
  border-color: rgba(245, 165, 36, 0.45);
}

.ap-player__caption {
  color: #7f8c9c;
  font-family: var(--vp-font-family-mono);
  font-size: 10px;
  letter-spacing: 0.02em;
  text-align: right;
}

.ap-player__sr {
  position: absolute;
  width: 1px;
  height: 1px;
  overflow: hidden;
  clip: rect(0, 0, 0, 0);
  white-space: nowrap;
}

@media (max-width: 720px) {
  .ap-player__bar {
    align-items: flex-start;
    flex-direction: column;
    gap: 8px;
  }

  .ap-player__caption {
    text-align: left;
  }
}
</style>
