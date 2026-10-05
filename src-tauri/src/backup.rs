//! Backups: one zip holding a snapshot of the counts, the settings they were
//! kept with, and a manifest that says what the file is.
//!
//! Three kinds are made. A manual backup goes wherever it is saved. The daily
//! one goes to the backup folder (the app's own, or one the user picked) and
//! only the newest few are kept. And a restore first puts the counts it is
//! about to replace in a safety backup, so that a restore can be undone by
//! restoring again.
//!
//! A restore never swaps the database file out from under the app: the counts
//! are copied into the open database in one transaction (`Db::replace_from`).

use crate::db::{Db, SCHEMA_VERSION};
use crate::engine::runtime::{RuntimeHandle, Snapshot};
use crate::i18n;
use crate::settings::{self, SettingsStore};
use chrono::{Local, NaiveDate};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, Runtime};

/// The version of the layout below; a newer one is refused.
const FORMAT: u32 = 1;
const APP: &str = "jokbet";
/// The only entries a backup has, and the only ones ever read from one.
const MANIFEST: &str = "manifest.json";
const STATS: &str = "stats.sqlite";
const SETTINGS: &str = "settings.json";
/// Larger than any real database, so a damaged or hostile file cannot fill
/// the disk on its way out of the zip.
const MAX_DB_BYTES: u64 = 1 << 30;
const MAX_JSON_BYTES: u64 = 1 << 20;
/// How many safety backups a restore keeps around.
const SAFETY_KEEP: usize = 5;

const AUTO_PREFIX: &str = "jokbet-auto-";
const SAFETY_PREFIX: &str = "jokbet-safety-";
const MANUAL_PREFIX: &str = "jokbet-backup-";

/// The first part of the errors the settings window tells apart: not a backup
/// at all, or one from a newer version than this.
pub const INVALID: &str = "backup-invalid";
pub const TOO_NEW: &str = "backup-too-new";
/// A path the app never offered to restore: the pages may only name a file a
/// dialog here has handed out, or one of the app's own backup folders'.
pub const UNOFFERED: &str = "backup-unoffered";

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Kind {
    Manual,
    Auto,
    Safety,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    pub format: u32,
    pub app: String,
    /// The app version that made it.
    pub version: String,
    /// The database schema the snapshot is at.
    pub schema: i64,
    /// RFC 3339, local time.
    pub created_at: String,
    /// How many days have counts.
    pub days: u64,
    pub kind: Kind,
    /// Which computer made it (see `device_id`).
    pub device: String,
}

/// A backup as the restore list shows it.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BackupInfo {
    pub path: String,
    pub kind: Kind,
    pub created_at: String,
    pub days: u64,
    pub version: String,
    pub size: u64,
    /// Made on another computer (a backup folder can be shared through a
    /// synced drive).
    pub other_device: bool,
}

/// What went wrong with the last daily backup, until one works again.
#[derive(Default)]
pub struct BackupState {
    pub last_error: Mutex<Option<String>>,
    /// Paths the app's own dialogs have handed out for a restore, and that a
    /// restore may therefore name. Files in the app's own backup folders are
    /// always allowed; this is for the one a dialog picked somewhere else.
    admitted: Mutex<HashSet<PathBuf>>,
}

// --- files ------------------------------------------------------------------

fn invalid(why: impl std::fmt::Display) -> String {
    format!("{INVALID}: {why}")
}

fn io_err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

/// `path` with `.part` added, where a backup is written before it is complete.
fn part_path(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(".part");
    PathBuf::from(name)
}

