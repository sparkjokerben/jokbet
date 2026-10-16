// What the mirror is asked for, and by whom: one row per download, written off
// the response path.
//
// The bucket is the only door to the release files (worker/download.ts), so a
// file it serves is a download this module can see. GitHub's own counter is the
// other half of the site's total, and the two never meet: a file served from the
// bucket is never fetched from GitHub, and a request that fell back there is
// GitHub's to count, not ours. Adding the two up is /api/downloads.json's job —
// what the bucket served (here) plus GitHub's numbers, which the "Refresh the
// download counts" workflow leaves in the bucket as manifests/downloads.json.

/// <reference types="@cloudflare/workers-types" />

import { type Env } from "./api.ts";

/** The kinds of client worth telling apart. The user agent itself is never
 * stored — only which of these it was. */
const CLIENTS: [client: string, pattern: RegExp][] = [
  // The app's own updater (tauri-plugin-updater) fetching an update bundle.
  ["updater", /tauri|wry/i],
  // A terminal or a script: curl, wget, PowerShell's Invoke-WebRequest…
  ["script", /curl|wget|powershell|libwww|python|go-http|node|httpie|aria2|okhttp|java/i],
  // A browser — and the last test, because PowerShell calls itself Mozilla/5.0
  // too, and so do a few command-line tools.
  ["browser", /mozilla|chrome|safari|firefox|edg/i],
];

/** Which kind of client asked: "browser", "script", "updater", or "other" when
 * nothing here knows it. @param request */
export function clientOf(request: Request): string {
  const agent = request.headers.get("user-agent") ?? "";
  return CLIENTS.find(([, pattern]) => pattern.test(agent))?.[0] ?? "other";
}

/** One row for one download, in the background: a visitor is downloading a
 * file, not waiting for a count, and a database that refuses may never be the
 * reason a download fails — so this waits on nothing and swallows everything.
 * @param request @param env @param ctx @param tag @param name */
export function record(
  request: Request,
  env: Env,
  ctx: ExecutionContext | undefined,
  tag: string,
  name: string,
): void {
  try {
    // No binding (a deploy before it, a test without one): the download is the
    // only thing that matters.
    if (!env.STATS) return;
    const at = Date.now();
    const write = env.STATS
      .prepare("INSERT INTO downloads (at, day, tag, name, client, country) VALUES (?, ?, ?, ?, ?, ?)")
      // The day is UTC, and must stay equal to date(at / 1000, 'unixepoch').
      .bind(at, new Date(at).toISOString().slice(0, 10), tag, name, clientOf(request), request.cf?.country ?? null)
      .run()
      .then(
        () => {},
        () => {},
      );
    // waitUntil also keeps the isolate alive for a write the response has
    // already raced past.
    if (ctx) ctx.waitUntil(write);
    else void write;
  } catch {
    // Nothing here can be allowed to break a download.
  }
}
