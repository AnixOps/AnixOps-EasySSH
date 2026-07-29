use eframe::egui;

pub fn icon_button(ui: &mut egui::Ui, glyph: &str, tooltip: &str) -> egui::Response {
    ui.add_sized(
        [34.0, 33.0],
        egui::Button::new(egui::RichText::new(glyph).size(19.0)),
    )
    .on_hover_text(tooltip)
}

pub fn status_badge(ui: &mut egui::Ui, text: &str, color: egui::Color32) {
    ui.label(egui::RichText::new(text).small().color(color));
}
