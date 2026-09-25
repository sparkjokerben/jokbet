//! One menu shared by the tray icon and the pet's right-click menu.

use crate::i18n::{t, Lang, Text};
use crate::panels::{self, Panel};
use crate::pet_window::PET_LABEL;
use crate::settings::SettingsStore;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use tauri::image::Image;
use tauri::menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager, Runtime};

const ID_TOGGLE: &str = "toggle-pet";
const ID_STATS: &str = "stats";
const ID_SETTINGS: &str = "settings";
const ID_PAUSE: &str = "pause";
const ID_RESET_POSITION: &str = "reset-position";
const ID_QUIT: &str = "quit";

pub struct AppMenu<R: Runtime> {
    pub menu: Menu<R>,
    stats: MenuItem<R>,
    settings: MenuItem<R>,
    toggle: MenuItem<R>,
    reset_position: MenuItem<R>,
    pause: CheckMenuItem<R>,
    quit: MenuItem<R>,
    lang: Mutex<Lang>,
    /// What the toggle item says: "Hide" while the pet is shown.
    pet_shown: AtomicBool,
}

impl<R: Runtime> AppMenu<R> {
    pub fn build(app: &AppHandle<R>) -> tauri::Result<Self> {
        let current = app.state::<SettingsStore>().get();
        let lang = Lang::resolve(current.language);
        let item =
            |id: &str, text: Text| MenuItem::with_id(app, id, t(lang, text), true, None::<&str>);
        let toggle = item(ID_TOGGLE, Text::HidePet)?;
        let reset_position = item(ID_RESET_POSITION, Text::ResetPosition)?;
        let stats = item(ID_STATS, Text::Stats)?;
        let settings = item(ID_SETTINGS, Text::Settings)?;
        let pause = CheckMenuItem::with_id(
            app,
            ID_PAUSE,
            t(lang, Text::PauseCounting),
            true,
            current.paused,
            None::<&str>,
        )?;
        let quit = item(ID_QUIT, Text::Quit)?;
        let menu = Menu::with_items(
            app,
            &[
                &stats,
                &settings,
                &PredefinedMenuItem::separator(app)?,
                &toggle,
                &reset_position,
                &pause,
                &PredefinedMenuItem::separator(app)?,
                &quit,
            ],
        )?;
        Ok(Self {
            menu,
            stats,
            settings,
            toggle,
            reset_position,
            pause,
            quit,
            lang: Mutex::new(lang),
            pet_shown: AtomicBool::new(true),
        })
    }

    pub fn lang(&self) -> Lang {
        *self.lang.lock().unwrap()
    }

    /// Says everything again in `lang`.
    pub fn set_lang(&self, lang: Lang) {
        *self.lang.lock().unwrap() = lang;
        let _ = self.stats.set_text(t(lang, Text::Stats));
        let _ = self.settings.set_text(t(lang, Text::Settings));
        let _ = self.reset_position.set_text(t(lang, Text::ResetPosition));
        let _ = self.pause.set_text(t(lang, Text::PauseCounting));
        let _ = self.quit.set_text(t(lang, Text::Quit));
        self.sync_toggle(self.pet_shown.load(Ordering::Relaxed));
    }

    pub fn sync_paused(&self, paused: bool) {
        let _ = self.pause.set_checked(paused);
    }

    pub fn sync_toggle(&self, visible: bool) {
        self.pet_shown.store(visible, Ordering::Relaxed);
        let text = if visible {
            Text::HidePet
        } else {
            Text::ShowPet
        };
        let _ = self.toggle.set_text(t(self.lang(), text));
    }
}

/// Where a menu change has to happen.
///
/// Items are retitled, ticked and shown from whichever thread noticed the
/// change — the updater, the full-screen watcher — and AppKit takes a change to
/// a menu it is currently displaying as a reason to abort the process. Nothing
/// here adds to or removes from `menu` any more: it is built once, so the only
/// changes left are to items that are already in it, and they happen on the
/// main thread.
pub fn on_main<R: Runtime>(app: &AppHandle<R>, change: impl FnOnce(&AppMenu<R>) + Send + 'static) {
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        if let Some(menu) = handle.try_state::<AppMenu<R>>() {
            change(&menu);
        }
    });
}

pub fn create_tray<R: Runtime>(app: &AppHandle<R>, menu: &AppMenu<R>) -> tauri::Result<()> {
    // macOS tints a black template image to match the menu bar; elsewhere use color.
    #[cfg(target_os = "macos")]
    let icon = Image::from_bytes(include_bytes!("../icons/source/tray-template.png"))?;
    #[cfg(not(target_os = "macos"))]
    let icon = Image::from_bytes(include_bytes!("../icons/source/tray-color.png"))?;

    TrayIconBuilder::with_id("main")
        .icon(icon)
        .icon_as_template(cfg!(target_os = "macos"))
        .tooltip("Jokbet")
        .menu(&menu.menu)
        .show_menu_on_left_click(true)
        .build(app)?;
    Ok(())
}

pub fn handle_event<R: Runtime>(app: &AppHandle<R>, event: MenuEvent) {
    match event.id().as_ref() {
        ID_TOGGLE => crate::visibility::toggle(app),
        ID_RESET_POSITION => {
            if let Some(window) = app.get_webview_window(PET_LABEL) {
                if let Err(e) = crate::pet_window::reset_position(&window) {
                    log::error!("moving the pet back failed: {e}");
                }
            }
            crate::visibility::show(app);
        }
        ID_STATS => open_panel(app, Panel::Stats),
        ID_SETTINGS => open_panel(app, Panel::Settings),
        ID_PAUSE => toggle_pause(app),
        ID_QUIT => {
            crate::updater::install_pending(app);
            app.exit(0);
        }
        _ => {}
    }
}

/// Pauses counting, or resumes it.
pub fn toggle_pause<R: Runtime>(app: &AppHandle<R>) {
    let paused = !app.state::<SettingsStore>().get().paused;
    if let Err(e) = crate::commands::apply_patch(app, &serde_json::json!({ "paused": paused })) {
        log::error!("pausing failed: {e}");
    }
}

pub fn open_panel<R: Runtime>(app: &AppHandle<R>, panel: Panel) {
    let lang = app.state::<AppMenu<R>>().lang();
    if let Err(e) = panels::open(app, panel, &panel_title(lang, panel)) {
        log::error!("opening {panel:?} failed: {e}");
    }
}

/// A panel's window title: the menu item that opens it, without the ellipsis.
pub fn panel_title(lang: Lang, panel: Panel) -> String {
    match panel {
        Panel::Stats => t(lang, Text::Stats),
        Panel::Settings => t(lang, Text::Settings),
        Panel::Onboarding | Panel::Tour => "Jokbet",
    }
    .trim_end_matches('…')
    .to_string()
}

/// Retitles whichever panels are open, after the language changed.
pub fn retitle_panels<R: Runtime>(app: &AppHandle<R>, lang: Lang) {
    for panel in [Panel::Stats, Panel::Settings] {
        if let Some(window) = app.get_webview_window(panel.label()) {
            let _ = window.set_title(&panel_title(lang, panel));
        }
    }
}
