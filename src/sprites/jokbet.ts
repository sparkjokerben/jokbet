// Jokbet, the pet, drawn the way Claude's mascot is drawn in the app: a 24x16
// pixel grid where the torso is 16 cells wide, the arms are 4x4 blocks on the
// sides, four 2x4 legs hang below and the eyes are 2x2 squares.
//
//    ....OOOOOOOOOOOOOOOO....     torso   x 4..19, y 0..11
//    ....OO..OOOOOOOO..OO....     eyes    2x2 at x 6 / 16, y 2
//    OOOOOOOOOOOOOOOOOOOOOOOO     arms    4x4 at x 0..3 / 20..23, y 4..7
//    ....OO..OO....OO..OO....     legs    2x4 at x 4, 8, 14, 18, y 12..15
//
// The canvas is larger than the pet so the ball can be juggled above it and
// the laptop can sit beside it; the pet's own box sits at (OX, OY) inside it.

import { TRANSPARENT } from "./palette.ts";
import { SOCCER, SOCCER_IDLE_AFTER } from "./frames/soccer.ts";
import { TYPING_INTRO, TYPING_LOOP, TYPING_OUTRO } from "./frames/typing.ts";
import type { RawFrame } from "./frames/types.ts";

export const GRID_W = 40;
export const GRID_H = 26;
/** The pet's 24x16 box inside the canvas: room above for the ball it juggles,
 * room on the right for the laptop, and the feet on the canvas's bottom row. */
const OX = 6;
const OY = 10;
export const PET_X = OX;
export const PET_Y = OY;
export const PET_W = 24;
export const PET_H = 16;

export type ArmPose = "rest" | "up" | "high" | "down" | "reach";
export type EyeState = "open" | "closed" | "wide" | "happy" | "x";
/** The mouth the pet only has while yawning: a small o, open, wide open. */
export type Mouth = "o" | "open" | "yawn";
export type Side = -1 | 1;
export type Effect =
  | "zzz1"
  | "zzz2"
  | "sparkleA"
  | "sparkleB"
  | "heart"
  | "sweat1"
  | "sweat2"
  | "exclaim"
  | "question"
  | "pause";

export interface Pose {
  /** Whole-body offset in cells; negative dy jumps up. */
  dx?: number;
  dy?: number;
  /** 1 lowers the torso's top edge by one row (breathing, squish, sitting). */
  squash?: 0 | 1;
  /** Raises the torso's top edge by that many rows, as in a stretch; the face
   * and arms go up with it, the feet stay put. */
  stretch?: 0 | 1 | 2;
  armL?: ArmPose;
  armR?: ArmPose;
  /** Length (0..4) of each of the four legs, left to right. */
  legs?: readonly [number, number, number, number];
  /** Sideways shift of each leg in cells, left to right, as in a stride. */
  legDx?: readonly [number, number, number, number];
  /** Turns three-quarters toward a side: the far edge is shaded, the eyes follow. */
  turn?: Side;
  /** The outermost leg on that side stretches out to kick. */
  kick?: Side;
  eyes?: EyeState;
  /** Eye offset, each component in -1..1. */
  gaze?: readonly [number, number];
  mouth?: Mouth;
  /** A tear at the corner of the left eye, as after a big yawn. */
  tear?: boolean;
  fx?: readonly Effect[];
}

export interface Frame {
  ms: number;
  pose?: Pose;
  /** Raw canvas rows, used by the frames recovered from the reference videos. */
  rows?: readonly string[];
}

