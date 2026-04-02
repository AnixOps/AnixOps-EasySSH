//! Layout System
//!
//! Responsive layout primitives for cross-platform UI:
//! - Breakpoint-based responsive design
//! - Spacing utilities
//! - Grid and flexbox abstractions
//! - Platform-aware layout

use serde::{Deserialize, Serialize};

/// Breakpoint definitions for responsive design
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Breakpoint {
    /// Extra small (phones): < 480px
    Xs,
    /// Small (large phones): 480px - 639px
    Sm,
    /// Medium (tablets): 640px - 767px
    Md,
    /// Large (small laptops): 768px - 1023px
    Lg,
    /// Extra large (desktops): 1024px - 1279px
    Xl,
    /// 2x extra large (large screens): 1280px+
    Xxl,
}

impl Breakpoint {
    /// Get breakpoint min width in pixels
    pub fn min_px(&self) -> u32 {
        match self {
            Breakpoint::Xs => 0,
            Breakpoint::Sm => 480,
            Breakpoint::Md => 640,
            Breakpoint::Lg => 768,
            Breakpoint::Xl => 1024,
            Breakpoint::Xxl => 1280,
        }
    }

    /// Get breakpoint max width in pixels
    pub fn max_px(&self) -> Option<u32> {
        match self {
            Breakpoint::Xs => Some(479),
            Breakpoint::Sm => Some(639),
            Breakpoint::Md => Some(767),
            Breakpoint::Lg => Some(1023),
            Breakpoint::Xl => Some(1279),
            Breakpoint::Xxl => None, // No upper bound
        }
    }

    /// Get CSS media query for this breakpoint
    pub fn to_css(&self) -> String {
        match self.max_px() {
            Some(max) => format!(
                "(min-width: {}px) and (max-width: {}px)",
                self.min_px(),
                max
            ),
            None => format!("(min-width: {}px)", self.min_px()),
        }
    }

    /// Get next larger breakpoint
    pub fn up(&self) -> Option<Self> {
        match self {
            Breakpoint::Xs => Some(Breakpoint::Sm),
            Breakpoint::Sm => Some(Breakpoint::Md),
            Breakpoint::Md => Some(Breakpoint::Lg),
            Breakpoint::Lg => Some(Breakpoint::Xl),
            Breakpoint::Xl => Some(Breakpoint::Xxl),
            Breakpoint::Xxl => None,
        }
    }

    /// Get next smaller breakpoint
    pub fn down(&self) -> Option<Self> {
        match self {
            Breakpoint::Xs => None,
            Breakpoint::Sm => Some(Breakpoint::Xs),
            Breakpoint::Md => Some(Breakpoint::Sm),
            Breakpoint::Lg => Some(Breakpoint::Md),
            Breakpoint::Xl => Some(Breakpoint::Lg),
            Breakpoint::Xxl => Some(Breakpoint::Xl),
        }
    }

    /// Get breakpoint from width
    pub fn from_width(width: u32) -> Self {
        match width {
            0..=479 => Breakpoint::Xs,
            480..=639 => Breakpoint::Sm,
            640..=767 => Breakpoint::Md,
            768..=1023 => Breakpoint::Lg,
            1024..=1279 => Breakpoint::Xl,
            _ => Breakpoint::Xxl,
        }
    }
}

/// Spacing values (4px base grid)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Spacing {
    /// 0
    None,
    /// 4px (1 unit)
    Xs,
    /// 8px (2 units)
    Sm,
    /// 12px (3 units)
    Md,
    /// 16px (4 units)
    Base,
    /// 20px (5 units)
    Lg,
    /// 24px (6 units)
    Xl,
    /// 32px (8 units)
    Xxl,
    /// 48px (12 units)
    Xxxl,
}

impl Spacing {
    /// Get spacing in pixels
    pub fn as_px(&self) -> u32 {
        match self {
            Spacing::None => 0,
            Spacing::Xs => 4,
            Spacing::Sm => 8,
            Spacing::Md => 12,
            Spacing::Base => 16,
            Spacing::Lg => 20,
            Spacing::Xl => 24,
            Spacing::Xxl => 32,
            Spacing::Xxxl => 48,
        }
    }

