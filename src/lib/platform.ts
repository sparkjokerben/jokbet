// Which desktop the page runs on, from the webview's user agent.

export type Platform = "mac" | "windows" | "linux";

export function detectPlatform(userAgent: string | undefined): Platform {
  if (!userAgent) return "linux";
  if (/Mac/.test(userAgent)) return "mac";
  if (/Win/.test(userAgent)) return "windows";
  return "linux";
}

export const platform: Platform = detectPlatform(typeof navigator === "undefined" ? undefined : navigator.userAgent);
