/** The interfaces a scene renderer needs. Real implementations shell out to Chrome and
 * ffmpeg; tests inject fakes. */

/** A request for a run of frames from one HTML page. */
export interface FrameSpec {
  /** Scene id; frames are written under a directory named for it. */
  id: string;
  /** Page path: relative to the assets directory, or an absolute filesystem path. */
  page: string;
  /** Query parameters passed to the page (read by its script). */
  query?: Record<string, string>;
  /** Canvas width in pixels. */
  width: number;
  /** Canvas height in pixels. */
  height: number;
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
  /** Canvas width in pixels. */
  width: number;
  /** Canvas height in pixels. */
  height: number;
  /** Output frame rate. */
  fps: number;
  /** Directory for finished clips. */
  outDir: string;
  /** Directory for intermediates (pages, frames). */
  workDir: string;
}
