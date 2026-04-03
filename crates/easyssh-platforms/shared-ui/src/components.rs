//! Shared UI Components
//!
//! Cross-platform UI component definitions and utilities.
//! Platform-specific implementations use these as specifications.

use crate::accessibility::{A11yProps, AriaRole};
use crate::animations::Animation;
use crate::icons::IconId;
use crate::layout::Spacing;
use crate::theme::{Color, Theme};
use serde::{Deserialize, Serialize};

/// Button variants
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ButtonVariant {
    /// Primary action button
    #[default]
    Primary,
    /// Secondary action button
    Secondary,
    /// Ghost/transparent button
    Ghost,
    /// Destructive action
    Danger,
    /// Success action
    Success,
    /// Icon-only button
    Icon,
}

/// Button sizes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ButtonSize {
    /// Extra small
    Xs,
    /// Small
    Sm,
    /// Medium (default)
    #[default]
    Md,
    /// Large
    Lg,
    /// Extra large
    Xl,
}

impl ButtonSize {
    /// Get button height in pixels
    pub fn height_px(&self) -> u32 {
        match self {
            ButtonSize::Xs => 24,
            ButtonSize::Sm => 32,
            ButtonSize::Md => 36,
            ButtonSize::Lg => 44,
            ButtonSize::Xl => 52,
        }
    }

    /// Get horizontal padding in pixels
    pub fn padding_px(&self) -> u32 {
        match self {
            ButtonSize::Xs => 8,
            ButtonSize::Sm => 12,
            ButtonSize::Md => 16,
            ButtonSize::Lg => 20,
            ButtonSize::Xl => 24,
        }
    }
}

/// Button component specification
#[derive(Debug, Clone)]
pub struct Button {
    /// Button label
    pub label: String,
    /// Button variant
    pub variant: ButtonVariant,
    /// Button size
    pub size: ButtonSize,
    /// Leading icon
    pub icon: Option<IconId>,
    /// Trailing icon
    pub trailing_icon: Option<IconId>,
    /// Loading state
    pub loading: bool,
    /// Disabled state
    pub disabled: bool,
    /// Full width
    pub full_width: bool,
    /// Accessibility properties
    pub a11y: A11yProps,
    /// Animation on press
    pub press_animation: Option<Animation>,
}

impl Button {
    /// Create a new button
    pub fn new(label: &str) -> Self {
        Self {
            label: label.to_string(),
            variant: ButtonVariant::Primary,
            size: ButtonSize::Md,
            icon: None,
            trailing_icon: None,
            loading: false,
            disabled: false,
            full_width: false,
            a11y: A11yProps::new().role(AriaRole::Button).focusable(),
            press_animation: Some(Animation::ScaleOut),
        }
    }

    /// Set variant
    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Set size
    pub fn size(mut self, size: ButtonSize) -> Self {
        self.size = size;
        self
    }

    /// Set icon
    pub fn icon(mut self, icon: IconId) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Set loading state
    pub fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Set disabled state
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Set full width
    pub fn full_width(mut self) -> Self {
        self.full_width = true;
        self
    }

    /// Get button colors based on theme
    pub fn colors(&self, theme: &Theme) -> ButtonColors {
        match self.variant {
            ButtonVariant::Primary => ButtonColors {
                background: theme.colors.interactive_primary.clone(),
                background_hover: theme.colors.interactive_primary_hover.clone(),
                background_active: theme.colors.interactive_primary_active.clone(),
                text: theme.colors.text_inverted.clone(),
                border: None,
            },
            ButtonVariant::Secondary => ButtonColors {
                background: theme.colors.interactive_secondary.clone(),
                background_hover: theme.colors.interactive_secondary_hover.clone(),
                background_active: theme.colors.interactive_secondary_hover.clone(),
                text: theme.colors.text_primary.clone(),
                border: Some(theme.colors.border_default.clone()),
            },
            ButtonVariant::Ghost => ButtonColors {
                background: Color::rgba(0, 0, 0, 0),
                background_hover: theme.colors.interactive_ghost_hover.clone(),
                background_active: theme.colors.interactive_ghost_hover.clone(),
                text: theme.colors.text_primary.clone(),
                border: None,
            },
            ButtonVariant::Danger => ButtonColors {
                background: Color::hex("#EF4444"),
                background_hover: Color::hex("#DC2626"),
                background_active: Color::hex("#B91C1C"),
                text: Color::hex("#FFFFFF"),
                border: None,
            },
            ButtonVariant::Success => ButtonColors {
                background: Color::hex("#22C55E"),
                background_hover: Color::hex("#16A34A"),
                background_active: Color::hex("#15803D"),
                text: Color::hex("#FFFFFF"),
                border: None,
            },
            ButtonVariant::Icon => ButtonColors {
                background: Color::rgba(0, 0, 0, 0),
                background_hover: theme.colors.interactive_ghost_hover.clone(),
                background_active: theme.colors.interactive_ghost_hover.clone(),
                text: theme.colors.text_secondary.clone(),
                border: None,
            },
        }
    }
}

