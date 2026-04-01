#![allow(dead_code)]

use eframe::egui;
use egui::{Color32, Frame, RichText, Ui, Window};

use easyssh_core::sso::*;

/// SSO Login UI Panel for SAML/OIDC authentication
pub struct SsoLoginPanel {
    /// SSO manager instance
    sso_manager: SsoManager,
    /// Currently selected provider
    selected_provider: Option<String>,
    /// Show provider configuration dialog
    show_provider_config: bool,
    /// Show login dialog
    show_login_dialog: bool,
    /// New provider form data
    new_provider_name: String,
    new_provider_type: SsoProviderType,
    /// SAML configuration form
    saml_config: SamlConfigForm,
    /// OIDC configuration form
    oidc_config: OidcConfigForm,
    /// Authentication status
    auth_status: SsoAuthStatus,
    /// Current user info after successful login
    current_user: Option<SsoUserInfo>,
    /// Current session
    current_session: Option<SsoSession>,
    /// Error message
    error_message: Option<String>,
    /// Show user profile panel
    show_profile: bool,
    /// Pending auth request ID
    pending_auth_id: Option<String>,
}

#[derive(Clone, Debug, Default)]
struct SamlConfigForm {
    idp_entity_id: String,
    idp_sso_url: String,
    idp_slo_url: String,
    idp_certificate: String,
    sp_entity_id: String,
    sp_acs_url: String,
    name_id_format: NameIdFormat,
    signature_algorithm: SignatureAlgorithm,
}

#[derive(Clone, Debug, Default)]
struct OidcConfigForm {
    issuer_url: String,
    client_id: String,
    client_secret: String,
    redirect_uri: String,
    scopes: String,
    use_pkce: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SsoAuthStatus {
    Idle,
    Authenticating,
    Authenticated,
    Failed,
}

impl Default for SsoAuthStatus {
    fn default() -> Self {
        SsoAuthStatus::Idle
    }
}

/// Response from SSO login panel
#[derive(Debug, Default)]
pub struct SsoLoginResponse {
    pub login_initiated: bool,
    pub logout_requested: bool,
    pub provider_added: Option<SsoProvider>,
    pub provider_removed: Option<String>,
    pub session_updated: Option<SsoSession>,
    pub user_info_updated: Option<SsoUserInfo>,
}

impl SsoLoginPanel {
    pub fn new() -> Self {
        let mut panel = Self {
            sso_manager: SsoManager::new(),
            selected_provider: None,
            show_provider_config: false,
            show_login_dialog: false,
            new_provider_name: String::new(),
            new_provider_type: SsoProviderType::Oidc,
            saml_config: SamlConfigForm::default(),
            oidc_config: OidcConfigForm::default(),
            auth_status: SsoAuthStatus::Idle,
            current_user: None,
            current_session: None,
            error_message: None,
            show_profile: false,
            pending_auth_id: None,
        };

        // Initialize with some demo providers for testing
        panel.add_demo_providers();
        panel
    }

    fn add_demo_providers(&mut self) {
        // Add a demo OIDC provider (like Okta/Auth0 style)
        let oidc_config = OidcConfig::standard(
            "https://demo.auth.example.com",
            "easyssh_client",
            "",
            "http://localhost:8765/sso/callback",
        );
        let provider = SsoProvider::new_oidc("Demo OIDC Provider", oidc_config);
        let _ = self.sso_manager.add_provider(provider);

        // Add a demo SAML provider
        let saml_config = SamlConfig {
            idp_entity_id: "https://demo-saml.example.com".to_string(),
            idp_sso_url: "https://demo-saml.example.com/sso".to_string(),
            idp_slo_url: Some("https://demo-saml.example.com/slo".to_string()),
            idp_certificate: "-----BEGIN CERTIFICATE-----\nMIIDXTCCAkWgAwIBAgIJAKoK/heBjcOu...\n-----END CERTIFICATE-----".to_string(),
            sp_entity_id: "easyssh-pro-client".to_string(),
            sp_acs_url: "http://localhost:8765/sso/acs".to_string(),
            name_id_format: NameIdFormat::EmailAddress,
            signature_algorithm: SignatureAlgorithm::RsaSha256,
            require_signed_assertions: true,
            attribute_mapping: SamlAttributeMapping::default_mapping(),
        };
        let provider = SsoProvider::new_saml("Demo SAML Provider", saml_config);
        let _ = self.sso_manager.add_provider(provider);
    }

