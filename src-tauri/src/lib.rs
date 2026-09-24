mod commands;
mod db;
mod engine;
mod hover;
mod i18n;
mod input;
mod legacy;
mod menu;
mod panels;
mod pet_window;
mod platform;
mod settings;
mod shortcuts;
mod updater;
mod visibility;

use engine::runtime::RuntimeHandle;
use hover::HoverState;
use menu::AppMenu;
use settings::SettingsStore;
use std::time::Duration;
use tauri::{Manager, RunEvent};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // First, so that what the other plugins say is kept too.
        .plugin(logger())
        .plugin(tauri_plugin_opener::init())
        .plugin(shortcuts::plugin())
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
            commands::glass_support,
            commands::set_glass_bubble,
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
            commands::open_external,
            commands::suspend_shortcuts,
        ])
        .on_window_event(|window, event| {
            // Recording a shortcut lets go of the others; closing the window
            // mid-recording must not leave them that way.
            if matches!(event, tauri::WindowEvent::Destroyed)
                && window.label() == panels::Panel::Settings.label()
            {
                shortcuts::suspend(window.app_handle(), false);
            }
        })
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let config_dir = app.path().app_config_dir()?;
            let data_dir = app.path().app_data_dir()?;
            // The app used to be called jokerben-desktop-pet, and its data lives
            // under that name: bring it over rather than start from zero.
            for path in legacy::adopt(&data_dir, &config_dir) {
                log::info!("kept {} from the pre-rename app", path.display());
            }
            let settings = SettingsStore::load(config_dir.join("settings.json"));
            let initial = settings.get();
            app.manage(settings);
            app.manage(HoverState::default());
            app.manage(visibility::PetVisibility::default());

            let db_path = data_dir.join("stats.sqlite");
            let db = db::Db::open(&db_path)
                .inspect_err(|e| log::error!("opening {} failed: {e}", db_path.display()))
                .ok();
            app.manage(engine::runtime::spawn(app.handle().clone(), db, &initial));

            let menu = AppMenu::build(app.handle())?;
            menu::create_tray(app.handle(), &menu)?;
            app.manage(menu);

            let pet = pet_window::create(app.handle())?;
            #[cfg(target_os = "macos")]
            platform::pin_to_all_spaces(&pet, !initial.hide_in_fullscreen);
            #[cfg(not(target_os = "macos"))]
            let _ = pet;
            if !initial.onboarded {
                menu::open_panel(app.handle(), panels::Panel::Onboarding);
            }
            hover::spawn(app.handle().clone());
            visibility::spawn(app.handle().clone());
            if let Err(e) = shortcuts::register(app.handle(), &initial.shortcuts) {
                log::warn!("shortcuts: {e}");
            }
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

/// Writes to the platform's log folder (and stdout): release builds have no
/// console, and a bug report is only as good as what it can attach.
fn logger<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    use tauri_plugin_log::{RotationStrategy, Target, TargetKind, TimezoneStrategy};
    tauri_plugin_log::Builder::new()
        .targets([
            Target::new(TargetKind::Stdout),
            Target::new(TargetKind::LogDir { file_name: None }),
        ])
        .level(log::LevelFilter::Info)
        .timezone_strategy(TimezoneStrategy::UseLocal)
        .max_file_size(1024 * 1024)
        .rotation_strategy(RotationStrategy::KeepSome(3))
        .build()
}
