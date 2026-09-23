// GET /dl/<name> — the installers, served from R2, with a way out.
//
// The bucket is private and the page never talks to it: this function streams
// the object (ranges and all), and when the object is not there — or R2 cannot
// be reached at all — it hands the browser over to the same file on GitHub.
// A download link on the site therefore cannot dead-end.

/// <reference types="@cloudflare/workers-types" />

interface Env {
  DOWNLOADS: R2Bucket;
  ASSETS: Fetcher;
}

/** Where the GitHub copies live; the repo is the one in scripts/site-manifest.ts. */
const REPO = "sparkjokerben/jokerben-desktop-pet";
const RELEASES = `https://github.com/${REPO}/releases`;

/** The names Tauri produces, and nothing else: no slashes, no traversal. */
const NAME = /^jokerben-desktop-pet_[A-Za-z0-9][A-Za-z0-9._-]{0,180}$/;
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

export const onRequest: PagesFunction<Env> = async (context) => {
  const segments = Array.isArray(context.params.path) ? context.params.path : [context.params.path ?? ""];
  const name = segments.length === 1 ? segments[0] : "";
  if (!NAME.test(name)) return new Response("Not found", { status: 404 });

  const method = context.request.method;
  if (method === "GET" || method === "HEAD") {
    try {
      // HEAD asks the bucket for the metadata only, so a probe costs nothing.
      const object =
        method === "HEAD"
          ? await context.env.DOWNLOADS.head(name)
          : await context.env.DOWNLOADS.get(name, { range: context.request.headers });
      if (object && object.size > 0) return served(object, method, name);
    } catch {
      // R2 unreachable, bucket gone: GitHub is the fallback.
    }
  }
  return github(context, name);
};

function served(object: R2Object | R2ObjectBody, method: string, name: string): Response {
  const headers = new Headers();
  object.writeHttpMetadata(headers);
  headers.set("content-type", contentType(name));
  headers.set("etag", object.httpEtag);
  headers.set("accept-ranges", "bytes");
  headers.set("content-disposition", `attachment; filename="${name}"`);
  headers.set("cache-control", IMMUTABLE);
  headers.set("x-download-source", "r2");
  const body = method === "HEAD" || !("body" in object) ? null : object.body;
  const range = object.range;
  if (range && "offset" in range && typeof range.offset === "number" && typeof range.length === "number") {
    headers.set("content-range", `bytes ${range.offset}-${range.offset + range.length - 1}/${object.size}`);
    headers.set("content-length", String(range.length));
    return new Response(body, { status: 206, headers });
  }
  headers.set("content-length", String(object.size));
  return new Response(body, { status: 200, headers });
}

/** The same file on GitHub, or the releases page when even the version is unknown. */
async function github(context: EventContext<Env, string, unknown>, name: string): Promise<Response> {
  // The page says which version it is offering; only the probe (or a stale
  // bookmark) has to ask the bucket.
  const hint = new URL(context.request.url).searchParams.get("v");
  let version = hint && VERSION.test(hint) ? hint : null;
  if (!version) {
    try {
      const object = await context.env.DOWNLOADS.get("latest.json");
      const manifest = object ? ((await object.json()) as { version?: unknown }) : null;
      if (typeof manifest?.version === "string" && VERSION.test(manifest.version)) version = manifest.version;
    } catch {
      // fall through to the releases page
    }
  }
  const location = version ? `https://github.com/${REPO}/releases/download/v${version}/${name}` : RELEASES;
  return new Response(null, {
    status: 302,
    headers: { location, "cache-control": "no-store", "x-download-source": "github" },
  });
}
