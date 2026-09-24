// Every page's behaviour, keyed on <body data-page>: the pet (which runs the
// app's own code, see src/pet/web.ts) and the language switch everywhere; the
// download buttons on the home page, the installer list on /download and the
// release list on /changelog, built from the Worker's /api/ manifests.
// Everything else is in the HTML.

/** @typedef {{ name: string, size: number | null, sha256: string | null }} ManifestFile */
/** @typedef {{ schema: number, version: string | null, tag: string | null, pubDate: string | null,
 *   draft: boolean | null, notes: string | null, releaseUrl: string | null, downloadBase: string | null,
 *   source?: string, files: Record<string, ManifestFile> }} Manifest */
/** @typedef {{ platform: string, name: string, size: number | null }} ReleaseFile */
/** @typedef {{ version: string, tag: string, name: string | null, pubDate: string | null,
 *   prerelease: boolean, notes: string | null, url: string | null, files: ReleaseFile[] }} Release */
/** @typedef {{ schema: number, generatedAt: string | null, releases: Release[], source?: string }} Changelog */
/** @typedef {"zh" | "en"} Lang */
/** @typedef {{ input: (a: "typing" | "click") => void, setKeysPerSecond: (n: number) => void,
 *   point: (x: number, y: number) => void, setLang: (l: Lang) => void,
 *   setBlocked: (b: string | null) => void, demo: (d: string) => void }} Pet */

const html = document.documentElement;
const RELEASES = "https://github.com/sparkjokerben/jokbet/releases";
const PAGES = /** @type {const} */ (["home", "download", "changelog", "about", "404"]);
/** Which page this is: the body says so. */
const page = PAGES.find((p) => p === document.body.dataset.page) ?? "home";

/** @param {string} id */
const $ = (id) => document.getElementById(id);
/** @template {keyof HTMLElementTagNameMap} K
 * @param {K} tag @param {string} [text] @returns {HTMLElementTagNameMap[K]} */
const el = (tag, text) => {
  const node = document.createElement(tag);
  if (text !== undefined) node.textContent = text;
  return node;
};

/** The page's own words: everything the JS builds, in both languages. */
const T = {
  zh: {
    titles: {
      home: "Jokbet — 屏幕角落的像素桌宠",
      download: "下载 — Jokbet",
      changelog: "更新日志 — Jokbet",
      about: "关于 — Jokbet",
      "404": "404 — Jokbet",
    },
    platforms: "macOS 11+ · Windows · Linux（X11）",
    unreleased: "还没有正式发布的版本",
    primary: { "macos-aarch64": "下载 macOS 版", "windows-x64": "下载 Windows 版", "linux-appimage": "下载 Linux 版" },
    altMac: "Intel 版",
    releases: "去 GitHub 下载",
    everyPlatform: "选择平台下载",
    released: "发布于",
    sourceR2: "下载由本站镜像提供",
    sourceGithub: "镜像暂时不可用，已改用 GitHub",
    empty: "还没有正式发布的版本",
    unreachable: "暂时读不到版本信息，所有版本都在 GitHub 上。",
    emptyHome: "第一个版本发布后，这里会列出各平台的安装包。",
    emptyHint: "第一个版本发布后，这里会自动列出每个版本的改动和安装包。",
    github: "GitHub 下载",
    checksum: "SHA-256 校验值",
    prerelease: "预发布",
    macos: { "macos-aarch64": "macOS · Apple 芯片（aarch64）", "macos-x64": "macOS · Intel（x64）" },
    windows: { "windows-x64": "Windows · 安装程序", "windows-x64-msi": "Windows · MSI" },
    linux: { "linux-appimage": "Linux · AppImage", "linux-deb": "Linux · deb" },
  },
  en: {
    titles: {
      home: "Jokbet — a pixel pet in the corner of your screen",
      download: "Download — Jokbet",
      changelog: "Changelog — Jokbet",
      about: "About — Jokbet",
      "404": "404 — Jokbet",
    },
    platforms: "macOS 11+ · Windows · Linux (X11)",
    unreleased: "no release yet",
    primary: { "macos-aarch64": "Download for macOS", "windows-x64": "Download for Windows", "linux-appimage": "Download for Linux" },
    altMac: "Intel build",
    releases: "Downloads on GitHub",
    everyPlatform: "Pick your platform",
    released: "Released",
    sourceR2: "Downloads come from this site's own mirror",
    sourceGithub: "The mirror is unavailable — downloads go to GitHub",
    empty: "No releases yet",
    unreachable: "The release list cannot be read right now; every version is on GitHub.",
    emptyHome: "The first release will list its installers here.",
    emptyHint: "The first release will list itself here, with its notes and its installers.",
    github: "On GitHub",
    checksum: "SHA-256",
    prerelease: "prerelease",
    macos: { "macos-aarch64": "macOS · Apple silicon (aarch64)", "macos-x64": "macOS · Intel (x64)" },
    windows: { "windows-x64": "Windows · installer", "windows-x64-msi": "Windows · MSI" },
    linux: { "linux-appimage": "Linux · AppImage", "linux-deb": "Linux · .deb" },
  },
};

