//! Typed user settings persisted as `settings.json` in the app config dir.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum PetSize {
    Small,
    #[default]
    Medium,
    Large,
}

impl PetSize {
    /// Pixels per sprite cell; Clawd's body is 16 cells wide (64/96/128 px).
    pub fn scale(self) -> f64 {
        match self {
            PetSize::Small => 4.0,
            PetSize::Medium => 6.0,
            PetSize::Large => 8.0,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    pub pet_size: PetSize,
    /// Top-left of the pet window in physical pixels; `None` means default placement.
    pub pet_position: Option<[i32; 2]>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            pet_size: PetSize::Medium,
            pet_position: None,
        }
    }
}

pub struct SettingsStore {
    path: PathBuf,
    inner: Mutex<Settings>,
}

impl SettingsStore {
    /// Loads settings, falling back to defaults when the file is missing or unreadable.
    pub fn load(path: PathBuf) -> Self {
        let settings = std::fs::read(&path)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default();
        Self {
            path,
            inner: Mutex::new(settings),
        }
    }

    pub fn get(&self) -> Settings {
        self.inner.lock().unwrap().clone()
    }

    /// Applies `f` and persists the result; returns the new settings.
    pub fn update(&self, f: impl FnOnce(&mut Settings)) -> std::io::Result<Settings> {
        let mut guard = self.inner.lock().unwrap();
        f(&mut guard);
        write_atomic(&self.path, &serde_json::to_vec_pretty(&*guard)?)?;
        Ok(guard.clone())
    }
}

fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(tmp, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("jdp-settings-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir.join("settings.json")
    }

    #[test]
    fn missing_file_gives_defaults() {
        let store = SettingsStore::load(temp_path("missing"));
        assert_eq!(store.get(), Settings::default());
    }

    #[test]
    fn update_round_trips_through_disk() {
        let path = temp_path("roundtrip");
        let store = SettingsStore::load(path.clone());
        store
            .update(|s| {
                s.pet_size = PetSize::Large;
                s.pet_position = Some([10, -20]);
            })
            .unwrap();
        let reloaded = SettingsStore::load(path);
        assert_eq!(reloaded.get().pet_size, PetSize::Large);
        assert_eq!(reloaded.get().pet_position, Some([10, -20]));
    }

    #[test]
    fn unknown_and_missing_fields_fall_back_to_defaults() {
        let path = temp_path("partial");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, br#"{"petSize":"small","future":1}"#).unwrap();
        let store = SettingsStore::load(path);
        assert_eq!(store.get().pet_size, PetSize::Small);
        assert_eq!(store.get().pet_position, None);
    }
}