    /// Get spacing as CSS value
    pub fn to_css(&self) -> String {
        format!("{}px", self.as_px())
    }

    /// Get spacing as f32 for calculations
    pub fn as_f32(&self) -> f32 {
        self.as_px() as f32
    }
}

/// Layout direction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LayoutDirection {
    /// Horizontal layout
    Horizontal,
    /// Vertical layout
    Vertical,
}

/// Layout alignment
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Alignment {
    /// Start alignment
    Start,
    /// Center alignment
    Center,
    /// End alignment
    End,
    /// Space between
    SpaceBetween,
    /// Space around
    SpaceAround,
    /// Space evenly
    SpaceEvenly,
}

/// Layout configuration
#[derive(Debug, Clone)]
pub struct LayoutConfig {
    /// Base breakpoint
    pub breakpoint: Breakpoint,
    /// Container padding
    pub padding: Spacing,
    /// Content gap
    pub gap: Spacing,
    /// Max content width
    pub max_width: Option<u32>,
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            breakpoint: Breakpoint::Xl,
            padding: Spacing::Base,
            gap: Spacing::Base,
            max_width: Some(1280),
        }
    }
}

/// Responsive layout manager
pub struct ResponsiveLayout {
    config: LayoutConfig,
    current_width: u32,
    current_breakpoint: Breakpoint,
}

impl ResponsiveLayout {
    /// Create a new responsive layout manager
    pub fn new(config: &LayoutConfig) -> Self {
        let width = 1024; // Default width
        Self {
            config: config.clone(),
            current_width: width,
            current_breakpoint: Breakpoint::from_width(width),
        }
    }

    /// Update viewport width
    pub fn set_width(&mut self, width: u32) {
        self.current_width = width;
        self.current_breakpoint = Breakpoint::from_width(width);
    }

    /// Get current breakpoint
    pub fn breakpoint(&self) -> Breakpoint {
        self.current_breakpoint
    }

    /// Check if current breakpoint is at least the given breakpoint
    pub fn is_at_least(&self, breakpoint: Breakpoint) -> bool {
        self.current_width >= breakpoint.min_px()
    }

    /// Check if current breakpoint is at most the given breakpoint
    pub fn is_at_most(&self, breakpoint: Breakpoint) -> bool {
        match breakpoint.max_px() {
            Some(max) => self.current_width <= max,
            None => true,
        }
    }

    /// Get responsive value based on breakpoint
    pub fn responsive_value<T>(&self, values: &[(Breakpoint, T)]) -> &T {
        // Find the value for current or nearest smaller breakpoint
        for (bp, value) in values.iter().rev() {
            if self.is_at_least(*bp) {
                return value;
            }
        }
        // Return first value as fallback
        &values[0].1
    }

    /// Get container padding for current breakpoint
    pub fn container_padding(&self) -> Spacing {
        match self.current_breakpoint {
            Breakpoint::Xs => Spacing::Sm,
            Breakpoint::Sm => Spacing::Base,
            _ => Spacing::Lg,
        }
    }

    /// Get content gap for current breakpoint
    pub fn content_gap(&self) -> Spacing {
        match self.current_breakpoint {
            Breakpoint::Xs => Spacing::Sm,
            Breakpoint::Sm => Spacing::Md,
            _ => Spacing::Base,
        }
    }
}

/// Grid layout configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GridConfig {
    /// Number of columns
    pub columns: u32,
    /// Gap between items
    pub gap: Spacing,
    /// Minimum item width
    pub min_item_width: Option<u32>,
}

impl GridConfig {
    /// Create a responsive grid config
    pub fn responsive() -> [(Breakpoint, GridConfig); 6] {
        [
            (
                Breakpoint::Xs,
                GridConfig {
                    columns: 1,
                    gap: Spacing::Sm,
                    min_item_width: None,
                },
            ),
            (
                Breakpoint::Sm,
                GridConfig {
                    columns: 2,
                    gap: Spacing::Md,
                    min_item_width: None,
                },
            ),
            (
                Breakpoint::Md,
                GridConfig {
                    columns: 2,
                    gap: Spacing::Base,
                    min_item_width: None,
                },
            ),
            (
                Breakpoint::Lg,
                GridConfig {
                    columns: 3,
                    gap: Spacing::Base,
                    min_item_width: None,
                },
            ),
            (
                Breakpoint::Xl,
                GridConfig {
                    columns: 4,
                    gap: Spacing::Lg,
                    min_item_width: None,
                },
            ),
            (
                Breakpoint::Xxl,
                GridConfig {
                    columns: 4,
                    gap: Spacing::Xl,
                    min_item_width: None,
                },
            ),
        ]
    }
}

