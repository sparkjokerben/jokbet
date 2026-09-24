// The Worker's two namespaces: /api/ with each manifest's own way out when the
// bucket has nothing, and /dl/ with its R2-then-GitHub fallback. Node has
// Request/Response/Headers, so the Worker runs here with a stub bucket — no
// Workers runtime needed.

import { beforeEach, describe, expect, it, vi } from "vitest";
import latestFallback from "./fallback/latest.json";
import releasesFallback from "./fallback/releases.json";

const GITHUB = "https://github.com/sparkjokerben/jokbet/releases";

/** A bucket holding the given objects under their full keys; anything else is
 * missing, and `fail` makes every call throw the way an unreachable R2 does. */
function bucket(objects: Record<string, Uint8Array | string> = {}, fail = false) {
  const bytesOf = (value: Uint8Array | string) => (typeof value === "string" ? new TextEncoder().encode(value) : value);
  const meta = (bytes: Uint8Array) => ({
    size: bytes.length,
    httpEtag: '"etag"',
    writeHttpMetadata(headers: Headers) {
      headers.set("content-type", "application/octet-stream");
    },
    // What an R2ObjectBody can do with its contents.
    text: async () => new TextDecoder().decode(bytes),
    json: async () => JSON.parse(new TextDecoder().decode(bytes)),
  });
  const asked: string[] = [];
  return {
    asked,
    async get(key: string, options?: { range?: Headers }) {
      asked.push(key);
      if (fail) throw new Error("R2 is down");
      const value = objects[key];
      if (value === undefined) return null;
      const bytes = bytesOf(value);
      const match = options?.range?.get("range")?.match(/^bytes=(\d+)-(\d*)$/);
      const offset = match ? Number(match[1]) : 0;
      const length = match ? (match[2] ? Number(match[2]) - offset + 1 : bytes.length - offset) : bytes.length;
      // R2 describes whatever it hands back as a range, the whole object
      // included — so a stub that only sets `range` for a ranged request would
      // hide the difference between a 200 and a 206.
      return { ...meta(bytes), range: { offset, length }, body: bytes.slice(offset, offset + length) };
    },
    async head(key: string) {
      asked.push(key);
      if (fail) throw new Error("R2 is down");
      const value = objects[key];
      if (value === undefined) return null;
      const bytes = bytesOf(value);
      return { ...meta(bytes), range: { offset: 0, length: bytes.length } };
    },
  };
}

/** The site's static files, standing in for the ASSETS binding. */
const assets = () => {
  const asked: string[] = [];
  return {
    asked,
    fetch: async (input: Request | URL | string) => {
      asked.push(typeof input === "string" ? input : "url" in input ? input.url : String(input));
      return new Response("<!doctype html>", { status: 404 });
    },
  };
};

interface Env {
  DOWNLOADS: unknown;
  ASSETS: unknown;
}

/** The Worker's export, as the platform calls it. */
type Worker = { fetch: (request: Request, env: Env) => Promise<Response> };

let worker: Worker;

beforeEach(async () => {
  // A fresh module per test: the Worker remembers the bucket for a minute.
  vi.resetModules();
  worker = (await import("./index.ts")).default as unknown as Worker;
});

const call = (path: string, env: Env, init?: RequestInit) =>
  worker.fetch(new Request(`https://jokbet.jokerben.top${path}`, init), env);

const installer = "Jokbet_0.1.0_x64.dmg";

