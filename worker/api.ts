// GET /api/<name>.json — the three manifests the site and the app read.
//
// The release job writes each one into the bucket under manifests/. The bucket
// is asked at most once a minute per isolate, and when it has nothing to give
// (the object is missing, or R2 cannot be reached) each manifest has its own
// way out:
//
//   /api/latest.json    the newest release, for the download buttons
//                       → the copy compiled into this Worker (worker/fallback/)
//   /api/releases.json  every published release, for the changelog
//                       → the copy compiled into this Worker
//   /api/update.json    Tauri's updater manifest, its URLs pointed at /dl/
//                       → GitHub's own copy of it, by redirect
//
// The two the page reads carry `source` ("r2" or "fallback"), so the page can
// say where its numbers came from. The updater's is passed through untouched.

/// <reference types="@cloudflare/workers-types" />

import latestFallback from "./fallback/latest.json";
import releasesFallback from "./fallback/releases.json";

export interface Env {
  /** The R2 bucket: releases/<tag>/<name> and manifests/<name>.json. */
  DOWNLOADS: R2Bucket;
  /** The site's static files. */
  ASSETS: Fetcher;
}

export const REPO = "sparkjokerben/jokbet";
export const GITHUB_RELEASES = `https://github.com/${REPO}/releases`;

/** How long an isolate keeps what the bucket said before asking again. */
const TTL_MS = 60_000;

/** The bucket's last answer per key — null included, so a failing bucket is
 * not asked again on every request either. */
const remembered = new Map<string, { at: number; body: string | null }>();

async function fromBucket(env: Env, key: string): Promise<string | null> {
  const hit = remembered.get(key);
  if (hit && Date.now() - hit.at < TTL_MS) return hit.body;
  let body: string | null = null;
  try {
    const object = await env.DOWNLOADS.get(key);
    if (object) body = await object.text();
  } catch {
    // Bucket missing, binding missing, R2 down: the fallback answers.
  }
  remembered.set(key, { at: Date.now(), body });
  return body;
}

const json = (body: string) =>
  new Response(body, {
    headers: {
      "content-type": "application/json; charset=utf-8",
      "cache-control": "public, max-age=60",
    },
  });

/** The object if it is JSON at all: a corrupt one is as good as a missing one. */
function parsed(body: string | null): Record<string, unknown> | null {
  if (body === null) return null;
  try {
    const value = JSON.parse(body) as unknown;
    return value && typeof value === "object" && !Array.isArray(value) ? (value as Record<string, unknown>) : null;
  } catch {
    return null;
  }
}

/** A manifest the page reads, from the bucket or the compiled-in copy. */
async function labelled(env: Env, key: string, fallback: object): Promise<Response> {
  const fromR2 = parsed(await fromBucket(env, key));
  return json(JSON.stringify(fromR2 ? { ...fromR2, source: "r2" } : { ...fallback, source: "fallback" }));
}

export async function api(request: Request, env: Env, name: string): Promise<Response> {
  if (request.method !== "GET" && request.method !== "HEAD") {
    return new Response("Method not allowed", { status: 405, headers: { allow: "GET, HEAD" } });
  }
  switch (name) {
    case "latest.json":
      return labelled(env, "manifests/latest.json", latestFallback);
    case "releases.json":
      return labelled(env, "manifests/releases.json", releasesFallback);
    case "update.json": {
      const body = await fromBucket(env, "manifests/update.json");
      if (parsed(body)) return json(body!);
      // The updater follows redirects, and it has GitHub as its second
      // endpoint anyway; this just means the mirror never answers with nothing.
      return new Response(null, {
        status: 302,
        headers: { location: `${GITHUB_RELEASES}/latest/download/latest.json`, "cache-control": "no-store" },
      });
    }
    default:
      return new Response("Not found", { status: 404 });
  }
}
