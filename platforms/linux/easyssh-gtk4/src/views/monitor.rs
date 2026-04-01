use gtk4::prelude::*;
use gtk4::glib;
use libadwaita::prelude::*;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::mpsc as std_mpsc;
use std::time::Instant;

use crate::app::AppViewModel;

/// Maximum number of data points to keep for charts
const MAX_HISTORY_POINTS: usize = 60;
/// Default refresh interval in seconds
const REFRESH_INTERVAL_SECS: u64 = 5;

/// Single data point for metrics history
#[derive(Clone, Debug)]
pub struct MetricPoint {
    pub timestamp: Instant,
    pub cpu: f32,
    pub memory: f32,
    pub disk: f32,
    pub net_in_bytes: u64,
    pub net_out_bytes: u64,
}

impl MetricPoint {
    pub fn new(cpu: f32, memory: f32, disk: f32, net_in: u64, net_out: u64) -> Self {
        Self {
            timestamp: Instant::now(),
            cpu,
            memory,
            disk,
            net_in_bytes: net_in,
            net_out_bytes: net_out,
        }
    }
}

/// Result from a monitor refresh operation
#[derive(Clone, Debug)]
pub struct MonitorRefreshResult {
    pub cpu: f32,
    pub memory: f32,
    pub disk: f32,
    pub uptime: String,
    pub net_in: String,
    pub net_out: String,
    pub load: String,
    pub has_errors: bool,
    pub net_in_bytes: u64,
    pub net_out_bytes: u64,
}

/// Monitor panel showing server metrics with charts
pub struct MonitorPanel {
    widget: gtk4::Box,
    view_model: Arc<Mutex<AppViewModel>>,

    // Header widgets
    title_label: gtk4::Label,
    status_label: gtk4::Label,
    refresh_button: gtk4::Button,
    auto_refresh_switch: adw::SwitchRow,

    // Metric cards
    cpu_card: MetricCard,
    memory_card: MetricCard,
    disk_card: MetricCard,
    load_card: MetricCard,

    // Network display
    net_in_label: gtk4::Label,
    net_out_label: gtk4::Label,
    uptime_label: gtk4::Label,

    // Chart drawing areas
    cpu_chart: gtk4::DrawingArea,
    memory_chart: gtk4::DrawingArea,

    // State
    history: RefCell<VecDeque<MetricPoint>>,
    session_id: RefCell<Option<String>>,
    refreshing: RefCell<bool>,
    auto_refresh: RefCell<bool>,
    last_refresh: RefCell<Option<Instant>>,
    result_rx: RefCell<Option<std_mpsc::Receiver<MonitorRefreshResult>>>,

    // Network previous values for rate calculation
    net_prev: RefCell<Option<(u64, u64, Instant)>>,
}

/// A metric card showing current value and sparkline
struct MetricCard {
    container: adw::ActionRow,
    value_label: gtk4::Label,
    bar: gtk4::LevelBar,
}

impl MetricCard {
    fn new(title: &str, icon_name: &str) -> Self {
        let container = adw::ActionRow::new();
        container.set_title(title);
        container.set_icon_name(icon_name);

        let value_label = gtk4::Label::new(Some("-"));
        value_label.add_css_class("title-3");
        value_label.set_margin_end(12);

        let bar = gtk4::LevelBar::new();
        bar.set_min_value(0.0);
        bar.set_max_value(100.0);
        bar.set_value(0.0);
        bar.set_width_request(100);
        bar.add_css_class("metric-bar");

        // Add suffix widgets
        let suffix_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
        suffix_box.append(&bar);
        suffix_box.append(&value_label);

        container.add_suffix(&suffix_box);

        Self {
            container,
            value_label,
            bar,
        }
    }

