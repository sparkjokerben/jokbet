# Changelog

Notable changes, newest first. This file lists the latest releases and links to
the fuller entries elsewhere:

- [Releases](https://github.com/sparkjokerben/jokbet/releases) — every release
  with its notes and its installers
- [jokbet.jokerben.top/changelog](https://jokbet.jokerben.top/changelog) — the
  same, browsable, with checksums

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and
the project uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.2] — 2026-09-24

### Added

- The macOS build now ships as a `.zip` as well as a `.dmg`. Gatekeeper stops a
  downloaded disk image when it is opened and then the app inside it; out of a
  zip, only the app. The download page lists both and says how many stops each
  costs, and the home page offers the zip.
- `install.sh` on the website: one line in Terminal installs the newest release
  without any Gatekeeper prompt, checks it against the published SHA-256, and
  updates the app in place when run again.

### Changed

- **Milestones now count what the head counter counts.** The built-in daily and
  all-time milestones used to count key presses only, while the head counter
  defaulted to keys + clicks, so the two numbers could disagree. Both now come
  from the same setting, and the stats window gained a matching keys + clicks
  metric.

### Removed

- **The blindfold.** The pet used to wear one while macOS Secure Keyboard Entry
  was on, but clicks, mouse movement and the modifier keys were still being
  counted at the time, so the animation misrepresented what the pet could see.
  A missing Input Monitoring permission is the only state that leaves it unable
  to count anything, and it still shows a confused face for that.

## [0.1.1] — 2026-09-24

### Added

- Updates have a second route. Checking for updates asks
  `jokbet.jokerben.top` first and GitHub second, and a download that fails or
  stalls on one host is retried on the other. Both hosts serve the same files,
  verified against the same signature.

## [0.1.0] — 2026-09-24

### Added

- First public release. A desktop pet that counts key presses per key,
  left/right/middle clicks, scroll gestures, and mouse travel, stored per day;
  a head counter for today's total or the live rate; hover stats; a menu from
  the tray; milestones; a stats window with a trend and a keyboard heatmap; and
  an interface in English and Simplified Chinese.

[0.1.2]: https://github.com/sparkjokerben/jokbet/releases/tag/v0.1.2
[0.1.1]: https://github.com/sparkjokerben/jokbet/releases/tag/v0.1.1
[0.1.0]: https://github.com/sparkjokerben/jokbet/releases/tag/v0.1.0
