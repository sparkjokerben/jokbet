//! zh/en strings for native UI (tray, menus, dialogs), in the language the
//! settings ask for (the system's by default).

use crate::settings::Language;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lang {
    Zh,
    En,
}

impl Lang {
    pub fn from_locale(locale: Option<&str>) -> Self {
        match locale {
            Some(l) if l.to_ascii_lowercase().starts_with("zh") => Lang::Zh,
            _ => Lang::En,
        }
    }

    pub fn system() -> Self {
        Self::from_locale(sys_locale::get_locale().as_deref())
    }

    /// The language a setting stands for.
    pub fn resolve(setting: Language) -> Self {
        match setting {
            Language::System => Self::system(),
            Language::Zh => Lang::Zh,
            Language::En => Lang::En,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Text {
    ShowPet,
    HidePet,
    ResetPosition,
    Stats,
    Settings,
    PauseCounting,
    Quit,
}

pub fn t(lang: Lang, text: Text) -> &'static str {
    match (lang, text) {
        (Lang::Zh, Text::ShowPet) => "显示桌宠",
        (Lang::Zh, Text::HidePet) => "隐藏桌宠",
        (Lang::Zh, Text::ResetPosition) => "把桌宠移回角落",
        (Lang::Zh, Text::Stats) => "统计…",
        (Lang::Zh, Text::Settings) => "设置…",
        (Lang::Zh, Text::PauseCounting) => "暂停计数",
        (Lang::Zh, Text::Quit) => "退出",
        (Lang::En, Text::ShowPet) => "Show Pet",
        (Lang::En, Text::HidePet) => "Hide Pet",
        (Lang::En, Text::ResetPosition) => "Move Pet Back to Corner",
        (Lang::En, Text::Stats) => "Stats…",
        (Lang::En, Text::Settings) => "Settings…",
        (Lang::En, Text::PauseCounting) => "Pause Counting",
        (Lang::En, Text::Quit) => "Quit",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chinese_locales_map_to_zh() {
        for l in ["zh-CN", "zh-Hans-CN", "zh_TW", "ZH"] {
            assert_eq!(Lang::from_locale(Some(l)), Lang::Zh, "{l}");
        }
    }

    #[test]
    fn a_chosen_language_wins_over_the_system() {
        assert_eq!(Lang::resolve(Language::Zh), Lang::Zh);
        assert_eq!(Lang::resolve(Language::En), Lang::En);
        assert_eq!(Lang::resolve(Language::System), Lang::system());
    }

    #[test]
    fn everything_else_is_en() {
        for l in [Some("en-US"), Some("ja-JP"), None] {
            assert_eq!(Lang::from_locale(l), Lang::En);
        }
    }
}