    fn update(&self, value: f32, suffix: &str) {
        let value = value.clamp(0.0, 100.0);
        self.bar.set_value(value as f64);
        self.value_label.set_text(&format!("{:.1}{}", value, suffix));

        // Update color based on value
        if value > 80.0 {
            self.bar.add_css_class("high");
            self.bar.remove_css_class("medium");
            self.bar.remove_css_class("low");
        } else if value > 50.0 {
            self.bar.add_css_class("medium");
            self.bar.remove_css_class("high");
            self.bar.remove_css_class("low");
        } else {
            self.bar.add_css_class("low");
            self.bar.remove_css_class("medium");
            self.bar.remove_css_class("high");
        }
    }

    fn set_unavailable(&self) {
        self.bar.set_value(0.0);
        self.value_label.set_text("-");
        self.bar.remove_css_class("high");
        self.bar.remove_css_class("medium");
        self.bar.remove_css_class("low");
    }

    fn widget(&self) -> &adw::ActionRow {
        &self.container
    }
}

impl MonitorPanel {
    pub fn new(view_model: Arc<Mutex<AppViewModel>>) -> Self {
        let box_ = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        box_.set_vexpand(true);

        // Header
        let header_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 16);
        header_box.set_margin_top(16);
        header_box.set_margin_bottom(8);
        header_box.set_margin_start(16);
        header_box.set_margin_end(16);

        let icon = gtk4::Image::from_icon_name("dashboard-symbolic");
        icon.set_pixel_size(32);

        let title_box = gtk4::Box::new(gtk4::Orientation::Vertical, 4);
        let title_label = gtk4::Label::new(Some("Server Monitor"));
        title_label.add_css_class("title-2");
        title_label.set_halign(gtk4::Align::Start);

        let status_label = gtk4::Label::new(Some("Not connected"));
        status_label.add_css_class("dim-label");
        status_label.add_css_class("caption");
        status_label.set_halign(gtk4::Align::Start);

        title_box.append(&title_label);
        title_box.append(&status_label);

        let refresh_button = gtk4::Button::from_icon_name("view-refresh-symbolic");
        refresh_button.set_tooltip_text(Some("Refresh Now"));
        refresh_button.add_css_class("circular");

        header_box.append(&icon);
        header_box.append(&title_box);

        let header_spacer = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        header_spacer.set_hexpand(true);
        header_box.append(&header_spacer);

        header_box.append(&refresh_button);

        // Auto-refresh switch
        let auto_refresh_switch = adw::SwitchRow::new();
        auto_refresh_switch.set_title("Auto Refresh");
        auto_refresh_switch.set_subtitle("Update every 5 seconds");

        let prefs_group = adw::PreferencesGroup::new();
        prefs_group.set_margin_start(16);
        prefs_group.set_margin_end(16);
        prefs_group.set_margin_bottom(8);
        prefs_group.add(&auto_refresh_switch);

        // Metrics grid
        let metrics_grid = adw::PreferencesGroup::new();
        metrics_grid.set_title("Metrics");
        metrics_grid.set_margin_start(16);
        metrics_grid.set_margin_end(16);
        metrics_grid.set_margin_bottom(16);

        let cpu_card = MetricCard::new("CPU Usage", "cpu-symbolic");
        let memory_card = MetricCard::new("Memory", "memory-symbolic");
        let disk_card = MetricCard::new("Disk Usage", "drive-harddisk-symbolic");
        let load_card = MetricCard::new("Load Average", "speedometer-symbolic");

        // Load card doesn't use percentage
        load_card.bar.set_visible(false);

        metrics_grid.add(cpu_card.widget());
        metrics_grid.add(memory_card.widget());
        metrics_grid.add(disk_card.widget());
        metrics_grid.add(load_card.widget());

        // Charts section
        let charts_group = adw::PreferencesGroup::new();
        charts_group.set_title("History");
        charts_group.set_margin_start(16);
        charts_group.set_margin_end(16);
        charts_group.set_margin_bottom(16);

        // CPU chart
        let cpu_chart = gtk4::DrawingArea::new();
        cpu_chart.set_content_height(100);
        cpu_chart.set_hexpand(true);
        cpu_chart.set_tooltip_text(Some("CPU Usage History"));

