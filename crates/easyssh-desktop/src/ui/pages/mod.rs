use easyssh_core::Workspace;

#[cfg_attr(not(feature = "ui-test"), allow(dead_code))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Home,
    Hosts,
    Transfers,
    Keys,
    Settings,
}

impl From<Workspace> for Page {
    fn from(value: Workspace) -> Self {
        match value {
            Workspace::Home => Self::Home,
            Workspace::Hosts | Workspace::Files | Workspace::Snippets | Workspace::Forwarding => {
                Self::Hosts
            }
            Workspace::Transfers => Self::Transfers,
            Workspace::Keys => Self::Keys,
            Workspace::Settings => Self::Settings,
        }
    }
}
