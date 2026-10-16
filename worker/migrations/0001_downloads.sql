-- One row per download the Worker served from the bucket, written off the
-- response path so a database that is down can never break a download. This is
-- the mirror's half of the site's download total; worker/stats.ts writes it and
-- /api/downloads.json reads it back.
--
-- No AUTOINCREMENT, since nothing here is ever reused and it would cost a
-- sqlite_sequence write per download. No index either: every read is one full
-- scan by design (one aggregate query, five dimensions out of it), so an index
-- would only make the write more expensive.
--
-- Nothing here identifies anyone: no address, no cookie, no user agent text —
-- only the coarse class of client it was.
CREATE TABLE downloads (
  id      INTEGER PRIMARY KEY,
  -- Epoch milliseconds, UTC.
  at      INTEGER NOT NULL,
  -- The UTC date of `at`, YYYY-MM-DD, and always equal to
  -- date(at / 1000, 'unixepoch') — the pages count days in UTC, so it is never
  -- shifted at write time; a Beijing day is date(at / 1000, 'unixepoch', '+8 hours')
  -- at read time, for a hand-written query.
  day     TEXT NOT NULL,
  -- 'v0.4.8': the release the file belongs to, as /dl/ names it.
  tag     TEXT NOT NULL,
  -- 'Jokbet_0.4.8_aarch64.dmg': the file, named exactly as it is on GitHub.
  name    TEXT NOT NULL,
  -- 'browser' | 'script' | 'updater' | 'other', from the user agent.
  client  TEXT NOT NULL,
  -- ISO 3166-1 alpha-2, as Cloudflare saw it; null when it could not tell.
  country TEXT
);
