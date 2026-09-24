// The Worker's three routes: the two JSON endpoints with their R2-then-baked
// fallback, and the download proxy's R2-then-GitHub fallback. Node has
// Request/Response/Headers, so the Worker runs here with a stub bucket — no
// Workers runtime needed.

import { beforeEach, describe, expect, it, vi } from "vitest";

const REPO = "sparkjokerben/jokbet";

/** A bucket holding the given objects; anything else is missing. */
function bucket(objects: Record<string, Uint8Array> = {}, fail = false) {
  const meta = (bytes: Uint8Array) => ({
    size: bytes.length,
    httpEtag: '"etag"',
    writeHttpMetadata(headers: Headers) {
      headers.set("content-type", "application/octet-stream");
    },
    // What an R2ObjectBody can do with its contents.
    json: async () => JSON.parse(new TextDecoder().decode(bytes)),
    text: async () => new TextDecoder().decode(bytes),
    arrayBuffer: async () => bytes.slice().buffer,
  });
  return {
    async get(name: string, options?: { range?: Headers }) {
      if (fail) throw new Error("R2 is down");
      const bytes = objects[name];
      if (!bytes) return null;
      const match = options?.range?.get("range")?.match(/^bytes=(\d+)-(\d*)$/);
      const offset = match ? Number(match[1]) : 0;
      const length = match ? (match[2] ? Number(match[2]) - offset + 1 : bytes.length - offset) : bytes.length;
      // R2 describes whatever it hands back as a range, the whole object
      // included — so a stub that only sets `range` for a ranged request would
      // hide the difference between a 200 and a 206.
      return { ...meta(bytes), range: { offset, length }, body: bytes.slice(offset, offset + length) };
    },
    async head(name: string) {
      if (fail) throw new Error("R2 is down");
      const bytes = objects[name];
      return bytes ? { ...meta(bytes), range: { offset: 0, length: bytes.length } } : null;
    },
  };
}

/** The site's own static files, standing in for the ASSETS binding. */
const assets = (body: unknown, ok = true) => ({
  fetch: async () => new Response(JSON.stringify(body), { status: ok ? 200 : 404 }),
});

interface Env {
  DOWNLOADS: unknown;
  ASSETS: unknown;
}

/** The Worker's export, as the two routes call it. */
type Worker = { fetch: (request: Request, env: Env) => Promise<Response> };

let worker: Worker;

beforeEach(async () => {
  vi.resetModules();
  worker = (await import("../worker/index.ts")).default as unknown as Worker;
});

const call = (request: Request, env: Env) => worker.fetch(request, env);
const page = (path: string, init?: RequestInit) => new Request(`https://jokbet.jokerben.top${path}`, init);

const installer = "Jokbet_0.1.0_x64.dmg";

describe("the download proxy", () => {
  it("streams an installer from R2", async () => {
    const bytes = new Uint8Array(4096).fill(7);
    const response = await call(page(`/dl/${installer}`), { DOWNLOADS: bucket({ [installer]: bytes }), ASSETS: assets({}) });

    expect(response.status).toBe(200);
    expect(response.headers.get("content-type")).toBe("application/x-apple-diskimage");
    expect(response.headers.get("content-length")).toBe("4096");
    expect(response.headers.get("cache-control")).toContain("immutable");
    expect(response.headers.get("x-download-source")).toBe("r2");
    // A file download is not a page, and search engines should leave it alone.
    expect(response.headers.get("x-robots-tag")).toBe("noindex");
  });

  it("sends the whole file when nothing asked for a range", async () => {
    // The bucket calls this a range covering everything; answering 206 to a
    // request that carried no Range header makes browsers refuse the download.
    const bytes = new Uint8Array(4096).fill(7);
    const response = await call(page(`/dl/${installer}`), { DOWNLOADS: bucket({ [installer]: bytes }), ASSETS: assets({}) });

    expect(response.status).toBe(200);
    expect(response.headers.get("content-range")).toBe(null);
    expect((await response.arrayBuffer()).byteLength).toBe(4096);
  });

  it("passes a range request through as 206", async () => {
    const bytes = new Uint8Array(4096).fill(7);
    const response = await call(page(`/dl/${installer}`, { headers: { range: "bytes=0-99" } }), {
      DOWNLOADS: bucket({ [installer]: bytes }),
      ASSETS: assets({}),
    });

    expect(response.status).toBe(206);
    expect(response.headers.get("content-range")).toBe("bytes 0-99/4096");
    expect(response.headers.get("content-length")).toBe("100");
  });

  it("answers a probe without reading the file", async () => {
    const name = "Jokbet_0.1.0_amd64.deb";
    const response = await call(page(`/dl/${name}`, { method: "HEAD" }), {
      DOWNLOADS: bucket({ [name]: new Uint8Array(10) }),
      ASSETS: assets({}),
    });

    expect(response.status).toBe(200);
    expect(await response.text()).toBe("");
    expect(response.headers.get("content-type")).toBe("application/vnd.debian.binary-package");
  });

  it("sends a missing installer to GitHub, at the version the page named", async () => {
    const response = await call(page(`/dl/${installer}?v=0.1.0`), { DOWNLOADS: bucket(), ASSETS: assets({}) });

    expect(response.status).toBe(302);
    expect(response.headers.get("location")).toBe(`https://github.com/${REPO}/releases/download/v0.1.0/${installer}`);
    expect(response.headers.get("x-download-source")).toBe("github");
  });

  it("reads the version from the manifest when the link does not say", async () => {
    const manifest = new TextEncoder().encode(JSON.stringify({ version: "0.2.0" }));
    const response = await call(page(`/dl/${installer}`), {
      DOWNLOADS: bucket({ "latest.json": manifest }),
      ASSETS: assets({}),
    });

    expect(response.headers.get("location")).toBe(`https://github.com/${REPO}/releases/download/v0.2.0/${installer}`);
  });

  it("falls back to GitHub when R2 cannot be reached at all", async () => {
    const response = await call(page(`/dl/${installer}?v=0.1.0`), { DOWNLOADS: bucket({}, true), ASSETS: assets({}) });

    expect(response.status).toBe(302);
    expect(response.headers.get("location")).toContain("releases/download/v0.1.0/");
  });

  it("sends everything it cannot place to the releases page", async () => {
    const response = await call(page(`/dl/${installer}`), { DOWNLOADS: bucket({}, true), ASSETS: assets({}) });

    expect(response.headers.get("location")).toBe(`https://github.com/${REPO}/releases`);
  });

  it("refuses names that are not ours", async () => {
    for (const name of ["evil.dmg", "Jokbet_/../x.dmg", ""]) {
      const response = await call(page(`/dl/${name}`), { DOWNLOADS: bucket(), ASSETS: assets({}) });
      expect(response.status, name).toBe(404);
    }
  });

  it("refuses a nested path", async () => {
    const response = await call(page("/dl/a/b"), { DOWNLOADS: bucket(), ASSETS: assets({}) });
    expect(response.status).toBe(404);
  });
});

