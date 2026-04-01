#![allow(dead_code)]

use eframe::egui;
use egui::{
    Color32, DragValue, Frame, Margin, Pos2, Rect, Response, Rounding, Sense, Stroke, Ui, Vec2,
};
use uuid::Uuid;

use easyssh_core::script_library::ScriptLibrary;
use easyssh_core::workflow_engine::*;

/// Visual workflow editor component
pub struct WorkflowEditor {
    /// Currently edited workflow
    workflow: Workflow,
    /// Selected step ID
    selected_step: Option<String>,
    /// View offset for panning
    view_offset: Vec2,
    /// Zoom level
    zoom: f32,
    /// Grid size for snapping
    grid_size: f32,
    /// Show grid
    show_grid: bool,
    /// Drag state
    dragging_step: Option<(String, Pos2)>,
    /// Connecting state (from_step_id, is_true_branch)
    connecting_from: Option<(String, bool)>,
    /// Step being hovered for connection
    hover_target: Option<String>,
    /// Show properties panel
    show_properties: bool,
    /// Show variable panel
    show_variables: bool,
    /// Step type selector expanded
    step_type_selector_open: bool,
    /// New step being created at position
    new_step_position: Option<Pos2>,
    /// Validation errors
    validation_errors: Vec<String>,
    /// Execution preview mode
    preview_mode: bool,
    /// Current execution step (for preview)
    preview_current_step: Option<String>,
}

impl WorkflowEditor {
    pub fn new(workflow: Workflow) -> Self {
        Self {
            workflow,
            selected_step: None,
            view_offset: Vec2::ZERO,
            zoom: 1.0,
            grid_size: 20.0,
            show_grid: true,
            dragging_step: None,
            connecting_from: None,
            hover_target: None,
            show_properties: true,
            show_variables: false,
            step_type_selector_open: false,
            new_step_position: None,
            validation_errors: Vec::new(),
            preview_mode: false,
            preview_current_step: None,
        }
    }

    pub fn with_new(name: &str) -> Self {
        Self::new(Workflow::new(name))
    }

    pub fn ui(&mut self, ui: &mut Ui) -> WorkflowEditorResponse {
        let mut response = WorkflowEditorResponse::default();

        // Toolbar
        self.render_toolbar(ui, &mut response);

        ui.separator();

        // Main editor area
        let available_rect = ui.available_rect_before_wrap();
        let editor_rect = available_rect;

        // Split view: editor + sidebar
        let sidebar_width = if self.show_properties { 300.0 } else { 0.0 };
        let editor_width = editor_rect.width() - sidebar_width - 10.0;

        let editor_area = Rect::from_min_size(
            editor_rect.min,
            Vec2::new(editor_width, editor_rect.height()),
        );

        let sidebar_area = Rect::from_min_size(
            Pos2::new(editor_rect.min.x + editor_width + 10.0, editor_rect.min.y),
            Vec2::new(sidebar_width, editor_rect.height()),
        );

        // Render canvas
        let _canvas_response = self.render_canvas(ui, editor_area, &mut response);

        // Render sidebar
        if self.show_properties {
            self.render_sidebar(ui, sidebar_area, &mut response);
        }

        // Render floating panels
        if self.step_type_selector_open {
            if let Some(pos) = self.new_step_position {
                self.render_step_type_selector(ui, pos, &mut response);
            }
        }

        // Update workflow in response
        response.workflow = Some(self.workflow.clone());
        response.selected_step = self.selected_step.clone();

        response
    }

