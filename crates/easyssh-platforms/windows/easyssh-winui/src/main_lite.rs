//! EasySSH Lite - Windows Native UI
//!
//! A lightweight SSH configuration manager with native Windows Terminal integration.
//! Features:
//! - Secure server storage with AES-256-GCM encryption
//! - Native terminal launching (Windows Terminal, PowerShell, CMD)
//! - Group-based server organization
//! - Quick search and filtering
//!
//! This is the Lite edition - for full embedded terminal support, use EasySSH Standard.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod dialogs;
mod detail_panel;
mod sidebar;
mod terminal_launcher;

use app::EasySshApp;
use std::sync::{Arc, Mutex};
use tracing::{error, info};

// View models from existing project
mod viewmodels;
use viewmodels::AppViewModel;

/// Application metadata
const APP_NAME: &str = "EasySSH Lite";
const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() -> eframe::Result {
    // Initialize logging
    tracing_subscriber::fmt::init();

    info!("Starting {} v{}", APP_NAME, APP_VERSION);

    // Initialize application view model
    let view_model = match AppViewModel::new() {
        Ok(vm) => {
            info!("View model initialized successfully");
            Arc::new(Mutex::new(vm))
        }
        Err(e) => {
            error!("Failed to initialize view model: {}", e);
            // Continue with empty view model - app will show error state
            return Err(eframe::Error::AppCreation(format!(
                "Failed to initialize: {}",
                e
            )));
        }
    };

    // Application options
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1024.0, 768.0])
            .with_min_inner_size([800.0, 600.0])
            .with_title(format!("{} v{}", APP_NAME, APP_VERSION)),
        ..Default::default()
    };

    // Run application
    eframe::run_native(
        APP_NAME,
        options,
        Box::new(|_cc| {
            match EasySshApp::new(view_model) {
                Ok(app) => Ok(Box::new(app)),
                Err(e) => {
                    error!("Failed to create app: {}", e);
                    Err(eframe::Error::AppCreation(format!(
                        "Failed to create app: {}",
                        e
                    )))
                }
            }
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_metadata() {
        assert_eq!(APP_NAME, "EasySSH Lite");
        assert!(!APP_VERSION.is_empty());
    }
}
