// GET /latest — the download manifest the page reads.
//
// The bucket's copy, then the one baked into the site; the page is told which
// answered (`source: "r2" | "baked"`) and says so where it offers the files.

/// <reference types="@cloudflare/workers-types" />

import { mirror, type Env } from "./mirror.ts";

const EMPTY = { schema: 1, version: null, files: {}, source: "unavailable" };

export const onRequest: PagesFunction<Env> = mirror("latest.json", "/latest.baked.json", EMPTY);