        // Memory chart
        let memory_chart = gtk4::DrawingArea::new();
        memory_chart.set_content_height(100);
        memory_chart.set_hexpand(true);
        memory_chart.set_margin_top(8);
        memory_chart.set_tooltip_text(Some("Memory Usage History"));

        let charts_box = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        charts_box.append(&cpu_chart);
        charts_box.append(&memory_chart);
        charts_group.add(&charts_box);

        // Network and uptime section
        let net_group = adw::PreferencesGroup::new();
        net_group.set_title("Network & Uptime");
        net_group.set_margin_start(16);
        net_group.set_margin_end(16);
        net_group.set_margin_bottom(16);

        let net_in_row = adw::ActionRow::new();
        net_in_row.set_title("Network In");
        net_in_row.set_icon_name("network-receive-symbolic");
        let net_in_label = gtk4::Label::new(Some("-"));
        net_in_label.add_css_class("title-4");
        net_in_label.set_margin_end(12);
        net_in_row.add_suffix(&net_in_label);

        let net_out_row = adw::ActionRow::new();
        net_out_row.set_title("Network Out");
        net_out_row.set_icon_name("network-transmit-symbolic");
        let net_out_label = gtk4::Label::new(Some("-"));
        net_out_label.add_css_class("title-4");
        net_out_label.set_margin_end(12);
        net_out_row.add_suffix(&net_out_label);

        let uptime_row = adw::ActionRow::new();
        uptime_row.set_title("Uptime");
        uptime_row.set_icon_name("clock-symbolic");
        let uptime_label = gtk4::Label::new(Some("-"));
        uptime_label.add_css_class("title-4");
        uptime_label.set_margin_end(12);
        uptime_row.add_suffix(&uptime_label);

        net_group.add(&net_in_row);
        net_group.add(&net_out_row);
        net_group.add(&uptime_row);

        // Assemble main layout
        let content_box = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        content_box.append(&header_box);
        content_box.append(&prefs_group);
        content_box.append(&metrics_grid);
        content_box.append(&charts_group);
        content_box.append(&net_group);

        // Scroll container
        let scroll = gtk4::ScrolledWindow::new();
        scroll.set_child(Some(&content_box));
        scroll.set_vexpand(true);

        box_.append(&scroll);

        let panel = Self {
            widget: box_,
            view_model,
            title_label,
            status_label,
            refresh_button,
            auto_refresh_switch,
            cpu_card,
            memory_card,
            disk_card,
            load_card,
            net_in_label,
            net_out_label,
            uptime_label,
            cpu_chart,
            memory_chart,
            history: RefCell::new(VecDeque::with_capacity(MAX_HISTORY_POINTS)),
            session_id: RefCell::new(None),
            refreshing: RefCell::new(false),
            auto_refresh: RefCell::new(false),
            last_refresh: RefCell::new(None),
            result_rx: RefCell::new(None),
            net_prev: RefCell::new(None),
        };

        panel.setup_signals();
        panel.setup_charts();
        panel.setup_auto_refresh();

