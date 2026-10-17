-- The mirror's earlier history, recovered from R2 Operations Analytics on
-- 2026-10-17 UTC. This is the view the site shows: one complete object read is
-- one download; the historical client and country were not in the R2 metrics.
--
-- Query: the last 31 days of successful GetObject operations in
-- jokbet-downloads, grouped by object name and time. Keep only release assets
-- whose responseBytes equalled responseObjectSize; this leaves whole-object
-- reads and excludes the partial Range requests. Manifest reads (manifests/*,
-- latest.json, changelog.json) are not releases. One matching read is already
-- in `downloads` from the live counter, so it is subtracted here rather than
-- counted twice. The R2 table cannot distinguish a Range that covered the
-- whole file, nor prove the client wrote the bytes to disk.
--
-- Cloudflare exposes at most 31 days of R2 metrics. The earliest event this
-- query could see was 2026-09-24. Counts are kept by UTC day, release tag and
-- filename; the Worker merges them into the same mirror totals and assigns
-- client = 'unknown' when it reads this table. There is no country to infer.
CREATE TABLE download_history (
  day       TEXT NOT NULL,
  tag       TEXT NOT NULL,
  name      TEXT NOT NULL,
  downloads INTEGER NOT NULL CHECK (downloads > 0),
  PRIMARY KEY (day, tag, name)
);

INSERT INTO download_history (day, tag, name, downloads) VALUES
  ('2026-09-24', 'v0.1.0', 'Jokbet_0.1.0_aarch64.dmg', 20),
  ('2026-09-24', 'v0.1.0', 'Jokbet_0.1.0_amd64.AppImage', 3),
  ('2026-09-24', 'v0.1.0', 'Jokbet_0.1.0_amd64.deb', 6),
  ('2026-09-24', 'v0.1.0', 'Jokbet_0.1.0_x64-setup.exe', 6),
  ('2026-09-24', 'v0.1.0', 'Jokbet_0.1.0_x64.dmg', 2),
  ('2026-09-24', 'v0.1.0', 'Jokbet_0.1.0_x64_en-US.msi', 3),
  ('2026-09-24', 'v0.1.0', 'Jokbet_aarch64.app.tar.gz', 3),
  ('2026-09-24', 'v0.1.0', 'Jokbet_x64.app.tar.gz', 2),
  ('2026-09-24', 'v0.1.1', 'Jokbet_0.1.1_aarch64.dmg', 7),
  ('2026-09-24', 'v0.1.1', 'Jokbet_0.1.1_amd64.AppImage', 1),
  ('2026-09-24', 'v0.1.1', 'Jokbet_0.1.1_amd64.deb', 1),
  ('2026-09-24', 'v0.1.1', 'Jokbet_0.1.1_x64-setup.exe', 1),
  ('2026-09-24', 'v0.1.1', 'Jokbet_0.1.1_x64_en-US.msi', 1),
  ('2026-09-24', 'v0.1.1', 'Jokbet_aarch64.app.tar.gz', 1),
  ('2026-09-24', 'v0.1.1', 'Jokbet_x64.app.tar.gz', 1),
  ('2026-09-24', 'v0.1.2', 'Jokbet_0.1.2_aarch64.app.zip', 1),
  ('2026-09-24', 'v0.1.2', 'Jokbet_0.1.2_aarch64.dmg', 3),
  ('2026-09-24', 'v0.1.2', 'Jokbet_0.1.2_amd64.AppImage', 2),
  ('2026-09-24', 'v0.1.2', 'Jokbet_0.1.2_amd64.AppImage.sig', 1),
  ('2026-09-24', 'v0.1.2', 'Jokbet_0.1.2_amd64.deb', 2),
  ('2026-09-24', 'v0.1.2', 'Jokbet_0.1.2_amd64.deb.sig', 1),
  ('2026-09-24', 'v0.1.2', 'Jokbet_0.1.2_x64-setup.exe', 2),
  ('2026-09-24', 'v0.1.2', 'Jokbet_0.1.2_x64-setup.exe.sig', 1),
  ('2026-09-24', 'v0.1.2', 'Jokbet_0.1.2_x64.app.zip', 1),
  ('2026-09-24', 'v0.1.2', 'Jokbet_0.1.2_x64.dmg', 1),
  ('2026-09-24', 'v0.1.2', 'Jokbet_0.1.2_x64_en-US.msi', 2),
  ('2026-09-24', 'v0.1.2', 'Jokbet_0.1.2_x64_en-US.msi.sig', 1),
  ('2026-09-24', 'v0.1.2', 'Jokbet_aarch64.app.tar.gz', 1),
  ('2026-09-24', 'v0.1.2', 'Jokbet_aarch64.app.tar.gz.sig', 1),
  ('2026-09-24', 'v0.1.2', 'Jokbet_x64.app.tar.gz', 1),
  ('2026-09-24', 'v0.2.0', 'Jokbet_0.2.0_aarch64.dmg', 1),
  ('2026-09-25', 'v0.2.0', 'Jokbet_0.2.0_x64-setup.exe', 7),
  ('2026-09-25', 'v0.3.0', 'Jokbet_0.3.0_aarch64.dmg', 1),
  ('2026-09-25', 'v0.3.0', 'Jokbet_aarch64.app.tar.gz', 4),
  ('2026-09-25', 'v0.3.1', 'Jokbet_0.3.1_aarch64.dmg', 1),
  ('2026-09-26', 'v0.3.1', 'Jokbet_0.3.1_x64-setup.exe', 1),
  ('2026-09-26', 'v0.3.2', 'Jokbet_0.3.2_x64-setup.exe', 3),
  ('2026-09-27', 'v0.3.2', 'Jokbet_0.3.2_x64-setup.exe', 3),
  ('2026-09-27', 'v0.3.2', 'Jokbet_aarch64.app.tar.gz', 1),
  ('2026-09-28', 'v0.3.3', 'Jokbet_aarch64.app.tar.gz', 1),
  ('2026-09-29', 'v0.3.3', 'Jokbet_0.3.3_x64-setup.exe', 2),
  ('2026-09-29', 'v0.4.0', 'Jokbet_aarch64.app.tar.gz', 1),
  ('2026-10-01', 'v0.4.1', 'Jokbet_aarch64.app.tar.gz', 1),
  ('2026-10-02', 'v0.4.2', 'Jokbet_0.4.2_x64-setup.exe', 1),
  ('2026-10-02', 'v0.4.2', 'Jokbet_aarch64.app.tar.gz', 2),
  ('2026-10-04', 'v0.4.2', 'Jokbet_0.4.2_x64-setup.exe', 1),
  ('2026-10-04', 'v0.4.3', 'Jokbet_0.4.3_x64-setup.exe', 1),
  ('2026-10-04', 'v0.4.3', 'Jokbet_aarch64.app.tar.gz', 1),
  ('2026-10-05', 'v0.4.4', 'Jokbet_0.4.4_x64-setup.exe', 1),
  ('2026-10-07', 'v0.4.4', 'Jokbet_aarch64.app.tar.gz', 1),
  ('2026-10-08', 'v0.4.5', 'Jokbet_aarch64.app.tar.gz', 1),
  ('2026-10-12', 'v0.4.6', 'Jokbet_aarch64.app.tar.gz', 1),
  ('2026-10-14', 'v0.4.7', 'Jokbet_0.4.7_aarch64.dmg', 1),
  ('2026-10-14', 'v0.4.7', 'Jokbet_0.4.7_x64-setup.exe', 1),
  ('2026-10-14', 'v0.4.7', 'Jokbet_aarch64.app.tar.gz', 1),
  ('2026-10-15', 'v0.4.8', 'Jokbet_aarch64.app.tar.gz', 2);