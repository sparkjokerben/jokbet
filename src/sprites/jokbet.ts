// Jokbet, the pet. Its pixels are decoded from the Claude Code terminal logo:
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

export type ArmPose = "rest" | "up" | "high" | "down";
export type EyeState = "open" | "closed" | "wide" | "happy" | "x";
export type Side = -1 | 1;
export type Effect =
  | "zzz1"
  | "zzz2"
  | "sparkleA"
  | "sparkleB"
  | "heart"
  | "sweat"
  | "exclaim"
  | "question"
  | "pause";

export interface Pose {
  /** Whole-body offset; negative dy jumps up. */
  dx?: number;
  dy?: number;
  /** 1 lowers the torso by one row (breathing, squish, sleep). */
  squash?: 0 | 1;
  armL?: ArmPose;
  armR?: ArmPose;
  /** Length (0..2) of each of the four legs, left to right. */
  legs?: readonly [number, number, number, number];
  /** Turns three-quarters toward a side: the far edge is shaded, the eyes follow. */
  turn?: Side;
  /** The outermost leg on that side stretches out to kick. */
  kick?: Side;
  /** Sits in profile facing right at a laptop; the value picks which paw is down. */
  typing?: 0 | 1;
  eyes?: EyeState;
  /** Eye offset, each component in -1..1. */
  gaze?: readonly [number, number];
  blindfold?: boolean;
  /** Grid cell of the soccer ball's top-left corner. */
  ball?: readonly [number, number];
  laptop?: "held" | "closed" | "open";
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

const ARM_Y: Record<ArmPose, number> = { high: 0, up: 2, rest: 4, down: 6 };
const LEG_X = [4, 6, 11, 13] as const;
const EYE_X = [5, 12] as const;

/** A checkered ball, like the one Clawd juggles. */
const BALL = ["WEW", "EWE", "WEW"];

/** Laptop glyphs in body-local coordinates (x, y of the top-left cell). */
const LAPTOP: Record<NonNullable<Pose["laptop"]>, { x: number; y: number; rows: readonly string[] }> = {
  // Closed, held overhead in the raised right paw.
  held: { x: 14, y: -2, rows: ["GGGGG", "GGGGG"] },
  // Closed, lying on the ground to the right.
  closed: { x: 15, y: 8, rows: ["GGGGG", "GGGGG"] },
  // Open, side view: keyboard on the ground, screen leaning away.
  open: { x: 15, y: 6, rows: [".....G", "....GG", "...GG.", "GGGG.."] },
};

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
};

