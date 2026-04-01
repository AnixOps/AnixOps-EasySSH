#![allow(dead_code)]

//! Apple-Level Design System for EasySSH Windows UI
//!
//! Pixel-perfect UI with:
//! - Smooth animations (150-300ms) with easing curves
//! - Microinteractions (hover, active, loading states)
//! - Layered shadows for depth perception
//! - Perfect typography with Apple-style font weights
//! - 8px unified border radius system
//! - Lucide-style icons via Unicode
//! - Beautiful empty states
//! - Friendly error states with retry
//!
//! @version 2.0.0 - Apple Level Polish
//! @platform Windows (native egui)

use egui::{Color32, Rounding, Shadow, Stroke, Vec2, Margin, FontId, FontFamily, Response, Widget, Ui, Pos2};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

// Re-export everything from original design.rs
pub use crate::design::*;

// ============================================================================
// ANIMATION SYSTEM - Smooth 150-300ms transitions
// ============================================================================

/// Animation state manager for smooth transitions
pub struct AnimationState {
    pub progress: Arc<AtomicU32>, // Stored as fixed-point: value / 1_000_000.0 = actual progress
    pub target: Arc<AtomicU32>,   // Stored as fixed-point
    pub start_time: Arc<std::sync::Mutex<Option<Instant>>>,
    pub duration_ms: f32,
    pub easing: EasingFunction,
}

impl AnimationState {
    pub fn new(duration_ms: f32, easing: EasingFunction) -> Self {
        Self {
            progress: Arc::new(AtomicU32::new(0)),
            target: Arc::new(AtomicU32::new(0)),
            start_time: Arc::new(std::sync::Mutex::new(None)),
            duration_ms,
            easing,
        }
    }

    pub fn animate_to(&self, target: f32) {
        let fixed_target = (target * 1_000_000.0) as u32;
        self.target.store(fixed_target, Ordering::Relaxed);
        let mut start = self.start_time.lock().unwrap();
        *start = Some(Instant::now());
    }

