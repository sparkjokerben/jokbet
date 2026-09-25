//! The runtime thread: the only owner of the counters. It receives hook events,
//! manages the input hook's lifecycle, and pushes throttled updates to the pet.

use super::aggregator::{Activity, Aggregator, Totals};
use super::distance::DisplayMap;
use super::milestones::{self, Celebrations, Hit, MilestoneDef, Reached};
use crate::db::Db;
use crate::input::{self, EventSink, InputHandle, Permission};
use crate::pet_window::PET_LABEL;
use crate::platform;
use crate::settings::{Period, Settings};
use chrono::{Local, NaiveDate, Timelike};
use crossbeam_channel::{bounded, select, tick, Receiver, Sender};
use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Runtime};

const EVENT_QUEUE: usize = 16_384;
const TICK: Duration = Duration::from_millis(100);
const HOUSEKEEPING_EVERY: u32 = 10; // ticks
const DISPLAYS_EVERY: u32 = 100; // ticks
const FLUSH_EVERY: u32 = 300; // ticks = 30 s

pub enum Control {
    /// The pet window is listening; resend the current state.
    PetReady,
    /// Settings changed: pause state and milestone definitions.
    Settings(Box<Settings>),
    /// Saves pending counts, then answers with the last `days` days.
    Stats {
        days: u32,
        reply: Sender<Result<Stats, String>>,
    },
    /// Writes daily.csv and keys.csv into `dir`.
    ExportCsv {
        dir: PathBuf,
        reply: Sender<Result<(), String>>,
    },
    /// Deletes all counts, including today's.
    Clear(Sender<Result<(), String>>),
    /// Current permission / hook / pause state.
    GetStatus(Sender<Result<Status, String>>),
    /// Stop the hook and acknowledge once everything is saved.
    Shutdown(Sender<()>),
}

#[derive(Clone)]
pub struct RuntimeHandle {
    ctrl: Sender<Control>,
}

impl RuntimeHandle {
    pub fn send(&self, c: Control) {
        let _ = self.ctrl.send(c);
    }

    fn ask<T>(&self, make: impl FnOnce(Sender<Result<T, String>>) -> Control) -> Result<T, String> {
        let (tx, rx) = bounded(1);
        self.ctrl.send(make(tx)).map_err(|e| e.to_string())?;
        rx.recv_timeout(Duration::from_secs(10))
            .map_err(|e| e.to_string())?
    }

    pub fn stats(&self, days: u32) -> Result<Stats, String> {
        self.ask(|reply| Control::Stats { days, reply })
    }

    pub fn export_csv(&self, dir: PathBuf) -> Result<(), String> {
        self.ask(|reply| Control::ExportCsv { dir, reply })
    }

    pub fn clear(&self) -> Result<(), String> {
        self.ask(Control::Clear)
    }

    pub fn status(&self) -> Result<Status, String> {
        self.ask(Control::GetStatus)
    }