/// Writes a backup to `dest`. It appears there only once it is complete, so a
/// half-written file is never taken for a backup.
pub fn write(
    dest: &Path,
    snapshot: &Path,
    settings_json: &[u8],
    manifest: &Manifest,
) -> Result<(), String> {
    if let Some(dir) = dest.parent() {
        std::fs::create_dir_all(dir).map_err(io_err)?;
    }
    let part = part_path(dest);
    let written = (|| -> Result<(), String> {
        let file = std::fs::File::create(&part).map_err(io_err)?;
        let mut zip = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        let manifest = serde_json::to_vec_pretty(manifest).map_err(io_err)?;
        zip.start_file(MANIFEST, options).map_err(io_err)?;
        zip.write_all(&manifest).map_err(io_err)?;
        zip.start_file(STATS, options).map_err(io_err)?;
        let mut db = std::fs::File::open(snapshot).map_err(io_err)?;
        std::io::copy(&mut db, &mut zip).map_err(io_err)?;
        zip.start_file(SETTINGS, options).map_err(io_err)?;
        zip.write_all(settings_json).map_err(io_err)?;
        zip.finish().map_err(io_err)?.sync_all().map_err(io_err)?;
        std::fs::rename(&part, dest).map_err(io_err)
    })();
    if written.is_err() {
        let _ = std::fs::remove_file(&part);
    }
    written
}

fn open_archive(path: &Path) -> Result<zip::ZipArchive<std::fs::File>, String> {
    let file = std::fs::File::open(path).map_err(io_err)?;
    zip::ZipArchive::new(file).map_err(invalid)
}

fn read_entry(
    archive: &mut zip::ZipArchive<std::fs::File>,
    name: &str,
    max: u64,
) -> Result<Vec<u8>, String> {
    let entry = archive.by_name(name).map_err(invalid)?;
    if entry.size() > max {
        return Err(invalid(format!("{name} is too large")));
    }
    let mut bytes = Vec::new();
    entry
        .take(max + 1)
        .read_to_end(&mut bytes)
        .map_err(invalid)?;
    Ok(bytes)
}

/// Reads a backup's manifest and checks that this version can restore it.
pub fn inspect(path: &Path, device: &str) -> Result<BackupInfo, String> {
    let mut archive = open_archive(path)?;
    let manifest: Manifest =
        serde_json::from_slice(&read_entry(&mut archive, MANIFEST, MAX_JSON_BYTES)?)
            .map_err(invalid)?;
    if manifest.app != APP {
        return Err(invalid("not a Jokbet backup"));
    }
    if manifest.format > FORMAT || manifest.schema > SCHEMA_VERSION {
        return Err(format!("{TOO_NEW}: made by Jokbet {}", manifest.version));
    }
    if archive.index_for_name(STATS).is_none() {
        return Err(invalid("no database in it"));
    }
    Ok(BackupInfo {
        path: path.to_string_lossy().into_owned(),
        kind: manifest.kind,
        created_at: manifest.created_at,
        days: manifest.days,
        version: manifest.version,
        size: std::fs::metadata(path).map(|m| m.len()).unwrap_or(0),
        other_device: manifest.device != device,
    })
}

/// Takes the database out of a backup, to `to`.
pub fn extract_db(path: &Path, to: &Path) -> Result<(), String> {
    let mut archive = open_archive(path)?;
    let entry = archive.by_name(STATS).map_err(invalid)?;
    if entry.size() > MAX_DB_BYTES {
        return Err(invalid("the database is too large"));
    }
    let mut out = std::fs::File::create(to).map_err(io_err)?;
    let copied = std::io::copy(&mut entry.take(MAX_DB_BYTES + 1), &mut out).map_err(invalid)?;
    if copied > MAX_DB_BYTES {
        return Err(invalid("the database is too large"));
    }
    out.sync_all().map_err(io_err)
}

/// The settings a backup was made with, if it has them.
pub fn read_settings(path: &Path) -> Option<Vec<u8>> {
    let mut archive = open_archive(path).ok()?;
    read_entry(&mut archive, SETTINGS, MAX_JSON_BYTES).ok()
}

/// Removes a database and the sidecars SQLite may have left beside it.
pub fn remove_db_files(path: &Path) {
    for suffix in ["", "-wal", "-shm", "-journal"] {
        let mut name = path.as_os_str().to_owned();
        name.push(suffix);
        let _ = std::fs::remove_file(PathBuf::from(name));
    }
}

