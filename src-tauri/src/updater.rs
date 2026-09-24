//! Background updates: check at start and every 6 h, download silently, and
//! install when the user picks "Restart to Update" or quits.
//!
//! Both steps have two sources. The check asks the site's mirror first and
//! GitHub second (the `endpoints` in tauri.conf.json; the plugin walks them in
//! order). The download goes wherever the manifest that answered points, and if
//! it fails there it is retried on the other host: every release file lives at
//! the same path on both (`/dl/<tag>/<name>` on the mirror,
//! `/releases/download/<tag>/<name>` on GitHub), and the signature the manifest
//! carries is the file's, so either copy verifies against it.

use crate::menu::AppMenu;
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Manager, Runtime, Url};
use tauri_plugin_updater::{Update, UpdaterExt};

const CHECK_EVERY: Duration = Duration::from_secs(6 * 60 * 60);
/// A source that does not answer a check in this long is skipped for the next.
const CHECK_TIMEOUT: Duration = Duration::from_secs(20);
/// Downloads have no overall limit (an AppImage on a slow line takes a while),
/// but a connection that cannot be made, or goes quiet, fails — so the other
/// source gets its turn instead of the updater waiting forever.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
const STALL_TIMEOUT: Duration = Duration::from_secs(60);

/// The two homes of a release file, up to the `<tag>/<name>` they share.
const MIRROR: &str = "https://jokbet.jokerben.top/dl/";
const GITHUB: &str = "https://github.com/sparkjokerben/jokbet/releases/download/";

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
    let updater = app
        .updater_builder()
        .timeout(CHECK_TIMEOUT)
        .configure_client(|client| {
            client
                .connect_timeout(CONNECT_TIMEOUT)
                .read_timeout(STALL_TIMEOUT)
        })
        .build()?;
    let Some(update) = updater.check().await? else {
        return Ok(());
    };
    let (update, bytes) = download(update).await?;
    let version = update.version.clone();
    *app.state::<PendingUpdate>().0.lock().unwrap() = Some((update, bytes));
    app.state::<AppMenu<R>>().show_update(app, &version);
    Ok(())
}

/// Downloads (and verifies) the update from where the manifest says, and from
/// the other host if that fails.
async fn download(update: Update) -> Result<(Update, Vec<u8>), tauri_plugin_updater::Error> {
    let first = match update.download(|_, _| {}, || {}).await {
        Ok(bytes) => return Ok((update, bytes)),
        Err(e) => e,
    };
    let Some(url) = other_host(&update.download_url) else {
        return Err(first);
    };
    eprintln!(
        "update download from {} failed ({first}); trying {url}",
        update.download_url
    );
    let mut again = update;
    again.download_url = url;
    let bytes = again.download(|_, _| {}, || {}).await?;
    Ok((again, bytes))
}

/// The same release file on the other host, if `url` is one of ours.
fn other_host(url: &Url) -> Option<Url> {
    [(MIRROR, GITHUB), (GITHUB, MIRROR)]
        .into_iter()
        .find_map(|(from, to)| {
            let rest = url
                .as_str()
                .strip_prefix(from)
                .filter(|rest| is_tag_and_name(rest))?;
            Url::parse(&format!("{to}{rest}")).ok()
        })
}

/// `<tag>/<name>` and nothing more: no query, no deeper path.
fn is_tag_and_name(rest: &str) -> bool {
    let mut parts = rest.split('/');
    matches!(
        (parts.next(), parts.next(), parts.next()),
        (Some(tag), Some(name), None) if tag.starts_with('v') && !name.is_empty() && !rest.contains(['?', '#'])
    )
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

#[cfg(test)]
mod tests {
    use super::*;

    fn url(s: &str) -> Url {
        Url::parse(s).unwrap()
    }

    #[test]
    fn the_mirror_and_github_swap_one_for_the_other() {
        let mirror = url("https://jokbet.jokerben.top/dl/v0.2.0/Jokbet_aarch64.app.tar.gz");
        let github = url("https://github.com/sparkjokerben/jokbet/releases/download/v0.2.0/Jokbet_aarch64.app.tar.gz");
        assert_eq!(other_host(&mirror), Some(github.clone()));
        assert_eq!(other_host(&github), Some(mirror));
    }

    #[test]
    fn a_swap_there_and_back_is_where_it_started() {
        let start =
            url("https://jokbet.jokerben.top/dl/v1.2.3-beta.1/Jokbet_1.2.3-beta.1_x64-setup.exe");
        let back = other_host(&other_host(&start).unwrap()).unwrap();
        assert_eq!(back, start);
    }

    #[test]
    fn anything_else_has_no_other_host() {
        for s in [
            "https://example.com/dl/v0.2.0/Jokbet_aarch64.app.tar.gz",
            "https://github.com/someone/else/releases/download/v0.2.0/Jokbet_aarch64.app.tar.gz",
            "https://jokbet.jokerben.top/dl/Jokbet_0.1.0_x64.dmg",
            "https://jokbet.jokerben.top/dl/v0.2.0/",
            "https://jokbet.jokerben.top/dl/v0.2.0/a/b",
            "https://jokbet.jokerben.top/dl/v0.2.0/Jokbet_x64.app.tar.gz?token=1",
            "https://jokbet.jokerben.top/api/update.json",
        ] {
            assert_eq!(other_host(&url(s)), None, "{s}");
        }
    }
}
