//! Background updates: check at start and every 6 h, download silently, and
//! install when the user picks "Restart to Update" or quits.

use crate::menu::AppMenu;
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Manager, Runtime};
use tauri_plugin_updater::{Update, UpdaterExt};

const CHECK_EVERY: Duration = Duration::from_secs(6 * 60 * 60);

#[derive(Default)]
pub struct PendingUpdate(Mutex<Option<(Update, Vec<u8>)>>);

pub fn spawn<R: Runtime>(app: AppHandle<R>) {
    // Dev builds have no release to compare against.
    if cfg!(debug_assertions) {
        return;
    }
    std::thread::Builder::new()
        .name("updater".into())
        .spawn(move || loop {
            let ready = app.state::<PendingUpdate>().0.lock().unwrap().is_some();
            if !ready {
                if let Err(e) = tauri::async_runtime::block_on(check(&app)) {
                    eprintln!("update check failed: {e}");
                }
            }
            std::thread::sleep(CHECK_EVERY);
        })
        .expect("spawn updater thread");
}

async fn check<R: Runtime>(app: &AppHandle<R>) -> Result<(), tauri_plugin_updater::Error> {
    let Some(update) = app.updater()?.check().await? else {
        return Ok(());
    };
    let bytes = update.download(|_, _| {}, || {}).await?;
    let version = update.version.clone();
    *app.state::<PendingUpdate>().0.lock().unwrap() = Some((update, bytes));
    app.state::<AppMenu<R>>().show_update(app, &version);
    Ok(())
}

/// Installs a downloaded update; returns true if one was installed.
pub fn install_pending<R: Runtime>(app: &AppHandle<R>) -> bool {
    let Some((update, bytes)) = app.state::<PendingUpdate>().0.lock().unwrap().take() else {
        return false;
    };
    match update.install(bytes) {
        Ok(()) => true,
        Err(e) => {
            eprintln!("installing update failed: {e}");
            false
        }
    }
}
