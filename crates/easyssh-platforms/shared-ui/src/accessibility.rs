//! Accessibility System
//!
//! WCAG 2.1 AA compliant accessibility features:
//! - Screen reader support
//! - Keyboard navigation
//! - Focus management
//! - High contrast mode
//! - Reduced motion support
//! - ARIA attributes

use serde::{Deserialize, Serialize};

/// Accessibility configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccessibilityConfig {
    /// Enable screen reader announcements
    pub screen_reader_enabled: bool,
    /// Enable keyboard navigation
    pub keyboard_navigation: bool,
    /// Focus visible style
    pub focus_visible: bool,
    /// High contrast mode
    pub high_contrast: bool,
    /// Reduced motion preference
    pub reduced_motion: bool,
    /// Minimum font size (px)
    pub min_font_size: u32,
    /// Colorblind mode
    pub colorblind_mode: Option<ColorblindType>,
}

impl Default for AccessibilityConfig {
    fn default() -> Self {
        Self {
            screen_reader_enabled: true,
            keyboard_navigation: true,
            focus_visible: true,
            high_contrast: false,
            reduced_motion: false,
            min_font_size: 12,
            colorblind_mode: None,
        }
    }
}

/// Colorblind types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ColorblindType {
    /// Red-green (deuteranopia)
    Deuteranopia,
    /// Red-green (protanopia)
    Protanopia,
    /// Blue-yellow (tritanopia)
    Tritanopia,
    /// Complete color blindness
    Achromatopsia,
}

/// ARIA roles for semantic HTML
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AriaRole {
    /// Generic button
    Button,
    /// Checkbox
    Checkbox,
    /// Radio button
    Radio,
    /// Menu item
    MenuItem,
    /// Menu bar
    Menubar,
    /// Navigation
    Navigation,
    /// Main content
    Main,
    /// Complementary content
    Complementary,
    /// Article
    Article,
    /// Section
    Section,
    /// Dialog/Modal
    Dialog,
    /// Alert dialog
    AlertDialog,
    /// Tooltip
    Tooltip,
    /// Tab panel
    Tabpanel,
    /// Tab
    Tab,
    /// Tab list
    Tablist,
    /// Tree
    Tree,
    /// Tree item
    TreeItem,
    /// List
    List,
    /// List item
    ListItem,
    /// Grid
    Grid,
    /// Grid cell
    GridCell,
    /// Text box
    Textbox,
    /// Search box
    Searchbox,
    /// Combo box
    Combobox,
    /// Progress bar
    ProgressBar,
    /// Slider
    Slider,
    /// Switch
    Switch,
    /// Link
    Link,
    /// Heading
    Heading,
    /// Banner
    Banner,
    /// Content info
    ContentInfo,
    /// Form
    Form,
    /// Search
    Search,
    /// Separator
    Separator,
    /// Status
    Status,
    /// Alert
    Alert,
    /// Log
    Log,
    /// Timer
    Timer,
    /// Marquee
    Marquee,
    /// Spin button
    SpinButton,
    /// Option
    Option,
    /// Presentation (decorative)
    Presentation,
    /// None
    None,
}

