//! SQLite storage: one row of totals per day plus per-key counts per day.
//! Writes are deltas added in one transaction, so a crash loses at most one
//! flush interval and never double counts.

use crate::engine::aggregator::{DayCounters, Totals};
use crate::engine::milestones::{Reached, LIFETIME};
use chrono::NaiveDate;
use rusqlite::{params, Connection, OptionalExtension};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

const DATE_FMT: &str = "%Y-%m-%d";

const SCHEMA_V1: &str = "
CREATE TABLE daily(
  date TEXT PRIMARY KEY,
  keys INTEGER NOT NULL DEFAULT 0,
  click_left INTEGER NOT NULL DEFAULT 0,
  click_right INTEGER NOT NULL DEFAULT 0,
  click_middle INTEGER NOT NULL DEFAULT 0,
  scrolls INTEGER NOT NULL DEFAULT 0,
  move_px REAL NOT NULL DEFAULT 0,
  move_mm REAL NOT NULL DEFAULT 0
) WITHOUT ROWID;
CREATE TABLE daily_keys(
  date TEXT NOT NULL,
  key TEXT NOT NULL,
  count INTEGER NOT NULL,
  PRIMARY KEY(date, key)
) WITHOUT ROWID;
CREATE TABLE milestone_state(
  id TEXT NOT NULL,
  period TEXT NOT NULL,
  last_value REAL NOT NULL,
  fired_at INTEGER NOT NULL,
  PRIMARY KEY(id, period)
) WITHOUT ROWID;
";

/// The same totals again, split by the hour of the day they happened in, so a
/// day can say when it was busy. Days counted before this version have no rows
/// here, which reads as an hour with nothing in it.
const SCHEMA_V2: &str = "
CREATE TABLE hourly(
  date TEXT NOT NULL,
  hour INTEGER NOT NULL,
  keys INTEGER NOT NULL DEFAULT 0,
  click_left INTEGER NOT NULL DEFAULT 0,
  click_right INTEGER NOT NULL DEFAULT 0,
  click_middle INTEGER NOT NULL DEFAULT 0,
  scrolls INTEGER NOT NULL DEFAULT 0,
  move_px REAL NOT NULL DEFAULT 0,
  move_mm REAL NOT NULL DEFAULT 0,
  PRIMARY KEY(date, hour)
) WITHOUT ROWID;
";

/// Version 3 counts scrolling in lines, in a `scroll_lines` column on both of
/// these. The `scrolls` columns keep the gestures counted before; they are no
/// longer written, and a day from back then reads as one with nothing scrolled.
///
/// A build from before 0.4.2 shipped took a database to version 3 with the
/// scrolling in millimetres instead, as `scroll_mm`. What it left, and the
/// backups it made, are at version 3 without `scroll_lines`, so the column is
/// looked for on every open rather than trusted to come with the number.
const SCROLL_LINES_TABLES: [&str; 2] = ["daily", "hourly"];

/// The schema `init` brings a database up to. A backup made by a newer version
/// may be ahead of it, and is refused rather than read wrong.
pub const SCHEMA_VERSION: i64 = 3;

/// The tables a restore copies over, with their columns in order. Every count
/// the app keeps is in one of these; settings live in their own file.
const TABLES: [(&str, &str); 4] = [
    (
        "daily",
        "date, keys, click_left, click_right, click_middle, scrolls, scroll_lines, move_px, move_mm",
    ),
    ("daily_keys", "date, key, count"),
    ("milestone_state", "id, period, last_value, fired_at"),
    (
        "hourly",
        "date, hour, keys, click_left, click_right, click_middle, scrolls, scroll_lines, move_px, move_mm",
    ),
];

pub struct Db {
    conn: Connection,
}

/// How far back the insights look, matching the stats window's own clamp.
const HISTORY_DAYS: i64 = 3660;

fn day_key(date: NaiveDate) -> String {
    date.format(DATE_FMT).to_string()
}

fn has_column(conn: &Connection, table: &str, column: &str) -> rusqlite::Result<bool> {
    conn.query_row(
        "SELECT COUNT(*) FROM pragma_table_info(?1) WHERE name = ?2",
        [table, column],
        |r| r.get::<_, i64>(0),
    )
    .map(|n| n > 0)
}

fn totals_from_row(row: &rusqlite::Row, first: usize) -> rusqlite::Result<Totals> {
    Ok(Totals {
        keys: row.get::<_, i64>(first)? as u64,
        click_left: row.get::<_, i64>(first + 1)? as u64,
        click_right: row.get::<_, i64>(first + 2)? as u64,
        click_middle: row.get::<_, i64>(first + 3)? as u64,
        scroll_lines: row.get(first + 4)?,
        move_px: row.get(first + 5)?,
        move_mm: row.get(first + 6)?,
    })
}

impl Db {
    pub fn open(path: &Path) -> rusqlite::Result<Self> {
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        Self::init(Connection::open(path)?)
    }

