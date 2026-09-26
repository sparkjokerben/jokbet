// `tauri icon` writes an android/ and an ios/ set beside the desktop icons it
// is asked for, and the app ships neither of them. Removing them here rather
// than with `rm -rf`, which npm's shell on Windows — cmd.exe — does not have.
//
//   node scripts/clean-icons.ts

import { rmSync } from "node:fs";

for (const dir of ["android", "ios"]) {
  rmSync(`src-tauri/icons/${dir}`, { recursive: true, force: true });
}
