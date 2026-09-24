// The one JSON door to the bucket, shared by /latest and /changelog.json.
//
// Same origin as the page, so there is no CORS and one place knows about R2: it
// tries the bucket, then the copy baked into the site, and says which one
// answered, so the page can be honest about where its numbers come from.

/// <reference types="@cloudflare/workers-types" />

export interface Env {
  /** The R2 bucket holding the installers and the published JSON. */
  DOWNLOADS: R2Bucket;
  /** The site's own static files, for the baked fallbacks. */
  ASSETS: Fetcher;
}

/** How long an isolate keeps an object to itself before asking the bucket again. */
const TTL_MS = 60_000;

/** The last answer per key, so a busy minute does not hammer the bucket. */
const cache = new Map<string, { at: number; body: string }>();

const json = (body: string) =>
  new Response(body, {
    status: 200,
    headers: {
      "content-type": "application/json; charset=utf-8",
      "cache-control": "public, max-age=60",
    },
  });

/** The object from the bucket, else the baked copy, else `empty`. */
async function read(
  request: Request,
  env: Env,
  key: string,
  baked: string,
  empty: Record<string, unknown>,
): Promise<string> {
  try {
    const object = await env.DOWNLOADS.get(key);
    if (object) {
      const body = (await object.json()) as Record<string, unknown>;
      return JSON.stringify({ ...body, source: "r2" });
    }
  } catch {
    // Bucket missing, binding missing, R2 down: the baked copy is next.
  }
  try {
    const response = await env.ASSETS.fetch(new URL(baked, request.url));
    if (response.ok) {
      const body = (await response.json()) as Record<string, unknown>;
      return JSON.stringify({ ...body, source: "baked" });
    }
  } catch {
    // No baked copy either: the page falls back to the links it already has.
  }
  return JSON.stringify(empty);
}

/** GET (or HEAD) one such object, with its own minute of memory. */
export async function mirror(
  request: Request,
  env: Env,
  key: string,
  baked: string,
  empty: Record<string, unknown>,
): Promise<Response> {
  if (request.method !== "GET" && request.method !== "HEAD") {
    return new Response("Method not allowed", { status: 405, headers: { allow: "GET, HEAD" } });
  }
  const remembered = cache.get(key);
  if (!remembered || Date.now() - remembered.at >= TTL_MS) {
    cache.set(key, { at: Date.now(), body: await read(request, env, key, baked, empty) });
  }
  return json(cache.get(key)!.body);
}
