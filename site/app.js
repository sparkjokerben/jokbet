// The page's behaviour: the mascot's flipbooks, the download list built from
// the manifest (with its source shown honestly), the platform guess behind the
// main button, and the language switch. Everything else is in the HTML.

/** @typedef {{ name: string, size: number | null, sha256: string | null }} ManifestFile */
/** @typedef {{ schema: number, version: string | null, tag: string | null, pubDate: string | null,
 *   draft: boolean | null, notes: string | null, releaseUrl: string | null, downloadBase: string | null,
 *   source?: string, files: Record<string, ManifestFile> }} Manifest */
/** @typedef {{ file: string, frames: number, ms: number[], loop: boolean }} SpriteAnim */
/** @typedef {{ cell: number[], animations: Record<string, SpriteAnim> }} SpriteData */

const html = document.documentElement;

/** The languages the page carries; the JS-built parts live here. */
const T = {
  zh: {
    primary: { "macos-aarch64": "下载 macOS 版", "windows-x64": "下载 Windows 版", "linux-appimage": "下载 Linux 版" },
    altMac: "Intel 版",
    all: "所有平台",
    pick: "选择你的平台",
    version: "版本",
    date: "发布于",
    sourceR2: "下载由本站镜像提供",
    sourceGithub: "镜像暂时不可用，已切换到 GitHub 下载源",
    sourceNone: "还没有正式发布的版本，可以先到 GitHub Releases 看看。",
    notes: "本次更新",
    releases: "GitHub Releases",
    github: "GitHub 下载",
    checksum: "SHA-256 校验值",
    macos: { "macos-aarch64": "macOS · Apple 芯片 (aarch64)", "macos-x64": "macOS · Intel 芯片 (x64)" },
    windows: { "windows-x64": "Windows · 安装程序", "windows-x64-msi": "Windows · MSI 安装包" },
    linux: { "linux-appimage": "Linux · AppImage（自动更新）", "linux-deb": "Linux · deb 包" },
    poke: "戳它试试",
  },
  en: {
    primary: { "macos-aarch64": "Download for macOS", "windows-x64": "Download for Windows", "linux-appimage": "Download for Linux" },
    altMac: "Intel build",
    all: "All platforms",
    pick: "Pick your platform",
    version: "Version",
    date: "Released",
    sourceR2: "Downloads come from this site's own mirror",
    sourceGithub: "The mirror is unavailable — downloads go straight to GitHub",
    sourceNone: "Nothing is published yet; the GitHub releases page is the place to look.",
    notes: "What changed",
    releases: "GitHub Releases",
    github: "Download from GitHub",
    checksum: "SHA-256",
    macos: { "macos-aarch64": "macOS · Apple silicon (aarch64)", "macos-x64": "macOS · Intel (x64)" },
    windows: { "windows-x64": "Windows · installer", "windows-x64-msi": "Windows · MSI package" },
    linux: { "linux-appimage": "Linux · AppImage (self-updating)", "linux-deb": "Linux · .deb" },
    poke: "Poke it",
  },
};

const lang = () => (html.dataset.lang === "zh" ? "zh" : "en");

/** The platforms, in the order they are listed, with the label each one gets. */
/** @type {{ key: "macos" | "windows" | "linux", ids: string[] }[]} */
const GROUPS = [
  { key: "macos", ids: ["macos-aarch64", "macos-x64"] },
  { key: "windows", ids: ["windows-x64", "windows-x64-msi"] },
  { key: "linux", ids: ["linux-appimage", "linux-deb"] },
];

/** @param {number | null} n */
/** Where a visitor ends up when this site has nothing to offer yet. */
const RELEASES = "https://github.com/sparkjokerben/jokbet/releases";

/** @param {number | null} n */
const bytes = (n) => (n === null ? "" : n >= 1048576 ? `${(n / 1048576).toFixed(1)} MB` : `${Math.ceil(n / 1024)} KB`);

// --- the mascot -------------------------------------------------------------

/** @type {SpriteData | null} */
let sprites = null;
/** @type {Map<Element, number>} */
const timers = new Map();

const still = () => window.matchMedia?.("(prefers-reduced-motion: reduce)").matches ?? false;

/** Plays one animation in an element, once, or looping. */
/**
 * @param {HTMLElement} element
 * @param {string} name
 * @param {boolean} [loop]
 */
