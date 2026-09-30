//! The portable Windows build: the release's `…_x64-portable.zip` holds
//! `Jokbet.exe` with a `portable.txt` beside it. With that file there, what the
//! app keeps — counts, settings, backups, logs, the webview's own files — goes
//! into a `data` folder next to the exe instead of the user's profile, and an
//! update replaces the exe where it is instead of running the installer.
//!
//! Without the file, every path here is the platform's usual one.

use std::io;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::Duration;
use tauri::{AppHandle, Manager, Runtime};

/// The file that makes a copy portable, beside the exe.
const MARKER: &str = "portable.txt";

/// The folder a portable copy keeps everything in; `None` when installed.
pub fn root() -> Option<&'static Path> {
    static ROOT: OnceLock<Option<PathBuf>> = OnceLock::new();
    ROOT.get_or_init(|| {
        if !cfg!(windows) {
            return None;
        }
        let exe = tauri::utils::platform::current_exe().ok()?;
        let beside = exe.parent()?;
        beside.join(MARKER).is_file().then(|| beside.join("data"))
    })
    .as_deref()
}

pub fn data_dir<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<PathBuf> {
    match root() {
        Some(root) => Ok(root.to_path_buf()),
        None => app.path().app_data_dir(),
    }
}

pub fn config_dir<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<PathBuf> {
    match root() {
        Some(root) => Ok(root.to_path_buf()),
        None => app.path().app_config_dir(),
    }
}

pub fn log_dir<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<PathBuf> {
    match root() {
        Some(root) => Ok(root.join("logs")),
        None => app.path().app_log_dir(),
    }
}

/// Where the webviews keep their own files, when not where WebView2 would.
pub fn webview_dir() -> Option<PathBuf> {
    root().map(|root| root.join("webview"))
}

/// The updater manifest's entry for this build: the zip, not the installer.
pub fn updater_target() -> Option<String> {
    root().map(|_| format!("windows-{}-portable", std::env::consts::ARCH))
}

/// Puts the exe out of an update's zip where this one is. A running exe cannot
/// be overwritten, but it can be renamed: this one moves aside to
/// `Jokbet.exe.old` (removed on the next start) and the new one takes its
/// name, so the restart that follows runs it.
pub fn install(zip_bytes: &[u8]) -> io::Result<()> {
    replace(&tauri::utils::platform::current_exe()?, zip_bytes)
}

fn replace(exe: &Path, zip_bytes: &[u8]) -> io::Result<()> {
    let (new, old) = (sibling(exe, "new"), sibling(exe, "old"));
    let mut archive = zip::ZipArchive::new(io::Cursor::new(zip_bytes)).map_err(io::Error::other)?;
    let name = archive
        .file_names()
        .find(|name| name.to_ascii_lowercase().ends_with(".exe"))
        .ok_or_else(|| io::Error::other("the update has no .exe in it"))?
        .to_owned();
    {
        let mut entry = archive.by_name(&name).map_err(io::Error::other)?;
        let mut out = std::fs::File::create(&new)?;
        io::copy(&mut entry, &mut out)?;
        out.sync_all()?;
    }
    let _ = std::fs::remove_file(&old);
    std::fs::rename(exe, &old)?;
    if let Err(e) = std::fs::rename(&new, exe) {
        let _ = std::fs::rename(&old, exe);
        return Err(e);
    }
    Ok(())
}

/// Removes the exe the last update moved aside. The process it belonged to
/// may still be on its way out, so this tries for a few seconds.
pub fn remove_old_exe() {
    if root().is_none() {
        return;
    }
    let Ok(exe) = tauri::utils::platform::current_exe() else {
        return;
    };
    let old = sibling(&exe, "old");
    for _ in 0..10 {
        match std::fs::remove_file(&old) {
            Err(e) if e.kind() != io::ErrorKind::NotFound => {
                std::thread::sleep(Duration::from_secs(1))
            }
            _ => return,
        }
    }
    log::warn!("could not remove {}", old.display());
}

/// `Jokbet.exe` → `Jokbet.exe.<ext>`.
fn sibling(exe: &Path, ext: &str) -> PathBuf {
    let mut name = exe.as_os_str().to_owned();
    name.push(".");
    name.push(ext);
    PathBuf::from(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn zip_of(files: &[(&str, &[u8])]) -> Vec<u8> {
        let mut out = zip::ZipWriter::new(io::Cursor::new(Vec::new()));
        for (name, body) in files {
            out.start_file(*name, zip::write::SimpleFileOptions::default())
                .unwrap();
            out.write_all(body).unwrap();
        }
        out.finish().unwrap().into_inner()
    }

    fn scratch(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("jokbet-portable-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn the_exe_in_the_zip_takes_the_old_ones_place() {
        let dir = scratch("replace");
        let exe = dir.join("Jokbet.exe");
        std::fs::write(&exe, b"old").unwrap();
        let zip = zip_of(&[
            ("Jokbet/portable.txt", b"marker"),
            ("Jokbet/Jokbet.exe", b"new"),
        ]);
        replace(&exe, &zip).unwrap();
        assert_eq!(std::fs::read(&exe).unwrap(), b"new");
        assert_eq!(std::fs::read(dir.join("Jokbet.exe.old")).unwrap(), b"old");
        assert!(!dir.join("Jokbet.exe.new").exists());
    }

    #[test]
    fn a_zip_without_an_exe_leaves_the_old_one_alone() {
        let dir = scratch("no-exe");
        let exe = dir.join("Jokbet.exe");
        std::fs::write(&exe, b"old").unwrap();
        let zip = zip_of(&[("Jokbet/portable.txt", b"marker")]);
        assert!(replace(&exe, &zip).is_err());
        assert_eq!(std::fs::read(&exe).unwrap(), b"old");
    }
}
