// GET /latest — the download manifest the page reads.
//
// Same origin as the page, so there is no CORS and one place knows about R2:
// it tries the bucket, then the copy baked into the site, and says which one
// answered so the page can tell the truth about where a download comes from.

/// <reference types="@cloudflare/workers-types" />

interface Env {
  /** The R2 bucket holding the installers and the published latest.json. */
  DOWNLOADS: R2Bucket;
  /** The site's own static files, for the baked fallback. */
  ASSETS: Fetcher;
}

const EMPTY = { schema: 1, version: null, files: {}, source: "unavailable" };
/** How long a worker keeps the manifest to itself before asking R2 again. */
const TTL_MS = 60_000;

let cached: { at: number; body: string } | null = null;

const json = (body: string) =>
  new Response(body, {
    status: 200,
    headers: {
      "content-type": "application/json; charset=utf-8",
      "cache-control": "public, max-age=60",
    },
  });

async function manifest(context: EventContext<Env, string, unknown>): Promise<string> {
  try {
    const object = await context.env.DOWNLOADS.get("latest.json");
    if (object) {
      const body = (await object.json()) as Record<string, unknown>;
      return JSON.stringify({ ...body, source: "r2" });
    }
  } catch {
    // Bucket missing, binding missing, R2 down: the baked copy is next.
  }
  try {
    const response = await context.env.ASSETS.fetch(new URL("/latest.baked.json", context.request.url));
    if (response.ok) {
      const body = (await response.json()) as Record<string, unknown>;
      return JSON.stringify({ ...body, source: "baked" });
    }
  } catch {
    // No baked copy either: the page falls back to the GitHub links it has.
  }
  return JSON.stringify(EMPTY);
}

export const onRequest: PagesFunction<Env> = async (context) => {
  if (context.request.method !== "GET" && context.request.method !== "HEAD") {
    return new Response("Method not allowed", { status: 405, headers: { allow: "GET, HEAD" } });
  }
  if (cached && Date.now() - cached.at < TTL_MS) return json(cached.body);
  cached = { at: Date.now(), body: await manifest(context) };
  return json(cached.body);
};
