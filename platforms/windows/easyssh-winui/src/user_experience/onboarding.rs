//! First-time User Onboarding for EasySSH
//!
//! Provides a welcoming first-run experience with guided tutorials.

use std::time::Instant;
use egui::{RichText, Ui, Vec2, Frame, Stroke, Rounding, Margin, Align, Layout};

/// Onboarding state and progress tracking
#[derive(Default)]
pub struct OnboardingState {
    pub is_first_run: bool,
    pub current_step: OnboardingStep,
    pub completed_steps: Vec<OnboardingStep>,
    pub show_tutorial: bool,
    pub tutorial_completed: bool,
    pub preferences_set: bool,
    pub last_shown: Option<Instant>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OnboardingStep {
    Welcome,
    AddServer,
    ConnectGuide,
    TerminalGuide,
    FileBrowserGuide,
    SnippetsGuide,
    ShortcutsGuide,
    Complete,
}

impl Default for OnboardingStep {
    fn default() -> Self {
        OnboardingStep::Welcome
    }
}

impl OnboardingStep {
    pub fn display_name(&self) -> &'static str {
        match self {
            OnboardingStep::Welcome => "欢迎使用",
            OnboardingStep::AddServer => "添加服务器",
            OnboardingStep::ConnectGuide => "连接服务器",
            OnboardingStep::TerminalGuide => "使用终端",
            OnboardingStep::FileBrowserGuide => "文件管理",
            OnboardingStep::SnippetsGuide => "命令片段",
            OnboardingStep::ShortcutsGuide => "快捷键",
            OnboardingStep::Complete => "完成",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            OnboardingStep::Welcome => "让我们开始配置您的第一个SSH连接",
            OnboardingStep::AddServer => "学习如何添加和管理服务器配置",
            OnboardingStep::ConnectGuide => "连接到服务器并开始您的会话",
            OnboardingStep::TerminalGuide => "使用内置终端执行命令",
            OnboardingStep::FileBrowserGuide => "通过SFTP管理远程文件",
            OnboardingStep::SnippetsGuide => "保存和重用常用命令",
            OnboardingStep::ShortcutsGuide => "掌握快捷键提高效率",
            OnboardingStep::Complete => "您已准备就绪！",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            OnboardingStep::Welcome => "👋",
            OnboardingStep::AddServer => "➕",
            OnboardingStep::ConnectGuide => "🔌",
            OnboardingStep::TerminalGuide => "▸",
            OnboardingStep::FileBrowserGuide => "📁",
            OnboardingStep::SnippetsGuide => "⚡",
            OnboardingStep::ShortcutsGuide => "⌨",
            OnboardingStep::Complete => "✓",
        }
    }

    pub fn total_steps() -> usize {
        7 // Excluding Complete
    }

    pub fn step_number(&self) -> usize {
        match self {
            OnboardingStep::Welcome => 1,
            OnboardingStep::AddServer => 2,
            OnboardingStep::ConnectGuide => 3,
            OnboardingStep::TerminalGuide => 4,
            OnboardingStep::FileBrowserGuide => 5,
            OnboardingStep::SnippetsGuide => 6,
            OnboardingStep::ShortcutsGuide => 7,
            OnboardingStep::Complete => 8,
        }
    }

    pub fn next(&self) -> Option<OnboardingStep> {
        match self {
            OnboardingStep::Welcome => Some(OnboardingStep::AddServer),
            OnboardingStep::AddServer => Some(OnboardingStep::ConnectGuide),
            OnboardingStep::ConnectGuide => Some(OnboardingStep::TerminalGuide),
            OnboardingStep::TerminalGuide => Some(OnboardingStep::FileBrowserGuide),
            OnboardingStep::FileBrowserGuide => Some(OnboardingStep::SnippetsGuide),
            OnboardingStep::SnippetsGuide => Some(OnboardingStep::ShortcutsGuide),
            OnboardingStep::ShortcutsGuide => Some(OnboardingStep::Complete),
            OnboardingStep::Complete => None,
        }
    }

