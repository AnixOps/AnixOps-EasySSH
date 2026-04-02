import SwiftUI
import SwiftData

/// Settings container with tab navigation and SwiftData integration
struct SettingsContainer: View {
    @EnvironmentObject var appState: AppState
    @Environment(\.modelContext) private var modelContext

    @Query private var settingsQuery: [AppSettingsModel]
    @State private var settings: AppSettingsModel?

    var body: some View {
        Group {
            if let settings = settings ?? settingsQuery.first {
                SettingsTabs(settings: settings)
                    .onAppear {
                        self.settings = settings
                    }
            } else {
                // Create default settings if none exist
                ProgressView("Loading settings...")
                    .onAppear {
                        createDefaultSettings()
                    }
            }
        }
        .frame(width: 600, height: 450)
    }

    private func createDefaultSettings() {
        let newSettings = AppSettingsModel()
        modelContext.insert(newSettings)
        try? modelContext.save()
        settings = newSettings
    }
}

struct SettingsTabs: View {
    @Bindable var settings: AppSettingsModel

    var body: some View {
        TabView {
            GeneralSettings(settings: settings)
                .tabItem {
                    Label("General", systemImage: "gear")
                }

            AppearanceSettings(settings: settings)
                .tabItem {
                    Label("Appearance", systemImage: "paintbrush")
                }

            ConnectionSettings(settings: settings)
                .tabItem {
                    Label("Connection", systemImage: "network")
                }

            SecuritySettings(settings: settings)
                .tabItem {
                    Label("Security", systemImage: "lock.shield")
                }

            TerminalSettings(settings: settings)
                .tabItem {
                    Label("Terminal", systemImage: "terminal")
                }

            SyncSettings(settings: settings)
                .tabItem {
                    Label("Sync", systemImage: "arrow.clockwise.icloud")
                }

            AdvancedSettings(settings: settings)
                .tabItem {
                    Label("Advanced", systemImage: "gearshape.2")
                }
        }
    }
}

// MARK: - General Settings

struct GeneralSettings: View {
    @Bindable var settings: AppSettingsModel
    @AppStorage("launchAtLogin") private var launchAtLogin = false
    @AppStorage("confirmBeforeQuit") private var confirmBeforeQuit = false
    @AppStorage("showInMenuBar") private var showInMenuBar = true
    @AppStorage("showInDock") private var showInDock = true

    var body: some View {
        Form {
            Section("Startup & Quit") {
                Toggle("Launch at login", isOn: $launchAtLogin)

                Toggle("Show in menu bar", isOn: $showInMenuBar)

                Toggle("Show in Dock", isOn: $showInDock)

                if !showInMenuBar && !showInDock {
                    Text("At least one visibility option (Menu Bar or Dock) must be enabled")
                        .font(.caption)
                        .foregroundStyle(.red)
                }

                Toggle("Confirm before quitting", isOn: $confirmBeforeQuit)
            }

            Section("Session Management") {
                Picker("Default connection mode", selection: $settings.connectionMode) {
                    ForEach(ConnectionMode.allCases, id: \.self) { mode in
                        Text(mode.rawValue).tag(mode)
                    }
                }

                Picker("Startup behavior", selection: $settings.startupBehavior) {
                    Text("Show main window").tag(StartupBehavior.showWindow)
                    Text("Show menu bar only").tag(StartupBehavior.showMenuBarOnly)
                    Text("Restore sessions").tag(StartupBehavior.restoreSessions)
                }
            }

            Section("Notifications") {
                Toggle("Connection notifications", isOn: .constant(true))
                Toggle("Transfer complete notifications", isOn: .constant(true))
                Toggle("Error notifications", isOn: .constant(true))
            }
        }
        .formStyle(.grouped)
        .padding()
    }
}

// MARK: - Appearance Settings

struct AppearanceSettings: View {
    @Bindable var settings: AppSettingsModel

