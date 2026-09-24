// The parts every page shares — the common <head>, the top bar, the pet's
// column and the footer — are written once, in site/partials/, and copied into
// each page between markers:
//
//   <!-- partial:header -->
//   …
//   <!-- /partial:header -->
//
//   node scripts/site-pages.ts gen     copy the partials into every page
//   node scripts/site-pages.ts check   fail if a page no longer matches (CI)
//
// So the pages stay plain files that are deployed as they are, and a change to
// the nav or the footer is one edit and one `npm run site:pages`. The only
// thing that differs between the copies is the nav's current page: the link to
// the page being written gets aria-current="page".

import { readdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const SITE = "site";
const PARTIALS = join(SITE, "partials");

/** A page's own address, from its file name; the 404 page has none. */
export function pathOf(file: string): string | null {
  if (file === "index.html") return "/";
  if (file === "404.html") return null;
  return `/${file.replace(/\.html$/, "")}`;
}

/** The page with every marked region refilled from its partial, indented to
 * the marker, and the page's own link in the nav marked as the current one. */
export function fill(page: string, partials: Record<string, string>, path: string | null): string {
  return page.replace(
    /^([ \t]*)<!-- partial:([a-z]+) -->\n[\s\S]*?^\1<!-- \/partial:\2 -->$/gm,
    (_, indent: string, name: string) => {
      let body = partials[name];
      if (body === undefined) throw new Error(`there is no partial named ${name}`);
      if (path) body = body.replace(/<nav[\s\S]*?<\/nav>/, (nav) => nav.replace(`<a href="${path}">`, `<a href="${path}" aria-current="page">`));
      const lines = body.trimEnd().split("\n").map((line) => (line ? indent + line : line));
      return `${indent}<!-- partial:${name} -->\n${lines.join("\n")}\n${indent}<!-- /partial:${name} -->`;
    },
  );
}

/** The partials a page carries, by name. */
export const carried = (page: string) => [...page.matchAll(/<!-- partial:([a-z]+) -->/g)].map((m) => m[1]);

function main(mode: string) {
  const partials = Object.fromEntries(
    readdirSync(PARTIALS)
      .filter((f) => f.endsWith(".html"))
      .map((f) => [f.replace(/\.html$/, ""), readFileSync(join(PARTIALS, f), "utf8")]),
  );
  const problems: string[] = [];
  for (const file of readdirSync(SITE).filter((f) => f.endsWith(".html"))) {
    const before = readFileSync(join(SITE, file), "utf8");
    // Every page carries every partial: a page without the footer is a page
    // without the disclaimer.
    const missing = Object.keys(partials).filter((name) => !carried(before).includes(name));
    if (missing.length) problems.push(`${file} has no place for: ${missing.join(", ")}`);
    const after = fill(before, partials, pathOf(file));
    if (after === before) continue;
    if (mode === "gen") {
      writeFileSync(join(SITE, file), after);
      console.log(`${file}: refilled`);
    } else {
      problems.push(`${file} does not match site/partials/ (run npm run site:pages)`);
    }
  }
  if (problems.length) {
    console.error(problems.join("\n"));
    process.exit(1);
  }
  if (mode === "check") console.log("every page matches site/partials/");
}

if (import.meta.filename === process.argv[1]) {
  const mode = process.argv[2];
  if (mode !== "gen" && mode !== "check") {
    console.error("usage: node scripts/site-pages.ts gen|check");
    process.exit(1);
  }
  main(mode);
}
