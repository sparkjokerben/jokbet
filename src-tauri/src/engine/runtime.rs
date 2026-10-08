//! The runtime thread: the only owner of the counters. It receives hook events,
//! manages the input hook's lifecycle, and pushes throttled updates to the pet.

use super::aggregator::{Activity, Aggregator, DayCounters, Totals};
use super::distance::DisplayMap;
use super::milestones::{self, Celebrations, Hit, MilestoneDef, Reached};
use super::rest::RestTracker;
use crate::db::Db;
use crate::input::{self, EventSink, InputHandle, Permission};
use crate::pet_window::PET_LABEL;
use crate::platform;
use crate::settings::{AutoBackup, Period, Settings};
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
    /// Saves pending counts, then writes a copy of the database to `to`.
    Snapshot {
        to: PathBuf,
        reply: Sender<Result<Snapshot, String>>,
    },
    /// Replaces every count with the ones in the database at `from`.
    Restore {
        from: PathBuf,
        reply: Sender<Result<(), String>>,
    },
    /// Current permission / hook / pause state.
    GetStatus(Sender<Result<Status, String>>),
    /// The pet was clicked: a break reminder, if one is up, has been seen.
    RestAck,
    /// Saves pending counts now and acknowledges once the flush ran. The
    /// paths that end the process without the exit event (`app.restart`,
    /// an installer's own exit) have to call this first.
    ///
    /// The reply is always `Ok`: a failed write keeps the batch pending and
    /// is logged by the flush itself, and nothing the caller can do helps.
    Flush(Sender<Result<(), String>>),
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

    /// `ask` without a deadline, for the whole-database work: a restore on a
    /// slow disk can outlast any fixed timeout, and the caller reporting
    /// failure while the worker carries it out leaves the two sides holding
    /// different truths about what the counts are.
    fn ask_open<T>(
        &self,
        make: impl FnOnce(Sender<Result<T, String>>) -> Control,
    ) -> Result<T, String> {
        let (tx, rx) = bounded(1);
        self.ctrl.send(make(tx)).map_err(|e| e.to_string())?;
        rx.recv().map_err(|e| e.to_string())?
    }

    pub fn stats(&self, days: u32) -> Result<Stats, String> {
        self.ask(|reply| Control::Stats { days, reply })
    }

    pub fn export_csv(&self, dir: PathBuf) -> Result<(), String> {
        self.ask_open(|reply| Control::ExportCsv { dir, reply })
    }

    pub fn clear(&self) -> Result<(), String> {
        self.ask_open(Control::Clear)
    }

    pub fn snapshot(&self, to: PathBuf) -> Result<Snapshot, String> {
        self.ask_open(|reply| Control::Snapshot { to, reply })
    }

    pub fn restore(&self, from: PathBuf) -> Result<(), String> {
        self.ask_open(|reply| Control::Restore { from, reply })
    }

    /// Writes the pending counts to the database now. Everything that ends
    /// the process without the exit event — `AppHandle::restart`, the
    /// updater's own exits — must call this first, or the last stretch of
    /// counts dies with the process.
    pub fn flush(&self) -> Result<(), String> {
        self.ask(Control::Flush)
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

/// Time for a break: how long the stretch at the keyboard has lasted.
#[derive(Clone, Copy, Debug, Serialize)]
pub struct Rest {
    pub minutes: u32,
}

/// What a snapshot of the database holds, for a backup's manifest.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Snapshot {
    pub schema: i64,
    pub days: u64,
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
    auto_backup: AutoBackup,
    /// The day a daily backup was last seen to or tried, so a failing one is
    /// not retried every second.
    auto_backup_day: Option<NaiveDate>,
    rest: RestTracker,
}

/// What the counters start a day from: today's counts, the lifetime totals on
/// disk, and the milestones already celebrated.
fn load_state(db: Option<&Db>, date: NaiveDate) -> (DayCounters, Totals, Reached) {
    let Some(db) = db else {
        return Default::default();
    };
    (
        db.load_day(date).unwrap_or_default(),
        db.lifetime_totals().unwrap_or_default(),
        db.load_reached(date).unwrap_or_default(),
    )
}

