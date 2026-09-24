<!--
Thanks for the pull request. Keep it small enough to review in one sitting; if it
grew past that, say so and it can be split.
-->

## What this changes

<!-- One paragraph, in the present tense: what is different after this lands. -->

## Why

<!-- The problem it solves, or the issue it closes: "Closes #123". -->

## How it was tested

<!--
Which commands you ran, and which platforms you tried it on. If it changes how
something looks, say what you looked at — screenshots welcome.
-->

## Checklist

- [ ] `npm run check`, `npm test`, and `cd src-tauri && cargo test` pass
- [ ] `cd src-tauri && cargo fmt --check && cargo clippy --all-targets -- -D warnings` is clean
- [ ] If it touched `src/sprites/` or `src/pet/`, `npm run site:assets` was run and the result committed
- [ ] If it touched `site/partials/`, `npm run site:pages` was run and the result committed
- [ ] If it changed visible text, both languages were updated (`src/lib/i18n.ts`, `src-tauri/src/i18n.rs`)
- [ ] If it touched the input hooks, the platforms tested are named above