// --- names ------------------------------------------------------------------

/// This computer's daily backup for `date`. The device is in the name so that
/// two computers sharing one folder each keep their own.
pub fn auto_name(date: NaiveDate, device: &str) -> String {
    format!("{AUTO_PREFIX}{}-{device}.zip", date.format("%Y-%m-%d"))
}

fn safety_name(now: chrono::DateTime<Local>) -> String {
    format!("{SAFETY_PREFIX}{}.zip", now.format("%Y%m%d-%H%M%S"))
}

fn is_hex_id(s: &str) -> bool {
    s.len() == 8 && s.bytes().all(|b| b.is_ascii_hexdigit())
}

/// The device of a daily backup's file name, if that is what it is.
fn auto_device(name: &str) -> Option<&str> {
    let rest = name.strip_prefix(AUTO_PREFIX)?.strip_suffix(".zip")?;
    let (date, device) = rest.split_at_checked(10)?;
    NaiveDate::parse_from_str(date, "%Y-%m-%d").ok()?;
    let device = device.strip_prefix('-')?;
    is_hex_id(device).then_some(device)
}

fn is_safety_name(name: &str) -> bool {
    name.strip_prefix(SAFETY_PREFIX)
        .and_then(|rest| rest.strip_suffix(".zip"))
        .is_some_and(|stamp| {
            stamp.len() == 15
                && stamp.bytes().enumerate().all(|(i, b)| {
                    if i == 8 {
                        b == b'-'
                    } else {
                        b.is_ascii_digit()
                    }
                })
        })
}

fn file_names(dir: &Path) -> Vec<String> {
    std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .flatten()
                .filter_map(|e| e.file_name().into_string().ok())
                .collect()
        })
        .unwrap_or_default()
}

/// Deletes all but the newest `keep` of the files in `dir` that `matches`
/// picks out. Names sort by the time in them, so the oldest go first; nothing
/// the pattern does not match is ever touched.
fn prune(dir: &Path, keep: usize, matches: impl Fn(&str) -> bool) {
    let mut names: Vec<String> = file_names(dir).into_iter().filter(|n| matches(n)).collect();
    names.sort_unstable_by(|a, b| b.cmp(a));
    for name in names.into_iter().skip(keep) {
        if let Err(e) = std::fs::remove_file(dir.join(&name)) {
            log::warn!("backup: removing {name} failed: {e}");
        }
    }
}

/// Keeps this computer's newest `keep` daily backups.
pub fn prune_auto(dir: &Path, device: &str, keep: usize) {
    prune(dir, keep, |n| auto_device(n) == Some(device));
}

fn prune_safety(dir: &Path) {
    prune(dir, SAFETY_KEEP, is_safety_name);
}

/// Every backup in `dirs`, newest first. Files that are not backups, or that
/// cannot be read, are left out.
pub fn list(dirs: &[PathBuf], device: &str) -> Vec<BackupInfo> {
    let mut seen = std::collections::HashSet::new();
    let mut items: Vec<BackupInfo> = dirs
        .iter()
        .filter(|dir| seen.insert(std::fs::canonicalize(dir).unwrap_or_else(|_| dir.to_path_buf())))
        .flat_map(|dir| {
            file_names(dir)
                .into_iter()
                .filter(|n| {
                    n.ends_with(".zip")
                        && [AUTO_PREFIX, SAFETY_PREFIX, MANUAL_PREFIX]
                            .iter()
                            .any(|p| n.starts_with(p))
                })
                .filter_map(|n| inspect(&dir.join(n), device).ok())
                .collect::<Vec<_>>()
        })
        .collect();
    items.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    items
}

