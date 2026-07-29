use eframe::egui;

#[derive(Clone, Copy)]
pub struct Palette {
    pub canvas: egui::Color32,
    pub surface: egui::Color32,
    pub surface_muted: egui::Color32,
    pub text: egui::Color32,
    pub muted: egui::Color32,
    pub border: egui::Color32,
    pub primary: egui::Color32,
    pub success: egui::Color32,
    pub warning: egui::Color32,
    pub danger: egui::Color32,
}

pub const LIGHT: Palette = Palette {
    canvas: egui::Color32::from_rgb(246, 247, 251),
    surface: egui::Color32::WHITE,
    surface_muted: egui::Color32::from_rgb(236, 238, 246),
    text: egui::Color32::from_rgb(30, 33, 43),
    muted: egui::Color32::from_rgb(102, 109, 125),
    border: egui::Color32::from_rgb(210, 214, 225),
    primary: egui::Color32::from_rgb(86, 101, 230),
    success: egui::Color32::from_rgb(42, 146, 91),
    warning: egui::Color32::from_rgb(176, 120, 31),
    danger: egui::Color32::from_rgb(193, 58, 69),
};
pub const DARK: Palette = Palette {
    canvas: egui::Color32::from_rgb(18, 20, 27),
    surface: egui::Color32::from_rgb(30, 33, 43),
    surface_muted: egui::Color32::from_rgb(25, 28, 38),
    text: egui::Color32::from_rgb(235, 237, 244),
    muted: egui::Color32::from_rgb(163, 170, 188),
    border: egui::Color32::from_rgb(65, 70, 84),
    primary: egui::Color32::from_rgb(126, 137, 255),
    success: egui::Color32::from_rgb(78, 194, 132),
    warning: egui::Color32::from_rgb(225, 171, 72),
    danger: egui::Color32::from_rgb(223, 91, 101),
};

pub const CORNER: f32 = 6.0;
pub const CONTROL_HEIGHT: f32 = 32.0;
pub const MOTION_MS: u32 = 180;
