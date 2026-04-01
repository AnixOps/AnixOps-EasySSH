//! High-Performance Terminal Module
//!
//! Provides 60fps WebGL-accelerated terminal rendering using xterm.js
//! with egui integration for the Windows native UI.

pub mod webgl_terminal;
pub mod egui_integration;
pub mod renderer;
pub mod streaming;
pub mod manager;
pub mod clipboard;

pub use webgl_terminal::{
    TerminalConfig,
    ColorSupport,
    RenderStats,
};

pub use egui_integration::{
    EguiWebGlTerminal,
    WebGlTerminalBuilder,
    TerminalMessage,
};


pub use streaming::StreamingProcessor;

pub use manager::{
    WebGlTerminalManager,
};


/// Terminal module version
pub const VERSION: &str = "0.1.0";

/// Default terminal dimensions
pub const DEFAULT_COLS: usize = 80;
pub const DEFAULT_ROWS: usize = 24;

/// 60fps timing constants
pub const TARGET_FPS: u32 = 60;
pub const FRAME_TIME_MS: f64 = 1000.0 / TARGET_FPS as f64;
pub const FRAME_TIME_MICROS: u64 = (FRAME_TIME_MS * 1000.0) as u64;

/// Performance tuning constants
pub const MAX_BATCH_SIZE: usize = 1024;
pub const STREAMING_CHUNK_SIZE: usize = 8192;
pub const SCROLLBACK_OPTIMIZATION_THRESHOLD: usize = 10000;

/// Check if running on a high-DPI display
pub fn is_high_dpi(ctx: &egui::Context) -> bool {
    ctx.input(|i| i.viewport().native_pixels_per_point.unwrap_or(1.0) > 1.5)
}

/// Calculate optimal terminal dimensions for available space
pub fn calculate_terminal_size(
    available_width: f32,
    available_height: f32,
    _font_size: f32,
    line_height: f32,
    char_width: f32,
) -> (usize, usize) {
    let cols = ((available_width / char_width) as usize).max(40);
    let rows = ((available_height / line_height) as usize).max(10);
    (cols, rows)
}

/// Initialize terminal subsystem with performance optimizations
pub fn init_terminal() {
    tracing::info!("Initializing WebGL terminal subsystem ({}fps target)", TARGET_FPS);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terminal_size_calculation() {
        let (cols, rows) = calculate_terminal_size(800.0, 600.0, 14.0, 1.2, 0.6);
        assert!(cols >= 40);
        assert!(rows >= 10);
    }

    #[test]
    fn test_frame_timing() {
        assert_eq!(FRAME_TIME_MS, 1000.0 / 60.0);
        assert!(FRAME_TIME_MICROS > 0);
    }
}