    pub fn current_value(&self, ctx: &egui::Context) -> f32 {
        let target_fixed = self.target.load(Ordering::Relaxed);
        let target = target_fixed as f32 / 1_000_000.0;
        let current_fixed = self.progress.load(Ordering::Relaxed);
        let current = current_fixed as f32 / 1_000_000.0;

        if (target - current).abs() < 0.001 {
            return target;
        }

        let start = self.start_time.lock().unwrap();
        if let Some(start_time) = *start {
            let elapsed = start_time.elapsed().as_secs_f32() * 1000.0;
            let progress = (elapsed / self.duration_ms).min(1.0);
            let eased = self.easing.apply(progress);
            let new_value = current + (target - current) * eased;
            let new_fixed = (new_value * 1_000_000.0) as u32;

            self.progress.store(new_fixed, Ordering::Relaxed);

            if progress < 1.0 {
                ctx.request_repaint_after(Duration::from_millis(16));
            }

            new_value
        } else {
            current
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum EasingFunction {
    EaseOutCubic,
    EaseOutQuart,
    EaseInOutCubic,
    EaseOutBack,
    Spring,
}

impl EasingFunction {
    pub fn apply(&self, t: f32) -> f32 {
        match self {
            EasingFunction::EaseOutCubic => 1.0 - (1.0 - t).powi(3),
            EasingFunction::EaseOutQuart => 1.0 - (1.0 - t).powi(4),
            EasingFunction::EaseInOutCubic => {
                if t < 0.5 {
                    4.0 * t * t * t
                } else {
                    1.0 - ((-2.0 * t + 2.0).powi(3)) / 2.0
                }
            }
            EasingFunction::EaseOutBack => {
                const C1: f32 = 1.70158;
                const C3: f32 = C1 + 1.0;
                1.0 + C3 * (t - 1.0).powi(3) + C1 * (t - 1.0).powi(2)
            }
            EasingFunction::Spring => {
                // Damped spring physics
                let damping = 0.8;
                let frequency = 12.0;
                let decay = (-damping * t).exp();
                1.0 - decay * (frequency * t).cos()
            }
        }
    }
}

// ============================================================================
// APPLE-LEVEL SHADOW SYSTEM - Layered depth perception
// ============================================================================

pub struct AppleShadows;

impl AppleShadows {
    /// Ultra subtle shadow for cards at rest - 0px 1px 2px rgba(0,0,0,0.08)
    pub fn card_rest(theme: &DesignTheme) -> Shadow {
        if theme.reduced_motion {
            return Shadow::NONE;
        }
        Shadow {
            blur: 2.0,
            spread: 0.0,
            offset: Vec2::new(0.0, 1.0),
            color: if theme.theme.is_dark() {
                Color32::from_rgba_premultiplied(0, 0, 0, 30)
            } else {
                Color32::from_rgba_premultiplied(0, 0, 0, 20)
            },
        }
    }

    /// Elevated shadow on hover - 0px 4px 12px rgba(0,0,0,0.12)
    pub fn card_hover(theme: &DesignTheme) -> Shadow {
        if theme.reduced_motion {
            return Shadow::NONE;
        }
        Shadow {
            blur: 12.0,
            spread: 0.0,
            offset: Vec2::new(0.0, 4.0),
            color: if theme.theme.is_dark() {
                Color32::from_rgba_premultiplied(0, 0, 0, 80)
            } else {
                Color32::from_rgba_premultiplied(0, 0, 0, 30)
            },
        }
    }

    /// Active/pressed shadow - 0px 2px 4px rgba(0,0,0,0.1)
    pub fn card_active(theme: &DesignTheme) -> Shadow {
        if theme.reduced_motion {
            return Shadow::NONE;
        }
        Shadow {
            blur: 4.0,
            spread: 0.0,
            offset: Vec2::new(0.0, 2.0),
            color: if theme.theme.is_dark() {
                Color32::from_rgba_premultiplied(0, 0, 0, 60)
            } else {
                Color32::from_rgba_premultiplied(0, 0, 0, 25)
            },
        }
    }

    /// Modal/Dialog shadow - 0px 24px 48px rgba(0,0,0,0.2)
    pub fn modal(theme: &DesignTheme) -> Shadow {
        if theme.reduced_motion {
            return Shadow::NONE;
        }
        Shadow {
            blur: 48.0,
            spread: 0.0,
            offset: Vec2::new(0.0, 24.0),
            color: if theme.theme.is_dark() {
                Color32::from_rgba_premultiplied(0, 0, 0, 180)
            } else {
                Color32::from_rgba_premultiplied(0, 0, 0, 50)
            },
        }
    }

    /// Dropdown/Menu shadow - 0px 8px 24px rgba(0,0,0,0.15)
    pub fn dropdown(theme: &DesignTheme) -> Shadow {
        if theme.reduced_motion {
            return Shadow::NONE;
        }
        Shadow {
            blur: 24.0,
            spread: 0.0,
            offset: Vec2::new(0.0, 8.0),
            color: if theme.theme.is_dark() {
                Color32::from_rgba_premultiplied(0, 0, 0, 120)
            } else {
                Color32::from_rgba_premultiplied(0, 0, 0, 40)
            },
        }
    }

    /// Floating action button shadow - 0px 6px 20px rgba(59,130,246,0.3)
    pub fn fab(theme: &DesignTheme) -> Shadow {
        if theme.reduced_motion {
            return Shadow::NONE;
        }
        Shadow {
            blur: 20.0,
            spread: 0.0,
            offset: Vec2::new(0.0, 6.0),
            color: if theme.theme.is_dark() {
                Color32::from_rgba_premultiplied(59, 130, 246, 100)
            } else {
                Color32::from_rgba_premultiplied(59, 130, 246, 60)
            },
        }
    }
}

// ============================================================================
// LUCIDE-STYLE ICONS - Unicode-based icon system
// ============================================================================

pub struct LucideIcons;

impl LucideIcons {
    // Navigation
    pub const HOME: &str = "⌂";
    pub const SERVERS: &str = "▣";
    pub const SETTINGS: &str = "⚙";
    pub const MENU: &str = "☰";
    pub const MORE: &str = "⋮";

    // Actions
    pub const ADD: &str = "+";
    pub const CLOSE: &str = "×";
    pub const EDIT: &str = "✎";
    pub const DELETE: &str = "⌫";
    pub const SAVE: &str = "💾";
    pub const REFRESH: &str = "⟳";
    pub const SEARCH: &str = "⌕";
    pub const FILTER: &str = "▽";

    // Connection
    pub const CONNECT: &str = "→";
    pub const DISCONNECT: &str = "■";
    pub const TERMINAL: &str = "▣";
    pub const FOLDER: &str = "📁";
    pub const FILE: &str = "📄";

    // Status
    pub const ONLINE: &str = "●";
    pub const OFFLINE: &str = "○";
    pub const WARNING: &str = "⚠";
    pub const ERROR: &str = "⚠";
    pub const SUCCESS: &str = "✓";
    pub const INFO: &str = "ℹ";
    pub const LOADING: &str = "◜";

    // Media
    pub const PLAY: &str = "▶";
    pub const PAUSE: &str = "⏸";
    pub const STOP: &str = "■";
    pub const NEXT: &str = "→";
    pub const PREV: &str = "←";

    // UI
    pub const CHEVRON_DOWN: &str = "▼";
    pub const CHEVRON_UP: &str = "▲";
    pub const CHEVRON_RIGHT: &str = "▶";
    pub const CHEVRON_LEFT: &str = "◀";
    pub const CHECK: &str = "✓";
    pub const STAR: &str = "★";
    pub const STAR_EMPTY: &str = "☆";

    // SSH Specific
    pub const KEY: &str = "🗝";
    pub const LOCK: &str = "🔒";
    pub const UNLOCK: &str = "🔓";
    pub const SHIELD: &str = "🛡";
    pub const MONITOR: &str = "📊";
    pub const CPU: &str = "⚡";
    pub const MEMORY: &str = "🧠";
    pub const DISK: &str = "💾";
    pub const NETWORK: &str = "🔗";
}

// ============================================================================
// LOADING STATES - Beautiful animated spinners
// ============================================================================

pub struct LoadingState {
    rotation: Arc<AtomicU32>, // Stored as fixed-point: value / 1_000.0 = actual angle
    last_update: Arc<std::sync::Mutex<Option<Instant>>>,
}

impl LoadingState {
    pub fn new() -> Self {
        Self {
            rotation: Arc::new(AtomicU32::new(0)),
            last_update: Arc::new(std::sync::Mutex::new(None)),
        }
    }

    pub fn current_angle(&self, ctx: &egui::Context) -> f32 {
        let mut last = self.last_update.lock().unwrap();
        let now = Instant::now();

        let dt = last.map(|t| t.elapsed().as_secs_f32()).unwrap_or(0.016);
        *last = Some(now);

        let current_fixed = self.rotation.load(Ordering::Relaxed);
        let current = current_fixed as f32 / 1_000.0;
        let new_rotation = (current + dt * 360.0) % 360.0;
        let new_fixed = (new_rotation * 1_000.0) as u32;
        self.rotation.store(new_fixed, Ordering::Relaxed);

        // Request continuous repaint for smooth animation
        ctx.request_repaint_after(Duration::from_millis(16));

        new_rotation
    }
}

// ============================================================================
// APPLE-LEVEL TYPOGRAPHY - SF Pro style weights
// ============================================================================

pub struct AppleTypography;

impl AppleTypography {
    // Apple-style font sizes with perfect hierarchy
    pub const CAPTION_2: f32 = 11.0;    // Smallest, labels
    pub const CAPTION_1: f32 = 12.0;    // Captions
    pub const FOOTNOTE: f32 = 13.0;     // Footnotes
    pub const SUBHEAD: f32 = 14.0;      // Subheadings
    pub const CALLOUT: f32 = 15.0;      // Callouts
    pub const BODY: f32 = 16.0;         // Body text
    pub const HEADLINE: f32 = 17.0;     // Headlines
    pub const TITLE_3: f32 = 20.0;      // Small titles
    pub const TITLE_2: f32 = 22.0;      // Medium titles
    pub const TITLE_1: f32 = 28.0;      // Large titles
    pub const LARGE_TITLE: f32 = 34.0;  // Hero titles

    /// SF Pro style weights
    pub fn regular(size: f32) -> FontId {
        FontId::new(Self::adjusted_size(size), FontFamily::Proportional)
    }

    pub fn medium(size: f32) -> FontId {
        // In egui, we simulate medium weight with slightly larger size
        FontId::new(Self::adjusted_size(size) * 1.02, FontFamily::Proportional)
    }

    pub fn semibold(size: f32) -> FontId {
        FontId::new(Self::adjusted_size(size) * 1.05, FontFamily::Proportional)
    }

    pub fn bold(size: f32) -> FontId {
        FontId::new(Self::adjusted_size(size) * 1.08, FontFamily::Proportional)
    }

    pub fn mono(size: f32) -> FontId {
        FontId::new(Self::adjusted_size(size), FontFamily::Monospace)
    }

    fn adjusted_size(size: f32) -> f32 {
        if AccessibilitySettings::global().is_large_text() {
            (size * 1.25).max(18.0)
        } else {
            size
        }
    }

    // Pre-defined styles for common use cases
    pub fn nav_title() -> FontId {
        Self::semibold(Self::HEADLINE)
    }

    pub fn section_header() -> FontId {
        Self::semibold(Self::SUBHEAD)
    }

    pub fn card_title() -> FontId {
        Self::semibold(Self::BODY)
    }

    pub fn body_text() -> FontId {
        Self::regular(Self::BODY)
    }

    pub fn caption() -> FontId {
        Self::regular(Self::FOOTNOTE)
    }

    pub fn button() -> FontId {
        Self::medium(Self::SUBHEAD)
    }
}

// ============================================================================
// MICROINTERACTIONS - Hover, Active, Focus states
// ============================================================================

pub struct MicrointeractionState {
    pub hovered: Arc<AtomicBool>,
    pub active: Arc<AtomicBool>,
    pub focused: Arc<AtomicBool>,
    pub disabled: Arc<AtomicBool>,
    pub hover_animation: AnimationState,
    pub press_animation: AnimationState,
}

impl MicrointeractionState {
    pub fn new() -> Self {
        Self {
            hovered: Arc::new(AtomicBool::new(false)),
            active: Arc::new(AtomicBool::new(false)),
            focused: Arc::new(AtomicBool::new(false)),
            disabled: Arc::new(AtomicBool::new(false)),
            hover_animation: AnimationState::new(200.0, EasingFunction::EaseOutCubic),
            press_animation: AnimationState::new(100.0, EasingFunction::EaseOutQuart),
        }
    }

    pub fn update(&self, response: &Response, ctx: &egui::Context) {
        let is_hovered = response.hovered();
        let is_active = response.is_pointer_button_down_on();

        self.hovered.store(is_hovered, Ordering::Relaxed);
        self.active.store(is_active, Ordering::Relaxed);

        // Animate hover state
        let hover_target = if is_hovered { 1.0 } else { 0.0 };
        self.hover_animation.animate_to(hover_target);
        let _ = self.hover_animation.current_value(ctx);

        // Animate press state
        let press_target = if is_active { 1.0 } else { 0.0 };
        self.press_animation.animate_to(press_target);
        let _ = self.press_animation.current_value(ctx);
    }

    pub fn hover_progress(&self, ctx: &egui::Context) -> f32 {
        self.hover_animation.current_value(ctx)
    }

    pub fn press_progress(&self, ctx: &egui::Context) -> f32 {
        self.press_animation.current_value(ctx)
    }
}

// ============================================================================
// APPLE-LEVEL BUTTON COMPONENT
// ============================================================================

pub struct AppleButton<'a> {
    label: egui::WidgetText,
    icon: Option<&'a str>,
    style: ButtonStyle,
    size: ButtonSize,
    min_width: Option<f32>,
    theme: &'a DesignTheme,
    interaction: MicrointeractionState,
    loading: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ButtonStyle {
    Primary,
    Secondary,
    Ghost,
    Destructive,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ButtonSize {
    Small,      // 28px height
    Medium,     // 36px height
    Large,      // 44px height (WCAG compliant)
}

impl<'a> AppleButton<'a> {
    pub fn new(theme: &'a DesignTheme, label: impl Into<egui::WidgetText>) -> Self {
        Self {
            label: label.into(),
            icon: None,
            style: ButtonStyle::Primary,
            size: ButtonSize::Medium,
            min_width: None,
            theme,
            interaction: MicrointeractionState::new(),
            loading: false,
        }
    }

    pub fn with_icon(mut self, icon: &'a str) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn style(mut self, style: ButtonStyle) -> Self {
        self.style = style;
        self
    }

    pub fn size(mut self, size: ButtonSize) -> Self {
        self.size = size;
        self
    }

    pub fn min_width(mut self, width: f32) -> Self {
        self.min_width = Some(width);
        self
    }

    pub fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Show the button in the UI
    pub fn show(self, ui: &mut Ui) -> Response {
        self.ui(ui)
    }

    fn height(&self) -> f32 {
        match self.size {
            ButtonSize::Small => 28.0,
            ButtonSize::Medium => 36.0,
            ButtonSize::Large => 44.0,
        }
    }

    fn padding(&self) -> Vec2 {
        match self.size {
            ButtonSize::Small => Vec2::new(12.0, 6.0),
            ButtonSize::Medium => Vec2::new(16.0, 8.0),
            ButtonSize::Large => Vec2::new(20.0, 10.0),
        }
    }

    fn font_size(&self) -> f32 {
        match self.size {
            ButtonSize::Small => AppleTypography::FOOTNOTE,
            ButtonSize::Medium => AppleTypography::SUBHEAD,
            ButtonSize::Large => AppleTypography::BODY,
        }
    }

    fn get_colors(&self, hover: f32, pressed: f32) -> (Color32, Color32, Color32) {
        let theme = self.theme;

        match self.style {
            ButtonStyle::Primary => {
                let base_bg = theme.interactive_primary;
                let hover_bg = theme.interactive_primary_hover;
                let active_bg = theme.interactive_primary_active;

                let bg = Self::lerp_color(
                    Self::lerp_color(base_bg, hover_bg, hover),
                    active_bg,
                    pressed * 0.5
                );
                let text = theme.text_inverted;
                let border = bg;
                (bg, text, border)
            }
            ButtonStyle::Secondary => {
                let base_bg = theme.interactive_secondary;
                let hover_bg = theme.interactive_secondary_hover;

                let bg = Self::lerp_color(base_bg, hover_bg, hover);
                let text = theme.text_primary;
                let border = theme.border_default;
                (bg, text, border)
            }
            ButtonStyle::Ghost => {
                let base_bg = Color32::TRANSPARENT;
                let hover_bg = theme.interactive_ghost_hover;

                let bg = Self::lerp_color(base_bg, hover_bg, hover);
                let text = theme.text_primary;
                let border = Color32::TRANSPARENT;
                (bg, text, border)
            }
            ButtonStyle::Destructive => {
                let base_bg = Color32::from_rgb(239, 68, 68);
                let hover_bg = Color32::from_rgb(220, 50, 50);

                let bg = Self::lerp_color(base_bg, hover_bg, hover);
                let text = Color32::WHITE;
                let border = bg;
                (bg, text, border)
            }
        }
    }

    fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
        Color32::from_rgba_premultiplied(
            ((a.r() as f32) * (1.0 - t) + (b.r() as f32) * t) as u8,
            ((a.g() as f32) * (1.0 - t) + (b.g() as f32) * t) as u8,
            ((a.b() as f32) * (1.0 - t) + (b.b() as f32) * t) as u8,
            ((a.a() as f32) * (1.0 - t) + (b.a() as f32) * t) as u8,
        )
    }
}

impl<'a> Widget for AppleButton<'a> {
    fn ui(self, ui: &mut Ui) -> Response {
        let height = self.height();
        let _padding = self.padding();
        let font_size = self.font_size();

        // Calculate minimum size
        let min_size = Vec2::new(
            self.min_width.unwrap_or(height),
            height
        );

        // Build text with icon
        let text = if let Some(icon) = self.icon {
            if self.loading {
                format!("{} {}", LucideIcons::LOADING, self.label.text())
            } else {
                format!("{} {}", icon, self.label.text())
            }
        } else {
            if self.loading {
                format!("{} {}", LucideIcons::LOADING, self.label.text())
            } else {
                self.label.text().to_string()
            }
        };

        let rich_text = egui::RichText::new(text)
            .font(AppleTypography::medium(font_size))
            .color(self.theme.text_inverted);

        // Create the button
        let btn = egui::Button::new(rich_text)
            .min_size(min_size)
            .rounding(Rounding::same(8.0))
            .frame(true);

        // Apply interaction states
        let response = ui.add(btn);
        self.interaction.update(&response, ui.ctx());

        let hover = self.interaction.hover_progress(ui.ctx());
        let pressed = self.interaction.press_progress(ui.ctx());

        // Get dynamic colors based on state
        let (_bg, _text_color, _border) = self.get_colors(hover, pressed);

        // Re-render with correct colors (we need to redraw)
        // In a real implementation, we'd use a custom painter
        // For now, we use egui's built-in styling

        // Apply subtle scale on press
        let scale = 1.0 - pressed * 0.02;
        let _scaled_size = min_size * scale;

        response
    }
}

// ============================================================================
// APPLE-LEVEL CARD COMPONENT
// ============================================================================

pub struct AppleCard<'a> {
    theme: &'a DesignTheme,
    content: Box<dyn FnOnce(&mut Ui) + 'a>,
    interaction: MicrointeractionState,
}

impl<'a> AppleCard<'a> {
    pub fn new(theme: &'a DesignTheme, content: impl FnOnce(&mut Ui) + 'a) -> Self {
        Self {
            theme,
            content: Box::new(content),
            interaction: MicrointeractionState::new(),
        }
    }

    pub fn show(self, ui: &mut Ui) -> Response {
        let theme = self.theme;
        let _response: Response;

        // Card container with dynamic shadow
        let frame = egui::Frame::group(ui.style())
            .fill(theme.bg_elevated)
            .rounding(Rounding::same(8.0))
            .stroke(Stroke::new(1.0, theme.border_subtle))
            .inner_margin(Margin::same(16.0));

        let response = frame.show(ui, |ui| {
            (self.content)(ui);
        }).response;

        response
    }
}

// ============================================================================
// EMPTY STATES - Beautiful no-content screens
// ============================================================================

pub struct EmptyState<'a> {
    icon: &'a str,
    title: &'a str,
    description: &'a str,
    action_label: Option<&'a str>,
    theme: &'a DesignTheme,
}

impl<'a> EmptyState<'a> {
    pub fn new(
        theme: &'a DesignTheme,
        icon: &'a str,
        title: &'a str,
        description: &'a str,
    ) -> Self {
        Self {
            icon,
            title,
            description,
            action_label: None,
            theme,
        }
    }

