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
const ID_RESTART_UPDATE: &str = "restart-update";
const ID_QUIT: &str = "quit";

pub struct AppMenu<R: Runtime> {
    pub menu: Menu<R>,
    stats: MenuItem<R>,
    settings: MenuItem<R>,
    toggle: MenuItem<R>,
    pause: CheckMenuItem<R>,
    quit: MenuItem<R>,
    /// "Restart to Update" and its version, once an update is downloaded.
    update: Mutex<Option<(MenuItem<R>, String)>>,
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
            pause,
            quit,
            update: Mutex::new(None),
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
        let _ = self.pause.set_text(t(lang, Text::PauseCounting));
        let _ = self.quit.set_text(t(lang, Text::Quit));
        self.sync_toggle(self.pet_shown.load(Ordering::Relaxed));
        if let Some((item, version)) = &*self.update.lock().unwrap() {
            let _ = item.set_text(update_text(lang, version));
        }
    }

    pub fn sync_paused(&self, paused: bool) {
        let _ = self.pause.set_checked(paused);
    }

    /// Adds "Restart to Update to vX" at the top once an update is downloaded.
    pub fn show_update(&self, app: &AppHandle<R>, version: &str) {
        let mut update = self.update.lock().unwrap();
        if let Some((item, shown)) = &mut *update {
            let _ = item.set_text(update_text(self.lang(), version));
            *shown = version.to_string();
            return;
        }
        let text = update_text(self.lang(), version);
        if let Ok(item) = MenuItem::with_id(app, ID_RESTART_UPDATE, text, true, None::<&str>) {
            if self.menu.prepend(&item).is_ok() {
                *update = Some((item, version.to_string()));
            }
        }
    }

    fn sync_toggle(&self, visible: bool) {
        self.pet_shown.store(visible, Ordering::Relaxed);
        let text = if visible {
            Text::HidePet
        } else {
            Text::ShowPet
        };
        let _ = self.toggle.set_text(t(self.lang(), text));
    }
}

fn update_text(lang: Lang, version: &str) -> String {
    format!("{} v{version}", t(lang, Text::RestartToUpdate))
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
        ID_TOGGLE => {
            if let Some(window) = app.get_webview_window(PET_LABEL) {
                let visible = window.is_visible().unwrap_or(true);
                let _ = if visible {
                    window.hide()
                } else {
                    window.show()
                };
                app.state::<AppMenu<R>>().sync_toggle(!visible);
            }
        }
        ID_STATS => open_panel(app, Panel::Stats),
        ID_SETTINGS => open_panel(app, Panel::Settings),
        ID_PAUSE => {
            let paused = !app.state::<SettingsStore>().get().paused;
            let _ = crate::commands::apply_patch(app, &serde_json::json!({ "paused": paused }));
        }
        ID_RESTART_UPDATE => {
            if crate::updater::install_pending(app) {
                app.restart();
            }
        }
        ID_QUIT => {
            crate::updater::install_pending(app);
            app.exit(0);
        }
        _ => {}
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
