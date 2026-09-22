// Clawd, decoded from the Claude Code terminal logo:
//
//    ▐▛███▜▌
//   ▝▜█████▛▘
//     ▘▘ ▝▝
//
// Each character is a 2x2 block of quadrants; terminal cells are twice as
// tall as wide, so every quadrant row is doubled to get square pixels.
// Result in body-local coordinates (x 0..17, y 0..9):
//   torso x3..14 y0..7 · arms 2x2 at x1..2 / x15..16 y4..5
//   eyes 1x2 at x5 / x12 y2..3 · legs 1x2 at x4 x6 x11 x13 y8..9

import { TRANSPARENT } from "./palette.ts";

export const GRID_W = 24;
export const GRID_H = 16;
/** Body-local origin inside the grid; the top 6 rows are room for effects. */
const OX = 3;
const OY = 6;

export type ArmPose = "rest" | "up" | "down";
export type EyeState = "open" | "closed" | "wide" | "happy" | "x";
export type Effect =
  | "zzz1"
  | "zzz2"
  | "sparkleA"
  | "sparkleB"
  | "heart"
  | "sweat"
  | "exclaim"
  | "question"
  | "pause"
  | "tapL"
  | "tapR";

export interface Pose {
  /** Whole-body vertical offset; negative jumps up. */
  dy?: number;
  /** 1 lowers the torso by one row (breathing, squish, sleep). */
  squash?: 0 | 1;
  armL?: ArmPose;
  armR?: ArmPose;
  /** Length (0..2) of each of the four legs, left to right. */
  legs?: readonly [number, number, number, number];
  eyes?: EyeState;
  /** Eye offset, each component in -1..1. */
  gaze?: readonly [number, number];
  blindfold?: boolean;
  fx?: readonly Effect[];
}

export interface Frame {
  ms: number;
  pose: Pose;
}

type Grid = string[][];

function blank(): Grid {
  return Array.from({ length: GRID_H }, () => Array<string>(GRID_W).fill(TRANSPARENT));
}

function put(g: Grid, x: number, y: number, ch: string) {
  if (x >= 0 && x < GRID_W && y >= 0 && y < GRID_H) g[y][x] = ch;
}

function rect(g: Grid, x: number, y: number, w: number, h: number, ch: string) {
  for (let j = 0; j < h; j++) for (let i = 0; i < w; i++) put(g, x + i, y + j, ch);
}

/** Stamps a small glyph given as rows; "." cells are skipped. */
function stamp(g: Grid, x: number, y: number, rows: readonly string[]) {
  rows.forEach((row, j) => {
    [...row].forEach((ch, i) => {
      if (ch !== TRANSPARENT) put(g, x + i, y + j, ch);
    });
  });
}

const ARM_Y: Record<ArmPose, number> = { up: 2, rest: 4, down: 6 };
const LEG_X = [4, 6, 11, 13] as const;
const EYE_X = [5, 12] as const;

const GLYPHS: Record<Effect, { x: number; y: number; rows: readonly string[] }> = {
  zzz1: { x: 17, y: 2, rows: ["ZZZZ", "..Z.", ".Z..", "ZZZZ"] },
  zzz2: { x: 21, y: 0, rows: ["ZZZ", "..Z", ".Z.", "ZZZ"] },
  sparkleA: { x: 1, y: 1, rows: [".Y.", "YYY", ".Y."] },
  sparkleB: { x: 19, y: 0, rows: [".Y.", "YYY", ".Y."] },
  heart: { x: 17, y: 0, rows: [".R.R.", "RRRRR", ".RRR.", "..R.."] },
  sweat: { x: 19, y: 5, rows: ["S", "S"] },
  exclaim: { x: 20, y: 0, rows: ["R", "R", "R", ".", "R"] },
  question: { x: 19, y: 0, rows: ["ZZ.", "..Z", ".Z.", "...", ".Z."] },
  pause: { x: 19, y: 1, rows: ["Z.Z", "Z.Z", "Z.Z"] },
  tapL: { x: 2, y: 11, rows: ["Y"] },
  tapR: { x: 21, y: 11, rows: ["Y"] },
};