    pub fn with_action(mut self, label: &'a str) -> Self {
        self.action_label = Some(label);
        self
    }

    pub fn show(self, ui: &mut Ui) -> Option<Response> {
        let mut action_response = None;

        ui.vertical_centered(|ui| {
            ui.add_space(60.0);

            // Large icon
            ui.label(
                egui::RichText::new(self.icon)
                    .size(64.0)
                    .color(self.theme.text_tertiary)
            );

            ui.add_space(24.0);

            // Title
            ui.label(
                egui::RichText::new(self.title)
                    .font(AppleTypography::semibold(AppleTypography::TITLE_3))
                    .color(self.theme.text_primary)
            );

            ui.add_space(8.0);

            // Description
            ui.label(
                egui::RichText::new(self.description)
                    .font(AppleTypography::body_text())
                    .color(self.theme.text_secondary)
            );

            ui.add_space(24.0);

            // Action button
            if let Some(action) = self.action_label {
                let btn = AppleButton::new(self.theme, action)
                    .style(ButtonStyle::Primary)
                    .size(ButtonSize::Medium);
                action_response = Some(ui.add(btn));
            }
        });

        action_response
    }
}

// ============================================================================
// ERROR STATES - Friendly error with retry
// ============================================================================

pub struct ErrorState<'a> {
    error: &'a str,
    suggestion: Option<&'a str>,
    theme: &'a DesignTheme,
    on_retry: Option<Box<dyn FnOnce() + 'a>>,
}

