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
const ID_ADMIN: &str = "restart-as-admin";
const ID_RESTART_UPDATE: &str = "restart-update";
const ID_RESET_POSITION: &str = "reset-position";
const ID_QUIT: &str = "quit";

pub struct AppMenu<R: Runtime> {
    pub menu: Menu<R>,
    stats: MenuItem<R>,
    settings: MenuItem<R>,
    toggle: MenuItem<R>,
    reset_position: MenuItem<R>,
    pause: CheckMenuItem<R>,
    /// "Restart and Update", in the menu for good — the structure never
    /// changes again, the lesson of the crash of 0.2.0 — and greyed out until
    /// an update is downloaded, lit with its version after.
    update: MenuItem<R>,
    /// The version the update item is lit for, so a language change can
    /// retitle it to match.
    update_version: Mutex<Option<String>>,
    /// The item that asks for an administrator's token, where the platform
    /// could and the process does not have it. `None` once there is nothing
    /// left to ask for (elevated, or everywhere but Windows).
    admin: Option<MenuItem<R>>,
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
        // Greyed to start, on its own: the item closure above lights every
        // item up, and nothing else in the menu has a reason to be dark.
        let update = MenuItem::with_id(
            app,
            ID_RESTART_UPDATE,
            t(lang, Text::RestartToUpdate),
            false,
            None::<&str>,
        )?;
        let admin = match crate::platform::is_elevated() {
            true => None,
            false => Some(item(ID_ADMIN, Text::RestartAsAdmin)?),
        };
        let quit = item(ID_QUIT, Text::Quit)?;
        let above_panels = PredefinedMenuItem::separator(app)?;
        let above_quit = PredefinedMenuItem::separator(app)?;
        let mut items: Vec<&dyn tauri::menu::IsMenuItem<R>> = vec![
            &stats,
            &settings,
            &above_panels,
            &toggle,
            &reset_position,
            &pause,
        ];
        match &admin {
            Some(admin) => {
                items.push(&above_quit);
                items.push(admin);
            }
            None => items.push(&above_quit),
        }
        items.push(&update);
        items.push(&quit);
        let menu = Menu::with_items(app, &items)?;
        Ok(Self {
            menu,
            stats,
            settings,
            toggle,
            reset_position,
            pause,
            update,
            update_version: Mutex::new(None),
            admin,
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
        if let Some(admin) = &self.admin {
            let _ = admin.set_text(t(lang, Text::RestartAsAdmin));
        }
        let _ = self.quit.set_text(t(lang, Text::Quit));
        // The version the item is lit for, if it is lit: read before the
        // retitle, whose way home runs over the main thread again.
        let lit_for = self.update_version.lock().unwrap().clone();
        self.sync_update(lit_for.as_deref());
        self.sync_toggle(self.pet_shown.load(Ordering::Relaxed));
    }

    /// Says what the update item should: grey without an update downloaded,
    /// lit with its version when there is one. Like the pause tick and the
    /// toggle's title, a change to an item the menu already has — and so
    /// main-thread work, on the terms the crash of 0.2.0 set down.
    pub fn sync_update(&self, version: Option<&str>) {
        *self.update_version.lock().unwrap() = version.map(str::to_owned);
        let _ = self.update.set_enabled(version.is_some());
        let _ = self.update.set_text(update_label(self.lang(), version));
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
/// Items are retitled, ticked, enabled and shown from whichever thread noticed the
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

/// Shapes the update item from off the main thread, where the updater tends
/// to notice its status. The version is copied in, a borrow cannot cross into
/// the closure.
pub fn sync_update<R: Runtime>(app: &AppHandle<R>, version: Option<&str>) {
    let version = version.map(str::to_owned);
    on_main(app, move |menu| menu.sync_update(version.as_deref()));
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
            crate::walker::stop(app, crate::walker::Stop::Moved);
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
        ID_ADMIN => match crate::platform::relaunch_elevated() {
            // The next copy is up and asking for nothing; this one goes, and
            // the counts it still holds pending are written out as always.
            Ok(()) => app.exit(0),
            Err(e) => log::warn!("the relaunched copy was not started: {e}"),
        },
        ID_RESTART_UPDATE => {
            // install_pending writes the counts it holds out before going on.
            if crate::updater::install_pending(app) {
                crate::commands::restart(app);
            } else {
                log::warn!("the pending update could not be installed");
            }
        }
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

/// What the update item says: the words the settings' own button uses, and
/// the version once there is one waiting.
fn update_label(lang: Lang, version: Option<&str>) -> String {
    match version {
        Some(v) => format!("{} v{v}", t(lang, Text::RestartToUpdate)),
        None => t(lang, Text::RestartToUpdate).to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_update_item_names_the_version_when_one_waits() {
        assert_eq!(update_label(Lang::Zh, None), "重启并更新");
        assert_eq!(update_label(Lang::Zh, Some("0.4.8")), "重启并更新 v0.4.8");
        assert_eq!(update_label(Lang::En, None), "Restart and Update");
        assert_eq!(
            update_label(Lang::En, Some("0.4.8")),
            "Restart and Update v0.4.8"
        );
    }
}
