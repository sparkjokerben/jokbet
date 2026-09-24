# Contributing to Jokbet

Thanks for taking the time to help. This document covers getting the project
running, the conventions it follows, and what a pull request should look like.

By taking part you agree to the [Code of Conduct](CODE_OF_CONDUCT.md). For
anything larger than a bug fix — a new feature, a change to how something looks,
a new setting — please open an issue first so the approach can be agreed on
before you spend time on it.

## What the project needs

Node 24+ and a stable Rust toolchain. On Linux you also need the WebKit and
AppIndicator development packages that the CI workflow installs
(`libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev patchelf`).

```sh
npm install
npm run tauri dev        # run the app, with hot reload for the frontend
npm test                 # frontend tests (vitest)
npm run check            # svelte-check: types and Svelte diagnostics
cd src-tauri && cargo test
```

`npm run dev` on its own serves the frontend at `http://localhost:1420`. The
dev-only page `/preview.html?view=stats` (`settings`, `onboarding`, `pet`)
renders a window in a normal browser against fake data, so layout work does not
need the Tauri shell or a rebuilt app.

The complete check that CI runs on every push and pull request:

```sh
npm run check && npm test && npm run build
npm run site:check
npx tsc -p worker/tsconfig.json
cd src-tauri && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
```

## Layout

| Path | What is in it |
|---|---|
| `src/app/` | the on-demand windows: stats, settings, onboarding |
| `src/pet/` | the pet window: the controller, the sprite renderer, the hover card |
| `src/sprites/` | the pixel art — poses are composed from parts, plus frames decoded from the reference recordings |
| `src/lib/` | shared types, formatting, and the zh/en UI strings |
| `src-tauri/src/` | the Rust side: the input hooks, the counting engine, the database, the tray and menus, the updater |
| `src-tauri/src/input/` | one listen-only global hook per platform behind one interface |
| `src-tauri/src/engine/` | what to do with the events: aggregate, rate, distance, milestones |
| `worker/` | the Cloudflare Worker that serves the website's `/api/` and `/dl/` routes |
| `site/` | the website — see [`site/README.md`](site/README.md) |
| `scripts/` | the generators: sprites, site assets, manifests, page partials |
| `tools/extract-reference-frames/` | how the reference recordings became pixel frames |

`src-tauri/src/lib.rs` wires the Rust modules together; `src/app/main.ts` and
`src/pet/main.ts` are the two frontend entry points.

## Generated files

Some files in the repository are build output that is committed on purpose, so
that the website and the app icons can never drift from the pixel art. If you
change their source, regenerate them and commit the result — CI fails otherwise:

| If you change | Run | Which rewrites |
|---|---|---|
| `src/sprites/`, `src/pet/` | `npm run site:assets` | `site/assets/` (the pet bundle, the poster, the icons, the OG card) |
| `site/partials/` | `npm run site:pages` | the `<!-- partial:… -->` blocks in `site/*.html` |
| anything a release manifest describes | `npm run site:fallback` | `worker/fallback/*.json` (needs `gh`, and a published release) |

`npm run site:check` verifies all of it, and `sprites.png` — a contact sheet of
every animation frame — is git-ignored and purely for looking at.

The screenshots in the README are the app's own `/preview.html?view=pet-frame`
page rather than hand-taken: a headless browser opens it over `npm run dev` at
the pet window's real size (220×271 px at the default pet scale of 3.5), a plain
desktop color is injected behind it, the milestone banner is allowed to pass so
the head counter shows, and the capture is clipped evenly top and bottom. It is
done twice, with `navigator.language` set to `en-US` and to `zh-CN`, giving
`docs/screenshot.png` and `docs/screenshot.zh-CN.png`. Redo both when the pet's
looks or the hover card change.

## Conventions

**Comments and documentation are in English**, and explain why rather than what.
Commits are in English, imperative mood, and say what changed and what it is
for; look at `git log` for the register to match.

Rust is formatted with `cargo fmt` and is expected to be clean under
`cargo clippy --all-targets -- -D warnings`. TypeScript goes through
`svelte-check`; there is no formatter, so match the file you are editing.

Keep the two halves of the counting pipeline honest. Anything hooked into the
input path (`src-tauri/src/input/`) must stay allocation-free and non-blocking —
the hook callbacks only normalise an event and drop it into a channel. Counts are
aggregated in `src-tauri/src/engine/`, and the pet's animation state is pure and
testable in `src/pet/machine.ts`.

The interface is bilingual. A new string goes in **both** `src/lib/i18n.ts` (the
windows and the pet) and `src-tauri/src/i18n.rs` (the tray, menus, and native
dialogs) when it appears in those places, with the Chinese written as Chinese
rather than translated word for word. Layouts are checked in both languages:
Chinese strings are shorter but the pixel font only covers Latin.

## The website

The pages, the Worker, the release mirror, and their addresses are documented
in [`site/README.md`](site/README.md). Two things are easy to get wrong there:
`run_worker_first` in `wrangler.toml` must keep listing `/api/*` and `/dl/*`
(downloads are navigations, and the assets layer would answer them with the 404
page), and the pages themselves must *not* be added to that list.

Local work does not need Cloudflare credentials:

```sh
npm run site:dev         # wrangler dev: real routes, headers and bindings, local R2
```

Test `/dl/` the way a browser asks for it, with
`curl -H 'Sec-Fetch-Dest: document' -H 'Sec-Fetch-Mode: navigate'`; a plain
`curl` is not a navigation and hides routing mistakes. Never write to the real
bucket from a local shell — pass `--local` to every `wrangler r2 object` command.

## Sending a change

1. Fork the repository and branch off `main`.
2. Make the change, with tests where the behaviour is testable.
3. Run the full check above. If you touched anything in the generated-files
   table, regenerate and commit it.
4. Open a pull request describing what changed and why. Screenshots help for
   anything visual — include one at the pet's real size, and one in each
   language if the text changed.
5. CI runs on macOS, Windows, and Linux; all three need to pass.

Small, focused pull requests are much easier to review than large ones. If a
change touches the input hooks, say which platforms you tested it on.