/// A short random name for this computer, made once and kept in the data
/// directory — not in the settings, which a restore could copy to another one.
pub fn device_id(data_dir: &Path) -> String {
    use std::hash::BuildHasher;
    let path = data_dir.join("device-id");
    if let Ok(id) = std::fs::read_to_string(&path) {
        let id = id.trim();
        if is_hex_id(id) {
            return id.to_owned();
        }
    }
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default();
    let seed = std::collections::hash_map::RandomState::new().hash_one((nanos, std::process::id()));
    let id = format!("{:08x}", seed as u32);
    let _ = std::fs::create_dir_all(data_dir);
    if let Err(e) = std::fs::write(&path, &id) {
        log::warn!("backup: saving the device id failed: {e}");
    }
    id
}

// --- the app's side ---------------------------------------------------------

fn data_dir<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, String> {
    crate::portable::data_dir(app).map_err(io_err)
}

/// The app's own backup folder.
pub fn default_dir<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, String> {
    Ok(data_dir(app)?.join("backups"))
}

/// Where the daily backups go: the folder the user picked, or the app's own.
pub fn auto_dir<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, String> {
    match app.state::<SettingsStore>().get().auto_backup.dir {
        Some(dir) => Ok(PathBuf::from(dir)),
        None => default_dir(app),
    }
}

/// Zips a snapshot the runtime has written into a backup at `dest`, with the
/// current settings, and removes the snapshot.
pub fn package<R: Runtime>(
    app: &AppHandle<R>,
    snapshot: &Path,
    snap: Snapshot,
    dest: &Path,
    kind: Kind,
) -> Result<(), String> {
    let result = (|| {
        let settings =
            serde_json::to_vec_pretty(&app.state::<SettingsStore>().get()).map_err(io_err)?;
        let manifest = Manifest {
            format: FORMAT,
            app: APP.into(),
            version: app.package_info().version.to_string(),
            schema: snap.schema,
            created_at: Local::now().to_rfc3339(),
            days: snap.days,
            kind,
            device: device_id(&data_dir(app)?),
        };
        write(dest, snapshot, &settings, &manifest)
    })();
    remove_db_files(snapshot);
    result
}

/// A manual backup, to `dest`.
pub fn create<R: Runtime>(app: &AppHandle<R>, dest: &Path) -> Result<(), String> {
    let snapshot = data_dir(app)?.join(".backup-manual.sqlite");
    let snap = app.state::<RuntimeHandle>().snapshot(snapshot.clone())?;
    package(app, &snapshot, snap, dest, Kind::Manual)?;
    let _ = app.emit("app://backups-changed", ());
    Ok(())
}

/// The daily backup the runtime is due to make: where it goes, and whether it
/// is there already.
pub struct AutoTarget {
    pub dir: PathBuf,
    pub dest: PathBuf,
    pub device: String,
}

impl AutoTarget {
    pub fn for_day<R: Runtime>(app: &AppHandle<R>, date: NaiveDate) -> Result<Self, String> {
        let dir = auto_dir(app)?;
        let device = device_id(&data_dir(app)?);
        Ok(Self {
            dest: dir.join(auto_name(date, &device)),
            dir,
            device,
        })
    }

    /// Where the runtime writes its snapshot for it: in the app's own data,
    /// never in a synced folder.
    pub fn snapshot_path<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, String> {
        Ok(data_dir(app)?.join(".backup-auto.sqlite"))
    }
}

/// Finishes the daily backup the runtime has taken a snapshot for: off the
/// runtime thread, since a synced folder can be slow to write to.
pub fn finish_auto<R: Runtime>(
    app: AppHandle<R>,
    snapshot: PathBuf,
    snap: Snapshot,
    target: AutoTarget,
    keep: u32,
) {
    let spawned = std::thread::Builder::new()
        .name("backup".into())
        .spawn(move || {
            let result = (|| {
                package(&app, &snapshot, snap, &target.dest, Kind::Auto)?;
                prune_auto(&target.dir, &target.device, keep as usize);
                Ok::<_, String>(target.dest)
            })();
            let state = app.state::<BackupState>();
            match result {
                Ok(dest) => {
                    log::info!("backup: saved {}", dest.display());
                    *state.last_error.lock().unwrap() = None;
                }
                Err(e) => {
                    log::error!("backup: the daily backup failed: {e}");
                    *state.last_error.lock().unwrap() = Some(e);
                }
            }
            let _ = app.emit("app://backups-changed", ());
        });
    if let Err(e) = spawned {
        log::error!("backup: starting the daily backup failed: {e}");
    }
}

