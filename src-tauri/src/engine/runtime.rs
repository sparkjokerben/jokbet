//! The runtime thread: the only owner of the counters. It receives hook events,
//! manages the input hook's lifecycle, and pushes throttled updates to the pet.

use super::aggregator::{Activity, Aggregator, Totals};
use super::distance::DisplayMap;
use crate::db::Db;
use crate::input::{self, EventSink, InputHandle, Permission};
use crate::pet_window::PET_LABEL;
use crate::platform;
use chrono::{Local, NaiveDate};
use crossbeam_channel::{bounded, select, tick, Receiver, Sender};
use serde::Serialize;
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
    SetPaused(bool),
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
    /// Keys and clicks per minute over the last few seconds.
    pub kpm: u32,
    pub cpm: u32,
    /// Most recent activity since the previous tick.
    pub activity: Option<Activity>,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub permission: Permission,
    /// The hook is installed and delivering events.
    pub listening: bool,
    pub paused: bool,
}

/// Starts the runtime thread; `db` is `None` if storage could not be opened.
pub fn spawn<R: Runtime>(app: AppHandle<R>, db: Option<Db>, paused: bool) -> RuntimeHandle {
    let (ctrl_tx, ctrl_rx) = bounded(64);
    std::thread::Builder::new()
        .name("runtime".into())
        .spawn(move || Worker::new(app, db, paused).run(ctrl_rx))
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
    displays: DisplayMap,
    sink: EventSink,
    events: Receiver<input::TimedEvent>,
    hook: Option<Box<dyn InputHandle>>,
    status: Option<Status>,
    last_tick: Option<Tick>,
    activity: Option<Activity>,
}

impl<R: Runtime> Worker<R> {
    fn new(app: AppHandle<R>, db: Option<Db>, paused: bool) -> Self {
        let (tx, events) = bounded(EVENT_QUEUE);
        if input::permission() == Permission::Denied {
            input::request_permission();
        }
        let date = Local::now().date_naive();
        let today = db
            .as_ref()
            .and_then(|db| db.load_day(date).ok())
            .unwrap_or_default();
        let lifetime_saved = db
            .as_ref()
            .and_then(|db| db.lifetime_totals().ok())
            .unwrap_or_default();
        let mut agg = Aggregator::with_today(today);
        agg.paused = paused;
        Self {
            app,
            agg,
            db,
            lifetime_saved,
            date,
            displays: DisplayMap(platform::displays()),
            sink: EventSink::new(tx),
            events,
            hook: None,
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
            select! {
                recv(self.events) -> e => {
                    if let Ok(e) = e {
                        if let Some(a) = self.agg.ingest(e, &self.displays) {
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
                    Ok(Control::SetPaused(p)) => {
                        self.agg.paused = p;
                        self.housekeeping();
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
        };
        if self.status != Some(status) {
            self.status = Some(status);
            let _ = self.app.emit_to(PET_LABEL, "pet://status", status);
        }

        let today = Local::now().date_naive();
        if today != self.date {
            self.flush();
            self.date = today;
            self.agg.roll_over();
            self.last_tick = None;
        }
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
                eprintln!("saving counts failed: {e}");
                self.agg.pending.merge(pending);
            }
        }
    }

    /// Pushes counters to the pet when anything visible changed (at most 10 Hz).
    fn emit_tick(&mut self) {
        let now = input::now_ms();
        let tick = Tick {
            today: self.agg.today.totals.clone(),
            kpm: self.agg.keys_per_minute(now),
            cpm: self.agg.clicks_per_minute(now),
            activity: self.activity.take(),
        };
        if self.last_tick.as_ref() != Some(&tick) {
            let _ = self.app.emit_to(PET_LABEL, "pet://tick", &tick);
            self.last_tick = Some(tick);
        }
    }
}
