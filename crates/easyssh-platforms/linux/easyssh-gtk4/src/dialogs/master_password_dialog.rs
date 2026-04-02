//! Master Password Dialog for EasySSH Lite (GTK4/Libadwaita)
//!
//! Provides secure master password management:
//! - First-time setup
//! - Application unlock
//! - Password change
//! - Reset with data loss warning

use gtk4::prelude::*;
use libadwaita::prelude::*;

/// Dialog modes for master password operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MasterPasswordMode {
    Setup,
    Verify,
    Change,
    Reset,
}

/// Result of the master password dialog
#[derive(Debug, Clone)]
pub enum MasterPasswordResult {
    Cancelled,
    SetPassword {
        password: String,
    },
    Verify {
        password: String,
    },
    ChangePassword {
        old_password: String,
        new_password: String,
    },
    ResetConfirmed,
    ForgotPassword,
}

/// Password strength levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PasswordStrength {
    VeryWeak,
    Weak,
    Fair,
    Good,
    Strong,
}

impl PasswordStrength {
    fn from_score(score: u32) -> Self {
        match score {
            0..=20 => PasswordStrength::VeryWeak,
            21..=40 => PasswordStrength::Weak,
            41..=60 => PasswordStrength::Fair,
            61..=80 => PasswordStrength::Good,
            _ => PasswordStrength::Strong,
        }
    }

    fn as_str(&self) -> &'static str {
        match self {
            PasswordStrength::VeryWeak => "Very Weak",
            PasswordStrength::Weak => "Weak",
            PasswordStrength::Fair => "Fair",
            PasswordStrength::Good => "Good",
            PasswordStrength::Strong => "Strong",
        }
    }

    fn css_class(&self) -> &'static str {
        match self {
            PasswordStrength::VeryWeak => "error",
            PasswordStrength::Weak => "warning",
            PasswordStrength::Fair => "warning",
            PasswordStrength::Good => "success",
            PasswordStrength::Strong => "success",
        }
    }
}

/// Calculate password strength score
fn calculate_password_strength(password: &str) -> (u32, Vec<&'static str>) {
    let mut score = 0u32;
    let mut feedback = Vec::new();

    // Length check
    if password.len() >= 8 {
        score += 20;
    } else {
        feedback.push("Password must be at least 8 characters");
    }
    if password.len() >= 12 {
        score += 10;
    }
    if password.len() >= 16 {
        score += 10;
    }

    // Character variety
    if password.chars().any(|c| c.is_ascii_lowercase()) {
        score += 15;
    } else {
        feedback.push("Add lowercase letters");
    }

    if password.chars().any(|c| c.is_ascii_uppercase()) {
        score += 15;
    } else {
        feedback.push("Add uppercase letters");
    }

    if password.chars().any(|c| c.is_ascii_digit()) {
        score += 15;
    } else {
        feedback.push("Add numbers");
    }

    if password.chars().any(|c| !c.is_alphanumeric()) {
        score += 15;
    } else {
        feedback.push("Add special characters");
    }

    (score, feedback)
}

/// Show master password dialog for first-time setup
pub fn show_master_password_setup<F>(parent: &adw::ApplicationWindow, callback: F)
where
    F: FnOnce(MasterPasswordResult) + 'static,
{
    show_master_password_dialog(parent, MasterPasswordMode::Setup, 0, callback);
}

/// Show master password dialog for verification
pub fn show_master_password_verify<F>(
    parent: &adw::ApplicationWindow,
    failed_attempts: u32,
    callback: F,
) where
    F: FnOnce(MasterPasswordResult) + 'static,
{
    show_master_password_dialog(
        parent,
        MasterPasswordMode::Verify,
        failed_attempts,
        callback,
    );
}

/// Show master password dialog for changing password
pub fn show_master_password_change<F>(parent: &adw::ApplicationWindow, callback: F)
where
    F: FnOnce(MasterPasswordResult) + 'static,
{
    show_master_password_dialog(parent, MasterPasswordMode::Change, 0, callback);
}

/// Show master password reset warning dialog
pub fn show_master_password_reset<F>(parent: &adw::ApplicationWindow, callback: F)
where
    F: FnOnce(MasterPasswordResult) + 'static,
{
    show_master_password_dialog(parent, MasterPasswordMode::Reset, 0, callback);
}