/// Records a daily backup that could not even be started.
pub fn auto_failed<R: Runtime>(app: &AppHandle<R>, error: String) {
    log::error!("backup: the daily backup failed: {error}");
    *app.state::<BackupState>().last_error.lock().unwrap() = Some(error);
    let _ = app.emit("app://backups-changed", ());
}

/// Where the list of backups comes from, and what is in it.
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct BackupList {
    /// Where the daily backups go.
    pub dir: String,
    /// The app's own backup folder, where safety backups always go.
    pub default_dir: String,
    pub items: Vec<BackupInfo>,
    /// Why the last daily backup failed, if it did.
    pub last_error: Option<String>,
}

pub fn list_all<R: Runtime>(app: &AppHandle<R>) -> Result<BackupList, String> {
    let default = default_dir(app)?;
    let dir = auto_dir(app)?;
    let device = device_id(&data_dir(app)?);
    Ok(BackupList {
        items: list(&[dir.clone(), default.clone()], &device),
        dir: dir.to_string_lossy().into_owned(),
        default_dir: default.to_string_lossy().into_owned(),
        last_error: app
            .state::<BackupState>()
            .last_error
            .lock()
            .unwrap()
            .clone(),
    })
}

pub fn inspect_file<R: Runtime>(app: &AppHandle<R>, path: &Path) -> Result<BackupInfo, String> {
    inspect(path, &device_id(&data_dir(app)?))
}

/// Whether a restore may name this path: a file in one of the app's own
/// backup folders, or one a dialog on this side has handed out. The IPC
/// surface carries no such trust by itself — a webview is only ever a
/// dialog's client, not the origin of where its files live.
fn is_admitted<R: Runtime>(app: &AppHandle<R>, path: &Path) -> bool {
    if let Ok(dir) = auto_dir(app) {
        if path.starts_with(dir) {
            return true;
        }
    }
    if let Ok(dir) = default_dir(app) {
        if path.starts_with(dir) {
            return true;
        }
    }
    app.state::<BackupState>()
        .admitted
        .lock()
        .unwrap()
        .contains(path)
}

/// Marks a path a dialog on this side has handed out.
fn admit<R: Runtime>(app: &AppHandle<R>, path: &Path) {
    app.state::<BackupState>()
        .admitted
        .lock()
        .unwrap()
        .insert(path.to_path_buf());
}

/// Asks where the backup goes, then makes one there. `None` when cancelled.
pub fn create_dialog<R: Runtime>(app: &AppHandle<R>) -> Result<Option<PathBuf>, String> {
    use tauri_plugin_dialog::DialogExt;

    let dir = auto_dir(app)?;
    let filter_name = i18n::t(
        i18n::Lang::resolve(app.state::<SettingsStore>().get().language),
        i18n::Text::BackupFilter,
    );
    let name = format!("{}.zip", Local::now().format("jokbet-backup-%Y%m%d-%H%M"));
    let picked = app
        .dialog()
        .file()
        .add_filter(filter_name, &["zip"])
        .set_file_name(name)
        .set_directory(&dir)
        .blocking_save_file();
    let Some(picked) = picked else {
        return Ok(None);
    };
    let dest = picked.into_path().map_err(io_err)?;
    create(app, &dest)?;
    Ok(Some(dest))
}

/// Asks which backup file to inspect, then says what it is. `None` when
/// cancelled.
pub fn inspect_dialog<R: Runtime>(app: &AppHandle<R>) -> Result<Option<BackupInfo>, String> {
    use tauri_plugin_dialog::DialogExt;

    let filter_name = i18n::t(
        i18n::Lang::resolve(app.state::<SettingsStore>().get().language),
        i18n::Text::BackupFilter,
    );
    let picked = app
        .dialog()
        .file()
        .add_filter(filter_name, &["zip"])
        .blocking_pick_file();
    let Some(picked) = picked else {
        return Ok(None);
    };
    let path = picked.into_path().map_err(io_err)?;
    let info = inspect_file(app, &path)?;
    admit(app, &path);
    Ok(Some(info))
}

