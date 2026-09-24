// The manifests the site and the app read. Shared by the release job (which
// builds them from the published release), CI (which validates the fallback
// copies compiled into the Worker) and the tests.
//
// Every file has two addresses, one per host, with the same shape:
//
//   https://jokbet.jokerben.top/dl/<tag>/<name>                  the mirror
//   https://github.com/<repo>/releases/download/<tag>/<name>     GitHub
//
// The site's download links, the Worker's fallback and the app's updater all
// rely on that: each can go from one to the other without looking anything up.

export const REPO = "sparkjokerben/jokbet";
export const SITE = "https://jokbet.jokerben.top";

export const githubUrl = (tag: string, name: string) => `https://github.com/${REPO}/releases/download/${tag}/${name}`;
export const mirrorUrl = (tag: string, name: string) => `${SITE}/dl/${tag}/${name}`;

/** One installer, as the download page offers it. */
export interface ManifestFile {
  name: string;
  /** Bytes; null when nothing was there to measure (the empty manifest before a release). */
  size: number | null;
  sha256: string | null;
}

export interface Manifest {
  /** Bumped if the shape ever changes; the page ignores what it cannot read. */
  schema: 1;
  version: string | null;
  tag: string | null;
  pubDate: string | null;
  /** True while the GitHub release is still a draft (the fallback links 404 then). */
  draft: boolean | null;
  notes: string | null;
  releaseUrl: string | null;
  /** Where the GitHub copies live; the page falls back to these. */
  downloadBase: string | null;
  files: Record<string, ManifestFile>;
}

/** One installer every release ships (from `since` on, when it came later). */
export interface Platform {
  id: string;
  file: (version: string) => string;
  /** Where it comes from: a Tauri bundle, or "app" for the zip of the .app the release job makes. */
  bundle: string;
  /** The first version that ships it; the releases before it are not expected to. */
  since?: string;
}

/** The installers, in the order the page lists them. On macOS the zip comes
 * first: Gatekeeper stops the app inside once, where a disk image is stopped
 * itself and then the app again. */
export const PLATFORMS: Platform[] = [
  { id: "macos-aarch64-zip", file: (v) => `Jokbet_${v}_aarch64.app.zip`, bundle: "app", since: "0.1.2" },
  { id: "macos-aarch64", file: (v) => `Jokbet_${v}_aarch64.dmg`, bundle: "dmg" },
  { id: "macos-x64-zip", file: (v) => `Jokbet_${v}_x64.app.zip`, bundle: "app", since: "0.1.2" },
  { id: "macos-x64", file: (v) => `Jokbet_${v}_x64.dmg`, bundle: "dmg" },
  { id: "windows-x64", file: (v) => `Jokbet_${v}_x64-setup.exe`, bundle: "nsis" },
  { id: "windows-x64-msi", file: (v) => `Jokbet_${v}_x64_en-US.msi`, bundle: "msi" },
  { id: "linux-appimage", file: (v) => `Jokbet_${v}_amd64.AppImage`, bundle: "appimage" },
  { id: "linux-deb", file: (v) => `Jokbet_${v}_amd64.deb`, bundle: "deb" },
];

export const PLATFORM_IDS = PLATFORMS.map((p) => p.id);

/** -1, 0 or 1, comparing the numbers of two versions (a prerelease counts as its release). */
export function compareVersions(a: string, b: string): number {
  const parts = (v: string) => v.replace(/^v/, "").split(/[-+]/)[0].split(".").map(Number);
  const [x, y] = [parts(a), parts(b)];
  for (let i = 0; i < Math.max(x.length, y.length); i++) {
    const d = (x[i] ?? 0) - (y[i] ?? 0);
    if (d) return Math.sign(d);
  }
  return 0;
}

/** The installers a version ships. */
export const platformsFor = (version: string) =>
  PLATFORMS.filter((p) => !p.since || compareVersions(version, p.since) >= 0);

export const downloadBase = (tag: string | null) => (tag ? `https://github.com/${REPO}/releases/download/${tag}` : null);
export const releaseUrl = (tag: string | null) => (tag ? `https://github.com/${REPO}/releases/tag/${tag}` : null);

/** The manifest a version-less or asset-less install starts from. */
export function emptyManifest(version: string | null = null): Manifest {
  const tag = version ? `v${version}` : null;
  const files: Record<string, ManifestFile> = {};
  if (version) {
    for (const p of platformsFor(version)) files[p.id] = { name: p.file(version), size: null, sha256: null };
  }
  return {
    schema: 1,
    version,
    tag,
    pubDate: null,
    draft: null,
    notes: null,
    releaseUrl: releaseUrl(tag),
    downloadBase: downloadBase(tag),
    files,
  };
}