/// Flexbox layout configuration
#[derive(Debug, Clone, Copy)]
pub struct FlexConfig {
    /// Layout direction
    pub direction: LayoutDirection,
    /// Item alignment (cross axis)
    pub align: Alignment,
    /// Content justification (main axis)
    pub justify: Alignment,
    /// Gap between items
    pub gap: Spacing,
    /// Wrap items
    pub wrap: bool,
}

impl Default for FlexConfig {
    fn default() -> Self {
        Self {
            direction: LayoutDirection::Horizontal,
            align: Alignment::Center,
            justify: Alignment::Start,
            gap: Spacing::Base,
            wrap: false,
        }
    }
}

impl FlexConfig {
    /// Create a row flex layout
    pub fn row() -> Self {
        Self::default()
    }

    /// Create a column flex layout
    pub fn column() -> Self {
        Self {
            direction: LayoutDirection::Vertical,
            ..Default::default()
        }
    }

    /// Set alignment
    pub fn align(mut self, alignment: Alignment) -> Self {
        self.align = alignment;
        self
    }

    /// Set justification
    pub fn justify(mut self, justify: Alignment) -> Self {
        self.justify = justify;
        self
    }

    /// Set gap
    pub fn gap(mut self, gap: Spacing) -> Self {
        self.gap = gap;
        self
    }

    /// Enable wrapping
    pub fn wrap(mut self) -> Self {
        self.wrap = true;
        self
    }
}

/// Platform-specific layout adjustments
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    /// Windows desktop
    Windows,
    /// macOS desktop
    MacOS,
    /// Linux desktop
    Linux,
    /// iOS
    IOS,
    /// Android
    Android,
    /// Web
    Web,
}

impl Platform {
    /// Get platform-specific padding adjustment
    pub fn padding_multiplier(&self) -> f32 {
        match self {
            Platform::IOS => 1.2, // More padding for touch
            Platform::Android => 1.2,
            _ => 1.0, // Standard for desktop
        }
    }

    /// Get platform-specific minimum touch target
    pub fn min_touch_target(&self) -> u32 {
        match self {
            Platform::IOS | Platform::Android => 44,
            _ => 32, // Desktop
        }
    }

    /// Check if platform uses touch input
    pub fn is_touch(&self) -> bool {
        matches!(self, Platform::IOS | Platform::Android)
    }
}

/// Layout utilities
pub struct LayoutUtils;

impl LayoutUtils {
    /// Calculate responsive grid columns
    pub fn calculate_grid_columns(available_width: u32, min_item_width: u32, gap: u32) -> u32 {
        if available_width < min_item_width {
            return 1;
        }

        let effective_width = available_width.saturating_sub(gap);
        let columns = effective_width / (min_item_width + gap);
        columns.max(1)
    }

    /// Clamp value between min and max
    pub fn clamp<T: PartialOrd>(value: T, min: T, max: T) -> T {
        if value < min {
            min
        } else if value > max {
            max
        } else {
            value
        }
    }

    /// Scale spacing for platform
    pub fn scale_spacing(spacing: Spacing, platform: Platform) -> u32 {
        let base = spacing.as_px() as f32;
        (base * platform.padding_multiplier()) as u32
    }
}

/// CSS class generator for layouts
pub struct LayoutClassBuilder;

