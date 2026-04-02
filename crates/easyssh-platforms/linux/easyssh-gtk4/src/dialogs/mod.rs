use gtk4::prelude::*;
use libadwaita::prelude::*;

mod add_server_dialog;
mod edit_server_dialog;
mod group_dialog;
mod master_password_dialog;
mod password_dialog;

pub use add_server_dialog::show_add_server_dialog;
pub use edit_server_dialog::show_edit_server_dialog;
pub use group_dialog::show_add_group_dialog;
pub use master_password_dialog::*;
pub use password_dialog::show_password_dialog;

pub fn show_confirm_delete_dialog<F>(parent: &adw::ApplicationWindow, item_name: &str, callback: F)
where
    F: FnOnce() + 'static,
{
    let dialog = adw::MessageDialog::builder()
        .heading("Delete Server?")
        .body(&format!(
            "Are you sure you want to delete \"{}\"?\n\nThis action cannot be undone.",
            item_name
        ))
        .transient_for(parent)
        .modal(true)
        .build();

    dialog.add_response("cancel", "Cancel");
    dialog.add_response("delete", "Delete");
    dialog.set_response_appearance("delete", adw::ResponseAppearance::Destructive);
    dialog.set_default_response(Some("cancel"));

    dialog.connect_response(None, move |_, response| {
        if response == "delete" {
            callback();
        }
    });

    dialog.present();
}
