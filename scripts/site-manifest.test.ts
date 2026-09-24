// The manifest builders: what they keep, what they leave out, and what they
// refuse to write. The page and the app's updater link to whatever these
// produce, so a draft — or a file that is not really there — must never make
// it through.

import { describe, expect, it } from "vitest";
import {
  changelogFrom,
  emptyChangelog,
  latestFrom,
  updaterForMirror,
  validateChangelog,
  validateManifest,
  type GithubRelease,
  type UpdaterManifest,
} from "./site-manifest.ts";

const asset = (name: string, size = 1024, digest: string | null = null) => ({ name, size, digest });

const release = (over: Partial<GithubRelease> = {}): GithubRelease => ({
  tag_name: "v0.1.0",
  name: "Jokbet v0.1.0",
  draft: false,
  prerelease: false,
  published_at: "2026-09-24T10:00:00Z",
  body: "First release.\n\n- waves\n- counts\n",
  html_url: "https://github.com/sparkjokerben/jokbet/releases/tag/v0.1.0",
  assets: [
    asset("Jokbet_0.1.0_aarch64.dmg"),
    asset("Jokbet_0.1.0_x64.dmg"),
    asset("Jokbet_0.1.0_x64-setup.exe"),
    asset("Jokbet_0.1.0_amd64.AppImage"),
    asset("latest.json"), // Tauri's updater manifest: not an installer
    asset("Jokbet_0.1.0_aarch64.dmg.sig"), // nor a signature
  ],
  ...over,
});

describe("the changelog builder", () => {
  it("keeps the installers and nothing else", () => {
    const log = changelogFrom([release()], "2026-09-24T12:00:00Z");
    expect(log.generatedAt).toBe("2026-09-24T12:00:00Z");
    expect(log.releases).toHaveLength(1);
    expect(log.releases[0].files.map((f) => f.platform)).toEqual(["macos-aarch64", "macos-x64", "windows-x64", "linux-appimage"]);
    expect(log.releases[0].files[0].name).toBe("Jokbet_0.1.0_aarch64.dmg");
  });

  it("never lists a draft", () => {
    expect(changelogFrom([release({ draft: true })]).releases).toHaveLength(0);
  });

  it("carries the notes, the tag and the date across", () => {
    const [r] = changelogFrom([release()]).releases;
    expect(r.version).toBe("0.1.0");
    expect(r.tag).toBe("v0.1.0");
    expect(r.notes).toContain("- counts");
    expect(r.pubDate).toBe("2026-09-24T10:00:00Z");
    expect(r.url).toContain("/releases/tag/v0.1.0");
  });

  it("takes GitHub's own digest as the checksum, when it has one", () => {
    const sha = "a".repeat(64);
    const log = changelogFrom([release({ assets: [asset("Jokbet_0.1.0_aarch64.dmg", 2048, `sha256:${sha}`)] })]);
    expect(log.releases[0].files[0]).toMatchObject({ size: 2048, sha256: sha });
  });

  it("keeps a release whose assets are all missing, rather than dropping the version", () => {
    const log = changelogFrom([release({ assets: [] })]);
    expect(log.releases).toHaveLength(1);
    expect(log.releases[0].files).toEqual([]);
  });

  it("lists the newest first", () => {
    const older = release({ tag_name: "v0.0.9", published_at: "2026-01-01T00:00:00Z" });
    const newer = release({ tag_name: "v0.1.0", published_at: "2026-09-24T10:00:00Z" });
    expect(changelogFrom([older, newer]).releases.map((r) => r.version)).toEqual(["0.1.0", "0.0.9"]);
  });

  it("writes a changelog that passes its own check", () => {
    expect(validateChangelog(changelogFrom([release()]))).toEqual([]);
    expect(validateChangelog(emptyChangelog())).toEqual([]);
  });

  it("refuses an installer that does not match its version", () => {
    const log = changelogFrom([release()]);
    log.releases[0].files[0].name = "Jokbet_0.0.9_aarch64.dmg";
    expect(validateChangelog(log)).toContainEqual(expect.stringContaining("is not the macos-aarch64 of 0.1.0"));
  });

  it("refuses a platform nobody ships", () => {
    const log = changelogFrom([release()]);
    log.releases[0].files.push({ platform: "haiku", name: "Jokbet_0.1.0_haiku.bin", size: 1, sha256: null });
    expect(validateChangelog(log)).toContainEqual(expect.stringContaining("not a known platform"));
  });

  it("refuses a changelog of the wrong shape", () => {
    expect(validateChangelog({ schema: 2, releases: [] })).toContainEqual(expect.stringContaining("schema must be 1"));
    expect(validateChangelog({ schema: 1, releases: undefined })).toContainEqual(
      expect.stringContaining("releases must be an array"),
    );
  });
});

