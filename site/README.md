# The website

`jokbet.jokerben.top` — a static site (two pages, a 404 and its assets) served
by a Cloudflare Worker, with the installers in an R2 bucket and the Worker
answering the three paths that need them. There is no build step: what is in
`site/` is what is deployed.

```
wrangler.toml           the Worker: name, entry point, the assets directory, the R2 binding
worker/index.ts         the routes: /latest, /changelog.json, /dl/<name>, /changelog, then the assets
worker/mirror.ts        the bucket-then-baked JSON door, shared by the two JSON routes
worker/download.ts      the installer proxy: R2 first, then GitHub
worker/index.test.ts    vitest coverage of all of it, with a stub bucket
worker/tsconfig.json    type-checks the Worker (@cloudflare/workers-types)

site/
  index.html            the home page: hero, downloads, what it does, install, privacy, limits, licence
  changelog.html        every version, its notes and its installers
  404.html              what an unknown path gets (noindex, with the way home)
  styles.css            the app's palette, the layout, the pet's own looks
  app.js                the language switch, the download list, the changelog, and the page's half of the pet
  lang-boot.js          picks zh/en before the first paint (the app's own rule)
  _headers              security headers (CSP) and cache rules
  _redirects            /changelog.html → /changelog
  .assetsignore         what lives beside the site but is not part of it (this README)
  robots.txt sitemap.xml
  latest.baked.json     the last-known-good manifest, served when R2 cannot be
  releases.baked.json   the same, for the changelog (empty until the first release)
  assets/               generated: the pet bundle, the poster, the wordmark, the fonts, the icons, the OG card
```

## The pet on the page

The creature on the home page is not a GIF or a sprite sheet: it is the app's
own animation code, running in the browser.