impl AriaRole {
    /// Get ARIA role string
    pub fn as_str(&self) -> &'static str {
        match self {
            AriaRole::Button => "button",
            AriaRole::Checkbox => "checkbox",
            AriaRole::Radio => "radio",
            AriaRole::MenuItem => "menuitem",
            AriaRole::Menubar => "menubar",
            AriaRole::Navigation => "navigation",
            AriaRole::Main => "main",
            AriaRole::Complementary => "complementary",
            AriaRole::Article => "article",
            AriaRole::Section => "section",
            AriaRole::Dialog => "dialog",
            AriaRole::AlertDialog => "alertdialog",
            AriaRole::Tooltip => "tooltip",
            AriaRole::Tabpanel => "tabpanel",
            AriaRole::Tab => "tab",
            AriaRole::Tablist => "tablist",
            AriaRole::Tree => "tree",
            AriaRole::TreeItem => "treeitem",
            AriaRole::List => "list",
            AriaRole::ListItem => "listitem",
            AriaRole::Grid => "grid",
            AriaRole::GridCell => "gridcell",
            AriaRole::Textbox => "textbox",
            AriaRole::Searchbox => "searchbox",
            AriaRole::Combobox => "combobox",
            AriaRole::ProgressBar => "progressbar",
            AriaRole::Slider => "slider",
            AriaRole::Switch => "switch",
            AriaRole::Link => "link",
            AriaRole::Heading => "heading",
            AriaRole::Banner => "banner",
            AriaRole::ContentInfo => "contentinfo",
            AriaRole::Form => "form",
            AriaRole::Search => "search",
            AriaRole::Separator => "separator",
            AriaRole::Status => "status",
            AriaRole::Alert => "alert",
            AriaRole::Log => "log",
            AriaRole::Timer => "timer",
            AriaRole::Marquee => "marquee",
            AriaRole::SpinButton => "spinbutton",
            AriaRole::Option => "option",
            AriaRole::Presentation => "presentation",
            AriaRole::None => "none",
        }
    }
}

/// ARIA states and properties
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AriaAttribute {
    /// Whether element is expanded
    Expanded(bool),
    /// Whether element is collapsed
    Collapsed(bool),
    /// Whether element is selected
    Selected(bool),
    /// Whether element is disabled
    Disabled(bool),
    /// Whether element is required
    Required(bool),
    /// Whether element is read-only
    ReadOnly(bool),
    /// Whether element is checked
    Checked(Option<bool>), // Some(true)=checked, Some(false)=unchecked, None=mixed
    /// Whether element is pressed
    Pressed(Option<bool>),
    /// Current value
    Value(String),
    /// Value now (for progress/slider)
    ValueNow(i32),
    /// Minimum value
    ValueMin(i32),
    /// Maximum value
    ValueMax(i32),
    /// Text value
    ValueText(String),
    /// Set size (for lists)
    SetSize(i32),
    /// Position in set
    PosInSet(i32),
    /// Level (for trees)
    Level(i32),
    /// Sort direction
    Sort(AriaSort),
    /// Orientation
    Orientation(AriaOrientation),
    /// Label
    Label(String),
    /// Labelled by element ID
    LabelledBy(String),
    /// Description
    DescribedBy(String),
    /// Details
    Details(String),
    /// Error message
    ErrorMessage(String),
    /// Controls element ID
    Controls(String),
    /// Owns element ID
    Owns(String),
    /// Has popup
    HasPopup(AriaPopup),
    /// Modal
    Modal(bool),
    /// Busy
    Busy(bool),
    /// Live region type
    Live(AriaLive),
    /// Atomic update
    Atomic(bool),
    /// Relevant changes
    Relevant(AriaRelevant),
    /// Hidden
    Hidden(bool),
    /// Key shortcuts
    KeyShortcuts(String),
    /// Roledescription
    RoleDescription(String),
}

