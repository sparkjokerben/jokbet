// Mirrors the Rust types serialized over IPC (camelCase).

export type PetSize = "small" | "medium" | "large";
export type CounterKind = "today" | "rate";
export type Period = "daily" | "lifetime";
export type Metric = "keys" | "clicks" | "scrolls" | "distance";
export type Activity = "typing" | "click" | "scroll";
export type Permission = "granted" | "denied" | "notRequired";

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
  petSize: PetSize;
  petPosition: [number, number] | null;
  headCounter: HeadCounter;
  bubble: boolean;
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
  kpm: number;
  cpm: number;
  activity: Activity | null;
}

export interface Status {
  permission: Permission;
  listening: boolean;
  secureInput: boolean;
  paused: boolean;
}

export const SCALE: Record<PetSize, number> = { small: 4, medium: 6, large: 8 };
