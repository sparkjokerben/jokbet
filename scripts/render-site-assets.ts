// Renders the website's assets from the very same sprite data the app draws,
// so the site can never drift from the pet:
//   node scripts/render-site-assets.ts [gen|check] [outDir]   (default: gen site/assets)
//
// Emits, into site/assets/: one transparent sprite sheet per animation the page
// flipbooks, a JSON with each animation's exact frame timings (the app's own
// timings; they are not uniform, so the page cannot use CSS steps()), the pixel
// wordmark as SVG, the favicons and app icons, and the Open Graph card.
//
// `check` re-renders in memory and compares pixel digests against the JSON, so
// a sprite change that was not regenerated fails CI instead of shipping.

import { createHash } from "node:crypto";
import { mkdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { compileGrid } from "../src/sprites/compile.ts";
import { ANIMS, GRID_H, GRID_W, PET_H, PET_W, PET_X, PET_Y, compose, frameRows } from "../src/sprites/jokbet.ts";
import { Canvas, encodePng, hex, paletteColor, type RGBA } from "./png.ts";

/** The page's flipbooks, and the scale each sheet is drawn at. The page shows a
 * sheet at `GRID_W * SCALE` CSS px, which keeps every source pixel square. */
export const SCALE = 6;
const ANIMATIONS = ["idle", "wave", "hearts", "soccer"] as const;

const PAPER: RGBA = hex("#f5f4f0");
const INK: RGBA = hex("#1f1e1d");
const SCREEN_LINE: RGBA = hex("#dcd9d2");
const CREAM: RGBA = hex("#f5f0e8");

/** A 5x7 pixel font: just the six letters the wordmark needs. */
const GLYPHS: Record<string, string[]> = {
  J: ["..###", "...#.", "...#.", "...#.", "...#.", "#..#.", ".##.."],
  O: [".###.", "#...#", "#...#", "#...#", "#...#", "#...#", ".###."],
  K: ["#...#", "#..#.", "#.#..", "##...", "#.#..", "#..#.", "#...#"],
  B: ["####.", "#...#", "#...#", "####.", "#...#", "#...#", "####."],
  E: ["#####", "#....", "#....", "####.", "#....", "#....", "#####"],
  T: ["#####", "..#..", "..#..", "..#..", "..#..", "..#..", "..#.."],
};
const WORD = "JOKBET";
/** The colour the wordmark is drawn in: the pet's own body colour. */
const MARK = "#d97757";
/** "E" is the palette's ink colour; the SVG recolours the path anyway. */
const ON = "E";

/** The wordmark as a sprite grid: 5x7 glyphs, one blank column between letters. */
function wordmarkRows(): string[] {
  const letters = [...WORD].map((ch) => GLYPHS[ch]);
  return Array.from({ length: 7 }, (_, r) => letters.map((g) => g[r]).join(".")).map((row) =>
    row.replaceAll("#", ON),
  );
}

function wordmarkSvg(rows: string[]): string {
  const d = compileGrid(rows)
    .map((p) => p.d)
    .join("");
  const w = rows[0].length;
  return [
    `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${w} ${rows.length}" width="${w * 4}" height="${
      rows.length * 4
    }" shape-rendering="crispEdges" role="img" aria-label="${WORD}">`,
    `<title>${WORD}</title>`,
    `<path fill="${MARK}" d="${d}"/>`,
    `</svg>`,
    ``,
  ].join("\n");
}

/** Every animation the page can play, with the app's own per-frame timings. */
function spriteData(digests: Record<string, string>) {
  const animations: Record<string, unknown> = {};
  for (const name of ANIMATIONS) {
    const anim = ANIMS[name];
    animations[name] = {
      file: `jokbet-${name}.png`,
      frames: anim.frames.length,
      ms: anim.frames.map((f) => f.ms),
      loop: !!anim.loop,
    };
  }
  return {
    cell: [GRID_W * SCALE, GRID_H * SCALE],
    grid: [GRID_W, GRID_H],
    animations,
    digests,
  };
}

/** The app icon: the pet on a rounded cream tile, sized proportionally. */
function iconTile(size: number): Buffer {
  const c = new Canvas(size, size);
  const inset = Math.round(size * 0.098);
  const radius = Math.round(size * 0.176);
  for (let y = inset; y < size - inset; y++) {
    for (let x = inset; x < size - inset; x++) {
      const dx = Math.max(inset + radius - x, 0, x - (size - inset - radius - 1));
      const dy = Math.max(inset + radius - y, 0, y - (size - inset - radius - 1));
      if (dx * dx + dy * dy <= radius * radius) c.set(x, y, CREAM);
    }
  }
  const body = compose()
    .slice(PET_Y, PET_Y + PET_H)
    .map((r) => r.slice(PET_X, PET_X + PET_W));
  const s = Math.max(1, Math.floor((size * 24) / 1024));
  c.sprite(body, Math.floor((size - PET_W * s) / 2), Math.floor((size - PET_H * s) / 2), s, paletteColor);
  return encodePng(c);
}

/** A minimal .ico wrapping one PNG, so /favicon.ico does not 404. */
function ico(png: Buffer, size: number): Buffer {
  const header = Buffer.alloc(6);
  header.writeUInt16LE(0, 0); // reserved
  header.writeUInt16LE(1, 2); // type: icon
  header.writeUInt16LE(1, 4); // one image
  const entry = Buffer.alloc(16);
  entry.writeUInt8(size >= 256 ? 0 : size, 0);
  entry.writeUInt8(size >= 256 ? 0 : size, 1);
  entry.writeUInt8(0, 2);
  entry.writeUInt8(0, 3);
  entry.writeUInt16LE(1, 4); // colour planes
  entry.writeUInt16LE(32, 6); // bits per pixel
  entry.writeUInt32LE(png.length, 8);
  entry.writeUInt32LE(header.length + entry.length, 12);
  return Buffer.concat([header, entry, png]);
}

/** The 1200x630 social card: paper, the pixel wordmark, the pet on a screen line. */
function ogCard(mark: string[]): Buffer {
  const W = 1200;
  const H = 630;
  const petScale = 12;
  const margin = 72;
  const c = new Canvas(W, H);
  c.fill(0, 0, W, H, PAPER);
  c.sprite(mark, margin, 104, 8, () => INK);
  const groundY = H - 88;
  c.sprite(compose(), W - margin - GRID_W * petScale, groundY - GRID_H * petScale, petScale, paletteColor);
  c.fill(margin, groundY, W - margin * 2, 3, SCREEN_LINE);
  return encodePng(c);
}

/** The rendered bytes of every generated file, keyed by name. */
function render(): Record<string, Buffer> {
  const out: Record<string, Buffer> = {};
  const digests: Record<string, string> = {};
  for (const name of ANIMATIONS) {
    const anim = ANIMS[name];
    const frameW = GRID_W * SCALE;
    const c = new Canvas(frameW * anim.frames.length, GRID_H * SCALE);
    anim.frames.forEach((f, i) => c.sprite(frameRows(f), i * frameW, 0, SCALE, paletteColor));
    out[`jokbet-${name}.png`] = encodePng(c);
    // of the raw pixels, so a zlib version difference cannot fail the check
    digests[`jokbet-${name}.png`] = createHash("sha256").update(c.data).digest("hex");
  }
  const mark = wordmarkRows();
  out["wordmark.svg"] = Buffer.from(wordmarkSvg(mark));
  out["og.png"] = ogCard(mark);
  for (const [file, size] of [
    ["favicon-32.png", 32],
    ["apple-touch-icon.png", 180],
    ["favicon-192.png", 192],
    ["icon-512.png", 512],
  ] as const) {
    out[file] = iconTile(size);
  }
  out["favicon.ico"] = ico(out["favicon-32.png"], 32);
  return { ...out, "jokbet.json": Buffer.from(JSON.stringify(spriteData(digests), null, 1) + "\n") };
}

function main(mode: string, outDir: string) {
  const files = render();
  if (mode === "check") {
    // Digests of the raw pixels, so a zlib build difference cannot fail this.
    const want = JSON.parse(readFileSync(join(outDir, "jokbet.json"), "utf8")) as {
      digests: Record<string, string>;
    };
    const have = JSON.parse(files["jokbet.json"].toString()) as { digests: Record<string, string> };
    let bad = 0;
    for (const [name, digest] of Object.entries(have.digests)) {
      if (want.digests[name] !== digest) {
        console.error(`${name} has drifted from the sprites — run: npm run site:assets`);
        bad++;
      }
    }
    for (const name of Object.keys(files)) {
      try {
        statSync(join(outDir, name));
      } catch {
        console.error(`missing ${join(outDir, name)} — run: npm run site:assets`);
        bad++;
      }
    }
    if (bad) process.exit(1);
    console.log(`site assets in ${outDir} match the sprites`);
    return;
  }

  mkdirSync(outDir, { recursive: true });
  for (const [name, bytes] of Object.entries(files)) writeFileSync(join(outDir, name), bytes);
  console.log(`site assets -> ${outDir}`);
  for (const name of Object.keys(files)) {
    console.log(`  ${name.padEnd(20)} ${Math.round(statSync(join(outDir, name)).size / 1024)} KB`);
  }
}

const [mode = "gen", dir] = process.argv.slice(2);
main(mode, dir ?? join(import.meta.dirname, "..", "site", "assets"));