    /// Opens the database, and when it will not open, the way the settings
    /// treat a damaged file: the old one is set aside as
    /// `stats.bad-<time>.sqlite` — with its sidecars, so a fresh pair does
    /// not adopt them — and a fresh one is tried. `None` when even that will
    /// not open, and the engine goes on counting in memory; a damaged file
    /// should not quietly turn into an app that keeps no counts at all.
    pub fn open_salvaging(path: &Path) -> Option<Db> {
        match Db::open(path) {
            Ok(db) => Some(db),
            Err(e) => {
                log::error!("opening {} failed: {e}", path.display());
                let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
                let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("stats");
                let aside = path.with_file_name(format!("{stem}.bad-{stamp}.sqlite"));
                match std::fs::rename(path, &aside) {
                    Ok(()) => {
                        // The sidecars belong to the file just set aside; a
                        // leftover pair would be adopted by the fresh one as
                        // its own, and what holds committed-but-unmerged data
                        // goes with the file it belongs to.
                        for suffix in ["-wal", "-shm"] {
                            let from = path.with_file_name(format!("{stem}.sqlite{suffix}"));
                            let to = PathBuf::from(format!("{}{}", aside.display(), suffix));
                            if std::fs::rename(&from, &to).is_err() {
                                let _ = std::fs::remove_file(&from);
                            }
                        }
                    }
                    // The rename failing usually means another process has
                    // the file open; starting a second database beside it
                    // would split the truth, so carry on without storage.
                    Err(e) => {
                        log::error!("setting {} aside failed: {e}", path.display());
                        return None;
                    }
                }
                log::warn!("set the unreadable database aside as {}", aside.display());
                match Db::open(path) {
                    Ok(db) => Some(db),
                    Err(e) => {
                        log::error!(
                            "opening {} after setting the old one aside failed: {e}",
                            path.display()
                        );
                        None
                    }
                }
            }
        }
    }

    #[cfg(test)]
    pub fn open_in_memory() -> rusqlite::Result<Self> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(mut conn: Connection) -> rusqlite::Result<Self> {
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.busy_timeout(std::time::Duration::from_secs(2))?;
        let version: i64 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
        // Each step is one transaction: a crash between its tables and its
        // version number would otherwise fail every start after it.
        if version < 1 {
            let tx = conn.transaction()?;
            tx.execute_batch(SCHEMA_V1)?;
            tx.pragma_update(None, "user_version", 1)?;
            tx.commit()?;
        }
        if version < 2 {
            let tx = conn.transaction()?;
            tx.execute_batch(SCHEMA_V2)?;
            tx.pragma_update(None, "user_version", 2)?;
            tx.commit()?;
        }
        let mut missing = Vec::new();
        for table in SCROLL_LINES_TABLES {
            if !has_column(&conn, table, "scroll_lines")? {
                missing.push(table);
            }
        }
        if version < 3 || !missing.is_empty() {
            let tx = conn.transaction()?;
            for table in missing {
                tx.execute(
                    &format!("ALTER TABLE {table} ADD COLUMN scroll_lines REAL NOT NULL DEFAULT 0"),
                    [],
                )?;
            }
            if version < 3 {
                tx.pragma_update(None, "user_version", 3)?;
            }
            tx.commit()?;
        }
        Ok(Self { conn })
    }

    /// The schema version the file is at.
    pub fn schema_version(&self) -> rusqlite::Result<i64> {
        self.conn
            .pragma_query_value(None, "user_version", |r| r.get(0))
    }

    /// How many days have counts.
    pub fn day_count(&self) -> rusqlite::Result<u64> {
        self.conn
            .query_row("SELECT COUNT(*) FROM daily", [], |r| r.get::<_, i64>(0))
            .map(|n| n as u64)
    }

    /// Whether SQLite finds the file sound.
    pub fn integrity_ok(&self) -> rusqlite::Result<bool> {
        let result: String = self
            .conn
            .query_row("PRAGMA integrity_check", [], |r| r.get(0))?;
        Ok(result == "ok")
    }

    /// Writes a compact, self-contained copy of the database to `to`: one file,
    /// with nothing left in a write-ahead log beside it. Anything already at
    /// `to` is replaced.
    pub fn snapshot_to(&self, to: &Path) -> rusqlite::Result<()> {
        if let Some(dir) = to.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        // VACUUM INTO will not write over a file.
        let _ = std::fs::remove_file(to);
        self.conn
            .execute("VACUUM INTO ?1", [to.to_string_lossy()])?;
        Ok(())
    }

    /// Replaces every count with the ones in the database at `src`, which must
    /// already be at this schema. All in one transaction: a failure part way
    /// leaves the counts as they were.
    pub fn replace_from(&mut self, src: &Path) -> rusqlite::Result<()> {
        self.conn
            .execute("ATTACH DATABASE ?1 AS src", [src.to_string_lossy()])?;
        let copied = self.copy_attached();
        let detached = self.conn.execute("DETACH DATABASE src", []);
        copied?;
        detached.map(|_| ())
    }

    fn copy_attached(&mut self) -> rusqlite::Result<()> {
        let tx = self.conn.transaction()?;
        for (table, cols) in TABLES {
            tx.execute(&format!("DELETE FROM main.{table}"), [])?;
            tx.execute(
                &format!("INSERT INTO main.{table}({cols}) SELECT {cols} FROM src.{table}"),
                [],
            )?;
        }
        tx.commit()
    }

