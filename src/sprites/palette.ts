// One character per pixel. "." is transparent.
export const PALETTE: Readonly<Record<string, string>> = {
  O: "#D97757", // body (Claude orange)
  D: "#B86245", // body turned away (shaded edge)
  E: "#1F1E1D", // eyes, the ball's dark checks
  B: "#3A3836", // blindfold
  W: "#FFFFFF", // the ball's white checks
  G: "#8E8E93", // laptop
  Y: "#F4C542", // sparkle
  S: "#6EC1F0", // sweat
  R: "#E5484D", // heart / alert
  Z: "#7C8DB5", // sleep Zzz, question mark, pause
};

export const TRANSPARENT = ".";