impl AriaAttribute {
    /// Get attribute name and value
    pub fn to_attr(&self) -> (&'static str, String) {
        match self {
            AriaAttribute::Expanded(v) => ("aria-expanded", v.to_string()),
            AriaAttribute::Collapsed(v) => ("aria-colapsed", v.to_string()),
            AriaAttribute::Selected(v) => ("aria-selected", v.to_string()),
            AriaAttribute::Disabled(v) => ("aria-disabled", v.to_string()),
            AriaAttribute::Required(v) => ("aria-required", v.to_string()),
            AriaAttribute::ReadOnly(v) => ("aria-readonly", v.to_string()),
            AriaAttribute::Checked(v) => (
                "aria-checked",
                match v {
                    Some(true) => "true".to_string(),
                    Some(false) => "false".to_string(),
                    None => "mixed".to_string(),
                },
            ),
            AriaAttribute::Pressed(v) => (
                "aria-pressed",
                match v {
                    Some(true) => "true".to_string(),
                    Some(false) => "false".to_string(),
                    None => "mixed".to_string(),
                },
            ),
            AriaAttribute::Value(v) => ("aria-value", v.clone()),
            AriaAttribute::ValueNow(v) => ("aria-valuenow", v.to_string()),
            AriaAttribute::ValueMin(v) => ("aria-valuemin", v.to_string()),
            AriaAttribute::ValueMax(v) => ("aria-valuemax", v.to_string()),
            AriaAttribute::ValueText(v) => ("aria-valuetext", v.clone()),
            AriaAttribute::SetSize(v) => ("aria-setsize", v.to_string()),
            AriaAttribute::PosInSet(v) => ("aria-posinset", v.to_string()),
            AriaAttribute::Level(v) => ("aria-level", v.to_string()),
            AriaAttribute::Sort(v) => ("aria-sort", v.as_str().to_string()),
            AriaAttribute::Orientation(v) => ("aria-orientation", v.as_str().to_string()),
            AriaAttribute::Label(v) => ("aria-label", v.clone()),
            AriaAttribute::LabelledBy(v) => ("aria-labelledby", v.clone()),
            AriaAttribute::DescribedBy(v) => ("aria-describedby", v.clone()),
            AriaAttribute::Details(v) => ("aria-details", v.clone()),
            AriaAttribute::ErrorMessage(v) => ("aria-errormessage", v.clone()),
            AriaAttribute::Controls(v) => ("aria-controls", v.clone()),
            AriaAttribute::Owns(v) => ("aria-owns", v.clone()),
            AriaAttribute::HasPopup(v) => ("aria-haspopup", v.as_str().to_string()),
            AriaAttribute::Modal(v) => ("aria-modal", v.to_string()),
            AriaAttribute::Busy(v) => ("aria-busy", v.to_string()),
            AriaAttribute::Live(v) => ("aria-live", v.as_str().to_string()),
            AriaAttribute::Atomic(v) => ("aria-atomic", v.to_string()),
            AriaAttribute::Relevant(v) => ("aria-relevant", v.as_str().to_string()),
            AriaAttribute::Hidden(v) => ("aria-hidden", v.to_string()),
            AriaAttribute::KeyShortcuts(v) => ("aria-keyshortcuts", v.clone()),
            AriaAttribute::RoleDescription(v) => ("aria-roledescription", v.clone()),
        }
    }
}

/// ARIA sort values
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AriaSort {
    /// Ascending sort
    Ascending,
    /// Descending sort
    Descending,
    /// No sort
    None,
    /// Other sort
    Other,
}

impl AriaSort {
    pub fn as_str(&self) -> &'static str {
        match self {
            AriaSort::Ascending => "ascending",
            AriaSort::Descending => "descending",
            AriaSort::None => "none",
            AriaSort::Other => "other",
        }
    }
}

/// ARIA orientation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AriaOrientation {
    /// Horizontal orientation
    Horizontal,
    /// Vertical orientation
    Vertical,
}

impl AriaOrientation {
    pub fn as_str(&self) -> &'static str {
        match self {
            AriaOrientation::Horizontal => "horizontal",
            AriaOrientation::Vertical => "vertical",
        }
    }
}

/// ARIA popup types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AriaPopup {
    /// False (no popup)
    False,
    /// True (generic popup)
    True,
    /// Menu popup
    Menu,
    /// Listbox popup
    Listbox,
    /// Tree popup
    Tree,
    /// Grid popup
    Grid,
    /// Dialog popup
    Dialog,
}

impl AriaPopup {
    pub fn as_str(&self) -> &'static str {
        match self {
            AriaPopup::False => "false",
            AriaPopup::True => "true",
            AriaPopup::Menu => "menu",
            AriaPopup::Listbox => "listbox",
            AriaPopup::Tree => "tree",
            AriaPopup::Grid => "grid",
            AriaPopup::Dialog => "dialog",
        }
    }
}

