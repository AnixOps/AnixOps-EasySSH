import SwiftUI

struct ContentView: View {
    @EnvironmentObject var appState: AppState
    @State private var searchText = ""
    @State private var showingAddServer = false

    var filteredServers: [Server] {
        if searchText.isEmpty {
            return appState.servers
        }
        return appState.servers.filter {
            $0.name.localizedCaseInsensitiveContains(searchText) ||
            $0.host.localizedCaseInsensitiveContains(searchText)
        }
    }

    var body: some View {
        NavigationSplitView {
            ServerSidebar(
                servers: filteredServers,
                groups: appState.groups,
                selectedServer: $appState.selectedServer,
                searchText: $searchText
            )
            .toolbar {
                ToolbarItem {
                    Button(action: { showingAddServer = true }) {
                        Label("Add Server", systemImage: "plus")
                    }
                }
            }
        } detail: {
            if let server = appState.selectedServer {
                ServerDetailView(server: server)
            } else {
                EmptyStateView()
            }
        }
        .sheet(isPresented: $showingAddServer) {
            AddServerView()
        }
    }
}

struct EmptyStateView: View {
    var body: some View {
        VStack(spacing: 16) {
            Image(systemName: "terminal.fill")
                .font(.system(size: 64))
                .foregroundStyle(.secondary)

            Text("Select a Server")
                .font(.title2)
                .fontWeight(.semibold)

            Text("Choose a server from the sidebar to connect")
                .foregroundStyle(.secondary)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background(.background)
    }
}

struct SettingsView: View {
    var body: some View {
        TabView {
            GeneralSettings()
                .tabItem {
                    Label("General", systemImage: "gear")
                }

            SecuritySettings()
                .tabItem {
                    Label("Security", systemImage: "lock.shield")
                }
        }
        .frame(width: 500, height: 300)
    }
}

struct GeneralSettings: View {
    var body: some View {
        Form {
            Section {
                Picker("Default Terminal", selection: .constant("Terminal.app")) {
                    Text("Terminal.app").tag("Terminal.app")
                    Text("iTerm2").tag("iTerm2")
                    Text("Kitty").tag("Kitty")
                }
            }

            Section {
                Toggle("Show connection notifications", isOn: .constant(true))
                Toggle("Auto-save sessions", isOn: .constant(true))
            }
        }
        .padding()
    }
}

struct SecuritySettings: View {
    var body: some View {
        Form {
            Section {
                Toggle("Use Keychain for passwords", isOn: .constant(true))
                Toggle("Lock after inactivity", isOn: .constant(false))
            }

            Section {
                Button("Clear all credentials...") {
                    // TODO: Implement credential clearing
                }
                .foregroundColor(.red)
            }
        }
        .padding()
    }
}