    pub fn ui(&mut self, ui: &mut Ui) -> SsoLoginResponse {
        let mut response = SsoLoginResponse::default();

        // Header
        ui.horizontal(|ui| {
            ui.heading("SSO Login");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if self.current_user.is_some() {
                    if ui.button("🚪 Logout").clicked() {
                        self.handle_logout();
                        response.logout_requested = true;
                    }
                    if ui.button("👤 Profile").clicked() {
                        self.show_profile = !self.show_profile;
                    }
                } else {
                    let btn_text = match self.auth_status {
                        SsoAuthStatus::Authenticating => "⏳ Authenticating...",
                        _ => "🔐 Add Provider",
                    };
                    if ui.button(btn_text).clicked() {
                        self.show_provider_config = true;
                        self.error_message = None;
                    }
                }
            });
        });

        ui.separator();

        // Error message display
        if let Some(ref error) = self.error_message {
            ui.horizontal(|ui| {
                ui.colored_label(Color32::RED, "⚠️");
                ui.colored_label(Color32::RED, error);
            });
            ui.add_space(8.0);
        }

        // User profile panel (when authenticated)
        if self.show_profile && self.current_user.is_some() {
            self.render_user_profile(ui);
        }

        // Authentication status
        match self.auth_status {
            SsoAuthStatus::Authenticated => {
                if let Some(ref user) = self.current_user {
                    ui.horizontal(|ui| {
                        ui.colored_label(Color32::GREEN, "✓");
                        ui.label(format!("Authenticated as: {}", user.email));
                    });
                    ui.add_space(8.0);
                }
            }
            SsoAuthStatus::Authenticating => {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label("Authenticating with SSO provider...");
                });

