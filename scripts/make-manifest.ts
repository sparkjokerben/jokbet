// Builds and validates the JSON the site reads.
//
//   node scripts/make-manifest.ts full --tag v0.1.0 --dir release-assets \
//     --release-json release.json [--out latest.json]
//   node scripts/make-manifest.ts baked [--version 0.1.0] --out site/latest.baked.json
//   node scripts/make-manifest.ts changelog [--releases-json file | --stdin] \
//     [--out changelog.json]
//   node scripts/make-manifest.ts check [path…]   (default site/latest.baked.json)
//
// `full` runs in the release job: every platform must be there, or it fails —
// that is how a change in Tauri's asset names shows up as a red job instead of
// a silently missing download. `changelog` runs there too, from the releases
// API's answer (`gh api "repos/:owner/:repo/releases?per_page=100"`), so the
// changelog page has every published version and not just the newest.

import { createHash } from "node:crypto";
import { readFileSync, statSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import {
  PLATFORMS,
  changelogFrom,
  downloadBase,
  emptyManifest,
  releaseUrl,
  validateChangelog,
  validateManifest,
  type Changelog,
  type GithubRelease,
  type Manifest,
} from "./site-manifest.ts";

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

function changelog(): Changelog {
  const from = flag("releases-json");
  // `--stdin` is how the release job pipes `gh api … |` straight in.
  const raw = from ? readFileSync(from, "utf8") : readFileSync(0, "utf8");
  const releases = JSON.parse(raw) as GithubRelease[];
  if (!Array.isArray(releases)) throw new Error("expected the releases API's array of releases");
  return changelogFrom(releases, flag("generated-at") ?? new Date().toISOString());
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
const out =
  flag("out") ?? (mode === "full" ? "latest.json" : mode === "changelog" ? "changelog.json" : "site/latest.baked.json");

switch (mode) {
  case "full": {
    const manifest = full();
    write(manifest, validateManifest(manifest));
    break;
  }
  case "baked": {
    const manifest = baked();
    write(manifest, validateManifest(manifest));
    break;
  }
  case "changelog": {
    const log = changelog();
    write(log, validateChangelog(log));
    const files = log.releases.reduce((n, r) => n + r.files.length, 0);
    console.log(`${out}: ${log.releases.length} releases, ${files} installers`);
    break;
  }
  case "check": {
    let bad = 0;
    for (const file of positional.slice(1).length ? positional.slice(1) : ["site/latest.baked.json"]) {
      const body = JSON.parse(readFileSync(file, "utf8")) as { releases?: unknown };
      const problems = Array.isArray(body.releases) ? validateChangelog(body) : validateManifest(body);
      if (problems.length) {
        console.error(`${file} is not usable:\n  ${problems.join("\n  ")}`);
        bad = problems.length;
        continue;
      }
      console.log(`${file} is valid`);
    }
    process.exit(bad ? 1 : 0);
    break;
  }
  default:
    console.error(
      "usage: node scripts/make-manifest.ts full|baked|changelog|check [--tag vX.Y.Z] [--dir dir] [--out file]",
    );
    process.exit(1);
}

function write(value: unknown, problems: string[]) {
  if (problems.length) {
    console.error(`refusing to write a manifest that is not usable:\n  ${problems.join("\n  ")}`);
    process.exit(1);
  }
  writeFileSync(out, JSON.stringify(value, null, 1) + "\n");
  const manifest = value as Manifest;
  const sizes = Object.values(manifest.files ?? {})
    .map((f) => `${f.name} ${f.size}`)
    .join("\n  ");
  if (sizes) console.log(`${out}: ${manifest.version} (${Object.keys(manifest.files).length} files)\n  ${sizes}`);
}
