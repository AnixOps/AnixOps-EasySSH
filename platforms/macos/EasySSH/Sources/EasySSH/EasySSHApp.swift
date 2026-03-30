import SwiftUI

@main
struct EasySSHApp: App {
    @StateObject private var appState = AppState()

    var body: some Scene {
        WindowGroup {
            ContentView()
                .environmentObject(appState)
        }
        .commands {
            CommandGroup(replacing: .appSettings) {
                SettingsLink()
            }
        }

        Settings {
            SettingsView()
        }
    }
}

class AppState: ObservableObject {
    @Published var servers: [Server] = []
    @Published var groups: [ServerGroup] = []
    @Published var selectedServer: Server?
    @Published var connectionMode: ConnectionMode = .lite

    private let core: EasySSHCoreBridge

    init() {
        self.core = EasySSHCoreBridge()
        loadData()
    }

    func loadData() {
        servers = core.getServers()
        groups = core.getGroups()
    }

    func connect(to server: Server) {
        core.connectNative(server: server)
    }
}

enum ConnectionMode {
    case lite      // Native terminal
    case standard  // Embedded terminal
    case pro       // Team features
}

struct SettingsLink: View {
    var body: some View {
        Button("Settings...") {
            NSApp.sendAction(Selector(("showPreferencesWindow:")), to: nil, from: nil)
        }
        .keyboardShortcut(",", modifiers: .command)
    }
}
