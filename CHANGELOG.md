# Changelog

Notable changes, newest first. This file lists the latest releases and links to
the fuller entries elsewhere:

- [Releases](https://github.com/sparkjokerben/jokbet/releases) — every release
  with its notes and its installers
- [jokbet.jokerben.top/changelog](https://jokbet.jokerben.top/changelog) — the
  same, browsable, with checksums

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and
the project uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.0] — 2026-09-25

### Added

- **When in the day you are busiest.** The stats window gains an activity-by-hour
  chart, drawn over whichever range is selected, with the busiest hour named
  below it and every hour listed in its data table. Counts are filed under the
  hour they happened in from this version on, so days counted earlier have
  nothing to show here, and the chart says so rather than drawing an empty day.
- **Insights from beyond the range.** Four tiles under the ones already there:
  your busiest day ever and when it was, how many days in a row you are
  currently active, the longest run you have ever had, and the last seven days
  beside the seven before them. All of them follow the metric you have picked
  and reach back further than the range on screen can.
- **A third CSV file.** Exporting writes `jokbet-hourly.csv` beside the daily and
  per-key files, one row for each day and hour.

### Changed

- **Update notes read as notes.** The About section used to print the release
  notes exactly as they are written for GitHub: both languages, the bilingual
  footer, and every `**` and `[link](url)` in sight. It now keeps the part in
  your language, drops the footer, and renders paragraphs, lists, bold and code
  as text, never as HTML. A link keeps its words only, since following it would
  take the Settings window away.

## [0.2.0] — 2026-09-24

### Added

- **It steps aside in full screen.** While another app plays video, presents or
  runs a game full screen, the pet hides, and comes back afterwards. A setting
  turns this off.
- **Global shortcuts.** One shows or hides the pet, one pauses and resumes
  counting, from any app. None is set until you record one under
  **Settings › Shortcuts**, so no combination is taken from other apps.
- **Choose the interface language.** Follow the system, 中文 or English; the tray
  menu and every window switch together.
- **An About section in Settings.** The version, a Check for Updates button, the
  release notes once an update has downloaded, and Restart and Update. The pet
  also says when a download finishes.
- **Move Pet Back to Corner** in the tray menu, for when the pet has been dragged
  somewhere you cannot find it.
- **It says when something goes wrong.** Stats that cannot be loaded, a database
  that cannot be opened or a damaged settings file used to fail silently; each is
  now reported, and a damaged settings file keeps whatever can still be read. A
  log file is new too, attachable from **Settings › About › Open Logs Folder**.

### Changed

- **The heatmap is drawn on the keyboard you have.** An Apple board on a Mac (the
  compact MacBook one, or the full-size one once a numeric keypad has been used),
  a PC board on Windows and Linux, with European ISO boards recognised. It comes
  in the pet's orange by default, or on the usual yellow-to-red heat scale, each
  with its own light and dark shades.

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

[0.3.0]: https://github.com/sparkjokerben/jokbet/releases/tag/v0.3.0
[0.2.0]: https://github.com/sparkjokerben/jokbet/releases/tag/v0.2.0
[0.1.2]: https://github.com/sparkjokerben/jokbet/releases/tag/v0.1.2
[0.1.1]: https://github.com/sparkjokerben/jokbet/releases/tag/v0.1.1
[0.1.0]: https://github.com/sparkjokerben/jokbet/releases/tag/v0.1.0
