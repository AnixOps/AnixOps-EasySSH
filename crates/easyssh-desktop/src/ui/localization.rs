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
            Locale::System => system_locale(),
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
            (Locale::ZhCn, Key::ImportSshConfig) => "\u{5bfc}\u{5165} SSH \u{914d}\u{7f6e}",
            (Locale::ZhCn, Key::Add) => "\u{6dfb}\u{52a0}",
            (Locale::ZhCn, Key::Added) => "\u{5df2}\u{6dfb}\u{52a0}",
            (Locale::ZhCn, Key::AddAll) => "\u{5168}\u{90e8}\u{6dfb}\u{52a0}",
            (Locale::ZhCn, Key::Checking) => "\u{6b63}\u{5728}\u{68c0}\u{67e5}",
            (Locale::ZhCn, Key::NotFound) => "\u{672a}\u{627e}\u{5230}",
            (Locale::ZhCn, Key::RemoteFileBrowser) => {
                "\u{8fdc}\u{7a0b}\u{6587}\u{4ef6}\u{6d4f}\u{89c8}\u{5668}"
            }
            (Locale::ZhCn, Key::LocalFirstWorkspace) => "\u{672c}\u{5730}\u{4f18}\u{5148}\u{7684} SSH \u{5de5}\u{4f5c}\u{533a}",
            (Locale::ZhCn, Key::QuickConnect) => "\u{5feb}\u{901f}\u{8fde}\u{63a5}",
            (Locale::ZhCn, Key::UsesSystemOpenSsh) => "\u{4f7f}\u{7528}\u{7cfb}\u{7edf} OpenSSH \u{914d}\u{7f6e}\u{548c}\u{4ee3}\u{7406}\u{3002}",
            (Locale::ZhCn, Key::Favorites) => "\u{6536}\u{85cf}\u{5939}",
            (Locale::ZhCn, Key::NoFavoriteHosts) => "\u{6682}\u{65e0}\u{6536}\u{85cf}\u{4e3b}\u{673a}\u{3002}",
            (Locale::ZhCn, Key::RecentConnections) => "\u{6700}\u{8fd1}\u{8fde}\u{63a5}",
            (Locale::ZhCn, Key::NoSessions) => "\u{8fd8}\u{6ca1}\u{6709}\u{4f1a}\u{8bdd}\u{8bb0}\u{5f55}\u{3002}",
            (Locale::ZhCn, Key::FromSshConfig) => "\u{6765}\u{81ea} SSH \u{914d}\u{7f6e}",
            (Locale::ZhCn, Key::NoHostAliases) => "\u{672a}\u{627e}\u{5230}\u{5177}\u{4f53}\u{4e3b}\u{673a}\u{522b}\u{540d}\u{3002}",
            (Locale::ZhCn, Key::Workspaces) => "\u{5de5}\u{4f5c}\u{533a}",
            (Locale::ZhCn, Key::Diagnostics) => "\u{8bca}\u{65ad}",
            (Locale::ZhCn, Key::CycleTheme) => "\u{5207}\u{6362}\u{4e3b}\u{9898}",
            (Locale::ZhCn, Key::GitMetadataSync) => "Git \u{5143}\u{6570}\u{636e}\u{540c}\u{6b65}",
            (Locale::ZhCn, Key::SearchCommands) => "\u{641c}\u{7d22}\u{547d}\u{4ee4}\u{548c}\u{4e3b}\u{673a}",
            (Locale::ZhCn, Key::ExternalTerminalLaunches) => "\u{5916}\u{90e8}\u{7ec8}\u{7aef}\u{542f}\u{52a8}",
            (Locale::ZhCn, Key::HideFromSessionBar) => "\u{4ece}\u{4f1a}\u{8bdd}\u{680f}\u{4e2d}\u{9690}\u{85cf}",
            (Locale::ZhCn, Key::AgentReady) => "\u{4ee3}\u{7406}\u{5df2}\u{5c31}\u{7eea}",
            (Locale::ZhCn, Key::Agent) => "\u{4ee3}\u{7406}",
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
            (_, Key::ImportSshConfig) => "Import SSH Config",
            (_, Key::Add) => "Add",
            (_, Key::Added) => "Added",
            (_, Key::AddAll) => "Add all",
            (_, Key::Checking) => "Checking",
            (_, Key::NotFound) => "Not found",
            (_, Key::RemoteFileBrowser) => "Remote file browser",
            (_, Key::LocalFirstWorkspace) => "Local-first SSH workspace",
            (_, Key::QuickConnect) => "Quick connect",
            (_, Key::UsesSystemOpenSsh) => "Uses the system OpenSSH configuration and agent.",
            (_, Key::Favorites) => "Favorites",
            (_, Key::NoFavoriteHosts) => "No favorite hosts yet.",
            (_, Key::RecentConnections) => "Recent connections",
            (_, Key::NoSessions) => "No sessions recorded yet.",
            (_, Key::FromSshConfig) => "From SSH Config",
            (_, Key::NoHostAliases) => "No concrete host aliases found.",
            (_, Key::Workspaces) => "Workspaces",
            (_, Key::Diagnostics) => "Diagnostics",
            (_, Key::CycleTheme) => "Cycle theme",
            (_, Key::GitMetadataSync) => "Git metadata sync",
            (_, Key::SearchCommands) => "Search commands and hosts",
            (_, Key::ExternalTerminalLaunches) => "External terminal launches",
            (_, Key::HideFromSessionBar) => "Hide from session bar",
            (_, Key::AgentReady) => "Agent ready",
            (_, Key::Agent) => "Agent",
        }
    }
}