/// Asks which folder the CSVs go to, then writes them there. `None` when
/// cancelled.
pub fn export_csv_dialog<R: Runtime>(
    app: &AppHandle<R>,
    runtime: &RuntimeHandle,
) -> Result<Option<PathBuf>, String> {
    use tauri_plugin_dialog::DialogExt;

    let default = data_dir(app)?;
    let Some(picked) = app
        .dialog()
        .file()
        .set_directory(&default)
        .blocking_pick_folder()
    else {
        return Ok(None);
    };
    let dir = picked.into_path().map_err(io_err)?;
    runtime.export_csv(dir.clone())?;
    Ok(Some(dir))
}

/// What a restore could not bring back.
#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct RestoreReport {
    /// The settings that were left as they were: `"shortcuts"` when another
    /// app holds one of the backup's, `"*"` when none could be applied.
    pub settings_skipped: Vec<String>,
}

/// Replaces every count with the backup's, after keeping the current ones in a
/// safety backup; with `with_settings`, the settings too.
pub fn restore<R: Runtime>(
    app: &AppHandle<R>,
    path: &Path,
    with_settings: bool,
) -> Result<RestoreReport, String> {
    let data = data_dir(app)?;
    if !is_admitted(app, path) {
        return Err(format!("{UNOFFERED}: {}", path.display()));
    }
    inspect(path, &device_id(&data))?;
    let staged = data.join(".restore.sqlite");
    remove_db_files(&staged);
    let result = (|| {
        extract_db(path, &staged)?;
        {
            // Opening it brings a backup from an older version up to this
            // schema, the same way the app's own database is.
            let db = Db::open(&staged).map_err(invalid)?;
            if !db.integrity_ok().unwrap_or(false) {
                return Err(invalid("the database is damaged"));
            }
            if db.schema_version().map_err(invalid)? != SCHEMA_VERSION {
                return Err(invalid("unexpected schema"));
            }
        }
        let runtime = app.state::<RuntimeHandle>();
        let safety = data.join(".backup-safety.sqlite");
        let snap = runtime.snapshot(safety.clone())?;
        let dir = default_dir(app)?;
        package(
            app,
            &safety,
            snap,
            &dir.join(safety_name(Local::now())),
            Kind::Safety,
        )?;
        prune_safety(&dir);
        runtime.restore(staged.clone())
    })();
    remove_db_files(&staged);
    result?;
    let _ = app.emit("app://backups-changed", ());

    let mut report = RestoreReport::default();
    if with_settings {
        match read_settings(path) {
            Some(bytes) => report.settings_skipped = restore_settings(app, &bytes),
            None => report.settings_skipped.push("*".into()),
        }
    }
    Ok(report)
}

