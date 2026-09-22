use tauri::{WebviewUrl, WebviewWindowBuilder};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            // P0: a plain window so the scaffold runs; P1 adds transparency,
            // always-on-top, click-through and the rest of the pet flags.
            WebviewWindowBuilder::new(app, "pet", WebviewUrl::App("pet.html".into()))
                .title("pet")
                .inner_size(160.0, 160.0)
                .build()?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
