// The site's server side.
//
// The files in site/ are served by the assets layer; this Worker answers the
// paths that need the bucket — /latest, /changelog.json and /dl/<name> — and
// defers the rest back to the assets layer, so an unknown path still gets the
// platform's own 404.
//
// Those paths are named in wrangler.toml's `run_worker_first`, and they have to
// be: the assets layer otherwise answers a request that matches no file on its
// own, and for a *navigation* it answers with the 404 page without ever calling
// this Worker. A browser makes a download a navigation, so every real download
// used to 404 while curl, which is not a navigation, went straight through.

/// <reference types="@cloudflare/workers-types" />

import { download } from "./download.ts";
import { mirror, type Env } from "./mirror.ts";

/** What /latest answers with when there is no bucket and no baked copy. */
const NO_MANIFEST = { schema: 1, version: null, files: {}, source: "unavailable" };
/** The same for /changelog.json. */
const NO_CHANGELOG = { schema: 1, generatedAt: null, releases: [], source: "unavailable" };

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const { pathname } = new URL(request.url);
    if (pathname === "/latest") {
      return mirror(request, env, "latest.json", "/latest.baked.json", NO_MANIFEST);
    }
    if (pathname === "/changelog.json") {
      return mirror(request, env, "changelog.json", "/releases.baked.json", NO_CHANGELOG);
    }
    if (pathname.startsWith("/dl/")) return download(request, env);
    // `/changelog` is the changelog page's clean URL; the assets layer's own
    // rewrite rules only speak about trailing slashes, so do it here.
    if (pathname === "/changelog") {
      return env.ASSETS.fetch(new Request(new URL("/changelog.html", request.url), request));
    }
    return env.ASSETS.fetch(request);
  },
} satisfies ExportedHandler<Env>;
