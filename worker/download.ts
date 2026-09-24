// GET /dl/<name> — the installers, served from R2, with a way out.
//
// The bucket is private and the page never talks to it: this route streams the
// object (ranges and all), and when the object is not there — or R2 cannot be
// reached at all — it hands the browser over to the same file on GitHub.
// A download link on the site therefore cannot dead-end.

/// <reference types="@cloudflare/workers-types" />

import type { Env } from "./mirror.ts";

/** Where the GitHub copies live; the repo is the one in scripts/site-manifest.ts. */
const REPO = "sparkjokerben/jokbet";
const RELEASES = `https://github.com/${REPO}/releases`;

/** The names Tauri produces, and nothing else: no slashes, no traversal. */
const NAME = /^(?:Jokbet|jokbet)_[A-Za-z0-9][A-Za-z0-9._-]{0,180}$/;
const VERSION = /^\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$/;
/** Version-stamped names never change under a URL, so they may be cached hard. */
const IMMUTABLE = "public, max-age=31536000, immutable";

/** R2 keeps what was uploaded (aws s3 cp guesses by extension); this is the belt. */
const TYPES: [string, string][] = [
  [".dmg", "application/x-apple-diskimage"],
  [".msi", "application/x-msi"],
  [".exe", "application/vnd.microsoft.portable-executable"],
  [".deb", "application/vnd.debian.binary-package"],
  [".AppImage", "application/octet-stream"],
  [".tar.gz", "application/gzip"],
  [".sig", "text/plain; charset=utf-8"],
];

const contentType = (name: string) =>
  TYPES.find(([extension]) => name.endsWith(extension))?.[1] ?? "application/octet-stream";

export async function download(request: Request, env: Env): Promise<Response> {
  const name = new URL(request.url).pathname.slice("/dl/".length);
  if (name.includes("/") || !NAME.test(name)) return new Response("Not found", { status: 404 });

  const method = request.method;
  // What the answer should be is the request's business, not the bucket's: R2
  // reports a range covering the whole object even when nothing asked for one,
  // and a 206 to a request that carried no Range header is a broken download to
  // a browser — Safari refuses to save the file at all. So ask the request.
  const asked = request.headers.has("range");
  if (method === "GET" || method === "HEAD") {
    try {
      // HEAD asks the bucket for the metadata only, so a probe costs nothing.
      const object =
        method === "HEAD"
          ? await env.DOWNLOADS.head(name)
          : await env.DOWNLOADS.get(name, { range: request.headers });
      if (object && object.size > 0) return served(object, method, name, asked);
    } catch {
      // R2 unreachable, bucket gone: GitHub is the fallback.
    }
  }
  return github(request, env, name);
}

function served(object: R2Object | R2ObjectBody, method: string, name: string, asked: boolean): Response {
  const headers = new Headers();
  object.writeHttpMetadata(headers);
  headers.set("content-type", contentType(name));
  headers.set("etag", object.httpEtag);
  headers.set("accept-ranges", "bytes");
  headers.set("content-disposition", `attachment; filename="${name}"`);
  headers.set("cache-control", IMMUTABLE);
  headers.set("x-download-source", "r2");
  // An installer is not a page: keep it out of search results entirely.
  headers.set("x-robots-tag", "noindex");
  const body = method === "HEAD" || !("body" in object) ? null : object.body;
  // A probe gets the whole length rather than a range it cannot describe: the
  // bucket's metadata read knows the size, not what the request asked for.
  const range = asked && method !== "HEAD" ? object.range : undefined;
  if (range && "offset" in range && typeof range.offset === "number" && typeof range.length === "number") {
    headers.set("content-range", `bytes ${range.offset}-${range.offset + range.length - 1}/${object.size}`);
    headers.set("content-length", String(range.length));
    return new Response(body, { status: 206, headers });
  }
  headers.set("content-length", String(object.size));
  return new Response(body, { status: 200, headers });
}

/** The same file on GitHub, or the releases page when even the version is unknown. */
async function github(request: Request, env: Env, name: string): Promise<Response> {
  // The page says which version it is offering; only the probe (or a stale
  // bookmark) has to ask the bucket.
  const hint = new URL(request.url).searchParams.get("v");
  let version = hint && VERSION.test(hint) ? hint : null;
  if (!version) {
    try {
      const object = await env.DOWNLOADS.get("latest.json");
      const manifest = object ? ((await object.json()) as { version?: unknown }) : null;
      if (typeof manifest?.version === "string" && VERSION.test(manifest.version)) version = manifest.version;
    } catch {
      // fall through to the releases page
    }
  }
  const location = version ? `https://github.com/${REPO}/releases/download/v${version}/${name}` : RELEASES;
  return new Response(null, {
    status: 302,
    headers: {
      location,
      "cache-control": "no-store",
      "x-download-source": "github",
      "x-robots-tag": "noindex",
    },
  });
}