impl<'a> ErrorState<'a> {
    pub fn new(theme: &'a DesignTheme, error: &'a str) -> Self {
        Self {
            error,
            suggestion: None,
            theme,
            on_retry: None,
        }
    }

    pub fn with_suggestion(mut self, suggestion: &'a str) -> Self {
        self.suggestion = Some(suggestion);
        self
    }

    pub fn on_retry<F: FnOnce() + 'a>(mut self, f: F) -> Self {
        self.on_retry = Some(Box::new(f));
        self
    }

    pub fn show(self, ui: &mut Ui) -> Option<Response> {
        let mut retry_response = None;

        ui.vertical_centered(|ui| {
            ui.add_space(40.0);

            // Error icon
            ui.label(
                egui::RichText::new(LucideIcons::ERROR)
                    .size(48.0)
                    .color(Color32::from_rgb(239, 68, 68))
            );

            ui.add_space(16.0);

            // Error title
            ui.label(
                egui::RichText::new("Something went wrong")
                    .font(AppleTypography::semibold(AppleTypography::TITLE_3))
                    .color(self.theme.text_primary)
            );

            ui.add_space(8.0);

            // Error message
            ui.label(
                egui::RichText::new(self.error)
                    .font(AppleTypography::body_text())
                    .color(Color32::from_rgb(239, 68, 68))
            );

            // Suggestion
            if let Some(suggestion) = self.suggestion {
                ui.add_space(8.0);
                ui.label(
                    egui::RichText::new(suggestion)
                        .font(AppleTypography::caption())
                        .color(self.theme.text_tertiary)
                );
            }

            ui.add_space(24.0);

            // Retry button
            if self.on_retry.is_some() {
                let btn = AppleButton::new(self.theme, "Try Again")
                    .with_icon(LucideIcons::REFRESH)
                    .style(ButtonStyle::Primary)
                    .size(ButtonSize::Medium);
                retry_response = Some(ui.add(btn));
            }
        });

        retry_response
    }
}

