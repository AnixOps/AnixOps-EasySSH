import SwiftUI
import SwiftData

/// Main app entry point with SwiftData integration
@main
struct EasySSHApp: App {
    @NSApplicationDelegateAdaptor var appDelegate: EasySSHAppDelegate

    // SwiftData container - shared across the app
    let container: ModelContainer

    init() {
        // Initialize SwiftData container with iCloud sync
        do {
            let schema = Schema([
                ServerModel.self,
                ServerGroupModel.self,
                SSHIdentityModel.self,
                ConnectionProfileModel.self,
                SessionRecordModel.self,
                SnippetModel.self,
                AppSettingsModel.self
            ])

            let configuration = ModelConfiguration(
                schema: schema,
                isStoredInMemoryOnly: false,
                cloudKitDatabase: .automatic
            )

            container = try ModelContainer(
                for: schema,
                configurations: [configuration]
            )

            print("SwiftData container initialized with CloudKit sync")

        } catch {
            fatalError("Failed to initialize SwiftData container: \(error)")
        }

        // Perform data migration in background
        Task { @MainActor in
            await performDataMigration()
        }
    }

    var body: some Scene {
        WindowGroup {
            AppShell()
                .modelContainer(container)
        }
        .defaultSize(width: 1200, height: 800)
        .windowStyle(.titleBar)
        .commands {
            appCommands
        }
        .windowResizability(.contentSize)

        Settings {
            SettingsContainer()
                .modelContainer(container)
        }

        // Quick Connect window
        WindowGroup("Quick Connect", id: "quick-connect") {
            QuickConnectView()
                .modelContainer(container)
        }
        .defaultSize(width: 400, height: 300)

        // Menu Bar Extra
        MenuBarExtra("EasySSH", systemImage: "terminal.fill") {
            MenuBarExtraView()
                .modelContainer(container)
        }
        .menuBarExtraStyle(.window)
    }

    private var appCommands: some Commands {
        Group {
            CommandGroup(replacing: .appSettings) {
                SettingsLink()
            }

            CommandMenu("Connection") {
                Button("Quick Connect...") {
                    NSApp.sendAction(Selector(("showQuickConnectWindow:")), to: nil, from: nil)
                }
                .keyboardShortcut("n", modifiers: [.command, .shift])

                Divider()

                Button("Connect to Selected") {
                    NotificationCenter.default.post(name: .connectSelectedServer, object: nil)
                }
                .keyboardShortcut(.return, modifiers: .command)

                Button("Disconnect") {
                    NotificationCenter.default.post(name: .disconnectSelectedServer, object: nil)
                }
                .keyboardShortcut("d", modifiers: [.command, .shift])

                Divider()

                Button("Edit Server...") {
                    NotificationCenter.default.post(name: .editSelectedServer, object: nil)
                }
                .keyboardShortcut("e", modifiers: .command)
            }

            CommandMenu("Server") {
                Button("Add Server...") {
                    NotificationCenter.default.post(name: .showAddServer, object: nil)
                }
                .keyboardShortcut("n", modifiers: .command)

                Button("Duplicate Server") {
                    NotificationCenter.default.post(name: .duplicateSelectedServer, object: nil)
                }
                .keyboardShortcut("d", modifiers: .command)

                Divider()

                Button("Delete Server") {
                    NotificationCenter.default.post(name: .deleteSelectedServer, object: nil)
                }
                .keyboardShortcut(.delete, modifiers: .command)

                Divider()

                Button("Import from ~/.ssh/config") {
                    NotificationCenter.default.post(name: .importSSHConfig, object: nil)
                }

                Button("Export Servers...") {
                    NotificationCenter.default.post(name: .exportData, object: nil)
                }
            }

            CommandMenu("View") {
                Button("Toggle Sidebar") {
                    NotificationCenter.default.post(name: .toggleSidebar, object: nil)
                }
                .keyboardShortcut("s", modifiers: [.command, .control])

                Divider()

                Picker("Connection Mode", selection: connectionModeBinding) {
                    Text("Lite (Native Terminal)").tag(ConnectionMode.lite)
                    Text("Standard (Embedded)").tag(ConnectionMode.standard)
                    Text("Pro (Team)").tag(ConnectionMode.pro)
                }
            }

            CommandMenu("Sync") {
                Button("Sync Now") {
                    SwiftDataService.shared.triggerCloudSync()
                }
                .keyboardShortcut("r", modifiers: .command)

                Divider()

                Button("Import Data...") {
                    NotificationCenter.default.post(name: .importData, object: nil)
                }

                Button("Export Data...") {
                    NotificationCenter.default.post(name: .exportData, object: nil)
                }
            }

            // Toolbar commands
            ToolbarCommands()
        }
    }

    private var connectionModeBinding: Binding<ConnectionMode> {
        Binding(
            get: { .lite },
            set: { _ in }
        )
    }

    @MainActor
    private func performDataMigration() async {
        let migrationService = DataMigrationService.shared

        if migrationService.isMigrationNeeded {
            print("Data migration required, starting...")

            do {
                let result = try await migrationService.performMigration()
                print(result.summary)

                if result.hasErrors {
                    // Log errors but don't fail - partial migration is acceptable
                    print("Migration completed with warnings:")
                    result.serverErrors.forEach { print("  - \($0)") }
                    result.groupErrors.forEach { print("  - \($0)") }
                }
            } catch {
                print("Migration failed: \(error)")
            }
        }

        // Ensure schema is up to date
        do {
            try await migrationService.migrateSchemaIfNeeded()
        } catch {
            print("Schema migration failed: \(error)")
        }
    }
}

// MARK: - Helper Views

struct SettingsLink: View {
    var body: some View {
        Button("Settings...") {
            NSApp.sendAction(Selector(("showPreferencesWindow:")), to: nil, from: nil)
        }
        .keyboardShortcut(",", modifiers: .command)
    }
}

// MARK: - Notification Names

extension Notification.Name {
    static let showAddServer = Notification.Name("easyssh.showAddServer")
    static let connectSelectedServer = Notification.Name("easyssh.connectSelectedServer")
    static let disconnectSelectedServer = Notification.Name("easyssh.disconnectSelectedServer")
    static let editSelectedServer = Notification.Name("easyssh.editSelectedServer")
    static let duplicateSelectedServer = Notification.Name("easyssh.duplicateSelectedServer")
    static let deleteSelectedServer = Notification.Name("easyssh.deleteSelectedServer")
    static let toggleSidebar = Notification.Name("easyssh.toggleSidebar")
    static let toggleFavoritesFilter = Notification.Name("easyssh.toggleFavoritesFilter")
    static let triggerCloudSync = Notification.Name("easyssh.triggerCloudSync")
    static let exportData = Notification.Name("easyssh.exportData")
    static let importData = Notification.Name("easyssh.importData")
    static let importSSHConfig = Notification.Name("easyssh.importSSHConfig")
}

// MARK: - Enums

enum SidebarVisibility: String {
    case visible, hidden, automatic
}

enum ConnectionMode: String, CaseIterable {
    case lite = "Lite"
    case standard = "Standard"
    case pro = "Pro"
}