        panel
    }

    fn setup_signals(&self) {
        // Refresh button click
        self.refresh_button.connect_clicked(glib::clone!(@weak self as panel => move |_| {
            panel.refresh();
        }));
    }

    fn setup_charts(&self) {
        // CPU chart draw function
        let history_cpu = self.history.clone();
        self.cpu_chart.set_draw_func(move |area, cr, width, height| {
            draw_metric_chart(area, cr, width, height, &history_cpu.borrow(), |p| p.cpu, 0.0, 100.0, "CPU %");
        });

        // Memory chart draw function
        let history_mem = self.history.clone();
        self.memory_chart.set_draw_func(move |area, cr, width, height| {
            draw_metric_chart(area, cr, width, height, &history_mem.borrow(), |p| p.memory, 0.0, 100.0, "Memory %");
        });
    }

    fn setup_auto_refresh(&self) {
        let panel_weak = self.widget.downgrade();
        let session_id_cell = self.session_id.clone();
        let refreshing_cell = self.refreshing.clone();
        let auto_refresh_cell = self.auto_refresh.clone();
        let result_rx_cell = self.result_rx.clone();
        let last_refresh_cell = self.last_refresh.clone();

        // Auto-refresh toggle
        self.auto_refresh_switch.connect_active_notify(glib::clone!(@weak self as panel => move |switch| {
            *panel.auto_refresh.borrow_mut() = switch.is_active();
        }));

        // Timer for auto-refresh
        glib::timeout_add_local(std::time::Duration::from_secs(1), move || {
            if let Some(_widget) = panel_weak.upgrade() {
                // Check auto-refresh
                if *auto_refresh_cell.borrow() {
                    if let Some(last) = *last_refresh_cell.borrow() {
                        if last.elapsed().as_secs() >= REFRESH_INTERVAL_SECS && !*refreshing_cell.borrow() {
                            // Trigger refresh
                            if session_id_cell.borrow().is_some() {
                                // We'll trigger refresh on next poll
                            }
                        }
                    }
                }

                // Poll for results
                if let Some(rx) = result_rx_cell.borrow_mut().as_mut() {
                    if let Ok(result) = rx.try_recv() {
                        // Would update UI here if we had access to panel
                        *refreshing_cell.borrow_mut() = false;
                        *last_refresh_cell.borrow_mut() = Some(Instant::now());
                    }
                }
            }
            glib::ControlFlow::Continue
        });
    }

    pub fn set_session_id(&self, session_id: Option<String>) {
        self.session_id.replace(session_id);
        self.history.borrow_mut().clear();
        self.net_prev.replace(None);

        if session_id.is_some() {
            self.status_label.set_text("Connected - click refresh to load metrics");
            self.refresh_button.set_sensitive(true);
        } else {
            self.status_label.set_text("Not connected");
            self.refresh_button.set_sensitive(false);
            self.set_unavailable();
        }

        // Queue redraw of charts
        self.cpu_chart.queue_draw();
        self.memory_chart.queue_draw();
    }

    pub fn refresh(&self) {
        if *self.refreshing.borrow() {
            return;
        }

        let session_id = match self.session_id.borrow().as_ref() {
            Some(id) => id.clone(),
            None => return,
        };

        let vm = match self.view_model.lock() {
            Ok(vm) => vm,
            Err(_) => return,
        };

        let prev_net = self.net_prev.borrow().clone();

        *self.refreshing.borrow_mut() = true;
        self.status_label.set_text("Refreshing...");

        // Create channel for result
        let (tx, rx) = std_mpsc::channel();
        self.result_rx.replace(Some(rx));

        // Spawn background thread
        std::thread::spawn(move || {
            let snapshot = collect_monitor_snapshot(&vm, &session_id, prev_net);
            let _ = tx.send(snapshot);
        });
    }

    pub fn poll_result(&self) -> bool {
        let mut updated = false;

        if let Some(rx) = self.result_rx.borrow_mut().as_mut() {
            if let Ok(result) = rx.try_recv() {
                self.apply_result(result);
                *self.refreshing.borrow_mut() = false;
                updated = true;
            }
        }

        // Check if we need auto-refresh
        if *self.auto_refresh.borrow() && !*self.refreshing.borrow() {
            if let Some(last) = *self.last_refresh.borrow() {
                if last.elapsed().as_secs() >= REFRESH_INTERVAL_SECS {
                    self.refresh();
                }
            } else {
                // First refresh
                self.refresh();
            }
        }

        updated
    }

    fn apply_result(&self, result: MonitorRefreshResult) {
        if result.has_errors {
            self.status_label.set_text("Error loading metrics");
            return;
        }

        self.status_label.set_text("Connected - metrics updated");

        // Update metric cards
        self.cpu_card.update(result.cpu, "%");
        self.memory_card.update(result.memory, "%");
        self.disk_card.update(result.disk, "%");

        // Load average
        self.load_card.value_label.set_text(&result.load);

        // Network
        self.net_in_label.set_text(&result.net_in);
        self.net_out_label.set_text(&result.net_out);

        // Uptime
        self.uptime_label.set_text(&result.uptime);

        // Add to history
        let point = MetricPoint::new(
            result.cpu,
            result.memory,
            result.disk,
            result.net_in_bytes,
            result.net_out_bytes,
        );

        let mut history = self.history.borrow_mut();
        if history.len() >= MAX_HISTORY_POINTS {
            history.pop_front();
        }
        history.push_back(point.clone());

        // Update network previous values
        self.net_prev.replace(Some((
            result.net_in_bytes,
            result.net_out_bytes,
            Instant::now(),
        )));

        // Queue chart redraws
        self.cpu_chart.queue_draw();
        self.memory_chart.queue_draw();

        self.last_refresh.replace(Some(Instant::now()));
    }

    fn set_unavailable(&self) {
        self.cpu_card.set_unavailable();
        self.memory_card.set_unavailable();
        self.disk_card.set_unavailable();
        self.load_card.value_label.set_text("-");
        self.net_in_label.set_text("-");
        self.net_out_label.set_text("-");
        self.uptime_label.set_text("-");
    }

    pub fn widget(&self) -> &gtk4::Box {
        &self.widget
    }
}

