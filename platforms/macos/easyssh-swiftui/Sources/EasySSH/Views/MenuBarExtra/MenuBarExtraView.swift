import SwiftUI

/// Menu Bar Extra mini window view
/// Displays connection status, quick connect list, and quick actions
struct MenuBarExtraView: View {
    @EnvironmentObject var appState: AppState
    @Environment(\.openWindow) private var openWindow
    @Environment(\.dismiss) private var dismiss

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            // Header with connection status
            headerView

            Divider()

            // Quick connect list (favorites and recent)
            quickConnectList

            Divider()

            // Actions
            actionsView
        }
        .frame(width: 300)
        .background(Color(.windowBackgroundColor))
    }

    // MARK: - Header View

    private var headerView: some View {
        HStack {
            Image(systemName: "terminal.fill")
                .font(.title2)
                .foregroundStyle(.accent)

            VStack(alignment: .leading, spacing: 2) {
                Text("EasySSH")
                    .font(.headline)

                HStack(spacing: 4) {
                    ConnectionStatusIndicator(
                        connected: appState.connectionStatus.connectedSessions,
                        total: appState.connectionStatus.totalServers
                    )
                }
            }

            Spacer()

            // Open main window button
            Button {
                openMainWindow()
                dismiss()
            } label: {
                Image(systemName: "arrow.up.forward.app")
                    .font(.body)
            }
            .buttonStyle(.plain)
            .foregroundStyle(.secondary)
            .help("Open EasySSH Main Window")
        }
        .padding(.horizontal, 16)
        .padding(.vertical, 12)
        .background(Color(.controlBackgroundColor))
    }

    // MARK: - Quick Connect List

    private var quickConnectList: some View {
        Group {
            if appState.servers.isEmpty {
                emptyStateView
            } else {
                ScrollView(.vertical, showsIndicators: true) {
                    LazyVStack(alignment: .leading, spacing: 0) {
                        // Favorites section
                        if !favoriteServers.isEmpty {
                            sectionHeader("Favorites")

                            ForEach(favoriteServers) { server in
                                ServerMenuRow(
                                    server: server,
                                    isConnected: isConnected(server),
                                    onConnect: { connect(to: server) },
                                    onDisconnect: { disconnect(from: server) }
                                )
                            }
                        }

                        // Recent connections section
                        if !recentServers.isEmpty {
                            sectionHeader("Recent")

                            ForEach(recentServers.prefix(5)) { server in
                                ServerMenuRow(
                                    server: server,
                                    isConnected: isConnected(server),
                                    onConnect: { connect(to: server) },
                                    onDisconnect: { disconnect(from: server) }
                                )
                            }
                        }

                        // All other servers
                        if !otherServers.isEmpty {
                            sectionHeader("All Servers")

                            ForEach(otherServers.prefix(10)) { server in
                                ServerMenuRow(
                                    server: server,
                                    isConnected: isConnected(server),
                                    onConnect: { connect(to: server) },
                                    onDisconnect: { disconnect(from: server) }
                                )
                            }
                        }
                    }
                }
                .frame(maxHeight: 400)
            }
        }
    }

    private var emptyStateView: some View {
        VStack(spacing: 12) {
            Image(systemName: "server.rack")
                .font(.system(size: 32))
                .foregroundStyle(.secondary)

            Text("No Servers Yet")
                .font(.headline)
                .foregroundStyle(.primary)

            Text("Add a server to get started")
                .font(.caption)
                .foregroundStyle(.secondary)
        }
        .frame(maxWidth: .infinity, minHeight: 150)
        .padding()
    }

    private func sectionHeader(_ title: String) -> some View {
        Text(title)
            .font(.caption)
            .fontWeight(.semibold)
            .foregroundStyle(.secondary)
            .padding(.horizontal, 16)
            .padding(.vertical, 6)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(Color(.controlBackgroundColor).opacity(0.5))
    }

    // MARK: - Actions View

    private var actionsView: some View {
        VStack(alignment: .leading, spacing: 0) {
            // Quick Connect button
            Button {
                openMainWindow()
                openWindow(id: "quick-connect")
                dismiss()
            } label: {
                Label("Quick Connect...", systemImage: "bolt.fill")
                    .frame(maxWidth: .infinity, alignment: .leading)
            }
            .buttonStyle(.plain)
            .padding(.horizontal, 16)
            .padding(.vertical, 8)
            .background(Color(.controlBackgroundColor))

            Divider()

            // Add Server button
            Button {
                openMainWindow()
                appState.showingAddServer = true
                dismiss()
            } label: {
                Label("Add Server...", systemImage: "plus")
                    .frame(maxWidth: .infinity, alignment: .leading)
            }
            .buttonStyle(.plain)
            .padding(.horizontal, 16)
            .padding(.vertical, 8)

            Divider()

            // Settings and Quit
            HStack {
                Button {
                    NSApp.sendAction(Selector(("showPreferencesWindow:")), to: nil, from: nil)
                    dismiss()
                } label: {
                    Label("Settings", systemImage: "gear")
                }
                .buttonStyle(.plain)

                Spacer()

                Button {
                    NSApplication.shared.terminate(nil)
                } label: {
                    Label("Quit", systemImage: "power")
                        .foregroundStyle(.red)
                }
                .buttonStyle(.plain)
            }
            .padding(.horizontal, 16)
            .padding(.vertical, 8)
        }
    }

    // MARK: - Computed Properties

    private var favoriteServers: [Server] {
        appState.servers.filter { $0.isFavorite }
            .sorted { ($0.lastConnected ?? Date.distantPast) > ($1.lastConnected ?? Date.distantPast) }
    }

    private var recentServers: [Server] {
        appState.servers
            .filter { $0.lastConnected != nil && !$0.isFavorite }
            .sorted { ($0.lastConnected ?? Date.distantPast) > ($1.lastConnected ?? Date.distantPast) }
    }

    private var otherServers: [Server] {
        appState.servers
            .filter { $0.lastConnected == nil && !$0.isFavorite }
            .sorted { $0.name < $1.name }
    }

    // MARK: - Helper Methods

    private func isConnected(_ server: Server) -> Bool {
        server.status == .connected
    }

    private func connect(to server: Server) {
        appState.connect(to: server)
        dismiss()
    }

    private func disconnect(from server: Server) {
        appState.disconnect(from: server)
    }

    private func openMainWindow() {
        NSApp.unhide(nil)
        NSApp.activate(ignoringOtherApps: true)

        // Bring main window to front
        if let window = NSApp.mainWindow {
            window.makeKeyAndOrderFront(nil)
        } else {
            // If no main window, try to find any EasySSH window
            NSApp.windows.first?.makeKeyAndOrderFront(nil)
        }
    }
}

