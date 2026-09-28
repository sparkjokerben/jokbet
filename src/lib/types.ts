// Mirrors the Rust types serialized over IPC (camelCase).

/** Pixels per sprite cell; the slider covers the same range as PET_SCALE_* in Rust. */
export const PET_SCALE_MIN = 3;
export const PET_SCALE_MAX = 7;
export const PET_SCALE_DEFAULT = 3.5;
/** The tint slider's range, in percent; matches GLASS_TINT_MAX in Rust. */
export const GLASS_TINT_MAX = 60;
export type CounterKind = "today" | "rate";
export type Period = "daily" | "lifetime";
export type Metric = "keys" | "clicks" | "inputs" | "scrolls" | "distance";
/** What the engine saw since the last tick; a scroll only keeps the pet awake. */
export type Activity = "typing" | "click" | "scroll";
export type Permission = "granted" | "denied" | "notRequired" | "unsupported";
/** What the pet may do now and then while idle; in between it breathes. */
export type IdleAnim = "soccer" | "lookAround" | "walk";
export type ActionAnim = "poke" | "hearts" | "soccer" | "wave";
/** The key heatmap's colours: the usual heat scale, or the pet's orange. */
export type HeatmapPalette = "heat" | "brand";
/** The interface language: "system" follows the system locale. */
export type Language = "system" | "zh" | "en";

export interface HeadCounter {
  enabled: boolean;
  kind: CounterKind;
  keyboard: boolean;
  mouse: boolean;
}

/** Global shortcuts as accelerators ("Control+Alt+KeyJ"); null is unset. */
export interface Shortcuts {
  togglePet: string | null;
  pause: string | null;
}

export interface CustomMilestone {
  id: string;
  period: Period;
  metric: Metric;
  threshold: number;
  repeat: boolean;
}

/** A nudge to take a break after a long stretch at the keyboard. */
export interface RestReminder {
  enabled: boolean;
  /** Minutes without a key, click or scroll that count as a rest. */
  gapMin: number;
  /** Minutes of unbroken activity before the reminder. */
  afterMin: number;
}
/** The ranges offered for RestReminder; they match REST_* in Rust. */
export const REST_GAP_MIN = [1, 15] as const;
export const REST_AFTER_MIN = [15, 120] as const;

/** The backup the app makes by itself once a day. */
export interface AutoBackup {
  enabled: boolean;
  /** Where they go; null is the backups folder in the app's data. */
  dir: string | null;
  /** How many of this computer's daily backups to keep. */
  keep: number;
}
/** The range of AutoBackup.keep; matches BACKUP_KEEP_MAX in Rust. */
export const BACKUP_KEEP_MAX = 60;

export interface Settings {
  petScale: number;
  petPosition: [number, number] | null;
  headCounter: HeadCounter;
  /** Empty is just breathing. */
  idleAnims: IdleAnim[];
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
  language: Language;
  /** Take the pet off screen while another app is full screen or presenting. */
  hideInFullscreen: boolean;
  shortcuts: Shortcuts;
  heatmapPalette: HeatmapPalette;
  autoBackup: AutoBackup;
  restReminder: RestReminder;
}

export type BackupKind = "manual" | "auto" | "safety";

/** A backup as the restore list shows it (BackupInfo in Rust). */
export interface BackupInfo {
  path: string;
  kind: BackupKind;
  /** RFC 3339, local time. */
  createdAt: string;
  /** How many days have counts. */
  days: number;
  /** The app version that made it. */
  version: string;
  size: number;
  /** Made on another computer. */
  otherDevice: boolean;
}

export interface BackupList {
  /** Where the daily backups go. */
  dir: string;
  /** The app's own backup folder. */
  defaultDir: string;
  items: BackupInfo[];
  /** Why the last daily backup failed, if it did. */
  lastError: string | null;
}

export interface RestoreReport {
  /** Settings left as they were: "shortcuts", or "*" for all of them. */
  settingsSkipped: string[];
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
  paused: boolean;
  /** The database is open; without it counts are only kept in memory. */
  storage: boolean;
}

export interface DayStat extends Totals {
  date: string;
}

export interface Stats {
  /** Oldest first, ending today; days without data are zero. */
  days: DayStat[];
  /** Per-key counts over the same range. */
  keys: Record<string, number>;
  /** Totals per hour of the day over the same range; index 0 is midnight. */
  hours: Totals[];
  /**
   * Every day that has counts, oldest first. Days without counts are never
   * stored, so this is sparse and shorter than the span it covers.
   */
  history: DayStat[];
  lifetime: Totals;
  /** Every key pressed on any day. */
  everPressed: string[];
}

export interface MilestoneHit {
  id: string;
  period: Period;
  metric: Metric;
  level: number;
}

/** Where updating stands (UpdateStatus in Rust). */
export type UpdateStatus =
  | { state: "idle" | "checking" | "upToDate" }
  | { state: "ready"; version: string; notes: string | null; date: string | null }
  | { state: "failed"; error: string };