/// Draw a metric chart using Cairo
fn draw_metric_chart<F>(
    _area: &gtk4::DrawingArea,
    cr: &cairo::Context,
    width: i32,
    height: i32,
    history: &VecDeque<MetricPoint>,
    value_extractor: F,
    min_value: f32,
    max_value: f32,
    label: &str,
) where
    F: Fn(&MetricPoint) -> f32,
{
    let width = width as f64;
    let height = height as f64;

    // Clear background
    cr.set_source_rgb(0.95, 0.95, 0.97);
    cr.paint().ok();

    if history.len() < 2 {
        // Not enough data
        cr.set_source_rgb(0.5, 0.5, 0.5);
        cr.move_to(width / 2.0 - 30.0, height / 2.0);
        cr.show_text("Collecting data...").ok();
        return;
    }

    let padding = 20.0;
    let chart_width = width - padding * 2.0;
    let chart_height = height - padding * 2.0;

    // Draw grid lines
    cr.set_source_rgb(0.85, 0.85, 0.87);
    cr.set_line_width(1.0);

    // Horizontal grid lines (0%, 50%, 100%)
    for i in 0..=4 {
        let y = padding + chart_height * (i as f64 / 4.0);
        cr.move_to(padding, y);
        cr.line_to(width - padding, y);
    }
    cr.stroke().ok();

    // Draw line chart
    let values: Vec<f32> = history.iter().map(&value_extractor).collect();
    let range = max_value - min_value;

    if range > 0.0 {
        // Determine color based on latest value
        let latest = values.last().copied().unwrap_or(0.0);
        let (r, g, b) = if latest > 80.0 {
            (0.9, 0.3, 0.3) // Red for high
        } else if latest > 50.0 {
            (0.9, 0.7, 0.2) // Yellow for medium
        } else {
            (0.3, 0.7, 0.4) // Green for low
        };

        cr.set_source_rgb(r, g, b);
        cr.set_line_width(2.0);

        let step_x = chart_width / (MAX_HISTORY_POINTS - 1) as f64;

        for (i, value) in values.iter().enumerate() {
            let x = padding + i as f64 * step_x;
            let normalized = ((value - min_value) / range).clamp(0.0, 1.0);
            let y = padding + chart_height * (1.0 - normalized as f64);

            if i == 0 {
                cr.move_to(x, y);
            } else {
                cr.line_to(x, y);
            }
        }
        cr.stroke().ok();

        // Fill area under line
        cr.set_source_rgba(r, g, b, 0.2);
        cr.line_to(padding + (values.len() - 1) as f64 * step_x, padding + chart_height);
        cr.line_to(padding, padding + chart_height);
        cr.close_path();
        cr.fill().ok();
    }

    // Draw label
    cr.set_source_rgb(0.4, 0.4, 0.4);
    cr.set_font_size(10.0);
    cr.move_to(padding, padding - 5.0);
    cr.show_text(label).ok();
}