    pub fn previous(&self) -> Option<OnboardingStep> {
        match self {
            OnboardingStep::Welcome => None,
            OnboardingStep::AddServer => Some(OnboardingStep::Welcome),
            OnboardingStep::ConnectGuide => Some(OnboardingStep::AddServer),
            OnboardingStep::TerminalGuide => Some(OnboardingStep::ConnectGuide),
            OnboardingStep::FileBrowserGuide => Some(OnboardingStep::TerminalGuide),
            OnboardingStep::SnippetsGuide => Some(OnboardingStep::FileBrowserGuide),
            OnboardingStep::ShortcutsGuide => Some(OnboardingStep::SnippetsGuide),
            OnboardingStep::Complete => Some(OnboardingStep::ShortcutsGuide),
        }
    }
}

impl OnboardingState {
    pub fn new() -> Self {
        Self {
            is_first_run: true,
            current_step: OnboardingStep::Welcome,
            completed_steps: Vec::new(),
            show_tutorial: false,
            tutorial_completed: false,
            preferences_set: false,
            last_shown: None,
        }
    }

    /// Check if onboarding should be shown
    pub fn should_show(&self) -> bool {
        self.is_first_run && !self.tutorial_completed
    }

    /// Advance to next step
    pub fn next_step(&mut self) {
        if !self.completed_steps.contains(&self.current_step) {
            self.completed_steps.push(self.current_step.clone());
        }

        if let Some(next) = self.current_step.next() {
            self.current_step = next;
            self.last_shown = Some(Instant::now());
        }
    }

    /// Go to previous step
    pub fn previous_step(&mut self) {
        if let Some(prev) = self.current_step.previous() {
            self.current_step = prev;
        }
    }

    /// Skip to specific step
    pub fn go_to_step(&mut self, step: OnboardingStep) {
        self.current_step = step;
        self.last_shown = Some(Instant::now());
    }

    /// Complete onboarding
    pub fn complete(&mut self) {
        self.tutorial_completed = true;
        self.is_first_run = false;
    }

    /// Reset onboarding
    pub fn reset(&mut self) {
        self.is_first_run = true;
        self.current_step = OnboardingStep::Welcome;
        self.completed_steps.clear();
        self.tutorial_completed = false;
    }

    /// Get progress percentage
    pub fn progress_percent(&self) -> f32 {
        let current = self.current_step.step_number() as f32;
        let total = OnboardingStep::total_steps() as f32;
        (current / total).min(1.0)
    }
}

/// Onboarding wizard UI
pub struct OnboardingWizard {
    pub state: OnboardingState,
    pub show_preferences: bool,
    pub theme_preference: String,
    pub language_preference: String,
    pub enable_notifications: bool,
}

impl Default for OnboardingWizard {
    fn default() -> Self {
        Self {
            state: OnboardingState::new(),
            show_preferences: true,
            theme_preference: "dark".to_string(),
            language_preference: "zh-CN".to_string(),
            enable_notifications: true,
        }
    }
}

impl OnboardingWizard {
    pub fn render(&mut self, ui: &mut Ui, theme: &crate::design::DesignTheme) -> Option<OnboardingAction> {
        let action;

        match self.state.current_step {
            OnboardingStep::Welcome => {
                action = self.render_welcome(ui, theme);
            }
            OnboardingStep::AddServer => {
                action = self.render_add_server_guide(ui, theme);
            }
            OnboardingStep::ConnectGuide => {
                action = self.render_connect_guide(ui, theme);
            }
            OnboardingStep::TerminalGuide => {
                action = self.render_terminal_guide(ui, theme);
            }
            OnboardingStep::FileBrowserGuide => {
                action = self.render_file_browser_guide(ui, theme);
            }
            OnboardingStep::SnippetsGuide => {
                action = self.render_snippets_guide(ui, theme);
            }
            OnboardingStep::ShortcutsGuide => {
                action = self.render_shortcuts_guide(ui, theme);
            }
            OnboardingStep::Complete => {
                action = self.render_complete(ui, theme);
            }
        }

        action
    }