    let accentColors = [
        ("Blue", "blue", Color.blue),
        ("Purple", "purple", Color.purple),
        ("Green", "green", Color.green),
        ("Orange", "orange", Color.orange),
        ("Red", "red", Color.red),
        ("Pink", "pink", Color.pink),
        ("Teal", "teal", Color.teal),
        ("Indigo", "indigo", Color.indigo)
    ]

    var body: some View {
        Form {
            Section("Theme") {
                Picker("Sidebar visibility", selection: $settings.sidebarVisibility) {
                    Text("Always show").tag(SidebarVisibility.visible)
                    Text("Auto-hide").tag(SidebarVisibility.automatic)
                    Text("Always hide").tag(SidebarVisibility.hidden)
                }
                .pickerStyle(.segmented)

                Picker("Sidebar icon style", selection: .constant("medium")) {
                    Text("Small").tag("small")
                    Text("Medium").tag("medium")
                    Text("Large").tag("large")
                }
            }

            Section("Accent Color") {
                LazyVGrid(columns: [
                    GridItem(.adaptive(minimum: 60))
                ], spacing: 12) {
                    ForEach(accentColors, id: \.0) { name, key, color in
                        AccentColorButton(
                            name: name,
                            color: color,
                            isSelected: false
                        ) {
                            // Apply accent color
                        }
                    }
                }
            }

            Section("Font & Typography") {
                Picker("Interface font size", selection: .constant(13)) {
                    Text("Small (11pt)").tag(11)
                    Text("Default (13pt)").tag(13)
                    Text("Large (15pt)").tag(15)
                }

                Toggle("Use monospace font for server names", isOn: .constant(false))
            }
        }
        .formStyle(.grouped)
        .padding()
    }
}

struct AccentColorButton: View {
    let name: String
    let color: Color
    let isSelected: Bool
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            VStack(spacing: 4) {
                Circle()
                    .fill(color)
                    .frame(width: 32, height: 32)
                    .overlay(
                        Circle()
                            .stroke(isSelected ? Color.white : Color.clear, lineWidth: 2)
                    )
                    .shadow(radius: isSelected ? 2 : 0)

                Text(name)
                    .font(.system(size: 11))
            }
        }
        .buttonStyle(.plain)
    }
}

// MARK: - Connection Settings

struct ConnectionSettings: View {
    @Bindable var settings: AppSettingsModel

    var body: some View {
        Form {
            Section("Defaults") {
                Picker("Default authentication", selection: $settings.connectionProfile.authType) {
                    ForEach(AuthType.allCases) { type in
                        Text(type.displayName).tag(type)
                    }
                }
            }

            Section("Connection Behavior") {
                Toggle("Keep alive enabled", isOn: $settings.connectionProfile.keepAlive)
                Toggle("Forward SSH agent", isOn: $settings.connectionProfile.forwardAgent)

                Picker("Default port", selection: $settings.connectionProfile.port) {
                    Text("22 (Standard)").tag(22)
                    Text("2222").tag(2222)
                    Text("8022").tag(8022)
                }
            }

            Section("SSH Configuration") {
                Toggle("Use system SSH config (~/.ssh/config)", isOn: .constant(true))
                Toggle("Load SSH keys from agent", isOn: .constant(true))
                Toggle("Strict host key checking", isOn: .constant(true))
            }
        }
        .formStyle(.grouped)
        .padding()
    }
}

// MARK: - Security Settings

struct SecuritySettings: View {
    @Bindable var settings: AppSettingsModel
    @AppStorage("useKeychain") private var useKeychain = true

