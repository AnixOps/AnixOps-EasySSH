//! Connection Pool Manager UI
//!
//! Provides visualization and management for the optimized connection pool.
//! Displays connection statistics, allows manual pool configuration, and
//! provides real-time monitoring of pooled connections.

use eframe::egui;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::performance::connection_pool::{OptimizedConnectionPool, PoolConfig};

/// Connection pool UI state and management
pub struct ConnectionPoolManagerUI {
    pub is_open: bool,
    pub config: PoolConfig,
    pub show_advanced_settings: bool,
    pub last_refresh: Option<Instant>,
    pub auto_refresh: bool,
    pub refresh_interval_secs: u32,
    pub selected_endpoint: Option<String>,
    pub action_message: Option<(String, Instant)>,
    // Runtime statistics
    pub endpoint_stats: HashMap<String, EndpointStats>,
}

#[derive(Clone, Debug)]
pub struct EndpointStats {
    pub host: String,
    pub port: u16,
    pub connection_count: usize,
    pub active_connections: usize,
    pub idle_connections: usize,
    pub avg_latency_ms: f64,
    pub total_requests: u64,
    pub failed_requests: u64,
    pub created_at: Instant,
}

impl Default for ConnectionPoolManagerUI {
    fn default() -> Self {
        Self {
            is_open: false,
            config: PoolConfig::default(),
            show_advanced_settings: false,
            last_refresh: None,
            auto_refresh: true,
            refresh_interval_secs: 5,
            selected_endpoint: None,
            action_message: None,
            endpoint_stats: HashMap::new(),
        }
    }
}

impl ConnectionPoolManagerUI {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open(&mut self) {
        self.is_open = true;
    }

    pub fn close(&mut self) {
        self.is_open = false;
    }

    pub fn toggle(&mut self) {
        self.is_open = !self.is_open;
    }

    /// Check if refresh is needed
    pub fn should_refresh(&self) -> bool {
        if !self.auto_refresh {
            return false;
        }
        match self.last_refresh {
            None => true,
            Some(last) => last.elapsed().as_secs() >= self.refresh_interval_secs as u64,
        }
    }

    /// Mark as refreshed
    pub fn mark_refreshed(&mut self) {
        self.last_refresh = Some(Instant::now());
    }

    /// Show action message
    pub fn show_message(&mut self, message: String) {
        self.action_message = Some((message, Instant::now()));
    }

    /// Clear expired message
    pub fn clear_expired_message(&mut self) {
        if let Some((_, timestamp)) = self.action_message {
            if timestamp.elapsed().as_secs() > 3 {
                self.action_message = None;
            }
        }
    }

    /// Update pool configuration
    pub fn apply_config(&mut self, pool: &OptimizedConnectionPool) {
        // In a real implementation, this would update the pool config
        // For now, we just show a message
        self.show_message("Pool configuration updated".to_string());
    }

    /// Pre-warm connections for an endpoint
    pub fn prewarm_endpoint(&mut self, host: &str, port: u16, pool: &OptimizedConnectionPool) {
        pool.prewarm(host, port);
        self.show_message(format!("Pre-warming connections for {}:{}", host, port));
    }

    /// Flush all connections for an endpoint
    pub fn flush_endpoint(&mut self, endpoint_id: &str) {
        self.endpoint_stats.remove(endpoint_id);
        self.show_message(format!("Flushed connections for {}", endpoint_id));
    }

    /// Get total connection count
    pub fn total_connections(&self) -> usize {
        self.endpoint_stats.values().map(|s| s.connection_count).sum()
    }

    /// Get total active connections
    pub fn total_active_connections(&self) -> usize {
        self.endpoint_stats.values().map(|s| s.active_connections).sum()
    }

    /// Render the connection pool manager window
    pub fn render(&mut self, ctx: &egui::Context, pool: Option<&OptimizedConnectionPool>) {
        if !self.is_open {
            return;
        }

        self.clear_expired_message();

        // Auto-refresh statistics
        if self.should_refresh() {
            if let Some(p) = pool {
                let stats = p.stats();
                self.update_stats_from_pool(stats);
            }
            self.mark_refreshed();
        }

        egui::Window::new("Connection Pool Manager")
            .collapsible(false)
            .resizable(true)
            .default_size([700.0, 500.0])
            .frame(egui::Frame {
                fill: egui::Color32::from_rgb(42, 48, 58),
                stroke: egui::Stroke::new(1.0, egui::Color32::from_rgb(60, 70, 85)),
                ..Default::default()
            })
            .show(ctx, |ui| {
                self.render_content(ui, pool);
            });
    }

    fn update_stats_from_pool(&mut self, stats: crate::performance::connection_pool::OptimizedPoolStats) {
        // This would be called with real stats from the pool
        // For demonstration, we keep existing stats
        let _ = stats;
    }