    fn render_welcome(&mut self, ui: &mut Ui, theme: &crate::design::DesignTheme) -> Option<OnboardingAction> {
        let mut action = None;

        ui.vertical_centered(|ui| {
            ui.add_space(48.0);

            // Welcome icon
            ui.label(
                RichText::new("🚀")
                    .size(64.0)
            );

            ui.add_space(24.0);

            // Title
            ui.label(
                RichText::new("欢迎使用 EasySSH")
                    .size(28.0)
                    .strong()
                    .color(theme.text_primary)
            );

            ui.add_space(16.0);

            // Subtitle
            ui.label(
                RichText::new("安全、高效的SSH客户端")
                    .size(16.0)
                    .color(theme.text_secondary)
            );

            ui.add_space(32.0);

            // Feature highlights
            ui.horizontal(|ui| {
                self.render_feature_card(ui, theme, "🔒", "安全存储", "密码加密存储\n保障数据安全");
                ui.add_space(16.0);
                self.render_feature_card(ui, theme, "⚡", "快速连接", "一键连接服务器\n支持多种认证");
                ui.add_space(16.0);
                self.render_feature_card(ui, theme, "📁", "文件管理", "可视化SFTP浏览\n轻松传输文件");
            });

            ui.add_space(48.0);

            // Preferences section
            if self.show_preferences {
                Frame::group(ui.style())
                    .fill(theme.bg_tertiary)
                    .rounding(Rounding::same(8.0))
                    .show(ui, |ui| {
                        ui.set_width(400.0);

                        ui.label(
                            RichText::new("快速设置")
                                .size(14.0)
                                .strong()
                                .color(theme.text_primary)
                        );

                        ui.add_space(16.0);

                        // Theme preference
                        ui.horizontal(|ui| {
                            ui.label("主题:");
                            ui.radio_value(&mut self.theme_preference, "light".to_string(), "浅色");
                            ui.radio_value(&mut self.theme_preference, "dark".to_string(), "深色");
                            ui.radio_value(&mut self.theme_preference, "system".to_string(), "跟随系统");
                        });

                        ui.add_space(8.0);

                        // Notifications
                        ui.checkbox(&mut self.enable_notifications, "启用通知提醒");
                    });

                ui.add_space(24.0);
            }

            // Start button
            if ui.add(
                crate::design::AccessibleButton::new(theme, "开始使用 →")
                    .style(crate::design::AccessibleButtonStyle::Primary)
                    .build()
                    .min_size(Vec2::new(200.0, 44.0))
            ).clicked() {
                action = Some(OnboardingAction::Next);
            }

            ui.add_space(16.0);

            // Skip option
            if ui.small_button("跳过教程").clicked() {
                action = Some(OnboardingAction::Skip);
            }
        });

        action
    }