    /// Adds a day's delta to what is stored.
    pub fn add_day(&mut self, date: NaiveDate, delta: &DayCounters) -> rusqlite::Result<()> {
        let d = day_key(date);
        let t = &delta.totals;
        let tx = self.conn.transaction()?;
        tx.execute(
            "INSERT INTO daily(date, keys, click_left, click_right, click_middle, scroll_lines, move_px, move_mm)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(date) DO UPDATE SET
               keys = keys + excluded.keys,
               click_left = click_left + excluded.click_left,
               click_right = click_right + excluded.click_right,
               click_middle = click_middle + excluded.click_middle,
               scroll_lines = scroll_lines + excluded.scroll_lines,
               move_px = move_px + excluded.move_px,
               move_mm = move_mm + excluded.move_mm",
            params![
                d,
                t.keys as i64,
                t.click_left as i64,
                t.click_right as i64,
                t.click_middle as i64,
                t.scroll_lines,
                t.move_px,
                t.move_mm
            ],
        )?;
        {
            let mut stmt = tx.prepare_cached(
                "INSERT INTO daily_keys(date, key, count) VALUES (?1, ?2, ?3)
                 ON CONFLICT(date, key) DO UPDATE SET count = count + excluded.count",
            )?;
            for (key, n) in &delta.per_key {
                stmt.execute(params![d, key, *n as i64])?;
            }
        }
        {
            // Its own scope, like the one above: the commit cannot move a
            // transaction a statement is still borrowing.
            let mut stmt = tx.prepare_cached(
                "INSERT INTO hourly(date, hour, keys, click_left, click_right, click_middle, scroll_lines, move_px, move_mm)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                 ON CONFLICT(date, hour) DO UPDATE SET
                   keys = keys + excluded.keys,
                   click_left = click_left + excluded.click_left,
                   click_right = click_right + excluded.click_right,
                   click_middle = click_middle + excluded.click_middle,
                   scroll_lines = scroll_lines + excluded.scroll_lines,
                   move_px = move_px + excluded.move_px,
                   move_mm = move_mm + excluded.move_mm",
            )?;
            for (hour, t) in delta.per_hour.iter().enumerate() {
                if *t == Totals::default() {
                    continue;
                }
                stmt.execute(params![
                    d,
                    hour as i64,
                    t.keys as i64,
                    t.click_left as i64,
                    t.click_right as i64,
                    t.click_middle as i64,
                    t.scroll_lines,
                    t.move_px,
                    t.move_mm
                ])?;
            }
        }
        tx.commit()
    }

    pub fn load_day(&self, date: NaiveDate) -> rusqlite::Result<DayCounters> {
        let d = day_key(date);
        let totals = self
            .conn
            .query_row(
                "SELECT keys, click_left, click_right, click_middle, scroll_lines, move_px, move_mm
                 FROM daily WHERE date = ?1",
                [&d],
                |row| totals_from_row(row, 0),
            )
            .optional()?
            .unwrap_or_default();
        let mut stmt = self
            .conn
            .prepare("SELECT key, count FROM daily_keys WHERE date = ?1")?;
        let per_key = stmt
            .query_map([&d], |row| Ok((row.get(0)?, row.get::<_, i64>(1)? as u64)))?
            .collect::<rusqlite::Result<HashMap<String, u64>>>()?;
        // Today's hours come back too, so `today` stays what it says it is.
        let mut per_hour: [Totals; 24] = std::array::from_fn(|_| Totals::default());
        let mut hours = self.conn.prepare(
            "SELECT hour, keys, click_left, click_right, click_middle, scroll_lines, move_px, move_mm
             FROM hourly WHERE date = ?1",
        )?;
        let mut rows = hours.query([&d])?;
        while let Some(row) = rows.next()? {
            let hour: i64 = row.get(0)?;
            if let Some(slot) = per_hour.get_mut(hour as usize) {
                *slot = totals_from_row(row, 1)?;
            }
        }
        Ok(DayCounters {
            totals,
            per_key,
            per_hour,
        })
    }

    /// One entry per day from `from` to `to` inclusive; days without data are zero.
    pub fn range(
        &self,
        from: NaiveDate,
        to: NaiveDate,
    ) -> rusqlite::Result<Vec<(NaiveDate, Totals)>> {
        let mut stmt = self.conn.prepare(
            "SELECT date, keys, click_left, click_right, click_middle, scroll_lines, move_px, move_mm
             FROM daily WHERE date BETWEEN ?1 AND ?2",
        )?;
        let stored = stmt
            .query_map([day_key(from), day_key(to)], |row| {
                Ok((row.get::<_, String>(0)?, totals_from_row(row, 1)?))
            })?
            .collect::<rusqlite::Result<HashMap<String, Totals>>>()?;
        Ok(from
            .iter_days()
            .take_while(|d| *d <= to)
            .map(|d| (d, stored.get(&day_key(d)).cloned().unwrap_or_default()))
            .collect())
    }