- `src/pet/web.ts` is the browser adapter — it builds the same DOM the app's Svelte
  component does (the sprite as SVG paths, at the app's own offsets), hands it a
  `PetController`, and feeds it what the visitor does: keys and clicks on the
  page, the pointer for its eyes, and the chips under the hero for one animation
  at a time. The number the app keeps over its head is an option here, and the
  page leaves it off: the site is not counting anything for you. The pet can be
  dragged anywhere on the page, and stays where it is put.
- `scripts/build-site-pet.ts` bundles that (with the sprite data) into
  `assets/pet.js` with vite — about 9 KB gzipped. `npm run site:assets` runs it,
  and `npm run site:check` fails if the bundle no longer matches the source.
- `assets/jokbet-poster.png` is the still frame the page shows where the pet will
  be, so a browser without JavaScript — or the first paint, before the bundle
  arrives — still has the creature on it. The live sprite replaces it exactly.

So a change to the sprites, the poses or the controller changes the website's pet
as well, and CI notices if the generated files were not regenerated.

## How a download works

The page never talks to R2 or GitHub itself; it links `/dl/<name>?v=<version>`
and says which source answered.

1. `/latest` → the newest release's manifest. It comes from `latest.json` in the
   bucket; if the bucket cannot be reached, from `latest.baked.json` in `site/`;
   the page is told which one it got (`source: "r2" | "baked"`) and says so.
2. `/changelog.json` → every published release (the same fallback order, with
   `releases.baked.json`). The release job writes it from the GitHub releases
   API, so the page needs no third-party API at view time.
3. `/dl/<name>` → the installer is streamed from the bucket (ranges included,
   `cache-control: immutable`, since the names carry the version, and
   `x-robots-tag: noindex`, since an installer is not a page). If the object is
   not there — older versions are never mirrored — or R2 is unreachable, the
   browser is redirected to the same file on GitHub, so no link on the page can
   dead-end.
4. Everything else is the assets layer's: a file if one matches, and `404.html`
   with a 404 status if none does.

The bucket stays private; the Worker is the only door to it, and the names it
will serve are checked against what Tauri produces.

## Working on it

```sh
npm run site:dev       # the whole thing locally: assets, routes, headers and the R2 binding
npm run site:deploy    # deploy it by hand (the Git build does this on every push to main)
npm run site:assets    # regenerate site/assets from src/sprites and src/pet (pixel-exact)
npm run site:check     # fail if assets/ or the pet bundle has drifted, or a manifest is invalid
npm test               # includes worker/index.test.ts, with a stub bucket
npx tsc -p worker/tsconfig.json
```

`npm run site:dev` is `wrangler dev`, which reads `wrangler.toml` — so the routes,
the `_headers` (CSP included) and the bindings are all the real ones, and the
bucket starts empty: `/latest` and `/changelog.json` answer from the baked
copies, and every `/dl/` link redirects to GitHub. That is the fallback path, end
to end; `npm test` covers the R2 path instead, and the deployed site is the real
check.

To look at the pages without the Worker at all, serve `site/` and stub the two
JSON endpoints:

```sh
cp -r site /tmp/site-preview
cp site/latest.baked.json /tmp/site-preview/latest
cp site/releases.baked.json /tmp/site-preview/changelog.json
python3 -m http.server -d /tmp/site-preview 8099     # then open http://127.0.0.1:8099/
```

The poster, the favicons and the Open Graph card are generated from the same
sprite code the app draws, so the site can never drift from the pet. The Latin
display face is [Silkscreen](https://github.com/googlefonts/silkscreen) (SIL Open
Font License, see `site/assets/fonts/OFL.txt`); the Chinese text uses the system
font, which is why the pixel face is scoped to `U+0000-00FF`.

## Search engines

- Both pages carry a title, a description, a canonical URL, `robots` directives
  and an Open Graph set; the home page also carries JSON-LD
  (`WebSite` + `SoftwareApplication` + `SoftwareSourceCode`, the licence and the
  repository included), the changelog a `BreadcrumbList`.
- `sitemap.xml` lists the two canonical URLs and `robots.txt` points at it, with
  `/dl/`, `/latest` and `/changelog.json` excluded — none of them is a page.
- The `<title>` in the markup is bilingual, so it reads the same to a crawler as
  to a visitor who has not picked a language; the tab title follows the language
  only once someone switches it.
- `/changelog.html` redirects to `/changelog`, so the page has one URL.
- The installers are served with `x-robots-tag: noindex`, and `404.html` is
  `noindex` as well.

## What is already set up (Cloudflare)

The account has, in `jokbet.jokerben.top`'s Cloudflare account:

| Thing | Name | Notes |
|---|---|---|
| Worker | `jokbet-site` | the Git-connected build runs `npx wrangler deploy`; `wrangler.toml` is the source of truth for the name and the bindings |
| Custom domain | `jokbet.jokerben.top` | attached to that Worker (Workers → `jokbet-site` → Settings → Domains & Routes) |
| R2 bucket | `jokbet-downloads` | private; the `/dl` route is the only way in |
| GitHub secrets | `R2_ACCOUNT_ID`, `R2_ACCESS_KEY_ID`, `R2_SECRET_ACCESS_KEY` | used by the *Publish to R2* workflow |

The bucket name is written into `wrangler.toml` and the publish workflow, so it is
not a secret; only the three secret values are.

## Releasing

1. Bump `version` in `package.json`, commit, tag `vX.Y.Z`, push the tag — CI
   builds every platform into a **draft** release (see the root README).
2. Check the draft, then **publish** it. That fires the *Publish to R2*
   workflow, which copies the installers, `latest.json` and `changelog.json`
   into the bucket. The page picks them up within a minute (both are cached for
   60 s).
3. Refresh the two fallbacks in the repo and commit them, so the baked copies
   name the same release as the mirror:

   ```sh
   npm run site:manifest   # site/latest.baked.json
   npm run site:releases   # site/releases.baked.json, from the GitHub API (needs gh)
   ```

Before the first release, `latest.baked.json` carries no version at all, and the
page says there is nothing published yet rather than offering files that are not
there.
