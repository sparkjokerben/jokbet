# The website

`jokbet.jokerben.top` — two static pages (plus three Cloudflare Pages Functions)
served by Cloudflare Pages, with the installers in an R2 bucket. There is no
build step: what is in this directory is what is deployed.

```
site/
  index.html            the home page: hero, downloads, what it does, install, privacy, limits, licence
  changelog.html        every version, its notes and its installers
  styles.css            the app's palette, the layout, the pet's own looks
  app.js                the language switch, the download list, the changelog, and the page's half of the pet
  lang-boot.js          picks zh/en before the first paint (the app's own rule)
  _headers _routes.json robots.txt sitemap.xml
  latest.baked.json     the last-known-good manifest, served when R2 cannot be
  releases.baked.json   the same, for the changelog (empty until the first release)
  assets/               generated: the pet bundle, the poster, the wordmark, the fonts, the icons, the OG card
  functions/mirror.ts   the one JSON door to the bucket: bucket, then baked copy
  functions/latest.ts         GET /latest         — the newest release's manifest
  functions/changelog.json.ts GET /changelog.json — every published release
  functions/dl/               GET /dl/<name>      — the installer: R2 first, then GitHub
  wrangler.toml         the Pages project: name, output dir, the R2 binding
  tests/                vitest coverage of the functions
```

## The pet on the page

The creature on the home page is not a GIF or a sprite sheet: it is the app's
own animation code, running in the browser.

- `src/pet/web.ts` is the browser adapter — it builds the same DOM the app's Svelte
  component does (the sprite as SVG paths, the counter chip over its head, at the
  app's own offsets), hands it a `PetController`, and feeds it what the visitor
  does: keys and clicks on the page, the pointer for its eyes, and the chips
  under the hero for one animation at a time.
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
   bucket; if the bucket cannot be reached, from `latest.baked.json` in this
   directory; the page is told which one it got (`source: "r2" | "baked"`) and
   says so.
2. `/changelog.json` → every published release (the same fallback order, with
   `releases.baked.json`). The release job writes it from the GitHub releases
   API, so the page needs no third-party API at view time.
3. `/dl/<name>` → the installer is streamed from the bucket (ranges included,
   `cache-control: immutable`, since the names carry the version). If the object
   is not there — the older versions are never mirrored — or R2 is unreachable,
   the browser is redirected to the same file on GitHub, so no link on the page
   can dead-end.
4. When the manifest came from the baked copy and there is no version in it yet,
   the page skips `/dl/` altogether and points at GitHub.

The bucket stays private; the function is the only door to it, and the names it
will serve are checked against what Tauri produces.

## Working on it

```sh
npm run site:assets    # regenerate assets/ from src/sprites and src/pet (pixel-exact)
npm run site:check     # fail if assets/ or the pet bundle has drifted, or a manifest is invalid
npm test               # includes site/tests: the functions, with a stub bucket
npx tsc -p site/functions/tsconfig.json
```

To look at the page without Cloudflare, serve the directory and stub the two
JSON endpoints:

```sh
cp -r site /tmp/site-preview
cp site/latest.baked.json /tmp/site-preview/latest
cp site/releases.baked.json /tmp/site-preview/changelog.json
python3 -m http.server -d /tmp/site-preview 8099     # then open http://127.0.0.1:8099/
```

`/latest` and `/changelog.json` are functions, so a plain file server needs those
two files; the page fetches the endpoints first and treats what it gets as the
mirror's answer.

`npm run site:dev` runs the functions for real, under `wrangler pages dev`
(`--r2=DOWNLOADS`, since the dev server takes the binding from the flag rather
than from `wrangler.toml`) — this also applies `_headers`, so it is the way to
check the page under its real CSP. The bucket starts empty, so `/latest` answers
from `latest.baked.json`, `/changelog.json` from `releases.baked.json`, and every
`/dl/` link redirects to GitHub — the fallback path, end to end. Seeding an
object into the local bucket is fiddly (the CLI's `r2 object put --local` and the
dev server have to agree on `--persist-to`); `npm test` covers the R2 path
instead, and the deployed site is the real check.

The poster, the favicons and the Open Graph card are generated from the same
sprite code the app draws, so the site can never drift from the pet. The Latin
display face is [Silkscreen](https://github.com/googlefonts/silkscreen) (SIL Open
Font License, see `assets/fonts/OFL.txt`); the Chinese text uses the system font,
which is why the pixel face is scoped to `U+0000-00FF`.

## Setting it up (once, in the Cloudflare dashboard)

1. **R2** → *Create bucket* → `jokbet-downloads`. Leave it private:
   the download function is the only way in.
2. **R2** → *Account details* → *Manage* API tokens → **Create Account API
   token** → permissions **Object Read & Write**, scoped to that bucket → copy
   the **Access Key ID** and **Secret Access Key** (shown once), and the
   **Account ID**.
3. **Workers & Pages** → *Create* → **Pages** → *Connect to Git* → pick this
   repository. Production branch `main`, **framework preset None**, **build
   command empty**, **root directory `site`**, **build output directory `.`**.
   `wrangler.toml` in this directory carries the project name and the R2
   binding, and Pages treats the file as the source of truth for both (those
   fields then become read-only in the dashboard). If the form refuses `.` as
   the output directory, set the root directory to the repository root, the
   output directory to `site`, and move `functions/` and `wrangler.toml` up to
   the repository root instead — Pages wants `functions/` at the project root,
   not inside the published directory.
4. **Pages** → the project → *Custom domains* → *Set up a domain* →
   `jokbet.jokerben.top`. With `jokerben.top` in the same Cloudflare account the
   CNAME is added for you; skipping this step leaves the hostname returning 522.
5. **GitHub** → the repository → *Settings* → *Secrets and variables* →
   *Actions* → **New repository secret**, three of them:

   | Secret | Value |
   |---|---|
   | `R2_ACCOUNT_ID` | the Cloudflare account id from step 2 |
   | `R2_ACCESS_KEY_ID` | the R2 token's access key id |
   | `R2_SECRET_ACCESS_KEY` | the R2 token's secret access key |

The bucket name is written into `wrangler.toml` and the publish workflow, so it
is not a secret; only the three values above are.

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