    /// Per-key counts summed over a date range.
    pub fn key_counts(
        &self,
        from: NaiveDate,
        to: NaiveDate,
    ) -> rusqlite::Result<HashMap<String, u64>> {
        let mut stmt = self.conn.prepare(
            "SELECT key, SUM(count) FROM daily_keys WHERE date BETWEEN ?1 AND ?2 GROUP BY key",
        )?;
        let rows = stmt.query_map([day_key(from), day_key(to)], |row| {
            Ok((row.get(0)?, row.get::<_, i64>(1)? as u64))
        })?;
        rows.collect()
    }

    /// Totals per hour of the day, summed over a date range; index 0 is
    /// midnight and hours without counts are zero. Days counted before the
    /// hourly table existed contribute nothing.
    pub fn hours(&self, from: NaiveDate, to: NaiveDate) -> rusqlite::Result<[Totals; 24]> {
        let mut out: [Totals; 24] = std::array::from_fn(|_| Totals::default());
        let mut stmt = self.conn.prepare(
            "SELECT hour, SUM(keys), SUM(click_left), SUM(click_right), SUM(click_middle),
                    SUM(scroll_lines), SUM(move_px), SUM(move_mm)
             FROM hourly WHERE date BETWEEN ?1 AND ?2 GROUP BY hour",
        )?;
        let mut rows = stmt.query([day_key(from), day_key(to)])?;
        while let Some(row) = rows.next()? {
            let hour: i64 = row.get(0)?;
            if let Some(slot) = out.get_mut(hour as usize) {
                *slot = totals_from_row(row, 1)?;
            }
        }
        Ok(out)
    }

    /// Every day that has counts, oldest first. Days with nothing in them are
    /// never written, so this is far smaller than the span it covers.
    pub fn history(&self) -> rusqlite::Result<Vec<(String, Totals)>> {
        let mut stmt = self.conn.prepare(
            "SELECT date, keys, click_left, click_right, click_middle, scroll_lines, move_px, move_mm
             FROM daily ORDER BY date DESC LIMIT ?1",
        )?;
        let mut days = stmt
            .query_map([HISTORY_DAYS], |row| {
                Ok((row.get::<_, String>(0)?, totals_from_row(row, 1)?))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        days.reverse();
        Ok(days)
    }

    /// Every key ever pressed, whatever the day (what the heatmap draws its
    /// board from).
    pub fn ever_pressed(&self) -> rusqlite::Result<Vec<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT DISTINCT key FROM daily_keys ORDER BY key")?;
        let rows = stmt.query_map([], |row| row.get(0))?;
        rows.collect()
    }

    /// All days as CSV: date, totals.
    pub fn daily_csv(&self) -> rusqlite::Result<String> {
        let mut out = String::from(
            "date,keys,click_left,click_right,click_middle,scroll_lines,move_px,move_m\n",
        );
        let mut stmt = self.conn.prepare(
            "SELECT date, keys, click_left, click_right, click_middle, scroll_lines, move_px, move_mm
             FROM daily ORDER BY date",
        )?;
        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            let d: String = row.get(0)?;
            let t = totals_from_row(row, 1)?;
            out.push_str(&format!(
                "{d},{},{},{},{},{:.0},{:.0},{:.2}\n",
                t.keys,
                t.click_left,
                t.click_right,
                t.click_middle,
                t.scroll_lines,
                t.move_px,
                t.move_mm / 1000.0
            ));
        }
        Ok(out)
    }

