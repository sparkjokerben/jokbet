//! zh/en strings for native UI (tray, menus, dialogs), following the system locale.

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
}

#[derive(Clone, Copy, Debug)]
pub enum Text {
    ShowPet,
    HidePet,
    Quit,
}

pub fn t(lang: Lang, text: Text) -> &'static str {
    match (lang, text) {
        (Lang::Zh, Text::ShowPet) => "显示桌宠",
        (Lang::Zh, Text::HidePet) => "隐藏桌宠",
        (Lang::Zh, Text::Quit) => "退出",
        (Lang::En, Text::ShowPet) => "Show Pet",
        (Lang::En, Text::HidePet) => "Hide Pet",
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
    fn everything_else_is_en() {
        for l in [Some("en-US"), Some("ja-JP"), None] {
            assert_eq!(Lang::from_locale(l), Lang::En);
        }
    }
}
