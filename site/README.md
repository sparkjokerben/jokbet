# The website

`jokbet.jokerben.top` — four pages of static files, served by a Cloudflare
Worker that also answers two namespaces: the manifests (`/api/`) and the
release files (`/dl/`), both mirrored in an R2 bucket with GitHub behind them.
There is no build step: what is in `site/` is what is deployed.

## Addresses

| Address | What it is | Served from |
|---|---|---|
| `/` | the pet, what it does, the download buttons | `site/index.html` |
| `/download` | every installer of the newest release, checksums, install steps | `site/download.html` |
| `/changelog` | every release: its notes and its installers | `site/changelog.html` |
| `/about` | privacy, known limits, the licence, notices | `site/about.html` |
| `/api/latest.json` | the newest release, for the buttons and `/download` | R2 `manifests/latest.json`, else `worker/fallback/latest.json` |
| `/api/releases.json` | every published release, for `/changelog` | R2 `manifests/releases.json`, else `worker/fallback/releases.json` |
| `/api/update.json` | the app's updater manifest, its downloads pointed at `/dl/` | R2 `manifests/update.json`, else a redirect to GitHub's |
| `/dl/<tag>/<name>` | one file of one release | R2 `releases/<tag>/<name>`, else a redirect to GitHub |
| `/install.sh` | the one-line macOS installer the download page offers | `site/install.sh` |

Every release file has one address per host, the same shape on both:
`/dl/<tag>/<name>` here, `releases/download/<tag>/<name>` on GitHub. The site's
links, the Worker's fallback and the app's updater all move between the two
without looking anything up, and a version in the path means an address never
changes what it serves.

`install.sh` is the way past Gatekeeper. A build a browser downloaded is
quarantined, and macOS then stops an un-notarized one — a disk image when it is
opened, the app inside it once more. `curl` marks nothing, so
`curl -fsSL https://jokbet.jokerben.top/install.sh | sh` installs the newest
release's `aarch64`/`x64` disk image for the Mac it runs on, checks it against
the manifest's SHA-256, puts it in `/Applications` (or `~/Applications` when
that is not writable) and opens it. Running it again updates in place: it asks
the running app to quit the way its own Quit item does, so the counts in hand
are saved first. The script is served as `text/plain`, so a browser shows it
rather than saving it, and the download page links to it for reading first.

Each macOS build also ships as a zip from 0.1.2 on, and that is what the pages
offer first: a zip is stopped once, a disk image twice. The zip comes from the
release job (`release.yml`), which zips the signed `Jokbet.app` tauri-action
just built and uploads it to the draft release, so it is the same binary the
disk image holds — signed with the same identity, which is what keeps the Input
Monitoring grant across updates. `scripts/site-manifest.ts` lists it with
`since: "0.1.2"`, so the releases before it are not expected to have one.

Older addresses still land: `/latest` and `/changelog.json` (the 0.1.0 page's
JSON) redirect to `/api/`, `/dl/<name>?v=<version>` (its download links) to
`/dl/v<version>/<name>`, the `.html` spellings to the clean URLs, and the home
page's old anchors (`/#download`, `/#install`, `/#privacy`, …) to the pages the
sections moved to.

## Files

```
wrangler.toml            the Worker: name, entry point, assets, run_worker_first, the R2 binding
worker/index.ts          the router: /api/*, /dl/*, then the assets
worker/api.ts            the three manifests, each with its own fallback
worker/download.ts       the release files: R2 first, then GitHub
worker/fallback/         the manifests compiled into the Worker, for when R2 cannot answer
worker/index.test.ts     vitest coverage of all of it, with a stub bucket

site/
  index.html download.html changelog.html about.html 404.html
  partials/              what every page shares: head, header, rail (the pet), footer
  styles.css             the app's palette, the layout, the pet's own looks
  app.js                 the language switch, the pet, and what each page builds from /api/
  boot.js                picks zh/en before the first paint; forwards the old anchors
  install.sh             the one-line macOS installer: manifest, checksum, ditto, open
  _headers               security headers (CSP) and cache rules
  _redirects             the .html aliases and the pre-/api/ addresses
  .assetsignore          what lives here but is not published (partials/, this README)
  robots.txt sitemap.xml
  assets/                generated: the pet bundle, the poster, the wordmark, the fonts, the icons, the OG card
```

`run_worker_first` in `wrangler.toml` lists `/api/*` and `/dl/*`, and it is not
optional. The assets layer answers a request that matches no file by itself,
and for a **navigation** it answers with `404.html` without calling the Worker
at all. A browser turns a download into a navigation, so without that list
every download from the page 404s while `curl` and `fetch` — which are not
navigations — are served correctly. Test `/dl/` the way a browser uses it, with
`Sec-Fetch-Dest: document`, or in a browser. The pages must *not* be added to
the list: the assets layer serves `/changelog` from `changelog.html` itself,
and a Worker asking for `changelog.html` by name meets the `_redirects` rule
pointing back at `/changelog` — a redirect loop.

## Pages and their shared parts

The top bar, the pet's column, the footer and the common `<head>` tags are
written once, in `site/partials/`, and copied into every page between
`<!-- partial:name -->` markers by `npm run site:pages`. Edit the partial, run
that, and commit the pages; `npm run site:check` fails in CI if a page no longer
matches. The one difference between the copies is the nav: each page's own
link gets `aria-current="page"`.

Everything else in a page is its own: the title, the description and the other
tags a search engine reads, and the content.

## The pet on the page

The creature on every page is not a GIF or a sprite sheet: it is the app's
own animation code, running in the browser.

