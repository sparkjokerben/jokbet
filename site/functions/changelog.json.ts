// GET /changelog.json — every published release, for the changelog page.
//
// Written by the release job from the GitHub releases API (see
// .github/workflows/publish.yml), so the page needs no third-party API and no
// rate limit at view time. The baked fallback in this directory is what it
// answers with when the bucket has nothing.

/// <reference types="@cloudflare/workers-types" />

import { mirror, type Env } from "./mirror.ts";

const EMPTY = { schema: 1, generatedAt: null, releases: [], source: "unavailable" };

export const onRequest: PagesFunction<Env> = mirror("changelog.json", "/releases.baked.json", EMPTY);
