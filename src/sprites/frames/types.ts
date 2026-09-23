/** A frame stored as raw canvas rows, straight from the reference animation. */
export interface RawFrame {
  ms: number;
  rows: readonly string[];
}
