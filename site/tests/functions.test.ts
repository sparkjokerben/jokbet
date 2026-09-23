// The two Pages Functions: the manifest's R2-then-baked fallback, and the
// download proxy's R2-then-GitHub fallback. Node has Request/Response/Headers,
// so the functions run here with a stub bucket — no Workers runtime needed.

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
      if (!match) return { ...meta(bytes), body: bytes };
      const offset = Number(match[1]);
      const length = match[2] ? Number(match[2]) - offset + 1 : bytes.length - offset;
      return { ...meta(bytes), range: { offset, length }, body: bytes.slice(offset, offset + length) };
    },
    async head(name: string) {
      if (fail) throw new Error("R2 is down");
      const bytes = objects[name];
      return bytes ? meta(bytes) : null;
    },
  };
}

/** The site's own static files, standing in for the ASSETS fetcher. */
const assets = (body: unknown, ok = true) => ({
  fetch: async () => new Response(JSON.stringify(body), { status: ok ? 200 : 404 }),
});

interface TestContext {
  request: Request;
  env: { DOWNLOADS: unknown; ASSETS: unknown };
  params: { path?: string | string[] };
}
type Handler = (context: TestContext) => Promise<Response>;

let dl: Handler;
let latest: Handler;

beforeEach(async () => {
  vi.resetModules();
  dl = (await import("../functions/dl/[[path]].ts")).onRequest as unknown as Handler;
  latest = (await import("../functions/latest.ts")).onRequest as unknown as Handler;
});

const call = (handler: Handler, request: Request, env: TestContext["env"], path: string[] = []) =>
  handler({ request, env, params: { path } } as TestContext);

const installer = "Jokbet_0.1.0_x64.dmg";
const at = (name: string, init?: RequestInit) => new Request(`https://jokbet.jokerben.top/dl/${name}`, init);

describe("the download proxy", () => {
  it("streams an installer from R2", async () => {
    const bytes = new Uint8Array(4096).fill(7);
    const response = await call(dl, at(installer), { DOWNLOADS: bucket({ [installer]: bytes }), ASSETS: assets({}) }, [
      installer,
    ]);

    expect(response.status).toBe(200);
    expect(response.headers.get("content-type")).toBe("application/x-apple-diskimage");
    expect(response.headers.get("content-length")).toBe("4096");
    expect(response.headers.get("cache-control")).toContain("immutable");
    expect(response.headers.get("x-download-source")).toBe("r2");
  });

  it("passes a range request through as 206", async () => {
    const bytes = new Uint8Array(4096).fill(7);
    const response = await call(
      dl,
      at(installer, { headers: { range: "bytes=0-99" } }),
      { DOWNLOADS: bucket({ [installer]: bytes }), ASSETS: assets({}) },
      [installer],
    );

    expect(response.status).toBe(206);
    expect(response.headers.get("content-range")).toBe("bytes 0-99/4096");
    expect(response.headers.get("content-length")).toBe("100");
  });

  it("answers a probe without reading the file", async () => {
    const name = "Jokbet_0.1.0_amd64.deb";
    const response = await call(
      dl,
      at(name, { method: "HEAD" }),
      { DOWNLOADS: bucket({ [name]: new Uint8Array(10) }), ASSETS: assets({}) },
      [name],
    );

    expect(response.status).toBe(200);
    expect(await response.text()).toBe("");
    expect(response.headers.get("content-type")).toBe("application/vnd.debian.binary-package");
  });

  it("sends a missing installer to GitHub, at the version the page named", async () => {
    const response = await call(
      dl,
      at(`${installer}?v=0.1.0`),
      { DOWNLOADS: bucket(), ASSETS: assets({}) },
      [installer],
    );

    expect(response.status).toBe(302);
    expect(response.headers.get("location")).toBe(
      `https://github.com/${REPO}/releases/download/v0.1.0/${installer}`,
    );
    expect(response.headers.get("x-download-source")).toBe("github");
  });

  it("reads the version from the manifest when the link does not say", async () => {
    const manifest = new TextEncoder().encode(JSON.stringify({ version: "0.2.0" }));
    const response = await call(dl, at(installer), { DOWNLOADS: bucket({ "latest.json": manifest }), ASSETS: assets({}) }, [
      installer,
    ]);

    expect(response.headers.get("location")).toBe(`https://github.com/${REPO}/releases/download/v0.2.0/${installer}`);
  });

  it("falls back to GitHub when R2 cannot be reached at all", async () => {
    const response = await call(
      dl,
      at(`${installer}?v=0.1.0`),
      { DOWNLOADS: bucket({}, true), ASSETS: assets({}) },
      [installer],
    );

    expect(response.status).toBe(302);
    expect(response.headers.get("location")).toContain("releases/download/v0.1.0/");
  });

  it("sends everything it cannot place to the releases page", async () => {
    const response = await call(dl, at(installer), { DOWNLOADS: bucket({}, true), ASSETS: assets({}) }, [installer]);

    expect(response.headers.get("location")).toBe(`https://github.com/${REPO}/releases`);
  });

  it("refuses names that are not ours", async () => {
    for (const name of ["evil.dmg", "Jokbet_/../x.dmg", ""]) {
      const response = await call(dl, at(name), { DOWNLOADS: bucket(), ASSETS: assets({}) }, [name]);
      expect(response.status, name).toBe(404);
    }
  });

  it("refuses a nested path", async () => {
    const response = await call(dl, at("a/b"), { DOWNLOADS: bucket(), ASSETS: assets({}) }, ["a", "b"]);
    expect(response.status).toBe(404);
  });
});

describe("the manifest", () => {
  it("comes from R2 when the bucket has it", async () => {
    const manifest = new TextEncoder().encode(JSON.stringify({ version: "0.1.0", files: {} }));
    const response = await call(latest, new Request("https://jokbet.jokerben.top/latest"), {
      DOWNLOADS: bucket({ "latest.json": manifest }),
      ASSETS: assets({}, false),
    });

    expect(await response.json()).toMatchObject({ version: "0.1.0", source: "r2" });
  });

  it("falls back to the baked copy", async () => {
    const response = await call(latest, new Request("https://jokbet.jokerben.top/latest"), {
      DOWNLOADS: bucket({}, true),
      ASSETS: assets({ version: "0.0.1", files: {} }),
    });

    expect(await response.json()).toMatchObject({ version: "0.0.1", source: "baked" });
  });

  it("says so when neither is available", async () => {
    const response = await call(latest, new Request("https://jokbet.jokerben.top/latest"), {
      DOWNLOADS: bucket({}, true),
      ASSETS: assets({}, false),
    });

    expect(await response.json()).toMatchObject({ version: null, source: "unavailable" });
  });
});