describe("/dl/<tag>/<name>", () => {
  it("streams a file from the release's folder in the bucket", async () => {
    const bytes = new Uint8Array(4096).fill(7);
    const env = { DOWNLOADS: bucket({ [`releases/v0.1.0/${installer}`]: bytes }), ASSETS: assets() };
    const response = await call(`/dl/v0.1.0/${installer}`, env);

    expect(response.status).toBe(200);
    expect(response.headers.get("content-type")).toBe("application/x-apple-diskimage");
    expect(response.headers.get("content-length")).toBe("4096");
    expect(response.headers.get("content-disposition")).toBe(`attachment; filename="${installer}"`);
    expect(response.headers.get("cache-control")).toContain("immutable");
    expect(response.headers.get("x-download-source")).toBe("r2");
    // A file download is not a page, and search engines should leave it alone.
    expect(response.headers.get("x-robots-tag")).toBe("noindex");
    expect((await response.arrayBuffer()).byteLength).toBe(4096);
  });

  it("serves the updater's bundles, whose names carry no version, by the tag in the path", async () => {
    const name = "Jokbet_aarch64.app.tar.gz";
    const env = { DOWNLOADS: bucket({ [`releases/v0.2.0/${name}`]: "new", [`releases/v0.1.0/${name}`]: "old" }), ASSETS: assets() };

    expect(await (await call(`/dl/v0.1.0/${name}`, env)).text()).toBe("old");
    expect(await (await call(`/dl/v0.2.0/${name}`, env)).text()).toBe("new");
  });

  it("sends the whole file when nothing asked for a range", async () => {
    // The bucket calls this a range covering everything; answering 206 to a
    // request that carried no Range header makes browsers refuse the download.
    const env = { DOWNLOADS: bucket({ [`releases/v0.1.0/${installer}`]: new Uint8Array(4096) }), ASSETS: assets() };
    const response = await call(`/dl/v0.1.0/${installer}`, env);

    expect(response.status).toBe(200);
    expect(response.headers.get("content-range")).toBe(null);
  });

  it("passes a range request through as 206", async () => {
    const env = { DOWNLOADS: bucket({ [`releases/v0.1.0/${installer}`]: new Uint8Array(4096) }), ASSETS: assets() };
    const response = await call(`/dl/v0.1.0/${installer}`, env, { headers: { range: "bytes=0-99" } });

    expect(response.status).toBe(206);
    expect(response.headers.get("content-range")).toBe("bytes 0-99/4096");
    expect(response.headers.get("content-length")).toBe("100");
  });

  it("answers a probe without reading the file", async () => {
    const name = "Jokbet_0.1.0_amd64.deb";
    const env = { DOWNLOADS: bucket({ [`releases/v0.1.0/${name}`]: new Uint8Array(10) }), ASSETS: assets() };
    const response = await call(`/dl/v0.1.0/${name}`, env, { method: "HEAD" });

    expect(response.status).toBe(200);
    expect(await response.text()).toBe("");
    expect(response.headers.get("content-length")).toBe("10");
    expect(response.headers.get("content-type")).toBe("application/vnd.debian.binary-package");
  });

  it("sends a file the bucket does not have to the same path on GitHub", async () => {
    const response = await call(`/dl/v0.1.0/${installer}`, { DOWNLOADS: bucket(), ASSETS: assets() });

    expect(response.status).toBe(302);
    expect(response.headers.get("location")).toBe(`${GITHUB}/download/v0.1.0/${installer}`);
    expect(response.headers.get("x-download-source")).toBe("github");
  });

  it("goes to GitHub when R2 is down", async () => {
    const response = await call(`/dl/v0.1.0/${installer}`, { DOWNLOADS: bucket({}, true), ASSETS: assets() });

    expect(response.status).toBe(302);
    expect(response.headers.get("location")).toBe(`${GITHUB}/download/v0.1.0/${installer}`);
  });

  it("refuses names and tags that are not a release's", async () => {
    for (const path of [
      "/dl/v0.1.0/evil.dmg",
      "/dl/v0.1.0/latest.json",
      "/dl/0.1.0/Jokbet_0.1.0_x64.dmg",
      "/dl/main/Jokbet_0.1.0_x64.dmg",
      "/dl/v0.1.0/a/Jokbet_0.1.0_x64.dmg",
      "/dl/v0.1.0/",
    ]) {
      const response = await call(path, { DOWNLOADS: bucket(), ASSETS: assets() });
      expect(response.status, path).toBe(404);
    }
  });

  it("never asks the bucket for anything outside releases/", async () => {
    const store = bucket();
    await call(`/dl/v0.1.0/${installer}`, { DOWNLOADS: store, ASSETS: assets() });
    expect(store.asked).toEqual([`releases/v0.1.0/${installer}`]);
  });
});

describe("/dl/<name>, the 0.1.0 page's links", () => {
  it("moves to the tagged path, by the version the link carried", async () => {
    const response = await call(`/dl/${installer}?v=0.1.0`, { DOWNLOADS: bucket(), ASSETS: assets() });

    expect(response.status).toBe(301);
    expect(response.headers.get("location")).toBe(`https://jokbet.jokerben.top/dl/v0.1.0/${installer}`);
  });

  it("reads the version from the name when the link carried none", async () => {
    const response = await call(`/dl/${installer}`, { DOWNLOADS: bucket(), ASSETS: assets() });

    expect(response.headers.get("location")).toBe(`https://jokbet.jokerben.top/dl/v0.1.0/${installer}`);
  });

  it("sends a name with no version at all to the releases page", async () => {
    const response = await call("/dl/Jokbet_aarch64.app.tar.gz", { DOWNLOADS: bucket(), ASSETS: assets() });

    expect(response.status).toBe(302);
    expect(response.headers.get("location")).toBe(GITHUB);
  });

  it("still refuses what is not a release file", async () => {
    const response = await call("/dl/evil.dmg?v=0.1.0", { DOWNLOADS: bucket(), ASSETS: assets() });
    expect(response.status).toBe(404);
  });
});