describe("the newest release, from the releases API", () => {
  it("is GitHub's latest: neither a draft nor a prerelease", () => {
    const releases = [
      release({ tag_name: "v0.3.0", draft: true, published_at: "2026-12-01T00:00:00Z" }),
      release({ tag_name: "v0.2.0-beta.1", prerelease: true, published_at: "2026-11-01T00:00:00Z" }),
      release({ tag_name: "v0.1.0", published_at: "2026-09-24T10:00:00Z" }),
    ];
    expect(latestFrom(releases)).toMatchObject({ version: "0.1.0", tag: "v0.1.0", draft: false });
  });

  it("takes sizes and GitHub's checksums from the API, and passes its own check", () => {
    const sha = "b".repeat(64);
    const assets = ["aarch64.dmg", "x64.dmg", "x64-setup.exe", "x64_en-US.msi", "amd64.AppImage", "amd64.deb"].map((end) =>
      asset(`Jokbet_0.1.0_${end}`, 4096, `sha256:${sha}`),
    );
    const manifest = latestFrom([release({ assets })]);
    expect(manifest.files["linux-deb"]).toEqual({ name: "Jokbet_0.1.0_amd64.deb", size: 4096, sha256: sha });
    expect(validateManifest(manifest)).toEqual([]);
  });

  it("is the empty manifest before the first release", () => {
    expect(latestFrom([])).toMatchObject({ version: null, files: {} });
    expect(validateManifest(latestFrom([release({ draft: true })]))).toEqual([]);
  });
});

describe("the updater manifest the mirror serves", () => {
  const github = "https://github.com/sparkjokerben/jokbet/releases/download/v0.1.0";
  const tauri = (): UpdaterManifest => ({
    version: "0.1.0",
    notes: "notes",
    pub_date: "2026-09-24T05:24:53.748Z",
    platforms: {
      "darwin-aarch64": { url: `${github}/Jokbet_aarch64.app.tar.gz`, signature: "sig-a" },
      "windows-x86_64-nsis": { url: `${github}/Jokbet_0.1.0_x64-setup.exe`, signature: "sig-w" },
    },
  });
  const present = new Set(["Jokbet_aarch64.app.tar.gz", "Jokbet_0.1.0_x64-setup.exe", "latest.json"]);

  it("points every download at the same file on the mirror, and keeps the signatures", () => {
    const out = updaterForMirror(tauri(), "v0.1.0", present);
    expect(out.platforms["darwin-aarch64"]).toEqual({
      url: "https://jokbet.jokerben.top/dl/v0.1.0/Jokbet_aarch64.app.tar.gz",
      signature: "sig-a",
    });
    expect(out.platforms["windows-x86_64-nsis"].url).toBe("https://jokbet.jokerben.top/dl/v0.1.0/Jokbet_0.1.0_x64-setup.exe");
    expect(out).toMatchObject({ version: "0.1.0", notes: "notes", pub_date: "2026-09-24T05:24:53.748Z" });
  });

  it("refuses a manifest for another version", () => {
    expect(() => updaterForMirror(tauri(), "v0.2.0", present)).toThrow(/for 0.1.0, not v0.2.0/);
  });

  it("refuses a URL that is not this release's file on GitHub", () => {
    const m = tauri();
    m.platforms["darwin-aarch64"].url = "https://example.com/Jokbet_aarch64.app.tar.gz";
    expect(() => updaterForMirror(m, "v0.1.0", present)).toThrow(/not a file of v0.1.0 on GitHub/);
  });

  it("refuses a file the release does not have", () => {
    expect(() => updaterForMirror(tauri(), "v0.1.0", new Set(["latest.json"]))).toThrow(/not among the release's files/);
  });

  it("refuses an empty manifest", () => {
    expect(() => updaterForMirror({ ...tauri(), platforms: {} }, "v0.1.0", present)).toThrow(/no platforms/);
  });
});
