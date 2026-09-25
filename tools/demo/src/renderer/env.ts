/** The interfaces a scene renderer needs. Real implementations shell out to Chrome and
 * ffmpeg; tests inject fakes. */

/** The bottom strip of the canvas reserved for the caption, in pixels. */
export const CAPTION_STRIP = 80;

/** The height of the content area: the canvas height minus the caption strip. */
export function contentHeight(height: number, captionStrip: number = CAPTION_STRIP): number {
  return height - captionStrip;
}

/** A request for a run of frames from one HTML page. */
export interface FrameSpec {
  /** Scene id; frames are written under a directory named for it. */
  id: string;
  /** Page path: relative to the assets directory, or an absolute filesystem path. */
  page: string;
  /** Query parameters passed to the page (read by its script). */
  query?: Record<string, string>;
  /** Design (CSS layout) width in pixels. */
  width: number;
  /** Design (CSS layout) height in pixels. */
  height: number;
  /** Device pixel factor; the captured image is `width * scale` by `height * scale`. */
  scale: number;
  /** Number of frames to capture. */
  count: number;
  /** Frame rate used to advance the page's time `t` between frames. */
  fps: number;
}

/** Produces a run of PNG frames for one HTML page. */
export interface FrameSource {
  /**
   * Captures `spec.count` frames, driving `window.render(t)` at increasing `t`.
   *
   * @returns the frame paths in order.
   */
  capture(spec: FrameSpec): Promise<string[]>;
}

/** Encodes an ordered list of PNG frames into a video clip. */
export interface Encoder {
  encodeFrames(frames: string[], out: string, options: { fps: number }): Promise<void>;
}

/** Everything a renderer needs to turn a scene into a clip. */
export interface RenderEnv {
  frames: FrameSource;
  encode: Encoder;
  /** Repository root; scene file paths are resolved against it. */
  root: string;
  /** Design (CSS layout) width in pixels. */
  width: number;
  /** Design (CSS layout) height in pixels. */
  height: number;
  /** Device pixel factor: HTML is captured at `width * scale` by `height * scale`. */
  scale: number;
  /** Final video width in pixels (may be wider than the design, for pillarboxing). */
  outputWidth: number;
  /** Final video height in pixels. */
  outputHeight: number;
  /** Bottom strip reserved for captions, in output pixels. */
  captionStrip: number;
  /** Output frame rate. */
  fps: number;
  /** Directory for finished clips. */
  outDir: string;
  /** Directory for intermediates (pages, frames). */
  workDir: string;
}
