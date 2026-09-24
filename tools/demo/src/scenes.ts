/**
 * The scene model for a demo video.
 *
 * A demo is a list of scenes. Each scene kind renders to one clip; the clips are
 * composed into the final video. All scene paths are repository-relative and are
 * resolved and checked by the config loader, not here.
 */

/** Fields shared by every scene. */
export interface SceneBase {
  /** Unique, non-empty identifier used by the gif selection and intermediate files. */
  id: string;
  /** Optional caption shown in the bottom strip. */
  caption?: string;
  /** Seconds on screen. When omitted, the kind's default is used. */
  duration?: number;
}

/** A full-screen title card. */
export type CardScene = SceneBase & {
  kind: "card";
  title: string;
  subtitle?: string;
  lines?: string[];
};

/** A closing card. */
export type OutroScene = SceneBase & {
  kind: "outro";
  title: string;
  lines: string[];
};

/** A source file shown with syntax highlighting, optionally scrolling. */
export type CodeScene = SceneBase & {
  kind: "code";
  file: string;
  /** Scroll from top to bottom across the scene when the file is taller than the view. */
  scroll?: boolean;
  /** One-based line numbers to highlight; the rest are dimmed. */
  highlight?: number[];
};

/** One or more shell commands typed into a real terminal. */
export type TerminalScene = SceneBase & {
  kind: "terminal";
  commands: string[];
};

/** A program recorded while running under the Amiga emulator. */
export type EmulatorScene = SceneBase & {
  kind: "emulator";
  file: string;
  /** Draw the branded frame around the capture. Defaults to true when omitted. */
  frame?: boolean;
};

/** Any scene. */
export type Scene = CardScene | OutroScene | CodeScene | TerminalScene | EmulatorScene;

/** A complete demo description. */
export interface DemoConfig {
  /** Output MP4 path, repository-relative. */
  out: string;
  /** At least one scene. */
  scenes: Scene[];
  /** Optional PNG poster (frame 0 of the first card scene). */
  poster?: string;
  /** Optional GIF loop built from the named scenes. */
  gif?: { out: string; sceneIds: string[] };
  /** Canvas width in pixels. Defaults to 1100. */
  width?: number;
  /** Canvas height in pixels. Defaults to 700. */
  height?: number;
  /** Output frame rate. Defaults to 50 (the PAL rate). */
  fps?: number;
  /** Crossfade duration in seconds between adjacent scenes. Defaults to 0.5. */
  transition?: number;
}

/** Default seconds on screen per scene kind when `duration` is omitted. */
export const DEFAULT_DURATION: Readonly<Record<Scene["kind"], number>> = {
  card: 7,
  outro: 7,
  code: 12,
  terminal: 14,
  emulator: 7,
};

/** The scenes' on-screen seconds, applying the per-kind default. */
export function sceneDuration(scene: Scene): number {
  return scene.duration ?? DEFAULT_DURATION[scene.kind];
}

/** Thrown when a demo config is structurally invalid. */
export class ConfigError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "ConfigError";
  }
}

const SCENE_KINDS: ReadonlySet<string> = new Set(["card", "outro", "code", "terminal", "emulator"]);

function isSceneKind(value: unknown): value is Scene["kind"] {
  return typeof value === "string" && SCENE_KINDS.has(value);
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

/**
 * Validates a raw demo config, collecting every problem rather than stopping at the
 * first. Throws a {@link ConfigError} whose message lists all of them.
 *
 * A valid config has an `out` path, at least one scene, unique non-empty scene ids, a
 * positive duration where given, and a `gif.sceneIds` that only names existing scenes.
 * Per-kind required fields (a card title, a code file, terminal commands, and so on) are
 * checked too. Path existence and repository-root containment are checked later by the
 * config loader, which knows the root.
 */
export function validateConfig(config: unknown): asserts config is DemoConfig {
  const problems: string[] = [];

  if (!isRecord(config)) {
    throw new ConfigError("demo config: expected an object");
  }
  if (typeof config.out !== "string" || config.out.length === 0) {
    problems.push('missing "out" path');
  }
  if (config.poster !== undefined && (typeof config.poster !== "string" || config.poster.length === 0)) {
    problems.push('"poster" must be a non-empty string path');
  }
  if (!Array.isArray(config.scenes) || config.scenes.length === 0) {
    problems.push("at least one scene is required");
  }

  const seen = new Set<string>();
  if (Array.isArray(config.scenes)) {
    for (const raw of config.scenes) {
      if (!isRecord(raw)) {
        problems.push("scene: expected an object");
        continue;
      }
      const id = typeof raw.id === "string" ? raw.id : "";
      const where = id.length > 0 ? `scene "${id}"` : "scene (missing id)";
      if (id.length === 0) {
        problems.push("scene: id must be a non-empty string");
      } else if (!/^[A-Za-z0-9_-]+$/.test(id)) {
        problems.push(`${where}: id may only contain letters, digits, "_" and "-"`);
      } else if (seen.has(id)) {
        problems.push(`duplicate scene id "${id}"`);
      } else {
        seen.add(id);
      }
      if (raw.duration !== undefined) {
        if (typeof raw.duration !== "number" || !Number.isFinite(raw.duration) || raw.duration <= 0) {
          problems.push(`${where}: duration must be a positive number`);
        }
      }
      const kind = raw.kind;
      if (!isSceneKind(kind)) {
        problems.push(`${where}: unknown kind ${JSON.stringify(kind)}`);
        continue;
      }
      switch (kind) {
        case "card":
          if (typeof raw.title !== "string" || raw.title.length === 0) {
            problems.push(`${where}: card needs a title`);
          }
          break;
        case "outro":
          if (typeof raw.title !== "string" || raw.title.length === 0) {
            problems.push(`${where}: outro needs a title`);
          }
          if (!Array.isArray(raw.lines)) {
            problems.push(`${where}: outro needs a lines array`);
          }
          break;
        case "code":
          if (typeof raw.file !== "string" || raw.file.length === 0) {
            problems.push(`${where}: code needs a file`);
          }
          break;
        case "terminal":
          if (!Array.isArray(raw.commands) || raw.commands.length === 0 || raw.commands.some((c) => typeof c !== "string")) {
            problems.push(`${where}: terminal needs a non-empty commands array`);
          }
          break;
        case "emulator":
          if (typeof raw.file !== "string" || raw.file.length === 0) {
            problems.push(`${where}: emulator needs a file`);
          }
          break;
      }
    }
  }

  if (config.gif !== undefined) {
    if (!isRecord(config.gif) || typeof config.gif.out !== "string" || !Array.isArray(config.gif.sceneIds)) {
      problems.push('gif needs an "out" and a "sceneIds" array');
    } else {
      for (const sceneId of config.gif.sceneIds) {
        if (typeof sceneId !== "string" || !seen.has(sceneId)) {
          problems.push(`gif names unknown scene ${JSON.stringify(sceneId)}`);
        }
      }
    }
  }

  for (const key of ["width", "height", "fps", "transition"] as const) {
    const value = config[key];
    if (value !== undefined && (typeof value !== "number" || !Number.isFinite(value) || value <= 0)) {
      problems.push(`${key} must be a positive number`);
    }
  }

  if (problems.length > 0) {
    throw new ConfigError(`demo config: ${problems.join("; ")}`);
  }
}

/**
 * Identity helper that gives a config module full type checking without a build step.
 *
 * @example
 * ```ts
 * export default defineDemo({ out: "demo/out/x.mp4", scenes: [] });
 * ```
 */
export function defineDemo(config: DemoConfig): DemoConfig {
  return config;
}