/// ARIA live region types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AriaLive {
    /// No live announcements
    Off,
    /// Polite announcements (wait for idle)
    Polite,
    /// Assertive announcements (interrupt)
    Assertive,
}

impl AriaLive {
    pub fn as_str(&self) -> &'static str {
        match self {
            AriaLive::Off => "off",
            AriaLive::Polite => "polite",
            AriaLive::Assertive => "assertive",
        }
    }
}

/// ARIA relevant values
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AriaRelevant {
    /// Additions
    Additions,
    /// Removals
    Removals,
    /// Text changes
    Text,
    /// All changes
    All,
    /// Additions and text
    AdditionsText,
}

impl AriaRelevant {
    pub fn as_str(&self) -> &'static str {
        match self {
            AriaRelevant::Additions => "additions",
            AriaRelevant::Removals => "removals",
            AriaRelevant::Text => "text",
            AriaRelevant::All => "all",
            AriaRelevant::AdditionsText => "additions text",
        }
    }
}

/// Accessibility properties container
#[derive(Debug, Clone, Default)]
pub struct A11yProps {
    /// ARIA role
    pub role: Option<AriaRole>,
    /// ARIA attributes
    pub attrs: Vec<AriaAttribute>,
    /// Tab index
    pub tab_index: Option<i32>,
    /// Title/tooltip
    pub title: Option<String>,
    /// Whether element is focusable
    pub focusable: bool,
}

impl A11yProps {
    /// Create new accessibility properties
    pub fn new() -> Self {
        Self::default()
    }

    /// Set ARIA role
    pub fn role(mut self, role: AriaRole) -> Self {
        self.role = Some(role);
        self
    }

    /// Add ARIA attribute
    pub fn attr(mut self, attr: AriaAttribute) -> Self {
        self.attrs.push(attr);
        self
    }

    /// Set tab index
    pub fn tab_index(mut self, index: i32) -> Self {
        self.tab_index = Some(index);
        self.focusable = true;
        self
    }

    /// Set title
    pub fn title(mut self, title: &str) -> Self {
        self.title = Some(title.to_string());
        self
    }

    /// Make focusable
    pub fn focusable(mut self) -> Self {
        self.focusable = true;
        self
    }

    /// Convert to HTML attributes
    pub fn to_html_attrs(&self) -> Vec<(String, String)> {
        let mut attrs = Vec::new();

        if let Some(role) = &self.role {
            attrs.push(("role".to_string(), role.as_str().to_string()));
        }

        for attr in &self.attrs {
            let (name, value) = attr.to_attr();
            attrs.push((name.to_string(), value));
        }

        if let Some(tab_index) = self.tab_index {
            attrs.push(("tabindex".to_string(), tab_index.to_string()));
        }

        if let Some(title) = &self.title {
            attrs.push(("title".to_string(), title.clone()));
        }

        attrs
    }
}

/// Focus management
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FocusStrategy {
    /// First focusable element
    First,
    /// Last focusable element
    Last,
    /// Specific element by ID
    ById(String),
    /// Next focusable element
    Next,
    /// Previous focusable element
    Previous,
}

/// Keyboard navigation keys
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavigationKey {
    /// Tab key
    Tab,
    /// Shift+Tab
    ShiftTab,
    /// Enter/Return
    Enter,
    /// Escape
    Escape,
    /// Space
    Space,
    /// Arrow up
    ArrowUp,
    /// Arrow down
    ArrowDown,
    /// Arrow left
    ArrowLeft,
    /// Arrow right
    ArrowRight,
    /// Home
    Home,
    /// End
    End,
    /// Page up
    PageUp,
    /// Page down
    PageDown,
}

/// Accessibility manager
pub struct AccessibilityManager {
    config: AccessibilityConfig,
    focus_history: Vec<String>,
    announcements: Vec<ScreenReaderAnnouncement>,
}