function play(element, name, loop = false) {
  const anim = sprites?.animations[name];
  if (!anim) return;
  clearTimeout(timers.get(element));
  const [width, height] = sprites ? sprites.cell : [160, 104];
  element.style.backgroundImage = `url(/assets/${anim.file})`;
  element.style.width = `${width}px`;
  element.style.height = `${height}px`;
  element.setAttribute("data-playing", name);
  if (still()) {
    element.style.backgroundPosition = "0 0";
    return;
  }
  let frame = 0;
  const step = () => {
    element.style.backgroundPosition = `${-frame * width}px 0`;
    const next = frame + 1 < anim.frames ? frame + 1 : loop ? 0 : -1;
    timers.set(
      element,
      window.setTimeout(() => {
        if (next < 0) {
          play(element, "idle", true);
          return;
        }
        frame = next;
        step();
      }, anim.ms[frame]),
    );
  };
  step();
}

/** One reaction at a time: a poke while the wave is running is dropped.
 * @param {HTMLElement} element
 * @param {string} name
 */
function react(element, name) {
  if (element.getAttribute("data-playing") === name) return;
  play(element, name);
}

function wirePet() {
  for (const element of /** @type {NodeListOf<HTMLElement>} */ (document.querySelectorAll("[data-flipbook]"))) {
    const name = element.getAttribute("data-flipbook") ?? "idle";
    play(element, name, element.hasAttribute("data-flipbook-loop") || name === "idle");
    element.addEventListener("pointerenter", () => {
      if (element.getAttribute("data-playing") === "idle") react(element, "wave");
    });
    element.addEventListener("click", () => react(element, "hearts"));
  }
}

// --- downloads --------------------------------------------------------------

/** @type {Manifest | null} */
let manifest = null;

/** Which build the visitor most likely wants; Apple silicon unless we know better. */
function guess() {
  const ua = navigator.userAgent;
  const platform = /** @type {{ platform?: string, userAgentData?: { platform?: string } }} */ (navigator).userAgentData?.platform ?? navigator.platform ?? "";
  if (/mac|iphone|ipad/i.test(platform) || /Mac OS X/.test(ua)) return "macos-aarch64";
  if (/win/i.test(platform) || /Windows/.test(ua)) return "windows-x64";
  if (/linux|x11/i.test(platform) || /Linux/.test(ua)) return "linux-appimage";
  return null;
}

/** The URL a download should use: our mirror, or GitHub when the mirror is out.
 * @param {ManifestFile} file
 */
function hrefFor(file) {
  const version = manifest?.tag?.replace(/^v/, "") ?? "";
  if (manifest?.source === "r2") return `/dl/${file.name}?v=${version}`;
  return `${manifest?.downloadBase ?? ""}/${file.name}`;
}

function renderDownloads() {
  const list = document.getElementById("files");
  const status = document.getElementById("download-status");
  const details = document.getElementById("notes-details");
  const cta = document.getElementById("cta-alt");
  if (!list || !status || !cta) return;
  const t = T[lang()];
  list.replaceChildren();

  if (!manifest?.version || !Object.keys(manifest.files).length) {
    status.textContent = t.sourceNone;
    status.dataset.state = "none";
    if (details) details.hidden = true;
    cta.hidden = true;
    return;
  }

  const version = manifest.version;
  status.replaceChildren();
  const head = document.createElement("span");
  head.className = "status-version";
  const when = manifest.pubDate ? `${t.date} ${manifest.pubDate.slice(0, 10)}` : "";
  head.textContent = when
    ? lang() === "zh"
      ? `${t.version} ${version}（${when}）`
      : `${t.version} ${version} (${when})`
    : `${t.version} ${version}`;
  const source = document.createElement("span");
  source.className = "status-source";
  source.textContent = manifest.source === "r2" ? t.sourceR2 : t.sourceGithub;
  // A space between them for screen readers; flexbox ignores the text node.
  status.append(head, document.createTextNode(" "), source);
  status.dataset.state = manifest.source === "r2" ? "mirror" : "github";
  if (details) {
    details.hidden = !manifest.notes;
    const body = document.getElementById("notes-body");
    if (body) body.textContent = manifest.notes ?? "";
  }

  for (const group of GROUPS) {
    for (const id of group.ids) {
      const file = manifest.files[id];
      if (!file) continue;
      const labels = /** @type {Record<string, string>} */ (t[group.key]);
      const label = labels[id] ?? id;
      const item = document.createElement("li");
      item.className = "file";

      const link = document.createElement("a");
      link.className = "file-main";
      link.href = hrefFor(file);
      link.setAttribute("download", "");
      link.dataset.platform = id;
      const name = document.createElement("span");
      name.className = "file-label";
      name.textContent = label;
      const meta = document.createElement("span");
      meta.className = "file-meta";
      const kind = file.name.slice(file.name.lastIndexOf("."));
      meta.textContent = [kind, bytes(file.size)].filter(Boolean).join(lang() === "zh" ? "，" : ", ");
      link.append(name, meta);
      item.append(link);

      const mirror = document.createElement("a");
      mirror.className = "file-mirror";
      mirror.href = `${manifest.downloadBase}/${file.name}`;
      mirror.textContent = t.github;
      item.append(mirror);

      if (file.sha256) {
        const sha = document.createElement("details");
        sha.className = "file-sha";
        const summary = document.createElement("summary");
        summary.textContent = t.checksum;
        const value = document.createElement("code");
        value.textContent = file.sha256;
        sha.append(summary, value);
        item.append(sha);
      }
      list.append(item);
    }
  }
}