// MARK: - Server Menu Row

struct ServerMenuRow: View {
    let server: Server
    let isConnected: Bool
    let onConnect: () -> Void
    let onDisconnect: () -> Void
    @EnvironmentObject var appState: AppState

    var body: some View {
        HStack(spacing: 12) {
            // Connection status indicator
            ZStack {
                Circle()
                    .fill(statusColor.opacity(0.2))
                    .frame(width: 8, height: 8)

                Circle()
                    .fill(statusColor)
                    .frame(width: 8, height: 8)
                    .overlay(
                        Circle()
                            .stroke(statusColor.opacity(0.3), lineWidth: 2)
                            .frame(width: 14, height: 14)
                    )
                    .opacity(isConnected ? 1 : 0)
            }
            .frame(width: 20)

            // Server info
            VStack(alignment: .leading, spacing: 2) {
                Text(server.name)
                    .font(.system(size: 13, weight: .medium))
                    .lineLimit(1)

                Text("\(server.username)@\(server.host):\(server.port)")
                    .font(.system(size: 11))
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
            }

            Spacer()

            // Connect/Disconnect button
            Button {
                if isConnected {
                    onDisconnect()
                } else {
                    onConnect()
                }
            } label: {
                Image(systemName: isConnected ? "xmark.circle.fill" : "arrow.right.circle.fill")
                    .font(.system(size: 18))
                    .foregroundStyle(isConnected ? .red : .accentColor)
            }
            .buttonStyle(.plain)
            .help(isConnected ? "Disconnect" : "Connect")
        }
        .padding(.horizontal, 16)
        .padding(.vertical, 8)
        .contentShape(Rectangle())
        .onTapGesture {
            if !isConnected {
                onConnect()
            }
        }
        .background(Color.clear)
        .contextMenu {
            ServerContextMenu(server: server, isConnected: isConnected)
        }
    }

    private var statusColor: Color {
        switch server.status {
        case .connected: return .green
        case .connecting: return .orange
        case .error: return .red
        default: return .gray
        }
    }
}

// MARK: - Server Context Menu

struct ServerContextMenu: View {
    let server: Server
    let isConnected: Bool
    @EnvironmentObject var appState: AppState
    @Environment(\.dismiss) private var dismiss

    var body: some View {
        Group {
            Button {
                if isConnected {
                    appState.disconnect(from: server)
                } else {
                    appState.connect(to: server)
                }
            } label: {
                Label(isConnected ? "Disconnect" : "Connect", systemImage: isConnected ? "xmark" : "arrow.right")
            }

            Divider()

            Button {
                appState.selectedServer = server
                openMainWindow()
                appState.editingServer = server
            } label: {
                Label("Edit Server...", systemImage: "pencil")
            }

            Button {
                appState.duplicate(server: server)
            } label: {
                Label("Duplicate", systemImage: "doc.on.doc")
            }

            Divider()

            Button {
                // Copy SSH command to clipboard
                let sshCommand = "ssh -p \(server.port) \(server.username)@\(server.host)"
                NSPasteboard.general.clearContents()
                NSPasteboard.general.setString(sshCommand, forType: .string)
            } label: {
                Label("Copy SSH Command", systemImage: "doc.on.clipboard")
            }

            Divider()

            Button(role: .destructive) {
                appState.serverToDelete = server
            } label: {
                Label("Delete...", systemImage: "trash")
            }
        }
    }

    private func openMainWindow() {
        NSApp.unhide(nil)
        NSApp.activate(ignoringOtherApps: true)
        NSApp.windows.first?.makeKeyAndOrderFront(nil)
    }
}

// MARK: - Connection Status Indicator

struct ConnectionStatusIndicator: View {
    let connected: Int
    let total: Int

    var body: some View {
        HStack(spacing: 4) {
            if connected > 0 {
                Circle()
                    .fill(Color.green)
                    .frame(width: 6, height: 6)

                Text("\(connected) connected")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            } else {
                Circle()
                    .fill(Color.gray)
                    .frame(width: 6, height: 6)

                Text("No active connections")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }

            Text("• \(total) servers")
                .font(.caption)
                .foregroundStyle(.secondary.opacity(0.7))
        }
    }
}
