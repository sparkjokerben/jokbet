//! On-demand windows (stats, settings, onboarding): created when opened and
//! destroyed when closed, so an idle app keeps only the pet's webview.

use tauri::{AppHandle, Manager, Runtime, WebviewUrl, WebviewWindowBuilder};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Panel {
    Stats,
    Settings,
    /// First-run steps, or only the permission step once onboarded.
    Onboarding,
    /// Every onboarding step again, asked for from the settings.
    Tour,
}

impl Panel {
    fn label(self) -> &'static str {
        match self {
            Panel::Stats => "stats",
            Panel::Settings => "settings",
            Panel::Onboarding | Panel::Tour => "onboarding",
        }
    }

    fn query(self) -> &'static str {
        match self {
            Panel::Tour => "view=onboarding&tour",
            _ => "",
        }
    }

    fn size(self) -> (f64, f64) {
        match self {
            Panel::Stats => (820.0, 640.0),
            Panel::Settings => (520.0, 660.0),
            Panel::Onboarding | Panel::Tour => (480.0, 520.0),
        }
    }
}

pub fn open<R: Runtime>(app: &AppHandle<R>, panel: Panel, title: &str) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window(panel.label()) {
        window.unminimize()?;
        window.show()?;
        return window.set_focus();
    }
    let (w, h) = panel.size();
    let query = match panel.query() {
        "" => format!("view={}", panel.label()),
        q => q.to_string(),
    };
    let url = WebviewUrl::App(format!("app.html?{query}").into());
    let window = WebviewWindowBuilder::new(app, panel.label(), url)
        .title(title)
        .inner_size(w, h)
        .min_inner_size(w * 0.75, h * 0.75)
        .center()
        .build()?;
    window.set_focus()
}