/** The hero button, which offers what this visitor probably needs. */
function renderCta() {
  const cta = document.getElementById("cta-alt");
  if (!cta) return;
  const t = T[lang()];
  const pick = guess();
  cta.replaceChildren();
  const primary = pick && manifest?.files[pick] ? pick : null;
  const ids = primary === "macos-aarch64" ? ["macos-aarch64", "macos-x64"] : primary ? [primary] : [];
  if (!ids.length) {
    const link = document.createElement("a");
    link.className = "btn btn-primary";
    // Nothing on the mirror to point at (no release yet, or none for this
    // platform): GitHub always has the answer.
    link.href = RELEASES;
    link.textContent = t.releases;
    cta.append(link);
    return;
  }
  ids.forEach((id, index) => {
    const file = manifest?.files[id];
    if (!file) return;
    const link = document.createElement("a");
    link.className = index === 0 ? "btn btn-primary" : "btn";
    link.href = hrefFor(file);
    link.setAttribute("download", "");
    link.dataset.platform = id;
    link.textContent = index === 0 ? /** @type {Record<string, string>} */ (t.primary)[id] ?? t.all : t.altMac;
    cta.append(link);
  });
}

async function loadManifest() {
  for (const url of ["/latest", "/latest.baked.json"]) {
    try {
      const response = await fetch(url, { cache: "no-store" });
      if (!response.ok) continue;
      const body = /** @type {Manifest} */ (await response.json());
      return { ...body, source: body.source ?? (url === "/latest" ? "r2" : "baked") };
    } catch {
      // try the next one
    }
  }
  return { schema: 1, version: null, tag: null, pubDate: null, draft: null, notes: null, releaseUrl: null, downloadBase: null, files: {}, source: "unavailable" };
}

// --- language ---------------------------------------------------------------

/** @param {string} next */
function setLang(next) {
  html.dataset.lang = next;
  html.lang = next === "zh" ? "zh-Hans" : "en";
  try {
    localStorage.setItem("jokbet.lang", next);
  } catch {
    // storage disabled: the choice just will not be remembered
  }
  for (const button of document.querySelectorAll("[data-set-lang]")) {
    button.setAttribute("aria-pressed", String(button.getAttribute("data-set-lang") === next));
  }
  renderCta();
  renderDownloads();
}

// --- start ------------------------------------------------------------------

async function main() {
  for (const button of document.querySelectorAll("[data-set-lang]")) {
    button.addEventListener("click", () => setLang(button.getAttribute("data-set-lang") ?? "en"));
  }
  setLang(lang());
  try {
    const response = await fetch("/assets/jokbet.json");
    if (response.ok) sprites = /** @type {SpriteData} */ (await response.json());
  } catch {
    // the mascot just stays on its first frame
  }
  wirePet();
  manifest = await loadManifest();
  renderCta();
  renderDownloads();
}

void main();
