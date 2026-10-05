mod backup;
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
mod portable;
mod settings;
mod shortcuts;
mod updater;
mod visibility;
mod walker;

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
        .manage(updater::Updates::default())
        .on_menu_event(menu::handle_event)
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::update_settings,
            commands::set_hit_rect,
            commands::set_bubble_rect,
            commands::glass_support,
            commands::set_glass_bubble,
            commands::pet_drag_start,
            commands::walk_start,
            commands::walk_stop,
            commands::rest_ack,
            commands::show_context_menu,
            commands::pet_ready,
            commands::get_stats,
            commands::export_csv,
            commands::clear_data,
            commands::backup_create,
            commands::list_backups,
            commands::inspect_backup,
            commands::restore_backup,
            commands::open_panel,
            commands::get_status,
            commands::open_input_monitoring_settings,
            commands::restart_app,
            commands::open_external,
            commands::suspend_shortcuts,
            commands::update_status,
            commands::check_update,
            commands::install_update,
            commands::keyboard_kind,
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
            // What a bug report's log is from.
            log::info!(
                "Jokbet {} starting on {} {}",
                app.package_info().version,
                std::env::consts::OS,
                std::env::consts::ARCH
            );

            let config_dir = portable::config_dir(app.handle())?;
            let data_dir = portable::data_dir(app.handle())?;
            if let Some(root) = portable::root() {
                log::info!("portable: keeping everything in {}", root.display());
            } else {
                // The app used to be called jokerben-desktop-pet, and its data
                // lives under that name: bring it over rather than start from zero.
                for path in legacy::adopt(&data_dir, &config_dir) {
                    log::info!("kept {} from the pre-rename app", path.display());
                }
            }
            let settings = SettingsStore::load(config_dir.join("settings.json"));
            let initial = settings.get();
            app.manage(settings);
            app.manage(HoverState::default());
            app.manage(visibility::PetVisibility::default());
            app.manage(backup::BackupState::default());
            app.manage(walker::WalkState::default());

            let db_path = data_dir.join("stats.sqlite");
            // A database that will not open is set aside and a fresh one is
            // started; going on without storage quietly is the one thing this
            // must not do.
            let db = db::Db::open_salvaging(&db_path);
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
            #[cfg(target_os = "macos")]
            {
                match input::permission() {
                    input::Permission::Granted => {
                        // Remembered so that the grant a future update takes
                        // away is recognizable as one that was there.
                        if !initial.input_granted_once {
                            let _ = app
                                .state::<SettingsStore>()
                                .patch(&serde_json::json!({"inputGrantedOnce": true}));
                        }
                    }
                    // A grant that was there and is gone now — an update that
                    // changed the signature, most often — leaves the app
                    // counting nothing, and macOS will not say so again; the
                    // settings page is where the fix is.
                    input::Permission::Denied
                        if initial.onboarded && initial.input_granted_once =>
                    {
                        log::warn!("input monitoring was granted before and is not now; opening the settings");
                        menu::open_panel(app.handle(), panels::Panel::Settings);
                    }
                    _ => {}
                }
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
            Target::new(match portable::root() {
                Some(root) => TargetKind::Folder {
                    path: root.join("logs"),
                    file_name: None,
                },
                None => TargetKind::LogDir { file_name: None },
            }),
        ])
        .level(log::LevelFilter::Info)
        .timezone_strategy(TimezoneStrategy::UseLocal)
        .max_file_size(1024 * 1024)
        .rotation_strategy(RotationStrategy::KeepSome(3))
        .build()
}
