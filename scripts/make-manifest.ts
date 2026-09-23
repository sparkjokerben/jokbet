// Builds and validates the download manifest that the site reads.
//
//   node scripts/make-manifest.ts full --tag v0.1.0 --dir release-assets \
//     --release-json release.json [--out latest.json]
//   node scripts/make-manifest.ts baked [--version 0.1.0] --out site/latest.baked.json
//   node scripts/make-manifest.ts check [path]          (default site/latest.baked.json)
//
// `full` runs in the release job: every platform must be there, or it fails —
// that is how a change in Tauri's asset names shows up as a red job instead of
// a silently missing download.

import { createHash } from "node:crypto";
import { readFileSync, statSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { PLATFORMS, downloadBase, emptyManifest, releaseUrl, validateManifest, type Manifest } from "./site-manifest.ts";

function flag(name: string): string | undefined {
  const i = process.argv.indexOf(`--${name}`);
  return i === -1 ? undefined : process.argv[i + 1];
}

function full(): Manifest {
  const tag = flag("tag");
  if (!tag) throw new Error("--tag is required");
  const version = tag.replace(/^v/, "");
  const dir = flag("dir") ?? "release-assets";
  const releaseJson = flag("release-json");
  const release: { body?: string; createdAt?: string; isDraft?: boolean } = releaseJson
    ? JSON.parse(readFileSync(releaseJson, "utf8"))
    : {};

  const files: Manifest["files"] = {};
  const missing: string[] = [];
  for (const p of PLATFORMS) {
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
    pubDate: release.createdAt ?? new Date().toISOString(),
    draft: release.isDraft ?? null,
    notes: (release.body ?? "").trim() || null,
    releaseUrl: releaseUrl(tag),
    downloadBase: downloadBase(tag),
    files,
  };
}

function baked(): Manifest {
  const version = flag("version") ?? JSON.parse(readFileSync("package.json", "utf8")).version;
  const m = emptyManifest(version);
  // Notes for the next release are unknown until it happens; the page then
  // shows the version and the GitHub links, which is all the fallback needs.
  return { ...m, notes: flag("notes") ?? null };
}

const argv = process.argv.slice(2);
const positional: string[] = [];
const flags: Record<string, string> = {};
for (let i = 0; i < argv.length; i++) {
  if (argv[i].startsWith("--")) flags[argv[i].slice(2)] = argv[++i] ?? "";
  else positional.push(argv[i]);
}

const mode = positional[0] ?? "";
const out = flag("out") ?? (mode === "full" ? "latest.json" : "site/latest.baked.json");

let manifest: Manifest;
switch (mode) {
  case "full":
    manifest = full();
    break;
  case "baked":
    manifest = baked();
    break;
  case "check": {
    const file = positional[1] ?? "site/latest.baked.json";
    const problems = validateManifest(JSON.parse(readFileSync(file, "utf8")));
    if (problems.length) {
      console.error(`${file} is not usable:\n  ${problems.join("\n  ")}`);
      process.exit(1);
    }
    console.log(`${file} is valid`);
    process.exit(0);
  }
  default:
    console.error("usage: node scripts/make-manifest.ts full|baked|check [--tag vX.Y.Z] [--dir dir] [--out file]");
    process.exit(1);
}

const problems = validateManifest(manifest);
if (problems.length) {
  console.error(`refusing to write a manifest that is not usable:\n  ${problems.join("\n  ")}`);
  process.exit(1);
}
writeFileSync(out, JSON.stringify(manifest, null, 1) + "\n");
const sizes = Object.values(manifest.files).map((f) => `${f.name} ${f.size}`).join("\n  ");
console.log(`${out}: ${manifest.version} (${Object.keys(manifest.files).length} files)\n  ${sizes}`);
