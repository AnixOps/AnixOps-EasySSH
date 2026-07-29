use easyssh_core::Locale;

/// Typed desktop-facing strings. `System` intentionally selects Chinese only
/// for a zh-CN locale; all other systems receive English.
#[derive(Clone, Copy)]
pub struct Strings {
    locale: Locale,
}

impl Strings {
    pub fn new(locale: Locale) -> Self {
        let locale = match locale {
            Locale::System => resolve_system_locale(std::env::var("LANG").ok().as_deref()),
            value => value,
        };
        Self { locale }
    }

    pub fn text(self, key: Key) -> &'static str {
        match (self.locale, key) {
            (Locale::ZhCn, Key::Home) => "\u{4e3b}\u{9875}",
            (Locale::ZhCn, Key::Hosts) => "\u{4e3b}\u{673a}",
            (Locale::ZhCn, Key::Transfers) => "\u{4f20}\u{8f93}",
            (Locale::ZhCn, Key::Keys) => "\u{5bc6}\u{94a5}",
            (Locale::ZhCn, Key::Settings) => "\u{8bbe}\u{7f6e}",
            (Locale::ZhCn, Key::Refresh) => "\u{5237}\u{65b0}",
            (Locale::ZhCn, Key::NewHost) => "\u{65b0}\u{5efa}\u{4e3b}\u{673a}",
            (Locale::ZhCn, Key::RecentlyLaunched) => "\u{6700}\u{8fd1}\u{542f}\u{52a8}",
            (Locale::ZhCn, Key::DiscardChanges) => "\u{653e}\u{5f03}\u{66f4}\u{6539}",
            (Locale::ZhCn, Key::KeepEditing) => "\u{7ee7}\u{7eed}\u{7f16}\u{8f91}",
            (Locale::ZhCn, Key::Connect) => "\u{8fde}\u{63a5}",
            (_, Key::Home) => "Home",
            (_, Key::Hosts) => "Hosts",
            (_, Key::Transfers) => "Transfers",
            (_, Key::Keys) => "Keys",
            (_, Key::Settings) => "Settings",
            (_, Key::Refresh) => "Refresh",
            (_, Key::NewHost) => "New host",
            (_, Key::RecentlyLaunched) => "Recently launched",
            (_, Key::DiscardChanges) => "Discard changes",
            (_, Key::KeepEditing) => "Keep editing",
            (_, Key::Connect) => "Connect",
        }
    }
}

fn resolve_system_locale(value: Option<&str>) -> Locale {
    match value.map(|value| value.replace('_', "-").to_ascii_lowercase()) {
        Some(value) if value == "zh-cn" || value.starts_with("zh-cn.") => Locale::ZhCn,
        _ => Locale::En,
    }
}

#[derive(Clone, Copy)]
pub enum Key {
    Home,
    Hosts,
    Transfers,
    Keys,
    Settings,
    Refresh,
    NewHost,
    RecentlyLaunched,
    DiscardChanges,
    KeepEditing,
    Connect,
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEYS: [Key; 11] = [
        Key::Home,
        Key::Hosts,
        Key::Transfers,
        Key::Keys,
        Key::Settings,
        Key::Refresh,
        Key::NewHost,
        Key::RecentlyLaunched,
        Key::DiscardChanges,
        Key::KeepEditing,
        Key::Connect,
    ];

    #[test]
    fn both_locales_cover_every_key() {
        for key in KEYS {
            assert!(!Strings::new(Locale::En).text(key).is_empty());
            assert!(!Strings::new(Locale::ZhCn).text(key).is_empty());
        }
    }

    #[test]
    fn system_locale_selects_chinese_only_for_zh_cn() {
        assert_eq!(resolve_system_locale(Some("zh_CN.UTF-8")), Locale::ZhCn);
        assert_eq!(resolve_system_locale(Some("zh-TW")), Locale::En);
        assert_eq!(resolve_system_locale(Some("en_GB.UTF-8")), Locale::En);
    }
}