describe("the manifest", () => {
  it("comes from R2 when the bucket has it", async () => {
    const manifest = new TextEncoder().encode(JSON.stringify({ version: "0.1.0", files: {} }));
    const response = await call(page("/latest"), { DOWNLOADS: bucket({ "latest.json": manifest }), ASSETS: assets({}, false) });

    expect(await response.json()).toMatchObject({ version: "0.1.0", source: "r2" });
  });

  it("falls back to the baked copy", async () => {
    const response = await call(page("/latest"), {
      DOWNLOADS: bucket({}, true),
      ASSETS: assets({ version: "0.0.1", files: {} }),
    });

    expect(await response.json()).toMatchObject({ version: "0.0.1", source: "baked" });
  });

  it("says so when neither is available", async () => {
    const response = await call(page("/latest"), { DOWNLOADS: bucket({}, true), ASSETS: assets({}, false) });

    expect(await response.json()).toMatchObject({ version: null, source: "unavailable" });
  });
});

describe("the changelog", () => {
  const list = { schema: 1, generatedAt: "2026-09-24T00:00:00Z", releases: [{ version: "0.1.0", tag: "v0.1.0" }] };

  it("comes from R2 when the bucket has it", async () => {
    const response = await call(page("/changelog.json"), {
      DOWNLOADS: bucket({ "changelog.json": new TextEncoder().encode(JSON.stringify(list)) }),
      ASSETS: assets({}, false),
    });

    expect(await response.json()).toMatchObject({ source: "r2", releases: [{ tag: "v0.1.0" }] });
  });

  it("falls back to the baked copy", async () => {
    const response = await call(page("/changelog.json"), { DOWNLOADS: bucket({}, true), ASSETS: assets(list) });

    expect(await response.json()).toMatchObject({ source: "baked", releases: [{ tag: "v0.1.0" }] });
  });

  it("answers with an empty list rather than inventing a version", async () => {
    const response = await call(page("/changelog.json"), { DOWNLOADS: bucket({}, true), ASSETS: assets({}, false) });

    expect(await response.json()).toMatchObject({ releases: [], source: "unavailable" });
  });

  it("is a JSON endpoint, not a page", async () => {
    const response = await call(page("/changelog.json"), { DOWNLOADS: bucket({}), ASSETS: assets({}, false) });

    expect(response.headers.get("content-type")).toContain("application/json");
  });
});

describe("the rest of the site", () => {
  it("hands everything else to the assets layer", async () => {
    const request = page("/styles.css");
    const env = { DOWNLOADS: bucket({}, true), ASSETS: assets({}, false) };
    const response = await call(request, env);

    // The stub answers 404 for everything, which is enough to prove the route:
    // an unmatched path is not this Worker's business.
    expect(response.status).toBe(404);
  });

  it("serves /changelog from changelog.html, its clean URL", async () => {
    let asked = "";
    const env = {
      DOWNLOADS: bucket({}, true),
      ASSETS: {
        fetch: async (input: Request | URL) => {
          asked = typeof input === "string" ? input : "url" in input ? input.url : String(input);
          return new Response("<html></html>", { status: 200 });
        },
      },
    };
    const response = await call(page("/changelog"), env);

    expect(response.status).toBe(200);
    expect(asked).toContain("/changelog.html");
  });

  it("refuses anything but GET and HEAD on the JSON endpoints", async () => {
    const response = await call(page("/latest", { method: "POST" }), { DOWNLOADS: bucket({}), ASSETS: assets({}) });

    expect(response.status).toBe(405);
    expect(response.headers.get("allow")).toBe("GET, HEAD");
  });
});
