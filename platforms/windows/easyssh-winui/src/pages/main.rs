use windows::core::*;
use windows::UI::Xaml::*;
use windows::UI::Xaml::Controls::*;
use windows::UI::Xaml::Media::*;

pub struct MainPage {
    page: Controls::Page,
}

impl MainPage {
    pub fn new() -> Result<Self> {
        let page = Controls::Page::new()?;

        // Create main layout: NavigationView
        let nav_view = NavigationView::new()?;
        nav_view.SetIsSettingsVisible(true)?;

        // Menu items
        let servers_item = NavigationViewItem::new()?;
        servers_item.SetContent(IInspectable::from(h!("Servers")))?;
        servers_item.SetIcon(IconElement::from(FontIcon::CreateFontIconWithGlyph(h!("\uE977")))?)?;

        let groups_item = NavigationViewItem::new()?;
        groups_item.SetContent(IInspectable::from(h!("Groups")))?;
        groups_item.SetIcon(IconElement::from(FontIcon::CreateFontIconWithGlyph(h!("\uE8B7")))?)?;

        let menu_items = nav_view.MenuItems()?;
        menu_items.Append(servers_item)?;
        menu_items.Append(groups_item)?;

        // Content frame
        let content_frame = Frame::new()?;
        nav_view.SetContent(content_frame)?;

        page.SetContent(nav_view)?;

        Ok(Self { page })
    }
}

impl std::ops::Deref for MainPage {
    type Target = Controls::Page;

    fn deref(&self) -> &Self::Target {
        &self.page
    }
}
