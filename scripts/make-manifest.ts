// Builds and validates the JSON the site and the app read.
//
// In the release job, one file per manifest the Worker serves from the bucket:
//
//   node scripts/make-manifest.ts full --tag v0.1.0 --dir release-assets \
//     --release-json release.json --out manifests/latest.json
//   gh api "repos/:owner/:repo/releases?per_page=100" \
//     | node scripts/make-manifest.ts changelog --stdin --out manifests/releases.json
//   node scripts/make-manifest.ts update --tag v0.1.0 --dir release-assets \
//     --out manifests/update.json
//
// After a release, to refresh the copies compiled into the Worker (what the
// site shows when the bucket cannot answer) — `npm run site:fallback`:
//
//   gh api "repos/:owner/:repo/releases?per_page=100" \
//     | node scripts/make-manifest.ts fallback --stdin --dir worker/fallback
//
// And in CI: node scripts/make-manifest.ts check [path…]
//
// `full` and `update` fail unless every file they name is really there — that
// is how a change in Tauri's asset names shows up as a red job instead of a
// download that is silently missing.

import { createHash } from "node:crypto";
import { readdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import {
  changelogFrom,
  downloadBase,
  latestFrom,
  platformsFor,
  releaseUrl,
  updaterForMirror,
  validateChangelog,
  validateManifest,
  type GithubRelease,
  type Manifest,
  type UpdaterManifest,
} from "./site-manifest.ts";

const FALLBACK_DIR = "worker/fallback";

function flag(name: string): string | undefined {
  const i = process.argv.indexOf(`--${name}`);
  return i === -1 ? undefined : process.argv[i + 1];
}

function required(name: string): string {
  const value = flag(name);
  if (!value) throw new Error(`--${name} is required`);
  return value;
}

/** The newest release's manifest, from the files the release job downloaded. */
function full(): Manifest {
  const tag = required("tag");
  const version = tag.replace(/^v/, "");
  const dir = flag("dir") ?? "release-assets";
  const releaseJson = flag("release-json");
  const release: { body?: string; createdAt?: string; publishedAt?: string; isDraft?: boolean } = releaseJson
    ? JSON.parse(readFileSync(releaseJson, "utf8"))
    : {};

  const files: Manifest["files"] = {};
  const missing: string[] = [];
  for (const p of platformsFor(version)) {
    const name = p.file(version);
    const path = join(dir, name);
    let size: number;
    try {
      size = statSync(path).size;
    } catch {
      missing.push(name);
      continue;
    }
    const sha256 = createHash("sha256").update(readFileSync(path)).digest("hex");
    files[p.id] = { name, size, sha256 };
  }
  if (missing.length) throw new Error(`missing installers:\n  ${missing.join("\n  ")}`);

  return {
    schema: 1,
    version,
    tag,
    // When it was published, as the changelog says, not when its draft was made.
    pubDate: release.publishedAt || release.createdAt || new Date().toISOString(),
    draft: release.isDraft ?? null,
    notes: (release.body ?? "").trim() || null,
    releaseUrl: releaseUrl(tag),
    downloadBase: downloadBase(tag),
    files,
  };
}

/** The release's own updater manifest, pointed at the mirror. */
function update(): UpdaterManifest {
  const tag = required("tag");
  const dir = flag("dir") ?? "release-assets";
  const source = JSON.parse(readFileSync(flag("in") ?? join(dir, "latest.json"), "utf8")) as UpdaterManifest;
  return updaterForMirror(source, tag, new Set(readdirSync(dir)));
}

/** The releases API's answer, from `--releases-json` or piped in with `--stdin`. */
function releasesFromApi(): GithubRelease[] {
  const from = flag("releases-json");
  const raw = from ? readFileSync(from, "utf8") : readFileSync(0, "utf8");
  const releases = JSON.parse(raw) as GithubRelease[];
  if (!Array.isArray(releases)) throw new Error("expected the releases API's array of releases");
  return releases;
}

const argv = process.argv.slice(2);
const positional: string[] = [];
for (let i = 0; i < argv.length; i++) {
  // Flags are read with flag(); this only has to know which words are paths.
  // A value-less flag (--stdin) must not swallow the flag after it.
  if (argv[i].startsWith("--")) {
    if (argv[i + 1] && !argv[i + 1].startsWith("--")) i++;
  } else {
    positional.push(argv[i]);
  }
}

const mode = positional[0] ?? "";
const generatedAt = flag("generated-at") ?? new Date().toISOString();

switch (mode) {
  case "full":
    write(flag("out") ?? "latest.json", full(), validateManifest);
    break;
  case "changelog":
    write(flag("out") ?? "releases.json", changelogFrom(releasesFromApi(), generatedAt), validateChangelog);
    break;
  case "update":
    write(flag("out") ?? "update.json", update(), () => []);
    break;
  case "fallback": {
    const dir = flag("dir") ?? FALLBACK_DIR;
    const releases = releasesFromApi();
    write(join(dir, "latest.json"), latestFrom(releases), validateManifest);
    write(join(dir, "releases.json"), changelogFrom(releases, generatedAt), validateChangelog);
    break;
  }
  case "check": {
    let bad = 0;
    const files = positional.slice(1);
    for (const file of files.length ? files : [join(FALLBACK_DIR, "latest.json"), join(FALLBACK_DIR, "releases.json")]) {
      const body = JSON.parse(readFileSync(file, "utf8")) as { releases?: unknown };
      const problems = Array.isArray(body.releases) ? validateChangelog(body) : validateManifest(body);
      if (problems.length) {
        console.error(`${file} is not usable:\n  ${problems.join("\n  ")}`);
        bad += problems.length;
        continue;
      }
      console.log(`${file} is valid`);
    }
    process.exit(bad ? 1 : 0);
    break;
  }
  default:
    console.error("usage: node scripts/make-manifest.ts full|changelog|update|fallback|check …  (see the top of this file)");
    process.exit(1);
}

function write<T>(out: string, value: T, validate: (value: T) => string[]) {
  const problems = validate(value);
  if (problems.length) {
    console.error(`refusing to write ${out}, which is not usable:\n  ${problems.join("\n  ")}`);
    process.exit(1);
  }
  writeFileSync(out, JSON.stringify(value, null, 1) + "\n");
  console.log(`${out}: ${summary(value)}`);
}

function summary(value: unknown): string {
  const v = value as { version?: string | null; files?: object; releases?: { files: unknown[] }[]; platforms?: object };
  if (Array.isArray(v.releases)) {
    return `${v.releases.length} releases, ${v.releases.reduce((n, r) => n + r.files.length, 0)} installers`;
  }
  if (v.platforms) return `${v.version}, ${Object.keys(v.platforms).length} updater targets`;
  return v.version ? `${v.version}, ${Object.keys(v.files ?? {}).length} installers` : "no release yet";
}