    fn render_toolbar(&mut self, ui: &mut Ui, response: &mut WorkflowEditorResponse) {
        ui.horizontal(|ui| {
            // Workflow name
            ui.heading(&self.workflow.name);

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // View controls
                ui.checkbox(&mut self.show_grid, "Grid");
                ui.checkbox(&mut self.show_properties, "Properties");
                ui.checkbox(&mut self.show_variables, "Variables");

                ui.separator();

                // Zoom controls
                if ui.button("Reset View").clicked() {
                    self.view_offset = Vec2::ZERO;
                    self.zoom = 1.0;
                }

                ui.add(DragValue::new(&mut self.zoom).speed(0.1).range(0.5..=2.0));
                ui.label("Zoom:");

                ui.separator();

                // Validation
                if ui.button("Validate").clicked() {
                    self.validation_errors = match self.workflow.validate() {
                        Ok(_) => vec!["Workflow is valid".to_string()],
                        Err(errors) => errors,
                    };
                }

                // Run
                if ui.button("Run").clicked() {
                    response.run_requested = true;
                }

                // Save
                if ui.button("Save").clicked() {
                    response.save_requested = true;
                }
            });
        });
    }

    fn render_canvas(
        &mut self,
        ui: &mut Ui,
        rect: Rect,
        _response: &mut WorkflowEditorResponse,
    ) -> Response {
        let canvas = ui.allocate_rect(rect, Sense::click_and_drag());

        // Background
        let painter = ui.painter_at(rect);

        // Draw grid
        if self.show_grid {
            self.draw_grid(&painter, rect);
        }

        // Draw connections first (behind nodes)
        self.draw_connections(&painter, rect);

        // Draw steps
        let mut clicked_step: Option<String> = None;
        let mut step_responses: Vec<(String, Response)> = Vec::new();

        for step in &self.workflow.steps.clone() {
            if let Some((step_response, step_rect)) = self.draw_step(ui, rect, step, &painter) {
                step_responses.push((step.id.clone(), step_response.clone()));

                if step_response.clicked() {
                    clicked_step = Some(step.id.clone());
                }

                if step_response.drag_started() {
                    self.dragging_step = Some((step.id.clone(), step_rect.center()));
                }

                if step_response.hovered() {
                    self.hover_target = Some(step.id.clone());
                }
            }
        }

        // Handle dragging
        if let Some((step_id, _)) = &self.dragging_step {
            let drag_delta = canvas.drag_delta();
            if drag_delta != Vec2::ZERO {
                if let Some(step) = self.workflow.get_step_mut(step_id) {
                    if let Some((x, y)) = step.position {
                        let new_x = x + drag_delta.x / self.zoom;
                        let new_y = y + drag_delta.y / self.zoom;
                        step.position = Some((new_x, new_y));
                    }
                }
            }
        }

        if canvas.drag_stopped() {
            self.dragging_step = None;
        }

        // Handle step selection
        if let Some(ref step_id) = clicked_step {
            self.selected_step = Some(step_id.clone());
        }

        // Handle canvas click (deselect)
        if canvas.clicked() && clicked_step.is_none() {
            self.selected_step = None;
        }

        // Handle double-click to create step
        if canvas.double_clicked() {
            if let Some(pos) = canvas.interact_pointer_pos() {
                let canvas_pos = self.screen_to_canvas(pos, rect);
                self.new_step_position = Some(canvas_pos);
                self.step_type_selector_open = true;
            }
        }

        // Handle connecting
        if let Some((from_id, _is_true_branch)) = &self.connecting_from {
            if let Some(hover_id) = &self.hover_target {
                if hover_id != from_id {
                    // Draw temporary connection line
                    if let (Some(from_step), Some(to_step)) = (
                        self.workflow.get_step(from_id),
                        self.workflow.get_step(hover_id),
                    ) {
                        if let (Some((x1, y1)), Some((_x2, _y2))) =
                            (from_step.position, to_step.position)
                        {
                            let p1 = self.canvas_to_screen(Pos2::new(x1, y1), rect);
                            let p2 = ui.ctx().input(|i| i.pointer.hover_pos().unwrap_or(p1));
                            painter.line_segment([p1, p2], Stroke::new(2.0, Color32::YELLOW));
                        }
                    }
                }
            }
        }

        // Canvas pan
        if canvas.dragged_by(egui::PointerButton::Middle)
            || (canvas.dragged() && self.selected_step.is_none() && self.dragging_step.is_none())
        {
            self.view_offset += canvas.drag_delta();
        }

        canvas
    }

    fn draw_grid(&self, painter: &egui::Painter, rect: Rect) {
        let grid_color = Color32::from_gray(40);
        let grid_spacing = self.grid_size * self.zoom;

        let offset_x = self.view_offset.x % grid_spacing;
        let offset_y = self.view_offset.y % grid_spacing;

        // Vertical lines
        let mut x = rect.min.x + offset_x;
        while x < rect.max.x {
            painter.line_segment(
                [Pos2::new(x, rect.min.y), Pos2::new(x, rect.max.y)],
                Stroke::new(1.0, grid_color),
            );
            x += grid_spacing;
        }

        // Horizontal lines
        let mut y = rect.min.y + offset_y;
        while y < rect.max.y {
            painter.line_segment(
                [Pos2::new(rect.min.x, y), Pos2::new(rect.max.x, y)],
                Stroke::new(1.0, grid_color),
            );
            y += grid_spacing;
        }
    }

    fn draw_connections(&self, painter: &egui::Painter, rect: Rect) {
        let connection_color = Color32::from_gray(150);
        let selected_color = Color32::from_rgb(100, 200, 255);

        for step in &self.workflow.steps {
            if let Some((x1, y1)) = step.position {
                let start_pos = self.canvas_to_screen(Pos2::new(x1, y1), rect);

                // Draw connection to next step
                if let Some(ref next_id) = step.next_step {
                    if let Some(next_step) = self.workflow.get_step(next_id) {
                        if let Some((x2, y2)) = next_step.position {
                            let end_pos = self.canvas_to_screen(Pos2::new(x2, y2), rect);
                            let color = if self.selected_step.as_ref() == Some(&step.id) {
                                selected_color
                            } else {
                                connection_color
                            };
                            self.draw_connection_line(painter, start_pos, end_pos, color, true);
                        }
                    }
                }

                // Draw false branch for conditions
                if step.step_type == StepType::Condition {
                    if let Some(ref false_id) = step.false_branch {
                        if let Some(false_step) = self.workflow.get_step(false_id) {
                            if let Some((x2, y2)) = false_step.position {
                                let end_pos = self.canvas_to_screen(Pos2::new(x2, y2), rect);
                                self.draw_connection_line(
                                    painter,
                                    start_pos,
                                    end_pos,
                                    Color32::RED,
                                    false,
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    fn draw_connection_line(
        &self,
        painter: &egui::Painter,
        start: Pos2,
        end: Pos2,
        color: Color32,
        is_true_branch: bool,
    ) {
        // Draw curved connection
        let mid_x = (start.x + end.x) / 2.0;
        let cp1 = Pos2::new(mid_x, start.y);
        let cp2 = Pos2::new(mid_x, end.y);

        // Bezier curve
        let points: Vec<Pos2> = (0..=20)
            .map(|i| {
                let t = i as f32 / 20.0;
                self.bezier_point(start, cp1, cp2, end, t)
            })
            .collect();

        for i in 0..points.len() - 1 {
            painter.line_segment([points[i], points[i + 1]], Stroke::new(2.0, color));
        }

        // Arrow head
        let arrow_size = 10.0;
        let angle = (end.y - points[points.len() - 2].y).atan2(end.x - points[points.len() - 2].x);
        let arrow_p1 = Pos2::new(
            end.x - arrow_size * (angle + 0.5).cos(),
            end.y - arrow_size * (angle + 0.5).sin(),
        );
        let arrow_p2 = Pos2::new(
            end.x - arrow_size * (angle - 0.5).cos(),
            end.y - arrow_size * (angle - 0.5).sin(),
        );

        painter.line_segment([end, arrow_p1], Stroke::new(2.0, color));
        painter.line_segment([end, arrow_p2], Stroke::new(2.0, color));

        // Label for condition branches
        if !is_true_branch {
            let label_pos = Pos2::new((start.x + end.x) / 2.0, (start.y + end.y) / 2.0 - 10.0);
            painter.text(
                label_pos,
                egui::Align2::CENTER_CENTER,
                "No",
                egui::FontId::monospace(12.0),
                Color32::RED,
            );
        }
    }

    fn bezier_point(&self, p0: Pos2, p1: Pos2, p2: Pos2, p3: Pos2, t: f32) -> Pos2 {
        let u = 1.0 - t;
        let tt = t * t;
        let uu = u * u;
        let uuu = uu * u;
        let ttt = tt * t;

        Pos2::new(
            uuu * p0.x + 3.0 * uu * t * p1.x + 3.0 * u * tt * p2.x + ttt * p3.x,
            uuu * p0.y + 3.0 * uu * t * p1.y + 3.0 * u * tt * p2.y + ttt * p3.y,
        )
    }

    fn draw_step(
        &self,
        ui: &mut Ui,
        rect: Rect,
        step: &WorkflowStep,
        painter: &egui::Painter,
    ) -> Option<(Response, Rect)> {
        let pos = step.position?;
        let screen_pos = self.canvas_to_screen(Pos2::new(pos.0, pos.1), rect);

        // Step node size
        let size = Vec2::new(140.0 * self.zoom, 60.0 * self.zoom);
        let step_rect = Rect::from_center_size(screen_pos, size);

        // Step color based on type
        let (bg_color, border_color, icon) = self.get_step_style(step);

        // Draw step node
        let rounding = Rounding::same(8.0);
        painter.rect_filled(step_rect, rounding, bg_color);
        painter.rect_stroke(step_rect, rounding, Stroke::new(2.0, border_color));

        // Icon
        let icon_pos = step_rect.min + Vec2::new(10.0 * self.zoom, step_rect.height() / 2.0);
        painter.text(
            icon_pos,
            egui::Align2::LEFT_CENTER,
            icon,
            egui::FontId::monospace(16.0 * self.zoom),
            Color32::WHITE,
        );

        // Step name
        let text_pos = step_rect.center();
        let font_size = (14.0 * self.zoom).max(10.0);
        painter.text(
            text_pos,
            egui::Align2::CENTER_CENTER,
            &step.name,
            egui::FontId::proportional(font_size),
            Color32::WHITE,
        );

        // Preview mode highlighting
        if self.preview_mode && self.preview_current_step.as_ref() == Some(&step.id) {
            // Pulsing highlight
            let pulse = (ui.ctx().input(|i| i.time) * 4.0).sin() * 0.5 + 0.5;
            let highlight_color =
                Color32::from_rgba_premultiplied(100, 200, 255, (128.0 + 127.0 * pulse) as u8);
            painter.rect_stroke(
                step_rect.expand(4.0 + 4.0 * pulse as f32),
                rounding,
                Stroke::new(3.0, highlight_color),
            );
        }

        // Selected highlight
        if self.selected_step.as_ref() == Some(&step.id) {
            painter.rect_stroke(
                step_rect.expand(4.0),
                rounding,
                Stroke::new(2.0, Color32::YELLOW),
            );
        }

        // Step type indicator
        let type_pos = step_rect.max - Vec2::new(8.0 * self.zoom, 8.0 * self.zoom);
        let type_color = self.get_step_type_color(&step.step_type);
        painter.circle_filled(type_pos, 4.0 * self.zoom, type_color);

        // Interactive area
        let response = ui.allocate_rect(step_rect, Sense::click_and_drag());

        Some((response, step_rect))
    }

    fn get_step_style(&self, step: &WorkflowStep) -> (Color32, Color32, &'static str) {
        if !step.enabled {
            return (Color32::from_gray(60), Color32::from_gray(100), "⏸");
        }

        match step.step_type {
            StepType::SshCommand => (
                Color32::from_rgb(45, 55, 72),
                Color32::from_rgb(100, 150, 255),
                "$",
            ),
            StepType::SftpUpload => (
                Color32::from_rgb(45, 72, 55),
                Color32::from_rgb(100, 255, 150),
                "↑",
            ),
            StepType::SftpDownload => (
                Color32::from_rgb(55, 45, 72),
                Color32::from_rgb(150, 100, 255),
                "↓",
            ),
            StepType::LocalCommand => (
                Color32::from_rgb(72, 55, 45),
                Color32::from_rgb(255, 150, 100),
                "⌘",
            ),
            StepType::Condition => (
                Color32::from_rgb(72, 72, 45),
                Color32::from_rgb(255, 255, 100),
                "?",
            ),
            StepType::Loop => (
                Color32::from_rgb(55, 72, 72),
                Color32::from_rgb(100, 255, 255),
                "↻",
            ),
            StepType::Wait => (
                Color32::from_rgb(72, 72, 72),
                Color32::from_rgb(200, 200, 200),
                "◷",
            ),
            StepType::SetVariable => (
                Color32::from_rgb(55, 55, 72),
                Color32::from_rgb(150, 150, 255),
                "=",
            ),
            StepType::Notification => (
                Color32::from_rgb(72, 72, 55),
                Color32::from_rgb(255, 255, 150),
                "🔔",
            ),
            StepType::Parallel => (
                Color32::from_rgb(45, 72, 72),
                Color32::from_rgb(100, 255, 255),
                "∥",
            ),
            StepType::SubWorkflow => (
                Color32::from_rgb(55, 55, 55),
                Color32::from_rgb(200, 200, 200),
                "⊃",
            ),
            StepType::ErrorHandler => (
                Color32::from_rgb(72, 45, 45),
                Color32::from_rgb(255, 100, 100),
                "⚠",
            ),
            StepType::Break => (
                Color32::from_rgb(72, 55, 72),
                Color32::from_rgb(255, 150, 255),
                "■",
            ),
            StepType::Continue => (
                Color32::from_rgb(55, 72, 55),
                Color32::from_rgb(150, 255, 150),
                "→",
            ),
            StepType::Return => (
                Color32::from_rgb(72, 72, 72),
                Color32::from_rgb(255, 255, 255),
                "↩",
            ),
        }
    }

    fn get_step_type_color(&self, step_type: &StepType) -> Color32 {
        match step_type {
            StepType::SshCommand => Color32::BLUE,
            StepType::SftpUpload => Color32::GREEN,
            StepType::SftpDownload => Color32::from_rgb(128, 0, 128),
            StepType::LocalCommand => Color32::from_rgb(255, 165, 0),
            StepType::Condition => Color32::YELLOW,
            StepType::Loop => Color32::from_rgb(0, 255, 255),
            StepType::Wait => Color32::GRAY,
            StepType::SetVariable => Color32::LIGHT_BLUE,
            StepType::Notification => Color32::GOLD,
            StepType::Parallel => Color32::LIGHT_GREEN,
            StepType::SubWorkflow => Color32::WHITE,
            StepType::ErrorHandler => Color32::RED,
            StepType::Break => Color32::DARK_RED,
            StepType::Continue => Color32::DARK_GREEN,
            StepType::Return => Color32::WHITE,
        }
    }

    fn render_sidebar(&mut self, ui: &mut Ui, rect: Rect, response: &mut WorkflowEditorResponse) {
        let frame = Frame::side_top_panel(ui.style()).inner_margin(Margin::same(8.0));
        ui.allocate_ui_at_rect(rect, |ui| {
            frame.show(ui, |ui| {
                ui.set_min_width(rect.width());

                if let Some(ref step_id) = self.selected_step {
                    if let Some(step) = self.workflow.get_step(step_id).cloned() {
                        self.render_step_properties(ui, &step, response);
                    }
                } else {
                    self.render_workflow_properties(ui, response);
                }
            });
        });
    }

    fn render_step_properties(
        &mut self,
        ui: &mut Ui,
        step: &WorkflowStep,
        response: &mut WorkflowEditorResponse,
    ) {
        ui.heading("Step Properties");
        ui.separator();

        // Step name
        ui.label("Name:");
        let mut name = step.name.clone();
        if ui.text_edit_singleline(&mut name).changed() {
            if let Some(s) = self.workflow.get_step_mut(&step.id) {
                s.name = name;
            }
        }

        // Step type (read-only display)
        ui.label(format!("Type: {:?}", step.step_type));

        // Description
        ui.label("Description:");
        let mut desc = step.description.clone().unwrap_or_default();
        if ui.text_edit_multiline(&mut desc).changed() {
            if let Some(s) = self.workflow.get_step_mut(&step.id) {
                s.description = if desc.is_empty() { None } else { Some(desc) };
            }
        }

        // Enabled checkbox
        let mut enabled = step.enabled;
        if ui.checkbox(&mut enabled, "Enabled").changed() {
            if let Some(s) = self.workflow.get_step_mut(&step.id) {
                s.enabled = enabled;
            }
        }

        ui.separator();

        // Step-specific configuration
        let step_id = step.id.clone();
        match &step.config {
            StepConfig::SshCommand {
                command,
                fail_on_error,
                ..
            } => {
                ui.label("Command:");
                let mut cmd = command.clone();
                if ui.text_edit_multiline(&mut cmd).changed() {
                    if let Some(s) = self.workflow.get_step_mut(&step_id) {
                        if let StepConfig::SshCommand { command, .. } = &mut s.config {
                            *command = cmd;
                        }
                    }
                }

                let mut fail = *fail_on_error;
                if ui.checkbox(&mut fail, "Fail on error").changed() {
                    if let Some(s) = self.workflow.get_step_mut(&step_id) {
                        if let StepConfig::SshCommand { fail_on_error, .. } = &mut s.config {
                            *fail_on_error = fail;
                        }
                    }
                }
            }
            StepConfig::Condition {
                operator,
                left_operand,
                right_operand,
                ..
            } => {
                ui.label("Condition:");
                ui.label("Left operand:");
                let mut left = left_operand.clone();
                ui.text_edit_singleline(&mut left);

                ui.label("Operator:");
                let mut op = operator.clone();
                egui::ComboBox::from_id_source("operator")
                    .selected_text(&op)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut op, "eq".to_string(), "Equals");
                        ui.selectable_value(&mut op, "ne".to_string(), "Not equals");
                        ui.selectable_value(&mut op, "gt".to_string(), "Greater than");
                        ui.selectable_value(&mut op, "lt".to_string(), "Less than");
                        ui.selectable_value(&mut op, "contains".to_string(), "Contains");
                    });

                ui.label("Right operand:");
                let mut right = right_operand.clone();
                ui.text_edit_singleline(&mut right);
            }
            _ => {
                ui.label("Configuration options for this step type coming soon...");
            }
        }

        ui.separator();

        // Error handling
        ui.label("Error Handling:");
        let mut action = step.error_handling.action.clone();
        egui::ComboBox::from_id_source("error_action")
            .selected_text(format!("{:?}", action))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut action, ErrorAction::Abort, "Abort workflow");
                ui.selectable_value(&mut action, ErrorAction::Skip, "Skip to next");
                ui.selectable_value(&mut action, ErrorAction::Retry, "Retry");
                ui.selectable_value(&mut action, ErrorAction::Ignore, "Ignore and continue");
            });

        // Timeout
        ui.label("Timeout (seconds):");
        let mut timeout = step.timeout_secs.unwrap_or(0) as f64;
        ui.add(DragValue::new(&mut timeout).speed(1.0).range(0.0..=3600.0));

        ui.separator();

        // Actions
        if ui.button("Duplicate Step").clicked() {
            response.duplicate_step = Some(step.id.clone());
        }
        if ui.button("Delete Step").clicked() {
            response.delete_step = Some(step.id.clone());
            self.selected_step = None;
        }

        // Connection controls
        ui.separator();
        ui.label("Connections:");
        if ui.button("Connect to...").clicked() {
            self.connecting_from = Some((step.id.clone(), true));
        }
        if step.step_type == StepType::Condition
            && ui.button("Connect false branch to...").clicked()
        {
            self.connecting_from = Some((step.id.clone(), false));
        }
    }

    fn render_workflow_properties(&mut self, ui: &mut Ui, _response: &mut WorkflowEditorResponse) {
        ui.heading("Workflow Properties");
        ui.separator();

        // Workflow name
        ui.label("Name:");
        ui.text_edit_singleline(&mut self.workflow.name);

        // Description
        ui.label("Description:");
        let mut desc = self.workflow.description.clone().unwrap_or_default();
        if ui.text_edit_multiline(&mut desc).changed() {
            self.workflow.description = if desc.is_empty() { None } else { Some(desc) };
        }

        // Category
        ui.label("Category:");
        let mut cat = self.workflow.category.clone().unwrap_or_default();
        ui.text_edit_singleline(&mut cat);
        self.workflow.category = if cat.is_empty() { None } else { Some(cat) };

        ui.separator();

        // Variables
        ui.heading("Variables");
        ui.label(format!(
            "{} variables defined",
            self.workflow.variables.len()
        ));

        ui.separator();

        // Validation results
        if !self.validation_errors.is_empty() {
            ui.heading("Validation");
            for error in &self.validation_errors {
                ui.colored_label(Color32::YELLOW, error);
            }
        }

        ui.separator();

        // Stats
        ui.label(format!("Steps: {}", self.workflow.steps.len()));
        ui.label(format!("Version: {}", self.workflow.version));
        ui.label(format!(
            "Created: {}",
            self.workflow.created_at.format("%Y-%m-%d %H:%M")
        ));
    }

    fn render_step_type_selector(
        &mut self,
        ui: &mut Ui,
        pos: Pos2,
        _response: &mut WorkflowEditorResponse,
    ) {
        let id = ui.make_persistent_id("step_type_selector");
        let area = egui::Area::new(id)
            .fixed_pos(pos)
            .movable(false)
            .order(egui::Order::Foreground);

        area.show(ui.ctx(), |ui| {
            Frame::popup(ui.style()).show(ui, |ui| {
                ui.set_min_width(200.0);
                ui.heading("Add Step");
                ui.separator();

                let step_types = vec![
                    (
                        StepType::SshCommand,
                        "$ SSH Command",
                        "Execute command on remote server",
                    ),
                    (
                        StepType::SftpUpload,
                        "↑ Upload File",
                        "Upload file via SFTP",
                    ),
                    (
                        StepType::SftpDownload,
                        "↓ Download File",
                        "Download file via SFTP",
                    ),
                    (StepType::Condition, "? Condition", "If/then/else branching"),
                    (StepType::Loop, "↻ Loop", "Repeat actions"),
                    (StepType::Wait, "◷ Wait", "Pause execution"),
                    (
                        StepType::SetVariable,
                        "= Set Variable",
                        "Create or update variable",
                    ),
                    (StepType::Notification, "🔔 Notify", "Send notification"),
                    (StepType::Parallel, "∥ Parallel", "Run steps in parallel"),
                    (
                        StepType::SubWorkflow,
                        "⊃ Sub-workflow",
                        "Call another workflow",
                    ),
                ];

                for (step_type, label, desc) in step_types {
                    if ui.button(format!("{} - {}", label, desc)).clicked() {
                        let new_step =
                            WorkflowStep::new(step_type.clone(), &format!("{:?}", step_type))
                                .with_position(pos.x, pos.y);
                        let step_id = self.workflow.add_step(new_step);
                        self.selected_step = Some(step_id);
                        self.step_type_selector_open = false;
                        self.new_step_position = None;
                    }
                }

                ui.separator();
                if ui.button("Cancel").clicked() {
                    self.step_type_selector_open = false;
                    self.new_step_position = None;
                }
            });
        });
    }

    fn screen_to_canvas(&self, screen_pos: Pos2, rect: Rect) -> Pos2 {
        Pos2::new(
            (screen_pos.x - rect.min.x - self.view_offset.x) / self.zoom,
            (screen_pos.y - rect.min.y - self.view_offset.y) / self.zoom,
        )
    }

    fn canvas_to_screen(&self, canvas_pos: Pos2, rect: Rect) -> Pos2 {
        Pos2::new(
            canvas_pos.x * self.zoom + rect.min.x + self.view_offset.x,
            canvas_pos.y * self.zoom + rect.min.y + self.view_offset.y,
        )
    }

    /// Process editor responses and update state
    pub fn process_response(&mut self, response: &WorkflowEditorResponse) {
        if let Some(ref step_id) = response.delete_step {
            self.workflow.remove_step(step_id);
        }

        if let Some(ref step_id) = response.duplicate_step {
            if let Some(step) = self.workflow.get_step(step_id).cloned() {
                let mut new_step = step;
                new_step.id = Uuid::new_v4().to_string();
                new_step.name = format!("{} (Copy)", new_step.name);
                if let Some((x, y)) = new_step.position {
                    new_step.position = Some((x + 50.0, y + 50.0));
                }
                self.workflow.add_step(new_step);
            }
        }
    }

    /// Get the current workflow
    pub fn workflow(&self) -> &Workflow {
        &self.workflow
    }

    /// Set workflow
    pub fn set_workflow(&mut self, workflow: Workflow) {
        self.workflow = workflow;
        self.selected_step = None;
    }
}

/// Response from workflow editor
#[derive(Debug, Default, Clone)]
pub struct WorkflowEditorResponse {
    pub workflow: Option<Workflow>,
    pub selected_step: Option<String>,
    pub save_requested: bool,
    pub run_requested: bool,
    pub delete_step: Option<String>,
    pub duplicate_step: Option<String>,
}

/// Workflow library browser
pub struct ScriptLibraryBrowser {
    search_query: String,
    selected_category: Option<String>,
    selected_script: Option<String>,
    view_mode: LibraryViewMode,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LibraryViewMode {
    Grid,
    List,
}

impl ScriptLibraryBrowser {
    pub fn new() -> Self {
        Self {
            search_query: String::new(),
            selected_category: None,
            selected_script: None,
            view_mode: LibraryViewMode::Grid,
        }
    }

    pub fn ui(&mut self, ui: &mut Ui, library: &ScriptLibrary) -> Option<ScriptSelection> {
        let mut selection: Option<ScriptSelection> = None;

        // Search bar
        ui.horizontal(|ui| {
            ui.text_edit_singleline(&mut self.search_query);

            ui.label("View:");
            ui.selectable_value(&mut self.view_mode, LibraryViewMode::Grid, "Grid");
            ui.selectable_value(&mut self.view_mode, LibraryViewMode::List, "List");
        });

        ui.separator();

        // Category filter
        ui.horizontal_wrapped(|ui| {
            if ui
                .selectable_label(self.selected_category.is_none(), "All")
                .clicked()
            {
                self.selected_category = None;
            }
            for cat in library.get_categories() {
                let selected = self.selected_category.as_ref() == Some(&cat.id);
                if ui.selectable_label(selected, &cat.name).clicked() {
                    self.selected_category = Some(cat.id.clone());
                }
            }
        });

        ui.separator();

        // Script grid/list
        let scripts = library.search(easyssh_core::script_library::ScriptSearchOptions {
            query: if self.search_query.is_empty() {
                None
            } else {
                Some(self.search_query.clone())
            },
            category: self.selected_category.clone(),
            script_type: Some(easyssh_core::script_library::ScriptType::All),
            ..Default::default()
        });

        egui::ScrollArea::vertical().show(ui, |ui| match self.view_mode {
            LibraryViewMode::Grid => {
                ui.horizontal_wrapped(|ui| {
                    for script in scripts {
                        if let Some(sel) = self.render_script_card(ui, &script) {
                            selection = Some(sel);
                        }
                    }
                });
            }
            LibraryViewMode::List => {
                for script in scripts {
                    if let Some(sel) = self.render_script_list_item(ui, &script) {
                        selection = Some(sel);
                    }
                }
            }
        });

        selection
    }

    fn render_script_card(
        &self,
        ui: &mut Ui,
        script: &easyssh_core::script_library::ScriptSearchResult,
    ) -> Option<ScriptSelection> {
        let size = Vec2::new(200.0, 120.0);
        let (rect, response) = ui.allocate_exact_size(size, Sense::click());

        let painter = ui.painter_at(rect);

        // Card background
        let bg_color = if response.hovered() {
            Color32::from_gray(50)
        } else {
            Color32::from_gray(40)
        };
        painter.rect_filled(rect, Rounding::same(8.0), bg_color);

        // Type indicator color
        let type_color = match script.script_type() {
            easyssh_core::script_library::ScriptType::Workflow => Color32::from_rgb(100, 150, 255),
            easyssh_core::script_library::ScriptType::Macro => Color32::from_rgb(100, 255, 150),
            _ => Color32::GRAY,
        };
        painter.rect_filled(
            Rect::from_min_size(rect.min, Vec2::new(4.0, rect.height())),
            Rounding::same(8.0),
            type_color,
        );

        // Script name
        painter.text(
            rect.min + Vec2::new(16.0, 16.0),
            egui::Align2::LEFT_TOP,
            script.name(),
            egui::FontId::proportional(16.0),
            Color32::WHITE,
        );

        // Description
        if let Some(desc) = script.description() {
            let desc_text = if desc.len() > 60 {
                format!("{}...", &desc[..60])
            } else {
                desc.to_string()
            };
            painter.text(
                rect.min + Vec2::new(16.0, 44.0),
                egui::Align2::LEFT_TOP,
                desc_text,
                egui::FontId::proportional(12.0),
                Color32::from_gray(180),
            );
        }

        // Stats
        let stats_text = format!("Used {} times", script.metadata().usage_count);
        painter.text(
            rect.min + Vec2::new(16.0, 90.0),
            egui::Align2::LEFT_TOP,
            stats_text,
            egui::FontId::proportional(11.0),
            Color32::from_gray(150),
        );

        if response.clicked() {
            Some(ScriptSelection {
                id: script.id().to_string(),
                script_type: script.script_type(),
            })
        } else {
            None
        }
    }

    fn render_script_list_item(
        &self,
        ui: &mut Ui,
        script: &easyssh_core::script_library::ScriptSearchResult,
    ) -> Option<ScriptSelection> {
        let height = 60.0;
        let (rect, response) =
            ui.allocate_exact_size(Vec2::new(ui.available_width(), height), Sense::click());

        let painter = ui.painter_at(rect);

        // Background
        let bg_color = if response.hovered() {
            Color32::from_gray(50)
        } else {
            Color32::from_gray(35)
        };
        painter.rect_filled(rect, Rounding::same(4.0), bg_color);

        // Type icon
        let icon = match script.script_type() {
            easyssh_core::script_library::ScriptType::Workflow => "W",
            easyssh_core::script_library::ScriptType::Macro => "M",
            _ => "?",
        };
        let type_color = match script.script_type() {
            easyssh_core::script_library::ScriptType::Workflow => Color32::from_rgb(100, 150, 255),
            easyssh_core::script_library::ScriptType::Macro => Color32::from_rgb(100, 255, 150),
            _ => Color32::GRAY,
        };
        painter.circle_filled(
            rect.min + Vec2::new(20.0, rect.height() / 2.0),
            12.0,
            type_color,
        );
        painter.text(
            rect.min + Vec2::new(20.0, rect.height() / 2.0),
            egui::Align2::CENTER_CENTER,
            icon,
            egui::FontId::monospace(10.0),
            Color32::WHITE,
        );

        // Name and description
        painter.text(
            rect.min + Vec2::new(48.0, 12.0),
            egui::Align2::LEFT_TOP,
            script.name(),
            egui::FontId::proportional(14.0),
            Color32::WHITE,
        );

        if let Some(desc) = script.description() {
            painter.text(
                rect.min + Vec2::new(48.0, 32.0),
                egui::Align2::LEFT_TOP,
                desc,
                egui::FontId::proportional(12.0),
                Color32::from_gray(180),
            );
        }

        if response.clicked() {
            Some(ScriptSelection {
                id: script.id().to_string(),
                script_type: script.script_type(),
            })
        } else {
            None
        }
    }
}

/// Selected script info
#[derive(Debug, Clone)]
pub struct ScriptSelection {
    pub id: String,
    pub script_type: easyssh_core::script_library::ScriptType,
}
