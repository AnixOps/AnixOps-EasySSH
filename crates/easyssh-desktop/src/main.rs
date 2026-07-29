mod app;
mod ui;
#[cfg(feature = "ui-test")]
mod ui_test;

fn main() -> eframe::Result<()> {
    app::run()
}
