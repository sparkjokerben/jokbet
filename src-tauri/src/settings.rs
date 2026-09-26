//! Typed user settings persisted as `settings.json` in the app config dir.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Pixels per sprite cell: the pet's body is 24 cells wide. The window is
/// sized from this, so the slider's range is what keeps it sane.
pub const PET_SCALE_MIN: f64 = 3.0;
pub const PET_SCALE_MAX: f64 = 7.0;
pub const PET_SCALE_DEFAULT: f64 = 3.5;
/// The tint slider's range, in percent of the card colour.
pub const GLASS_TINT_MAX: u32 = 60;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum CounterKind {
    /// Today's total.
    #[default]
    Today,
    /// Per-second rate over the last few seconds.
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

impl HeadCounter {
    /// What the count adds up: key presses, clicks, or both (also when
    /// neither is picked, which hides the counter but still leaves a count).
    pub fn metric(&self) -> Metric {
        match (self.keyboard, self.mouse) {
            (true, false) => Metric::Keys,
            (false, true) => Metric::Clicks,
            _ => Metric::Inputs,
        }
    }
}

impl Default for HeadCounter {
    fn default() -> Self {
        Self {
            enabled: true,
            kind: CounterKind::Today,
            keyboard: true,
            mouse: true,
        }
    }
}

/// The language of the interface.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum Language {
    /// Whatever the system is set to (Chinese for any Chinese locale, else English).
    #[default]
    System,
    Zh,
    En,
}

/// Global shortcuts, as accelerators like `Control+Alt+KeyJ`; `None` is unset.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct Shortcuts {
    pub toggle_pet: Option<String>,
    pub pause: Option<String>,
}

/// The first part of the error a shortcut in use by another app gives, which
/// the settings window tells apart from other errors.
pub const SHORTCUT_TAKEN: &str = "shortcut-taken";
/// Likewise for the same combination given to both actions.
pub const SHORTCUT_DUPLICATE: &str = "shortcut-duplicate";

/// Parses an accelerator the settings hold.
pub fn parse_shortcut(accelerator: &str) -> Result<tauri_plugin_global_shortcut::Shortcut, String> {
    accelerator
        .parse()
        .map_err(|e| format!("shortcut {accelerator}: {e}"))
}

/// The key heatmap's colours.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum HeatmapPalette {
    /// Yellow to red on light, dark red to yellow on dark: the usual heat scale.
    Heat,
    /// The pet's orange, in one hue.
    #[default]
    Brand,
}

/// What the pet does while nothing else is going on.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum IdleAnim {
    Breathe,
    #[default]
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
    /// Key presses and clicks together.
    Inputs,
    Scrolls,
    /// Metres of mouse travel.
    Distance,
}

impl Metric {
    /// Smallest allowed step for a repeating milestone.
    pub fn min_repeat_step(self) -> f64 {
        match self {
            Metric::Keys | Metric::Clicks | Metric::Inputs => 500.0,
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
    /// Pixels per sprite cell, in `PET_SCALE_MIN..=PET_SCALE_MAX`.
    pub pet_scale: f64,
    /// Top-left of the pet window in physical pixels; `None` means default placement.
    pub pet_position: Option<[i32; 2]>,
    pub head_counter: HeadCounter,
    pub idle_anim: IdleAnim,
    pub click_anim: ActionAnim,
    pub double_click_anim: ActionAnim,
    /// Show today's stats in a bubble while hovering the pet.
    pub bubble: bool,
    /// Draw the hover bubble with the system's glass material, where there is one.
    pub liquid_glass: bool,
    /// How much the page tints that material, in percent: 0 is the bare
    /// material, which is what the material is for, and the maximum is enough
    /// to read as a solid card.
    pub glass_tint: u32,
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
    pub language: Language,
    /// Take the pet off screen while another app is full screen or presenting.
    pub hide_in_fullscreen: bool,
    pub shortcuts: Shortcuts,
    pub heatmap_palette: HeatmapPalette,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            pet_scale: PET_SCALE_DEFAULT,
            pet_position: None,
            head_counter: HeadCounter::default(),
            idle_anim: IdleAnim::Soccer,
            click_anim: ActionAnim::Wave,
            double_click_anim: ActionAnim::Hearts,
            bubble: true,
            liquid_glass: false,
            glass_tint: 0,
            typing_speed: true,
            milestones: true,
            custom_milestones: Vec::new(),
            sleep_after_min: 1,
            paused: false,
            onboarded: false,
            language: Language::System,
            hide_in_fullscreen: true,
            shortcuts: Shortcuts::default(),
            heatmap_palette: HeatmapPalette::Brand,
        }
    }
}