/// Main master password dialog implementation
fn show_master_password_dialog<F>(
    parent: &adw::ApplicationWindow,
    mode: MasterPasswordMode,
    failed_attempts: u32,
    callback: F,
) where
    F: FnOnce(MasterPasswordResult) + 'static,
{
    let dialog = adw::Dialog::builder()
        .title(match mode {
            MasterPasswordMode::Setup => "Set Master Password",
            MasterPasswordMode::Verify => "Unlock EasySSH",
            MasterPasswordMode::Change => "Change Master Password",
            MasterPasswordMode::Reset => "Reset Master Password",
        })
        .content_width(450)
        .content_height(match mode {
            MasterPasswordMode::Setup => 520,
            MasterPasswordMode::Verify => 350,
            MasterPasswordMode::Change => 550,
            MasterPasswordMode::Reset => 450,
        })
        .build();

    let toolbar_view = adw::ToolbarView::new();

    // Header bar
    let header = adw::HeaderBar::new();
    header.add_css_class("flat");

    let cancel_button = gtk4::Button::builder().label("Cancel").build();
    header.pack_start(&cancel_button);

    let action_button = gtk4::Button::builder()
        .label(match mode {
            MasterPasswordMode::Setup => "Set Password",
            MasterPasswordMode::Verify => "Unlock",
            MasterPasswordMode::Change => "Change Password",
            MasterPasswordMode::Reset => "Reset",
        })
        .css_classes(vec![if mode == MasterPasswordMode::Reset {
            "destructive-action"
        } else {
            "suggested-action"
        }])
        .build();
    header.pack_end(&action_button);

    toolbar_view.add_top_bar(&header);

    // Content box
    let content = gtk4::Box::new(gtk4::Orientation::Vertical, 18);
    content.set_margin_start(24);
    content.set_margin_end(24);
    content.set_margin_top(24);
    content.set_margin_bottom(24);

    // Icon
    let icon = gtk4::Image::from_icon_name(match mode {
        MasterPasswordMode::Setup => "dialog-password-symbolic",
        MasterPasswordMode::Verify => "system-lock-screen-symbolic",
        MasterPasswordMode::Change => "preferences-system-privacy-symbolic",
        MasterPasswordMode::Reset => "dialog-warning-symbolic",
    });
    icon.set_pixel_size(64);
    icon.set_margin_bottom(12);
    content.append(&icon);

    // Title
    let title_label = gtk4::Label::builder()
        .label(match mode {
            MasterPasswordMode::Setup => "Welcome to EasySSH",
            MasterPasswordMode::Verify => "Enter Master Password",
            MasterPasswordMode::Change => "Change Master Password",
            MasterPasswordMode::Reset => "Reset Master Password",
        })
        .css_classes(vec!["title-2"])
        .build();
    content.append(&title_label);

    // Description
    let desc_label = gtk4::Label::builder()
        .label(match mode {
            MasterPasswordMode::Setup => {
                "Create a strong master password to secure your SSH configurations."
            }
            MasterPasswordMode::Verify => {
                "Enter your master password to access your encrypted server configurations."
            }
            MasterPasswordMode::Change => {
                "Change your master password. All encrypted data will be re-encrypted."
            }
            MasterPasswordMode::Reset => {
                "WARNING: This will permanently delete all encrypted data!"
            }
        })
        .wrap(true)
        .wrap_mode(gtk4::pango::WrapMode::Word)
        .build();
    content.append(&desc_label);

    // Show content based on mode
    match mode {
        MasterPasswordMode::Setup => {
            setup_setup_ui(&content, &action_button);
        }
        MasterPasswordMode::Verify => {
            setup_verify_ui(&content, &action_button, failed_attempts);
        }
        MasterPasswordMode::Change => {
            setup_change_ui(&content, &action_button);
        }
        MasterPasswordMode::Reset => {
            setup_reset_ui(&content, &action_button);
        }
    }

    toolbar_view.set_content(Some(&content));
    dialog.set_child(Some(&toolbar_view));

    // Cancel button handler
    let dialog_weak = dialog.downgrade();
    cancel_button.connect_clicked(move |_| {
        if let Some(dialog) = dialog_weak.upgrade() {
            dialog.close();
        }
    });

    // Action button handler
    let dialog_weak = dialog.downgrade();
    action_button.connect_clicked(glib::clone!(@weak content => move |_| {
        let result = match mode {
            MasterPasswordMode::Setup => handle_setup_action(&content),
            MasterPasswordMode::Verify => handle_verify_action(&content),
            MasterPasswordMode::Change => handle_change_action(&content),
            MasterPasswordMode::Reset => handle_reset_action(&content),
        };

        if let Some(result) = result {
            callback(result);
            if let Some(dialog) = dialog_weak.upgrade() {
                dialog.close();
            }
        }
    }));

    dialog.present(parent);
}

