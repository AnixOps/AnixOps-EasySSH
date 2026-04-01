import SwiftUI

/// Detailed server view with connection controls and info
struct ServerDetailView: View {
    let server: Server
    @EnvironmentObject var appState: AppState
    @State private var showingPasswordPrompt = false
    @State private var password = ""
    @State private var showingTerminal = false
    @State private var showingSFTP = false
    @State private var isConnecting = false
    @State private var activeTab: DetailTab = .overview

    enum DetailTab: String, CaseIterable {
        case overview = "Overview"
        case terminal = "Terminal"
        case files = "Files"
        case logs = "Logs"
        case settings = "Settings"
    }

    var body: some View {
        VStack(spacing: 0) {
            // Header
            ServerDetailHeader(server: server)

            Divider()

            // Tab navigation for Standard/Pro mode
            if appState.connectionMode != .lite {
                DetailTabBar(selected: $activeTab)
                Divider()
            }

            // Content based on tab
            Group {
                switch activeTab {
                case .overview:
                    OverviewTab(server: server)
                case .terminal:
                    TerminalPlaceholderView()
                case .files:
                    SFTPPlaceholderView()
                case .logs:
                    LogsView()
                case .settings:
                    ServerSettingsView(server: server)
                }
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity)
        }
        .background(.background)
        .alert("Enter Password", isPresented: $showingPasswordPrompt) {
            SecureField("Password", text: $password)
            Button("Cancel", role: .cancel) { }
            Button("Connect") {
                connectWithPassword()
            }
        } message: {
            Text("Enter password for \(server.username)@\(server.host)")
        }
    }

    private func connectWithPassword() {
        isConnecting = true
        // Store password and connect
        Task {
            if !password.isEmpty {
                try? await appState.updateServer(server, password: password)
            }
            await MainActor.run {
                appState.connect(to: server)
                isConnecting = false
                password = ""
            }
        }
    }
}

// MARK: - Detail Header

struct ServerDetailHeader: View {
    let server: Server
    @EnvironmentObject var appState: AppState
    @State private var isHovered = false

    var body: some View {
        HStack(spacing: 20) {
            // Server icon and status
            ServerIcon(server: server)

            // Server info
            VStack(alignment: .leading, spacing: 4) {
                Text(server.name)
                    .font(.system(size: 24, weight: .bold))

                HStack(spacing: 8) {
                    ConnectionBadge(status: server.status)

                    Text("\(server.username)@\(server.host):")
                        .font(.system(size: 14))
                        .foregroundStyle(.secondary)

                    Text("\(server.port)")
                        .font(.system(size: 14, weight: .medium))
                        .foregroundStyle(.secondary)
                        .padding(.horizontal, 6)
                        .padding(.vertical, 2)
                        .background(.secondary.opacity(0.1))
                        .clipShape(RoundedRectangle(cornerRadius: 4))
                }
            }

            Spacer()

            // Action buttons
            HStack(spacing: 12) {
                if server.status == .connected {
                    Button {
                        appState.disconnect(from: server)
                    } label: {
                        Label("Disconnect", systemImage: "stop.fill")
                    }
                    .buttonStyle(.bordered)
                    .tint(.red)
                    .controlSize(.large)
                } else {
                    Button {
                        if server.authType == .password {
                            // Will trigger password prompt
                        }
                        appState.connect(to: server)
                    } label: {
                        if server.status == .connecting {
                            ProgressView()
                                .controlSize(.small)
                                .scaleEffect(0.8)
                        } else {
                            Label("Connect", systemImage: "play.fill")
                        }
                    }
                    .buttonStyle(.borderedProminent)
                    .controlSize(.large)
                    .disabled(server.status == .connecting)
                }

                Menu {
                    Button {
                        // Copy SSH command
                        let command = "ssh -p \(server.port) \(server.username)@\(server.host)"
                        NSPasteboard.general.clearContents()
                        NSPasteboard.general.setString(command, forType: .string)
                    } label: {
                        Label("Copy SSH Command", systemImage: "doc.on.doc")
                    }

                    Divider()

                    Button {
                        appState.editingServer = server
                    } label: {
                        Label("Edit Server", systemImage: "pencil")
                    }

                    Button {
                        appState.duplicate(server: server)
                    } label: {
                        Label("Duplicate", systemImage: "doc.on.doc")
                    }

                    Divider()

                    Button(role: .destructive) {
                        appState.serverToDelete = server
                    } label: {
                        Label("Delete Server", systemImage: "trash")
                    }
                } label: {
                    Image(systemName: "ellipsis.circle")
                        .font(.system(size: 18))
                }
                .menuStyle(.borderlessButton)
            }
        }
        .padding(.horizontal, 24)
        .padding(.vertical, 20)
        .background(.ultraThinMaterial)
    }
}

struct ServerIcon: View {
    let server: Server

    var body: some View {
        ZStack {
            RoundedRectangle(cornerRadius: 16)
                .fill(statusColor.opacity(0.15))
                .frame(width: 64, height: 64)

            Image(systemName: "server.rack")
                .font(.system(size: 28))
                .foregroundStyle(statusColor)

            // Status dot
            Circle()
                .fill(statusColor)
                .frame(width: 12, height: 12)
                .overlay(
                    Circle()
                        .stroke(.background, lineWidth: 2)
                )
                .offset(x: 20, y: 20)
        }
    }

