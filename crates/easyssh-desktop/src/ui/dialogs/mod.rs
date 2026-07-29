#[cfg_attr(not(feature = "ui-test"), allow(dead_code))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dialog {
    HostEditor,
    QuickConnect,
    Diagnostics,
    Sync,
}
