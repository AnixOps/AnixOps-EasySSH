//! EasySSH Standard Edition Main Entry Point
//!
//! Launches the egui-based application for Windows.

use easyssh_egui::app::EasySSHApp;
use eframe::egui;
use tracing::info;
use tracing_subscriber::EnvFilter;

fn main() -> eframe::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()))
        .init();

    info!("Starting EasySSH Standard Edition (egui)");

    // Configure egui options
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_min_inner_size([800.0, 600.0])
            .with_title("EasySSH Standard")
            .with_resizable(true)
            .with_fullscreen(false)
            .with_drag_and_drop(true),
        persist_window: true,
        ..Default::default()
    };

    // Launch the application
    eframe::run_native(
        "EasySSH Standard",
        options,
        Box::new(|cc| {
            // Set up egui style
            setup_egui_style(&cc.egui_ctx);

            // Create app with storage persistence
            Ok(Box::new(EasySSHApp::new(cc)))
        }),
    )
}

/// Configure egui visual style for terminal application
fn setup_egui_style(ctx: &egui::Context) {
    // Set dark theme appropriate for terminal
    let mut style = (*ctx.style()).clone();

    // Adjust spacing for denser terminal layout
    style.spacing.item_spacing = egui::vec2(4.0, 4.0);
    style.spacing.window_margin = egui::Margin::same(8.0);

    // Visuals for dark mode
    style.visuals = egui::Visuals::dark();

    // Custom window styling
    style.visuals.window_fill = egui::Color32::from_rgb(25, 28, 36);
    style.visuals.panel_fill = egui::Color32::from_rgb(30, 33, 42);
    style.visuals.extreme_bg_color = egui::Color32::from_rgb(22, 25, 30);

    // Button styling
    style.visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(40, 44, 52);
    style.visuals.widgets.inactive.fg_stroke.color = egui::Color32::from_rgb(200, 200, 200);
    style.visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(50, 55, 66);
    style.visuals.widgets.active.bg_fill = egui::Color32::from_rgb(60, 66, 80);

    // Selection color
    style.visuals.selection.bg_fill = egui::Color32::from_rgba_unmultiplied(100, 150, 255, 80);

    ctx.set_style(style);

    // Enable font fallback for terminal characters
    let mut fonts = egui::FontDefinitions::default();

    // Use system monospace font for terminal
    fonts
        .families
        .entry(egui::FontFamily::Monospace)
        .or_default()
        .insert(0, "Courier New".to_string()); // Fallback to built-in monospace

    ctx.set_fonts(fonts);
}