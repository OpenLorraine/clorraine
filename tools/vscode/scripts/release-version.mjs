// Check that a release tag, the compiler version, and the extension manifest
// agree before the extension is packaged or published.
//
// A Marketplace version can be published only once, so a mismatch must stop
// the workflow before anything leaves the build. The compiler's Cargo
// manifest is the version of record; the extension manifest must carry the
// same version, and a release tag must be exactly `v<version>`. Branch refs
// are allowed for dry runs only: `--publish` requires a release tag.
//
// Usage:
//   node release-version.mjs --ref <git ref> --cargo <Cargo.toml> --manifest <package.json> [--publish]
//
// Exit status: 0 when everything agrees, 1 on a mismatch, 2 on a usage error
// or an unreadable file. On success the version is printed and, when
// GITHUB_OUTPUT is set, appended there as `version=<version>`.

import { appendFileSync, readFileSync } from "node:fs";

/** Prints a message and exits with the given status. */
function fail(status, message) {
  console.error(`release-version: ${message}`);
  process.exit(status);
}

/** Parses the command line into its options; unknown or incomplete options are usage errors. */
function parseArguments(argv) {
  const options = { publish: false };
  for (let i = 0; i < argv.length; i++) {
    const flag = argv[i];
    if (flag === "--publish") {
      options.publish = true;
    } else if (flag === "--ref" || flag === "--cargo" || flag === "--manifest") {
      if (i + 1 >= argv.length) {
        fail(2, `${flag} needs a value`);
      }
      options[flag.slice(2)] = argv[++i];
    } else {
      fail(2, `unknown argument ${flag}`);
    }
  }
  for (const required of ["ref", "cargo", "manifest"]) {
    if (!options[required]) {
      fail(2, `--${required} is required`);
    }
  }
  return options;
}

/** Reads a text file, turning any I/O failure into a usage error that names the path. */
function read(path) {
  try {
    return readFileSync(path, "utf8");
  } catch {
    return fail(2, `cannot read ${path}`);
  }
}

/**
 * The `version` of the `[package]` table in a Cargo manifest. Only that
 * table counts: dependency tables also carry `version` keys.
 * @example cargoVersion('[package]\nversion = "1.3.0"\n') === "1.3.0"
 */
function cargoVersion(text, path) {
  const table = /^\[package\]\s*$([\s\S]*?)(?=^\[|(?![\s\S]))/m.exec(text);
  const version = table && /^version\s*=\s*"([^"]+)"/m.exec(table[1]);
  if (!version) {
    fail(2, `no [package] version in ${path}`);
  }
  return version[1];
}

/** The top-level `version` of an extension manifest. */
function manifestVersion(text, path) {
  let manifest;
  try {
    manifest = JSON.parse(text);
  } catch {
    fail(2, `${path} is not valid JSON`);
  }
  if (typeof manifest?.version !== "string" || manifest.version === "") {
    fail(2, `no version in ${path}`);
  }
  return manifest.version;
}

const options = parseArguments(process.argv.slice(2));
const compiler = cargoVersion(read(options.cargo), options.cargo);
const extension = manifestVersion(read(options.manifest), options.manifest);

if (compiler !== extension) {
  fail(1, `compiler version ${compiler} does not match extension manifest ${extension}`);
}
const tagPrefix = "refs/tags/";
if (options.ref.startsWith(tagPrefix)) {
  const tag = options.ref.slice(tagPrefix.length);
  if (tag !== `v${compiler}`) {
    fail(1, `tag ${tag} does not match version ${compiler}`);
  }
} else if (options.publish) {
  fail(1, `publishing requires a release tag, got ${options.ref}`);
}

console.log(`release-version: ${compiler}`);
if (process.env.GITHUB_OUTPUT) {
  appendFileSync(process.env.GITHUB_OUTPUT, `version=${compiler}\n`);
}
