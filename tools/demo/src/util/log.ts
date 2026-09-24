/** Writes a progress line to stderr so stdout stays free for machine-readable output. */
export function info(message: string): void {
  process.stderr.write(`${message}\n`);
}

/** Writes a warning to stderr. */
export function warn(message: string): void {
  process.stderr.write(`warning: ${message}\n`);
}

/** Writes a formatted error to stderr. */
export function error(message: string): void {
  process.stderr.write(`error: ${message}\n`);
}

/** The message of an unknown thrown value. */
export function messageOf(value: unknown): string {
  return value instanceof Error ? value.message : String(value);
}