fn resolve_system_locale(value: Option<&str>) -> Locale {
    match value.map(|value| value.replace('_', "-").to_ascii_lowercase()) {
        Some(value) if value == "zh-cn" || value.starts_with("zh-cn.") => Locale::ZhCn,
        _ => Locale::En,
    }
}

fn system_locale() -> Locale {
    let environment = ["LC_ALL", "LC_MESSAGES", "LANGUAGE", "LANG"]
        .into_iter()
        .find_map(|name| std::env::var(name).ok())
        .filter(|value| !value.trim().is_empty());
    #[cfg(windows)]
    let environment = environment.or_else(windows_ui_locale);
    resolve_system_locale(environment.as_deref())
}

#[cfg(windows)]
fn windows_ui_locale() -> Option<String> {
    unsafe extern "system" {
        fn GetUserDefaultLocaleName(locale_name: *mut u16, cch_locale_name: i32) -> i32;
    }
    let mut buffer = [0u16; 85];
    // Windows documents LOCALE_NAME_MAX_LENGTH as 85, including the NUL.
    let length = unsafe { GetUserDefaultLocaleName(buffer.as_mut_ptr(), buffer.len() as i32) };
    (length > 1).then(|| String::from_utf16_lossy(&buffer[..length as usize - 1]))
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
    ImportSshConfig,
    Add,
    Added,
    AddAll,
    Checking,
    NotFound,
    RemoteFileBrowser,
    LocalFirstWorkspace,
    QuickConnect,
    UsesSystemOpenSsh,
    Favorites,
    NoFavoriteHosts,
    RecentConnections,
    NoSessions,
    FromSshConfig,
    NoHostAliases,
    Workspaces,
    Diagnostics,
    CycleTheme,
    GitMetadataSync,
    SearchCommands,
    ExternalTerminalLaunches,
    HideFromSessionBar,
    AgentReady,
    Agent,
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEYS: [Key; 36] = [
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
        Key::ImportSshConfig,
        Key::Add,
        Key::Added,
        Key::AddAll,
        Key::Checking,
        Key::NotFound,
        Key::RemoteFileBrowser,
        Key::LocalFirstWorkspace,
        Key::QuickConnect,
        Key::UsesSystemOpenSsh,
        Key::Favorites,
        Key::NoFavoriteHosts,
        Key::RecentConnections,
        Key::NoSessions,
        Key::FromSshConfig,
        Key::NoHostAliases,
        Key::Workspaces,
        Key::Diagnostics,
        Key::CycleTheme,
        Key::GitMetadataSync,
        Key::SearchCommands,
        Key::ExternalTerminalLaunches,
        Key::HideFromSessionBar,
        Key::AgentReady,
        Key::Agent,
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

    #[test]
    fn locale_parser_accepts_windows_language_names() {
        assert_eq!(resolve_system_locale(Some("zh-CN")), Locale::ZhCn);
    }
}
