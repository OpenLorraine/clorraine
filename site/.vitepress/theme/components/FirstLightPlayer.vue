<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from "vue";
import { withBase } from "vitepress";

/**
 * Minimal typing for the Copperline browser bundle. The bundle is loaded at
 * runtime from a static path, so the site does not bundle the emulator and the
 * page stays a normal VitePress document until the visitor asks for the demo.
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
  load_rom(rom: Uint8Array, ext: Uint8Array): void;
  load_state(state: Uint8Array): void;
  set_floppy_speed(speed: number): void;
  set_volume_percent(percent: number): void;
  free(): void;
}

interface CopperlineModule {
  default: () => Promise<CopperlineWasm>;
  WebEmu: new (model: string, video: string, drives: number) => CopperlineEmu;
}

type Phase = "idle" | "loading" | "running" | "paused" | "error";

const poster = withBase("/screenshots/first-light.webp");
const adfUrl = withBase("/demos/first-light.adf");
const fallbackUrl =
  "https://copperline.dev/try/?machine=A500&df0=" +
  encodeURIComponent(
    "https://openlorraine.github.io/clorraine/demos/first-light.adf",
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
let disposed = false;

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

/** Draws the current presentation buffer, cropped to a clean 4:3 frame. */
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

  // Copperline reports the active raster box. The first row is the remains of
  // the AROS screen border, so drop a few lines before centring the 4:3 crop.
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

  const border = Math.min(6, Math.max(0, sourceHeight - 8));
  const cropHeight = sourceHeight - border;
  const cropWidth = Math.round((cropHeight * 4) / 3);
  const visibleWidth = Math.min(cropWidth, sourceWidth);
  const left =
    sourceX + Math.max(0, Math.round((sourceWidth - visibleWidth) / 2));

  if (canvas.width !== visibleWidth || canvas.height !== cropHeight) {
    canvas.width = visibleWidth;
    canvas.height = cropHeight;
  }
  const context = canvas.getContext("2d", { alpha: false });
  if (!context) return;
  context.imageSmoothingEnabled = false;
  context.drawImage(
    offscreen,
    left,
    sourceY + border,
    visibleWidth,
    cropHeight,
    0,
    0,
    visibleWidth,
    cropHeight,
  );
}

function loop(now: number): void {
  if (!looping || !emulator) return;
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

/** Loads the emulator, the AROS ROM, and the saved First Light machine. */
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

    message.value = "Loading AROS and the First Light machine…";
    let loaded = 0;
    const report = (bytes: number, total: number): void => {
      loaded += bytes;
      progress.value = Math.min(99, Math.round((loaded / (3 * 1024 * 1024)) * 100));
    };
    const [rom, ext, state] = await Promise.all([
      fetchBytes(withBase("/copperline/aros/aros-amiga-m68k-rom.bin"), report),
      fetchBytes(withBase("/copperline/aros/aros-amiga-m68k-ext.bin"), report),
      fetchBytes(withBase("/demos/first-light.clstate"), report),
    ]);
    if (disposed) return;

    const machine = new module.WebEmu("A500", "PAL", 2);
    machine.set_floppy_speed(0);
    machine.set_volume_percent(0);
    machine.load_rom(rom, ext);
    machine.load_state(state);
    emulator = machine;
    stateBytes = state;
    machine.resync_clock();

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
  phase.value = "paused";
}

function resume(): void {
  if (phase.value !== "paused" || !emulator) return;
  emulator.resync_clock();
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
  emulator.resync_clock();
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
  } else if (phase.value === "running") {
    emulator.resync_clock();
    startLoop();
  }
}

onMounted(() => {
  document.addEventListener("visibilitychange", onVisibility);
});

onBeforeUnmount(() => {
  disposed = true;
  document.removeEventListener("visibilitychange", onVisibility);
  stopLoop();
  emulator?.free();
  emulator = null;
  stateBytes = null;
  offscreen = null;
  offscreenContext = null;
});
</script>