impl LayoutClassBuilder {
    /// Build flex container classes
    pub fn flex(config: &FlexConfig) -> String {
        let direction = match config.direction {
            LayoutDirection::Horizontal => "flex-row",
            LayoutDirection::Vertical => "flex-col",
        };

        let align = match config.align {
            Alignment::Start => "items-start",
            Alignment::Center => "items-center",
            Alignment::End => "items-end",
            _ => "items-center",
        };

        let justify = match config.justify {
            Alignment::Start => "justify-start",
            Alignment::Center => "justify-center",
            Alignment::End => "justify-end",
            Alignment::SpaceBetween => "justify-between",
            Alignment::SpaceAround => "justify-around",
            Alignment::SpaceEvenly => "justify-evenly",
        };

        let wrap = if config.wrap {
            "flex-wrap"
        } else {
            "flex-nowrap"
        };
        let gap = format!("gap-{}", config.gap.as_px() / 4);

        format!("flex {} {} {} {} {}", direction, align, justify, wrap, gap)
    }

    /// Build grid container classes
    pub fn grid(columns: u32, gap: Spacing) -> String {
        let gap_class = format!("gap-{}", gap.as_px() / 4);
        format!("grid grid-cols-{} {}", columns, gap_class)
    }

    /// Build container classes
    pub fn container(padding: Spacing, max_width: Option<u32>) -> String {
        let padding_class = format!("p-{}", padding.as_px() / 4);
        let max_width_class = max_width
            .map(|w| format!(" max-w-[{}px] mx-auto", w))
            .unwrap_or_default();

        format!("{}{}", padding_class, max_width_class)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_breakpoint_widths() {
        assert_eq!(Breakpoint::Xs.min_px(), 0);
        assert_eq!(Breakpoint::Sm.min_px(), 480);
        assert_eq!(Breakpoint::Xxl.min_px(), 1280);
    }

    #[test]
    fn test_breakpoint_from_width() {
        assert_eq!(Breakpoint::from_width(320), Breakpoint::Xs);
        assert_eq!(Breakpoint::from_width(500), Breakpoint::Sm);
        assert_eq!(Breakpoint::from_width(800), Breakpoint::Lg);
        assert_eq!(Breakpoint::from_width(1400), Breakpoint::Xxl);
    }

    #[test]
    fn test_breakpoint_comparison() {
        let layout = ResponsiveLayout::new(&LayoutConfig::default());

        assert!(layout.is_at_least(Breakpoint::Xs));
        assert!(!layout.is_at_least(Breakpoint::Xxl));
    }

    #[test]
    fn test_spacing_values() {
        assert_eq!(Spacing::None.as_px(), 0);
        assert_eq!(Spacing::Xs.as_px(), 4);
        assert_eq!(Spacing::Base.as_px(), 16);
        assert_eq!(Spacing::Xxxl.as_px(), 48);
    }

    #[test]
    fn test_flex_config_builder() {
        let config = FlexConfig::row()
            .align(Alignment::Start)
            .justify(Alignment::SpaceBetween)
            .gap(Spacing::Lg)
            .wrap();

        assert_eq!(config.direction, LayoutDirection::Horizontal);
        assert_eq!(config.align, Alignment::Start);
        assert_eq!(config.justify, Alignment::SpaceBetween);
        assert_eq!(config.gap, Spacing::Lg);
        assert!(config.wrap);
    }

    #[test]
    fn test_layout_utils() {
        assert_eq!(LayoutUtils::calculate_grid_columns(800, 200, 16), 3);
        assert_eq!(LayoutUtils::calculate_grid_columns(100, 200, 16), 1);
        assert_eq!(LayoutUtils::clamp(50, 0, 100), 50);
        assert_eq!(LayoutUtils::clamp(150, 0, 100), 100);
    }

    #[test]
    fn test_platform_specifics() {
        assert!(Platform::IOS.is_touch());
        assert!(!Platform::Windows.is_touch());
        assert_eq!(Platform::IOS.min_touch_target(), 44);
        assert_eq!(Platform::Windows.min_touch_target(), 32);
    }

    #[test]
    fn test_class_builder() {
        let config = FlexConfig::column()
            .align(Alignment::Center)
            .justify(Alignment::Center);

        let classes = LayoutClassBuilder::flex(&config);
        assert!(classes.contains("flex-col"));
        assert!(classes.contains("items-center"));
        assert!(classes.contains("justify-center"));

        let grid = LayoutClassBuilder::grid(3, Spacing::Base);
        assert!(grid.contains("grid-cols-3"));
        assert!(grid.contains("gap-4"));
    }
}