/// Button color scheme
#[derive(Debug, Clone)]
pub struct ButtonColors {
    /// Background color
    pub background: Color,
    /// Hover background
    pub background_hover: Color,
    /// Active/pressed background
    pub background_active: Color,
    /// Text color
    pub text: Color,
    /// Border color (optional)
    pub border: Option<Color>,
}

/// Input field variants
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InputVariant {
    /// Default input
    #[default]
    Default,
    /// Filled input
    Filled,
    /// Outlined input
    Outlined,
    /// Ghost input
    Ghost,
}

/// Input component specification
#[derive(Debug, Clone)]
pub struct Input {
    /// Input value
    pub value: String,
    /// Placeholder text
    pub placeholder: String,
    /// Input label
    pub label: Option<String>,
    /// Helper text
    pub helper: Option<String>,
    /// Error message
    pub error: Option<String>,
    /// Input variant
    pub variant: InputVariant,
    /// Disabled state
    pub disabled: bool,
    /// Read-only state
    pub read_only: bool,
    /// Required field
    pub required: bool,
    /// Leading icon
    pub icon: Option<IconId>,
    /// Clear button
    pub clearable: bool,
    /// Password visibility toggle
    pub password_toggle: bool,
    /// Focused state
    pub focused: bool,
    /// Accessibility properties
    pub a11y: A11yProps,
}

impl Input {
    /// Create a new input
    pub fn new(placeholder: &str) -> Self {
        Self {
            value: String::new(),
            placeholder: placeholder.to_string(),
            label: None,
            helper: None,
            error: None,
            variant: InputVariant::Default,
            disabled: false,
            read_only: false,
            required: false,
            icon: None,
            clearable: false,
            password_toggle: false,
            focused: false,
            a11y: A11yProps::new().role(AriaRole::Textbox).focusable(),
        }
    }

    /// Set label
    pub fn label(mut self, label: &str) -> Self {
        self.label = Some(label.to_string());
        self
    }

    /// Set error
    pub fn error(mut self, error: &str) -> Self {
        self.error = Some(error.to_string());
        self
    }

    /// Set icon
    pub fn icon(mut self, icon: IconId) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Make clearable
    pub fn clearable(mut self) -> Self {
        self.clearable = true;
        self
    }

    /// Make password input
    pub fn password(mut self) -> Self {
        self.password_toggle = true;
        self
    }
}

/// Card component specification
#[derive(Debug, Clone)]
pub struct Card {
    /// Card title
    pub title: Option<String>,
    /// Card content
    pub content: String,
    /// Header icon
    pub icon: Option<IconId>,
    /// Header actions
    pub actions: Vec<CardAction>,
    /// Card elevation (shadow level)
    pub elevation: u32, // 0-5
    /// Clickable card
    pub clickable: bool,
    /// Disabled state
    pub disabled: bool,
    /// Hover animation
    pub hover_animation: bool,
    /// Accessibility properties
    pub a11y: A11yProps,
}

/// Card action button
#[derive(Debug, Clone)]
pub struct CardAction {
    /// Action icon
    pub icon: IconId,
    /// Action label
    pub label: String,
    /// Disabled state
    pub disabled: bool,
}

impl Card {
    /// Create a new card
    pub fn new(content: &str) -> Self {
        Self {
            title: None,
            content: content.to_string(),
            icon: None,
            actions: Vec::new(),
            elevation: 1,
            clickable: false,
            disabled: false,
            hover_animation: true,
            a11y: A11yProps::new(),
        }
    }

    /// Set title
    pub fn title(mut self, title: &str) -> Self {
        self.title = Some(title.to_string());
        self
    }