/// Screen reader announcement
#[derive(Debug, Clone)]
pub struct ScreenReaderAnnouncement {
    /// Announcement text
    pub text: String,
    /// Priority level
    pub priority: AnnouncementPriority,
    /// Whether announcement is polite or assertive
    pub live: AriaLive,
}

/// Announcement priority
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnnouncementPriority {
    /// Low priority
    Low,
    /// Medium priority
    Medium,
    /// High priority
    High,
}

impl AccessibilityManager {
    /// Create a new accessibility manager
    pub fn new(config: &AccessibilityConfig) -> Self {
        Self {
            config: *config,
            focus_history: Vec::new(),
            announcements: Vec::new(),
        }
    }

    /// Announce to screen reader
    pub fn announce(&mut self, text: &str, priority: AnnouncementPriority) {
        if !self.config.screen_reader_enabled {
            return;
        }

        let live = match priority {
            AnnouncementPriority::High => AriaLive::Assertive,
            _ => AriaLive::Polite,
        };

        self.announcements.push(ScreenReaderAnnouncement {
            text: text.to_string(),
            priority,
            live,
        });
    }

    /// Track focus change
    pub fn track_focus(&mut self, element_id: &str) {
        self.focus_history.push(element_id.to_string());
        // Keep only last 10
        if self.focus_history.len() > 10 {
            self.focus_history.remove(0);
        }
    }

    /// Get previous focused element
    pub fn previous_focus(&self) -> Option<&String> {
        if self.focus_history.len() >= 2 {
            self.focus_history.get(self.focus_history.len() - 2)
        } else {
            None
        }
    }

    /// Clear announcements
    pub fn clear_announcements(&mut self) {
        self.announcements.clear();
    }

    /// Check if high contrast is enabled
    pub fn is_high_contrast(&self) -> bool {
        self.config.high_contrast
    }

    /// Check if reduced motion is preferred
    pub fn prefers_reduced_motion(&self) -> bool {
        self.config.reduced_motion
    }

    /// Get adjusted font size
    pub fn adjusted_font_size(&self, base_size: u32) -> u32 {
        base_size.max(self.config.min_font_size)
    }

    /// Get colorblind adjustments if any
    pub fn colorblind_adjustment(&self) -> Option<ColorblindType> {
        self.config.colorblind_mode
    }
}

/// Focus ring utilities
pub struct FocusRing;

impl FocusRing {
    /// Get focus ring CSS style
    pub fn style(color: &str, width: u32, offset: u32) -> String {
        format!(
            "outline: none; box-shadow: 0 0 0 {}px {}, 0 0 0 {}px {};",
            width,
            color,
            width + offset,
            "var(--focus-ring-inner)"
        )
    }

    /// Get high contrast focus ring
    pub fn high_contrast_style() -> String {
        "outline: 2px solid currentColor; outline-offset: 2px;".to_string()
    }
}

/// Color contrast utilities
pub struct ColorContrast;

impl ColorContrast {
    /// Calculate relative luminance
    pub fn luminance(r: u8, g: u8, b: u8) -> f64 {
        let rs_rgb = r as f64 / 255.0;
        let gs_rgb = g as f64 / 255.0;
        let bs_rgb = b as f64 / 255.0;

        let r = if rs_rgb <= 0.03928 {
            rs_rgb / 12.92
        } else {
            ((rs_rgb + 0.055) / 1.055).powf(2.4)
        };

        let g = if gs_rgb <= 0.03928 {
            gs_rgb / 12.92
        } else {
            ((gs_rgb + 0.055) / 1.055).powf(2.4)
        };

        let b = if bs_rgb <= 0.03928 {
            bs_rgb / 12.92
        } else {
            ((bs_rgb + 0.055) / 1.055).powf(2.4)
        };

        0.2126 * r + 0.7152 * g + 0.0722 * b
    }

