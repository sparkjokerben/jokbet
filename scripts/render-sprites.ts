// Renders Jokbet to PNG without any image library:
//   node scripts/render-sprites.ts sheet <out.png>   every animation frame, for review
//   node scripts/render-sprites.ts icons <dir>       app icon source + tray icons
//   node scripts/render-sprites.ts anim <name> <out.png> [cols] [scale]

import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { ANIMS, GRID_H, GRID_W, PET_H, PET_W, PET_X, PET_Y, compose, frameRows } from "../src/sprites/jokbet.ts";
import { Canvas, encodePng, paletteColor, type RGBA } from "./png.ts";

function renderSheet(out: string) {
  const scale = 6;
  const pad = 6;
  const cellW = GRID_W * scale + pad;
  const cellH = GRID_H * scale + pad;
  const entries = Object.entries(ANIMS);
  const cols = Math.max(...entries.map(([, a]) => a.frames.length)) + 2; // + gaze demos
  const c = new Canvas(cols * cellW + pad, entries.length * cellH + pad);
  c.fill(0, 0, c.w, c.h, [200, 200, 205, 255]);
  const draw = (rows: readonly string[], col: number, row: number) => {
    const x = pad + col * cellW;
    const y = pad + row * cellH;
    c.fill(x, y, GRID_W * scale, GRID_H * scale, [236, 236, 240, 255]);
    c.sprite(rows, x, y, scale, paletteColor);
  };
  entries.forEach(([name, anim], row) => {
    anim.frames.forEach((f, col) => draw(frameRows(f), col, row));
    if (name === "idle") {
      draw(compose({ gaze: [-1, -1] }), cols - 2, row);
      draw(compose({ gaze: [1, 1] }), cols - 1, row);
    }
  });
  writeFileSync(out, encodePng(c));
  console.log(`sheet: ${entries.map(([n]) => n).join(", ")} -> ${out}`);
}

/** Jokbet's own box inside the canvas (a 24x16 sprite at PET_X, PET_Y). */
const BODY = { x: PET_X, y: PET_Y, w: PET_W, h: PET_H };
const bodyRows = () =>
  compose()
    .slice(BODY.y, BODY.y + BODY.h)
    .map((r) => r.slice(BODY.x, BODY.x + BODY.w));

function renderIcons(dir: string) {
  mkdirSync(dir, { recursive: true });
  const rows = bodyRows();

  // App icon source (1024², fed to `tauri icon`): Jokbet on a rounded cream tile.
  const size = 1024;
  const icon = new Canvas(size, size);
  const inset = 100;
  const radius = 180;
  const cream: RGBA = [245, 240, 232, 255];
  for (let y = inset; y < size - inset; y++) {
    for (let x = inset; x < size - inset; x++) {
      const dx = Math.max(inset + radius - x, 0, x - (size - inset - radius - 1));
      const dy = Math.max(inset + radius - y, 0, y - (size - inset - radius - 1));
      if (dx * dx + dy * dy <= radius * radius) icon.set(x, y, cream);
    }
  }
  const s = 24;
  icon.sprite(rows, (size - BODY.w * s) / 2, (size - BODY.h * s) / 2 + 20, s, paletteColor);
  writeFileSync(join(dir, "icon-source.png"), encodePng(icon));

  // Tray icons at @2x: macOS template (black silhouette, eyes cut out) and a colored one.
  const tray = (colorOf: (ch: string) => RGBA | null) => {
    const t = new Canvas(BODY.w * 2 + 4, BODY.h * 2 + 4);
    t.sprite(rows, 2, 2, 2, colorOf);
    return encodePng(t);
  };
  writeFileSync(join(dir, "tray-template.png"), tray((ch) => (ch === "O" ? [0, 0, 0, 255] : null)));
  writeFileSync(join(dir, "tray-color.png"), tray(paletteColor));
  console.log(`icons -> ${dir}`);
}

/** One animation, every frame, big enough to read. */
function renderAnim(name: string, out: string, cols: number, scale: number) {
  const anim = ANIMS[name as keyof typeof ANIMS];
  const pad = 4;
  const cellW = GRID_W * scale + pad;
  const cellH = GRID_H * scale + pad;
  const rows = Math.ceil(anim.frames.length / cols);
  const c = new Canvas(cols * cellW + pad, rows * cellH + pad);
  c.fill(0, 0, c.w, c.h, [32, 32, 34, 255]);
  anim.frames.forEach((f, i) => {
    const x = pad + (i % cols) * cellW;
    const y = pad + Math.floor(i / cols) * cellH;
    c.sprite(frameRows(f), x, y, scale, paletteColor);
  });
  writeFileSync(out, encodePng(c));
  console.log(`${name}: ${anim.frames.length} frames -> ${out}`);
}

const [mode, ...rest] = process.argv.slice(2);
const [a, b, c, d] = rest;
if (mode === "sheet" && a) renderSheet(a);
else if (mode === "icons" && a) renderIcons(a);
else if (mode === "anim" && a && b) renderAnim(a, b, Number(c) || 8, Number(d) || 6);
else {
  console.error("usage: node scripts/render-sprites.ts sheet <out.png> | icons <dir> | anim <name> <out.png> [cols] [scale]");
  process.exit(1);
}
