// Sends the page's uncaught errors to the app's log file, where a bug report
// can pick them up; the webview's own console is gone once the window closes.

import { error } from "@tauri-apps/plugin-log";

function describe(reason: unknown): string {
  if (reason instanceof Error) return reason.stack ?? `${reason.name}: ${reason.message}`;
  return String(reason);
}

export function forwardErrors(page: string) {
  const send = (what: string) => void error(`[${page}] ${what}`).catch(() => {});
  window.addEventListener("error", (e) => send(describe(e.error ?? e.message)));
  window.addEventListener("unhandledrejection", (e) => send(`unhandled rejection: ${describe(e.reason)}`));
}
