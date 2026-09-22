mod commands;
mod db;
mod engine;
mod hover;
mod i18n;
mod input;
mod menu;
mod panels;
mod pet_window;
mod platform;
mod settings;
mod updater;

use engine::runtime::RuntimeHandle;
use hover::HoverState;
use menu::AppMenu;
use settings::SettingsStore;
use std::time::Duration;
use tauri::{Manager, RunEvent};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .manage(updater::PendingUpdate::default())
        .on_menu_event(menu::handle_event)
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::update_settings,
            commands::set_hit_rect,
            commands::pet_drag_start,
            commands::show_context_menu,
            commands::pet_ready,
            commands::get_stats,
            commands::export_csv,
            commands::clear_data,
            commands::open_panel,
            commands::get_status,
            commands::open_input_monitoring_settings,
            commands::restart_app,
        ])
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let config_dir = app.path().app_config_dir()?;
            let settings = SettingsStore::load(config_dir.join("settings.json"));
            let initial = settings.get();
            app.manage(settings);
            app.manage(HoverState::default());

            let db_path = app.path().app_data_dir()?.join("stats.sqlite");
            let db = db::Db::open(&db_path)
                .inspect_err(|e| eprintln!("opening {} failed: {e}", db_path.display()))
                .ok();
            app.manage(engine::runtime::spawn(app.handle().clone(), db, &initial));

            let menu = AppMenu::build(app.handle())?;
            menu::create_tray(app.handle(), &menu)?;
            app.manage(menu);

            let pet = pet_window::create(app.handle())?;
            #[cfg(target_os = "macos")]
            platform::pin_to_all_spaces(&pet);
            #[cfg(not(target_os = "macos"))]
            let _ = pet;
            if !initial.onboarded {
                menu::open_panel(app.handle(), panels::Panel::Onboarding);
            }
            hover::spawn(app.handle().clone());
            updater::spawn(app.handle().clone());
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| match event {
            // Closing panel windows must not quit the app; only the Quit item does.
            RunEvent::ExitRequested {
                code: None, api, ..
            } => api.prevent_exit(),
            RunEvent::Exit => app
                .state::<RuntimeHandle>()
                .shutdown(Duration::from_secs(3)),
            _ => {}
        });
}