const lang = () => (html.dataset.lang === "zh" ? "zh" : "en");
/** @param {Lang} l @param {string} id */
const platformLabel = (l, id) => {
  const t = T[l];
  for (const group of [t.macos, t.windows, t.linux]) {
    if (id in group) return /** @type {Record<string, string>} */ (group)[id];
  }
  return id;
};

/** The platforms, in the order they are listed. */
const GROUPS = /** @type {{ key: "macos" | "windows" | "linux", ids: string[] }[]} */ ([
  { key: "macos", ids: ["macos-aarch64", "macos-x64"] },
  { key: "windows", ids: ["windows-x64", "windows-x64-msi"] },
  { key: "linux", ids: ["linux-appimage", "linux-deb"] },
]);

/** @param {number | null} n */
const bytes = (n) => (n === null ? "" : n >= 1048576 ? `${(n / 1048576).toFixed(1)} MB` : `${Math.ceil(n / 1024)} KB`);
/** @param {string | null} iso */
const day = (iso) => (iso ? iso.slice(0, 10) : "");

// --- the release notes ------------------------------------------------------

/** Just enough Markdown for a release body: headings, rules, bullets, bold,
 * code and links. Built out of DOM nodes, so nothing in the notes can become
 * markup. */
/** @param {string} text */
function inline(text) {
  const frag = document.createDocumentFragment();
  const pattern = /(\*\*[^*]+\*\*|`[^`]+`|\[[^\]]+\]\(https?:\/\/[^)\s]+\))/g;
  let last = 0;
  for (const match of text.matchAll(pattern)) {
    if (match.index > last) frag.append(text.slice(last, match.index));
    const token = match[0];
    const link = token.match(/^\[([^\]]+)\]\((https?:\/\/[^)\s]+)\)$/);
    if (link) {
      const a = el("a", link[1]);
      a.href = link[2];
      frag.append(a);
    } else if (token.startsWith("**")) {
      frag.append(el("strong", token.slice(2, -2)));
    } else {
      frag.append(el("code", token.slice(1, -1)));
    }
    last = match.index + token.length;
  }
  if (last < text.length) frag.append(text.slice(last));
  return frag;
}

/** @param {string} text */
function notesToDom(text) {
  const root = el("div");
  /** @type {HTMLElement | null} */
  let list = null;
  for (const line of text.split("\n")) {
    // A thematic break, before the bullets: a line of nothing but - * or _.
    if (/^\s*(?:[-*_]\s*){3,}$/.test(line)) {
      root.append(el("hr"));
      list = null;
      continue;
    }
    const bullet = line.match(/^\s*[-*]\s+(.*)$/);
    if (bullet) {
      if (!list) {
        list = el("ul");
        root.append(list);
      }
      const item = el("li");
      item.append(inline(bullet[1]));
      list.append(item);
      continue;
    }
    list = null;
    const heading = line.match(/^#{1,6}\s+(.*)$/);
    if (heading) {
      const h = el("h4");
      h.append(inline(heading[1]));
      root.append(h);
      continue;
    }
    if (!line.trim()) continue;
    const p = el("p");
    p.append(inline(line));
    root.append(p);
  }
  return root;
}

// --- downloads --------------------------------------------------------------

/** @type {Manifest | null} */
let manifest = null;
/** @type {Changelog | null} */
let changelog = null;

/** Which build the visitor most likely wants; Apple silicon unless we know better. */
function guess() {
  const ua = navigator.userAgent;
  const platform =
    /** @type {{ userAgentData?: { platform?: string } }} */ (navigator).userAgentData?.platform ?? navigator.platform ?? "";
  if (/mac|iphone|ipad/i.test(platform) || /Mac OS X/.test(ua)) return "macos-aarch64";
  if (/win/i.test(platform) || /Windows/.test(ua)) return "windows-x64";
  if (/linux|x11/i.test(platform) || /Linux/.test(ua)) return "linux-appimage";
  return null;
}

/** A release file on this site's mirror, and the same file on GitHub.
 * @param {string} tag @param {string} name */
const mirrorHref = (tag, name) => `/dl/${tag}/${name}`;
/** @param {string} tag @param {string} name */
const githubHref = (tag, name) => `${RELEASES}/download/${tag}/${name}`;

/** Where the newest release's buttons point: the mirror, unless the page has
 * already been told the mirror is out — then straight to GitHub, one hop less.
 * (The mirror would send the browser there anyway.)
 * @param {ManifestFile} file */
function latestHref(file) {
  const tag = manifest?.tag ?? "";
  return manifest?.source === "r2" ? mirrorHref(tag, file.name) : githubHref(tag, file.name);
}

/** One row of an installer list.
 * @param {string} platform @param {{ name: string, size: number | null, sha256?: string | null }} file
 * @param {string} href @param {string} github
 */
function fileRow(platform, file, href, github) {
  const item = el("li");
  item.className = "file";
  const link = el("a");
  link.className = "file-main";
  link.href = href;
  link.setAttribute("download", "");
  link.dataset.platform = platform;
  const label = el("span", platformLabel(lang(), platform));
  label.className = "file-label";
  const meta = el("span", [file.name.slice(file.name.lastIndexOf(".") + 1), bytes(file.size)].filter(Boolean).join(" · "));
  meta.className = "file-meta";
  link.append(label, meta);
  const from = el("a", T[lang()].github);
  from.className = "file-mirror";
  from.href = github;
  item.append(link, from);
  if (file.sha256) {
    const sha = el("details");
    sha.className = "file-sha";
    sha.append(el("summary", T[lang()].checksum), el("code", file.sha256));
    item.append(sha);
  }
  return item;
}

/** Whatever of the newest release this page shows: the buttons and the line
 * under them (home), the installer list (/download). */
function renderRelease() {
  if (!manifest) return;
  renderCta();
  renderReleaseLine();
  renderFiles();
}

/** The installer list on /download. */
function renderFiles() {
  const t = T[lang()];
  const list = $("files");
  const empty = $("release-empty");
  const head = $("release-head");
  if (!list || !empty || !head || !manifest) return;
  list.replaceChildren();

  const version = manifest.version ?? null;
  const tag = manifest.tag ?? "";
  const hasFiles = !!version && Object.keys(manifest.files ?? {}).length > 0;

  head.hidden = !hasFiles;
  empty.hidden = hasFiles;
  list.hidden = !hasFiles;
  if (!hasFiles) {
    const unreachable = manifest.source === "unavailable";
    empty.replaceChildren(el("span", unreachable ? t.unreachable : t.empty));
    const link = el("a", t.releases);
    link.href = RELEASES;
    const p = el("p");
    p.className = "note";
    p.append(link);
    if (!unreachable) {
      const hint = el("p", t.emptyHome);
      hint.className = "note";
      empty.append(hint);
    }
    empty.append(p);
    return;
  }

  const when = day(manifest.pubDate);
  $("release-version")?.replaceChildren(el("span", `v${version}`));
  $("release-when")?.replaceChildren(el("span", when ? `${t.released} ${when}` : ""));
  const source = $("release-source");
  if (source) {
    source.replaceChildren(el("span", manifest.source === "r2" ? t.sourceR2 : t.sourceGithub));
    source.className = manifest.source === "r2" ? "" : "warn";
  }

  const notes = $("notes-details");
  const body = $("notes");
  if (notes && body) {
    notes.hidden = !manifest.notes;
    body.replaceChildren(manifest.notes ? notesToDom(manifest.notes) : "");
  }

  for (const group of GROUPS) {
    for (const id of group.ids) {
      const file = manifest.files?.[id];
      if (!file) continue;
      list.append(fileRow(id, file, latestHref(file), githubHref(tag, file.name)));
    }
  }
}

/** The home page's buttons: the build this visitor probably needs, and a way
 * to all of them. */
function renderCta() {
  const cta = $("cta");
  if (!cta || !manifest) return;
  const t = T[lang()];
  cta.replaceChildren();
  const hasFiles = !!manifest.version && Object.keys(manifest.files ?? {}).length > 0;
  const pick = guess();
  const file = pick ? manifest.files?.[pick] : undefined;
  if (!pick || !file) {
    // Nothing to guess from: the download page lists every platform, or, with
    // no release at all, GitHub is where one will appear.
    const link = el("a", hasFiles ? t.everyPlatform : t.releases);
    link.className = "btn btn-primary";
    link.href = hasFiles ? "/download" : RELEASES;
    cta.append(link);
    return;
  }
  const ids = pick === "macos-aarch64" && manifest.files?.["macos-x64"] ? ["macos-aarch64", "macos-x64"] : [pick];
  ids.forEach((id, index) => {
    const f = manifest?.files?.[id];
    if (!f) return;
    const link = el("a", index === 0 ? /** @type {Record<string, string>} */ (t.primary)[id] ?? id : t.altMac);
    link.className = index === 0 ? "btn btn-primary" : "btn";
    link.href = latestHref(f);
    link.setAttribute("download", "");
    link.dataset.platform = id;
    const size = el("span", bytes(f.size));
    size.className = "sub";
    if (f.size) link.append(size);
    cta.append(link);
  });
}

/** The line under the buttons: which version, and what it runs on. */
function renderReleaseLine() {
  const line = $("release-line");
  if (!line) return;
  const t = T[lang()];
  const version = manifest?.version ? `v${manifest.version}` : t.unreleased;
  line.replaceChildren(el("span", `${version} · ${t.platforms}`));
}

// --- the changelog ----------------------------------------------------------

function renderChangelog() {
  const host = $("releases");
  const empty = $("releases-empty");
  // Until the list arrives the markup's own "reading…" line stands.
  if (!host || !empty || !changelog) return;
  const t = T[lang()];
  const releases = changelog.releases;
  host.replaceChildren();
  empty.hidden = releases.length > 0;

  if (!releases.length) {
    const unreachable = changelog.source === "unavailable";
    empty.replaceChildren(el("span", unreachable ? t.unreachable : t.empty));
    const link = el("a", t.releases);
    link.href = RELEASES;
    const p = el("p");
    p.className = "note";
    p.append(link);
    if (!unreachable) {
      const hint = el("p", t.emptyHint);
      hint.className = "note";
      empty.append(hint);
    }
    empty.append(p);
    return;
  }

  for (const release of releases) {
    const section = el("section");
    section.className = "release";
    const head = el("div");
    head.className = "release-head";
    const h2 = el("h2", `v${release.version}`);
    if (release.name) h2.title = release.name;
    head.append(h2);
    const when = day(release.pubDate);
    if (when) {
      const w = el("span", when);
      w.className = "when";
      head.append(w);
    }
    if (release.prerelease) {
      const badge = el("span", t.prerelease);
      badge.className = "badge";
      head.append(badge);
    }
    const from = el("a", "GitHub");
    from.className = "all";
    from.href = release.url ?? RELEASES;
    head.append(from);
    section.append(head);

    if (release.notes) {
      const notes = el("div");
      notes.className = "notes";
      notes.append(notesToDom(release.notes));
      section.append(notes);
    }

    if (release.files.length) {
      const list = el("ul");
      list.className = "files";
      for (const file of release.files) {
        list.append(fileRow(file.platform, file, mirrorHref(release.tag, file.name), githubHref(release.tag, file.name)));
      }
      section.append(list);
    }
    host.append(section);
  }
}

/** One of the Worker's manifests, or null if the Worker could not be reached.
 * (The Worker has its own fallback for the bucket, so null means the site
 * itself is having trouble.)
 * @param {string} url */
async function loadJson(url) {
  try {
    const response = await fetch(url, { cache: "no-store" });
    return response.ok ? await response.json() : null;
  } catch {
    return null;
  }
}

// --- the pet ----------------------------------------------------------------

/** @type {Pet | null} */
let pet = null;

async function wirePet() {
  const host = $("pet");
  if (!host) return;
  const still = window.matchMedia?.("(prefers-reduced-motion: reduce)").matches ?? false;
  /** @type {{ createPet: (h: HTMLElement, o: Record<string, unknown>) => Pet }} */
  let bundled;
  try {
    // The app's own pet code, bundled for the browser by scripts/build-site-pet.ts.
    const source = "/assets/pet.js";
    bundled = await import(/* @vite-ignore */ source);
  } catch {
    // No bundle (offline, or a very old browser): the poster stays, and the
    // page is the same page without the pet moving.
    return;
  }
  // Take the poster out before the live pet arrives, so the two never overlap.
  host.replaceChildren();
  pet = bundled.createPet(host, {
    scale: 6,
    still,
    lang: lang(),
    idleAnim: "soccer",
    clickAnim: "wave",
    doubleClickAnim: "hearts",
    sleepAfterMs: 60_000,
    // No number over its head here: the page is not counting anything for you,
    // it is showing what the app does.
    counter: false,
    // Anywhere on the page, not just the column it starts in.
    bounds: () => new DOMRect(0, 0, window.innerWidth, window.innerHeight),
  });

  /** The keystrokes of the last second, which set the pace of its typing. */
  /** @type {number[]} */
  let recent = [];

  window.addEventListener("keydown", () => {
    const now = performance.now();
    recent = recent.filter((t) => now - t < 1000);
    recent.push(now);
    pet?.setKeysPerSecond(recent.length);
    pet?.input("typing");
  });
  document.addEventListener("pointerdown", () => {
    pet?.input("click");
  });

  // Its eyes follow the pointer, one look per frame at most.
  let where = /** @type {[number, number] | null} */ (null);
  let queued = false;
  document.addEventListener("pointermove", (e) => {
    where = [e.clientX, e.clientY];
    if (queued) return;
    queued = true;
    requestAnimationFrame(() => {
      queued = false;
      if (where) pet?.point(where[0], where[1]);
    });
  });

  for (const chip of document.querySelectorAll("[data-demo]")) {
    chip.addEventListener("click", () => pet?.demo(/** @type {HTMLElement} */ (chip).dataset.demo ?? "wave"));
  }
  for (const button of document.querySelectorAll("[data-block]")) {
    button.addEventListener("click", () => pet?.setBlocked(/** @type {HTMLElement} */ (button).dataset.block ?? null));
  }
}

// --- language ---------------------------------------------------------------

/** @param {Lang} next @param {boolean} [chosen] the visitor picked it */
function setLang(next, chosen = false) {
  html.dataset.lang = next;
  html.lang = next === "zh" ? "zh-Hans" : "en";
  // The title in the markup is bilingual, which is what a crawler should see;
  // it follows the language only once someone actually picks one.
  if (chosen) document.title = T[next].titles[page];
  try {
    localStorage.setItem("jokbet.lang", next);
  } catch {
    // storage disabled: the choice just will not be remembered
  }
  for (const button of document.querySelectorAll("[data-set-lang]")) {
    button.setAttribute("aria-pressed", String(button.getAttribute("data-set-lang") === next));
  }
  pet?.setLang(next);
  renderRelease();
  renderChangelog();
}

// --- start ------------------------------------------------------------------

async function main() {
  for (const button of document.querySelectorAll("[data-set-lang]")) {
    button.addEventListener("click", () => setLang(/** @type {Lang} */ (button.getAttribute("data-set-lang")), true));
  }
  setLang(lang());
  // The pet and the data are fetched together; neither waits for the other.
  const petting = wirePet();
  if (page === "changelog") {
    changelog = /** @type {Changelog | null} */ (await loadJson("/api/releases.json")) ?? {
      schema: 1,
      generatedAt: null,
      releases: [],
      source: "unavailable",
    };
    renderChangelog();
  } else if (page === "home" || page === "download") {
    manifest = /** @type {Manifest | null} */ (await loadJson("/api/latest.json")) ?? {
      schema: 1,
      version: null,
      tag: null,
      pubDate: null,
      draft: null,
      notes: null,
      releaseUrl: null,
      downloadBase: null,
      files: {},
      source: "unavailable",
    };
    renderRelease();
  }
  await petting;
}

void main();
