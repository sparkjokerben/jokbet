// GET /dl/<tag>/<name> — every file of every release, from the mirror first.
//
// The bucket keeps each release under releases/<tag>/, named exactly as on
// GitHub, so this path and GitHub's releases/download/<tag>/<name> are one
// address on two hosts. That is the whole fallback: when the object is not in
// the bucket, or R2 cannot be reached, the browser is sent to the same file on
// GitHub, and the app's updater does the same in the other direction when a
// download fails (src-tauri/src/updater.rs). A link on the site cannot
// dead-end, and a version in the path means a URL never changes what it serves.

/// <reference types="@cloudflare/workers-types" />

import { GITHUB_RELEASES, type Env } from "./api.ts";

const TAG = /^v\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$/;
/** The names Tauri produces, and nothing else. */
const NAME = /^(?:Jokbet|jokbet)_[A-Za-z0-9][A-Za-z0-9._-]{0,180}$/;
const VERSION = /^\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$/;
/** The path carries the version, so what it serves never changes. */
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

const notFound = () => new Response("Not found", { status: 404 });

const redirect = (location: string, status: 301 | 302) =>
  new Response(null, {
    status,
    headers: {
      location,
      "cache-control": status === 301 ? "public, max-age=86400" : "no-store",
      "x-robots-tag": "noindex",
      ...(status === 302 ? { "x-download-source": "github" } : {}),
    },
  });

export async function download(request: Request, env: Env): Promise<Response> {
  const parts = new URL(request.url).pathname.slice("/dl/".length).split("/");
  if (parts.length === 1) return legacy(request, parts[0]);
  const [tag, name] = parts;
  if (parts.length !== 2 || !TAG.test(tag) || !NAME.test(name)) return notFound();

  const method = request.method;
  if (method !== "GET" && method !== "HEAD") return notFound();
  // What the answer should be is the request's business, not the bucket's: R2
  // reports a range covering the whole object even when nothing asked for one,
  // and a 206 to a request that carried no Range header is a broken download to
  // a browser. So ask the request.
  const asked = request.headers.has("range");
  try {
    const key = `releases/${tag}/${name}`;
    // HEAD asks the bucket for the metadata only, so a probe costs nothing.
    const object = method === "HEAD" ? await env.DOWNLOADS.head(key) : await env.DOWNLOADS.get(key, { range: request.headers });
    if (object && object.size > 0) return served(object, method, name, asked);
  } catch {
    // R2 unreachable, bucket gone: GitHub is the fallback.
  }
  return redirect(`${GITHUB_RELEASES}/download/${tag}/${name}`, 302);
}

/** The 0.1.0 page linked /dl/<name>?v=<version>; those links still land. */
function legacy(request: Request, name: string): Response {
  if (!NAME.test(name)) return notFound();
  const hint = new URL(request.url).searchParams.get("v");
  const version = hint && VERSION.test(hint) ? hint : name.match(/^(?:Jokbet|jokbet)_(\d+\.\d+\.\d+)_/)?.[1];
  if (!version) return redirect(GITHUB_RELEASES, 302);
  return redirect(new URL(`/dl/v${version}/${name}`, request.url).toString(), 301);
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
  // A file download is not a page: keep it out of search results entirely.
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