    fn render_add_server_guide(&mut self, ui: &mut Ui, theme: &crate::design::DesignTheme) -> Option<OnboardingAction> {
        let mut action = None;

        self.render_step_header(ui, theme, OnboardingStep::AddServer);

        ui.vertical_centered(|ui| {
            ui.add_space(32.0);

            // Visual guide
            ui.label(
                RichText::new("➕")
                    .size(48.0)
                    .color(theme.interactive_primary)
            );

            ui.add_space(16.0);

            ui.label(
                RichText::new("添加您的第一台服务器")
                    .size(20.0)
                    .strong()
                    .color(theme.text_primary)
            );

            ui.add_space(16.0);

            // Step by step guide
            Frame::group(ui.style())
                .fill(theme.bg_secondary)
                .rounding(Rounding::same(8.0))
                .inner_margin(Margin::same(24.0))
                .show(ui, |ui| {
                    ui.set_width(500.0);

                    self.render_guide_step(ui, theme, "1", "点击左侧面板的  +  添加服务器");
                    self.render_guide_step(ui, theme, "2", "输入服务器名称、主机地址和端口");
                    self.render_guide_step(ui, theme, "3", "选择认证方式（密码或密钥）");
                    self.render_guide_step(ui, theme, "4", "保存配置");
                });

            ui.add_space(24.0);

            // Tip
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("💡")
                        .size(20.0)
                );
                ui.label(
                    RichText::new("提示：您可以导入 ~/.ssh/config 文件快速添加多个服务器")
                        .size(13.0)
                        .color(theme.text_secondary)
                );
            });

            ui.add_space(32.0);