                // Simulate authentication progress
                if ui.button("Cancel").clicked() {
                    self.auth_status = SsoAuthStatus::Idle;
                    self.pending_auth_id = None;
                }
            }
            _ => {}
        }

        // Provider list
        self.render_provider_list(ui, &mut response);

        // Provider configuration dialog
        if self.show_provider_config {
            self.render_provider_config_dialog(ui.ctx(), &mut response);
        }

        // Login dialog
        if self.show_login_dialog {
            self.render_login_dialog(ui.ctx(), &mut response);
        }

        response
    }

    fn render_provider_list(&mut self, ui: &mut Ui, response: &mut SsoLoginResponse) {
        ui.label("Available SSO Providers:");
        ui.add_space(4.0);

        let providers: Vec<_> = self.sso_manager.list_enabled_providers()
            .into_iter()
            .map(|p| (p.id.clone(), p.name.clone(), p.provider_type))
            .collect();

        if providers.is_empty() {
            ui.label("No SSO providers configured.");
            ui.label("Click 'Add Provider' to configure SAML or OIDC authentication.");
        } else {
            Frame::group(ui.style())
                .fill(Color32::from_gray(40))
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());

                    for (provider_id, name, provider_type) in providers {
                        ui.horizontal(|ui| {
                            let icon = match provider_type {
                                SsoProviderType::Saml => "🔷",
                                SsoProviderType::Oidc => "🔶",
                                SsoProviderType::Ldap => "📁",
                            };

                            ui.label(format!("{} {}", icon, name));
                            ui.label(RichText::new(format!("({})", provider_type)).small());

                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.button("⚙️").on_hover_text("Configure").clicked() {
                                    self.selected_provider = Some(provider_id.clone());
                                }

                                if self.current_user.is_none() {
                                    let login_btn = egui::Button::new("Login")
                                        .fill(Color32::from_rgb(64, 156, 255));
                                    if ui.add(login_btn).clicked() {
                                        self.initiate_login(&provider_id);
                                        response.login_initiated = true;
                                    }
                                }
                            });
                        });
                        ui.separator();
                    }
                });
        }

        ui.add_space(16.0);

        // Session info (if authenticated)
        if let Some(ref session) = self.current_session {
            ui.collapsing("Session Details", |ui| {
                ui.label(format!("Session ID: {}", &session.id[..8]));
                ui.label(format!("User ID: {}", session.user_id));
                ui.label(format!("Provider: {}", session.provider_id));
                ui.label(format!("Created: {}", session.created_at.format("%Y-%m-%d %H:%M")));
                ui.label(format!("Expires: {}", session.expires_at.format("%Y-%m-%d %H:%M")));

                if session.is_expired() {
                    ui.colored_label(Color32::RED, "⚠️ Session expired");
                } else {
                    ui.colored_label(Color32::GREEN, "✓ Session active");
                }
            });
        }
    }

    fn render_user_profile(&mut self, ui: &mut Ui) {
        if let Some(ref user) = self.current_user {
            Frame::group(ui.style())
                .fill(Color32::from_rgb(40, 50, 60))
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    ui.heading("User Profile");
                    ui.add_space(8.0);

                    ui.label(format!("👤 User ID: {}", user.user_id));
                    ui.label(format!("📧 Email: {}", user.email));
                    ui.label(format!("🔑 Username: {}", user.username));

                    if let Some(ref first) = user.first_name {
                        if let Some(ref last) = user.last_name {
                            ui.label(format!("📝 Name: {} {}", first, last));
                        }
                    }

                    if !user.groups.is_empty() {
                        ui.add_space(4.0);
                        ui.label("👥 Groups:");
                        for group in &user.groups {
                            ui.label(format!("   • {}", group));
                        }
                    }

                    if !user.team_ids.is_empty() {
                        ui.add_space(4.0);
                        ui.label("🏢 Teams:");
                        for team_id in &user.team_ids {
                            ui.label(format!("   • {}", team_id));
                        }
                    }
                });

            ui.add_space(8.0);
        }
    }

    fn render_provider_config_dialog(&mut self, ctx: &egui::Context, _response: &mut SsoLoginResponse) {
        let id = egui::Id::new("sso_provider_config");
        Window::new("Configure SSO Provider")
            .id(id)
            .collapsible(false)
            .resizable(true)
            .default_size([500.0, 600.0])
            .show(ctx, |ui| {
                ui.label("Provider Name:");
                ui.text_edit_singleline(&mut self.new_provider_name);
                ui.add_space(8.0);

                ui.label("Provider Type:");
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.new_provider_type, SsoProviderType::Oidc, "🔶 OpenID Connect (OIDC)");
                    ui.selectable_value(&mut self.new_provider_type, SsoProviderType::Saml, "🔷 SAML 2.0");
                });
                ui.add_space(16.0);

                match self.new_provider_type {
                    SsoProviderType::Oidc => self.render_oidc_config_form(ui),
                    SsoProviderType::Saml => self.render_saml_config_form(ui),
                    _ => {}
                }

                ui.add_space(16.0);

                ui.horizontal(|ui| {
                    let mut saved_provider = None;
                    let mut saved_provider_id = None;

                    if ui.button("💾 Save Provider").clicked() && !self.new_provider_name.is_empty() {
                        if let Some(provider) = self.create_provider_from_form() {
                            let provider_id = provider.id.clone();
                            if let Err(e) = self.sso_manager.add_provider(provider.clone()) {
                                self.error_message = Some(format!("Failed to add provider: {}", e));
                            } else {
                                saved_provider = Some(provider);
                                saved_provider_id = Some(provider_id);
                                self.show_provider_config = false;
                                self.new_provider_name.clear();
                            }
                        }
                    }

                    if saved_provider.is_some() {
                        response.provider_added = saved_provider;
                    }
                    if saved_provider_id.is_some() {
                        self.selected_provider = saved_provider_id;
                    }

                    if ui.button("❌ Cancel").clicked() {
                        self.show_provider_config = false;
                        self.error_message = None;
                    }
                });
            });
    }

    fn render_oidc_config_form(&mut self, ui: &mut Ui) {
        ui.collapsing("OIDC Configuration", |ui| {
            ui.label("Issuer URL:");
            ui.text_edit_singleline(&mut self.oidc_config.issuer_url);
            ui.label("Example: https://company.okta.com/oauth2/default");
            ui.add_space(8.0);

            ui.label("Client ID:");
            ui.text_edit_singleline(&mut self.oidc_config.client_id);
            ui.add_space(8.0);

            ui.label("Client Secret:");
            ui.add(egui::TextEdit::singleline(&mut self.oidc_config.client_secret).password(true));
            ui.add_space(8.0);

            ui.label("Redirect URI:");
            ui.text_edit_singleline(&mut self.oidc_config.redirect_uri);
            ui.label("Example: http://localhost:8765/sso/callback");
            ui.add_space(8.0);

            ui.label("Scopes (space-separated):");
            if self.oidc_config.scopes.is_empty() {
                self.oidc_config.scopes = "openid profile email".to_string();
            }
            ui.text_edit_singleline(&mut self.oidc_config.scopes);
            ui.add_space(8.0);

            ui.checkbox(&mut self.oidc_config.use_pkce, "Use PKCE (recommended for security)");
        });
    }

    fn render_saml_config_form(&mut self, ui: &mut Ui) {
        ui.collapsing("SAML 2.0 Configuration", |ui| {
            ui.label("IdP Entity ID:");
            ui.text_edit_singleline(&mut self.saml_config.idp_entity_id);
            ui.label("Example: https://company.okta.com");
            ui.add_space(8.0);

            ui.label("IdP SSO URL:");
            ui.text_edit_singleline(&mut self.saml_config.idp_sso_url);
            ui.add_space(8.0);

            ui.label("IdP Certificate (X.509):");
            ui.add(egui::TextEdit::multiline(&mut self.saml_config.idp_certificate).desired_rows(5));
            ui.add_space(8.0);

            ui.label("SP Entity ID:");
            ui.text_edit_singleline(&mut self.saml_config.sp_entity_id);
            ui.label("Example: easyssh-pro-client");
            ui.add_space(8.0);

            ui.label("SP ACS URL:");
            ui.text_edit_singleline(&mut self.saml_config.sp_acs_url);
            ui.label("Example: http://localhost:8765/sso/acs");
            ui.add_space(8.0);

            ui.label("NameID Format:");
            egui::ComboBox::from_id_source("nameid_format")
                .selected_text(format!("{:?}", self.saml_config.name_id_format))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.saml_config.name_id_format, NameIdFormat::EmailAddress, "Email Address");
                    ui.selectable_value(&mut self.saml_config.name_id_format, NameIdFormat::Persistent, "Persistent");
                    ui.selectable_value(&mut self.saml_config.name_id_format, NameIdFormat::Transient, "Transient");
                });
        });
    }

    fn render_login_dialog(&mut self, ctx: &egui::Context, _response: &mut SsoLoginResponse) {
        let id = egui::Id::new("sso_login_dialog");
        Window::new("SSO Login")
            .id(id)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                if let Some(ref provider_id) = self.selected_provider {
                    if let Some(provider) = self.sso_manager.get_provider(provider_id) {
                        ui.heading(format!("Login with {}", provider.name));
                        ui.add_space(16.0);

                        ui.label("You will be redirected to your identity provider to authenticate.");
                        ui.label("After authentication, you'll return to EasySSH.");
                        ui.add_space(16.0);

                        let provider_id_clone = provider_id.clone();
                        ui.horizontal(|ui| {
                            if ui.button("🔐 Proceed to Login").clicked() {
                                self.show_login_dialog = false;
                                self.auth_status = SsoAuthStatus::Authenticating;
                                self.simulate_authentication(provider_id_clone);
                            }

                            if ui.button("❌ Cancel").clicked() {
                                self.show_login_dialog = false;
                            }
                        });
                    }
                }
            });
    }

    fn create_provider_from_form(&self) -> Option<SsoProvider> {
        if self.new_provider_name.is_empty() {
            return None;
        }

        let provider = match self.new_provider_type {
            SsoProviderType::Oidc => {
                let scopes: Vec<String> = self.oidc_config.scopes
                    .split_whitespace()
                    .map(|s| s.to_string())
                    .collect();

                let config = OidcConfig {
                    issuer_url: self.oidc_config.issuer_url.clone(),
                    authorization_endpoint: format!("{}/oauth2/v1/authorize", self.oidc_config.issuer_url),
                    token_endpoint: format!("{}/oauth2/v1/token", self.oidc_config.issuer_url),
                    userinfo_endpoint: format!("{}/oauth2/v1/userinfo", self.oidc_config.issuer_url),
                    jwks_uri: format!("{}/oauth2/v1/keys", self.oidc_config.issuer_url),
                    end_session_endpoint: Some(format!("{}/oauth2/v1/logout", self.oidc_config.issuer_url)),
                    client_id: self.oidc_config.client_id.clone(),
                    client_secret: self.oidc_config.client_secret.clone(),
                    redirect_uri: self.oidc_config.redirect_uri.clone(),
                    scopes,
                    response_type: "code".to_string(),
                    attribute_mapping: OidcAttributeMapping::default_mapping(),
                    use_pkce: self.oidc_config.use_pkce,
                };
                SsoProvider::new_oidc(&self.new_provider_name, config)
            }
            SsoProviderType::Saml => {
                let config = SamlConfig {
                    idp_entity_id: self.saml_config.idp_entity_id.clone(),
                    idp_sso_url: self.saml_config.idp_sso_url.clone(),
                    idp_slo_url: Some(self.saml_config.idp_slo_url.clone()).filter(|s| !s.is_empty()),
                    idp_certificate: self.saml_config.idp_certificate.clone(),
                    sp_entity_id: self.saml_config.sp_entity_id.clone(),
                    sp_acs_url: self.saml_config.sp_acs_url.clone(),
                    name_id_format: self.saml_config.name_id_format,
                    signature_algorithm: self.saml_config.signature_algorithm,
                    require_signed_assertions: true,
                    attribute_mapping: SamlAttributeMapping::default_mapping(),
                };
                SsoProvider::new_saml(&self.new_provider_name, config)
            }
            _ => return None,
        };

        Some(provider)
    }

    fn initiate_login(&mut self, provider_id: &str) {
        self.selected_provider = Some(provider_id.to_string());
        self.show_login_dialog = true;
        self.error_message = None;
    }

    fn simulate_authentication(&mut self, provider_id: String) {
        // In a real implementation, this would:
        // 1. Initiate the SSO flow with the provider
        // 2. Open a browser or embedded webview
        // 3. Handle the callback
        // 4. Exchange the code for tokens
        // 5. Validate the tokens

        // For this demo, we simulate successful authentication
        let user_info = SsoUserInfo {
            user_id: format!("user_{}", uuid::Uuid::new_v4().to_string()[..8].to_string()),
            email: "user@company.com".to_string(),
            username: "ssouser".to_string(),
            first_name: Some("Demo".to_string()),
            last_name: Some("User".to_string()),
            groups: vec!["Developers".to_string(), "DevOps".to_string()],
            team_ids: vec!["team1".to_string()],
            raw_attributes: std::collections::HashMap::new(),
        };

        let session = SsoSession::new(&user_info.user_id, &provider_id, 8);

        self.current_user = Some(user_info);
        self.current_session = Some(session);
        self.auth_status = SsoAuthStatus::Authenticated;
    }

    fn handle_logout(&mut self) {
        if let Some(ref session) = self.current_session {
            let _ = self.sso_manager.terminate_session(&session.id);
        }

        self.current_user = None;
        self.current_session = None;
        self.auth_status = SsoAuthStatus::Idle;
        self.show_profile = false;
        self.pending_auth_id = None;
    }

    pub fn is_authenticated(&self) -> bool {
        matches!(self.auth_status, SsoAuthStatus::Authenticated) && self.current_user.is_some()
    }

    pub fn get_current_user(&self) -> Option<&SsoUserInfo> {
        self.current_user.as_ref()
    }

    pub fn get_sso_manager(&self) -> &SsoManager {
        &self.sso_manager
    }

    pub fn get_sso_manager_mut(&mut self) -> &mut SsoManager {
        &mut self.sso_manager
    }
}

impl Default for SsoLoginPanel {
    fn default() -> Self {
        Self::new()
    }
}
