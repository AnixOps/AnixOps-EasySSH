import SwiftUI

#Preview("App Shell") {
    AppShell()
        .environmentObject(AppState())
        .environmentObject(ThemeManager())
        .frame(width: 1200, height: 800)
}

#Preview("Sidebar") {
    SidebarView()
        .environmentObject(mockAppState())
        .frame(width: 280, height: 700)
}

#Preview("Server Detail") {
    ServerDetailView(server: .preview)
        .environmentObject(mockAppState())
        .frame(width: 800, height: 600)
}

#Preview("Add Server") {
    AddServerView { _, _ in }
        .frame(width: 500, height: 650)
}

#Preview("Settings") {
    SettingsContainer()
        .environmentObject(AppState())
        .environmentObject(ThemeManager())
        .frame(width: 600, height: 450)
}

#Preview("Empty State") {
    EmptyStateView()
        .environmentObject(mockAppState())
        .frame(width: 800, height: 600)
}

// MARK: - Mock Data

func mockAppState() -> AppState {
    let state = AppState()
    state.servers = [
        Server(
            id: "1",
            name: "Production Web",
            host: "prod.example.com",
            port: 22,
            username: "deploy",
            authType: .key,
            groupId: nil,
            tags: ["production", "web"],
            notes: "Main production server",
            status: .connected,
            lastConnected: Date(),
            isFavorite: true
        ),
        Server(
            id: "2",
            name: "Staging API",
            host: "staging.example.com",
            port: 22,
            username: "developer",
            authType: .agent,
            groupId: nil,
            tags: ["staging"],
            notes: "Staging environment",
            status: .disconnected,
            lastConnected: Date().addingTimeInterval(-86400)
        ),
        Server(
            id: "3",
            name: "Database Primary",
            host: "db1.internal",
            port: 22,
            username: "admin",
            authType: .password,
            groupId: nil,
            tags: ["database", "production"],
            notes: "PostgreSQL primary",
            status: .unknown
        )
    ]
    state.groups = [
        ServerGroup(id: "prod", name: "Production", color: "#FF6B6B", sortOrder: 0),
        ServerGroup(id: "staging", name: "Staging", color: "#4ECDC4", sortOrder: 1)
    ]
    return state
}

extension Server {
    static var preview: Server {
        Server(
            id: "preview-1",
            name: "Preview Server",
            host: "192.168.1.1",
            port: 22,
            username: "root",
            authType: .key,
            groupId: nil,
            tags: ["production", "web", "api"],
            notes: "This is the main production server for our application. Handle with care!\n\nImportant commands:\n- docker-compose ps\n- tail -f /var/log/app.log",
            status: .connected,
            lastConnected: Date(),
            isFavorite: true,
            startupCommand: "tmux new -s main"
        )
    }

    static var previews: [Server] {
        [
            Server(
                id: "1",
                name: "Production Web",
                host: "prod.example.com",
                port: 22,
                username: "deploy",
                authType: .key,
                groupId: nil,
                tags: ["production", "web"],
                notes: "Main production server",
                status: .connected,
                lastConnected: Date(),
                isFavorite: true
            ),
            Server(
                id: "2",
                name: "Staging API",
                host: "staging.example.com",
                port: 22,
                username: "developer",
                authType: .agent,
                groupId: nil,
                tags: ["staging"],
                notes: "Staging environment",
                status: .disconnected,
                lastConnected: Date().addingTimeInterval(-86400)
            ),
            Server(
                id: "3",
                name: "Development DB",
                host: "db1.internal",
                port: 22,
                username: "admin",
                authType: .password,
                groupId: nil,
                tags: ["database", "development"],
                notes: "PostgreSQL primary",
                status: .unknown
            )
        ]
    }

    static var previewConnected: Server {
        Server(
            id: "connected-1",
            name: "Connected Server",
            host: "connected.example.com",
            port: 22,
            username: "root",
            authType: .agent,
            status: .connected,
            lastConnected: Date()
        )
    }

    static var previewDisconnected: Server {
        Server(
            id: "disconnected-1",
            name: "Disconnected Server",
            host: "disconnected.example.com",
            port: 22,
            username: "root",
            authType: .agent,
            status: .disconnected,
            lastConnected: nil
        )
    }

    static var previewError: Server {
        Server(
            id: "error-1",
            name: "Error Server",
            host: "error.example.com",
            port: 22,
            username: "root",
            authType: .password,
            status: .error,
            lastConnected: nil
        )
    }
}

// MARK: - Empty State View

struct EmptyStateView: View {
    @EnvironmentObject var appState: AppState

    var body: some View {
        VStack(spacing: 20) {
            Image(systemName: "server.rack")
                .font(.system(size: 64))
                .foregroundStyle(.tertiary)

            Text("No Server Selected")
                .font(.system(size: 20, weight: .semibold))

            Text("Select a server from the sidebar or add a new one")
                .font(.system(size: 13))
                .foregroundStyle(.secondary)
                .multilineTextAlignment(.center)

            Button("Add Server...") {
                appState.showingAddServer = true
            }
            .buttonStyle(.borderedProminent)
            .controlSize(.large)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background(.background)
    }
}