    /// Set icon
    pub fn icon(mut self, icon: IconId) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Add action
    pub fn action(mut self, icon: IconId, label: &str) -> Self {
        self.actions.push(CardAction {
            icon,
            label: label.to_string(),
            disabled: false,
        });
        self
    }

    /// Set elevation
    pub fn elevation(mut self, elevation: u32) -> Self {
        self.elevation = elevation.min(5);
        self
    }

    /// Make clickable
    pub fn clickable(mut self) -> Self {
        self.clickable = true;
        self.a11y = self.a11y.role(AriaRole::Button);
        self
    }
}

/// Badge/Tag component
#[derive(Debug, Clone)]
pub struct Badge {
    /// Badge text
    pub text: String,
    /// Badge style
    pub variant: BadgeVariant,
    /// Badge size
    pub size: BadgeSize,
    /// Dot indicator
    pub dot: bool,
    /// Count (for numeric badges)
    pub count: Option<u32>,
    /// Max count to display
    pub max_count: Option<u32>,
}

/// Badge variants
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BadgeVariant {
    #[default]
    Default,
    Primary,
    Success,
    Warning,
    Danger,
    Info,
}

/// Badge sizes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BadgeSize {
    #[default]
    Sm,
    Md,
    Lg,
}

impl Badge {
    /// Create a new badge
    pub fn new(text: &str) -> Self {
        Self {
            text: text.to_string(),
            variant: BadgeVariant::Default,
            size: BadgeSize::Sm,
            dot: false,
            count: None,
            max_count: Some(99),
        }
    }

    /// Set variant
    pub fn variant(mut self, variant: BadgeVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Set count
    pub fn count(mut self, count: u32) -> Self {
        self.count = Some(count);
        self
    }

    /// Show dot
    pub fn dot(mut self) -> Self {
        self.dot = true;
        self
    }

    /// Get display text
    pub fn display_text(&self) -> String {
        if let Some(count) = self.count {
            if let Some(max) = self.max_count {
                if count > max {
                    return format!("{}+", max);
                }
            }
            return count.to_string();
        }
        self.text.clone()
    }
}

/// Toast/Notification component
#[derive(Debug, Clone)]
pub struct Toast {
    /// Toast title
    pub title: String,
    /// Toast message
    pub message: String,
    /// Toast type
    pub toast_type: ToastType,
    /// Duration in milliseconds
    pub duration: u32,
    /// Show close button
    pub closable: bool,
    /// Show progress bar
    pub show_progress: bool,
    /// Icon override
    pub icon: Option<IconId>,
    /// Action button
    pub action: Option<ToastAction>,
}

/// Toast types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ToastType {
    #[default]
    Info,
    Success,
    Warning,
    Error,
}

impl ToastType {
    /// Get default icon
    pub fn default_icon(&self) -> IconId {
        match self {
            ToastType::Info => IconId::Info,
            ToastType::Success => IconId::Check,
            ToastType::Warning => IconId::AlertTriangle,
            ToastType::Error => IconId::AlertCircle,
        }
    }
}

/// Toast action
#[derive(Debug, Clone)]
pub struct ToastAction {
    /// Action label
    pub label: String,
}

impl Toast {
    /// Create a new toast
    pub fn new(title: &str, message: &str) -> Self {
        Self {
            title: title.to_string(),
            message: message.to_string(),
            toast_type: ToastType::Info,
            duration: 4000,
            closable: true,
            show_progress: true,
            icon: None,
            action: None,
        }
    }

    /// Set type
    pub fn toast_type(mut self, toast_type: ToastType) -> Self {
        self.toast_type = toast_type;
        self
    }

    /// Set duration
    pub fn duration(mut self, duration: u32) -> Self {
        self.duration = duration;
        self
    }

    /// Add action
    pub fn action(mut self, label: &str) -> Self {
        self.action = Some(ToastAction {
            label: label.to_string(),
        });
        self
    }

    /// Get icon
    pub fn icon(&self) -> IconId {
        self.icon
            .clone()
            .unwrap_or_else(|| self.toast_type.default_icon())
    }
}

