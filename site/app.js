// Every page's behaviour, keyed on <body data-page>: the pet (which runs the
// app's own code, see src/pet/web.ts) and the language switch everywhere; the
// one-line install on the home page, the installer list on /download and the
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
/** @typedef {{ day: string, count: number }} CountedDay */
/** One row of the download counts: what the mirror served, what GitHub
 * counted, and their sum. A half nothing could read is null — which is not the
 * same as a half that counted nothing. */
/** @typedef {{ mirror: number | null, github: number | null, total: number }} Split */
/** @typedef {{ platform: string | null } & Split} PlatformCount */
/** @typedef {{ version: string } & Split} VersionCount */
/** @typedef {{ name: string, platform: string | null, version: string } & Split} FileCount */
/** @typedef {{ schema: number, generatedAt: string, total: number | null,
 *   mirror: { total: number, first: string | null, last: string | null, byDay: CountedDay[],
 *     byClient: { client: string, count: number }[],
 *     byCountry: { country: string, count: number }[] } | null,
 *   github: { total: number, raw: number, updatedAt: string | null } | null,
 *   byPlatform: PlatformCount[], byVersion: VersionCount[], byFile: FileCount[],
 *   sources: { mirror: string, github: string } }} Counts */
/** @typedef {"zh" | "en"} Lang */
/** @typedef {{ input: (a: "typing" | "click" | "scroll") => void, setKeysPerSecond: (n: number) => void,
 *   point: (x: number, y: number) => void, setLang: (l: Lang) => void,
 *   setBlocked: (b: string | null) => void, demo: (d: string) => void }} Pet */