/// Setup UI for first-time password creation
fn setup_setup_ui(content: &gtk4::Box, action_button: &gtk4::Button) {
    let prefs_group = adw::PreferencesGroup::new();

    // Password entry
    let password_row = adw::PasswordEntryRow::builder()
        .title("Password")
        .show_apply_button(false)
        .build();
    prefs_group.add(&password_row);

    // Password strength indicator
    let strength_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    strength_box.set_margin_top(8);
    strength_box.set_margin_bottom(8);

    let strength_label = gtk4::Label::builder()
        .label("Strength: Very Weak")
        .css_classes(vec!["dim-label"])
        .build();
    strength_box.append(&strength_label);

    let strength_bar = gtk4::LevelBar::builder()
        .min_value(0.0)
        .max_value(100.0)
        .value(0.0)
        .build();
    strength_bar.add_offset_value("very-weak", 20.0);
    strength_bar.add_offset_value("weak", 40.0);
    strength_bar.add_offset_value("fair", 60.0);
    strength_bar.add_offset_value("good", 80.0);
    strength_bar.add_offset_value("strong", 100.0);
    strength_box.append(&strength_bar);

    prefs_group.add(&strength_box);

    // Confirm password entry
    let confirm_row = adw::PasswordEntryRow::builder()
        .title("Confirm Password")
        .show_apply_button(false)
        .build();
    prefs_group.add(&confirm_row);

    content.append(&prefs_group);

    // Requirements expander
    let expander = adw::ExpanderRow::builder()
        .title("Password Requirements")
        .build();
    expander.add_row(
        &gtk4::Label::builder()
            .label("• At least 8 characters")
            .halign(gtk4::Align::Start)
            .build(),
    );
    expander.add_row(
        &gtk4::Label::builder()
            .label("• Uppercase letters (A-Z)")
            .halign(gtk4::Align::Start)
            .build(),
    );
    expander.add_row(
        &gtk4::Label::builder()
            .label("• Lowercase letters (a-z)")
            .halign(gtk4::Align::Start)
            .build(),
    );
    expander.add_row(
        &gtk4::Label::builder()
            .label("• Numbers (0-9)")
            .halign(gtk4::Align::Start)
            .build(),
    );
    expander.add_row(
        &gtk4::Label::builder()
            .label("• Special characters (!@#$...)")
            .halign(gtk4::Align::Start)
            .build(),
    );

    content.append(&expander);

    // Error label
    let error_label = gtk4::Label::builder()
        .label("")
        .css_classes(vec!["error"])
        .wrap(true)
        .visible(false)
        .build();
    content.append(&error_label);

    // Update strength on password change
    password_row.connect_text_notify(
        glib::clone!(@weak password_row, @weak strength_bar, @weak strength_label => move |_| {
            let text = password_row.text();
            let (score, _) = calculate_password_strength(&text);
            let strength = PasswordStrength::from_score(score);

            strength_bar.set_value(score as f64);
            strength_label.set_label(&format!("Strength: {}", strength.as_str()));
        }),
    );

    // Store references for validation
    password_row.set_data("confirm_row", confirm_row.clone());
    password_row.set_data("error_label", error_label.clone());
    content.set_data("password_row", password_row.clone());
    content.set_data("confirm_row", confirm_row.clone());
    content.set_data("error_label", error_label.clone());
}

/// Setup UI for password verification
fn setup_verify_ui(content: &gtk4::Box, _action_button: &gtk4::Button, failed_attempts: u32) {
    let prefs_group = adw::PreferencesGroup::new();

    // Failed attempts warning
    if failed_attempts > 0 {
        let remaining = 5u32.saturating_sub(failed_attempts);
        let warning_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
        warning_box.set_margin_bottom(12);

        let warning_icon = gtk4::Image::from_icon_name("dialog-warning-symbolic");
        warning_box.append(&warning_icon);

        let warning_label = gtk4::Label::builder()
            .label(&format!(
                "Warning: {} failed attempts. {} attempts remaining.",
                failed_attempts, remaining
            ))
            .css_classes(vec!["warning"])
            .build();
        warning_box.append(&warning_label);

        content.append(&warning_box);
    }

    // Password entry
    let password_row = adw::PasswordEntryRow::builder()
        .title("Master Password")
        .show_apply_button(true)
        .build();
    prefs_group.add(&password_row);

    content.append(&prefs_group);

    // Error label
    let error_label = gtk4::Label::builder()
        .label("")
        .css_classes(vec!["error"])
        .wrap(true)
        .visible(false)
        .build();
    content.append(&error_label);

    // Forgot password link
    let forgot_button = gtk4::Button::builder()
        .label("Forgot password?")
        .css_classes(vec!["flat", "link"])
        .halign(gtk4::Align::Start)
        .build();
    content.append(&forgot_button);

    // Store references
    content.set_data("password_row", password_row.clone());
    content.set_data("error_label", error_label.clone());
    content.set_data("forgot_button", forgot_button.clone());
}