    var body: some View {
        Form {
            Section("Password & Keys") {
                Toggle("Use Keychain for passwords", isOn: $useKeychain)
                Toggle("Unlock SSH keys on demand", isOn: .constant(true))
                Toggle("Clear clipboard after copy", isOn: $settings.clearClipboardOnExit)

                Picker("Clipboard clear delay", selection: .constant(30)) {
                    Text("10 seconds").tag(10)
                    Text("30 seconds").tag(30)
                    Text("1 minute").tag(60)
                    Text("5 minutes").tag(300)
                }
            }

            Section("App Lock") {
                Toggle("Require unlock on launch", isOn: $settings.requireUnlockOnLaunch)

                HStack {
                    Text("Lock after inactivity")
                    Spacer()
                    Picker("", selection: $settings.lockAfterMinutes) {
                        Text("Never").tag(0)
                        Text("1 minute").tag(1)
                        Text("5 minutes").tag(5)
                        Text("15 minutes").tag(15)
                        Text("30 minutes").tag(30)
                    }
                    .frame(width: 150)
                }
            }

            Section {
                Button("Clear all saved passwords...") {
                    // Clear keychain
                }
                .foregroundStyle(.red)

                Button("Export credentials backup...") {
                    // Export encrypted backup
                }
            }
        }
        .formStyle(.grouped)
        .padding()
    }
}

// MARK: - Terminal Settings

struct TerminalSettings: View {
    @Bindable var settings: AppSettingsModel
    @AppStorage("defaultTerminal") private var defaultTerminal = "Terminal.app"

    var body: some View {
        Form {
            Section("External Terminal") {
                Picker("Default terminal", selection: $defaultTerminal) {
                    Text("Terminal.app").tag("Terminal.app")
                    Text("iTerm2").tag("iTerm2")
                    Text("Kitty").tag("Kitty")
                    Text("Alacritty").tag("Alacritty")
                    Text("WezTerm").tag("WezTerm")
                    Text("Hyper").tag("Hyper")
                }

                Toggle("Open in new tab instead of window", isOn: .constant(false))
            }

            Section("Embedded Terminal (Standard/Pro)") {
                Picker("Font", selection: $settings.fontFamily) {
                    Text("SF Mono").tag("SF Mono")
                    Text("JetBrains Mono").tag("JetBrains Mono")
                    Text("Fira Code").tag("Fira Code")
                    Text("Source Code Pro").tag("Source Code Pro")
                }

                HStack {
                    Text("Font size")
                    Spacer()
                    Slider(value: $settings.fontSize, in: 8...24, step: 1)
                        .frame(width: 150)
                    Text("\(Int(settings.fontSize))pt")
                        .foregroundStyle(.secondary)
                        .frame(width: 50, alignment: .trailing)
                }

                Toggle("Use WebGL acceleration", isOn: $settings.enableWebGL)
                Toggle("Blink cursor", isOn: .constant(true))
                Picker("Cursor style", selection: .constant("block")) {
                    Text("Block").tag("block")
                    Text("Line").tag("line")
                    Text("Bar").tag("bar")
                }
            }

            Section("Behavior") {
                Toggle("Copy on selection", isOn: .constant(true))
                Toggle("Paste on right-click", isOn: .constant(false))
            }
        }
        .formStyle(.grouped)
        .padding()
    }
}

// MARK: - Sync Settings

struct SyncSettings: View {
    @Bindable var settings: AppSettingsModel
    @ObservedObject private var swiftDataService = SwiftDataService.shared

    var body: some View {
        Form {
            Section("iCloud Sync") {
                Toggle("Enable iCloud sync", isOn: $settings.enableCloudSync)

                if settings.enableCloudSync {
                    VStack(alignment: .leading, spacing: 8) {
                        CloudSyncStatusView(status: swiftDataService.syncStatus)

                        if let lastSync = settings.lastSyncDate {
                            Text("Last synced: \(lastSync.formatted())")
                                .font(.caption)
                                .foregroundStyle(.secondary)
                        }
                    }
                    .padding(.vertical, 4)

                    Picker("Conflict resolution", selection: $settings.syncConflictResolution) {
                        Text("Keep newest").tag(SyncConflictResolution.newestWins)
                        Text("Keep local").tag(SyncConflictResolution.localWins)
                        Text("Keep cloud").tag(SyncConflictResolution.cloudWins)
                        Text("Ask me").tag(SyncConflictResolution.askUser)
                    }

                    Button("Sync now") {
                        swiftDataService.triggerCloudSync()
                    }
                    .disabled(swiftDataService.syncStatus == .syncing)
                }
            }

            Section("Data Management") {
                Button("Export backup...") {
                    // Export to file
                }

                Button("Import from backup...") {
                    // Import from file
                }

                Button("Reset iCloud data...") {
                    // Reset CloudKit data
                }
                .foregroundStyle(.red)
            }
        }
        .formStyle(.grouped)
        .padding()
    }
}

