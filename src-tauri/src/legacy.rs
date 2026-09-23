//! Until the project was renamed, the app called itself
//! `io.github.sparkjokerben.jokerben-desktop-pet`. Both Tauri and macOS key the
//! data and config directories by that identifier, so the rename on its own
//! would have started every install from zero — counters, settings and all.
//!
//! The first run under the new name therefore copies the old files across,
//! one at a time and only where the new copy does not exist yet. Nothing is
//! deleted or overwritten, and running it again does nothing.

use std::path::{Path, PathBuf};

/// The identifier the app used before it was renamed to `jokbet`.
const LEGACY_IDENTIFIER: &str = "io.github.sparkjokerben.jokerben-desktop-pet";

/// The database and the sidecars SQLite's WAL mode keeps beside it.
const DATA_FILES: [&str; 3] = ["stats.sqlite", "stats.sqlite-wal", "stats.sqlite-shm"];
const CONFIG_FILES: [&str; 1] = ["settings.json"];

/// Brings the pre-rename files over; returns the paths it wrote.
pub fn adopt(data_dir: &Path, config_dir: &Path) -> Vec<PathBuf> {
    let mut adopted = Vec::new();
    let groups: [(&Path, &[&str]); 2] = [(data_dir, &DATA_FILES), (config_dir, &CONFIG_FILES)];
    for (dir, files) in groups {
        let Some(legacy) = dir.parent().map(|parent| parent.join(LEGACY_IDENTIFIER)) else {
            continue;
        };
        for name in files {
            let from = legacy.join(name);
            let to = dir.join(name);
            if to.exists() || !from.is_file() {
                continue;
            }
            if std::fs::create_dir_all(dir).is_ok() && std::fs::copy(&from, &to).is_ok() {
                adopted.push(to);
            }
        }
    }
    adopted
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The two directories Tauri hands out are `<base>/<identifier>`; on macOS
    /// and Windows the data and config ones are the same directory, so the test
    /// uses one for both, the way the app sees it there.
    fn pair(name: &str) -> (PathBuf, PathBuf) {
        let root =
            std::env::temp_dir().join(format!("jokbet-legacy-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let old = root.join(LEGACY_IDENTIFIER);
        std::fs::create_dir_all(&old).unwrap();
        (old, root.join("io.github.sparkjokerben.jokbet"))
    }

    #[test]
    fn carries_the_files_of_the_old_name_over() {
        let (old, new) = pair("adopt");
        std::fs::write(old.join("stats.sqlite"), b"counts").unwrap();
        std::fs::write(old.join("stats.sqlite-wal"), b"wal").unwrap();
        std::fs::write(old.join("settings.json"), b"{}").unwrap();

        let adopted = adopt(&new, &new);

        assert_eq!(adopted.len(), 3);
        assert_eq!(std::fs::read(new.join("stats.sqlite")).unwrap(), b"counts");
        assert_eq!(std::fs::read(new.join("stats.sqlite-wal")).unwrap(), b"wal");
        assert_eq!(std::fs::read(new.join("settings.json")).unwrap(), b"{}");
        assert!(old.exists(), "the old directory is left alone");
    }

    #[test]
    fn never_overwrites_what_is_already_there() {
        let (old, new) = pair("keep");
        std::fs::write(old.join("settings.json"), b"old").unwrap();
        std::fs::create_dir_all(&new).unwrap();
        std::fs::write(new.join("settings.json"), b"current").unwrap();

        assert!(adopt(&new, &new).is_empty());
        assert_eq!(
            std::fs::read(new.join("settings.json")).unwrap(),
            b"current"
        );
    }

    #[test]
    fn does_nothing_when_there_is_nothing_to_adopt() {
        let (_old, new) = pair("empty");
        assert!(adopt(&new, &new).is_empty());
        assert!(!new.exists(), "no empty directory is left behind");
    }

    #[test]
    fn an_install_that_never_had_the_old_name_is_untouched() {
        // Data and config in different places, as on Linux, with only one of
        // them holding anything from the old app.
        let (old, new) = pair("split");
        std::fs::write(old.join("stats.sqlite"), b"counts").unwrap();
        let config = new.with_extension("config");

        let adopted = adopt(&new, &config);

        assert_eq!(adopted, vec![new.join("stats.sqlite")]);
        assert!(
            !config.exists(),
            "the config directory is only made when used"
        );
    }
}