/// Modal/Dialog component
#[derive(Debug, Clone)]
pub struct Modal {
    /// Modal title
    pub title: String,
    /// Modal content
    pub content: String,
    /// Modal size
    pub size: ModalSize,
    /// Close on overlay click
    pub close_on_overlay: bool,
    /// Show close button
    pub show_close: bool,
    /// Primary action
    pub primary_action: Option<ModalAction>,
    /// Secondary action
    pub secondary_action: Option<ModalAction>,
    /// Danger action (destructive)
    pub danger_action: Option<ModalAction>,
    /// Loading state
    pub loading: bool,
    /// Accessibility properties
    pub a11y: A11yProps,
}

/// Modal sizes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ModalSize {
    /// Small (400px)
    Sm,
    /// Medium (500px) - default
    #[default]
    Md,
    /// Large (600px)
    Lg,
    /// Extra large (800px)
    Xl,
    /// Full screen
    Fullscreen,
}

impl ModalSize {
    /// Get max width in pixels
    pub fn max_width_px(&self) -> Option<u32> {
        match self {
            ModalSize::Sm => Some(400),
            ModalSize::Md => Some(500),
            ModalSize::Lg => Some(600),
            ModalSize::Xl => Some(800),
            ModalSize::Fullscreen => None,
        }
    }
}

/// Modal action
#[derive(Debug, Clone)]
pub struct ModalAction {
    /// Action label
    pub label: String,
    /// Disabled state
    pub disabled: bool,
    /// Loading state
    pub loading: bool,
}

impl Modal {
    /// Create a new modal
    pub fn new(title: &str, content: &str) -> Self {
        Self {
            title: title.to_string(),
            content: content.to_string(),
            size: ModalSize::Md,
            close_on_overlay: true,
            show_close: true,
            primary_action: None,
            secondary_action: None,
            danger_action: None,
            loading: false,
            a11y: A11yProps::new()
                .role(AriaRole::Dialog)
                .attr(crate::accessibility::AriaAttribute::Modal(true)),
        }
    }

    /// Set primary action
    pub fn primary_action(mut self, label: &str) -> Self {
        self.primary_action = Some(ModalAction {
            label: label.to_string(),
            disabled: false,
            loading: false,
        });
        self
    }

    /// Set secondary action
    pub fn secondary_action(mut self, label: &str) -> Self {
        self.secondary_action = Some(ModalAction {
            label: label.to_string(),
            disabled: false,
            loading: false,
        });
        self
    }

    /// Set size
    pub fn size(mut self, size: ModalSize) -> Self {
        self.size = size;
        self
    }

    /// Prevent closing on overlay click
    pub fn no_close_on_overlay(mut self) -> Self {
        self.close_on_overlay = false;
        self
    }
}

/// List component specification
#[derive(Debug, Clone)]
pub struct List {
    /// List items
    pub items: Vec<ListItem>,
    /// List variant
    pub variant: ListVariant,
    /// Selectable items
    pub selectable: bool,
    /// Multi-select
    pub multi_select: bool,
    /// Enable drag and drop
    pub draggable: bool,
    /// Empty state message
    pub empty_message: String,
    /// Loading state
    pub loading: bool,
    /// Accessibility properties
    pub a11y: A11yProps,
}

/// List variants
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ListVariant {
    /// Default list
    #[default]
    Default,
    /// Bordered items
    Bordered,
    /// Striped items
    Striped,
    /// Compact list
    Compact,
}

/// List item
#[derive(Debug, Clone)]
pub struct ListItem {
    /// Item ID
    pub id: String,
    /// Primary text
    pub title: String,
    /// Secondary text
    pub subtitle: Option<String>,
    /// Leading icon
    pub icon: Option<IconId>,
    /// Trailing element (text or icon)
    pub trailing: Option<String>,
    /// Selected state
    pub selected: bool,
    /// Disabled state
    pub disabled: bool,
    /// Clickable
    pub clickable: bool,
}

impl List {
    /// Create a new list
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            variant: ListVariant::Default,
            selectable: false,
            multi_select: false,
            draggable: false,
            empty_message: "No items".to_string(),
            loading: false,
            a11y: A11yProps::new().role(AriaRole::List),
        }
    }

    /// Add item
    pub fn item(mut self, id: &str, title: &str) -> Self {
        self.items.push(ListItem {
            id: id.to_string(),
            title: title.to_string(),
            subtitle: None,
            icon: None,
            trailing: None,
            selected: false,
            disabled: false,
            clickable: true,
        });
        self
    }

    /// Make selectable
    pub fn selectable(mut self) -> Self {
        self.selectable = true;
        self
    }

    /// Enable multi-select
    pub fn multi_select(mut self) -> Self {
        self.selectable = true;
        self.multi_select = true;
        self
    }
}