    /// All per-key counts as CSV: date, key, count.
    pub fn keys_csv(&self) -> rusqlite::Result<String> {
        let mut out = String::from("date,key,count\n");
        let mut stmt = self
            .conn
            .prepare("SELECT date, key, count FROM daily_keys ORDER BY date, key")?;
        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            let (d, k, n): (String, String, i64) = (row.get(0)?, row.get(1)?, row.get(2)?);
            out.push_str(&format!("{d},{k},{n}\n"));
        }
        Ok(out)
    }

    /// All days as CSV: date, hour, totals.
    pub fn hourly_csv(&self) -> rusqlite::Result<String> {
        let mut out = String::from(
            "date,hour,keys,click_left,click_right,click_middle,scroll_lines,move_px,move_m\n",
        );
        let mut stmt = self.conn.prepare(
            "SELECT date, hour, keys, click_left, click_right, click_middle, scroll_lines, move_px, move_mm
             FROM hourly ORDER BY date, hour",
        )?;
        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            let (d, h): (String, i64) = (row.get(0)?, row.get(1)?);
            let t = totals_from_row(row, 2)?;
            out.push_str(&format!(
                "{d},{h},{},{},{},{},{:.0},{:.0},{:.2}\n",
                t.keys,
                t.click_left,
                t.click_right,
                t.click_middle,
                t.scroll_lines,
                t.move_px,
                t.move_mm / 1000.0
            ));
        }
        Ok(out)
    }

    /// Deletes every count (settings are kept).
    pub fn clear(&mut self) -> rusqlite::Result<()> {
        self.conn.execute_batch(
            "DELETE FROM daily; DELETE FROM daily_keys; DELETE FROM hourly; DELETE FROM milestone_state;",
        )
    }

    /// Milestones already celebrated (lifetime ones, and daily ones for `today`).
    pub fn load_reached(&self, today: NaiveDate) -> rusqlite::Result<Reached> {
        let mut stmt = self.conn.prepare(
            "SELECT id, period, last_value FROM milestone_state WHERE period IN (?1, ?2)",
        )?;
        let rows = stmt.query_map([LIFETIME.to_string(), day_key(today)], |row| {
            Ok((
                row.get::<_, String>(0)?,
                (row.get::<_, String>(1)?, row.get::<_, f64>(2)?),
            ))
        })?;
        Ok(Reached(rows.collect::<rusqlite::Result<_>>()?))
    }

    pub fn save_reached(
        &mut self,
        id: &str,
        period: &str,
        level: f64,
        fired_at: i64,
    ) -> rusqlite::Result<()> {
        self.conn.execute(
            "INSERT INTO milestone_state(id, period, last_value, fired_at) VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(id, period) DO UPDATE SET last_value = excluded.last_value, fired_at = excluded.fired_at",
            params![id, period, level, fired_at],
        )?;
        Ok(())
    }

    /// Drops daily milestone rows from before `today`.
    pub fn prune_reached(&mut self, today: NaiveDate) -> rusqlite::Result<()> {
        self.conn.execute(
            "DELETE FROM milestone_state WHERE period <> ?1 AND period < ?2",
            [LIFETIME.to_string(), day_key(today)],
        )?;
        Ok(())
    }

    pub fn lifetime_totals(&self) -> rusqlite::Result<Totals> {
        self.conn.query_row(
            "SELECT COALESCE(SUM(keys), 0), COALESCE(SUM(click_left), 0), COALESCE(SUM(click_right), 0),
                    COALESCE(SUM(click_middle), 0), COALESCE(SUM(scroll_lines), 0.0),
                    COALESCE(SUM(move_px), 0.0), COALESCE(SUM(move_mm), 0.0)
             FROM daily",
            [],
            |row| totals_from_row(row, 0),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn day(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    fn delta(keys: u64, per_key: &[(&str, u64)]) -> DayCounters {
        DayCounters {
            totals: Totals {
                keys,
                click_left: 1,
                move_mm: 2.5,
                ..Default::default()
            },
            per_key: per_key.iter().map(|(k, n)| (k.to_string(), *n)).collect(),
            ..Default::default()
        }
    }

    /// A day that pressed `n` keys in each of the given hours.
    fn hourly(hours: &[(usize, u64)]) -> DayCounters {
        let mut d = DayCounters::default();
        for (hour, n) in hours {
            d.totals.keys += n;
            d.per_hour[*hour].keys += n;
        }
        d
    }

    #[test]
    fn deltas_add_up() {
        let mut db = Db::open_in_memory().unwrap();
        let d = day(2026, 9, 23);
        db.add_day(d, &delta(3, &[("KeyA", 2), ("KeyB", 1)]))
            .unwrap();
        db.add_day(d, &delta(2, &[("KeyA", 2)])).unwrap();
        let loaded = db.load_day(d).unwrap();
        assert_eq!(loaded.totals.keys, 5);
        assert_eq!(loaded.totals.click_left, 2);
        assert_eq!(loaded.totals.move_mm, 5.0);
        assert_eq!(loaded.per_key["KeyA"], 4);
        assert_eq!(loaded.per_key["KeyB"], 1);
    }

    #[test]
    fn missing_day_is_empty() {
        let db = Db::open_in_memory().unwrap();
        assert!(db.load_day(day(2026, 1, 1)).unwrap().is_empty());
    }

    #[test]
    fn lifetime_sums_all_days() {
        let mut db = Db::open_in_memory().unwrap();
        assert_eq!(db.lifetime_totals().unwrap(), Totals::default());
        db.add_day(day(2026, 9, 22), &delta(10, &[])).unwrap();
        db.add_day(day(2026, 9, 23), &delta(5, &[])).unwrap();
        let life = db.lifetime_totals().unwrap();
        assert_eq!(life.keys, 15);
        assert_eq!(life.click_left, 2);
    }

    #[test]
    fn range_fills_missing_days_with_zero() {
        let mut db = Db::open_in_memory().unwrap();
        db.add_day(day(2026, 9, 21), &delta(4, &[])).unwrap();
        db.add_day(day(2026, 9, 23), &delta(6, &[])).unwrap();
        let r = db.range(day(2026, 9, 20), day(2026, 9, 23)).unwrap();
        let keys: Vec<_> = r
            .iter()
            .map(|(d, t)| (d.format("%d").to_string(), t.keys))
            .collect();
        assert_eq!(
            keys,
            [
                ("20".into(), 0),
                ("21".into(), 4),
                ("22".into(), 0),
                ("23".into(), 6)
            ]
        );
    }

    #[test]
    fn key_counts_sum_over_the_range_only() {
        let mut db = Db::open_in_memory().unwrap();
        db.add_day(day(2026, 9, 1), &delta(1, &[("KeyA", 5)]))
            .unwrap();
        db.add_day(day(2026, 9, 22), &delta(1, &[("KeyA", 2), ("Space", 1)]))
            .unwrap();
        db.add_day(day(2026, 9, 23), &delta(1, &[("KeyA", 3)]))
            .unwrap();
        let k = db.key_counts(day(2026, 9, 22), day(2026, 9, 23)).unwrap();
        assert_eq!(k["KeyA"], 5);
        assert_eq!(k["Space"], 1);
        assert_eq!(k.len(), 2);
    }

    #[test]
    fn ever_pressed_spans_every_day() {
        let mut db = Db::open_in_memory().unwrap();
        db.add_day(day(2025, 1, 1), &delta(1, &[("IntlBackslash", 1)]))
            .unwrap();
        db.add_day(day(2026, 9, 23), &delta(2, &[("KeyA", 2)]))
            .unwrap();
        assert_eq!(db.ever_pressed().unwrap(), ["IntlBackslash", "KeyA"]);
    }

    #[test]
    fn csv_exports_and_clear() {
        let mut db = Db::open_in_memory().unwrap();
        db.add_day(day(2026, 9, 23), &delta(3, &[("KeyA", 3)]))
            .unwrap();
        assert_eq!(
            db.daily_csv().unwrap(),
            "date,keys,click_left,click_right,click_middle,scroll_lines,move_px,move_m\n2026-09-23,3,1,0,0,0,0,0.00\n"
        );
        assert_eq!(
            db.keys_csv().unwrap(),
            "date,key,count\n2026-09-23,KeyA,3\n"
        );
        db.clear().unwrap();
        assert_eq!(db.lifetime_totals().unwrap(), Totals::default());
        assert_eq!(db.keys_csv().unwrap(), "date,key,count\n");
    }

    #[test]
    fn reached_milestones_round_trip_and_prune() {
        let mut db = Db::open_in_memory().unwrap();
        db.save_reached("life-100k", LIFETIME, 100_000.0, 1)
            .unwrap();
        db.save_reached("daily-1k", "2026-09-22", 1000.0, 1)
            .unwrap();
        db.save_reached("daily-5k", "2026-09-23", 5000.0, 2)
            .unwrap();
        let r = db.load_reached(day(2026, 9, 23)).unwrap();
        assert_eq!(r.0.len(), 2);
        assert_eq!(r.0["daily-5k"], ("2026-09-23".to_string(), 5000.0));
        db.prune_reached(day(2026, 9, 23)).unwrap();
        let count: i64 = db
            .conn
            .query_row("SELECT COUNT(*) FROM milestone_state", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 2);
    }

    #[test]
    fn reopening_keeps_data_and_schema() {
        let dir = std::env::temp_dir().join(format!("jdp-db-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("stats.sqlite");
        let d = day(2026, 9, 23);
        Db::open(&path).unwrap().add_day(d, &delta(7, &[])).unwrap();
        assert_eq!(Db::open(&path).unwrap().load_day(d).unwrap().totals.keys, 7);
    }

    #[test]
    fn a_database_that_will_not_open_is_set_aside() {
        let dir = std::env::temp_dir().join(format!("jdp-db-salvage-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("stats.sqlite");
        // Not a database: the open has to fail, not the salvage.
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(&path, b"this is not a database").unwrap();
        let d = day(2026, 9, 23);
        {
            let mut db =
                Db::open_salvaging(&path).expect("a fresh database goes where the bad one was");
            db.add_day(d, &delta(7, &[])).unwrap();
        } // The fresh connection closes before the file is read again below.
        assert_eq!(Db::open(&path).unwrap().load_day(d).unwrap().totals.keys, 7);
        // The old file is there to be read, not gone.
        let kept = std::fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .find(|e| e.file_name().to_string_lossy().contains(".bad-"))
            .expect("the damaged file is kept beside the fresh one");
        assert!(kept.file_name().to_string_lossy().ends_with(".sqlite"));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn hourly_deltas_add_up() {
        let mut db = Db::open_in_memory().unwrap();
        let d = day(2026, 9, 23);
        db.add_day(d, &hourly(&[(9, 3), (14, 2)])).unwrap();
        db.add_day(d, &hourly(&[(9, 4), (22, 1)])).unwrap();
        let loaded = db.load_day(d).unwrap();
        assert_eq!(loaded.totals.keys, 10);
        assert_eq!(loaded.per_hour[9].keys, 7);
        assert_eq!(loaded.per_hour[14].keys, 2);
        assert_eq!(loaded.per_hour[22].keys, 1);
        assert_eq!(loaded.per_hour[3], Totals::default());
    }

    #[test]
    fn hours_sum_over_the_range_only() {
        let mut db = Db::open_in_memory().unwrap();
        db.add_day(day(2026, 9, 1), &hourly(&[(9, 100)])).unwrap();
        db.add_day(day(2026, 9, 22), &hourly(&[(9, 5), (14, 2)]))
            .unwrap();
        db.add_day(day(2026, 9, 23), &hourly(&[(14, 3)])).unwrap();
        // A day counted before the hourly table existed has no hours of its own.
        db.add_day(day(2026, 9, 24), &delta(60, &[])).unwrap();
        let h = db.hours(day(2026, 9, 22), day(2026, 9, 23)).unwrap();
        assert_eq!(h[9].keys, 5);
        assert_eq!(h[14].keys, 5);
        assert_eq!(h.iter().map(|t| t.keys).sum::<u64>(), 10);
        // Widening the range picks the older day's hour back up, and still none
        // of the 60 keys that were never filed under an hour.
        let wide = db.hours(day(2026, 9, 1), day(2026, 9, 24)).unwrap();
        assert_eq!(wide[9].keys, 105);
        assert_eq!(wide[14].keys, 5);
        assert_eq!(wide.iter().map(|t| t.keys).sum::<u64>(), 110);
    }

    #[test]
    fn history_holds_only_the_days_with_counts() {
        let mut db = Db::open_in_memory().unwrap();
        for d in [day(2026, 9, 20), day(2026, 9, 21), day(2026, 9, 23)] {
            db.add_day(d, &delta(4, &[])).unwrap();
        }
        let history = db.history().unwrap();
        let seen: Vec<&str> = history.iter().map(|(d, _)| d.as_str()).collect();
        assert_eq!(seen, ["2026-09-20", "2026-09-21", "2026-09-23"]);
        assert_eq!(history.last().unwrap().1.keys, 4);
    }

    #[test]
    fn hourly_csv_exports_and_clear_drops_it() {
        let mut db = Db::open_in_memory().unwrap();
        db.add_day(day(2026, 9, 23), &hourly(&[(9, 3), (14, 5)]))
            .unwrap();
        assert_eq!(
            db.hourly_csv().unwrap(),
            "date,hour,keys,click_left,click_right,click_middle,scroll_lines,move_px,move_m\n\
             2026-09-23,9,3,0,0,0,0,0,0.00\n\
             2026-09-23,14,5,0,0,0,0,0,0.00\n"
        );
        db.clear().unwrap();
        assert_eq!(
            db.hourly_csv().unwrap(),
            "date,hour,keys,click_left,click_right,click_middle,scroll_lines,move_px,move_m\n"
        );
        assert!(db
            .hours(day(2026, 9, 1), day(2026, 9, 30))
            .unwrap()
            .iter()
            .all(|t| *t == Totals::default()));
    }

    #[test]
    fn upgrading_a_v1_database_keeps_the_days() {
        let dir = std::env::temp_dir().join(format!("jdp-db-v1-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("stats.sqlite");
        let d = day(2026, 9, 23);
        {
            // A database as the previous version left it: v1 tables, no hourly.
            let conn = Connection::open(&path).unwrap();
            conn.execute_batch(SCHEMA_V1).unwrap();
            conn.pragma_update(None, "user_version", 1).unwrap();
            conn.execute(
                "INSERT INTO daily(date, keys) VALUES (?1, ?2)",
                params![day_key(d), 42_i64],
            )
            .unwrap();
        }
        let db = Db::open(&path).unwrap();
        assert_eq!(db.load_day(d).unwrap().totals.keys, 42);
        assert!(db
            .hours(d, d)
            .unwrap()
            .iter()
            .all(|t| *t == Totals::default()));
        let version: i64 = db
            .conn
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .unwrap();
        assert_eq!(version, SCHEMA_VERSION);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn upgrading_a_v2_database_leaves_counted_scrolls_behind() {
        let dir = temp_dir("v2");
        let path = dir.join("stats.sqlite");
        let d = day(2026, 9, 23);
        {
            // Scrolls were counted as gestures then; there are no lines in them.
            let conn = Connection::open(&path).unwrap();
            conn.execute_batch(SCHEMA_V1).unwrap();
            conn.execute_batch(SCHEMA_V2).unwrap();
            conn.pragma_update(None, "user_version", 2).unwrap();
            conn.execute(
                "INSERT INTO daily(date, keys, scrolls) VALUES (?1, 42, 7)",
                [day_key(d)],
            )
            .unwrap();
        }
        let mut db = Db::open(&path).unwrap();
        assert_eq!(db.schema_version().unwrap(), SCHEMA_VERSION);
        let before = db.load_day(d).unwrap().totals;
        assert_eq!((before.keys, before.scroll_lines), (42, 0.0));
        let mut delta = DayCounters::default();
        delta.totals.scroll_lines = 1234.5;
        delta.per_hour[9].scroll_lines = 1234.5;
        db.add_day(d, &delta).unwrap();
        assert_eq!(db.load_day(d).unwrap().totals.scroll_lines, 1234.5);
        assert_eq!(db.hours(d, d).unwrap()[9].scroll_lines, 1234.5);
        assert_eq!(db.lifetime_totals().unwrap().scroll_lines, 1234.5);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// What the build that measured scrolling in millimetres left behind:
    /// version 3, with `scroll_mm` where `scroll_lines` should be.
    #[test]
    fn a_database_at_v3_without_scroll_lines_gets_them() {
        let dir = temp_dir("v3-mm");
        let path = dir.join("stats.sqlite");
        let d = day(2026, 10, 2);
        {
            let conn = Connection::open(&path).unwrap();
            conn.execute_batch(SCHEMA_V1).unwrap();
            conn.execute_batch(SCHEMA_V2).unwrap();
            conn.execute_batch(
                "ALTER TABLE daily ADD COLUMN scroll_mm REAL NOT NULL DEFAULT 0;
                 ALTER TABLE hourly ADD COLUMN scroll_mm REAL NOT NULL DEFAULT 0;",
            )
            .unwrap();
            conn.pragma_update(None, "user_version", 3).unwrap();
            conn.execute(
                "INSERT INTO daily(date, keys, scroll_mm) VALUES (?1, 945, 3689.5)",
                [day_key(d)],
            )
            .unwrap();
        }
        let mut db = Db::open(&path).unwrap();
        assert_eq!(db.schema_version().unwrap(), SCHEMA_VERSION);
        assert_eq!(db.load_day(d).unwrap().totals.keys, 945);
        let mut delta = DayCounters::default();
        delta.totals.scroll_lines = 12.0;
        delta.per_hour[9].scroll_lines = 12.0;
        db.add_day(d, &delta).unwrap();
        assert_eq!(db.range(d, d).unwrap()[0].1.scroll_lines, 12.0);
        assert_eq!(db.hours(d, d).unwrap()[9].scroll_lines, 12.0);
        drop(db);
        // Opening it again finds nothing more to do.
        assert_eq!(
            Db::open(&path)
                .unwrap()
                .load_day(d)
                .unwrap()
                .totals
                .scroll_lines,
            12.0
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("jdp-db-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn a_new_database_is_at_the_current_schema() {
        let db = Db::open_in_memory().unwrap();
        assert_eq!(db.schema_version().unwrap(), SCHEMA_VERSION);
        assert!(db.integrity_ok().unwrap());
    }

    #[test]
    fn a_snapshot_holds_every_day() {
        let dir = temp_dir("snapshot");
        let mut db = Db::open(&dir.join("stats.sqlite")).unwrap();
        db.add_day(day(2026, 9, 22), &delta(3, &[("KeyA", 3)]))
            .unwrap();
        db.add_day(day(2026, 9, 23), &hourly(&[(9, 4)])).unwrap();
        db.save_reached("life-100k", LIFETIME, 100_000.0, 1)
            .unwrap();
        let copy = dir.join("copy.sqlite");
        std::fs::write(&copy, b"in the way").unwrap();
        db.snapshot_to(&copy).unwrap();

        let snap = Db::open(&copy).unwrap();
        assert_eq!(snap.schema_version().unwrap(), SCHEMA_VERSION);
        assert_eq!(snap.day_count().unwrap(), 2);
        assert_eq!(snap.load_day(day(2026, 9, 22)).unwrap().per_key["KeyA"], 3);
        assert_eq!(
            snap.hours(day(2026, 9, 23), day(2026, 9, 23)).unwrap()[9].keys,
            4
        );
        assert_eq!(snap.load_reached(day(2026, 9, 23)).unwrap().0.len(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn replacing_takes_every_table_from_the_source() {
        let dir = temp_dir("replace");
        let src_path = dir.join("src.sqlite");
        {
            let mut src = Db::open(&src_path).unwrap();
            src.add_day(day(2026, 1, 1), &delta(9, &[("KeyZ", 9)]))
                .unwrap();
            src.add_day(day(2026, 1, 1), &hourly(&[(8, 1)])).unwrap();
            src.save_reached("daily-1k", "2026-01-01", 1000.0, 1)
                .unwrap();
        }
        let mut db = Db::open(&dir.join("stats.sqlite")).unwrap();
        db.add_day(day(2026, 9, 23), &delta(5, &[("KeyA", 5)]))
            .unwrap();
        db.save_reached("life-100k", LIFETIME, 100_000.0, 1)
            .unwrap();

        db.replace_from(&src_path).unwrap();
        assert_eq!(db.day_count().unwrap(), 1);
        assert!(db.load_day(day(2026, 9, 23)).unwrap().is_empty());
        let restored = db.load_day(day(2026, 1, 1)).unwrap();
        assert_eq!(restored.totals.keys, 10);
        assert_eq!(restored.per_key["KeyZ"], 9);
        assert_eq!(restored.per_hour[8].keys, 1);
        assert!(db.load_reached(day(2026, 9, 23)).unwrap().0.is_empty());
        // Still a working database after: the source is let go of.
        db.add_day(day(2026, 9, 24), &delta(1, &[])).unwrap();
        assert_eq!(db.day_count().unwrap(), 2);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_failed_replace_leaves_the_counts_alone() {
        let dir = temp_dir("replace-bad");
        let src_path = dir.join("src.sqlite");
        {
            // Some other database: none of the tables a restore needs.
            let conn = Connection::open(&src_path).unwrap();
            conn.execute_batch("CREATE TABLE daily(date TEXT PRIMARY KEY);")
                .unwrap();
        }
        let mut db = Db::open(&dir.join("stats.sqlite")).unwrap();
        db.add_day(day(2026, 9, 23), &delta(5, &[("KeyA", 5)]))
            .unwrap();
        assert!(db.replace_from(&src_path).is_err());
        assert_eq!(db.load_day(day(2026, 9, 23)).unwrap().totals.keys, 5);
        assert_eq!(db.day_count().unwrap(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