    private var statusColor: Color {
        switch server.status {
        case .connected: return .green
        case .disconnected: return .gray
        case .error: return .red
        case .unknown: return .orange
        case .connecting: return .blue
        }
    }
}

struct ConnectionBadge: View {
    let status: ServerStatus

    var body: some View {
        HStack(spacing: 4) {
            Circle()
                .fill(statusColor)
                .frame(width: 6, height: 6)

            Text(statusText)
                .font(.system(size: 11, weight: .medium))
        }
        .padding(.horizontal, 8)
        .padding(.vertical, 3)
        .background(statusColor.opacity(0.15))
        .foregroundStyle(statusColor)
        .clipShape(Capsule())
    }

    private var statusColor: Color {
        switch status {
        case .connected: return .green
        case .disconnected: return .gray
        case .error: return .red
        case .unknown: return .orange
        case .connecting: return .blue
        }
    }

    private var statusText: String {
        switch status {
        case .connected: return "Connected"
        case .disconnected: return "Disconnected"
        case .error: return "Error"
        case .unknown: return "Unknown"
        case .connecting: return "Connecting..."
        }
    }
}

// MARK: - Detail Tab Bar

struct DetailTabBar: View {
    @Binding var selected: ServerDetailView.DetailTab

    var body: some View {
        HStack(spacing: 0) {
            ForEach(ServerDetailView.DetailTab.allCases, id: \.self) { tab in
                TabButton(
                    tab: tab,
                    isSelected: selected == tab
                ) {
                    selected = tab
                }
            }

            Spacer()
        }
        .padding(.horizontal, 24)
        .padding(.vertical, 8)
    }
}

struct TabButton: View {
    let tab: ServerDetailView.DetailTab
    let isSelected: Bool
    let action: () -> Void

    var icon: String {
        switch tab {
        case .overview: return "info.circle"
        case .terminal: return "terminal"
        case .files: return "folder"
        case .logs: return "doc.text"
        case .settings: return "gear"
        }
    }

    var body: some View {
        Button(action: action) {
            VStack(spacing: 4) {
                Image(systemName: icon)
                    .font(.system(size: 16))

                Text(tab.rawValue)
                    .font(.system(size: 11, weight: isSelected ? .semibold : .regular))
            }
            .padding(.horizontal, 16)
            .padding(.vertical, 8)
            .foregroundStyle(isSelected ? .accentColor : .secondary)
            .background(isSelected ? Color.accentColor.opacity(0.1) : Color.clear)
            .clipShape(RoundedRectangle(cornerRadius: 8))
        }
        .buttonStyle(.plain)
    }
}

// MARK: - Overview Tab

struct OverviewTab: View {
    let server: Server
    @EnvironmentObject var appState: AppState

    var body: some View {
        ScrollView {
            VStack(spacing: 24) {
                // Connection info card
                ConnectionInfoCard(server: server)

                // Tags section
                if !server.tags.isEmpty {
                    TagsSection(tags: server.tags)
                }

                // Notes section
                if !server.notes.isEmpty {
                    NotesSection(notes: server.notes)
                }

                // Connection history
                ConnectionHistorySection()

                Spacer()
            }
            .padding(24)
        }
    }
}

struct ConnectionInfoCard: View {
    let server: Server

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            Text("Connection Details")
                .font(.system(size: 16, weight: .semibold))

            LazyVGrid(columns: [
                GridItem(.flexible()),
                GridItem(.flexible())
            ], spacing: 16) {
                InfoItem(label: "Host", value: server.host)
                InfoItem(label: "Port", value: "\(server.port)")
                InfoItem(label: "Username", value: server.username)
                InfoItem(label: "Authentication", value: server.authType.displayName)
            }

            if let lastConnected = server.lastConnected {
                Divider()

                HStack {
                    InfoItem(
                        label: "Last Connected",
                        value: lastConnected.formatted(date: .abbreviated, time: .shortened)
                    )

                    Spacer()
                }
            }
        }
        .padding(20)
        .background(.quaternary.opacity(0.3))
        .clipShape(RoundedRectangle(cornerRadius: 12))
    }
}

struct InfoItem: View {
    let label: String
    let value: String

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(label)
                .font(.system(size: 11))
                .foregroundStyle(.secondary)

            Text(value)
                .font(.system(size: 13, weight: .medium))
                .textSelection(.enabled)
        }
    }
}

struct TagsSection: View {
    let tags: [String]

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("Tags")
                .font(.system(size: 16, weight: .semibold))

            FlowLayout(spacing: 8) {
                ForEach(tags, id: \.self) { tag in
                    TagView(tag: tag)
                }
            }
        }
        .padding(20)
        .background(.quaternary.opacity(0.3))
        .clipShape(RoundedRectangle(cornerRadius: 12))
    }
}

struct TagView: View {
    let tag: String

