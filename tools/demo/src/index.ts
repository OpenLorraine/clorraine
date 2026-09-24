export {
  DEFAULT_DURATION,
  defineDemo,
  ConfigError,
  sceneDuration,
  validateConfig,
} from "./scenes.ts";
export type {
  CardScene,
  CodeScene,
  DemoConfig,
  EmulatorScene,
  OutroScene,
  Scene,
  SceneBase,
  TerminalScene,
} from "./scenes.ts";
export { loadConfig } from "./config.ts";
export type { LoadOptions } from "./config.ts";
export { defaultRepoRoot, PathError } from "./util/paths.ts";