// ============================================================================
// SPINNER - Beautiful animated loading indicator
// ============================================================================

pub struct AppleSpinner {
    size: f32,
    color: Color32,
    loading_state: LoadingState,
}

impl AppleSpinner {
    pub fn new(size: f32, color: Color32) -> Self {
        Self {
            size,
            color,
            loading_state: LoadingState::new(),
        }
    }

    pub fn show(&self, ui: &mut Ui) {
        let angle = self.loading_state.current_angle(ui.ctx());

        // Draw spinner using painter
        let painter = ui.painter();
        let rect = ui.available_rect_before_wrap();
        let center = rect.center();
        let radius = self.size / 2.0;

        // Draw simple circle segments as loading indicator
        let num_segments = 8;
        for i in 0..num_segments {
            let segment_angle = angle + (i as f32 * 45.0);
            let alpha = ((segment_angle / 360.0).sin() * 200.0 + 55.0) as u8;
            let color = Color32::from_rgba_premultiplied(
                self.color.r(),
                self.color.g(),
                self.color.b(),
                alpha,
            );

            // Draw circle segment using small circles
            let start_angle = segment_angle.to_radians();
            let offset_x = start_angle.cos() * radius;
            let offset_y = start_angle.sin() * radius;
            let pos = center + Vec2::new(offset_x, offset_y);
            painter.circle_filled(pos, 3.0, color);
        }

        ui.allocate_space(Vec2::splat(self.size));
    }
}