export interface Anim {
  frames: readonly Frame[];
  loop: boolean;
  /** Looping animations replay from this frame; earlier ones play once as an intro. */
  loopFrom?: number;
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

/** Where each arm pose sits, in rows from the top of the torso; "reach" is
 * above the head. */
const ARM_Y: Record<ArmPose, number> = { reach: -3, high: 0, up: 2, rest: 4, down: 6 };
/** Each mouth, from the top of the torso, centred between the eyes. */
const MOUTHS: Record<Mouth, { x: number; y: number; w: number; h: number }> = {
  o: { x: 11, y: 6, w: 2, h: 1 },
  open: { x: 11, y: 5, w: 2, h: 2 },
  yawn: { x: 10, y: 5, w: 4, h: 3 },
};
const ARM_W = 4;
const ARM_H = 4;
const LEG_X = [4, 8, 14, 18] as const;
const LEG_W = 2;
const EYE_X = [6, 16] as const;
const EYE_W = 2;
const TORSO_X = 4;
const TORSO_W = 16;
const TORSO_H = 12;

/** Effects, in canvas coordinates. */
const GLYPHS: Record<Effect, { x: number; y: number; rows: readonly string[] }> = {
  zzz1: { x: 26, y: 3, rows: ["ZZZZ", "..Z.", ".Z..", "ZZZZ"] },
  zzz2: { x: 31, y: 0, rows: ["ZZZ", "..Z", ".Z.", "ZZZ"] },
  sparkleA: { x: 1, y: 9, rows: [".Y.", "YYY", ".Y."] },
  sparkleB: { x: 32, y: 8, rows: [".Y.", "YYY", ".Y."] },
  heart: { x: 26, y: 2, rows: [".R.R.", "RRRRR", ".RRR.", "..R.."] },
  // A drop perched on the head's top right corner, in two shapes so it can
  // swell and settle while the pet struggles.
  sweat1: { x: 24, y: 9, rows: [".S", "SS", "SS"] },
  sweat2: { x: 24, y: 9, rows: [".S", "SS"] },
  exclaim: { x: 30, y: 4, rows: ["R", "R", "R", ".", "R"] },
  question: { x: 29, y: 3, rows: ["ZZ.", "..Z", ".Z.", "...", ".Z."] },
  pause: { x: 29, y: 4, rows: ["Z.Z", "Z.Z", "Z.Z"] },
};

/** Renders a pose into GRID_H strings of GRID_W palette characters. */
export function compose(pose: Pose = {}): string[] {
  const g = blank();
  // The top of the torso, in rows below the unsquashed top: everything above
  // the hips hangs from it.
  const sq = (pose.squash ?? 0) - (pose.stretch ?? 0);
  const bx = (x: number) => OX + (pose.dx ?? 0) + x;
  const by = (y: number) => OY + (pose.dy ?? 0) + y;

  const legs = pose.legs ?? [4, 4, 4, 4];
  const turn = pose.turn ?? 0;
  legs.forEach((len, i) => {
    const x = LEG_X[i] + (pose.legDx?.[i] ?? 0);
    if ((pose.kick === 1 && i === 3) || (pose.kick === -1 && i === 0)) {
      // The outermost leg stretches out toward the ball.
      const s = pose.kick;
      for (const [ox, oy] of [[0, 12], [1, 12], [2, 13], [3, 13], [1, 13], [2, 14]]) {
        put(g, bx(x + s * ox), by(oy), "O");
      }
    } else {
      // Far legs sit a row higher when the pet is turned; the rest hang from the hip.
      rect(g, bx(x), by(12 + (turn && i === 0 && turn === 1 ? 1 : 0)), LEG_W, len, "O");
    }
  });

  rect(g, bx(TORSO_X), by(sq), TORSO_W, TORSO_H - sq, "O");
  const shadeL = turn === 1;
  const shadeR = turn === -1;
  if (shadeL) rect(g, bx(TORSO_X), by(sq), 1, TORSO_H - sq, "D");
  if (shadeR) rect(g, bx(TORSO_X + TORSO_W - 1), by(sq), 1, TORSO_H - sq, "D");
  rect(g, bx(0), by(ARM_Y[pose.armL ?? "rest"] + sq), ARM_W, ARM_H, shadeL ? "D" : "O");
  rect(g, bx(20), by(ARM_Y[pose.armR ?? "rest"] + sq), ARM_W, ARM_H, shadeR ? "D" : "O");

  const [gx, gy] = pose.gaze ?? [0, 0];
  for (const ex of EYE_X) {
    const x = bx(ex + gx);
    const y = by(2 + sq + gy);
    switch (pose.eyes ?? "open") {
      case "open":
        rect(g, x, y, EYE_W, 2, "E");
        break;
      case "closed":
        rect(g, x, y + 1, EYE_W, 1, "E");
        break;
      case "wide":
        rect(g, x, y - 1, EYE_W, 3, "E");
        break;
      case "happy":
        stamp(g, x - 1, y, [".EE.", "E..E"]);
        break;
      case "x":
        stamp(g, x - 1, y - 1, ["E.E", ".E.", "E.E"]);
        break;
    }
  }

  if (pose.mouth) {
    const m = MOUTHS[pose.mouth];
    rect(g, bx(m.x), by(m.y + sq), m.w, m.h, "E");
  }
  if (pose.tear) rect(g, bx(EYE_X[0] - 1), by(4 + sq), 1, 2, "S");

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
  | "sleep"
  | "wake"
  | "celebrate"
  | "poke"
  | "hearts"
  | "soccer"
  | "wave"
  | "dragged"
  | "noperm"
  | "lookAround"
  | "stretch"
  | "walk";

const UP = { armL: "up", armR: "up" } as const;

/** The rows a frame draws: raw rows from the reference, or a composed pose. */
export function frameRows(frame: Frame): readonly string[] {
  return frame.rows ?? compose(frame.pose);
}

const BREATHE: readonly Frame[] = [{ ms: 1400, pose: {} }, { ms: 500, pose: { squash: 1 } }];

/** A stride of the walk (see ANIMS.walk). */
const WALK: readonly Frame[] = [
  { ms: 100, pose: { legs: [4, 3, 4, 3], legDx: [0, 1, 0, 1] } },
  { ms: 100, pose: { legs: [4, 4, 4, 4], squash: 1 } },
  { ms: 100, pose: { legs: [3, 4, 3, 4], legDx: [1, 0, 1, 0] } },
  { ms: 100, pose: { legs: [4, 4, 4, 4], squash: 1 } },
];

export const ANIMS: Record<AnimName, Anim> = {
  idle: { loop: true, frames: BREATHE },
  // Drawn after the reference video: get the laptop out, sit down, type, then
  // put it away. The typing cycle's pace follows the typing speed at runtime.
  typing: {
    loop: true,
    loopFrom: TYPING_INTRO.length,
    frames: [...TYPING_INTRO.map(raw), ...TYPING_LOOP.map(raw)],
  },
  typingEnd: { loop: false, frames: TYPING_OUTRO.map(raw) },
  click: {
    loop: false,
    frames: [
      { ms: 120, pose: { ...UP } },
      { ms: 160, pose: { squash: 1 } },
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
      { ms: 200, pose: { eyes: "wide", fx: ["sweat1"] } },
      { ms: 150, pose: { eyes: "wide", fx: ["sweat2"] } },
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
  // Recovered from the reference video: crouch, juggle the ball up the right
  // side, over the head, up and down both sides, then boot it away.
  soccer: { loop: false, frames: [...SOCCER.map(raw), ...SOCCER_IDLE_AFTER.map(raw)] },
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
  // Also played now and then while idle, when picked as the idle animation.
  lookAround: {
    loop: false,
    frames: [
      { ms: 1100, pose: { turn: -1 } },
      { ms: 400, pose: {} },
      { ms: 1100, pose: { turn: 1 } },
      { ms: 400, pose: {} },
    ],
  },
  dragged: {
    loop: true,
    frames: [
      // The drop stays for the whole struggle, swelling and settling rather
      // than blinking on and off, which read as a glitch.
      { ms: 160, pose: { ...UP, legs: [2, 4, 2, 4], eyes: "wide", fx: ["sweat1"] } },
      { ms: 160, pose: { ...UP, legs: [4, 2, 4, 2], eyes: "wide", fx: ["sweat2"] } },
    ],
  },
  noperm: {
    loop: true,
    frames: [
      { ms: 1200, pose: { eyes: "x", fx: ["question"] } },
      { ms: 600, pose: { eyes: "x", squash: 1 } },
    ],
  },
  // The break reminder: settle, then a yawn that grows into a stretch with the
  // arms up over the head, a tear from the yawn, and a contented look after.
  stretch: {
    loop: false,
    frames: [
      { ms: 300, pose: { squash: 1, eyes: "closed" } },
      { ms: 250, pose: { eyes: "closed", mouth: "o" } },
      { ms: 300, pose: { stretch: 1, ...UP, eyes: "closed", mouth: "open" } },
      { ms: 700, pose: { stretch: 2, armL: "reach", armR: "reach", eyes: "closed", mouth: "yawn" } },
      { ms: 450, pose: { stretch: 2, armL: "reach", armR: "reach", eyes: "closed", mouth: "yawn", tear: true } },
      { ms: 350, pose: { stretch: 1, ...UP, eyes: "closed", mouth: "open", tear: true } },
      { ms: 550, pose: { eyes: "happy", tear: true } },
      { ms: 250, pose: { squash: 1, eyes: "happy" } },
    ],
  },
  // A sideways scuttle, facing you, drawn for walking right. The legs go in two
  // pairs, the first and third and the second and fourth: one pair stays down
  // while the other is lifted and reaches a cell ahead, and the body dips as
  // they change over. Walking left, the reach is turned round; the pace
  // follows the walking speed.
  walk: {
    loop: true,
    frames: WALK,
  },
};

/** Wraps a stored frame from the reference videos as an animation frame. */
function raw(f: RawFrame): Frame {
  return { ms: f.ms, rows: f.rows };
}
