//! Team Collaboration UI Panel for EasySSH Pro
//!
//! Provides team management, member invitation, and resource sharing UI.

use crate::design::DesignTheme;
use chrono::{DateTime, Utc};
use egui::{Color32, RichText, Ui};

/// Team role in the organization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TeamRole {
    #[default]
    Owner,
    Admin,
    Member,
    Viewer,
}

impl TeamRole {
    pub fn display_name(&self) -> &'static str {
        match self {
            TeamRole::Owner => "所有者",
            TeamRole::Admin => "管理员",
            TeamRole::Member => "成员",
            TeamRole::Viewer => "观察者",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            TeamRole::Owner => "👑",
            TeamRole::Admin => "🛡️",
            TeamRole::Member => "👤",
            TeamRole::Viewer => "👁️",
        }
    }

    pub fn color(&self) -> Color32 {
        match self {
            TeamRole::Owner => Color32::from_rgb(255, 215, 0), // Gold
            TeamRole::Admin => Color32::from_rgb(64, 156, 255), // Blue
            TeamRole::Member => Color32::from_rgb(100, 200, 100), // Green
            TeamRole::Viewer => Color32::from_rgb(150, 150, 150), // Gray
        }
    }
}

/// Team member information
#[derive(Debug, Clone)]
pub struct TeamMember {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub email: String,
    pub role: TeamRole,
    pub avatar: Option<String>,
    pub joined_at: DateTime<Utc>,
    pub last_active: Option<DateTime<Utc>>,
    pub is_online: bool,
}

/// Team information
#[derive(Debug, Clone)]
pub struct Team {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub member_count: usize,
    pub server_count: usize,
    pub is_active: bool,
    pub settings: TeamSettings,
}

/// Team settings
#[derive(Debug, Clone, Default)]
pub struct TeamSettings {
    pub allow_member_invite: bool,
    pub allow_member_share: bool,
    pub require_approval_for_join: bool,
    pub default_role: TeamRole,
}

/// Team invitation
#[derive(Debug, Clone)]
pub struct TeamInvitation {
    pub id: String,
    pub email: String,
    pub role: TeamRole,
    pub invited_by: String,
    pub invited_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub status: InvitationStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvitationStatus {
    Pending,
    Accepted,
    Expired,
    Revoked,
}

/// Team manager UI state
#[derive(Default)]
pub struct TeamManagerUI {
    pub teams: Vec<Team>,
    pub current_team: Option<String>,
    pub members: Vec<TeamMember>,
    pub invitations: Vec<TeamInvitation>,
    pub show_create_dialog: bool,
    pub show_invite_dialog: bool,
    pub show_settings_dialog: bool,
    pub selected_member: Option<String>,
    pub search_query: String,
    pub active_tab: TeamTab,
    pub new_team_form: NewTeamForm,
    pub invite_form: InviteForm,
    pub is_loading: bool,
    pub error_message: Option<String>,
    pub success_message: Option<(String, std::time::Instant)>,
}

#[derive(Default)]
pub struct NewTeamForm {
    pub name: String,
    pub description: String,
    pub allow_member_invite: bool,
    pub require_approval: bool,
}

#[derive(Default)]
pub struct InviteForm {
    pub email: String,
    pub role: TeamRole,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TeamTab {
    #[default]
    Overview,
    Members,
    Invitations,
    Servers,
    Activity,
    Settings,
}

impl TeamTab {
    pub fn display_name(&self) -> &'static str {
        match self {
            TeamTab::Overview => "概览",
            TeamTab::Members => "成员",
            TeamTab::Invitations => "邀请",
            TeamTab::Servers => "服务器",
            TeamTab::Activity => "活动",
            TeamTab::Settings => "设置",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            TeamTab::Overview => "📊",
            TeamTab::Members => "👥",
            TeamTab::Invitations => "📧",
            TeamTab::Servers => "🖥️",
            TeamTab::Activity => "📋",
            TeamTab::Settings => "⚙️",
        }
    }
}

impl TeamManagerUI {
    pub fn new() -> Self {
        let mut manager = Self::default();
        manager.load_mock_data(); // TODO: Load from API
        manager
    }

