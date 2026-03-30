#![windows_subsystem = "windows"]

use std::sync::Arc;
use std::sync::Mutex;
use windows::core::*;
use windows::UI::Xaml::*;
use windows::UI::Xaml::Controls::*;
use windows::UI::Xaml::Navigation::*;
use windows::ApplicationModel::Activation::*;
use tracing::{info, warn, error};

mod pages;
mod viewmodels;

use pages::MainPage;
use viewmodels::AppViewModel;

fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    info!("Starting EasySSH for Windows");

    // Initialize WinUI
    windows::UI::Xaml::Application::Start(ApplicationCallback)?;

    Ok(())
}

#[implement(IApplicationOverrides)]
struct App {
    _view_model: Arc<Mutex<AppViewModel>>,
}

impl App {
    fn new() -> Result<Self> {
        let view_model = Arc::new(Mutex::new(AppViewModel::new()?));

        Ok(Self {
            _view_model: view_model,
        })
    }

    fn OnLaunched(&self, args: &Option<LaunchActivatedEventArgs>) -> Result<()> {
        info!("Application launched");

        // Create main window
        let window = Controls::Window::new()?;
        window.SetTitle(h!("EasySSH"))?;
        window.SetWidth(1000.0)?;
        window.SetHeight(700.0)?;

        // Create main page
        let main_page = MainPage::new()?;
        window.SetContent(main_page)?;

        // Activate window
        window.Activate()?;

        Ok(())
    }
}

fn ApplicationCallback() -> Result<()> {
    let app = App::new()?;
    let _ = Application::new(app)?;
    Ok(())
}
