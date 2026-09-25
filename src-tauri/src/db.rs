//! SQLite storage: one row of totals per day plus per-key counts per day.
//! Writes are deltas added in one transaction, so a crash loses at most one
//! flush interval and never double counts.

use crate::engine::aggregator::{DayCounters, Totals};
use crate::engine::milestones::{Reached, LIFETIME};
use chrono::NaiveDate;
use rusqlite::{params, Connection, OptionalExtension};
use std::collections::HashMap;
use std::path::Path;

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

pub struct Db {
    conn: Connection,
}

/// How far back the insights look, matching the stats window's own clamp.
const HISTORY_DAYS: i64 = 3660;

fn day_key(date: NaiveDate) -> String {
    date.format(DATE_FMT).to_string()
}

fn totals_from_row(row: &rusqlite::Row, first: usize) -> rusqlite::Result<Totals> {
    Ok(Totals {
        keys: row.get::<_, i64>(first)? as u64,
        click_left: row.get::<_, i64>(first + 1)? as u64,
        click_right: row.get::<_, i64>(first + 2)? as u64,
        click_middle: row.get::<_, i64>(first + 3)? as u64,
        scrolls: row.get::<_, i64>(first + 4)? as u64,
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
        Ok(Self { conn })
    }

    /// Adds a day's delta to what is stored.
    pub fn add_day(&mut self, date: NaiveDate, delta: &DayCounters) -> rusqlite::Result<()> {
        let d = day_key(date);
        let t = &delta.totals;
        let tx = self.conn.transaction()?;
        tx.execute(
            "INSERT INTO daily(date, keys, click_left, click_right, click_middle, scrolls, move_px, move_mm)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(date) DO UPDATE SET
               keys = keys + excluded.keys,
               click_left = click_left + excluded.click_left,
               click_right = click_right + excluded.click_right,
               click_middle = click_middle + excluded.click_middle,
               scrolls = scrolls + excluded.scrolls,
               move_px = move_px + excluded.move_px,
               move_mm = move_mm + excluded.move_mm",
            params![
                d,
                t.keys as i64,
                t.click_left as i64,
                t.click_right as i64,
                t.click_middle as i64,
                t.scrolls as i64,
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
                "INSERT INTO hourly(date, hour, keys, click_left, click_right, click_middle, scrolls, move_px, move_mm)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                 ON CONFLICT(date, hour) DO UPDATE SET
                   keys = keys + excluded.keys,
                   click_left = click_left + excluded.click_left,
                   click_right = click_right + excluded.click_right,
                   click_middle = click_middle + excluded.click_middle,
                   scrolls = scrolls + excluded.scrolls,
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
                    t.scrolls as i64,
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
                "SELECT keys, click_left, click_right, click_middle, scrolls, move_px, move_mm
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
            "SELECT hour, keys, click_left, click_right, click_middle, scrolls, move_px, move_mm
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
            "SELECT date, keys, click_left, click_right, click_middle, scrolls, move_px, move_mm
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
                    SUM(scrolls), SUM(move_px), SUM(move_mm)
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
            "SELECT date, keys, click_left, click_right, click_middle, scrolls, move_px, move_mm
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
        let mut out =
            String::from("date,keys,click_left,click_right,click_middle,scrolls,move_px,move_m\n");
        let mut stmt = self.conn.prepare(
            "SELECT date, keys, click_left, click_right, click_middle, scrolls, move_px, move_mm
             FROM daily ORDER BY date",
        )?;
        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            let d: String = row.get(0)?;
            let t = totals_from_row(row, 1)?;
            out.push_str(&format!(
                "{d},{},{},{},{},{},{:.0},{:.2}\n",
                t.keys,
                t.click_left,
                t.click_right,
                t.click_middle,
                t.scrolls,
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
            "date,hour,keys,click_left,click_right,click_middle,scrolls,move_px,move_m\n",
        );
        let mut stmt = self.conn.prepare(
            "SELECT date, hour, keys, click_left, click_right, click_middle, scrolls, move_px, move_mm
             FROM hourly ORDER BY date, hour",
        )?;
        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            let (d, h): (String, i64) = (row.get(0)?, row.get(1)?);
            let t = totals_from_row(row, 2)?;
            out.push_str(&format!(
                "{d},{h},{},{},{},{},{},{:.0},{:.2}\n",
                t.keys,
                t.click_left,
                t.click_right,
                t.click_middle,
                t.scrolls,
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
                    COALESCE(SUM(click_middle), 0), COALESCE(SUM(scrolls), 0),
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
            "date,keys,click_left,click_right,click_middle,scrolls,move_px,move_m\n2026-09-23,3,1,0,0,0,0,0.00\n"
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
            "date,hour,keys,click_left,click_right,click_middle,scrolls,move_px,move_m\n\
             2026-09-23,9,3,0,0,0,0,0,0.00\n\
             2026-09-23,14,5,0,0,0,0,0,0.00\n"
        );
        db.clear().unwrap();
        assert_eq!(
            db.hourly_csv().unwrap(),
            "date,hour,keys,click_left,click_right,click_middle,scrolls,move_px,move_m\n"
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
        assert_eq!(version, 2);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