    var body: some View {
        Text(tag)
            .font(.system(size: 12))
            .padding(.horizontal, 10)
            .padding(.vertical, 4)
            .background(Color.accentColor.opacity(0.15))
            .foregroundStyle(.accentColor)
            .clipShape(Capsule())
    }
}

struct NotesSection: View {
    let notes: String

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("Notes")
                .font(.system(size: 16, weight: .semibold))

            Text(notes)
                .font(.system(size: 13))
                .foregroundStyle(.secondary)
                .textSelection(.enabled)
        }
        .padding(20)
        .background(.quaternary.opacity(0.3))
        .clipShape(RoundedRectangle(cornerRadius: 12))
        .frame(maxWidth: .infinity, alignment: .leading)
    }
}

struct ConnectionHistorySection: View {
    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("Connection History")
                .font(.system(size: 16, weight: .semibold))

            HStack {
                Image(systemName: "clock.arrow.circlepath")
                    .font(.system(size: 32))
                    .foregroundStyle(.tertiary)

                Text("Connection history will appear here")
                    .font(.system(size: 13))
                    .foregroundStyle(.secondary)
            }
            .frame(maxWidth: .infinity, alignment: .center)
            .padding(.vertical, 32)
        }
        .padding(20)
        .background(.quaternary.opacity(0.3))
        .clipShape(RoundedRectangle(cornerRadius: 12))
    }
}

// MARK: - Placeholder Views

struct TerminalPlaceholderView: View {
    var body: some View {
        VStack(spacing: 16) {
            Image(systemName: "terminal")
                .font(.system(size: 48))
                .foregroundStyle(.tertiary)

            Text("Embedded Terminal")
                .font(.system(size: 18, weight: .semibold))

            Text("Connect to this server to open an embedded terminal session")
                .font(.system(size: 13))
                .foregroundStyle(.secondary)
                .multilineTextAlignment(.center)
                .frame(maxWidth: 300)

            Button("Connect") {
                // Connect action
            }
            .buttonStyle(.borderedProminent)
            .controlSize(.large)
        }
    }
}

struct SFTPPlaceholderView: View {
    var body: some View {
        VStack(spacing: 16) {
            Image(systemName: "folder")
                .font(.system(size: 48))
                .foregroundStyle(.tertiary)

            Text("SFTP File Manager")
                .font(.system(size: 18, weight: .semibold))

            Text("Connect to this server to browse and transfer files")
                .font(.system(size: 13))
                .foregroundStyle(.secondary)
                .multilineTextAlignment(.center)
                .frame(maxWidth: 300)
        }
    }
}

struct LogsView: View {
    var body: some View {
        VStack(spacing: 16) {
            Image(systemName: "doc.text")
                .font(.system(size: 48))
                .foregroundStyle(.tertiary)

            Text("Session Logs")
                .font(.system(size: 18, weight: .semibold))

            Text("Command history and session logs will appear here")
                .font(.system(size: 13))
                .foregroundStyle(.secondary)
        }
    }
}

struct ServerSettingsView: View {
    let server: Server
    @State private var launchOnStartup = false
    @State private var keepAlive = false
    @State private var autoReconnect = false

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 24) {
                Form {
                    Section("Connection Preferences") {
                        Toggle("Keep connection alive", isOn: $keepAlive)
                        Toggle("Auto-reconnect on disconnect", isOn: $autoReconnect)
                    }

                    Section("Startup") {
                        Toggle("Connect on app launch", isOn: $launchOnStartup)
                    }

                    Section("Notifications") {
                        Toggle("Notify on connection events", isOn: .constant(true))
                        Toggle("Notify on transfer complete", isOn: .constant(true))
                    }
                }
                .formStyle(.grouped)
                .frame(maxWidth: 500)
            }
            .padding(24)
        }
    }
}

// MARK: - Flow Layout

struct FlowLayout: Layout {
    var spacing: CGFloat = 8

    func sizeThatFits(proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) -> CGSize {
        let result = FlowResult(in: proposal.width ?? 0, subviews: subviews, spacing: spacing)
        return result.size
    }

    func placeSubviews(in bounds: CGRect, proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) {
        let result = FlowResult(in: bounds.width, subviews: subviews, spacing: spacing)
        for (index, subview) in subviews.enumerated() {
            subview.place(at: CGPoint(x: result.positions[index].x + bounds.minX,
                                      y: result.positions[index].y + bounds.minY),
                         proposal: .unspecified)
        }
    }

    struct FlowResult {
        var size: CGSize = .zero
        var positions: [CGPoint] = []

        init(in maxWidth: CGFloat, subviews: Subviews, spacing: CGFloat) {
            var x: CGFloat = 0
            var y: CGFloat = 0
            var rowHeight: CGFloat = 0

            for subview in subviews {
                let size = subview.sizeThatFits(.unspecified)

                if x + size.width > maxWidth && x > 0 {
                    x = 0
                    y += rowHeight + spacing
                    rowHeight = 0
                }

                positions.append(CGPoint(x: x, y: y))
                rowHeight = max(rowHeight, size.height)
                x += size.width + spacing
            }

            self.size = CGSize(width: maxWidth, height: y + rowHeight)
        }
    }
}
