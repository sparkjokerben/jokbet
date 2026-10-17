# The website

[`jokbet.jokerben.top`](https://jokbet.jokerben.top) — five static pages, served
by a Cloudflare Worker that also answers two namespaces: the manifests
(`/api/`) and the release files (`/dl/`), both mirrored in an R2 bucket with
GitHub behind them. There is no build step: what is in `site/` is what is
deployed.

This document covers the addresses, the pages and the release flow. The
application itself is in the [root README](../README.md); the check that CI runs
over this directory is in [CONTRIBUTING.md](../CONTRIBUTING.md).

## Addresses

| Address | What it is | Served from |
|---|---|---|
| `/` | the pet, what it does, the download buttons | `site/index.html` |
| `/download` | every installer of the newest release, checksums, install steps | `site/download.html` |
| `/changelog` | every release: its notes and its installers | `site/changelog.html` |
| `/about` | privacy, known limits, the licence, notices | `site/about.html` |
| `/stats` | every number behind the download counts: unlisted, `noindex`, in no sitemap | `site/stats.html` |
| `/api/latest.json` | the newest release, for the buttons and `/download` | R2 `manifests/latest.json`, else `worker/fallback/latest.json` |
| `/api/releases.json` | every published release, for `/changelog` | R2 `manifests/releases.json`, else `worker/fallback/releases.json` |
| `/api/update.json` | the app's updater manifest, its downloads pointed at `/dl/` | R2 `manifests/update.json`, else a redirect to GitHub's |
| `/api/downloads.json` | the download counts: what the mirror served and what GitHub counted | D1 `jokbet-stats`, and R2 `manifests/downloads.json` for GitHub's half |
| `/dl/<tag>/<name>` | one file of one release | R2 `releases/<tag>/<name>`, else a redirect to GitHub |
| `/install.sh` | the one-line installer for macOS and Linux | `site/install.sh` |
| `/install.ps1` | the one-line installer for Windows | `site/install.ps1` |

Every release file has one address per host, the same shape on both:
`/dl/<tag>/<name>` here, `releases/download/<tag>/<name>` on GitHub. The site's
links, the Worker's fallback and the app's updater all move between the two
without looking anything up, and a version in the path means an address never
changes what it serves.

The two installers are the way past Gatekeeper and SmartScreen, and the way the
download page recommends. A build a browser downloaded is quarantined on a Mac
and marked on Windows, and each system then stops an un-notarized or unsigned
one — on a Mac a disk image when it is opened and the app inside it once more,
on Windows the installer itself. `curl` and PowerShell's own download mark
nothing, so a build installed this way opens straight away.

`curl -fsSL https://jokbet.jokerben.top/install.sh | sh` installs the newest
release's `aarch64`/`x64` disk image for the Mac it runs on, or the AppImage on
Linux (into `~/.local/bin/jokbet`), checks it against the manifest's SHA-256 and
opens it. Running it again updates in place: on a Mac it asks the running app to
quit the way its own Quit item does, so the counts in hand are saved first.
`irm https://jokbet.jokerben.top/install.ps1 | iex` does the same on Windows
with the NSIS installer, run silently (`/S`) — the same way the app's own
updater runs it, which is what closes a running copy. It needs Windows
PowerShell 5.1 or PowerShell 7; `iex` of a string is not a script file, so the
execution policy does not come into it. cmd has no `irm`, so the page gives it
the one line that calls PowerShell: `powershell -NoProfile -Command "irm … |
iex"`.

Both are served as `text/plain; charset=utf-8`, so a browser shows them rather
than saving them, `irm` hands the page's bytes to `iex` with the right charset,
and the download page links to both for reading first.

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
wrangler.toml            the Worker: name, entry point, assets, run_worker_first, the R2 and D1 bindings
worker/index.ts          the router: /api/*, /dl/*, then the assets
worker/api.ts            the manifests, each with its own fallback, and the download counts
worker/download.ts       the release files: R2 first, then GitHub, and one row counted per download
worker/stats.ts          those rows, and the sums /api/downloads.json serves
worker/migrations/       the counter's table, for `wrangler d1 migrations apply`
worker/fallback/         the manifests compiled into the Worker, for when R2 cannot answer
worker/index.test.ts     vitest coverage of all of it, with a stub bucket

site/
  index.html download.html changelog.html about.html stats.html 404.html
  partials/              what every page shares: head, header, rail (the pet), footer
  styles.css             the app's palette, the layout, the pet's own looks
  app.js                 the language switch, the pet, and what each page builds from /api/
  boot.js                picks zh/en before the first paint; forwards the old anchors
  install.sh             the one-line installer for macOS and Linux: manifest, checksum, ditto or AppImage, open
  install.ps1            the one-line installer for Windows: manifest, checksum, the NSIS installer, open
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

## The download counts

The number on the home page and the download page is two counts added together,
and `/stats` — linked from nowhere, `noindex`, in no sitemap — is where both are
written out, under a statement of what is and is not in them.

- **The mirror's half** is the Worker's own count. Every whole file `/dl/` serves
  from the bucket writes a row into the D1 table (`worker/migrations/`), off the
  response path: a download is never slowed down or broken by the database, and a
  metadata probe, a resumed range or a failed lookup writes nothing. One query
  sums those rows by day, platform, version, client class and country. Nothing
  identifies anyone — no address, no cookie, and the user agent is reduced to one
  of four words before it is stored.
  The same mirror totals include the R2 history we could still recover on
  2026-10-17: successful asset reads whose response contained the whole object,
  from 2026-09-24 onward. Partial reads and manifest requests were left out; the
  old metrics have no client or country, so their client is `unknown` and no
  country is inferred. Cloudflare's R2 metrics expose at most 31 days, so anything
  older than that window cannot be recovered there. A Range that happened to read
  the entire object cannot be distinguished from a full GET; neither metric proves
  that a visitor saved the file.
- **GitHub's half** is the counters on its own release assets, which everyone who
  downloads from GitHub and the app's updater both feed. Only GitHub has those
  numbers and only GitHub moves them, so the *Refresh the download counts*
  workflow reads them once a day — and the release job once per release — into
  `manifests/downloads.json` in the bucket. The Worker cannot do it itself: its
  outbound addresses are shared with every other Worker, and GitHub's API allows
  60 unauthenticated requests an hour per address.
- **Neither half counts the other's downloads.** A request the bucket cannot
  serve is redirected to GitHub, which counts it there, so the Worker does not
  count the redirect. And GitHub's numbers carry the pipeline's own fetches —
  publishing a release downloads the whole release once — so each file is
  reduced by what publishing cost it before it is added up, and a signature file,
  which is read out of the manifest rather than downloaded, counts for nothing.
  `/stats` shows GitHub's raw number beside the corrected one.

Read them by hand:

```sh
npx wrangler d1 execute jokbet-stats --remote --command \
  "SELECT day, SUM(n) AS n FROM (SELECT day, count(*) AS n FROM downloads GROUP BY day UNION ALL SELECT day, SUM(downloads) AS n FROM download_history GROUP BY day) GROUP BY day ORDER BY day DESC LIMIT 30"
curl -s https://jokbet.jokerben.top/api/downloads.json
```

## Working on it

```sh
npm run site:dev       # the whole thing locally: assets, routes, headers and a local R2 bucket
npx wrangler d1 migrations apply jokbet-stats --local    # …its counter's table (--remote for the real one)
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

The Worker remembers what the bucket said for a minute, so give it one. Its
counter writes into the local D1 database; without the migration above, the
download is served and the row is dropped — which is what a database that is
down looks like from the outside, and is why no count is ever allowed to be the
reason a download fails.

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
- `sitemap.xml` lists the four published pages and `robots.txt` points at it,
  with `/api/`, `/dl/` and `/stats` kept out — nothing under the first two is a
  page, and the third is a page nobody is meant to find.
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
| D1 database | `jokbet-stats` | the download counter's rows, binding `STATS`; migrations in `worker/migrations/` |
| GitHub secrets | `R2_ACCOUNT_ID`, `R2_ACCESS_KEY_ID`, `R2_SECRET_ACCESS_KEY` | used by the *Publish to R2* and *Refresh the download counts* workflows |

The bucket name is written into `wrangler.toml` and the publish workflow, so it
is not a secret; only the three secret values are.

## Releasing

1. Bump `version` in `package.json`, commit, tag `vX.Y.Z`, push the tag — CI
   builds every platform into a **draft** release (see the root README).
2. Check the draft, then **publish** it. That fires the *Publish to R2*
   workflow: every file of the release goes to `releases/<tag>/`, then the three
   manifests to `manifests/` — `latest.json` and `update.json` only if the
   release is GitHub's latest, so re-publishing an old tag cannot roll anyone
   back. `update.json` takes its notes from the release's body, not from the
   copy Tauri wrote into the draft: those notes are written when the draft is
   published, and the app shows them. The site picks them up within a minute.
3. Refresh the fallback copies in the repo and commit them, so the Worker's own
   copies name the same release as the bucket:

   ```sh
   npm run site:fallback   # worker/fallback/{latest,releases}.json, from the GitHub API (needs gh)
   ```
