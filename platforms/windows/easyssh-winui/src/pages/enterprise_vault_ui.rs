#![allow(dead_code)]

//! Enterprise Vault UI Stub
//!
//! Stub implementation for the enterprise password vault UI.

use eframe::egui;

/// Enterprise vault window
pub struct EnterpriseVaultWindow {
    pub open: bool,
}

impl EnterpriseVaultWindow {
    pub fn new(_theme: crate::design::DesignTheme) -> Self {
        Self {
            open: false,
        }
    }

    pub fn open(&mut self) {
        self.open = true;
    }

    pub fn close(&mut self) {
        self.open = false;
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn render(&mut self, ctx: &egui::Context) {
        if !self.open {
            return;
        }

        egui::Window::new("Enterprise Vault")
            .open(&mut self.open)
            .show(ctx, |ui| {
                ui.label("Enterprise Password Vault");
                ui.label("This feature is available in the Pro edition.");
            });
    }
}