            // Navigation buttons
            action = self.render_navigation_buttons(ui, theme, true, true);
        });

        action
    }

    fn render_connect_guide(&mut self, ui: &mut Ui, theme: &crate::design::DesignTheme) -> Option<OnboardingAction> {
        let mut action = None;

        self.render_step_header(ui, theme, OnboardingStep::ConnectGuide);

        ui.vertical_centered(|ui| {
            ui.add_space(32.0);

            ui.label(
                RichText::new("🔌")
                    .size(48.0)
                    .color(theme.interactive_primary)
            );

            ui.add_space(16.0);

            ui.label(
                RichText::new("连接到服务器")
                    .size(20.0)
                    .strong()
                    .color(theme.text_primary)
            );

            ui.add_space(16.0);

            Frame::group(ui.style())
                .fill(theme.bg_secondary)
                .rounding(Rounding::same(8.0))
                .inner_margin(Margin::same(24.0))
                .show(ui, |ui| {
                    ui.set_width(500.0);

                    self.render_guide_step(ui, theme, "1", "在服务器列表中选择一个服务器");
                    self.render_guide_step(ui, theme, "2", "点击  连接  按钮");
                    self.render_guide_step(ui, theme, "3", "输入密码（或选择已保存的密码）");
                    self.render_guide_step(ui, theme, "4", "开始您的SSH会话");
                });

            ui.add_space(24.0);

            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("💡")
                        .size(20.0)
                );
                ui.label(
                    RichText::new("提示：勾选\"记住密码\"可自动填充下次登录")
                        .size(13.0)
                        .color(theme.text_secondary)
                );
            });

            ui.add_space(32.0);

            action = self.render_navigation_buttons(ui, theme, true, true);
        });

        action
    }

    fn render_terminal_guide(&mut self, ui: &mut Ui, theme: &crate::design::DesignTheme) -> Option<OnboardingAction> {
        let mut action = None;

        self.render_step_header(ui, theme, OnboardingStep::TerminalGuide);

        ui.vertical_centered(|ui| {
            ui.add_space(32.0);

            ui.label(
                RichText::new("▸")
                    .size(48.0)
                    .color(theme.interactive_primary)
            );

            ui.add_space(16.0);

            ui.label(
                RichText::new("使用终端")
                    .size(20.0)
                    .strong()
                    .color(theme.text_primary)
            );

            ui.add_space(16.0);

            Frame::group(ui.style())
                .fill(theme.bg_secondary)
                .rounding(Rounding::same(8.0))
                .inner_margin(Margin::same(24.0))
                .show(ui, |ui| {
                    ui.set_width(500.0);

                    self.render_guide_step(ui, theme, "⌨", "在终端中直接输入命令并回车执行");
                    self.render_guide_step(ui, theme, "📜", "使用 ↑/↓ 键浏览命令历史");
                    self.render_guide_step(ui, theme, "📋", "Ctrl+C 复制，Ctrl+V 粘贴");
                    self.render_guide_step(ui, theme, "🔍", "Ctrl+Plus/Minus 调整字体大小");
                });

            ui.add_space(32.0);

            action = self.render_navigation_buttons(ui, theme, true, true);
        });

        action
    }

    fn render_file_browser_guide(&mut self, ui: &mut Ui, theme: &crate::design::DesignTheme) -> Option<OnboardingAction> {
        let mut action = None;

        self.render_step_header(ui, theme, OnboardingStep::FileBrowserGuide);

        ui.vertical_centered(|ui| {
            ui.add_space(32.0);

            ui.label(
                RichText::new("📁")
                    .size(48.0)
                    .color(theme.interactive_primary)
            );

            ui.add_space(16.0);

            ui.label(
                RichText::new("文件管理")
                    .size(20.0)
                    .strong()
                    .color(theme.text_primary)
            );

            ui.add_space(16.0);

            Frame::group(ui.style())
                .fill(theme.bg_secondary)
                .rounding(Rounding::same(8.0))
                .inner_margin(Margin::same(24.0))
                .show(ui, |ui| {
                    ui.set_width(500.0);

                    self.render_guide_step(ui, theme, "1", "连接后点击  SFTP  按钮");
                    self.render_guide_step(ui, theme, "2", "浏览远程文件系统，双击进入目录");
                    self.render_guide_step(ui, theme, "3", "拖拽文件进行上传/下载");
                    self.render_guide_step(ui, theme, "4", "右键点击文件进行更多操作");
                });

            ui.add_space(32.0);

            action = self.render_navigation_buttons(ui, theme, true, true);
        });

        action
    }

    fn render_snippets_guide(&mut self, ui: &mut Ui, theme: &crate::design::DesignTheme) -> Option<OnboardingAction> {
        let mut action = None;

        self.render_step_header(ui, theme, OnboardingStep::SnippetsGuide);

        ui.vertical_centered(|ui| {
            ui.add_space(32.0);

            ui.label(
                RichText::new("⚡")
                    .size(48.0)
                    .color(theme.interactive_primary)
            );

            ui.add_space(16.0);

            ui.label(
                RichText::new("命令片段")
                    .size(20.0)
                    .strong()
                    .color(theme.text_primary)
            );

            ui.add_space(16.0);

            Frame::group(ui.style())
                .fill(theme.bg_secondary)
                .rounding(Rounding::same(8.0))
                .inner_margin(Margin::same(24.0))
                .show(ui, |ui| {
                    ui.set_width(500.0);

                    self.render_guide_step(ui, theme, "💾", "保存常用命令为片段");
                    self.render_guide_step(ui, theme, "⚡", "使用 Ctrl+Shift+Space 快速插入");
                    self.render_guide_step(ui, theme, "📂", "按类别组织您的片段库");
                    self.render_guide_step(ui, theme, "🔍", "搜索快速找到需要的命令");
                });

            ui.add_space(32.0);

            action = self.render_navigation_buttons(ui, theme, true, true);
        });

        action
    }

    fn render_shortcuts_guide(&mut self, ui: &mut Ui, theme: &crate::design::DesignTheme) -> Option<OnboardingAction> {
        let mut action = None;

        self.render_step_header(ui, theme, OnboardingStep::ShortcutsGuide);

        ui.vertical_centered(|ui| {
            ui.add_space(32.0);

            ui.label(
                RichText::new("⌨")
                    .size(48.0)
                    .color(theme.interactive_primary)
            );

            ui.add_space(16.0);

            ui.label(
                RichText::new("快捷键")
                    .size(20.0)
                    .strong()
                    .color(theme.text_primary)
            );

            ui.add_space(16.0);

            // Shortcut grid
            Frame::group(ui.style())
                .fill(theme.bg_secondary)
                .rounding(Rounding::same(8.0))
                .inner_margin(Margin::same(24.0))
                .show(ui, |ui| {
                    ui.set_width(500.0);

                    self.render_shortcut_row(ui, theme, "Ctrl + K", "打开命令面板");
                    self.render_shortcut_row(ui, theme, "Ctrl + T", "新建标签页");
                    self.render_shortcut_row(ui, theme, "Ctrl + W", "关闭标签页");
                    self.render_shortcut_row(ui, theme, "Ctrl + Shift + F", "全局搜索");
                    self.render_shortcut_row(ui, theme, "Ctrl + B", "切换侧边栏");
                    self.render_shortcut_row(ui, theme, "F11", "全屏模式");
                });

            ui.add_space(24.0);

            ui.label(
                RichText::new("按 Ctrl+Shift+/ 随时查看完整的快捷键列表")
                    .size(13.0)
                    .color(theme.text_secondary)
            );

            ui.add_space(32.0);

            action = self.render_navigation_buttons(ui, theme, true, true);
        });

        action
    }

    fn render_complete(&mut self, ui: &mut Ui, theme: &crate::design::DesignTheme) -> Option<OnboardingAction> {
        let mut action = None;

        ui.vertical_centered(|ui| {
            ui.add_space(48.0);

            // Success animation
            ui.label(
                RichText::new("✓")
                    .size(80.0)
                    .color(crate::design::SemanticColors::SUCCESS)
            );

            ui.add_space(24.0);

            ui.label(
                RichText::new("准备就绪！")
                    .size(28.0)
                    .strong()
                    .color(theme.text_primary)
            );

            ui.add_space(16.0);

            ui.label(
                RichText::new("您已完成所有教程，可以开始使用 EasySSH 了")
                    .size(16.0)
                    .color(theme.text_secondary)
            );

            ui.add_space(48.0);

            // Quick actions
            ui.label(
                RichText::new("接下来您可以：")
                    .size(14.0)
                    .strong()
                    .color(theme.text_primary)
            );

            ui.add_space(16.0);

            ui.horizontal(|ui| {
                if ui.button("🖥 添加服务器").clicked() {
                    action = Some(OnboardingAction::AddServer);
                }

                ui.add_space(8.0);

                if ui.button("⚙ 打开设置").clicked() {
                    action = Some(OnboardingAction::OpenSettings);
                }

                ui.add_space(8.0);

                if ui.button("❓ 查看帮助").clicked() {
                    action = Some(OnboardingAction::OpenHelp);
                }
            });

            ui.add_space(48.0);

            if ui.add(
                crate::design::AccessibleButton::new(theme, "开始使用")
                    .style(crate::design::AccessibleButtonStyle::Primary)
                    .build()
                    .min_size(Vec2::new(200.0, 44.0))
            ).clicked() {
                action = Some(OnboardingAction::Finish);
            }
        });

        action
    }

    fn render_step_header(&self, ui: &mut Ui, theme: &crate::design::DesignTheme, step: OnboardingStep) {
        ui.horizontal(|ui| {
            // Progress indicator
            let total = OnboardingStep::total_steps() as f32;
            let current = step.step_number() as f32;
            let progress = current / total;

            ui.add(
                egui::ProgressBar::new(progress)
                    .desired_width(200.0)
                    .text(format!("步骤 {} / {}", step.step_number(), total as usize))
            );
        });

        ui.add_space(16.0);

        ui.horizontal(|ui| {
            ui.label(
                RichText::new(step.icon())
                    .size(24.0)
            );

            ui.label(
                RichText::new(step.display_name())
                    .size(18.0)
                    .strong()
                    .color(theme.text_primary)
            );

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui.small_button("跳过教程").clicked() {
                    // Handled by caller
                }
            });
        });

        ui.separator();
    }

    fn render_feature_card(&self, ui: &mut Ui, theme: &crate::design::DesignTheme, icon: &str, title: &str, description: &str) {
        Frame::group(ui.style())
            .fill(theme.bg_tertiary)
            .rounding(Rounding::same(8.0))
            .inner_margin(Margin::same(16.0))
            .show(ui, |ui| {
                ui.set_width(120.0);

                ui.vertical_centered(|ui| {
                    ui.label(
                        RichText::new(icon)
                            .size(32.0)
                    );

                    ui.add_space(8.0);

                    ui.label(
                        RichText::new(title)
                            .size(14.0)
                            .strong()
                            .color(theme.text_primary)
                    );

                    ui.add_space(4.0);

                    ui.label(
                        RichText::new(description)
                            .size(12.0)
                            .color(theme.text_secondary)
                    );
                });
            });
    }

    fn render_guide_step(&self, ui: &mut Ui, theme: &crate::design::DesignTheme, icon: &str, description: &str) {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(icon)
                    .size(20.0)
                    .color(theme.interactive_primary)
            );

            ui.add_space(12.0);

            ui.label(
                RichText::new(description)
                    .size(14.0)
                    .color(theme.text_primary)
            );
        });

        ui.add_space(12.0);
    }

    fn render_shortcut_row(&self, ui: &mut Ui, theme: &crate::design::DesignTheme, shortcut: &str, description: &str) {
        ui.horizontal(|ui| {
            // Shortcut key display
            Frame::group(ui.style())
                .fill(theme.bg_tertiary)
                .rounding(Rounding::same(4.0))
                .inner_margin(Margin::symmetric(8.0, 4.0))
                .show(ui, |ui| {
                    ui.label(
                        RichText::new(shortcut)
                            .size(13.0)
                            .monospace()
                            .color(theme.text_primary)
                    );
                });

            ui.add_space(16.0);

            ui.label(
                RichText::new(description)
                    .size(14.0)
                    .color(theme.text_secondary)
            );
        });

        ui.add_space(8.0);
    }

    fn render_navigation_buttons(
        &self,
        ui: &mut Ui,
        theme: &crate::design::DesignTheme,
        show_previous: bool,
        show_next: bool,
    ) -> Option<OnboardingAction> {
        let mut action = None;

        ui.horizontal(|ui| {
            if show_previous {
                if ui.add(
                    crate::design::AccessibleButton::new(theme, "← 上一步")
                        .style(crate::design::AccessibleButtonStyle::Ghost)
                        .build()
                ).clicked() {
                    action = Some(OnboardingAction::Previous);
                }
            }

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if show_next {
                    if ui.add(
                        crate::design::AccessibleButton::new(theme, "下一步 →")
                            .style(crate::design::AccessibleButtonStyle::Primary)
                            .build()
                    ).clicked() {
                        action = Some(OnboardingAction::Next);
                    }
                }
            });
        });

        action
    }
}

/// Actions that can be triggered from onboarding
#[derive(Clone, Debug, PartialEq)]
pub enum OnboardingAction {
    Next,
    Previous,
    Skip,
    Finish,
    AddServer,
    OpenSettings,
    OpenHelp,
}

/// Quick tip that can be shown inline
pub struct QuickTip {
    pub icon: String,
    pub message: String,
    pub learn_more_url: Option<String>,
}

impl QuickTip {
    pub fn new(icon: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            icon: icon.into(),
            message: message.into(),
            learn_more_url: None,
        }
    }

    pub fn render(&self, ui: &mut Ui, theme: &crate::design::DesignTheme) {
        Frame::group(ui.style())
            .fill(crate::design::BrandColors::C500.linear_multiply(0.1))
            .rounding(Rounding::same(6.0))
            .stroke(Stroke::new(1.0, crate::design::BrandColors::C500.linear_multiply(0.3)))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(&self.icon)
                            .size(18.0)
                            .color(crate::design::BrandColors::C500)
                    );

                    ui.add_space(8.0);

                    ui.label(
                        RichText::new(&self.message)
                            .size(13.0)
                            .color(theme.text_primary)
                    );
                });
            });
    }
}
