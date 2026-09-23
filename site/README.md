# The website

`jokbet.jokerben.top` — one static page (plus two Cloudflare Pages Functions)
served by Cloudflare Pages, with the installers in an R2 bucket. There is no
build step: what is in this directory is what is deployed.

```
site/
  index.html            the page: hero, downloads, features, install, privacy, limits, footer
  styles.css            the app's palette and type, one column
  app.js                flipbooks, the download list from the manifest, the language switch
  lang-boot.js          picks zh/en before the first paint (the app's own rule)
  _headers _routes.json robots.txt
  latest.baked.json     the last-known-good manifest, served when R2 cannot be
  assets/               generated: sprite sheets, wordmark, icons, og card (see below)
  functions/latest.ts   GET /latest  — the manifest: R2 first, then the baked copy
  functions/dl/         GET /dl/<name> — the installer: R2 first, then GitHub
  wrangler.toml         the Pages project: name, output dir, the R2 binding
  tests/                vitest coverage of the two functions
```

## How a download works

The page never talks to R2 or GitHub itself; it links `/dl/<name>?v=<version>`
and says which source answered.

1. `/latest` → the manifest. It comes from `latest.json` in the bucket; if the
   bucket cannot be reached, from `latest.baked.json` in this directory; the
   page is told which one it got (`source: "r2" | "baked"`) and says so.
2. `/dl/<name>` → the installer is streamed from the bucket (ranges included,
   `cache-control: immutable`, since the names carry the version). If the object
   is not there, or R2 is unreachable, the browser is redirected to the same
   file on GitHub — so no link on the page can dead-end.
3. When the manifest came from the baked copy, the page skips `/dl/` altogether
   and points every download straight at GitHub.

The bucket stays private; the function is the only door to it, and the names it
will serve are checked against what Tauri produces.

## Working on it

```sh
npm run site:assets    # regenerate assets/ from src/sprites (pixel-exact)
npm run site:check     # fail if assets/ has drifted from the sprites, or the manifest is invalid
npm test               # includes site/tests: the two functions, with a stub bucket
npx tsc -p site/functions/tsconfig.json
```

To look at the page without Cloudflare, serve the directory and stub the
manifest:

```sh
cp -r site /tmp/site-preview && cp site/latest.baked.json /tmp/site-preview/latest
python3 -m http.server -d /tmp/site-preview 8099     # then open http://127.0.0.1:8099/
```

`/latest` is a function, so a plain file server needs that `latest` file (the
page fetches `/latest` first and treats it as the mirror's answer).

`npm run site:dev` runs the functions for real, under `wrangler pages dev`
(`--r2=DOWNLOADS`, since the dev server takes the binding from the flag rather
than from `wrangler.toml`). The bucket starts empty, so `/latest` answers from
`latest.baked.json` and every `/dl/` link redirects to GitHub — the fallback
path, end to end. Seeding an object into the local bucket is fiddly (the CLI's
`r2 object put --local` and the dev server have to agree on
`--persist-to`); `npm test` covers the R2 path instead, and the deployed site
is the real check.

The mascot, the favicons and the Open Graph card are generated from the same
sprite code the app draws, so the site can never drift from the pet: change a
sprite, run `npm run site:assets`, and commit what it writes.

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

If you already created the bucket or the Pages project under the old name (`jokerben-desktop-pet-downloads`, `jokerben-desktop-pet-site`), either rename them in the dashboard or put the old names back in `wrangler.toml` and the publish workflow — the names are just strings on both sides.

The bucket name is written into `wrangler.toml` and the publish workflow, so it
is not a secret; only the three values above are.

## Releasing

1. Bump `version` in `package.json`, commit, tag `vX.Y.Z`, push the tag — CI
   builds every platform into a **draft** release (see the root README).
2. Check the draft, then **publish** it. That fires the *Publish to R2*
   workflow, which copies the installers and the manifest into the bucket. The
   page picks the new version up within a minute (the manifest is cached for
   60 s).
3. `npm run site:manifest` refreshes `site/latest.baked.json` to the new
   version and commit it, so the fallback names the same release as the mirror.