/// Tabs component
#[derive(Debug, Clone)]
pub struct Tabs {
    /// Tab items
    pub tabs: Vec<TabItem>,
    /// Active tab index
    pub active_index: usize,
    /// Tab variant
    pub variant: TabsVariant,
    /// Full width tabs
    pub full_width: bool,
    /// Accessibility properties
    pub a11y: A11yProps,
}

/// Tab variants
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TabsVariant {
    /// Default tabs with underline
    #[default]
    Default,
    /// Enclosed tabs
    Enclosed,
    /// Pills style
    Pills,
    /// Buttons style
    Buttons,
}

/// Tab item
#[derive(Debug, Clone)]
pub struct TabItem {
    /// Tab label
    pub label: String,
    /// Tab icon
    pub icon: Option<IconId>,
    /// Disabled
    pub disabled: bool,
    /// Badge count
    pub badge: Option<u32>,
}

impl Tabs {
    /// Create new tabs
    pub fn new() -> Self {
        Self {
            tabs: Vec::new(),
            active_index: 0,
            variant: TabsVariant::Default,
            full_width: false,
            a11y: A11yProps::new().role(AriaRole::Tablist),
        }
    }

    /// Add tab
    pub fn tab(mut self, label: &str) -> Self {
        self.tabs.push(TabItem {
            label: label.to_string(),
            icon: None,
            disabled: false,
            badge: None,
        });
        self
    }

    /// Set active index
    pub fn active(mut self, index: usize) -> Self {
        self.active_index = index;
        self
    }

    /// Set variant
    pub fn variant(mut self, variant: TabsVariant) -> Self {
        self.variant = variant;
        self
    }
}

/// Switch/Toggle component
#[derive(Debug, Clone)]
pub struct Switch {
    /// Current state
    pub checked: bool,
    /// Label
    pub label: Option<String>,
    /// Disabled state
    pub disabled: bool,
    /// Loading state
    pub loading: bool,
    /// Size
    pub size: SwitchSize,
    /// Accessibility properties
    pub a11y: A11yProps,
}

/// Switch sizes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SwitchSize {
    /// Small
    Sm,
    /// Medium (default)
    #[default]
    Md,
    /// Large
    Lg,
}

impl Switch {
    /// Create new switch
    pub fn new(checked: bool) -> Self {
        Self {
            checked,
            label: None,
            disabled: false,
            loading: false,
            size: SwitchSize::Md,
            a11y: A11yProps::new().role(AriaRole::Switch).focusable(),
        }
    }

    /// Set label
    pub fn label(mut self, label: &str) -> Self {
        self.label = Some(label.to_string());
        self
    }

    /// Set size
    pub fn size(mut self, size: SwitchSize) -> Self {
        self.size = size;
        self
    }
}

/// Checkbox component
#[derive(Debug, Clone)]
pub struct Checkbox {
    /// Current state
    pub checked: Option<bool>, // Some(true)=checked, Some(false)=unchecked, None=indeterminate
    /// Label
    pub label: String,
    /// Disabled state
    pub disabled: bool,
    /// Accessibility properties
    pub a11y: A11yProps,
}

impl Checkbox {
    /// Create new checkbox
    pub fn new(label: &str) -> Self {
        Self {
            checked: Some(false),
            label: label.to_string(),
            disabled: false,
            a11y: A11yProps::new().role(AriaRole::Checkbox).focusable(),
        }
    }

    /// Set checked
    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = Some(checked);
        self
    }

    /// Set indeterminate
    pub fn indeterminate(mut self) -> Self {
        self.checked = None;
        self
    }

    /// Check if checked (false if indeterminate)
    pub fn is_checked(&self) -> bool {
        self.checked == Some(true)
    }

    /// Check if indeterminate
    pub fn is_indeterminate(&self) -> bool {
        self.checked.is_none()
    }
}

/// Radio button component
#[derive(Debug, Clone)]
pub struct Radio {
    /// Current state
    pub selected: bool,
    /// Label
    pub label: String,
    /// Disabled state
    pub disabled: bool,
    /// Group name
    pub name: String,
    /// Value
    pub value: String,
    /// Accessibility properties
    pub a11y: A11yProps,
}

