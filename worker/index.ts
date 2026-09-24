// The site's server side: two namespaces, and the static files for the rest.
//
//   /api/<name>.json    the manifests (api.ts)
//   /dl/<tag>/<name>    the release files (download.ts)
//   anything else       site/, served by the assets layer
//
// wrangler.toml's `run_worker_first` names /api/* and /dl/*, and it has to: the
// assets layer otherwise answers a request that matches no file by itself, and
// for a *navigation* it answers with the 404 page without ever calling this
// Worker. A browser makes a download a navigation, so without that list every
// download from the page 404s, while curl — which is not a navigation — goes
// straight through and says all is well.

/// <reference types="@cloudflare/workers-types" />

import { api, type Env } from "./api.ts";
import { download } from "./download.ts";

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const { pathname } = new URL(request.url);
    if (pathname.startsWith("/api/")) return api(request, env, pathname.slice("/api/".length));
    if (pathname.startsWith("/dl/")) return download(request, env);
    return env.ASSETS.fetch(request);
  },
} satisfies ExportedHandler<Env>;
