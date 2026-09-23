// Bundles the app's pet for the website: src/pet/web.ts (which pulls in the
// PetController, the sprite data and the palette) becomes site/assets/pet.js,
// the module site/app.js imports. The page then runs the app's own pet rather
// than a copy of it.
//
//   node scripts/build-site-pet.ts [gen|check] [outDir]   (default: gen site/assets)
//
// `check` rebuilds in memory and compares with the committed bundle, so a pet
// change that was never re-bundled fails CI instead of shipping a stale pet.

import { mkdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { build } from "vite";

const ENTRY = "src/pet/web.ts";
const FILE = "pet.js";
const BANNER =
  "/*! Jokbet's pet, running on the website. Generated from " +
  ENTRY +
  " by scripts/build-site-pet.ts — do not edit by hand. */";

/** The shape of what vite hands back with `write: false`. */
interface Bundled {
  output: { type: string; code?: string }[];
}

async function bundle(): Promise<string> {
  const root = resolve(import.meta.dirname, "..");
  const result = (await build({
    root,
    configFile: false,
    logLevel: "warn",
    publicDir: false,
    build: {
      write: false,
      minify: true,
      target: "es2022",
      lib: { entry: resolve(root, ENTRY), formats: ["es"], fileName: () => FILE },
      rollupOptions: { output: { banner: BANNER } },
      emptyOutDir: false,
    },
  })) as Bundled | Bundled[];
  const outputs = Array.isArray(result) ? result : [result];
  const chunk = outputs[0]?.output.find((part) => part.type === "chunk");
  if (!chunk?.code) throw new Error("vite produced no bundle");
  return chunk.code.endsWith("\n") ? chunk.code : `${chunk.code}\n`;
}

const [mode = "gen", dir] = process.argv.slice(2);
const out = join(dir ?? join(import.meta.dirname, "..", "site", "assets"), FILE);
const code = await bundle();

if (mode === "check") {
  const want = readFileSync(out, "utf8");
  if (want !== code) {
    console.error(`${out} has drifted from ${ENTRY} — run: npm run site:assets`);
    process.exit(1);
  }
  console.log(`${out} matches ${ENTRY}`);
} else {
  mkdirSync(dir ?? join(import.meta.dirname, "..", "site", "assets"), { recursive: true });
  writeFileSync(out, code);
  console.log(`site pet -> ${out} (${Math.round(statSync(out).size / 1024)} KB)`);
}
