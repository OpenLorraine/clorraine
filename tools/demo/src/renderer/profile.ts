import { ConfigError } from "../scenes.ts";

/**
 * How a demo's design maps onto its output canvas.
 *
 * The HTML pages are authored for a fixed design size (1100x700). A profile says how
 * large to render that design (`design * scale`, so text stays crisp) and what the
 * final video should be. When the output is wider than the design scaled to the output
 * height, the spare width is filled with the background color (pillarboxing).
 */
export interface RenderProfile {
  /** Design (CSS layout) width, in pixels. */
  designWidth: number;
  /** Design (CSS layout) height, in pixels. */
  designHeight: number;
  /** Device pixel factor: HTML is captured at `design * scale`. */
  scale: number;
  /** Final video width, in pixels. */
  outputWidth: number;
  /** Final video height, in pixels. */
  outputHeight: number;
  /** Bottom strip reserved for captions, in output pixels. */
  captionStrip: number;
}

/** The default profile: the design is the output, one CSS pixel per video pixel. */
export const DEFAULT_PROFILE: RenderProfile = {
  designWidth: 1100,
  designHeight: 700,
  scale: 1,
  outputWidth: 1100,
  outputHeight: 700,
  captionStrip: 80,
};

/** Rounds to the nearest even integer, so yuv420p encoding never gets an odd size. */
function even(value: number): number {
  return 2 * Math.round(value / 2);
}

/**
 * Builds a 16:9 profile whose height is `outputHeight`: the 1100x700 design is rendered
 * at twice its size (crisp) and pillarboxed into the wider canvas.
 *
 * @throws {ConfigError} when `outputHeight` is not a positive even number.
 */
export function profileForHeight(outputHeight: number): RenderProfile {
  if (!Number.isInteger(outputHeight) || outputHeight < 2 || outputHeight % 2 !== 0) {
    throw new ConfigError(`--size must be a positive even height, not ${outputHeight}`);
  }
  const scale = 2;
  return {
    designWidth: DEFAULT_PROFILE.designWidth,
    designHeight: DEFAULT_PROFILE.designHeight,
    scale,
    outputWidth: even((outputHeight * 16) / 9),
    outputHeight,
    captionStrip: Math.round((DEFAULT_PROFILE.captionStrip * outputHeight) / DEFAULT_PROFILE.designHeight),
  };
}

/** The width the design occupies on the output canvas, in output pixels (even). */
export function contentWidth(profile: RenderProfile): number {
  return even((profile.designWidth * profile.outputHeight) / profile.designHeight);
}