/// Setup UI for password change
fn setup_change_ui(content: &gtk4::Box, action_button: &gtk4::Button) {
    let prefs_group = adw::PreferencesGroup::new();

    // Current password
    let current_row = adw::PasswordEntryRow::builder()
        .title("Current Password")
        .show_apply_button(false)
        .build();
    prefs_group.add(&current_row);

    // New password
    let new_row = adw::PasswordEntryRow::builder()
        .title("New Password")
        .show_apply_button(false)
        .build();
    prefs_group.add(&new_row);

    // Password strength
    let strength_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    strength_box.set_margin_top(8);
    strength_box.set_margin_bottom(8);

    let strength_label = gtk4::Label::builder()
        .label("Strength: Very Weak")
        .css_classes(vec!["dim-label"])
        .build();
    strength_box.append(&strength_label);

    let strength_bar = gtk4::LevelBar::builder()
        .min_value(0.0)
        .max_value(100.0)
        .value(0.0)
        .build();
    strength_bar.add_offset_value("very-weak", 20.0);
    strength_bar.add_offset_value("weak", 40.0);
    strength_bar.add_offset_value("fair", 60.0);
    strength_bar.add_offset_value("good", 80.0);
    strength_bar.add_offset_value("strong", 100.0);
    strength_box.append(&strength_bar);

    prefs_group.add(&strength_box);

    // Confirm new password
    let confirm_row = adw::PasswordEntryRow::builder()
        .title("Confirm New Password")
        .show_apply_button(false)
        .build();
    prefs_group.add(&confirm_row);

    content.append(&prefs_group);

    // Error label
    let error_label = gtk4::Label::builder()
        .label("")
        .css_classes(vec!["error"])
        .wrap(true)
        .visible(false)
        .build();
    content.append(&error_label);

    // Update strength
    new_row.connect_text_notify(
        glib::clone!(@weak new_row, @weak strength_bar, @weak strength_label => move |_| {
            let text = new_row.text();
            let (score, _) = calculate_password_strength(&text);
            let strength = PasswordStrength::from_score(score);

            strength_bar.set_value(score as f64);
            strength_label.set_label(&format!("Strength: {}", strength.as_str()));
        }),
    );

    // Store references
    content.set_data("current_row", current_row.clone());
    content.set_data("new_row", new_row.clone());
    content.set_data("confirm_row", confirm_row.clone());
    content.set_data("error_label", error_label.clone());
}

/// Setup UI for password reset
fn setup_reset_ui(content: &gtk4::Box, _action_button: &gtk4::Button) {
    let warning_group = adw::PreferencesGroup::new();

    // Warning items
    let warnings = vec![
        "All stored SSH passwords will be deleted",
        "All encrypted server configurations will be lost",
        "All secure vault items will be removed",
        "Your encrypted keychain data will be cleared",
        "You will need to re-add all servers manually",
    ];

    for warning in warnings {
        let row = adw::ActionRow::builder().title(warning).build();
        row.add_prefix(&gtk4::Image::from_icon_name("dialog-warning-symbolic"));
        warning_group.add(&row);
    }

    content.append(&warning_group);

    // Confirmation entry
    let confirm_group = adw::PreferencesGroup::new();
    confirm_group.set_margin_top(24);

    let confirm_label = gtk4::Label::builder()
        .label("Type \"DELETE\" to confirm you understand the consequences:")
        .wrap(true)
        .halign(gtk4::Align::Start)
        .margin_bottom(8)
        .build();
    confirm_group.add(&confirm_label);

    let confirm_entry = adw::EntryRow::builder().title("Confirmation").build();
    confirm_group.add(&confirm_entry);

    content.append(&confirm_group);

    // Error label
    let error_label = gtk4::Label::builder()
        .label("")
        .css_classes(vec!["error"])
        .wrap(true)
        .visible(false)
        .margin_top(12)
        .build();
    content.append(&error_label);

    // Store references
    content.set_data("confirm_entry", confirm_entry.clone());
    content.set_data("error_label", error_label.clone());
}

