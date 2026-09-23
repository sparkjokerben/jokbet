// The download manifest the site reads. Shared by the release job (which builds
// it from the published assets), CI (which validates the baked fallback) and the
// download function (which needs the repo and the file names).

export const REPO = "sparkjokerben/jokerben-desktop-pet";

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
  { id: "macos-aarch64", file: (v) => `jokerben-desktop-pet_${v}_aarch64.dmg`, bundle: "dmg" },
  { id: "macos-x64", file: (v) => `jokerben-desktop-pet_${v}_x64.dmg`, bundle: "dmg" },
  { id: "windows-x64", file: (v) => `jokerben-desktop-pet_${v}_x64-setup.exe`, bundle: "nsis" },
  { id: "windows-x64-msi", file: (v) => `jokerben-desktop-pet_${v}_x64_en-US.msi`, bundle: "msi" },
  { id: "linux-appimage", file: (v) => `jokerben-desktop-pet_${v}_amd64.AppImage`, bundle: "appimage" },
  { id: "linux-deb", file: (v) => `jokerben-desktop-pet_${v}_amd64.deb`, bundle: "deb" },
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