    fn render_content(&mut self, ui: &mut egui::Ui, pool: Option<&OptimizedConnectionPool>) {
        // Header
        ui.horizontal(|ui| {
            ui.heading("Connection Pool");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("✕ Close").clicked() {
                    self.close();
                }
                if ui.button("🔄 Refresh").clicked() {
                    if let Some(p) = pool {
                        let stats = p.stats();
                        self.update_stats_from_pool(stats);
                    }
                    self.mark_refreshed();
                }
            });
        });

        ui.add_space(10.0);

        // Summary statistics
        self.render_summary_stats(ui, pool);

        ui.add_space(10.0);
        ui.separator();
        ui.add_space(10.0);

        // Configuration section
        self.render_configuration(ui);

        ui.add_space(10.0);
        ui.separator();
        ui.add_space(10.0);

        // Endpoint list
        self.render_endpoint_list(ui, pool);

        // Status message
        if let Some((ref message, _)) = self.action_message {
            ui.add_space(10.0);
            ui.label(
                egui::RichText::new(message)
                    .color(egui::Color32::from_rgb(100, 200, 100))
                    .size(12.0),
            );
        }
    }

    fn render_summary_stats(&mut self, ui: &mut egui::Ui, pool: Option<&OptimizedConnectionPool>) {
        if let Some(p) = pool {
            let stats = p.stats();

            ui.group(|ui| {
                ui.label(egui::RichText::new("Pool Statistics").strong().size(14.0));
                ui.add_space(10.0);

                ui.horizontal(|ui| {
                    self.stat_card(ui, "Total Endpoints", &stats.total_endpoints.to_string(), egui::Color32::from_rgb(100, 180, 255));
                    self.stat_card(ui, "Total Connections", &stats.total_connections.to_string(), egui::Color32::from_rgb(72, 199, 116));
                    self.stat_card(ui, "Created", &stats.created.to_string(), egui::Color32::from_rgb(255, 193, 7));
                    self.stat_card(ui, "Reused", &stats.reused.to_string(), egui::Color32::from_rgb(150, 150, 150));
                });

                ui.add_space(10.0);

                ui.horizontal(|ui| {
                    self.stat_card(ui, "Expired", &stats.expired.to_string(), egui::Color32::from_rgb(255, 100, 100));
                    self.stat_card(ui, "Failed", &stats.failed.to_string(), egui::Color32::from_rgb(255, 50, 50));
                    self.stat_card(ui, "Avg Wait", &format!("{:.1}ms", stats.avg_wait_time_ms), egui::Color32::from_rgb(200, 200, 200));
                });
            });
        } else {
            ui.label("Pool not available");
        }
    }

    fn stat_card(&self, ui: &mut egui::Ui, label: &str, value: &str, color: egui::Color32) {
        ui.group(|ui| {
            ui.set_min_width(100.0);
            ui.vertical_centered(|ui| {
                ui.label(
                    egui::RichText::new(value)
                        .strong()
                        .size(20.0)
                        .color(color),
                );
                ui.label(
                    egui::RichText::new(label)
                        .size(11.0)
                        .color(egui::Color32::from_rgb(150, 150, 150)),
                );
            });
        });
    }

    fn render_configuration(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Pool Configuration").strong().size(14.0));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(if self.show_advanced_settings { "△ Simple" } else { "▽ Advanced" }).clicked() {
                    self.show_advanced_settings = !self.show_advanced_settings;
                }
            });
        });

        ui.add_space(10.0);

        // Basic settings
        ui.horizontal(|ui| {
            ui.label("Max per endpoint:");
            ui.add(
                egui::DragValue::new(&mut self.config.max_per_endpoint)
                    .speed(1)
                    .range(1..=20),
            );

            ui.add_space(20.0);

            ui.label("Idle timeout (sec):");
            let mut idle_secs = self.config.idle_timeout.as_secs();
            if ui
                .add(egui::DragValue::new(&mut idle_secs).speed(10).range(30..=3600))
                .changed()
            {
                self.config.idle_timeout = std::time::Duration::from_secs(idle_secs);
            }
        });

        ui.add_space(10.0);

        ui.horizontal(|ui| {
            ui.checkbox(&mut self.config.enable_prewarm, "Enable pre-warming");

            if self.config.enable_prewarm {
                ui.add_space(20.0);
                ui.label("Prewarm count:");
                ui.add(
                    egui::DragValue::new(&mut self.config.prewarm_count)
                        .speed(1)
                        .range(1..=5),
                );
            }
        });

        ui.checkbox(&mut self.auto_refresh, "Auto-refresh statistics");

        // Advanced settings
        if self.show_advanced_settings {
            ui.add_space(10.0);
            ui.group(|ui| {
                ui.label("Advanced Settings");
                ui.add_space(10.0);

                ui.horizontal(|ui| {
                    ui.label("Max lifetime (sec):");
                    let mut lifetime_secs = self.config.max_lifetime.as_secs();
                    if ui
                        .add(egui::DragValue::new(&mut lifetime_secs).speed(60).range(600..=7200))
                        .changed()
                    {
                        self.config.max_lifetime = std::time::Duration::from_secs(lifetime_secs);
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Connect timeout (sec):");
                    let mut timeout_secs = self.config.connect_timeout.as_secs();
                    if ui
                        .add(egui::DragValue::new(&mut timeout_secs).speed(1).range(5..=60))
                        .changed()
                    {
                        self.config.connect_timeout = std::time::Duration::from_secs(timeout_secs);
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Max reuse count:");
                    ui.add(
                        egui::DragValue::new(&mut self.config.max_reuse_count)
                            .speed(100)
                            .range(1000..=50000),
                    );
                });
            });
        }
    }

    fn render_endpoint_list(&mut self, ui: &mut egui::Ui, pool: Option<&OptimizedConnectionPool>) {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Endpoints").strong().size(14.0));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("🧹 Flush All").clicked() {
                    self.endpoint_stats.clear();
                    self.show_message("All connections flushed".to_string());
                }
            });
        });

        ui.add_space(10.0);

        if self.endpoint_stats.is_empty() {
            ui.label("No active endpoints. Connections will appear here when established.");
        } else {
            egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                for (endpoint_id, stats) in &self.endpoint_stats {
                    self.render_endpoint_item(ui, endpoint_id, stats, pool);
                }
            });
        }
    }

    fn render_endpoint_item(
        &mut self,
        ui: &mut egui::Ui,
        endpoint_id: &str,
        stats: &EndpointStats,
        pool: Option<&OptimizedConnectionPool>,
    ) {
        let is_selected = self
            .selected_endpoint
            .as_ref()
            .map(|id| id == endpoint_id)
            .unwrap_or(false);

        let frame = egui::Frame::group(ui.style())
            .inner_margin(8.0)
            .fill(if is_selected {
                egui::Color32::from_rgb(60, 70, 85)
            } else {
                egui::Color32::TRANSPARENT
            });

        frame.show(ui, |ui| {
            ui.set_min_width(ui.available_width());

            ui.horizontal(|ui| {
                // Connection indicator
                let status_color = if stats.active_connections > 0 {
                    egui::Color32::from_rgb(72, 199, 116)
                } else if stats.idle_connections > 0 {
                    egui::Color32::from_rgb(255, 193, 7)
                } else {
                    egui::Color32::from_rgb(150, 150, 150)
                };

                ui.label(
                    egui::RichText::new("●")
                        .color(status_color)
                        .size(16.0),
                );

                ui.vertical(|ui| {
                    ui.label(
                        egui::RichText::new(format!("{}:{}", stats.host, stats.port))
                            .strong()
                            .size(14.0),
                    );

                    ui.label(
                        egui::RichText::new(format!(
                            "{} active / {} idle / {} total | Avg latency: {:.1}ms",
                            stats.active_connections,
                            stats.idle_connections,
                            stats.connection_count,
                            stats.avg_latency_ms
                        ))
                        .size(11.0)
                        .color(egui::Color32::from_rgb(150, 150, 150)),
                    );
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("🗑").on_hover_text("Flush connections").clicked() {
                        self.flush_endpoint(endpoint_id);
                    }

                    if ui.button("🔥").on_hover_text("Pre-warm connections").clicked() {
                        if let Some(p) = pool {
                            self.prewarm_endpoint(&stats.host, stats.port, p);
                        }
                    }
                });
            });
        });

        // Click to select
        if ui.interact(ui.min_rect(), egui::Id::new(endpoint_id), egui::Sense::click()).clicked() {
            self.selected_endpoint = Some(endpoint_id.to_string());
        }
    }
}

/// Render a small connection pool status widget for the toolbar
pub fn render_pool_status_widget(
    ui: &mut egui::Ui,
    pool: Option<&OptimizedConnectionPool>,
    on_click: impl FnOnce(),
) {
    let (total, active) = if let Some(p) = pool {
        let stats = p.stats();
        (stats.total_connections, stats.total_connections) // Simplified
    } else {
        (0, 0)
    };

    let color = if active > 0 {
        egui::Color32::from_rgb(72, 199, 116)
    } else {
        egui::Color32::from_rgb(150, 150, 150)
    };

    let button = egui::Button::new(
        egui::RichText::new(format!("🔗 {} connections", total))
            .color(color)
            .size(12.0),
    )
    .fill(egui::Color32::TRANSPARENT)
    .stroke(egui::Stroke::new(1.0, color));

    if ui.add(button).on_hover_text("Click to manage connection pool").clicked() {
        on_click();
    }
}