impl Radio {
    /// Create new radio button
    pub fn new(label: &str, name: &str, value: &str) -> Self {
        Self {
            selected: false,
            label: label.to_string(),
            disabled: false,
            name: name.to_string(),
            value: value.to_string(),
            a11y: A11yProps::new().role(AriaRole::Radio).focusable(),
        }
    }

    /// Set selected
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }
}

/// Progress component
#[derive(Debug, Clone)]
pub struct Progress {
    /// Current value
    pub value: f32,
    /// Maximum value
    pub max: f32,
    /// Show percentage label
    pub show_label: bool,
    /// Indeterminate state
    pub indeterminate: bool,
    /// Size
    pub size: ProgressSize,
    /// Variant
    pub variant: ProgressVariant,
    /// Accessibility properties
    pub a11y: A11yProps,
}

/// Progress sizes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProgressSize {
    /// Thin (2px)
    Thin,
    /// Small (4px)
    Sm,
    /// Medium (8px) - default
    #[default]
    Md,
    /// Large (12px)
    Lg,
}

/// Progress variants
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProgressVariant {
    /// Default (bar)
    #[default]
    Bar,
    /// Circular
    Circular,
    /// Steps
    Steps,
}

impl Progress {
    /// Create new progress bar
    pub fn new(value: f32, max: f32) -> Self {
        Self {
            value,
            max,
            show_label: false,
            indeterminate: false,
            size: ProgressSize::Md,
            variant: ProgressVariant::Bar,
            a11y: A11yProps::new().role(AriaRole::ProgressBar),
        }
    }

    /// Create indeterminate progress
    pub fn indeterminate() -> Self {
        Self {
            value: 0.0,
            max: 100.0,
            show_label: false,
            indeterminate: true,
            size: ProgressSize::Md,
            variant: ProgressVariant::Bar,
            a11y: A11yProps::new().role(AriaRole::ProgressBar),
        }
    }

    /// Get percentage
    pub fn percentage(&self) -> f32 {
        if self.max <= 0.0 {
            0.0
        } else {
            (self.value / self.max * 100.0).clamp(0.0, 100.0)
        }
    }

    /// Show label
    pub fn with_label(mut self) -> Self {
        self.show_label = true;
        self
    }

    /// Set size
    pub fn size(mut self, size: ProgressSize) -> Self {
        self.size = size;
        self
    }
}

/// Skeleton loader component
#[derive(Debug, Clone)]
pub struct Skeleton {
    /// Width (can be percentage or pixels)
    pub width: String,
    /// Height
    pub height: String,
    /// Circle shape
    pub circle: bool,
    /// Animation enabled
    pub animate: bool,
}

impl Skeleton {
    /// Create new skeleton
    pub fn new() -> Self {
        Self {
            width: "100%".to_string(),
            height: "16px".to_string(),
            circle: false,
            animate: true,
        }
    }

    /// Create circle skeleton
    pub fn circle(size: &str) -> Self {
        Self {
            width: size.to_string(),
            height: size.to_string(),
            circle: true,
            animate: true,
        }
    }

    /// Set dimensions
    pub fn dimensions(mut self, width: &str, height: &str) -> Self {
        self.width = width.to_string();
        self.height = height.to_string();
        self.circle = false;
        self
    }
}

/// Divider/Separator component
#[derive(Debug, Clone)]
pub struct Divider {
    /// Orientation
    pub orientation: DividerOrientation,
    /// Thickness in pixels
    pub thickness: u32,
    /// With label
    pub label: Option<String>,
    /// Spacing
    pub spacing: Spacing,
}

/// Divider orientation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DividerOrientation {
    /// Horizontal
    #[default]
    Horizontal,
    /// Vertical
    Vertical,
}

impl Divider {
    /// Create horizontal divider
    pub fn horizontal() -> Self {
        Self {
            orientation: DividerOrientation::Horizontal,
            thickness: 1,
            label: None,
            spacing: Spacing::Base,
        }
    }

    /// Create vertical divider
    pub fn vertical() -> Self {
        Self {
            orientation: DividerOrientation::Vertical,
            thickness: 1,
            label: None,
            spacing: Spacing::Base,
        }
    }

    /// Set label
    pub fn label(mut self, label: &str) -> Self {
        self.label = Some(label.to_string());
        self
    }
}