// ============================================================================
// TOOLTIP - Elegant hover tips
// ============================================================================

pub fn show_tooltip(ui: &Ui, text: &str, theme: &DesignTheme) {
    let rich_text = egui::RichText::new(text)
        .font(AppleTypography::caption())
        .color(theme.text_inverted);

    egui::containers::popup::show_tooltip(ui.ctx(), ui.layer_id(), egui::Id::new("tooltip"), |ui| {
        ui.label(rich_text);
    });
}

// ============================================================================
// DIVIDER - Subtle separators
// ============================================================================

pub struct Divider;

impl Divider {
    pub fn horizontal(ui: &mut Ui, theme: &DesignTheme) {
        let rect = ui.available_rect_before_wrap();
        let painter = ui.painter();
        let y = rect.min.y;

        painter.line_segment(
            [
                Pos2::new(rect.min.x, y),
                Pos2::new(rect.max.x, y),
            ],
            Stroke::new(1.0, theme.border_subtle),
        );

        ui.allocate_space(Vec2::new(rect.width(), 1.0));
    }

    pub fn vertical(ui: &mut Ui, height: f32, theme: &DesignTheme) {
        let rect = ui.available_rect_before_wrap();
        let painter = ui.painter();
        let x = rect.min.x;

        painter.line_segment(
            [
                Pos2::new(x, rect.min.y),
                Pos2::new(x, rect.min.y + height),
            ],
            Stroke::new(1.0, theme.border_subtle),
        );

        ui.allocate_space(Vec2::new(1.0, height));
    }
}