impl Settings {
    /// Rejects values the UI should never produce.
    pub fn validate(&self) -> Result<(), String> {
        if !(1..=120).contains(&self.sleep_after_min) {
            return Err("sleepAfterMin must be between 1 and 120".into());
        }
        if self.glass_tint > GLASS_TINT_MAX {
            return Err(format!("glassTint must be at most {GLASS_TINT_MAX}"));
        }
        if !self.pet_scale.is_finite() || !(PET_SCALE_MIN..=PET_SCALE_MAX).contains(&self.pet_scale)
        {
            return Err(format!(
                "petScale must be between {PET_SCALE_MIN} and {PET_SCALE_MAX}"
            ));
        }
        let shortcuts = [&self.shortcuts.toggle_pet, &self.shortcuts.pause];
        let parsed = shortcuts
            .into_iter()
            .flatten()
            .map(|a| parse_shortcut(a).map(|s| s.id()))
            .collect::<Result<Vec<_>, _>>()?;
        if parsed.len() == 2 && parsed[0] == parsed[1] {
            return Err(format!(
                "{SHORTCUT_DUPLICATE}: both actions have the same shortcut"
            ));
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

/// Carries settings from older versions over to the current shape.
fn migrate(value: &mut serde_json::Value) {
    let Some(obj) = value.as_object_mut() else {
        return;
    };
    // Until 0.0.1 the size was one of three steps; keep the closest scale.
    if let Some(size) = obj.remove("petSize") {
        if !obj.contains_key("petScale") {
            let scale = match size.as_str() {
                Some("small") => 4.0,
                Some("large") => 8.0,
                _ => 6.0,
            };
            obj.insert("petScale".into(), serde_json::json!(scale));
        }
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
    /// Loads settings, falling back to defaults when the file is missing or
    /// unreadable. A file that is damaged, or holds values this version
    /// rejects, keeps what it can: the rest goes back to the defaults, and the
    /// original is set aside next to it for whoever wants to look.
    pub fn load(path: PathBuf) -> Self {
        let settings = match std::fs::read(&path) {
            Ok(bytes) => {
                let (settings, dropped) = parse(&bytes);
                if !dropped.is_empty() {
                    set_aside(&path, &settings, &dropped);
                }
                settings
            }
            Err(_) => Settings::default(),
        };
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

    /// What a JSON merge patch would make of the settings, validated, without
    /// saving it.
    pub fn preview(&self, patch: &serde_json::Value) -> Result<Settings, String> {
        let current = self.get();
        patched(&current, patch)
    }

    /// Applies a JSON merge patch from the UI after validating the result.
    pub fn patch(&self, patch: &serde_json::Value) -> Result<Settings, String> {
        let mut guard = self.inner.lock().unwrap();
        let next = patched(&guard, patch)?;
        let bytes = serde_json::to_vec_pretty(&next).map_err(|e| e.to_string())?;
        write_atomic(&self.path, &bytes).map_err(|e| e.to_string())?;
        *guard = next.clone();
        Ok(next)
    }
}

fn patched(current: &Settings, patch: &serde_json::Value) -> Result<Settings, String> {
    let mut value = serde_json::to_value(current).map_err(|e| e.to_string())?;
    merge_patch(&mut value, patch);
    let next: Settings = serde_json::from_value(value).map_err(|e| e.to_string())?;
    next.validate()?;
    Ok(next)
}

/// What the file says, and the top-level fields that had to be dropped
/// (`"*"` when the file is not a settings object at all).
fn parse(bytes: &[u8]) -> (Settings, Vec<String>) {
    use serde_json::Value;
    let Ok(mut value) = serde_json::from_slice::<Value>(bytes) else {
        return (Settings::default(), vec!["*".into()]);
    };
    migrate(&mut value);
    let Value::Object(fields) = value else {
        return (Settings::default(), vec!["*".into()]);
    };
    if let Ok(settings) = serde_json::from_value::<Settings>(Value::Object(fields.clone())) {
        if settings.validate().is_ok() {
            return (settings, Vec::new());
        }
    }
    // Something in it is wrong: take the fields one at a time, and keep each
    // only if the whole still reads and validates with it.
    let mut good = serde_json::to_value(Settings::default()).unwrap_or(Value::Null);
    let mut dropped = Vec::new();
    for (key, field) in fields {
        let mut next = good.clone();
        next[&key] = field;
        match serde_json::from_value::<Settings>(next.clone()) {
            Ok(settings) if settings.validate().is_ok() => good = next,
            _ => dropped.push(key),
        }
    }
    (serde_json::from_value(good).unwrap_or_default(), dropped)
}

/// Keeps a copy of a damaged file as `settings.bad-<time>.json`, and writes
/// what was salvaged in its place so the next start reads the same.
fn set_aside(path: &Path, salvaged: &Settings, dropped: &[String]) {
    let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
    let backup = path.with_file_name(format!("settings.bad-{stamp}.json"));
    match std::fs::copy(path, &backup) {
        Ok(_) => log::warn!(
            "settings: dropped {} and kept the original as {}",
            dropped.join(", "),
            backup.display()
        ),
        Err(e) => log::warn!(
            "settings: dropped {}; keeping a copy failed: {e}",
            dropped.join(", ")
        ),
    }
    let written = serde_json::to_vec_pretty(salvaged)
        .map_err(std::io::Error::other)
        .and_then(|bytes| write_atomic(path, &bytes));
    if let Err(e) = written {
        log::error!("settings: writing the salvaged file failed: {e}");
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
                s.pet_scale = 6.5;
                s.pet_position = Some([10, -20]);
            })
            .unwrap();
        let reloaded = SettingsStore::load(path);
        assert_eq!(reloaded.get().pet_scale, 6.5);
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

    /// The defaults a fresh install starts from, as chosen in the settings UI.
    #[test]
    fn a_fresh_install_gets_the_shipped_defaults() {
        let s = Settings::default();
        assert_eq!(s.pet_scale, 3.5);
        assert!(s.head_counter.keyboard && s.head_counter.mouse);
        assert_eq!(s.head_counter.kind, CounterKind::Today);
        assert_eq!(s.idle_anim, IdleAnim::Soccer);
        assert_eq!(s.click_anim, ActionAnim::Wave);
        assert_eq!(s.double_click_anim, ActionAnim::Hearts);
        assert_eq!(s.sleep_after_min, 1);
        assert!(!s.liquid_glass, "the material starts switched off");
        assert_eq!(s.language, Language::System);
        assert!(s.hide_in_fullscreen);
        assert_eq!(s.heatmap_palette, HeatmapPalette::Brand);
    }

    #[test]
    fn pet_scale_outside_the_slider_is_rejected() {
        let at = |pet_scale: f64| Settings {
            pet_scale,
            ..Default::default()
        };
        assert!(at(PET_SCALE_MIN - 0.1).validate().is_err());
        assert!(at(f64::NAN).validate().is_err());
        assert!(at(PET_SCALE_MIN).validate().is_ok());
        assert!(at(PET_SCALE_MAX).validate().is_ok());
    }

    #[test]
    fn glass_tint_stays_within_its_slider() {
        let at = |glass_tint: u32| Settings {
            glass_tint,
            ..Default::default()
        };
        assert!(at(0).validate().is_ok());
        assert!(at(GLASS_TINT_MAX).validate().is_ok());
        assert!(at(GLASS_TINT_MAX + 1).validate().is_err());
    }

    #[test]
    fn the_scale_is_kept_when_the_file_already_has_one() {
        let path = temp_path("both");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, br#"{"petSize":"large","petScale":3.5}"#).unwrap();
        assert_eq!(SettingsStore::load(path).get().pet_scale, 3.5);
    }

    #[test]
    fn shortcuts_are_accelerators_the_settings_window_writes() {
        for a in [
            "Control+Alt+KeyJ",
            "Super+Shift+Digit1",
            "Alt+F13",
            "Control+Numpad5",
            "Super+ArrowUp",
            "Control+Alt+Shift+Super+Backquote",
            "Control+NumpadAdd",
            "Alt+PageUp",
        ] {
            assert!(parse_shortcut(a).is_ok(), "{a}");
        }
        assert!(parse_shortcut("KeyJ+Control").is_err());
        assert!(parse_shortcut("Control+Moonwalk").is_err());
    }

    #[test]
    fn shortcuts_are_validated_and_must_differ() {
        let store = SettingsStore::load(temp_path("shortcuts"));
        let next = store
            .patch(&serde_json::json!({"shortcuts": {"togglePet": "Control+Alt+KeyJ"}}))
            .unwrap();
        assert_eq!(
            next.shortcuts.toggle_pet.as_deref(),
            Some("Control+Alt+KeyJ")
        );
        assert_eq!(next.shortcuts.pause, None);
        let same = store
            .patch(&serde_json::json!({"shortcuts": {"pause": "Alt+Control+KeyJ"}}))
            .unwrap_err();
        assert!(same.starts_with(SHORTCUT_DUPLICATE), "{same}");
        assert!(store
            .patch(&serde_json::json!({"shortcuts": {"pause": "Control+Nope"}}))
            .is_err());
        let cleared = store
            .patch(&serde_json::json!({"shortcuts": {"togglePet": null}}))
            .unwrap();
        assert_eq!(cleared.shortcuts, Shortcuts::default());
    }

    #[test]
    fn a_preview_saves_nothing() {
        let path = temp_path("preview");
        let store = SettingsStore::load(path.clone());
        let next = store
            .preview(&serde_json::json!({"bubble": false}))
            .unwrap();
        assert!(!next.bubble);
        assert!(store.get().bubble);
        assert!(!path.exists());
    }

    fn backups(path: &Path) -> usize {
        std::fs::read_dir(path.parent().unwrap())
            .unwrap()
            .filter(|e| {
                e.as_ref()
                    .unwrap()
                    .file_name()
                    .to_string_lossy()
                    .starts_with("settings.bad-")
            })
            .count()
    }

    fn load_file(name: &str, contents: &[u8]) -> (SettingsStore, PathBuf) {
        let path = temp_path(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, contents).unwrap();
        (SettingsStore::load(path.clone()), path)
    }

    #[test]
    fn a_damaged_file_is_set_aside_for_the_defaults() {
        let (store, path) = load_file("garbage", b"{\"petScale\": 4,");
        assert_eq!(store.get(), Settings::default());
        assert_eq!(backups(&path), 1);
    }

    #[test]
    fn one_bad_value_loses_only_that_field() {
        let (store, path) = load_file(
            "badenum",
            br#"{"clickAnim":"moonwalk","petScale":5,"bubble":false}"#,
        );
        let s = store.get();
        assert_eq!(s.click_anim, Settings::default().click_anim);
        assert_eq!(s.pet_scale, 5.0);
        assert!(!s.bubble);
        assert_eq!(backups(&path), 1);
        // What was salvaged is what the next start reads.
        assert_eq!(SettingsStore::load(path).get(), s);
    }

    #[test]
    fn an_out_of_range_value_is_dropped_and_patches_work_again() {
        let (store, _) = load_file("range", br#"{"sleepAfterMin":500,"petScale":6}"#);
        assert_eq!(
            store.get().sleep_after_min,
            Settings::default().sleep_after_min
        );
        assert_eq!(store.get().pet_scale, 6.0);
        assert!(store.patch(&serde_json::json!({"bubble": false})).is_ok());
    }

    #[test]
    fn a_sound_file_is_not_set_aside() {
        let (store, path) = load_file("sound", br#"{"petScale":5}"#);
        assert_eq!(store.get().pet_scale, 5.0);
        assert_eq!(backups(&path), 0);
    }

    #[test]
    fn unknown_and_missing_fields_fall_back_to_defaults() {
        let path = temp_path("partial");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, br#"{"petSize":"small","future":1}"#).unwrap();
        let store = SettingsStore::load(path);
        // The old three-step size is carried over to the closest scale.
        assert_eq!(store.get().pet_scale, 4.0);
        assert_eq!(store.get().pet_position, None);
    }
}