// --- the changelog ----------------------------------------------------------

/** One installer of one release, as the changelog page offers it. */
export interface ReleaseFile {
  platform: string;
  name: string;
  size: number | null;
  sha256: string | null;
}

/** One published release. */
export interface ReleaseRecord {
  version: string;
  tag: string;
  name: string | null;
  pubDate: string | null;
  prerelease: boolean;
  notes: string | null;
  url: string | null;
  files: ReleaseFile[];
}

/** Every release the changelog page lists, newest first. */
export interface Changelog {
  schema: 1;
  generatedAt: string | null;
  releases: ReleaseRecord[];
}

export const emptyChangelog = (): Changelog => ({ schema: 1, generatedAt: null, releases: [] });

/** The GitHub releases API's shape, as much of it as this site uses. */
export interface GithubRelease {
  tag_name?: string;
  name?: string | null;
  draft?: boolean;
  prerelease?: boolean;
  published_at?: string | null;
  created_at?: string | null;
  body?: string | null;
  html_url?: string | null;
  /** `digest` is GitHub's own sha256 of the asset, when it has one. */
  assets?: { name?: string; size?: number; digest?: string | null }[];
}

/**
 * The changelog the page reads, from the releases API's answer. Drafts are
 * left out: their assets are not reachable yet, so nothing may link to them.
 */
export function changelogFrom(releases: GithubRelease[], generatedAt = new Date().toISOString()): Changelog {
  const records: ReleaseRecord[] = [];
  for (const release of releases) {
    const tag = release.tag_name ?? "";
    if (!tag || release.draft) continue;
    const version = tag.replace(/^v/, "");
    const files: ReleaseFile[] = [];
    for (const platform of PLATFORMS) {
      const name = platform.file(version);
      const asset = release.assets?.find((a) => a.name === name);
      if (!asset) continue;
      files.push({
        platform: platform.id,
        name,
        size: typeof asset.size === "number" ? asset.size : null,
        sha256: asset.digest?.replace(/^sha256:/, "") ?? null,
      });
    }
    records.push({
      version,
      tag,
      name: release.name ?? null,
      pubDate: release.published_at ?? release.created_at ?? null,
      prerelease: !!release.prerelease,
      notes: (release.body ?? "").trim() || null,
      url: release.html_url ?? releaseUrl(tag),
      files,
    });
  }
  records.sort((a, b) => (b.pubDate ?? "").localeCompare(a.pubDate ?? ""));
  return { schema: 1, generatedAt, releases: records };
}

/**
 * The newest release the download buttons should offer, as the releases API
 * describes it: GitHub's "latest" — neither a draft nor a prerelease. Sizes and
 * checksums come from the API (GitHub's own `digest`), so this needs no
 * download; the release job builds its copy from the files themselves instead.
 */
export function latestFrom(releases: GithubRelease[]): Manifest {
  const newest = releases
    .filter((r) => r.tag_name && !r.draft && !r.prerelease)
    .sort((a, b) => (b.published_at ?? "").localeCompare(a.published_at ?? ""))[0];
  if (!newest?.tag_name) return emptyManifest();
  const tag = newest.tag_name;
  const version = tag.replace(/^v/, "");
  const files: Record<string, ManifestFile> = {};
  for (const platform of platformsFor(version)) {
    const name = platform.file(version);
    const asset = newest.assets?.find((a) => a.name === name);
    files[platform.id] = {
      name,
      size: typeof asset?.size === "number" ? asset.size : null,
      sha256: asset?.digest?.replace(/^sha256:/, "") ?? null,
    };
  }
  return {
    schema: 1,
    version,
    tag,
    pubDate: newest.published_at ?? newest.created_at ?? null,
    draft: false,
    notes: (newest.body ?? "").trim() || null,
    releaseUrl: releaseUrl(tag),
    downloadBase: downloadBase(tag),
    files,
  };
}

// --- the updater's manifest ---------------------------------------------------

/** Tauri's updater manifest, as tauri-action writes it into every release. */
export interface UpdaterManifest {
  version: string;
  notes?: string;
  pub_date?: string;
  platforms: Record<string, { url: string; signature: string }>;
}