/// Applies a backup's settings the way the settings window would, so that
/// everything they drive follows. Where the pet stands and whether the welcome
/// has been seen belong to this computer, and stay.
fn restore_settings<R: Runtime>(app: &AppHandle<R>, bytes: &[u8]) -> Vec<String> {
    let restored = settings::parse_backup(bytes);
    let Ok(mut patch) = serde_json::to_value(&restored) else {
        return vec!["*".into()];
    };
    if let Some(fields) = patch.as_object_mut() {
        fields.remove("petPosition");
        fields.remove("onboarded");
    }
    if restored
        .auto_backup
        .dir
        .as_deref()
        .is_some_and(|dir| !Path::new(dir).is_dir())
    {
        // A folder from another computer: back to the app's own.
        patch["autoBackup"]["dir"] = serde_json::Value::Null;
    }
    let mut skipped = Vec::new();
    let mut result = crate::commands::apply_patch(app, &patch);
    let shortcuts_refused = matches!(&result, Err(e)
        if e.starts_with(settings::SHORTCUT_TAKEN) || e.starts_with(settings::SHORTCUT_DUPLICATE));
    if shortcuts_refused {
        // Another app holds one of them here: everything else still comes back.
        if let Some(fields) = patch.as_object_mut() {
            fields.remove("shortcuts");
        }
        skipped.push("shortcuts".into());
        result = crate::commands::apply_patch(app, &patch);
    }
    if let Err(e) = result {
        log::warn!("backup: restoring the settings failed: {e}");
        return vec!["*".into()];
    }
    skipped
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("jdp-backup-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn manifest(kind: Kind, device: &str, created_at: &str) -> Manifest {
        Manifest {
            format: FORMAT,
            app: APP.into(),
            version: "0.4.0".into(),
            schema: SCHEMA_VERSION,
            created_at: created_at.into(),
            days: 2,
            kind,
            device: device.into(),
        }
    }

    /// A real snapshot with one day in it.
    fn snapshot(dir: &Path) -> PathBuf {
        let db_path = dir.join("live.sqlite");
        let mut db = Db::open(&db_path).unwrap();
        let mut day = crate::engine::aggregator::DayCounters::default();
        day.totals.keys = 12;
        db.add_day(NaiveDate::from_ymd_opt(2026, 9, 28).unwrap(), &day)
            .unwrap();
        let snap = dir.join("snap.sqlite");
        db.snapshot_to(&snap).unwrap();
        snap
    }

    #[test]
    fn a_backup_round_trips() {
        let dir = temp_dir("roundtrip");
        let snap = snapshot(&dir);
        let dest = dir.join("jokbet-backup-20260928-1200.zip");
        write(
            &dest,
            &snap,
            br#"{"bubble":false}"#,
            &manifest(Kind::Manual, "0000abcd", "2026-09-28T12:00:00+08:00"),
        )
        .unwrap();
        assert!(!part_path(&dest).exists());

        let info = inspect(&dest, "0000abcd").unwrap();
        assert_eq!(info.kind, Kind::Manual);
        assert_eq!(info.days, 2);
        assert!(!info.other_device);
        assert!(inspect(&dest, "ffffffff").unwrap().other_device);
        assert_eq!(read_settings(&dest).unwrap(), br#"{"bubble":false}"#);

        let out = dir.join("out.sqlite");
        extract_db(&dest, &out).unwrap();
        let db = Db::open(&out).unwrap();
        assert_eq!(db.day_count().unwrap(), 1);
        assert_eq!(
            db.load_day(NaiveDate::from_ymd_opt(2026, 9, 28).unwrap())
                .unwrap()
                .totals
                .keys,
            12
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn what_is_not_a_backup_is_refused() {
        let dir = temp_dir("refuse");
        let snap = snapshot(&dir);

        let junk = dir.join("junk.zip");
        std::fs::write(&junk, b"not a zip").unwrap();
        assert!(inspect(&junk, "x").unwrap_err().starts_with(INVALID));

        let mut other = manifest(Kind::Manual, "0000abcd", "2026-09-28T12:00:00+08:00");
        other.app = "something-else".into();
        let foreign = dir.join("foreign.zip");
        write(&foreign, &snap, b"{}", &other).unwrap();
        assert!(inspect(&foreign, "x").unwrap_err().starts_with(INVALID));

        let mut newer = manifest(Kind::Manual, "0000abcd", "2026-09-28T12:00:00+08:00");
        newer.schema = SCHEMA_VERSION + 1;
        let future = dir.join("future.zip");
        write(&future, &snap, b"{}", &newer).unwrap();
        assert!(inspect(&future, "x").unwrap_err().starts_with(TOO_NEW));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn names_are_recognised_strictly() {
        let date = NaiveDate::from_ymd_opt(2026, 9, 28).unwrap();
        let name = auto_name(date, "0a1b2c3d");
        assert_eq!(name, "jokbet-auto-2026-09-28-0a1b2c3d.zip");
        assert_eq!(auto_device(&name), Some("0a1b2c3d"));
        assert_eq!(
            auto_device("jokbet-auto-2026-09-28-0a1b2c3d.zip.part"),
            None
        );
        assert_eq!(auto_device("jokbet-auto-2026-13-28-0a1b2c3d.zip"), None);
        assert_eq!(auto_device("jokbet-auto-2026-09-28-zzzzzzzz.zip"), None);
        assert!(is_safety_name("jokbet-safety-20260928-153000.zip"));
        assert!(!is_safety_name("jokbet-safety-20260928-1530.zip"));
        assert!(!is_safety_name("jokbet-safety-20260928-153000.zip.part"));
    }

    #[test]
    fn pruning_keeps_the_newest_and_only_touches_its_own() {
        let dir = temp_dir("prune");
        for day in 20..=28 {
            let date = NaiveDate::from_ymd_opt(2026, 9, day).unwrap();
            std::fs::write(dir.join(auto_name(date, "0000abcd")), b"").unwrap();
        }
        let theirs = auto_name(NaiveDate::from_ymd_opt(2026, 9, 1).unwrap(), "ffff0000");
        std::fs::write(dir.join(&theirs), b"").unwrap();
        std::fs::write(dir.join("jokbet-backup-20200101-0000.zip"), b"").unwrap();
        std::fs::write(dir.join("notes.txt"), b"").unwrap();

        prune_auto(&dir, "0000abcd", 3);

        let mut left = file_names(&dir);
        left.sort();
        assert_eq!(
            left,
            [
                "jokbet-auto-2026-09-01-ffff0000.zip",
                "jokbet-auto-2026-09-26-0000abcd.zip",
                "jokbet-auto-2026-09-27-0000abcd.zip",
                "jokbet-auto-2026-09-28-0000abcd.zip",
                "jokbet-backup-20200101-0000.zip",
                "notes.txt",
            ]
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn safety_backups_keep_five() {
        let dir = temp_dir("safety");
        for minute in 10..18 {
            std::fs::write(
                dir.join(format!("jokbet-safety-20260928-15{minute}00.zip")),
                b"",
            )
            .unwrap();
        }
        prune_safety(&dir);
        let mut left = file_names(&dir);
        left.sort();
        assert_eq!(left.len(), SAFETY_KEEP);
        assert_eq!(left[0], "jokbet-safety-20260928-151300.zip");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_list_is_newest_first_and_skips_the_rest() {
        let dir = temp_dir("list");
        let snap = snapshot(&dir);
        let older = dir.join(auto_name(
            NaiveDate::from_ymd_opt(2026, 9, 27).unwrap(),
            "0000abcd",
        ));
        write(
            &older,
            &snap,
            b"{}",
            &manifest(Kind::Auto, "0000abcd", "2026-09-27T09:00:00+08:00"),
        )
        .unwrap();
        let newer = dir.join("jokbet-safety-20260928-153000.zip");
        write(
            &newer,
            &snap,
            b"{}",
            &manifest(Kind::Safety, "0000abcd", "2026-09-28T15:30:00+08:00"),
        )
        .unwrap();
        std::fs::write(dir.join("jokbet-backup-broken.zip"), b"nope").unwrap();
        std::fs::write(dir.join("holiday.zip"), b"nope").unwrap();

        // The same folder twice (the picked one and the default) lists once.
        let items = list(&[dir.clone(), dir.clone()], "0000abcd");
        let kinds: Vec<Kind> = items.iter().map(|i| i.kind).collect();
        assert_eq!(kinds, [Kind::Safety, Kind::Auto]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_device_id_is_made_once() {
        let dir = temp_dir("device");
        let id = device_id(&dir);
        assert!(is_hex_id(&id), "{id}");
        assert_eq!(device_id(&dir), id);
        std::fs::write(dir.join("device-id"), "garbage").unwrap();
        let fresh = device_id(&dir);
        assert!(is_hex_id(&fresh));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