const html = document.documentElement;
const RELEASES = "https://github.com/sparkjokerben/jokbet/releases";
const PAGES = /** @type {const} */ (["home", "download", "changelog", "about", "stats", "404"]);
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
      stats: "下载统计 — Jokbet",
      "404": "404 — Jokbet",
    },
    platforms: "macOS 11+ · Windows · Linux（X11）",
    unreleased: "还没有正式发布的版本",
    copied: "已复制",
    copy: "复制",
    installLine: "推荐：命令安装",
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
    checksumNone: "未公布校验值",
    prerelease: "预发布",
    downloads: "已被下载 {n} 次",
    downloadsHint: "本站镜像与 GitHub 的合计，含应用自动更新",
    statsReach: "本站镜像 {mirror} 次 · GitHub {github} 次",
    statsReachMirror: "本站镜像 {mirror} 次 · GitHub 的数字暂时读不到",
    statsReachGithub: "GitHub {github} 次 · 本站镜像暂时读不到",
    statsSince: "镜像自 {first} 起计数，最后一条是 {last}（按 UTC 分日）。",
    statsUpdated: "GitHub 的数字更新于 {when}，每天刷新一次。",
    statsRaw: "GitHub 自己报告 {raw} 次；发布流程取走的、以及签名文件上的次数都没有计入合计。",
    statsBars: "最近 {days} 天的每日下载次数，最多的一天 {most} 次。",
    statsEmpty: "暂时读不到下载统计。",
    statsOther: "其他",
    unknown: "未知",
    clients: { browser: "浏览器", script: "命令安装", updater: "应用自动更新", other: "其他" },
    macos: {
      "macos-aarch64-zip": "macOS · Apple 芯片（aarch64）",
      "macos-aarch64": "macOS · Apple 芯片（aarch64）",
      "macos-x64-zip": "macOS · Intel（x64）",
      "macos-x64": "macOS · Intel（x64）",
    },
    windows: {
      "windows-x64": "Windows · 安装程序",
      "windows-x64-msi": "Windows · MSI",
      "windows-x64-portable": "Windows · 便携版（免安装）",
    },
    linux: { "linux-appimage": "Linux · AppImage", "linux-deb": "Linux · deb" },
  },
  en: {
    titles: {
      home: "Jokbet — a pixel pet in the corner of your screen",
      download: "Download — Jokbet",
      changelog: "Changelog — Jokbet",
      about: "About — Jokbet",
      stats: "Download counts — Jokbet",
      "404": "404 — Jokbet",
    },
    platforms: "macOS 11+ · Windows · Linux (X11)",
    unreleased: "no release yet",
    copied: "Copied",
    copy: "Copy",
    installLine: "Recommended: one line",
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
    checksumNone: "no published checksum",
    prerelease: "prerelease",
    downloads: "{n} downloads",
    downloadsHint: "This site's mirror and GitHub together, app updates included",
    statsReach: "{mirror} from this site's mirror · {github} from GitHub",
    statsReachMirror: "{mirror} from this site's mirror · GitHub's number cannot be read right now",
    statsReachGithub: "{github} from GitHub · this site's count cannot be read right now",
    statsSince: "The mirror has counted since {first}; its last day is {last} (UTC days).",
    statsUpdated: "GitHub's numbers were refreshed {when}, once a day.",
    statsRaw: "GitHub itself reports {raw}: what publishing the release fetched, and what a signature file collected, are not counted.",
    statsBars: "Downloads a day over the last {days} days; the busiest day had {most}.",
    statsEmpty: "The download counts cannot be read right now.",
    statsOther: "other",
    unknown: "unknown",
    clients: { browser: "a browser", script: "a command line", updater: "the app updating itself", other: "something else" },
    macos: {
      "macos-aarch64-zip": "macOS · Apple silicon (aarch64)",
      "macos-aarch64": "macOS · Apple silicon (aarch64)",
      "macos-x64-zip": "macOS · Intel (x64)",
      "macos-x64": "macOS · Intel (x64)",
    },
    windows: {
      "windows-x64": "Windows · installer",
      "windows-x64-msi": "Windows · MSI",
      "windows-x64-portable": "Windows · portable (no install)",
    },
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

/** The platforms, in the order they are listed. Each Mac build comes as a zip
 * (from 0.1.2 on) and as a disk image; the zip is first, being stopped by
 * Gatekeeper once where the disk image is stopped twice. */
const GROUPS = /** @type {{ key: "macos" | "windows" | "linux", ids: string[] }[]} */ ([
  { key: "macos", ids: ["macos-aarch64-zip", "macos-aarch64", "macos-x64-zip", "macos-x64"] },
  { key: "windows", ids: ["windows-x64", "windows-x64-msi", "windows-x64-portable"] },
  { key: "linux", ids: ["linux-appimage", "linux-deb"] },
]);

/** @param {number | null} n */
const bytes = (n) => (n === null ? "" : n >= 1048576 ? `${(n / 1048576).toFixed(1)} MB` : `${Math.ceil(n / 1024)} KB`);
/** @param {string | null} iso */
const day = (iso) => (iso ? iso.slice(0, 10) : "");
/** A number as the visitor's language writes them. @param {number} n */
const number = (n) => new Intl.NumberFormat(lang() === "zh" ? "zh-CN" : "en-US").format(n);
/** A sentence of T, with its {placeholders} filled in.
 * @param {string} template @param {Record<string, string | number>} [values] */
const say = (template, values = {}) => template.replace(/\{(\w+)\}/g, (_, key) => String(values[key] ?? ""));

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

/** Which file a button offers for a platform: a Mac gets the zip when the
 * release has one, and the disk image before that.
 * @param {string} id */
const offered = (id) => (manifest?.files?.[`${id}-zip`] ? `${id}-zip` : id);

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
  const meta = el(
    "span",
    [file.name.slice(file.name.lastIndexOf(".") + 1), bytes(file.size)].filter(Boolean).join(" · "),
  );
  meta.className = "file-meta";
  link.append(label, meta);
  const from = el("a", T[lang()].github);
  from.className = "file-mirror";
  from.href = github;
  item.append(link, from);
  // Shown even when nothing was published: a release without a checksum is
  // worth saying, not hiding — for an un-signed build that value is the only
  // published integrity anchor.
  const sha = el("details");
  sha.className = "file-sha";
  sha.append(el("summary", T[lang()].checksum), el("code", file.sha256 || T[lang()].checksumNone));
  item.append(sha);
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

/** The one-line installs, as `/install.sh` and `/install.ps1` are: the same two
 * the download page leads with. The project would rather people install this
 * way — a terminal download is not quarantined and not marked, so neither
 * Gatekeeper nor SmartScreen stops the first launch — so the home page offers
 * the line for the visitor's own system, and the download page for the rest. */
const UNIX_INSTALL = "curl -fsSL https://jokbet.jokerben.top/install.sh | sh";
const WINDOWS_INSTALL = "irm https://jokbet.jokerben.top/install.ps1 | iex";

/** The home page's call to action: the one-line install for the visitor's own
 * system, or, when their system cannot be guessed or has no build yet, a way
 * to the page that lists them all. */
function renderCta() {
  const cta = $("cta");
  if (!cta || !manifest) return;
  const t = T[lang()];
  const hasFiles = !!manifest.version && Object.keys(manifest.files ?? {}).length > 0;
  const pick = guess();
  cta.replaceChildren();
  const file = pick ? manifest.files?.[offered(pick)] : undefined;
  if (!pick || !file) {
    // Nothing to guess from: the download page lists every platform, or, with
    // no release at all, GitHub is where one will appear.
    const link = el("a", hasFiles ? t.everyPlatform : t.releases);
    link.className = "btn btn-primary";
    link.href = hasFiles ? "/download" : RELEASES;
    cta.append(link);
    return;
  }

  const hint = el("p", t.installLine);
  hint.className = "note";
  const command = el("div");
  command.className = "command";
  const pre = el("pre");
  const code = el("code", pick === "windows-x64" ? WINDOWS_INSTALL : UNIX_INSTALL);
  code.id = "cta-command";
  pre.append(code);
  const copy = el("button", t.copy);
  copy.className = "btn";
  copy.type = "button";
  copy.setAttribute("data-copy", "cta-command");
  command.append(pre, copy);
  cta.append(hint, command);
  // The line was just built: its copy button needs its wiring.
  wireCopy();
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

// --- the download counts ----------------------------------------------------

/** How many days the chart draws: the month anyone recognizes. The Worker
 * sends three of them. */
const DAYS = 30;

/** @type {Counts | null} */
let counts = null;

/** What the page has when the Worker cannot answer at all. No numbers is not
 * the same as no downloads, and every page that shows one says which it is. */
const NO_COUNTS = /** @type {Counts} */ ({
  schema: 1,
  generatedAt: "",
  total: null,
  mirror: null,
  github: null,
  byPlatform: [],
  byVersion: [],
  byFile: [],
  sources: { mirror: "unavailable", github: "unavailable" },
});

/** The counts, once a visit, drawn wherever they belong. */
async function loadCounts() {
  if (!counts) {
    counts = /** @type {Counts | null} */ (await loadJson("/api/downloads.json")) ?? NO_COUNTS;
  }
  drawBadge();
  drawStats();
}

/** The number of downloads, where a page has room for it. Nothing at all until
 * there is a number: a page claiming "0 downloads" because it could not read
 * one would be worse than a page saying nothing. */
function drawBadge() {
  const host = $("download-total");
  if (!host) return;
  const t = T[lang()];
  const total = counts?.total ?? null;
  host.replaceChildren();
  host.hidden = total === null;
  if (total === null) return;
  const badge = el("span", say(t.downloads, { n: number(total) }));
  badge.className = "stat";
  badge.title = t.downloadsHint;
  host.append(badge);
}

/** One half nothing could read is a dash, never a zero. @param {number | null} n */
const half = (n) => (n === null ? "—" : number(n));

/** A country code as a name, where the browser has one to give.
 * @param {string} code */
function countryName(code) {
  try {
    return new Intl.DisplayNames([lang() === "zh" ? "zh-CN" : "en"], { type: "region" }).of(code) ?? code;
  } catch {
    return code;
  }
}

/** A table body: the first cell of a row names it, the rest are numbers.
 * @param {HTMLElement | null} host @param {string[][]} rows */
function fillRows(host, rows) {
  if (!host) return;
  host.replaceChildren();
  for (const cells of rows) {
    const tr = el("tr");
    cells.forEach((cell, i) => {
      if (!i) {
        const th = el("th", cell);
        th.setAttribute("scope", "row");
        tr.append(th);
        return;
      }
      const td = el("td", cell);
      td.className = "n";
      tr.append(td);
    });
    host.append(tr);
  }
}

/** The whole of /stats: the number, the two halves it is made of, the days, and
 * the tables. The page's own words are in its markup; the numbers are here. */
function drawStats() {
  const body = $("stats-body");
  if (!body || !counts) return;
  const t = T[lang()];
  const total = counts.total;
  const empty = $("stats-empty");
  if (empty) {
    empty.hidden = total !== null;
    if (total === null) empty.replaceChildren(el("span", t.statsEmpty));
  }
  body.hidden = total === null;
  if (total === null) return;

  const mirror = counts.mirror;
  const github = counts.github;
  $("stats-total")?.replaceChildren(el("span", say(t.downloads, { n: number(total) })));
  const reach = mirror && github
    ? say(t.statsReach, { mirror: number(mirror.total), github: number(github.total) })
    : mirror
      ? say(t.statsReachMirror, { mirror: number(mirror.total) })
      : github
        ? say(t.statsReachGithub, { github: number(github.total) })
        : "";
  $("stats-split")?.replaceChildren(el("span", reach));

  // Where the numbers come from, and how old each half is. GitHub's own count
  // is always the larger one — it counts downloads nobody made.
  const about = $("stats-when");
  if (about) {
    const lines = [];
    if (mirror?.first) lines.push(say(t.statsSince, { first: mirror.first, last: mirror.last ?? mirror.first }));
    if (github) {
      lines.push(say(t.statsUpdated, { when: github.updatedAt ? day(github.updatedAt) : t.unknown }));
      if (github.raw !== github.total) lines.push(say(t.statsRaw, { raw: number(github.raw) }));
    }
    about.replaceChildren();
    for (const line of lines) {
      const p = el("p", line);
      p.className = "note";
      about.append(p);
    }
  }

  const days = (mirror?.byDay ?? []).slice(-DAYS);
  const bars = $("stats-bars");
  if (bars) {
    const most = Math.max(1, ...days.map((point) => point.count));
    bars.replaceChildren();
    for (const point of days) {
      const bar = el("div");
      bar.className = "bar";
      // A day with something on it is never too small to see.
      bar.style.height = point.count ? `${Math.max(3, Math.round((point.count / most) * 100))}%` : "0";
      bar.title = `${point.day} · ${number(point.count)}`;
      bars.append(bar);
    }
    bars.setAttribute("aria-label", say(t.statsBars, { days: number(days.length), most: number(most) }));
    $("stats-from")?.replaceChildren(el("span", days[0]?.day ?? ""));
    $("stats-to")?.replaceChildren(el("span", days[days.length - 1]?.day ?? ""));
  }
  fillRows($("stats-days"), days.map((point) => [point.day, number(point.count)]));

  // The mirror has these two; GitHub publishes a running total per file and
  // nothing else, so there is no day, no client and no country to give.
  const who = $("stats-who");
  const timeline = $("stats-days-block");
  if (who) who.hidden = !mirror;
  if (timeline) timeline.hidden = !mirror;
  const client = /** @param {string} id */ (id) => /** @type {Record<string, string>} */ (t.clients)[id] ?? id;
  fillRows($("stats-clients"), (mirror?.byClient ?? []).map((row) => [client(row.client), number(row.count)]));
  fillRows($("stats-countries"), (mirror?.byCountry ?? []).map((row) => [countryName(row.country), number(row.count)]));

  // A Mac build ships as a zip and as a disk image, and the page names both the
  // same — so the platform table adds such a pair together, and the file table
  // below is where the two are told apart.
  /** @type {Map<string, { mirror: number | null, github: number | null }>} */
  const sums = new Map();
  for (const row of counts.byPlatform) {
    const label = row.platform ? platformLabel(lang(), row.platform) : t.statsOther;
    const so = sums.get(label) ?? { mirror: 0, github: 0 };
    sums.set(label, {
      mirror: so.mirror === null || row.mirror === null ? null : so.mirror + row.mirror,
      github: so.github === null || row.github === null ? null : so.github + row.github,
    });
  }
  /** @param {{ mirror: number | null, github: number | null }} of */
  const together = (of) => (of.mirror ?? 0) + (of.github ?? 0);
  fillRows(
    $("stats-platforms"),
    [...sums]
      .sort(([, a], [, b]) => together(b) - together(a))
      .map(([label, of]) => [label, half(of.mirror), half(of.github), number(together(of))]),
  );
  fillRows(
    $("stats-versions"),
    counts.byVersion.map((row) => [`v${row.version}`, half(row.mirror), half(row.github), number(row.total)]),
  );
  fillRows(
    $("stats-files"),
    counts.byFile.map((row) => [
      row.name,
      row.platform ? platformLabel(lang(), row.platform) : t.statsOther,
      `v${row.version}`,
      half(row.mirror),
      half(row.github),
      number(row.total),
    ]),
  );
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
    idleAnims: ["soccer", "lookAround", "walk"],
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
  // Reading down the page keeps it awake, the way scrolling does in the app.
  window.addEventListener("wheel", () => pet?.input("scroll"), { passive: true });

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
  drawBadge();
  drawStats();
}

// --- copy buttons -----------------------------------------------------------

/** A button with data-copy="<id>" copies that element's text, and says so. */
function wireCopy() {
  for (const button of document.querySelectorAll("[data-copy]")) {
    // The home page builds its copy button again on every render: wiring the
    // ones already wired would copy twice per click.
    const element = /** @type {HTMLElement} */ (button);
    if (element.dataset.copyWired) continue;
    element.dataset.copyWired = "1";
    const source = $(button.getAttribute("data-copy") ?? "");
    if (!source || !navigator.clipboard) {
      // Nothing to copy with: the text can still be selected by hand.
      /** @type {HTMLElement} */ (button).hidden = true;
      continue;
    }
    /** @type {ReturnType<typeof setTimeout> | undefined} */
    let timer;
    button.addEventListener("click", async () => {
      try {
        await navigator.clipboard.writeText(source.textContent ?? "");
      } catch {
        return;
      }
      clearTimeout(timer);
      button.setAttribute("data-copied", T[lang()].copied);
      timer = setTimeout(() => button.removeAttribute("data-copied"), 1600);
    });
  }
}

// --- start ------------------------------------------------------------------

async function main() {
  for (const button of document.querySelectorAll("[data-set-lang]")) {
    button.addEventListener("click", () => setLang(/** @type {Lang} */ (button.getAttribute("data-set-lang")), true));
  }
  setLang(lang());
  wireCopy();
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
    // The number of downloads, beside the version it belongs to.
    await loadCounts();
  } else if (page === "stats") {
    await loadCounts();
  }
  await petting;
}

void main();