- `src/pet/web.ts` is the browser adapter — it builds the same DOM the app's
  Svelte component does (the sprite as SVG paths, at the app's own offsets),
  hands it a `PetController`, and feeds it what the visitor does: keys and clicks
  on the page, the pointer for its eyes, and the chips on the home page (and the
  two faces on `/about`) for one animation at a time. The number the app keeps
  over its head is an option here, and the site leaves it off. The pet can be
  dragged anywhere on the page, and stays where it is put.
- `scripts/build-site-pet.ts` bundles that (with the sprite data) into
  `assets/pet.js` with vite — about 9 KB gzipped. `npm run site:assets` runs it,
  and `npm run site:check` fails if the bundle no longer matches the source.
- `assets/jokbet-poster.png` is the still frame shown where the pet will be, so
  a browser without JavaScript — or the first paint, before the bundle arrives —
  still has the creature on it. The live sprite replaces it exactly.

So a change to the sprites, the poses or the controller changes the website's
pet as well, and CI notices if the generated files were not regenerated.

## Two ways to every file

- **Downloads from the site.** The buttons and lists link `/dl/<tag>/<name>`.
  The Worker streams the file from R2 (ranges included, `immutable`, and
  `x-robots-tag: noindex`); if the object is not there, or R2 cannot be reached,
  it redirects to the same file on GitHub. Each row also links GitHub directly.
  If `/api/latest.json` came from the fallback copy rather than R2, the page
  says so and links GitHub straight away.
- **Updates in the app.** The updater asks `/api/update.json` first and GitHub's
  `releases/latest/download/latest.json` second (`tauri.conf.json`); a source
  that does not answer within 20 s is skipped. It downloads from wherever the
  manifest that answered points, and if that fails it retries the same
  `<tag>/<name>` on the other host (`src-tauri/src/updater.rs`). The signature
  in the manifest is the file's, so either copy verifies.

The bucket stays private; the Worker is the only door to it, and it will only
hand out names Tauri produces, under `releases/`.

## Working on it

```sh
npm run site:dev       # the whole thing locally: assets, routes, headers and a local R2 bucket
npm run site:deploy    # deploy it by hand (the Git build does this on every push to main)
npm run site:pages     # copy site/partials/ into every page
npm run site:assets    # regenerate site/assets from src/sprites and src/pet (pixel-exact)
npm run site:check     # fail if assets/, the pet bundle, a page or a fallback manifest has drifted
npm test               # includes worker/index.test.ts, with a stub bucket
npx tsc -p worker/tsconfig.json
```

`npm run site:dev` is `wrangler dev`: the routes, the `_headers` (CSP included),
the `_redirects` and the bindings are all the real ones, and the local bucket
starts empty — so the manifests come from `worker/fallback/` and every `/dl/`
link redirects to GitHub. That is the fallback path, end to end. To try the
mirror path, put objects into the *local* bucket (never forget `--local`):

```sh
npx wrangler r2 object put jokbet-downloads/manifests/latest.json --file worker/fallback/latest.json --local
npx wrangler r2 object put jokbet-downloads/releases/v0.1.0/Jokbet_0.1.0_aarch64.dmg --file <the dmg> --local
```

The Worker remembers what the bucket said for a minute, so give it one.

The poster, the favicons and the Open Graph card are generated from the same
sprite code the app draws, so the site can never drift from the pet. The Latin
display face is [Silkscreen](https://github.com/googlefonts/silkscreen) (SIL
Open Font License, see `site/assets/fonts/OFL.txt`); the Chinese text uses the
system font, which is why the pixel face is scoped to `U+0000-00FF`.

## Search engines

- Every page carries a title, a description, a canonical URL, `robots`
  directives and an Open Graph set; the home page also carries JSON-LD
  (`WebSite` + `SoftwareApplication` + `SoftwareSourceCode`, the licence and the
  repository included), the other pages a `WebPage` and a `BreadcrumbList`.
- `sitemap.xml` lists the four pages and `robots.txt` points at it, with `/api/`
  and `/dl/` excluded — nothing under them is a page.
- The `<title>` in the markup is bilingual, so it reads the same to a crawler as
  to a visitor who has not picked a language; the tab title follows the language
  only once someone switches it.
- Release files are served with `x-robots-tag: noindex`, and `404.html` is
  `noindex` as well.

## What is already set up (Cloudflare)

| Thing | Name | Notes |
|---|---|---|
| Worker | `jokbet-site` | the Git-connected build runs `npx wrangler deploy`; `wrangler.toml` is the source of truth for the name and the bindings |
| Custom domain | `jokbet.jokerben.top` | attached to that Worker (Workers → `jokbet-site` → Settings → Domains & Routes) |
| R2 bucket | `jokbet-downloads` | private; `/api/` and `/dl/` are the only way in |
| GitHub secrets | `R2_ACCOUNT_ID`, `R2_ACCESS_KEY_ID`, `R2_SECRET_ACCESS_KEY` | used by the *Publish to R2* workflow |

The bucket name is written into `wrangler.toml` and the publish workflow, so it
is not a secret; only the three secret values are.

## Releasing

1. Bump `version` in `package.json`, commit, tag `vX.Y.Z`, push the tag — CI
   builds every platform into a **draft** release (see the root README).
2. Check the draft, then **publish** it. That fires the *Publish to R2*
   workflow: every file of the release goes to `releases/<tag>/`, then the three
   manifests to `manifests/` — `latest.json` and `update.json` only if the
   release is GitHub's latest, so re-publishing an old tag cannot roll anyone
   back. The site picks them up within a minute.
3. Refresh the fallback copies in the repo and commit them, so the Worker's own
   copies name the same release as the bucket:

   ```sh
   npm run site:fallback   # worker/fallback/{latest,releases}.json, from the GitHub API (needs gh)
   ```
