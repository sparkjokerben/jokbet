# Security policy

## Reporting a vulnerability

Please report vulnerabilities **privately**, through GitHub's
[private vulnerability reporting](https://github.com/sparkjokerben/jokbet/security/advisories/new)
(Security → Advisories → Report a vulnerability). Do not open a public issue for
something that could be exploited before a fix is out.

A useful report includes what an attacker gains, the steps to reproduce it, the
platform and version you tested, and any proof of concept. Expect an
acknowledgement within about a week. This is a project maintained in spare time
by one person, with no bug bounty — but a report that leads to a fix will be
credited in the release notes unless you would rather it were not.

## Supported versions

Only the newest release is supported. Jokbet updates itself, so a fix ships as a
new release rather than as a patch to an older one.

## What counts as a vulnerability

Jokbet observes every keystroke and click on the machine, stores them, and
installs updates of itself. That is a lot of trust, and these are the parts
worth attacking:

- **Anything that bypasses the update signature.** The updater verifies a
  minisign signature over the downloaded file before installing it. A way to
  install a file that does not verify — including a downgrade to an older signed
  release, or a signature check that can be skipped under some condition — is a
  serious finding.
- **Escaping the counting boundary.** The app is meant to know *how many* times
  a key was pressed and nothing more. If key codes, characters, typed words,
  window titles, or anything else about your input can leave the machine, in the
  database, in an export, or over the network, that is a bug.
- **Memory safety and FFI.** The input hooks call into platform APIs from Rust
  (`src-tauri/src/input/`) and the macOS window code uses Objective-C messaging.
  An out-of-bounds write, a use-after-free, or an unsound `unsafe` block that an
  attacker can reach through ordinary use is in scope.
- **The website and its Worker.** Path traversal or an unvalidated name in
  `/dl/`, reading objects the visitor should not have, injecting markup through a
  release's notes into a page, or anything that gets script execution on
  `jokbet.jokerben.top`.
- **The installer.** `site/install.sh` is fetched and run by `curl | sh`. A way
  to make it install something other than the release it verified — through
  argument, environment, or filename injection — is in scope.
- **Denial of service specific to the app:** a way to make it consume unbounded
  memory or CPU from ordinary input, or to wedge the input hook permanently.

## What is not a vulnerability

- **That the app reads global input.** That is what it is for; macOS and Windows
  require explicit user consent for it, and the app cannot run without it.
- **That counts are stored unencrypted in a local SQLite file.** They are counts,
  readable by the user who created them, on that user's machine. Anything that
  can read that file can already read the user's own files.
- **Gatekeeper and SmartScreen warnings.** The builds are intentionally not
  notarized and are not signed by a trusted authority. That is documented, and it
  is why macOS offers an "Open Anyway" step and a terminal installer.
- **Attacks needing administrator, root, or physical access**, or another
  already-compromised process on the same machine running as your user.
- **The count of a key being visible in the database while Secure Keyboard Entry
  is on**, or similar cases where the operating system denies input to every
  application: the README lists these under Known limitations.
- **Vulnerabilities in dependencies.** Report those upstream. If a dependency
  advisory actually affects Jokbet, tell us as well and it will be updated.

## Trust boundaries

Being explicit about what Jokbet trusts, so that reports and expectations line
up:

- **Binaries are signed.** Releases are signed with a minisign key whose public
  half is compiled into the app (`src-tauri/tauri.conf.json`). The private half
  lives in the repository's GitHub Actions secrets. Both the mirror and GitHub
  serve the same bytes, and the signature covers the file rather than its
  address, so either host verifies.
- **The installer trusts the site.** `install.sh` takes the file name and its
  SHA-256 from `/api/latest.json` and downloads it over HTTPS from the same
  origin, which is checked before installing. It therefore trusts
  `jokbet.jokerben.top` to describe an honest release; it does not carry the
  minisign signature that the in-app updater checks.
- **The website is a courier.** The Worker holds no state beyond a minute of
  cache and hands out only names that Tauri produces, from a private bucket.

## Dependencies

Dependency updates are welcome as pull requests. `cargo deny`-style auditing is
not set up yet; `npm audit` and `cargo audit` are run by hand now and then.
