# Changelog

Notable changes, newest first. This file lists the latest releases and links to
the fuller entries elsewhere:

- [Releases](https://github.com/sparkjokerben/jokbet/releases) — every release
  with its notes and its installers
- [jokbet.jokerben.top/changelog](https://jokbet.jokerben.top/changelog) — the
  same, browsable, with checksums

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and
the project uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.4.0] — 2026-09-28

### Added

- **Backups.** **Settings › Data** saves your counts and your settings to one
  `.zip`, and restores from one. A restore replaces every count with the
  backup's, and the settings too if you tick the box — though never where the
  pet stands. The counts it replaces go into a safety backup first, so a
  restore can itself be undone.
- **A backup every day.** Jokbet backs itself up once a day, in a `backups`
  folder beside its data or in a folder you pick (a synced one keeps it off this
  disk), and keeps the newest seven; both are settings. Each computer names and
  tidies its own, so two sharing a folder leave each other's alone.
- **A break reminder.** After 50 minutes of typing, clicking and scrolling
  without a five-minute pause, the pet stretches, yawns and tells you to get up,
  and again every quarter of an hour until you do; a click on the pet says you
  have seen it. Both lengths are settings, and so is the reminder itself. It
  waits while the pet is hidden, and nothing adds up while counting is paused.
- **Strolls.** While idle, the pet now and then takes a walk along the bottom of
  the screen — on the Dock or the taskbar — turned the way it is going and
  bobbing as it steps. It picks a spot anywhere along the way and takes three to
  fifteen seconds to get there, so a long way in a short time is a brisk walk
  and a short way in a long one an amble. It stops the moment you type, click or
  scroll, and stays where it stopped. A pet put down in the middle of the screen
  stays put. The one on the website strolls too.

### Changed

- **Idle animations are a choice of several.** Tick any of juggling, looking
  around and strolling, and the pet picks one of them at random now and then;
  tick none and it just breathes, which is what "Breathe" used to mean. An
  existing install keeps the one it had, so tick **Stroll** to see it walk.
- **Scrolling keeps the pet awake.** Reading down a long page no longer puts it
  to sleep; it still acts nothing out for a scroll.

## [0.3.3] — 2026-09-28

### Fixed

- **The pet stays out of the way of every full-screen app on macOS.** With
  "Hide in full screen" on, the pet went away for some apps (VS Code) but only
  flickered and came back for others (Edge, Mail), and it reappeared for all of
  them whenever the title bar was pulled down. It was judging full screen by
  whether the frontmost window covered the display, and a full-screen app's
  toolbar and title bar sit in a window of their own in front of it. It now asks
  the system whether the display is showing a full-screen Space, which needs no
  permission; a window covering the display still counts, for games and
  presentations that go full screen without one.
- **The hover card stays up while the pet is dragged.** On macOS the window
  trails the cursor a little while it is carried, and a quick drag put the
  cursor off the pet long enough to take the card down and bring it back, over
  and over. The pet counts as under the cursor for the whole of a drag now.
- **The words on the Liquid Glass card stay readable over a dark desktop.** The
  labels were a fixed grey, which sank into a grey desktop showing through the
  glass. On macOS the quieter words on the glass are now only a little lighter
  than the rest.

## [0.3.2] — 2026-09-27

### Fixed

- **The material on Windows is the acrylic, in a window of its own.** The switch
  that appeared as "Liquid Glass" on Windows put a blur behind a region of the
  pet's own window: a rectangle cut out of the pet's window, which followed the
  pet badly and had nothing to say about what it was covering. It is the acrylic
  now — the material that shows what is under it through — in a window of its
  own, cut to the card's rounded shape and kept directly under the pet. Windows
  10 1803 and later, and the switch is named for the material the system
  actually has: Acrylic on Windows, Liquid Glass on macOS 26 and vibrancy before
  it.
- **The card is drawn from what is behind it.** The material shows the desktop
  through, so the panel is drawn from what is under it rather than from the
  system's light or dark — which is what put dark ink over a dark desktop
  whenever the system was light, and a grey of its own that washed out wherever
  the desktop was neither dark nor pale. It reads the desktop now: dark ink over
  a pale one, light ink over a dark one, every word on the card in that one ink,
  and a little of the card's own colour where the desktop is too mixed to read
  against. A reading drifts into the next rather than stepping, the ink changes
  hands once and fades across, and a card that is taken away comes back in the
  colour it went down in instead of reading the desktop afresh and correcting
  itself in front of you. macOS keeps the material, and the card, it has always
  had.
- **The material goes where the pet goes.** It is a window of its own, so it has
  to be told: it follows the pet through a drag, stays with the cursor while the
  right-click menu holds the thread the page would have said so on, and goes
  away with the pet when the pet is hidden.

### Changed

- **The panel tint starts at 0** — the bare material — on every platform.

## [0.3.1] — 2026-09-25

### Fixed

- **A crash when the update arrived with the menu open.** Once an update had
  been downloaded, the app added "Restart to Update" to the top of the menu it
  shares with the pet's right-click menu — from the updater's own thread, and
  while macOS was displaying that menu. macOS aborts over a menu changed while
  it is on screen, and the release build aborts on any panic, so the app
  disappeared seconds after being launched with the menu open. Nothing is added
  to the menu after it is built now: the update is announced by the pet's
  bubble and by **Settings › About**, which is also where it is installed from.
  The menu changes that remain happen on the main thread, and the release
  profile no longer aborts the process over a panic.
- **The charts' tooltips land on what you are pointing at.** Hovering the daily
  trend, or the activity-by-hour chart that arrived in 0.3.0, highlighted a
  column about two columns to the left of the cursor: the width of the y-axis
  was taken off the pointer's position twice, once by the hit area and once
  again by the arithmetic. It had been that way since the stats window existed.

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

[0.4.0]: https://github.com/sparkjokerben/jokbet/releases/tag/v0.4.0
[0.3.3]: https://github.com/sparkjokerben/jokbet/releases/tag/v0.3.3
[0.3.2]: https://github.com/sparkjokerben/jokbet/releases/tag/v0.3.2
[0.3.1]: https://github.com/sparkjokerben/jokbet/releases/tag/v0.3.1
[0.3.0]: https://github.com/sparkjokerben/jokbet/releases/tag/v0.3.0
[0.2.0]: https://github.com/sparkjokerben/jokbet/releases/tag/v0.2.0
[0.1.2]: https://github.com/sparkjokerben/jokbet/releases/tag/v0.1.2
[0.1.1]: https://github.com/sparkjokerben/jokbet/releases/tag/v0.1.1
[0.1.0]: https://github.com/sparkjokerben/jokbet/releases/tag/v0.1.0
