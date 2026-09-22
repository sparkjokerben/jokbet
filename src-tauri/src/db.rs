//! SQLite storage: one row of totals per day plus per-key counts per day.
//! Writes are deltas added in one transaction, so a crash loses at most one
//! flush interval and never double counts.

use crate::engine::aggregator::{DayCounters, Totals};
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

pub struct Db {
    conn: Connection,
}

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

    fn init(conn: Connection) -> rusqlite::Result<Self> {
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.busy_timeout(std::time::Duration::from_secs(2))?;
        let version: i64 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
        if version < 1 {
            conn.execute_batch(SCHEMA_V1)?;
            conn.pragma_update(None, "user_version", 1)?;
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
        Ok(DayCounters { totals, per_key })
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
        }
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
    fn reopening_keeps_data_and_schema() {
        let dir = std::env::temp_dir().join(format!("jdp-db-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("stats.sqlite");
        let d = day(2026, 9, 23);
        Db::open(&path).unwrap().add_day(d, &delta(7, &[])).unwrap();
        assert_eq!(Db::open(&path).unwrap().load_day(d).unwrap().totals.keys, 7);
    }
}