/// Collect monitor snapshot by executing commands via SFTP
fn collect_monitor_snapshot(
    vm: &AppViewModel,
    session_id: &str,
    prev_net: Option<(u64, u64, Instant)>,
) -> MonitorRefreshResult {
    let mut has_errors = false;

    // Helper to execute command with error handling
    fn exec_cmd(vm: &AppViewModel, session_id: &str, cmd: &str) -> anyhow::Result<String> {
        vm.execute_via_sftp(session_id, cmd)
    }

    // CPU - parse /proc/stat
    let cpu = match exec_cmd(vm, session_id, "cat /proc/stat | head -1") {
        Ok(output) => {
            let mut cpu_val = 0.0f32;
            if let Some(line) = output.lines().next() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 5 {
                    if let (Ok(user), Ok(nice), Ok(system), Ok(idle)) = (
                        parts[1].parse::<u64>(),
                        parts[2].parse::<u64>(),
                        parts[3].parse::<u64>(),
                        parts[4].parse::<u64>(),
                    ) {
                        let total = user + nice + system + idle;
                        if total > 0 {
                            cpu_val = ((user + nice + system) as f32 / total as f32) * 100.0;
                        }
                    }
                }
            }
            cpu_val
        }
        Err(_) => {
            has_errors = true;
            0.0f32
        }
    };

    // Memory - parse /proc/meminfo
    let memory = match exec_cmd(vm, session_id, "cat /proc/meminfo | head -3") {
        Ok(output) => {
            let mut mem_total: u64 = 0;
            let mut mem_available: u64 = 0;
            for line in output.lines() {
                if line.starts_with("MemTotal:") {
                    if let Some(val) = line.split_whitespace().nth(1) {
                        mem_total = val.parse::<u64>().unwrap_or(0) * 1024;
                    }
                }
                if line.starts_with("MemAvailable:") {
                    if let Some(val) = line.split_whitespace().nth(1) {
                        mem_available = val.parse::<u64>().unwrap_or(0) * 1024;
                    }
                }
            }
            if mem_total > 0 {
                ((mem_total - mem_available) as f32 / mem_total as f32) * 100.0
            } else {
                0.0f32
            }
        }
        Err(_) => {
            has_errors = true;
            0.0f32
        }
    };

    // Disk - parse df output
    let disk = match exec_cmd(vm, session_id, "df -B1 / | tail -1") {
        Ok(output) => {
            let mut disk_val = 0.0f32;
            if let Some(line) = output.lines().next() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 6 {
                    if let (Ok(total), Ok(used)) = (parts[1].parse::<u64>(), parts[2].parse::<u64>()) {
                        if total > 0 {
                            disk_val = (used as f32 / total as f32) * 100.0;
                        }
                    }
                }
            }
            disk_val
        }
        Err(_) => {
            has_errors = true;
            0.0f32
        }
    };

    // Load average
    let load = match exec_cmd(vm, session_id, "cat /proc/loadavg") {
        Ok(output) => {
            let parts: Vec<&str> = output.split_whitespace().take(3).collect();
            if parts.is_empty() {
                String::from("-")
            } else {
                parts.join(" ")
            }
        }
        Err(_) => {
            has_errors = true;
            String::from("-")
        }
    };

    // Network
    let net_totals = match exec_cmd(vm, session_id, "cat /proc/net/dev") {
        Ok(output) => parse_net_totals(&output),
        Err(_) => {
            has_errors = true;
            None
        }
    };

    let now = Instant::now();
    let (net_in_str, net_out_str, net_in_bytes, net_out_bytes) =
        if let Some((current_in, current_out)) = net_totals {
            let display = if let Some((prev_in, prev_out, prev_t)) = prev_net {
                let dt = now.duration_since(prev_t).as_secs_f64();
                if dt > 0.0 {
                    let in_rate = current_in.saturating_sub(prev_in) as f64 / dt;
                    let out_rate = current_out.saturating_sub(prev_out) as f64 / dt;
                    (
                        fmt_rate(in_rate),
                        fmt_rate(out_rate),
                        current_in,
                        current_out,
                    )
                } else {
                    (
                        format!("{} total", fmt_bytes(current_in)),
                        format!("{} total", fmt_bytes(current_out)),
                        current_in,
                        current_out,
                    )
                }
            } else {
                (
                    format!("{} total", fmt_bytes(current_in)),
                    format!("{} total", fmt_bytes(current_out)),
                    current_in,
                    current_out,
                )
            };
            (display.0, display.1, display.2, display.3)
        } else {
            (String::from("-"), String::from("-"), 0, 0)
        };

    // Uptime
    let uptime = match exec_cmd(vm, session_id, "cat /proc/uptime") {
        Ok(output) => {
            let mut result = String::from("-");
            if let Some(uptime_secs) = output.split_whitespace().next() {
                if let Ok(secs) = uptime_secs.parse::<u64>() {
                    let days = secs / 86400;
                    let hours = (secs % 86400) / 3600;
                    let mins = (secs % 3600) / 60;
                    result = if days > 0 {
                        format!("{}d {}h {}m", days, hours, mins)
                    } else if hours > 0 {
                        format!("{}h {}m", hours, mins)
                    } else {
                        format!("{}m", mins)
                    };
                }
            }
            result
        }
        Err(_) => {
            has_errors = true;
            String::from("-")
        }
    };

    MonitorRefreshResult {
        cpu,
        memory,
        disk,
        uptime,
        net_in: net_in_str,
        net_out: net_out_str,
        load,
        has_errors,
        net_in_bytes,
        net_out_bytes,
    }
}

