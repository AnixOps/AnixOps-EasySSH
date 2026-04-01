#![allow(dead_code)]

/// Performance Monitoring Panel UI
/// Displays real-time FPS, memory usage, and optimization stats

use egui::*;
use crate::performance::{global_monitor, PerformanceReport, GLOBAL_TRACKER};

pub struct PerformancePanel {
    visible: bool,
    history: Vec<PerformanceSnapshot>,
    max_history: usize,
}

#[derive(Clone, Debug)]
struct PerformanceSnapshot {
    timestamp: std::time::Instant,
    fps: f64,
    frame_time_ms: f64,
    memory_mb: f64,
    cpu_percent: f64,
}

impl PerformancePanel {
    pub fn new() -> Self {
        Self {
            visible: false,
            history: Vec::with_capacity(300), // 5 seconds at 60fps
            max_history: 300,
        }
    }

    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn update(&mut self, fps: f64, frame_time_ms: f64) {
        let memory_bytes = GLOBAL_TRACKER.current_usage();
        let memory_mb = memory_bytes as f64 / (1024.0 * 1024.0);

        let snapshot = PerformanceSnapshot {
            timestamp: std::time::Instant::now(),
            fps,
            frame_time_ms,
            memory_mb,
            cpu_percent: 0.0, // Would need platform-specific APIs
        };

        if self.history.len() >= self.max_history {
            self.history.remove(0);
        }
        self.history.push(snapshot);
    }

    pub fn render(&mut self, ctx: &Context) {
        if !self.visible {
            return;
        }

        let monitor = global_monitor();
        let report = monitor.get_report();

        Window::new("Performance Monitor")
            .resizable(true)
            .default_size([400.0, 300.0])
            .show(ctx, |ui| {
                self.render_content(ui, &report);
            });
    }

    fn render_content(&self, ui: &mut Ui, report: &PerformanceReport) {
        // Header stats
        ui.horizontal(|ui| {
            ui.heading("Performance Metrics");
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui.button("Close").clicked() {
                    // Would need to signal back to parent
                }
            });
        });
        ui.separator();

        // Main stats grid
        Grid::new("perf_stats").num_columns(2).spacing([20.0, 8.0]).show(ui, |ui| {
            ui.label("Current FPS:");
            ui.label(format!("{:.1}", report.current_fps));

            ui.label("Avg FPS:");
            ui.label(format!("{:.1}", report.avg_fps));

            ui.label("Min FPS:");
            ui.label(format!("{:.1}", report.min_fps));

            ui.label("Max Frame Time:");
            ui.label(format!("{:.2} ms", report.max_frame_time_ms));

            ui.label("Memory Usage:");
            ui.label(format!("{:.1} MB", report.memory_usage_mb));

            ui.label("Peak Memory:");
            ui.label(format!("{:.1} MB", report.memory_peak_mb));
        });

        ui.separator();

        // Memory leak detection
        let leak_score = GLOBAL_TRACKER.leak_score();
        ui.horizontal(|ui| {
            ui.label("Memory Leak Score:");
            let color = if leak_score > 100 {
                Color32::RED
            } else if leak_score > 10 {
                Color32::YELLOW
            } else {
                Color32::GREEN
            };
            ui.colored_label(color, format!("{}", leak_score));
        });

        ui.separator();

        // FPS Graph
        if !self.history.is_empty() {
            ui.label("FPS History (5s):");
            let graph_height = 80.0;
            let graph_width = ui.available_width();

            let (response, painter) = ui.allocate_painter(
                vec2(graph_width, graph_height),
                Sense::hover(),
            );

            let rect = response.rect;

            // Background
            painter.rect_filled(rect, 0.0, Color32::from_rgb(30, 30, 30));

            // Grid lines
            for i in 0..=4 {
                let y = rect.min.y + rect.height() * (i as f32 / 4.0);
                painter.line_segment(
                    [pos2(rect.min.x, y), pos2(rect.max.x, y)],
                    Stroke::new(1.0, Color32::from_rgb(50, 50, 50)),
                );
            }

            // FPS line
            if self.history.len() > 1 {
                let max_fps = 60.0;
                let points: Vec<Pos2> = self.history.iter().enumerate().map(|(i, s)| {
                    let x = rect.min.x + (i as f32 / self.max_history as f32) * rect.width();
                    let y = rect.max.y - ((s.fps / max_fps) as f32) * rect.height();
                    pos2(x, y.clamp(rect.min.y, rect.max.y))
                }).collect();

                painter.add(Shape::line(points, Stroke::new(2.0, Color32::GREEN)));
            }

            // Target FPS line
            let target_y = rect.max.y - (60.0 / 60.0) * rect.height();
            painter.line_segment(
                [pos2(rect.min.x, target_y), pos2(rect.max.x, target_y)],
                Stroke::new(1.0, Color32::YELLOW),
            );
        }

        // Operation latencies
        ui.separator();
        ui.label("Operation Latencies:");

        ScrollArea::vertical().max_height(100.0).show(ui, |ui| {
            for (name, stats) in &report.operation_latencies {
                ui.horizontal(|ui| {
                    ui.label(name);
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let color = if stats.avg_ms > 100.0 {
                            Color32::RED
                        } else if stats.avg_ms > 16.0 {
                            Color32::YELLOW
                        } else {
                            Color32::GREEN
                        };
                        ui.colored_label(color, format!("{:.1} ms", stats.avg_ms));
                    });
                });
            }
        });

        ui.separator();

        // Optimization status
        ui.heading("Optimization Status");
        ui.horizontal(|ui| {
            ui.label("Level:");
            ui.label("EXTREME");
        });
        ui.checkbox(&mut true, "Memory Pooling");
        ui.checkbox(&mut true, "Thread Pool");
        ui.checkbox(&mut true, "Virtual Scrolling");
        ui.checkbox(&mut true, "Render Batching");
        ui.checkbox(&mut true, "Connection Pool");
    }
}

/// Quick performance overlay (mini version)
pub fn render_performance_overlay(ctx: &Context, fps: f64, frame_time_ms: f64) {
    let memory_mb = GLOBAL_TRACKER.current_usage() as f64 / (1024.0 * 1024.0);

    // Position in top-right corner
    let screen_rect = ctx.screen_rect();
    let overlay_width = 150.0;
    let overlay_height = 60.0;
    let pos = pos2(screen_rect.max.x - overlay_width - 10.0, screen_rect.min.y + 10.0);

    let overlay_rect = Rect::from_min_size(pos, vec2(overlay_width, overlay_height));

    let painter = ctx.layer_painter(LayerId::background());

    // Background
    painter.rect_filled(
        overlay_rect,
        4.0,
        Color32::from_rgba_premultiplied(0, 0, 0, 180),
    );

    // Text
    let fps_color = if fps < 30.0 {
        Color32::RED
    } else if fps < 55.0 {
        Color32::YELLOW
    } else {
        Color32::GREEN
    };

    painter.text(
        overlay_rect.min + vec2(8.0, 8.0),
        Align2::LEFT_TOP,
        format!("FPS: {:.0}", fps),
        FontId::monospace(14.0),
        fps_color,
    );

    painter.text(
        overlay_rect.min + vec2(8.0, 28.0),
        Align2::LEFT_TOP,
        format!("{:.1} ms", frame_time_ms),
        FontId::monospace(12.0),
        Color32::WHITE,
    );

    painter.text(
        overlay_rect.min + vec2(8.0, 44.0),
        Align2::LEFT_TOP,
        format!("{:.1} MB", memory_mb),
        FontId::monospace(12.0),
        Color32::LIGHT_BLUE,
    );
}