/** Renders a pose into GRID_H strings of GRID_W palette characters. */
export function compose(pose: Pose = {}): string[] {
  const g = blank();
  const sq = pose.squash ?? 0;
  const bx = (x: number) => OX + (pose.dx ?? 0) + x;
  const by = (y: number) => OY + (pose.dy ?? 0) + y;
  const typing = pose.typing !== undefined;

  const legs = pose.legs ?? [2, 2, 2, 2];
  legs.forEach((len, i) => {
    const x = LEG_X[i];
    if (typing) {
      // Sitting: knees forward, feet tucked back.
      put(g, bx(x), by(8), "O");
      put(g, bx(x - 1), by(9), "O");
    } else if ((pose.kick === 1 && i === 3) || (pose.kick === -1 && i === 0)) {
      // The leg stretches out and down toward the ball.
      const s = pose.kick;
      for (const [ox, oy] of [[0, 8], [1, 8], [2, 9], [3, 9]]) put(g, bx(x + s * ox), by(oy), "O");
    } else {
      rect(g, bx(x), by(8), 1, len, "O");
    }
  });

  rect(g, bx(3), by(sq), 12, 8 - sq, "O");
  if (typing) {
    // Profile: the back is in shade and both front paws work the keyboard.
    rect(g, bx(3), by(sq), 2, 8 - sq, "D");
    const down = pose.typing === 1;
    rect(g, bx(15), by(3 + sq), down ? 1 : 2, 2, "O");
    rect(g, bx(15), by(5 + sq), down ? 2 : 1, 2, "O");
  } else {
    const turn = pose.turn ?? 0;
    const shadeL = turn === 1;
    const shadeR = turn === -1;
    if (shadeL) rect(g, bx(3), by(sq), 1, 8 - sq, "D");
    if (shadeR) rect(g, bx(14), by(sq), 1, 8 - sq, "D");
    rect(g, bx(1), by(ARM_Y[pose.armL ?? "rest"] + sq), 2, 2, shadeL ? "D" : "O");
    rect(g, bx(15), by(ARM_Y[pose.armR ?? "rest"] + sq), 2, 2, shadeR ? "D" : "O");
  }

  if (pose.blindfold) {
    rect(g, bx(2), by(2 + sq), 14, 2, "B");
    stamp(g, bx(16), by(2 + sq), ["B", ".B"]);
  } else {
    const [gx, gy] = pose.gaze ?? [0, 0];
    const shift = typing ? 2 : (pose.turn ?? 0);
    for (const ex of EYE_X) {
      const x = bx(ex + shift + gx);
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

  if (pose.laptop) {
    const l = LAPTOP[pose.laptop];
    stamp(g, bx(l.x), by(l.y), l.rows);
  }
  if (pose.ball) stamp(g, pose.ball[0], pose.ball[1], BALL);
  for (const fx of pose.fx ?? []) {
    const glyph = GLYPHS[fx];
    stamp(g, glyph.x, glyph.y, glyph.rows);
  }
  return g.map((row) => row.join(""));
}

export type AnimName =
  | "idle"
  | "typing"
  | "typingEnd"
  | "click"
  | "scroll"
  | "sleep"
  | "wake"
  | "celebrate"
  | "poke"
  | "hearts"
  | "soccer"
  | "wave"
  | "dragged"
  | "noperm"
  | "secure"
  | "soccerIdle"
  | "lookAround";

export interface Anim {
  frames: readonly Frame[];
  loop: boolean;
  /** Looping animations replay from this frame; earlier ones play once as an intro. */
  loopFrom?: number;
}

const UP = { armL: "up", armR: "up" } as const;

type Cell = readonly [number, number];
/** Mirrors a ball position to the other side of the pet. */
const side = (s: Side, [x, y]: Cell): Cell => (s === 1 ? [x, y] : [GRID_W - 3 - x, y]);
const arm = (s: Side, pose: ArmPose): Pose => (s === 1 ? { armR: pose } : { armL: pose });

/**
 * Kicks the ball from the ground on side `s`, bumps it up that side and
 * lobs it over the head until it drops beside the other arm.
 */
function volley(s: Side): Frame[] {
  const at = (c: Cell) => side(s, c);
  const o = -s as Side;
  return [
    { ms: 110, pose: { turn: s, gaze: [s, 1], ball: at([20, 13]) } },
    { ms: 170, pose: { turn: s, gaze: [s, 1], kick: s, ball: at([20, 13]) } },
    { ms: 100, pose: { ...arm(s, "up"), turn: s, gaze: [s, 0], ball: at([20, 9]) } },
    { ms: 100, pose: { ...arm(s, "high"), turn: s, gaze: [s, -1], ball: at([20, 4]) } },
    { ms: 110, pose: { gaze: [s, -1], ball: at([15, 1]) } },
    { ms: 140, pose: { gaze: [0, -1], ball: at([10, 0]) } },
    { ms: 110, pose: { gaze: [o, -1], ball: at([6, 1]) } },
    { ms: 100, pose: { turn: o, gaze: [o, -1], ball: at([1, 4]) } },
    { ms: 100, pose: { turn: o, gaze: [o, 0], ball: at([1, 9]) } },
  ];
}

/** Crouch, catch the ball dropping in on the right, juggle it side to side, boot it away. */
const SOCCER: readonly Frame[] = [
  { ms: 220, pose: { squash: 1, eyes: "closed" } },
  { ms: 110, pose: { turn: 1, gaze: [1, -1], ball: [20, 4] } },
  { ms: 110, pose: { turn: 1, gaze: [1, 0], ball: [20, 9] } },
  ...volley(1),
  ...volley(-1),
  { ms: 110, pose: { turn: 1, gaze: [1, 1], ball: [20, 13] } },
  { ms: 170, pose: { turn: 1, gaze: [1, 1], kick: 1, ball: [20, 13] } },
  { ms: 100, pose: { turn: 1, gaze: [1, 0], ball: [21, 7] } },
  { ms: 100, pose: { turn: 1, gaze: [1, -1], ball: [22, 2] } },
  { ms: 400, pose: { eyes: "happy" } },
];

const BREATHE: readonly Frame[] = [
  { ms: 1400, pose: {} },
  { ms: 500, pose: { squash: 1 } },
];

export const ANIMS: Record<AnimName, Anim> = {
  idle: { loop: true, frames: BREATHE },
  // Gets the laptop out once, then types; the typing frames' period follows
  // typing speed at runtime.
  typing: {
    loop: true,
    loopFrom: 3,
    frames: [
      { ms: 160, pose: { armR: "high", gaze: [1, -1], laptop: "held" } },
      { ms: 150, pose: { gaze: [1, 1], laptop: "closed" } },
      { ms: 150, pose: { gaze: [1, 1], laptop: "open" } },
      { ms: 160, pose: { squash: 1, typing: 0, laptop: "open" } },
      { ms: 160, pose: { squash: 1, typing: 1, laptop: "open" } },
    ],
  },
  // Puts the laptop away when typing stops.
  typingEnd: {
    loop: false,
    frames: [
      { ms: 150, pose: { gaze: [1, 1], laptop: "closed" } },
      { ms: 170, pose: { armR: "high", gaze: [1, -1], laptop: "held" } },
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
  hearts: {
    loop: false,
    frames: [
      { ms: 120, pose: { squash: 1, eyes: "happy" } },
      { ms: 220, pose: { ...UP, dy: -3, eyes: "happy", fx: ["heart"] } },
      { ms: 220, pose: { ...UP, dy: -1, eyes: "happy", fx: ["heart"] } },
      { ms: 150, pose: { squash: 1, eyes: "happy", fx: ["heart"] } },
      { ms: 400, pose: { eyes: "happy", fx: ["heart"] } },
    ],
  },
  soccer: { loop: false, frames: SOCCER },
  wave: {
    loop: false,
    frames: [
      { ms: 180, pose: { armR: "high" } },
      { ms: 180, pose: { armR: "up" } },
      { ms: 180, pose: { armR: "high" } },
      { ms: 180, pose: { armR: "up" } },
      { ms: 180, pose: { armR: "high" } },
      { ms: 250, pose: {} },
    ],
  },
  // Idle variants. The eyes keep following the cursor over these.
  soccerIdle: { loop: true, frames: [...BREATHE, ...BREATHE, ...SOCCER] },
  lookAround: {
    loop: true,
    frames: [
      ...BREATHE,
      { ms: 1100, pose: { turn: -1 } },
      { ms: 400, pose: {} },
      { ms: 1100, pose: { turn: 1 } },
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
