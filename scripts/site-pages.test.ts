// The page generator: it refills the marked regions and nothing else, keeps
// the page's indentation, and marks the page's own link in the nav.

import { describe, expect, it } from "vitest";
import { carried, fill, pathOf } from "./site-pages.ts";

const partials = {
  header: '<nav>\n  <a href="/">Home</a>\n  <a href="/download">Download</a>\n</nav>\n',
  footer: "<footer>new</footer>\n",
};

const page = [
  "<body>",
  "    <!-- partial:header -->",
  "    <p>old header</p>",
  "    <!-- /partial:header -->",
  "    <main>mine</main>",
  "    <!-- partial:footer -->",
  "    <!-- /partial:footer -->",
  "</body>",
].join("\n");

describe("the page generator", () => {
  it("refills each region from its partial, at the marker's indentation", () => {
    const out = fill(page, partials, null);
    expect(out).toContain('    <!-- partial:header -->\n    <nav>\n      <a href="/">Home</a>');
    expect(out).toContain("    <!-- partial:footer -->\n    <footer>new</footer>\n    <!-- /partial:footer -->");
    expect(out).toContain("<main>mine</main>");
    expect(out).not.toContain("old header");
  });

  it("marks the page's own link in the nav, and only that one", () => {
    const out = fill(page, partials, "/download");
    expect(out).toContain('<a href="/download" aria-current="page">Download</a>');
    expect(out).toContain('<a href="/">Home</a>');
  });

  it("is already done when run twice", () => {
    const once = fill(page, partials, "/");
    expect(fill(once, partials, "/")).toBe(once);
  });

  it("names the pages by their clean addresses", () => {
    expect(pathOf("index.html")).toBe("/");
    expect(pathOf("download.html")).toBe("/download");
    expect(pathOf("404.html")).toBe(null);
  });

  it("knows which partials a page carries", () => {
    expect(carried(page)).toEqual(["header", "footer"]);
  });

  it("refuses a region with no partial behind it", () => {
    expect(() => fill("<!-- partial:nope -->\n<!-- /partial:nope -->", partials, null)).toThrow(/no partial named nope/);
  });
});
