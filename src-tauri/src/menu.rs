//! One menu shared by the tray icon and the pet's right-click menu.

use crate::i18n::{t, Lang, Text};
use crate::panels::{self, Panel};
use crate::pet_window::PET_LABEL;
use crate::settings::SettingsStore;
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
    toggle: MenuItem<R>,
    pause: CheckMenuItem<R>,
    lang: Lang,
}

impl<R: Runtime> AppMenu<R> {
    pub fn build(app: &AppHandle<R>) -> tauri::Result<Self> {
        let lang = Lang::system();
        let paused = app.state::<SettingsStore>().get().paused;
        let toggle = MenuItem::with_id(app, ID_TOGGLE, t(lang, Text::HidePet), true, None::<&str>)?;
        let stats = MenuItem::with_id(app, ID_STATS, t(lang, Text::Stats), true, None::<&str>)?;
        let settings = MenuItem::with_id(
            app,
            ID_SETTINGS,
            t(lang, Text::Settings),
            true,
            None::<&str>,
        )?;
        let pause = CheckMenuItem::with_id(
            app,
            ID_PAUSE,
            t(lang, Text::PauseCounting),
            true,
            paused,
            None::<&str>,
        )?;
        let quit = MenuItem::with_id(app, ID_QUIT, t(lang, Text::Quit), true, None::<&str>)?;
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
            toggle,
            pause,
            lang,
        })
    }

    pub fn sync_paused(&self, paused: bool) {
        let _ = self.pause.set_checked(paused);
    }

    /// Adds "Restart to Update to vX" at the top once an update is downloaded.
    pub fn show_update(&self, app: &AppHandle<R>, version: &str) {
        let text = format!("{} v{version}", t(self.lang, Text::RestartToUpdate));
        if let Ok(item) = MenuItem::with_id(app, ID_RESTART_UPDATE, text, true, None::<&str>) {
            let _ = self.menu.prepend(&item);
        }
    }

    fn sync_toggle(&self, visible: bool) {
        let text = if visible {
            Text::HidePet
        } else {
            Text::ShowPet
        };
        let _ = self.toggle.set_text(t(self.lang, text));
    }
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
    let lang = app.state::<AppMenu<R>>().lang;
    let title = match panel {
        Panel::Stats => t(lang, Text::Stats),
        Panel::Settings => t(lang, Text::Settings),
        Panel::Onboarding | Panel::Tour => "Jokbet",
    };
    if let Err(e) = panels::open(app, panel, title.trim_end_matches('…')) {
        log::error!("opening {panel:?} failed: {e}");
    }
}