<template>
  <figure class="fl-player">
    <div ref="stageRef" class="fl-player__stage">
      <img
        v-if="phase === 'idle' || phase === 'loading' || phase === 'error'"
        class="fl-player__poster"
        :src="poster"
        alt="First Light: a lighthouse on a dark headland beside a calm sea, under a glowing sunrise and a field of stars."
      />
      <canvas
        v-show="phase === 'running' || phase === 'paused'"
        ref="canvasRef"
        class="fl-player__canvas"
        role="img"
        aria-label="The First Light demo running on an emulated Amiga 500."
      ></canvas>

      <div v-if="phase === 'idle'" class="fl-player__veil">
        <button class="fl-player__play" type="button" @click="start">
          <span class="fl-player__play-icon" aria-hidden="true">▶</span>
          Run the demo
        </button>
        <p class="fl-player__hint">
          Runs in your browser with Copperline and AROS. First load takes a few
          seconds.
        </p>
      </div>

      <div v-else-if="phase === 'loading'" class="fl-player__veil">
        <p class="fl-player__loading">{{ message }}</p>
        <div class="fl-player__progress" aria-hidden="true">
          <span :style="{ width: `${progress ?? 8}%` }"></span>
        </div>
      </div>

      <div v-else-if="phase === 'error'" class="fl-player__veil">
        <p class="fl-player__loading">The live demo could not start.</p>
        <p class="fl-player__hint">{{ message }}</p>
        <p class="fl-player__links">
          <a :href="fallbackUrl" target="_blank" rel="noopener">
            Open it in Copperline
          </a>
          <a :href="adfUrl" download>Download the ADF</a>
        </p>
      </div>

      <span class="fl-player__badge">A500 · 16 colours</span>
    </div>

    <figcaption class="fl-player__bar">
      <div class="fl-player__controls">
        <button
          type="button"
          class="fl-player__button"
          :disabled="phase === 'idle' || phase === 'loading' || phase === 'error'"
          @click="toggle"
        >
          {{ phase === "paused" ? "Resume" : "Pause" }}
        </button>
        <button
          type="button"
          class="fl-player__button"
          :disabled="phase !== 'running' && phase !== 'paused'"
          @click="restart"
        >
          Restart
        </button>
        <button
          type="button"
          class="fl-player__button"
          :disabled="phase !== 'running' && phase !== 'paused'"
          @click="fullscreen"
        >
          Fullscreen
        </button>
      </div>
      <span class="fl-player__caption">{{
        phase === "running" || phase === "paused"
          ? "First Light · compiled with clc · live in Copperline"
          : "First Light · a real A500 program, not a mockup"
      }}</span>
    </figcaption>
    <p class="fl-player__sr" aria-live="polite">{{ message }}</p>
  </figure>
</template>

<style scoped>
.fl-player {
  margin: 28px 0;
  overflow: hidden;
  border: 1px solid var(--vp-c-divider);
  border-radius: 16px;
  background: #05070a;
  box-shadow: 0 30px 80px rgba(0, 0, 0, 0.28);
}

.fl-player__stage {
  position: relative;
  aspect-ratio: 4 / 3;
  overflow: hidden;
  background: #05070a;
}

.fl-player__poster,
.fl-player__canvas {
  position: absolute;
  inset: 0;
  display: block;
  width: 100%;
  height: 100%;
  object-fit: cover;
  image-rendering: pixelated;
}

.fl-player__canvas {
  background: #05070a;
}

.fl-player__veil {
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

.fl-player__play {
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

.fl-player__play:hover {
  background: rgba(245, 165, 36, 0.16);
  border-color: #f5a524;
  transform: translateY(-1px);
}

.fl-player__play-icon {
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

.fl-player__hint {
  max-width: 34ch;
  margin: 0;
  color: #c4cfdb;
  font-size: 12.5px;
  line-height: 1.5;
}

.fl-player__loading {
  margin: 0;
  color: #f4e8ce;
  font-size: 14px;
  font-weight: 560;
}

.fl-player__progress {
  width: min(280px, 70%);
  height: 3px;
  overflow: hidden;
  border-radius: 999px;
  background: rgba(255, 255, 255, 0.14);
}

.fl-player__progress span {
  display: block;
  height: 100%;
  border-radius: inherit;
  background: #f5a524;
  transition: width 0.2s ease;
}

.fl-player__links {
  display: flex;
  flex-wrap: wrap;
  justify-content: center;
  gap: 10px 18px;
  margin: 4px 0 0;
  font-size: 12.5px;
}

.fl-player__links a {
  color: #ffd68a;
}

.fl-player__badge {
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

.fl-player__bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 10px 14px;
  border-top: 1px solid rgba(255, 255, 255, 0.08);
  background: #090d12;
}

.fl-player__controls {
  display: flex;
  gap: 6px;
}

.fl-player__button {
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
}

.fl-player__button:hover:not(:disabled) {
  border-color: rgba(245, 165, 36, 0.55);
  color: #ffe1a6;
}

.fl-player__button:disabled {
  opacity: 0.38;
  cursor: default;
}

.fl-player__caption {
  color: #7f8c9c;
  font-family: var(--vp-font-family-mono);
  font-size: 10px;
  letter-spacing: 0.02em;
  text-align: right;
}

.fl-player__sr {
  position: absolute;
  width: 1px;
  height: 1px;
  overflow: hidden;
  clip: rect(0, 0, 0, 0);
  white-space: nowrap;
}

@media (max-width: 640px) {
  .fl-player__bar {
    align-items: flex-start;
    flex-direction: column;
    gap: 8px;
  }

  .fl-player__caption {
    text-align: left;
  }
}
</style>
