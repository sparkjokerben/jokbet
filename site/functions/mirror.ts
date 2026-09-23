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
  context: EventContext<Env, string, unknown>,
  key: string,
  baked: string,
  empty: Record<string, unknown>,
): Promise<string> {
  try {
    const object = await context.env.DOWNLOADS.get(key);
    if (object) {
      const body = (await object.json()) as Record<string, unknown>;
      return JSON.stringify({ ...body, source: "r2" });
    }
  } catch {
    // Bucket missing, binding missing, R2 down: the baked copy is next.
  }
  try {
    const response = await context.env.ASSETS.fetch(new URL(baked, context.request.url));
    if (response.ok) {
      const body = (await response.json()) as Record<string, unknown>;
      return JSON.stringify({ ...body, source: "baked" });
    }
  } catch {
    // No baked copy either: the page falls back to the links it already has.
  }
  return JSON.stringify(empty);
}

/** A handler for one such object, with its own minute of memory. */
export function mirror(key: string, baked: string, empty: Record<string, unknown>) {
  let cached: { at: number; body: string } | null = null;
  return async (context: EventContext<Env, string, unknown>): Promise<Response> => {
    if (context.request.method !== "GET" && context.request.method !== "HEAD") {
      return new Response("Method not allowed", { status: 405, headers: { allow: "GET, HEAD" } });
    }
    if (!cached || Date.now() - cached.at >= TTL_MS) {
      cached = { at: Date.now(), body: await read(context, key, baked, empty) };
    }
    return json(cached.body);
  };
}
