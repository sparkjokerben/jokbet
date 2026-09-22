// Renders Jokbet to PNG without any image library:
//   node scripts/render-sprites.ts sheet <out.png>   every animation frame, for review
//   node scripts/render-sprites.ts icons <dir>       app icon source + tray icons

import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { deflateSync } from "node:zlib";
import { ANIMS, GRID_H, GRID_W, compose, type Pose } from "../src/sprites/jokbet.ts";
import { PALETTE, TRANSPARENT } from "../src/sprites/palette.ts";

type RGBA = [number, number, number, number];

class Canvas {
  readonly w: number;
  readonly h: number;
  readonly data: Uint8Array;
  constructor(w: number, h: number) {
    this.w = w;
    this.h = h;
    this.data = new Uint8Array(w * h * 4);
  }
  set(x: number, y: number, c: RGBA) {
    if (x < 0 || y < 0 || x >= this.w || y >= this.h) return;
    this.data.set(c, (y * this.w + x) * 4);
  }
  fill(x: number, y: number, w: number, h: number, c: RGBA) {
    for (let j = 0; j < h; j++) for (let i = 0; i < w; i++) this.set(x + i, y + j, c);
  }
  /** Draws sprite rows; `colorOf` maps a palette char to a color (or null to skip). */
  sprite(rows: readonly string[], ox: number, oy: number, scale: number, colorOf: (ch: string) => RGBA | null) {
    rows.forEach((row, y) =>
      [...row].forEach((ch, x) => {
        if (ch === TRANSPARENT) return;
        const c = colorOf(ch);
        if (c) this.fill(ox + x * scale, oy + y * scale, scale, scale, c);
      }),
    );
  }
}

const hex = (h: string): RGBA => [
  parseInt(h.slice(1, 3), 16),
  parseInt(h.slice(3, 5), 16),
  parseInt(h.slice(5, 7), 16),
  255,
];
const paletteColor = (ch: string) => hex(PALETTE[ch]);

const CRC_TABLE = Array.from({ length: 256 }, (_, n) => {
  let c = n;
  for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
  return c >>> 0;
});
function crc32(buf: Uint8Array): number {
  let c = 0xffffffff;
  for (const b of buf) c = CRC_TABLE[(c ^ b) & 0xff] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
}
function chunk(type: string, body: Uint8Array): Buffer {
  const out = Buffer.alloc(12 + body.length);
  out.writeUInt32BE(body.length, 0);
  out.write(type, 4, "ascii");
  out.set(body, 8);
  out.writeUInt32BE(crc32(out.subarray(4, 8 + body.length)), 8 + body.length);
  return out;
}
function encodePng(c: Canvas): Buffer {
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(c.w, 0);
  ihdr.writeUInt32BE(c.h, 4);
  ihdr.set([8, 6, 0, 0, 0], 8); // 8-bit RGBA
  const raw = Buffer.alloc((c.w * 4 + 1) * c.h);
  for (let y = 0; y < c.h; y++) raw.set(c.data.subarray(y * c.w * 4, (y + 1) * c.w * 4), y * (c.w * 4 + 1) + 1);
  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk("IHDR", ihdr),
    chunk("IDAT", deflateSync(raw)),
    chunk("IEND", new Uint8Array()),
  ]);
}

function renderSheet(out: string) {
  const scale = 6;
  const pad = 6;
  const cellW = GRID_W * scale + pad;
  const cellH = GRID_H * scale + pad;
  const entries = Object.entries(ANIMS);
  const cols = Math.max(...entries.map(([, a]) => a.frames.length)) + 2; // + gaze demos
  const c = new Canvas(cols * cellW + pad, entries.length * cellH + pad);
  c.fill(0, 0, c.w, c.h, [200, 200, 205, 255]);
  const draw = (pose: Pose, col: number, row: number) => {
    const x = pad + col * cellW;
    const y = pad + row * cellH;
    c.fill(x, y, GRID_W * scale, GRID_H * scale, [236, 236, 240, 255]);
    c.sprite(compose(pose), x, y, scale, paletteColor);
  };
  entries.forEach(([name, anim], row) => {
    anim.frames.forEach((f, col) => draw(f.pose, col, row));
    if (name === "idle") {
      draw({ gaze: [-1, -1] }, cols - 2, row);
      draw({ gaze: [1, 1] }, cols - 1, row);
    }
  });
  writeFileSync(out, encodePng(c));
  console.log(`sheet: ${entries.map(([n]) => n).join(", ")} -> ${out}`);
}

/** Tight bounds of Jokbet's body in the idle pose (grid columns 4..19, rows 6..15). */
const BODY = { x: 4, y: 6, w: 16, h: 10 };
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
  const s = 40;
  icon.sprite(rows, (size - BODY.w * s) / 2, (size - BODY.h * s) / 2 + 20, s, paletteColor);
  writeFileSync(join(dir, "icon-source.png"), encodePng(icon));

  // Tray icons at @2x: macOS template (black silhouette, eyes cut out) and a colored one.
  const tray = (colorOf: (ch: string) => RGBA | null) => {
    const t = new Canvas(36, 24);
    t.sprite(rows, 2, 2, 2, colorOf);
    return encodePng(t);
  };
  writeFileSync(join(dir, "tray-template.png"), tray((ch) => (ch === "O" ? [0, 0, 0, 255] : null)));
  writeFileSync(join(dir, "tray-color.png"), tray(paletteColor));
  console.log(`icons -> ${dir}`);
}

const [mode, out] = process.argv.slice(2);
if (mode === "sheet" && out) renderSheet(out);
else if (mode === "icons" && out) renderIcons(out);
else {
  console.error("usage: node scripts/render-sprites.ts sheet <out.png> | icons <dir>");
  process.exit(1);
}