/** Renders a pose into GRID_H strings of GRID_W palette characters. */
export function compose(pose: Pose = {}): string[] {
  const g = blank();
  const dy = pose.dy ?? 0;
  const sq = pose.squash ?? 0;
  const bx = (x: number) => OX + x;
  const by = (y: number) => OY + dy + y;

  const legs = pose.legs ?? [2, 2, 2, 2];
  legs.forEach((len, i) => rect(g, bx(LEG_X[i]), by(8), 1, len, "O"));

  rect(g, bx(3), by(sq), 12, 8 - sq, "O");
  rect(g, bx(1), by(ARM_Y[pose.armL ?? "rest"] + sq), 2, 2, "O");
  rect(g, bx(15), by(ARM_Y[pose.armR ?? "rest"] + sq), 2, 2, "O");

  if (pose.blindfold) {
    rect(g, bx(2), by(2 + sq), 14, 2, "B");
    stamp(g, bx(16), by(2 + sq), ["B", ".B"]);
  } else {
    const [gx, gy] = pose.gaze ?? [0, 0];
    for (const ex of EYE_X) {
      const x = bx(ex + gx);
      const y = by(2 + sq + gy);
      switch (pose.eyes ?? "open") {
        case "open":
          rect(g, x, y, 1, 2, "E");
          break;
        case "closed":
          put(g, x, y + 1, "E");
          break;
        case "wide":
          rect(g, x, y - 1, 1, 3, "E");
          break;
        case "happy":
          stamp(g, x - 1, y, [".E.", "E.E"]);
          break;
        case "x":
          stamp(g, x - 1, y - 1, ["E.E", ".E.", "E.E"]);
          break;
      }
    }
  }

  for (const fx of pose.fx ?? []) {
    const glyph = GLYPHS[fx];
    stamp(g, glyph.x, glyph.y, glyph.rows);
  }
  return g.map((row) => row.join(""));
}

export type AnimName =
  | "idle"
  | "typing"
  | "click"
  | "scroll"
  | "sleep"
  | "wake"
  | "celebrate"
  | "poke"
  | "special"
  | "dragged"
  | "noperm"
  | "secure";

export interface Anim {
  frames: readonly Frame[];
  loop: boolean;
}

const UP = { armL: "up", armR: "up" } as const;

export const ANIMS: Record<AnimName, Anim> = {
  idle: {
    loop: true,
    frames: [
      { ms: 1400, pose: {} },
      { ms: 500, pose: { squash: 1 } },
    ],
  },
  // Frame period is overridden by typing speed at runtime.
  typing: {
    loop: true,
    frames: [
      { ms: 160, pose: { armL: "up", gaze: [0, 1], fx: ["tapR"] } },
      { ms: 160, pose: { armR: "up", gaze: [0, 1], fx: ["tapL"] } },
    ],
  },
  click: {
    loop: false,
    frames: [
      { ms: 120, pose: { ...UP } },
      { ms: 160, pose: { squash: 1 } },
    ],
  },
  scroll: {
    loop: true,
    frames: [
      { ms: 220, pose: { gaze: [0, -1] } },
      { ms: 220, pose: { gaze: [0, 1] } },
    ],
  },
  sleep: {
    loop: true,
    frames: [
      { ms: 1100, pose: { squash: 1, eyes: "closed", fx: ["zzz1"] } },
      { ms: 1100, pose: { squash: 1, eyes: "closed", fx: ["zzz1", "zzz2"] } },
    ],
  },
  wake: {
    loop: false,
    frames: [{ ms: 450, pose: { eyes: "wide", fx: ["exclaim"] } }],
  },
  celebrate: {
    loop: true,
    frames: [
      { ms: 140, pose: { squash: 1, eyes: "happy" } },
      { ms: 200, pose: { ...UP, dy: -3, eyes: "happy", fx: ["sparkleA", "sparkleB"] } },
      { ms: 200, pose: { ...UP, dy: -2, eyes: "happy", fx: ["sparkleB"] } },
      { ms: 140, pose: { squash: 1, eyes: "happy", fx: ["sparkleA"] } },
    ],
  },
  poke: {
    loop: false,
    frames: [
      { ms: 200, pose: { squash: 1, eyes: "closed", armL: "down", armR: "down" } },
      { ms: 350, pose: { eyes: "wide", fx: ["sweat"] } },
    ],
  },
  special: {
    loop: false,
    frames: [
      { ms: 120, pose: { squash: 1, eyes: "happy" } },
      { ms: 220, pose: { ...UP, dy: -3, eyes: "happy", fx: ["heart"] } },
      { ms: 220, pose: { ...UP, dy: -1, eyes: "happy", fx: ["heart"] } },
      { ms: 150, pose: { squash: 1, eyes: "happy", fx: ["heart"] } },
      { ms: 400, pose: { eyes: "happy", fx: ["heart"] } },
    ],
  },
  dragged: {
    loop: true,
    frames: [
      { ms: 160, pose: { ...UP, legs: [1, 2, 1, 2], eyes: "wide", fx: ["sweat"] } },
      { ms: 160, pose: { ...UP, legs: [2, 1, 2, 1], eyes: "wide" } },
    ],
  },
  noperm: {
    loop: true,
    frames: [
      { ms: 1200, pose: { eyes: "x", fx: ["question"] } },
      { ms: 600, pose: { eyes: "x", squash: 1 } },
    ],
  },
  secure: {
    loop: true,
    frames: [
      { ms: 900, pose: { blindfold: true, armL: "up" } },
      { ms: 900, pose: { blindfold: true, armR: "up" } },
    ],
  },
};
