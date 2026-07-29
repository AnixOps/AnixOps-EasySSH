use super::tokens::{self, Palette};
use easyssh_core::{DisplayDensity, Theme};
use eframe::egui;

pub fn apply(ctx: &egui::Context, theme: Theme, density: DisplayDensity) {
    let scale = match density {
        DisplayDensity::Compact => 0.9,
        DisplayDensity::Comfortable => 1.0,
        DisplayDensity::Large => 1.15,
    };
    let mut style = (*ctx.style()).clone();
    for (text_style, font) in [
        (
            egui::TextStyle::Body,
            egui::FontId::proportional(16.0 * scale),
        ),
        (
            egui::TextStyle::Button,
            egui::FontId::proportional(16.0 * scale),
        ),
        (
            egui::TextStyle::Heading,
            egui::FontId::proportional(22.0 * scale),
        ),
        (
            egui::TextStyle::Small,
            egui::FontId::proportional(13.0 * scale),
        ),
        (
            egui::TextStyle::Monospace,
            egui::FontId::monospace(15.0 * scale),
        ),
    ] {
        style.text_styles.insert(text_style, font);
    }
    style.spacing.interact_size.y = tokens::CONTROL_HEIGHT * scale;
    style.spacing.button_padding = egui::vec2(8.0 * scale, 4.0 * scale);
    style.animation_time = tokens::MOTION_MS as f32 / 1000.0;
    ctx.set_style(style);

    let dark = match theme {
        Theme::Dark => true,
        Theme::Light => false,
        Theme::System => ctx.system_theme().unwrap_or(egui::Theme::Dark) == egui::Theme::Dark,
    };
    let palette: Palette = if dark { tokens::DARK } else { tokens::LIGHT };
    let mut visuals = if dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };
    visuals.panel_fill = palette.canvas;
    visuals.window_fill = palette.surface;
    visuals.extreme_bg_color = visuals.window_fill;
    visuals.faint_bg_color = palette.surface_muted;
    visuals.override_text_color = Some(palette.text);
    visuals.widgets.noninteractive.fg_stroke.color = palette.muted;
    visuals.widgets.inactive.bg_stroke.color = palette.border;
    visuals.selection.bg_fill = palette.primary.gamma_multiply(0.6);
    visuals.widgets.hovered.bg_stroke.color = palette.primary;
    visuals.widgets.active.bg_stroke.color = palette.primary;
    visuals.hyperlink_color = palette.primary;
    visuals.warn_fg_color = palette.warning;
    visuals.error_fg_color = palette.danger;
    visuals.widgets.active.fg_stroke.color = palette.success;
    visuals.window_rounding = egui::Rounding::same(tokens::CORNER);
    visuals.widgets.inactive.rounding = egui::Rounding::same(4.0);
    visuals.widgets.hovered.rounding = egui::Rounding::same(4.0);
    visuals.widgets.active.rounding = egui::Rounding::same(4.0);
    ctx.set_visuals(visuals);
}
