// The download manifest the site reads. Shared by the release job (which builds
// it from the published assets), CI (which validates the baked fallback) and the
// download function (which needs the repo and the file names).

export const REPO = "sparkjokerben/jokbet";

/** One installer, as the download page offers it. */
export interface ManifestFile {
  name: string;
  /** Bytes; null in the baked fallback, where nothing was downloaded. */
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

/** The installers, in the order the page lists them. */
export const PLATFORMS: { id: string; file: (version: string) => string; bundle: string }[] = [
  { id: "macos-aarch64", file: (v) => `Jokbet_${v}_aarch64.dmg`, bundle: "dmg" },
  { id: "macos-x64", file: (v) => `Jokbet_${v}_x64.dmg`, bundle: "dmg" },
  { id: "windows-x64", file: (v) => `Jokbet_${v}_x64-setup.exe`, bundle: "nsis" },
  { id: "windows-x64-msi", file: (v) => `Jokbet_${v}_x64_en-US.msi`, bundle: "msi" },
  { id: "linux-appimage", file: (v) => `Jokbet_${v}_amd64.AppImage`, bundle: "appimage" },
  { id: "linux-deb", file: (v) => `Jokbet_${v}_amd64.deb`, bundle: "deb" },
];

export const PLATFORM_IDS = PLATFORMS.map((p) => p.id);

export const downloadBase = (tag: string | null) => (tag ? `https://github.com/${REPO}/releases/download/${tag}` : null);
export const releaseUrl = (tag: string | null) => (tag ? `https://github.com/${REPO}/releases/tag/${tag}` : null);

/** The manifest a version-less or asset-less install starts from. */
export function emptyManifest(version: string | null = null): Manifest {
  const tag = version ? `v${version}` : null;
  const files: Record<string, ManifestFile> = {};
  if (version) {
    for (const p of PLATFORMS) files[p.id] = { name: p.file(version), size: null, sha256: null };
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
  for (const p of PLATFORMS) {
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
    if (!PLATFORMS.some((p) => p.id === id)) bad.push(`files.${id} is not a known platform`);
  }
  if (man.tag !== `v${man.version}`) bad.push(`tag ${man.tag} does not match version ${man.version}`);
  if (man.downloadBase && !man.downloadBase.endsWith(`/download/${man.tag}`)) {
    bad.push(`downloadBase ${man.downloadBase} does not point at ${man.tag}`);
  }
  return bad;
}