/**
 * The updater manifest the mirror serves: the release's own, with every
 * download moved to the same file on the mirror. The signatures stay as they
 * are — they sign the file, not its address — so the copy on either host
 * verifies against them, which is what lets the app retry one on the other.
 *
 * Throws rather than write a manifest that points at nothing: every URL must be
 * this release's own file on GitHub, and that file must be among `present`
 * (what the release job is about to upload).
 */
export function updaterForMirror(manifest: UpdaterManifest, tag: string, present: ReadonlySet<string>): UpdaterManifest {
  if (manifest.version?.replace(/^v/, "") !== tag.replace(/^v/, "")) {
    throw new Error(`the updater manifest is for ${manifest.version}, not ${tag}`);
  }
  const entries = Object.entries(manifest.platforms ?? {});
  if (!entries.length) throw new Error("the updater manifest lists no platforms");
  const prefix = githubUrl(tag, "");
  const platforms: UpdaterManifest["platforms"] = {};
  for (const [target, { url, signature }] of entries) {
    const name = url.startsWith(prefix) ? url.slice(prefix.length) : "";
    if (!name || name.includes("/")) throw new Error(`${target}: ${url} is not a file of ${tag} on GitHub`);
    if (!present.has(name)) throw new Error(`${target}: ${name} is not among the release's files`);
    if (!signature) throw new Error(`${target}: no signature`);
    platforms[target] = { url: mirrorUrl(tag, name), signature };
  }
  return { ...manifest, platforms };
}

/** Everything wrong with a changelog, as messages; empty means it is usable. */
export function validateChangelog(c: unknown): string[] {
  const bad: string[] = [];
  const log = c as Partial<Changelog>;
  if (log?.schema !== 1) bad.push(`schema must be 1, got ${JSON.stringify(log?.schema)}`);
  if (!Array.isArray(log.releases)) return [...bad, "releases must be an array"];
  for (const release of log.releases) {
    const where = `releases[${release?.tag ?? "?"}]`;
    if (typeof release?.version !== "string" || !release.version) bad.push(`${where}: version must be a string`);
    if (typeof release.tag !== "string" || !release.tag) bad.push(`${where}: tag must be a string`);
    if (!Array.isArray(release.files)) {
      bad.push(`${where}: files must be an array`);
      continue;
    }
    for (const file of release.files) {
      const platform = PLATFORMS.find((p) => p.id === file.platform);
      if (!platform) {
        bad.push(`${where}: ${file.platform} is not a known platform`);
        continue;
      }
      if (file.name !== platform.file(release.version)) {
        bad.push(`${where}: ${file.name} is not the ${file.platform} of ${release.version}`);
      }
      if (file.size !== null && typeof file.size !== "number") bad.push(`${where}: ${file.name} has no size`);
      if (file.sha256 !== null && !/^[0-9a-f]{64}$/.test(String(file.sha256))) {
        bad.push(`${where}: ${file.name} has no sha256`);
      }
    }
  }
  return bad;
}

/** Everything wrong with a manifest, as messages; empty means it is usable. */
export function validateManifest(m: unknown): string[] {
  const bad: string[] = [];
  const man = m as Partial<Manifest>;
  if (man?.schema !== 1) bad.push(`schema must be 1, got ${JSON.stringify(man?.schema)}`);
  if (man.version === null) {
    if (Object.keys(man.files ?? {}).length) bad.push("a version-less manifest must not list files");
    return bad;
  }
  if (typeof man.version !== "string") return [...bad, "version must be a string or null"];
  for (const p of platformsFor(man.version)) {
    const f = man.files?.[p.id];
    if (!f) {
      bad.push(`files.${p.id} is missing`);
      continue;
    }
    if (f.name !== p.file(man.version)) {
      bad.push(`files.${p.id}.name is ${f.name}, expected ${p.file(man.version)}`);
    }
    if (f.size !== null && typeof f.size !== "number") bad.push(`files.${p.id}.size must be a number or null`);
    if (f.sha256 !== null && !/^[0-9a-f]{64}$/.test(String(f.sha256))) {
      bad.push(`files.${p.id}.sha256 is not a sha256`);
    }
  }
  for (const id of Object.keys(man.files ?? {})) {
    if (!platformsFor(man.version).some((p) => p.id === id)) bad.push(`files.${id} is not an installer of ${man.version}`);
  }
  if (man.tag !== `v${man.version}`) bad.push(`tag ${man.tag} does not match version ${man.version}`);
  if (man.downloadBase && !man.downloadBase.endsWith(`/download/${man.tag}`)) {
    bad.push(`downloadBase ${man.downloadBase} does not point at ${man.tag}`);
  }
  return bad;
}