// MARK: - Advanced Settings

struct AdvancedSettings: View {
    @Bindable var settings: AppSettingsModel
    @State private var showingResetConfirmation = false
    @State private var showingRollbackConfirmation = false

    var body: some View {
        Form {
            Section("SwiftData Storage") {
                HStack {
                    Text("Schema version")
                    Spacer()
                    Text("\(settings.schemaVersion)")
                        .foregroundStyle(.secondary)
                }

                if let migrationDate = settings.migrationCompletedAt {
                    HStack {
                        Text("Migration completed")
                        Spacer()
                        Text(migrationDate.formatted())
                            .foregroundStyle(.secondary)
                    }
                }

                Button("View database info") {
                    // Show database details
                }
            }

            Section("Logging & Debug") {
                Toggle("Enable debug logging", isOn: .constant(false))
                Toggle("Log SSH commands", isOn: .constant(false))

                Button("Open logs folder") {
                    // Open logs in Finder
                }
            }

            Section("Import & Export") {
                Button("Import from SSH config") {
                    // Import
                }

                Button("Export all servers") {
                    // Export
                }

                Button("Export as CSV...") {
                    // CSV export
                }
            }

            Section("Troubleshooting") {
                Button("Re-encrypt data (key rotation)") {
                    // Rotate encryption keys
                }

                Button("Rollback to legacy storage...") {
                    showingRollbackConfirmation = true
                }
                .foregroundStyle(.orange)
            }

            Section("Reset") {
                Button("Reset all settings to defaults...") {
                    showingResetConfirmation = true
                }
                .foregroundStyle(.red)

                Button("Clear all data and quit...") {
                    // Clear everything
                }
                .foregroundStyle(.red)
            }
        }
        .formStyle(.grouped)
        .padding()
        .alert("Reset Settings?", isPresented: $showingResetConfirmation) {
            Button("Cancel", role: .cancel) { }
            Button("Reset", role: .destructive) {
                resetSettings()
            }
        } message: {
            Text("This will reset all settings to their default values. Your server list will not be affected.")
        }
        .alert("Rollback Migration?", isPresented: $showingRollbackConfirmation) {
            Button("Cancel", role: .cancel) { }
            Button("Rollback", role: .destructive) {
                rollbackMigration()
            }
        } message: {
            Text("This will rollback to the legacy storage system. Your data will be preserved but you may lose recent changes.")
        }
    }

    private func resetSettings() {
        // Reset to defaults
        settings.connectionMode = .lite
        settings.startupBehavior = .showWindow
        settings.sidebarVisibility = .automatic
        settings.enableCloudSync = true
        settings.requireUnlockOnLaunch = false
        settings.lockAfterMinutes = 5
        settings.clearClipboardOnExit = true
    }

    private func rollbackMigration() {
        Task {
            do {
                try await DataMigrationService.shared.rollbackMigration()
            } catch {
                print("Rollback failed: \(error)")
            }
        }
    }
}

// MARK: - Connection Profile Wrapper

extension AppSettingsModel {
    var connectionProfile: ConnectionProfileWrapper {
        ConnectionProfileWrapper(settings: self)
    }
}

struct ConnectionProfileWrapper {
    let settings: AppSettingsModel

    var authType: AuthType {
        get { settings.connectionMode == .lite ? .agent : .password }
        nonmutating set { }
    }

    var port: Int {
        get { 22 }
        nonmutating set { }
    }

    var keepAlive: Bool {
        get { true }
        nonmutating set { }
    }

    var forwardAgent: Bool {
        get { false }
        nonmutating set { }
    }
}