    pub fn shutdown(&self, timeout: Duration) {
        let (ack_tx, ack_rx) = bounded(1);
        if self.ctrl.send(Control::Shutdown(ack_tx)).is_ok() {
            let _ = ack_rx.recv_timeout(timeout);
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Tick {
    pub today: Totals,
    /// Keys and clicks per second over the last few seconds.
    pub kps: f64,
    pub cps: f64,
    /// Most recent activity since the previous tick.
    pub activity: Option<Activity>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DayStat {
    pub date: String,
    #[serde(flatten)]
    pub totals: Totals,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Stats {
    /// Oldest first, ending today; days without data are zero.
    pub days: Vec<DayStat>,
    /// Per-key counts over the same range.
    pub keys: HashMap<String, u64>,
    /// Totals per hour of the day over the same range; index 0 is midnight.
    pub hours: [Totals; 24],
    /// Every day that has counts, oldest first. The insights look past the
    /// selected range, and days without counts are never stored, so this is
    /// smaller than the span it covers.
    pub history: Vec<DayStat>,
    pub lifetime: Totals,
    /// Every key pressed on any day, which tells what kind of keyboard it is.
    pub ever_pressed: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Celebrate {
    pub hits: Vec<Hit>,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub permission: Permission,
    /// The hook is installed and delivering events.
    pub listening: bool,
    pub paused: bool,
    /// The database is open; without it counts are only kept in memory.
    pub storage: bool,
}

/// Starts the runtime thread; `db` is `None` if storage could not be opened.
pub fn spawn<R: Runtime>(app: AppHandle<R>, db: Option<Db>, settings: &Settings) -> RuntimeHandle {
    let (ctrl_tx, ctrl_rx) = bounded(64);
    let settings = settings.clone();
    std::thread::Builder::new()
        .name("runtime".into())
        .spawn(move || Worker::new(app, db, &settings).run(ctrl_rx))
        .expect("spawn runtime thread");
    RuntimeHandle { ctrl: ctrl_tx }
}

struct Worker<R: Runtime> {
    app: AppHandle<R>,
    agg: Aggregator,
    db: Option<Db>,
    /// Lifetime totals already on disk; live lifetime adds `agg.pending`.
    lifetime_saved: Totals,
    date: NaiveDate,
    /// The local hour of day, refreshed about once a second while the loop
    /// runs. Events carry a monotonic timestamp, not a clock, so the hour has
    /// to be read here and handed to the aggregator.
    hour: u8,
    displays: DisplayMap,
    sink: EventSink,
    events: Receiver<input::TimedEvent>,
    hook: Option<Box<dyn InputHandle>>,
    milestone_defs: Vec<MilestoneDef>,
    milestones_on: bool,
    reached: Reached,
    celebrations: Celebrations,
    status: Option<Status>,
    last_tick: Option<Tick>,
    activity: Option<Activity>,
}

impl<R: Runtime> Worker<R> {
    fn new(app: AppHandle<R>, db: Option<Db>, settings: &Settings) -> Self {
        let (tx, events) = bounded(EVENT_QUEUE);
        if input::permission() == Permission::Denied {
            input::request_permission();
        }
        let now = Local::now();
        let date = now.date_naive();
        let today = db
            .as_ref()
            .and_then(|db| db.load_day(date).ok())
            .unwrap_or_default();
        let lifetime_saved = db
            .as_ref()
            .and_then(|db| db.lifetime_totals().ok())
            .unwrap_or_default();
        let reached = db
            .as_ref()
            .and_then(|db| db.load_reached(date).ok())
            .unwrap_or_default();
        let mut agg = Aggregator::with_today(today);
        agg.paused = settings.paused;
        Self {
            app,
            agg,
            db,
            lifetime_saved,
            date,
            hour: now.hour() as u8,
            displays: DisplayMap(platform::displays()),
            sink: EventSink::new(tx),
            events,
            hook: None,
            milestone_defs: milestones::definitions(
                &settings.custom_milestones,
                &settings.head_counter,
            ),
            milestones_on: settings.milestones,
            reached,
            celebrations: Celebrations::default(),
            status: None,
            last_tick: None,
            activity: None,
        }
    }

    fn run(mut self, ctrl: Receiver<Control>) {
        let ticker = tick(TICK);
        let mut ticks: u32 = 0;
        self.housekeeping();
        loop {
            // Unbiased on purpose: crossbeam shuffles the ready handles each
            // time round, so a flood of events cannot starve the ticker that
            // refreshes the hour. A `biased;` here would let it.
            select! {
                recv(self.events) -> e => {
                    if let Ok(e) = e {
                        if let Some(a) = self.agg.ingest(e, &self.displays, self.hour) {
                            self.activity = Some(a);
                        }
                    }
                }
                recv(ctrl) -> c => match c {
                    Ok(Control::PetReady) => {
                        self.status = None;
                        self.last_tick = None;
                        self.housekeeping();
                        self.emit_tick();
                    }
                    Ok(Control::Settings(s)) => {
                        self.agg.paused = s.paused;
                        self.milestones_on = s.milestones;
                        self.milestone_defs = milestones::definitions(&s.custom_milestones, &s.head_counter);
                        self.housekeeping();
                    }
                    Ok(Control::Stats { days, reply }) => {
                        let _ = reply.send(self.stats(days));
                    }
                    Ok(Control::ExportCsv { dir, reply }) => {
                        let _ = reply.send(self.export_csv(&dir));
                    }
                    Ok(Control::GetStatus(reply)) => {
                        self.housekeeping();
                        let _ = reply.send(self.status.ok_or_else(|| "no status yet".to_string()));
                    }
                    Ok(Control::Clear(reply)) => {
                        let _ = reply.send(self.clear());
                        self.last_tick = None;
                    }
                    Ok(Control::Shutdown(ack)) => {
                        if let Some(h) = self.hook.take() {
                            h.stop();
                        }
                        self.flush();
                        let _ = ack.send(());
                        return;
                    }
                    Err(_) => {
                        self.flush();
                        return;
                    }
                },
                recv(ticker) -> _ => {
                    ticks = ticks.wrapping_add(1);
                    if ticks.is_multiple_of(HOUSEKEEPING_EVERY) {
                        self.housekeeping();
                    }
                    if ticks.is_multiple_of(DISPLAYS_EVERY) {
                        self.displays = DisplayMap(platform::displays());
                    }
                    if ticks.is_multiple_of(FLUSH_EVERY) {
                        self.flush();
                    }
                    self.check_milestones();
                    self.emit_tick();
                }
            }
        }
    }

    /// Once a second: (re)start the hook, report status, roll the day over.
    fn housekeeping(&mut self) {
        let permission = input::permission();
        if self.hook.is_none() && permission != Permission::Denied {
            self.hook = input::start(self.sink.clone()).ok();
        }
        let listening = self.hook.as_ref().is_some_and(|h| h.check_health());
        let status = Status {
            permission,
            listening,
            paused: self.agg.paused,
            storage: self.db.is_some(),
        };
        if self.status != Some(status) {
            self.status = Some(status);
            let _ = self.app.emit("app://status", status);
        }

        // One reading of the clock for both, so the day and the hour can never
        // be a boundary apart. Events are filed under the hour cached here,
        // which is at most a second old: nothing against an hour of counts,
        // and the same slop the day boundary already has.
        let now = Local::now();
        let today = now.date_naive();
        if today != self.date {
            self.flush();
            self.date = today;
            self.agg.roll_over();
            self.last_tick = None;
            if let Some(db) = self.db.as_mut() {
                let _ = db.prune_reached(today);
            }
        }
        self.hour = now.hour() as u8;
    }

    /// Records newly crossed milestones and celebrates them (at most once a minute).
    fn check_milestones(&mut self) {
        let mut lifetime = self.lifetime_saved.clone();
        lifetime.add(&self.agg.pending.totals);
        let date = self.date.format("%Y-%m-%d").to_string();
        let hits = self.reached.check(
            &self.milestone_defs,
            &date,
            &self.agg.today.totals,
            &lifetime,
        );
        if !hits.is_empty() {
            if let Some(db) = self.db.as_mut() {
                let now = chrono::Utc::now().timestamp();
                for h in &hits {
                    let period = match h.period {
                        Period::Daily => date.as_str(),
                        Period::Lifetime => milestones::LIFETIME,
                    };
                    let _ = db.save_reached(&h.id, period, h.level, now);
                }
            }
            // Disabled milestones are still recorded, so turning them on later
            // does not replay everything already passed.
            if self.milestones_on {
                self.celebrations.push(hits);
            }
        }
        if let Some(hits) = self.celebrations.poll(input::now_ms()) {
            let _ = self
                .app
                .emit_to(PET_LABEL, "pet://celebrate", Celebrate { hits });
        }
    }

    fn db(&mut self) -> Result<&mut Db, String> {
        self.db
            .as_mut()
            .ok_or_else(|| "storage is unavailable".to_string())
    }

    fn stats(&mut self, days: u32) -> Result<Stats, String> {
        self.flush();
        let to = self.date;
        let from = to - chrono::Days::new(u64::from(days.clamp(1, 3660)) - 1);
        let db = self.db()?;
        let err = |e: rusqlite::Error| e.to_string();
        Ok(Stats {
            days: db
                .range(from, to)
                .map_err(err)?
                .into_iter()
                .map(|(date, totals)| DayStat {
                    date: date.format("%Y-%m-%d").to_string(),
                    totals,
                })
                .collect(),
            keys: db.key_counts(from, to).map_err(err)?,
            hours: db.hours(from, to).map_err(err)?,
            history: db
                .history()
                .map_err(err)?
                .into_iter()
                .map(|(date, totals)| DayStat { date, totals })
                .collect(),
            lifetime: db.lifetime_totals().map_err(err)?,
            ever_pressed: db.ever_pressed().map_err(err)?,
        })
    }

    fn export_csv(&mut self, dir: &std::path::Path) -> Result<(), String> {
        self.flush();
        let db = self.db()?;
        let daily = db.daily_csv().map_err(|e| e.to_string())?;
        let hourly = db.hourly_csv().map_err(|e| e.to_string())?;
        let keys = db.keys_csv().map_err(|e| e.to_string())?;
        std::fs::write(dir.join("jokbet-daily.csv"), daily).map_err(|e| e.to_string())?;
        std::fs::write(dir.join("jokbet-hourly.csv"), hourly).map_err(|e| e.to_string())?;
        std::fs::write(dir.join("jokbet-keys.csv"), keys).map_err(|e| e.to_string())
    }

    fn clear(&mut self) -> Result<(), String> {
        self.db()?.clear().map_err(|e| e.to_string())?;
        let paused = self.agg.paused;
        self.agg = Aggregator::with_today(Default::default());
        self.agg.paused = paused;
        self.lifetime_saved = Totals::default();
        self.reached = Reached::default();
        self.celebrations.clear();
        Ok(())
    }

    /// Adds pending counts to the database; keeps them pending if that fails.
    fn flush(&mut self) {
        let pending = self.agg.take_pending();
        if pending.is_empty() {
            return;
        }
        let Some(db) = self.db.as_mut() else {
            self.agg.pending.merge(pending);
            return;
        };
        match db.add_day(self.date, &pending) {
            Ok(()) => self.lifetime_saved.add(&pending.totals),
            Err(e) => {
                log::error!("saving counts failed: {e}");
                self.agg.pending.merge(pending);
            }
        }
    }

    /// Pushes counters to the pet when anything visible changed (at most 10 Hz).
    fn emit_tick(&mut self) {
        let now = input::now_ms();
        let tick = Tick {
            today: self.agg.today.totals.clone(),
            kps: self.agg.keys_per_second(now),
            cps: self.agg.clicks_per_second(now),
            activity: self.activity.take(),
        };
        if self.last_tick.as_ref() != Some(&tick) {
            let _ = self.app.emit_to(PET_LABEL, "pet://tick", &tick);
            self.last_tick = Some(tick);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The shape the stats window reads back: `hours` is twenty-four totals in
    /// order, and a history entry is a day like any other.
    #[test]
    fn stats_serialize_the_way_the_window_reads_them() {
        let stats = Stats {
            days: vec![DayStat {
                date: "2026-09-25".into(),
                totals: Totals {
                    keys: 7,
                    move_mm: 1.5,
                    ..Default::default()
                },
            }],
            keys: HashMap::new(),
            hours: std::array::from_fn(|hour| Totals {
                keys: hour as u64,
                ..Default::default()
            }),
            history: vec![DayStat {
                date: "2026-09-24".into(),
                totals: Totals::default(),
            }],
            lifetime: Totals::default(),
            ever_pressed: vec![],
        };
        let value = serde_json::to_value(&stats).unwrap();

        let hours = value["hours"].as_array().expect("hours is an array");
        assert_eq!(hours.len(), 24);
        assert_eq!(hours[0]["keys"].as_u64(), Some(0));
        assert_eq!(hours[23]["keys"].as_u64(), Some(23));
        // camelCase reaches an hour's totals the same way it reaches a day's.
        assert_eq!(hours[0]["clickLeft"].as_u64(), Some(0));
        assert_eq!(value["days"][0]["clickLeft"].as_u64(), Some(0));
        assert_eq!(value["days"][0]["moveMm"].as_f64(), Some(1.5));
        assert_eq!(value["history"][0]["date"].as_str(), Some("2026-09-24"));
    }
}