    /// Calculate contrast ratio between two colors
    pub fn contrast_ratio(color1: (u8, u8, u8), color2: (u8, u8, u8)) -> f64 {
        let lum1 = Self::luminance(color1.0, color1.1, color1.2);
        let lum2 = Self::luminance(color2.0, color2.1, color2.2);

        let lighter = lum1.max(lum2);
        let darker = lum1.min(lum2);

        (lighter + 0.05) / (darker + 0.05)
    }

    /// Check if contrast meets WCAG AA standard
    pub fn meets_wcag_aa(
        text_color: (u8, u8, u8),
        bg_color: (u8, u8, u8),
        large_text: bool,
    ) -> bool {
        let ratio = Self::contrast_ratio(text_color, bg_color);
        if large_text {
            ratio >= 3.0 // 3:1 for large text
        } else {
            ratio >= 4.5 // 4.5:1 for normal text
        }
    }

    /// Check if contrast meets WCAG AAA standard
    pub fn meets_wcag_aaa(
        text_color: (u8, u8, u8),
        bg_color: (u8, u8, u8),
        large_text: bool,
    ) -> bool {
        let ratio = Self::contrast_ratio(text_color, bg_color);
        if large_text {
            ratio >= 4.5 // 4.5:1 for large text
        } else {
            ratio >= 7.0 // 7:1 for normal text
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aria_role_strings() {
        assert_eq!(AriaRole::Button.as_str(), "button");
        assert_eq!(AriaRole::Navigation.as_str(), "navigation");
        assert_eq!(AriaRole::Dialog.as_str(), "dialog");
    }

    #[test]
    fn test_aria_attributes() {
        let attr = AriaAttribute::Expanded(true);
        let (name, value) = attr.to_attr();
        assert_eq!(name, "aria-expanded");
        assert_eq!(value, "true");

        let attr2 = AriaAttribute::Label("Close".to_string());
        let (name2, value2) = attr2.to_attr();
        assert_eq!(name2, "aria-label");
        assert_eq!(value2, "Close");
    }

    #[test]
    fn test_a11y_props_builder() {
        let props = A11yProps::new()
            .role(AriaRole::Button)
            .attr(AriaAttribute::Label("Submit".to_string()))
            .tab_index(0)
            .focusable();

        assert_eq!(props.role, Some(AriaRole::Button));
        assert!(props.focusable);
        assert_eq!(props.tab_index, Some(0));
    }

    #[test]
    fn test_color_contrast() {
        // Black and white should have high contrast
        let black = (0, 0, 0);
        let white = (255, 255, 255);
        let ratio = ColorContrast::contrast_ratio(black, white);
        assert!(ratio > 20.0);

        // Check WCAG compliance
        assert!(ColorContrast::meets_wcag_aa(black, white, false));
        assert!(ColorContrast::meets_wcag_aaa(black, white, false));
    }

    #[test]
    fn test_announcement_priority() {
        let config = AccessibilityConfig::default();
        let mut manager = AccessibilityManager::new(&config);

        manager.announce("Loading complete", AnnouncementPriority::Medium);
        assert_eq!(manager.announcements.len(), 1);
        assert_eq!(manager.announcements[0].live, AriaLive::Polite);

        manager.announce("Error occurred", AnnouncementPriority::High);
        assert_eq!(manager.announcements[1].live, AriaLive::Assertive);
    }

    #[test]
    fn test_focus_tracking() {
        let config = AccessibilityConfig::default();
        let mut manager = AccessibilityManager::new(&config);

        manager.track_focus("button-1");
        manager.track_focus("input-1");
        manager.track_focus("button-2");

        assert_eq!(manager.previous_focus(), Some(&"input-1".to_string()));
    }

    #[test]
    fn test_colorblind_types() {
        let config = AccessibilityConfig {
            colorblind_mode: Some(ColorblindType::Deuteranopia),
            ..Default::default()
        };

        let manager = AccessibilityManager::new(&config);
        assert_eq!(
            manager.colorblind_adjustment(),
            Some(ColorblindType::Deuteranopia)
        );
    }
}