describe("/api/latest.json and /api/releases.json", () => {
  const list = { schema: 1, generatedAt: "2026-09-24T00:00:00Z", releases: [{ version: "0.1.0", tag: "v0.1.0" }] };

  it("come from the bucket's manifests/ when it has them", async () => {
    const env = {
      DOWNLOADS: bucket({
        "manifests/latest.json": JSON.stringify({ version: "0.2.0", files: {} }),
        "manifests/releases.json": JSON.stringify(list),
      }),
      ASSETS: assets(),
    };

    expect(await (await call("/api/latest.json", env)).json()).toMatchObject({ version: "0.2.0", source: "r2" });
    expect(await (await call("/api/releases.json", env)).json()).toMatchObject({ source: "r2", releases: [{ tag: "v0.1.0" }] });
  });

  it("fall back to the copies compiled into the Worker", async () => {
    const env = { DOWNLOADS: bucket({}, true), ASSETS: assets() };

    expect(await (await call("/api/latest.json", env)).json()).toEqual({ ...latestFallback, source: "fallback" });
    expect(await (await call("/api/releases.json", env)).json()).toEqual({ ...releasesFallback, source: "fallback" });
  });

  it("treat a corrupt object as a missing one", async () => {
    const env = { DOWNLOADS: bucket({ "manifests/latest.json": "{ not json" }), ASSETS: assets() };

    expect(await (await call("/api/latest.json", env)).json()).toMatchObject({ source: "fallback" });
  });

  it("ask the bucket once a minute, not once a request", async () => {
    const store = bucket({ "manifests/latest.json": JSON.stringify({ version: "0.2.0", files: {} }) });
    const env = { DOWNLOADS: store, ASSETS: assets() };
    await call("/api/latest.json", env);
    await call("/api/latest.json", env);

    expect(store.asked).toEqual(["manifests/latest.json"]);
  });

  it("are JSON, and refuse anything but GET and HEAD", async () => {
    const env = { DOWNLOADS: bucket(), ASSETS: assets() };
    expect((await call("/api/latest.json", env)).headers.get("content-type")).toContain("application/json");

    const post = await call("/api/latest.json", env, { method: "POST" });
    expect(post.status).toBe(405);
    expect(post.headers.get("allow")).toBe("GET, HEAD");
  });
});

describe("/api/update.json", () => {
  const update = {
    version: "0.2.0",
    platforms: { "darwin-aarch64": { url: "https://jokbet.jokerben.top/dl/v0.2.0/Jokbet_aarch64.app.tar.gz", signature: "s" } },
  };

  it("passes the bucket's copy through untouched", async () => {
    const body = JSON.stringify(update);
    const response = await call("/api/update.json", { DOWNLOADS: bucket({ "manifests/update.json": body }), ASSETS: assets() });

    expect(response.status).toBe(200);
    expect(await response.text()).toBe(body);
  });

  it("sends the updater to GitHub's own manifest when the bucket has none", async () => {
    for (const store of [bucket(), bucket({}, true), bucket({ "manifests/update.json": "<html>" })]) {
      vi.resetModules();
      worker = (await import("./index.ts")).default as unknown as Worker;
      const response = await call("/api/update.json", { DOWNLOADS: store, ASSETS: assets() });

      expect(response.status).toBe(302);
      expect(response.headers.get("location")).toBe(`${GITHUB}/latest/download/latest.json`);
    }
  });
});

describe("everything else", () => {
  it("is the assets layer's, as it was asked for", async () => {
    const files = assets();
    for (const path of ["/", "/download", "/changelog", "/styles.css"]) {
      await call(path, { DOWNLOADS: bucket({}, true), ASSETS: files });
    }
    // Passed through untouched: in particular /changelog is never rewritten to
    // changelog.html, which _redirects would send straight back — a loop.
    expect(files.asked).toEqual(
      ["/", "/download", "/changelog", "/styles.css"].map((path) => `https://jokbet.jokerben.top${path}`),
    );
  });

  it("answers an unknown /api/ name with a 404", async () => {
    const response = await call("/api/nothing.json", { DOWNLOADS: bucket(), ASSETS: assets() });
    expect(response.status).toBe(404);
  });
});
