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
    /// Pixels per sprite cell; Jokbet's body is 16 cells wide (64/96/128 px).
    pub fn scale(self) -> f64 {
        match self {
            PetSize::Small => 4.0,
            PetSize::Medium => 6.0,
            PetSize::Large => 8.0,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum CounterKind {
    /// Today's total.
    #[default]
    Today,
    /// Per-minute rate over the last few seconds.
    Rate,
}

/// The single number above the pet's head.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct HeadCounter {
    pub enabled: bool,
    pub kind: CounterKind,
    /// Include key presses.
    pub keyboard: bool,
    /// Include mouse clicks; with `keyboard` too, the two are summed.
    pub mouse: bool,
}

impl Default for HeadCounter {
    fn default() -> Self {
        Self {
            enabled: true,
            kind: CounterKind::Today,
            keyboard: true,
            mouse: false,
        }
    }
}

/// What the pet does while nothing else is going on.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum IdleAnim {
    #[default]
    Breathe,
    Soccer,
    LookAround,
}

/// A one-off reaction the user can assign to a click or a double click.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ActionAnim {
    Poke,
    Hearts,
    Soccer,
    Wave,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Period {
    Daily,
    Lifetime,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Metric {
    Keys,
    Clicks,
    Scrolls,
    /// Metres of mouse travel.
    Distance,
}

impl Metric {
    /// Smallest allowed step for a repeating milestone.
    pub fn min_repeat_step(self) -> f64 {
        match self {
            Metric::Keys | Metric::Clicks => 500.0,
            Metric::Scrolls => 200.0,
            Metric::Distance => 50.0,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CustomMilestone {
    pub id: String,
    pub period: Period,
    pub metric: Metric,
    pub threshold: f64,
    /// Celebrate every `threshold` instead of once.
    pub repeat: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    pub pet_size: PetSize,
    /// Top-left of the pet window in physical pixels; `None` means default placement.
    pub pet_position: Option<[i32; 2]>,
    pub head_counter: HeadCounter,
    pub idle_anim: IdleAnim,
    pub click_anim: ActionAnim,
    pub double_click_anim: ActionAnim,
    /// Show today's stats in a bubble while hovering the pet.
    pub bubble: bool,
    /// Live typing speed: shown in the bubble and drives the typing animation.
    pub typing_speed: bool,
    pub milestones: bool,
    pub custom_milestones: Vec<CustomMilestone>,
    /// Minutes without input before the pet falls asleep.
    pub sleep_after_min: u32,
    /// Counting is paused (the pet still reacts).
    pub paused: bool,
    /// The first-run onboarding has been completed.
    pub onboarded: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            pet_size: PetSize::Medium,
            pet_position: None,
            head_counter: HeadCounter::default(),
            idle_anim: IdleAnim::Breathe,
            click_anim: ActionAnim::Poke,
            double_click_anim: ActionAnim::Hearts,
            bubble: true,
            typing_speed: true,
            milestones: true,
            custom_milestones: Vec::new(),
            sleep_after_min: 5,
            paused: false,
            onboarded: false,
        }
    }
}

impl Settings {
    /// Rejects values the UI should never produce.
    pub fn validate(&self) -> Result<(), String> {
        if !(1..=120).contains(&self.sleep_after_min) {
            return Err("sleepAfterMin must be between 1 and 120".into());
        }
        for m in &self.custom_milestones {
            if !(m.threshold.is_finite() && m.threshold > 0.0) {
                return Err(format!("milestone {}: threshold must be positive", m.id));
            }
            if m.repeat && m.threshold < m.metric.min_repeat_step() {
                return Err(format!(
                    "milestone {}: repeating step must be at least {}",
                    m.id,
                    m.metric.min_repeat_step()
                ));
            }
        }
        Ok(())
    }
}

/// RFC 7396 JSON merge patch: objects merge recursively, `null` resets a
/// field to its default, anything else replaces.
pub fn merge_patch(target: &mut serde_json::Value, patch: &serde_json::Value) {
    use serde_json::Value;
    match (target, patch) {
        (Value::Object(t), Value::Object(p)) => {
            for (k, v) in p {
                if v.is_null() {
                    t.remove(k);
                } else {
                    merge_patch(t.entry(k.clone()).or_insert(Value::Null), v);
                }
            }
        }
        (t, p) => *t = p.clone(),
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

    /// Applies a JSON merge patch from the UI after validating the result.
    pub fn patch(&self, patch: &serde_json::Value) -> Result<Settings, String> {
        let mut guard = self.inner.lock().unwrap();
        let mut value = serde_json::to_value(&*guard).map_err(|e| e.to_string())?;
        merge_patch(&mut value, patch);
        let next: Settings = serde_json::from_value(value).map_err(|e| e.to_string())?;
        next.validate()?;
        let bytes = serde_json::to_vec_pretty(&next).map_err(|e| e.to_string())?;
        write_atomic(&self.path, &bytes).map_err(|e| e.to_string())?;
        *guard = next.clone();
        Ok(next)
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
    fn patch_merges_nested_fields_and_validates() {
        let store = SettingsStore::load(temp_path("patch"));
        let next = store
            .patch(&serde_json::json!({"headCounter": {"mouse": true, "kind": "rate"}, "sleepAfterMin": 10}))
            .unwrap();
        assert!(next.head_counter.keyboard && next.head_counter.mouse);
        assert_eq!(next.head_counter.kind, CounterKind::Rate);
        assert_eq!(next.sleep_after_min, 10);

        assert!(store
            .patch(&serde_json::json!({"sleepAfterMin": 0}))
            .is_err());
        assert_eq!(
            store.get().sleep_after_min,
            10,
            "rejected patch leaves settings untouched"
        );
    }

    #[test]
    fn animation_choices_round_trip() {
        let store = SettingsStore::load(temp_path("anims"));
        let next = store
            .patch(&serde_json::json!({"idleAnim": "soccer", "clickAnim": "wave", "doubleClickAnim": "soccer"}))
            .unwrap();
        assert_eq!(next.idle_anim, IdleAnim::Soccer);
        assert_eq!(next.click_anim, ActionAnim::Wave);
        assert_eq!(next.double_click_anim, ActionAnim::Soccer);
        assert!(store
            .patch(&serde_json::json!({"clickAnim": "moonwalk"}))
            .is_err());
    }

    #[test]
    fn null_resets_a_field() {
        let store = SettingsStore::load(temp_path("null"));
        store
            .patch(&serde_json::json!({"petPosition": [5, 6]}))
            .unwrap();
        let next = store
            .patch(&serde_json::json!({"petPosition": null}))
            .unwrap();
        assert_eq!(next.pet_position, None);
    }

    #[test]
    fn repeating_milestones_need_a_minimum_step() {
        let mut s = Settings::default();
        s.custom_milestones.push(CustomMilestone {
            id: "m1".into(),
            period: Period::Daily,
            metric: Metric::Keys,
            threshold: 100.0,
            repeat: true,
        });
        assert!(s.validate().is_err());
        s.custom_milestones[0].threshold = 500.0;
        assert!(s.validate().is_ok());
        s.custom_milestones[0].repeat = false;
        s.custom_milestones[0].threshold = 100.0;
        assert!(s.validate().is_ok());
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