/// Parse network totals from /proc/net/dev
fn parse_net_totals(output: &str) -> Option<(u64, u64)> {
    let mut total_in: u64 = 0;
    let mut total_out: u64 = 0;
    let mut has_data = false;

    for line in output.lines().skip(2) {
        let parts: Vec<&str> = line.split(':').collect();
        if parts.len() < 2 {
            continue;
        }

        let iface = parts[0].trim();
        if is_ignored_iface(iface) {
            continue;
        }

        let data: Vec<&str> = parts[1].split_whitespace().collect();
        if data.len() >= 10 {
            if let (Ok(rx), Ok(tx)) = (data[0].parse::<u64>(), data[8].parse::<u64>()) {
                total_in = total_in.saturating_add(rx);
                total_out = total_out.saturating_add(tx);
                has_data = true;
            }
        }
    }

    if has_data {
        Some((total_in, total_out))
    } else {
        None
    }
}

/// Check if interface should be ignored
fn is_ignored_iface(iface: &str) -> bool {
    iface == "lo"
        || iface.starts_with("docker")
        || iface.starts_with("br-")
        || iface.starts_with("veth")
        || iface.starts_with("flannel")
        || iface.starts_with("cni")
        || iface.starts_with("tun")
        || iface.starts_with("tap")
        || iface.starts_with("virbr")
}

/// Format bytes to human readable
fn fmt_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// Format rate to human readable
fn fmt_rate(bytes_per_sec: f64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;

    if bytes_per_sec >= GB {
        format!("{:.1} GB/s", bytes_per_sec / GB)
    } else if bytes_per_sec >= MB {
        format!("{:.1} MB/s", bytes_per_sec / MB)
    } else if bytes_per_sec >= KB {
        format!("{:.1} KB/s", bytes_per_sec / KB)
    } else {
        format!("{:.0} B/s", bytes_per_sec)
    }
}