/// Handle setup action
fn handle_setup_action(content: &gtk4::Box) -> Option<MasterPasswordResult> {
    let password_row: adw::PasswordEntryRow = content.data("password_row")?;
    let confirm_row: adw::PasswordEntryRow = content.data("confirm_row")?;
    let error_label: gtk4::Label = content.data("error_label")?;

    let password = password_row.text();
    let confirm = confirm_row.text();

    // Validate
    if password.len() < 8 {
        error_label.set_label("Password must be at least 8 characters long");
        error_label.set_visible(true);
        return None;
    }

    let has_lower = password.chars().any(|c| c.is_ascii_lowercase());
    let has_upper = password.chars().any(|c| c.is_ascii_uppercase());
    let has_digit = password.chars().any(|c| c.is_ascii_digit());
    let has_special = password.chars().any(|c| !c.is_alphanumeric());

    if !has_lower || !has_upper || !has_digit || !has_special {
        error_label.set_label(
            "Password must contain uppercase, lowercase, numbers, and special characters",
        );
        error_label.set_visible(true);
        return None;
    }

    if password != confirm {
        error_label.set_label("Passwords do not match");
        error_label.set_visible(true);
        return None;
    }

    let (score, _) = calculate_password_strength(&password);
    if score < 60 {
        error_label.set_label("Password is too weak. Please use a stronger password.");
        error_label.set_visible(true);
        return None;
    }

    Some(MasterPasswordResult::SetPassword {
        password: password.to_string(),
    })
}

/// Handle verify action
fn handle_verify_action(content: &gtk4::Box) -> Option<MasterPasswordResult> {
    let password_row: adw::PasswordEntryRow = content.data("password_row")?;
    let error_label: gtk4::Label = content.data("error_label")?;

    let password = password_row.text();

    if password.is_empty() {
        error_label.set_label("Please enter your master password");
        error_label.set_visible(true);
        return None;
    }

    Some(MasterPasswordResult::Verify {
        password: password.to_string(),
    })
}

/// Handle change action
fn handle_change_action(content: &gtk4::Box) -> Option<MasterPasswordResult> {
    let current_row: adw::PasswordEntryRow = content.data("current_row")?;
    let new_row: adw::PasswordEntryRow = content.data("new_row")?;
    let confirm_row: adw::PasswordEntryRow = content.data("confirm_row")?;
    let error_label: gtk4::Label = content.data("error_label")?;

    let current = current_row.text();
    let new_pass = new_row.text();
    let confirm = confirm_row.text();

    if current.is_empty() {
        error_label.set_label("Please enter your current password");
        error_label.set_visible(true);
        return None;
    }

    if new_pass.len() < 8 {
        error_label.set_label("New password must be at least 8 characters long");
        error_label.set_visible(true);
        return None;
    }

    let has_lower = new_pass.chars().any(|c| c.is_ascii_lowercase());
    let has_upper = new_pass.chars().any(|c| c.is_ascii_uppercase());
    let has_digit = new_pass.chars().any(|c| c.is_ascii_digit());
    let has_special = new_pass.chars().any(|c| !c.is_alphanumeric());

    if !has_lower || !has_upper || !has_digit || !has_special {
        error_label.set_label(
            "New password must contain uppercase, lowercase, numbers, and special characters",
        );
        error_label.set_visible(true);
        return None;
    }

    if new_pass != confirm {
        error_label.set_label("New passwords do not match");
        error_label.set_visible(true);
        return None;
    }

    if current == new_pass {
        error_label.set_label("New password must be different from current password");
        error_label.set_visible(true);
        return None;
    }

    let (score, _) = calculate_password_strength(&new_pass);
    if score < 60 {
        error_label.set_label("New password is too weak");
        error_label.set_visible(true);
        return None;
    }

    Some(MasterPasswordResult::ChangePassword {
        old_password: current.to_string(),
        new_password: new_pass.to_string(),
    })
}

/// Handle reset action
fn handle_reset_action(content: &gtk4::Box) -> Option<MasterPasswordResult> {
    let confirm_entry: adw::EntryRow = content.data("confirm_entry")?;
    let error_label: gtk4::Label = content.data("error_label")?;

    let confirmation = confirm_entry.text();

    if confirmation != "DELETE" {
        error_label.set_label("Please type DELETE to confirm");
        error_label.set_visible(true);
        return None;
    }

    Some(MasterPasswordResult::ResetConfirmed)
}