    /// Load mock data for demonstration
    fn load_mock_data(&mut self) {
        // Mock team
        let team = Team {
            id: "team-001".to_string(),
            name: "开发团队".to_string(),
            description: Some("后端服务器管理团队".to_string()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            member_count: 5,
            server_count: 12,
            is_active: true,
            settings: TeamSettings {
                allow_member_invite: true,
                allow_member_share: true,
                require_approval_for_join: false,
                default_role: TeamRole::Member,
            },
        };
        self.teams.push(team);
        self.current_team = Some("team-001".to_string());

        // Mock members
        self.members = vec![
            TeamMember {
                id: "user-001".to_string(),
                user_id: "user-001".to_string(),
                name: "张三".to_string(),
                email: "zhangsan@example.com".to_string(),
                role: TeamRole::Owner,
                avatar: None,
                joined_at: Utc::now(),
                last_active: Some(Utc::now()),
                is_online: true,
            },
            TeamMember {
                id: "user-002".to_string(),
                user_id: "user-002".to_string(),
                name: "李四".to_string(),
                email: "lisi@example.com".to_string(),
                role: TeamRole::Admin,
                avatar: None,
                joined_at: Utc::now(),
                last_active: Some(Utc::now()),
                is_online: true,
            },
            TeamMember {
                id: "user-003".to_string(),
                user_id: "user-003".to_string(),
                name: "王五".to_string(),
                email: "wangwu@example.com".to_string(),
                role: TeamRole::Member,
                avatar: None,
                joined_at: Utc::now(),
                last_active: None,
                is_online: false,
            },
        ];
    }

    /// Render the team panel
    pub fn render(&mut self, ctx: &egui::Context, show_panel: &mut bool) {
        if !*show_panel {
            return;
        }

        let theme = DesignTheme::dark();

        egui::SidePanel::left("team_panel")
            .width_range(350.0..=500.0)
            .default_width(400.0)
            .frame(egui::Frame {
                fill: theme.bg_secondary,
                stroke: egui::Stroke::new(1.0, theme.border_default),
                ..Default::default()
            })
            .show(ctx, |ui| {
                self.render_header(ui, &theme);
                ui.separator();
                self.render_tabs(ui);
                ui.separator();

                match self.active_tab {
                    TeamTab::Overview => self.render_overview(ui, &theme),
                    TeamTab::Members => self.render_members(ui, &theme),
                    TeamTab::Invitations => self.render_invitations(ui, &theme),
                    TeamTab::Servers => self.render_servers(ui, &theme),
                    TeamTab::Activity => self.render_activity(ui, &theme),
                    TeamTab::Settings => self.render_settings(ui, &theme),
                }
            });

        // Render dialogs
        if self.show_create_dialog {
            self.render_create_team_dialog(ctx);
        }
        if self.show_invite_dialog {
            self.render_invite_dialog(ctx);
        }
        if self.show_settings_dialog {
            self.render_team_settings_dialog(ctx);
        }
    }

    fn render_header(&mut self, ui: &mut Ui, theme: &DesignTheme) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("👥").size(20.0));
            ui.heading(
                RichText::new("团队协作")
                    .color(theme.text_primary)
                    .size(18.0),
            );

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("+ 创建团队").clicked() {
                    self.show_create_dialog = true;
                    self.new_team_form = NewTeamForm::default();
                }
            });
        });

        // Team selector
        if !self.teams.is_empty() {
            ui.horizontal(|ui| {
                ui.label("当前团队:");
                egui::ComboBox::from_id_source("team_selector")
                    .selected_text(
                        self.current_team
                            .as_ref()
                            .and_then(|id| self.teams.iter().find(|t| &t.id == id))
                            .map(|t| t.name.as_str())
                            .unwrap_or("选择团队"),
                    )
                    .show_ui(ui, |ui| {
                        for team in &self.teams {
                            ui.selectable_value(
                                &mut self.current_team,
                                Some(team.id.clone()),
                                &team.name,
                            );
                        }
                    });
            });
        }
    }

    fn render_tabs(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            let tabs = [
                TeamTab::Overview,
                TeamTab::Members,
                TeamTab::Invitations,
                TeamTab::Servers,
                TeamTab::Activity,
                TeamTab::Settings,
            ];

            for tab in tabs {
                let is_active = self.active_tab == tab;
                let text = format!("{} {}", tab.icon(), tab.display_name());

                let btn = egui::Button::new(RichText::new(text).size(12.0)).fill(if is_active {
                    egui::Color32::from_rgb(64, 156, 255)
                } else {
                    egui::Color32::TRANSPARENT
                });

                if ui.add(btn).clicked() {
                    self.active_tab = tab;
                }
            }
        });
    }

    fn render_overview(&mut self, ui: &mut Ui, theme: &DesignTheme) {
        // Clone team data to avoid borrow issues in closures
        let team_opt = self
            .current_team
            .as_ref()
            .and_then(|id| self.teams.iter().find(|t| &t.id == id))
            .cloned();

        if let Some(team) = team_opt {
            ui.vertical(|ui| {
                ui.heading(RichText::new(&team.name).color(theme.text_primary));
                if let Some(desc) = &team.description {
                    ui.label(RichText::new(desc).color(theme.text_secondary).size(12.0));
                }

                ui.add_space(16.0);

                // Stats cards
                ui.horizontal(|ui| {
                    self.render_stat_card(ui, "👥", "成员", &team.member_count.to_string());
                    self.render_stat_card(ui, "🖥️", "服务器", &team.server_count.to_string());
                    self.render_stat_card(ui, "📁", "共享资源", "8");
                });

                ui.add_space(16.0);

                // Quick actions
                ui.heading("快速操作");
                ui.horizontal(|ui| {
                    if ui.button("📧 邀请成员").clicked() {
                        self.show_invite_dialog = true;
                    }
                    if ui.button("⚙️ 团队设置").clicked() {
                        self.show_settings_dialog = true;
                    }
                });

                ui.add_space(16.0);

                // Recent activity
                ui.heading("最近活动");
                self.render_activity_item(ui, "👤", "李四", "添加了服务器 prod-db-01", "2分钟前");
                self.render_activity_item(
                    ui,
                    "📁",
                    "王五",
                    "分享了代码片段 'Docker部署'",
                    "1小时前",
                );
                self.render_activity_item(ui, "🔐", "张三", "修改了团队设置", "3小时前");
            });
        } else {
            ui.centered_and_justified(|ui| {
                ui.label("请选择一个团队或创建新团队");
            });
        }
    }

    fn render_stat_card(&self, ui: &mut Ui, icon: &str, label: &str, value: &str) {
        ui.group(|ui| {
            ui.set_min_width(80.0);
            ui.vertical_centered(|ui| {
                ui.label(RichText::new(icon).size(24.0));
                ui.label(RichText::new(value).size(20.0).strong());
                ui.label(RichText::new(label).size(11.0));
            });
        });
    }

    fn render_activity_item(&self, ui: &mut Ui, icon: &str, user: &str, action: &str, time: &str) {
        ui.horizontal(|ui| {
            ui.label(icon);
            ui.label(RichText::new(user).strong());
            ui.label(action);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(RichText::new(time).size(10.0).color(egui::Color32::GRAY));
            });
        });
        ui.separator();
    }

    fn render_members(&mut self, ui: &mut Ui, theme: &DesignTheme) {
        // Search and filter
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.search_query)
                    .hint_text("🔍 搜索成员...")
                    .desired_width(200.0),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("📧 邀请成员").clicked() {
                    self.show_invite_dialog = true;
                }
            });
        });

        ui.add_space(8.0);

        // Members list
        let members_clone = self.members.clone();
        egui::ScrollArea::vertical().show(ui, |ui| {
            for member in &members_clone {
                Self::render_member_item(ui, member, theme);
            }
        });
    }

    fn render_member_item(ui: &mut Ui, member: &TeamMember, theme: &DesignTheme) {
        ui.horizontal(|ui| {
            // Avatar placeholder
            ui.label(RichText::new(member.role.icon()).size(20.0));

            ui.vertical(|ui| {
                ui.label(
                    RichText::new(&member.name)
                        .strong()
                        .color(theme.text_primary),
                );
                ui.label(
                    RichText::new(&member.email)
                        .size(11.0)
                        .color(theme.text_secondary),
                );
            });

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Role badge
                let role_text = format!(" {} ", member.role.display_name());
                ui.label(
                    RichText::new(role_text)
                        .size(10.0)
                        .background_color(member.role.color())
                        .color(egui::Color32::WHITE),
                );

                // Online status
                let status = if member.is_online { "🟢" } else { "⚪" };
                ui.label(status);
            });
        });

        ui.separator();
    }

    fn render_invitations(&mut self, ui: &mut Ui, _theme: &DesignTheme) {
        ui.horizontal(|ui| {
            ui.heading("待处理邀请");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("📧 发送邀请").clicked() {
                    self.show_invite_dialog = true;
                }
            });
        });

        ui.add_space(8.0);

        if self.invitations.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label("暂无待处理邀请");
            });
        } else {
            let invitations_clone = self.invitations.clone();
            egui::ScrollArea::vertical().show(ui, |ui| {
                for invitation in &invitations_clone {
                    Self::render_invitation_item(ui, invitation);
                }
            });
        }
    }

    fn render_invitation_item(ui: &mut Ui, invitation: &TeamInvitation) {
        ui.horizontal(|ui| {
            ui.label("📧");
            ui.vertical(|ui| {
                ui.label(&invitation.email);
                ui.label(
                    RichText::new(format!("角色: {}", invitation.role.display_name())).size(11.0),
                );
            });

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let status_text = match invitation.status {
                    InvitationStatus::Pending => "⏳ 待接受",
                    InvitationStatus::Accepted => "✅ 已接受",
                    InvitationStatus::Expired => "❌ 已过期",
                    InvitationStatus::Revoked => "🚫 已撤销",
                };
                ui.label(status_text);
            });
        });
        ui.separator();
    }

    fn render_servers(&mut self, ui: &mut Ui, _theme: &DesignTheme) {
        ui.heading("共享服务器");
        ui.label("团队共享的服务器列表 (Pro功能)");
        ui.add_space(8.0);

        // Placeholder for server list
        ui.group(|ui| {
            ui.set_min_height(200.0);
            ui.centered_and_justified(|ui| {
                ui.label("🖥️\n团队服务器列表\n(连接到Pro服务器后显示)");
            });
        });
    }

    fn render_activity(&mut self, ui: &mut Ui, _theme: &DesignTheme) {
        ui.heading("团队活动日志");
        ui.label("最近的操作记录 (Pro功能)");
        ui.add_space(8.0);

        // Activity items
        self.render_activity_item(ui, "🔐", "张三", "登录系统", "刚刚");
        self.render_activity_item(ui, "🖥️", "李四", "连接服务器 prod-web-01", "5分钟前");
        self.render_activity_item(ui, "📁", "王五", "上传文件到 /var/log", "15分钟前");
        self.render_activity_item(ui, "📧", "张三", "邀请赵六加入团队", "1小时前");
        self.render_activity_item(ui, "⚙️", "李四", "修改服务器配置", "2小时前");
    }

    fn render_settings(&mut self, ui: &mut Ui, _theme: &DesignTheme) {
        ui.heading("团队设置");

        if let Some(team) = self
            .current_team
            .as_ref()
            .and_then(|id| self.teams.iter_mut().find(|t| &t.id == id))
        {
            ui.add_space(8.0);

            ui.checkbox(&mut team.settings.allow_member_invite, "允许成员邀请新成员");
            ui.checkbox(&mut team.settings.allow_member_share, "允许成员分享资源");
            ui.checkbox(
                &mut team.settings.require_approval_for_join,
                "新成员加入需要审批",
            );

            ui.add_space(16.0);

            ui.horizontal(|ui| {
                if ui.button("💾 保存设置").clicked() {
                    self.show_success("设置已保存");
                }
                if ui.button("🗑️ 删除团队").clicked() {
                    // TODO: Show confirmation dialog
                }
            });
        }
    }

    fn render_create_team_dialog(&mut self, ctx: &egui::Context) {
        let mut open = true;
        egui::Window::new("创建新团队")
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .show(ctx, |ui| {
                ui.label("团队名称:");
                ui.text_edit_singleline(&mut self.new_team_form.name);

                ui.label("团队描述:");
                ui.text_edit_multiline(&mut self.new_team_form.description);

                ui.checkbox(&mut self.new_team_form.allow_member_invite, "允许成员邀请");
                ui.checkbox(&mut self.new_team_form.require_approval, "加入需要审批");

                ui.add_space(16.0);

                ui.horizontal(|ui| {
                    if ui.button("✅ 创建").clicked() {
                        self.create_team();
                        self.show_create_dialog = false;
                    }
                    if ui.button("❌ 取消").clicked() {
                        self.show_create_dialog = false;
                    }
                });
            });

        if !open {
            self.show_create_dialog = false;
        }
    }

    fn render_invite_dialog(&mut self, ctx: &egui::Context) {
        let mut open = true;
        egui::Window::new("邀请成员")
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .show(ctx, |ui| {
                ui.label("邮箱地址:");
                ui.text_edit_singleline(&mut self.invite_form.email);

                ui.label("角色:");
                egui::ComboBox::from_id_source("invite_role")
                    .selected_text(self.invite_form.role.display_name())
                    .show_ui(ui, |ui| {
                        for role in [TeamRole::Admin, TeamRole::Member, TeamRole::Viewer] {
                            ui.selectable_value(
                                &mut self.invite_form.role,
                                role,
                                role.display_name(),
                            );
                        }
                    });

                ui.label("邀请消息 (可选):");
                ui.text_edit_multiline(&mut self.invite_form.message);

                ui.add_space(16.0);

                ui.horizontal(|ui| {
                    if ui.button("📧 发送邀请").clicked() {
                        self.send_invitation();
                        self.show_invite_dialog = false;
                    }
                    if ui.button("❌ 取消").clicked() {
                        self.show_invite_dialog = false;
                    }
                });
            });

        if !open {
            self.show_invite_dialog = false;
        }
    }

    fn render_team_settings_dialog(&mut self, ctx: &egui::Context) {
        let mut open = true;
        egui::Window::new("团队设置")
            .collapsible(false)
            .resizable(true)
            .default_size([400.0, 300.0])
            .open(&mut open)
            .show(ctx, |ui| {
                self.render_settings(ui, &DesignTheme::dark());
            });

        if !open {
            self.show_settings_dialog = false;
        }
    }

    fn create_team(&mut self) {
        let team = Team {
            id: format!("team-{}", uuid::Uuid::new_v4()),
            name: self.new_team_form.name.clone(),
            description: if self.new_team_form.description.is_empty() {
                None
            } else {
                Some(self.new_team_form.description.clone())
            },
            created_at: Utc::now(),
            updated_at: Utc::now(),
            member_count: 1,
            server_count: 0,
            is_active: true,
            settings: TeamSettings {
                allow_member_invite: self.new_team_form.allow_member_invite,
                allow_member_share: true,
                require_approval_for_join: self.new_team_form.require_approval,
                default_role: TeamRole::Member,
            },
        };

        self.teams.push(team);
        self.show_success("团队创建成功");
    }

    fn send_invitation(&mut self) {
        let invitation = TeamInvitation {
            id: format!("inv-{}", uuid::Uuid::new_v4()),
            email: self.invite_form.email.clone(),
            role: self.invite_form.role,
            invited_by: "current_user".to_string(),
            invited_at: Utc::now(),
            expires_at: Utc::now() + chrono::Duration::days(7),
            status: InvitationStatus::Pending,
        };

        self.invitations.push(invitation);
        self.invite_form = InviteForm::default();
        self.show_success("邀请已发送");
    }

    fn show_success(&mut self, message: &str) {
        self.success_message = Some((message.to_string(), std::time::Instant::now()));
    }

    /// Update and clear expired success messages
    pub fn update(&mut self) {
        if let Some((_, time)) = &self.success_message {
            if time.elapsed().as_secs() > 3 {
                self.success_message = None;
            }
        }
    }
}

/// Render team panel helper function
pub fn render_team_panel(ctx: &egui::Context, show_panel: &mut bool, manager: &mut TeamManagerUI) {
    manager.update();
    manager.render(ctx, show_panel);

    // Show success message as notification
    if let Some((message, _)) = &manager.success_message {
        egui::TopBottomPanel::top("team_notification")
            .exact_height(40.0)
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.label(
                        RichText::new(format!("✅ {}", message))
                            .color(Color32::from_rgb(100, 200, 100))
                            .strong(),
                    );
                });
            });
    }
}