impl<R: Runtime> Worker<R> {
    fn new(app: AppHandle<R>, db: Option<Db>, settings: &Settings) -> Self {
        let (tx, events) = bounded(EVENT_QUEUE);
        if input::permission() == Permission::Denied {
            input::request_permission();
        }
        let now = Local::now();
        let date = now.date_naive();
        let (today, lifetime_saved, reached) = load_state(db.as_ref(), date);
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
            auto_backup: settings.auto_backup.clone(),
            auto_backup_day: None,
            rest: RestTracker::new(settings.rest_reminder.into()),
        }
    }

    fn run(mut self, ctrl: Receiver<Control>) {
        let ticker = tick(TICK);
        let mut ticks: u32 = 0;
        self.housekeeping();
        self.auto_backup();
        loop {
            // Unbiased on purpose: crossbeam shuffles the ready handles each
            // time round, so a flood of events cannot starve the ticker that
            // refreshes the hour. A `biased;` here would let it.
            select! {
                recv(self.events) -> e => {
                    if let Ok(e) = e {
                        let t = e.t_ms;
                        if let Some(a) = self.agg.ingest(e, &self.displays, self.hour) {
                            // A scroll only says someone is there: it does not
                            // take the place of a key or click the pet has yet
                            // to react to.
                            if a != Activity::Scroll || self.activity.is_none() {
                                self.activity = Some(a);
                            }
                            self.rest.activity(t, self.agg.paused);
                            // Only a key or a click needs the pet where it is, so
                            // only those end a stroll; a scroll leaves it be.
                            if a != Activity::Scroll {
                                crate::walker::stop(&self.app, crate::walker::Stop::Now);
                            }
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
                        self.rest.configure(s.rest_reminder.into());
                        if s.auto_backup != self.auto_backup {
                            // Switched on, or pointed at another folder: that
                            // folder may not have today's yet.
                            self.auto_backup = s.auto_backup.clone();
                            self.auto_backup_day = None;
                            self.auto_backup();
                        }
                        self.housekeeping();
                    }
                    Ok(Control::Stats { days, reply }) => {
                        let _ = reply.send(self.stats(days));
                    }
                    Ok(Control::ExportCsv { dir, reply }) => {
                        let _ = reply.send(self.export_csv(&dir));
                    }
                    Ok(Control::RestAck) => self.rest.ack(input::now_ms()),
                    Ok(Control::GetStatus(reply)) => {
                        self.housekeeping();
                        let _ = reply.send(self.status.ok_or_else(|| "no status yet".to_string()));
                    }
                    Ok(Control::Clear(reply)) => {
                        let _ = reply.send(self.clear());
                        self.last_tick = None;
                        let _ = self.app.emit("app://data-changed", ());
                    }
                    Ok(Control::Snapshot { to, reply }) => {
                        let _ = reply.send(self.snapshot(&to));
                    }
                    Ok(Control::Restore { from, reply }) => {
                        let _ = reply.send(self.restore(&from));
                        let _ = self.app.emit("app://data-changed", ());
                    }
                    Ok(Control::Flush(reply)) => {
                        self.flush();
                        let _ = reply.send(Ok(()));
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
        // A hook that says it is not delivering is let go of, so the start
        // below makes a fresh one: the OS can turn a listener off behind the
        // app's back, and one left in place would never be asked for again.
        if self.hook.as_ref().is_some_and(|h| !h.check_health()) {
            if let Some(h) = self.hook.take() {
                h.stop();
            }
        }
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
        let shown = crate::visibility::shown(&self.app);
        if let Some(minutes) = self.rest.poll(input::now_ms(), self.agg.paused, shown) {
            let _ = self.app.emit_to(PET_LABEL, "pet://rest", Rest { minutes });
        }

        // One reading of the clock for both, so the day and the hour can never
        // be a boundary apart. Events are filed under the hour cached here,
        // which is at most a second old: nothing against an hour of counts,
        // and the same slop the day boundary already has.
        let now = Local::now();
        let today = now.date_naive();
        if today != self.date {
            self.flush();
            // roll_over hands back whatever that flush could not save: a
            // failed write keeps the batch pending, and dropping it here
            // would lose the day along with the date. One last direct try
            // under the old date, so a late save still lands where it
            // belongs.
            let old_date = self.date;
            self.date = today;
            let unsaved = self.agg.roll_over();
            if !unsaved.is_empty() {
                match self.db.as_mut().map(|db| db.add_day(old_date, &unsaved)) {
                    Some(Ok(())) => self.lifetime_saved.add(&unsaved.totals),
                    Some(Err(e)) => log::error!("saving counts for {old_date} failed: {e}"),
                    None => log::error!("counts for {old_date} had nowhere to go: no database"),
                }
            }
            self.last_tick = None;
            if let Some(db) = self.db.as_mut() {
                let _ = db.prune_reached(today);
            }
            self.auto_backup();
        }
        self.hour = now.hour() as u8;
    }

    /// Makes today's daily backup if it is due: the snapshot here, where the
    /// database is, and the rest on a thread of its own.
    fn auto_backup(&mut self) {
        if !self.auto_backup.enabled || self.auto_backup_day == Some(self.date) {
            return;
        }
        self.auto_backup_day = Some(self.date);
        // Nothing to keep yet (a fresh install): no backup, so the ones from
        // before a reinstall are not rotated out by empty ones.
        if self
            .db
            .as_ref()
            .is_none_or(|db| db.day_count().unwrap_or(0) == 0)
        {
            return;
        }
        let target = match crate::backup::AutoTarget::for_day(&self.app, self.date) {
            Ok(target) => target,
            Err(e) => return crate::backup::auto_failed(&self.app, e),
        };
        if target.dest.exists() {
            return;
        }
        let snapshot = match crate::backup::AutoTarget::snapshot_path(&self.app) {
            Ok(path) => path,
            Err(e) => return crate::backup::auto_failed(&self.app, e),
        };
        match self.snapshot(&snapshot) {
            Ok(snap) => crate::backup::finish_auto(
                self.app.clone(),
                snapshot,
                snap,
                target,
                self.auto_backup.keep,
            ),
            Err(e) => crate::backup::auto_failed(&self.app, e),
        }
    }

    /// Records newly crossed milestones; returns them.
    fn record_milestones(&mut self) -> Vec<Hit> {
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
        }
        hits
    }

    /// Records newly crossed milestones and celebrates them (at most once a minute).
    fn check_milestones(&mut self) {
        let hits = self.record_milestones();
        // Disabled milestones are still recorded, so turning them on later
        // does not replay everything already passed.
        if !hits.is_empty() && self.milestones_on {
            self.celebrations.push(hits);
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

    fn snapshot(&mut self, to: &std::path::Path) -> Result<Snapshot, String> {
        self.flush();
        let db = self.db()?;
        let err = |e: rusqlite::Error| e.to_string();
        db.snapshot_to(to).map_err(err)?;
        Ok(Snapshot {
            schema: db.schema_version().map_err(err)?,
            days: db.day_count().map_err(err)?,
        })
    }

    /// Takes every count from the database at `from`, then starts the day
    /// over from what is now on disk, as a fresh start would.
    fn restore(&mut self, from: &std::path::Path) -> Result<(), String> {
        self.flush();
        self.db()?.replace_from(from).map_err(|e| e.to_string())?;
        let (today, lifetime, reached) = load_state(self.db.as_ref(), self.date);
        let paused = self.agg.paused;
        self.agg = Aggregator::with_today(today);
        self.agg.paused = paused;
        self.lifetime_saved = lifetime;
        self.reached = reached;
        self.celebrations.clear();
        self.last_tick = None;
        // Whatever the restored counts are already past is recorded without a
        // party: a backup from before a milestone existed would otherwise set
        // off every one of them at once.
        self.record_milestones();
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
