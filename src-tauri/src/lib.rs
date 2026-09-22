mod commands;
mod hover;
mod i18n;
mod menu;
mod pet_window;
mod settings;

use hover::HoverState;
use menu::AppMenu;
use settings::SettingsStore;
use tauri::{Manager, RunEvent};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .on_menu_event(menu::handle_event)
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::set_hit_rect,
            commands::pet_drag_end,
            commands::show_context_menu,
        ])
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let config_dir = app.path().app_config_dir()?;
            app.manage(SettingsStore::load(config_dir.join("settings.json")));
            app.manage(HoverState::default());

            let menu = AppMenu::build(app.handle())?;
            menu::create_tray(app.handle(), &menu)?;
            app.manage(menu);

            pet_window::create(app.handle())?;
            hover::spawn(app.handle().clone());
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_app, event| {
            // Closing panel windows must not quit the app; only the Quit item does.
            if let RunEvent::ExitRequested {
                code: None, api, ..
            } = event
            {
                api.prevent_exit();
            }
        });
}