/// Tooltip component
#[derive(Debug, Clone)]
pub struct Tooltip {
    /// Tooltip content
    pub content: String,
    /// Position
    pub position: TooltipPosition,
    /// Delay before showing (ms)
    pub delay: u32,
    /// Max width
    pub max_width: u32,
}

/// Tooltip positions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TooltipPosition {
    /// Top
    #[default]
    Top,
    /// Bottom
    Bottom,
    /// Left
    Left,
    /// Right
    Right,
}

impl Tooltip {
    /// Create new tooltip
    pub fn new(content: &str) -> Self {
        Self {
            content: content.to_string(),
            position: TooltipPosition::Top,
            delay: 300,
            max_width: 240,
        }
    }

    /// Set position
    pub fn position(mut self, position: TooltipPosition) -> Self {
        self.position = position;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_button_builder() {
        let button = Button::new("Submit")
            .variant(ButtonVariant::Primary)
            .size(ButtonSize::Lg)
            .icon(IconId::Check)
            .full_width();

        assert_eq!(button.label, "Submit");
        assert_eq!(button.variant, ButtonVariant::Primary);
        assert_eq!(button.size, ButtonSize::Lg);
        assert!(button.full_width);
    }

    #[test]
    fn test_button_sizes() {
        assert_eq!(ButtonSize::Xs.height_px(), 24);
        assert_eq!(ButtonSize::Xl.height_px(), 52);
        assert_eq!(ButtonSize::Md.padding_px(), 16);
    }

    #[test]
    fn test_input_builder() {
        let input = Input::new("Enter name")
            .label("Name")
            .error("Required field")
            .icon(IconId::User)
            .clearable();

        assert_eq!(input.placeholder, "Enter name");
        assert_eq!(input.label, Some("Name".to_string()));
        assert_eq!(input.error, Some("Required field".to_string()));
        assert!(input.clearable);
    }

    #[test]
    fn test_badge_display() {
        let badge = Badge::new("New").count(150).count(99);
        assert_eq!(badge.display_text(), "99");

        let badge2 = Badge::new("New").count(150);
        assert_eq!(badge2.display_text(), "99+");

        let badge3 = Badge::new("New").count(50);
        assert_eq!(badge3.display_text(), "50");
    }

    #[test]
    fn test_modal_builder() {
        let modal = Modal::new("Confirm", "Are you sure?")
            .primary_action("Yes")
            .secondary_action("Cancel")
            .size(ModalSize::Sm)
            .no_close_on_overlay();

        assert_eq!(modal.title, "Confirm");
        assert!(modal.primary_action.is_some());
        assert!(modal.secondary_action.is_some());
        assert!(!modal.close_on_overlay);
    }

    #[test]
    fn test_progress_percentage() {
        let progress = Progress::new(50.0, 100.0);
        assert_eq!(progress.percentage(), 50.0);

        let progress2 = Progress::new(75.0, 200.0);
        assert_eq!(progress2.percentage(), 37.5);
    }

    #[test]
    fn test_checkbox_states() {
        let checked = Checkbox::new("Option").checked(true);
        assert!(checked.is_checked());
        assert!(!checked.is_indeterminate());

        let indeterminate = Checkbox::new("Option").indeterminate();
        assert!(!indeterminate.is_checked());
        assert!(indeterminate.is_indeterminate());
    }

    #[test]
    fn test_toast_builder() {
        let toast = Toast::new("Success", "Operation completed")
            .toast_type(ToastType::Success)
            .duration(5000)
            .action("View");

        assert_eq!(toast.title, "Success");
        assert_eq!(toast.toast_type, ToastType::Success);
        assert!(toast.action.is_some());
    }

    #[test]
    fn test_list_builder() {
        let list = List::new()
            .item("1", "First")
            .item("2", "Second")
            .selectable();

        assert_eq!(list.items.len(), 2);
        assert!(list.selectable);
    }

    #[test]
    fn test_tabs_builder() {
        let tabs = Tabs::new()
            .tab("General")
            .tab("Advanced")
            .active(1)
            .variant(TabsVariant::Enclosed);

        assert_eq!(tabs.tabs.len(), 2);
        assert_eq!(tabs.active_index, 1);
    }

    #[test]
    fn test_skeleton() {
        let skeleton = Skeleton::new().dimensions("200px", "20px");
        assert_eq!(skeleton.width, "200px");
        assert!(!skeleton.circle);

        let circle = Skeleton::circle("48px");
        assert!(circle.circle);
    }
}
