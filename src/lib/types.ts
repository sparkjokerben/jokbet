// Mirrors the Rust types serialized over IPC (camelCase).

/** Pixels per sprite cell; the slider covers the same range as PET_SCALE_* in Rust. */
export const PET_SCALE_MIN = 3;
export const PET_SCALE_MAX = 7;
export const PET_SCALE_DEFAULT = 5;
/** The tint slider's range, in percent; matches GLASS_TINT_MAX in Rust. */
export const GLASS_TINT_MAX = 60;
export type CounterKind = "today" | "rate";
export type Period = "daily" | "lifetime";
export type Metric = "keys" | "clicks" | "scrolls" | "distance";
export type Activity = "typing" | "click";
export type Permission = "granted" | "denied" | "notRequired" | "unsupported";
export type IdleAnim = "breathe" | "soccer" | "lookAround";
export type ActionAnim = "poke" | "hearts" | "soccer" | "wave";

export interface HeadCounter {
  enabled: boolean;
  kind: CounterKind;
  keyboard: boolean;
  mouse: boolean;
}

export interface CustomMilestone {
  id: string;
  period: Period;
  metric: Metric;
  threshold: number;
  repeat: boolean;
}

export interface Settings {
  petScale: number;
  petPosition: [number, number] | null;
  headCounter: HeadCounter;
  idleAnim: IdleAnim;
  clickAnim: ActionAnim;
  doubleClickAnim: ActionAnim;
  bubble: boolean;
  /** Draw the hover bubble with the system's glass material. */
  liquidGlass: boolean;
  /** How much the card tints that material, in percent. */
  glassTint: number;
  typingSpeed: boolean;
  milestones: boolean;
  customMilestones: CustomMilestone[];
  sleepAfterMin: number;
  paused: boolean;
  onboarded: boolean;
}

export interface Totals {
  keys: number;
  clickLeft: number;
  clickRight: number;
  clickMiddle: number;
  scrolls: number;
  movePx: number;
  moveMm: number;
}

export interface Tick {
  today: Totals;
  /** Keys and clicks per second over the last few seconds. */
  kps: number;
  cps: number;
  activity: Activity | null;
}

export interface Status {
  permission: Permission;
  listening: boolean;
  secureInput: boolean;
  paused: boolean;
}



export interface MilestoneHit {
  id: string;
  period: Period;
  metric: Metric;
  level: number;
}
