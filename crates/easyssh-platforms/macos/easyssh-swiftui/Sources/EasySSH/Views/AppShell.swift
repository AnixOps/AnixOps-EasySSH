import SwiftUI
import SwiftData

/// Main app shell with NavigationSplitView and SwiftData integration
struct AppShell: View {
    @Environment(\.modelContext) private var modelContext
    @StateObject private var appState = AppState()
    @StateObject private var themeManager = ThemeManager()

    // Migration state
    @State private var showMigrationView = false
    @State private var migrationCompleted = false

    var body: some View {
        Group {
            if showMigrationView && !migrationCompleted {
                MigrationView {
                    migrationCompleted = true
                }
            } else {
                mainContent
            }
        }
        .onAppear {
            checkMigrationNeeded()
        }
    }

    private var mainContent: some View {
        NavigationSplitView {
            SidebarContainer()
                .navigationSplitViewColumnWidth(min: 220, ideal: 260, max: 350)
        } detail: {
            DetailContainer()
        }
        .sheet(isPresented: $appState.showingAddServer) {
            AddServerContainer()
                .environmentObject(appState)
        }
        .sheet(item: $appState.editingServer) { server in
            EditServerContainer(server: server)
                .environmentObject(appState)
        }
        .alert("Delete Server?", isPresented: .constant(appState.serverToDelete != nil), presenting: appState.serverToDelete) { server in
            Button("Cancel", role: .cancel) {
                appState.serverToDelete = nil
            }
            Button("Delete", role: .destructive) {
                Task {
                    await appState.deleteServer(server)
                }
            }
        } message: { server in
            Text("Are you sure you want to delete '\(server.name)'? This action cannot be undone.")
        }
        .task {
            // Initial data load from SwiftData
            appState.loadData()
        }
        .environmentObject(appState)
        .environmentObject(themeManager)
    }

    private func checkMigrationNeeded() {
        let migrationService = DataMigrationService.shared
        if migrationService.isMigrationNeeded {
            showMigrationView = true
        }
    }
}

// MARK: - Sidebar Container

struct SidebarContainer: View {
    @EnvironmentObject var appState: AppState
    @Environment(\.modelContext) private var modelContext
    @State private var showingNewGroup = false

    var body: some View {
        SidebarView()
            .toolbar {
                ToolbarItem(placement: .primaryAction) {
                    Button {
                        appState.showingAddServer = true
                    } label: {
                        Label("Add Server", systemImage: "plus")
                    }
                    .help("Add a new server (⌘N)")
                }

                ToolbarItem(placement: .automatic) {
                    Menu {
                        Button {
                            appState.showingAddServer = true
                        } label: {
                            Label("New Server...", systemImage: "server.rack")
                        }

                        Button {
                            showingNewGroup = true
                        } label: {
                            Label("New Group...", systemImage: "folder")
                        }

                        Divider()

                        Button {
                            appState.importSSHConfig()
                        } label: {
                            Label("Import from SSH Config", systemImage: "arrow.down.doc")
                        }

                        Button {
                            appState.exportAllData()
                        } label: {
                            Label("Export Servers...", systemImage: "square.and.arrow.up")
                        }

                        Divider()

                        Button {
                            SwiftDataService.shared.triggerCloudSync()
                        } label: {
                            Label("Sync Now", systemImage: "arrow.clockwise.icloud")
                        }
                    } label: {
                        Label("More", systemImage: "ellipsis.circle")
                    }
                }

                // Cloud sync status
                ToolbarItem(placement: .status) {
                    CloudSyncStatusView(status: appState.cloudSyncStatus)
                        .opacity(appState.cloudSyncStatus == .unknown ? 0 : 1)
                }
            }
            .sheet(isPresented: $showingNewGroup) {
                NewGroupView()
                    .environmentObject(appState)
            }
    }
}

// MARK: - Detail Container

struct DetailContainer: View {
    @EnvironmentObject var appState: AppState

    var body: some View {
        ZStack {
            if let server = appState.selectedServer {
                ServerDetailView(server: server)
                    .transition(.opacity.combined(with: .move(edge: .trailing)))
            } else {
                EmptyStateView()
            }
        }
        .animation(.smooth(duration: 0.2), value: appState.selectedServer?.id)
    }
}

// MARK: - Empty State

struct EmptyStateView: View {
    @EnvironmentObject var appState: AppState

    var body: some View {
        VStack(spacing: 24) {
            Spacer()

            // Animated icon
            ServerIconAnimation()

            VStack(spacing: 8) {
                Text("Welcome to EasySSH")
                    .font(.system(size: 28, weight: .semibold))
                    .foregroundStyle(.primary)

                Text("Select a server from the sidebar or add a new one to get started")
                    .font(.system(size: 14))
                    .foregroundStyle(.secondary)
                    .multilineTextAlignment(.center)
                    .frame(maxWidth: 320)
            }

            HStack(spacing: 16) {
                Button {
                    appState.showingAddServer = true
                } label: {
                    Label("Add Server", systemImage: "plus")
                        .fontWeight(.medium)
                }
                .buttonStyle(.borderedProminent)
                .controlSize(.large)

                Button {
                    appState.importSSHConfig()
                } label: {
                    Label("Import SSH Config", systemImage: "arrow.down.doc")
                }
                .buttonStyle(.bordered)
                .controlSize(.large)
            }
            .padding(.top, 16)

            Spacer()

            // Connection status footer
            ConnectionStatusBar()
                .padding(.bottom, 16)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background(.background)
    }
}

struct ServerIconAnimation: View {
    @State private var isAnimating = false

    var body: some View {
        ZStack {
            // Outer ring
            Circle()
                .stroke(Color.accentColor.opacity(0.2), lineWidth: 2)
                .frame(width: 120, height: 120)
                .scaleEffect(isAnimating ? 1.1 : 1.0)
                .opacity(isAnimating ? 0.5 : 1.0)

            // Middle ring
            Circle()
                .stroke(Color.accentColor.opacity(0.4), lineWidth: 2)
                .frame(width: 100, height: 100)
                .scaleEffect(isAnimating ? 1.05 : 1.0)

            // Inner content
            ZStack {
                Circle()
                    .fill(Color.accentColor.opacity(0.1))
                    .frame(width: 80, height: 80)

                Image(systemName: "terminal.fill")
                    .font(.system(size: 36))
                    .foregroundStyle(Color.accentColor)
            }
        }
        .onAppear {
            withAnimation(.easeInOut(duration: 2).repeatForever(autoreverses: true)) {
                isAnimating = true
            }
        }
    }
}

struct ConnectionStatusBar: View {
    @EnvironmentObject var appState: AppState

    var body: some View {
        HStack(spacing: 24) {
            StatusItem(
                icon: "server.rack",
                value: "\(appState.connectionStatus.totalServers)",
                label: "Servers"
            )

            StatusItem(
                icon: "link",
                value: "\(appState.connectionStatus.connectedSessions)",
                label: "Connected",
                color: .green
            )

            StatusItem(
                icon: "arrow.up.arrow.down",
                value: "\(appState.connectionStatus.activeTransfers)",
                label: "Transfers"
            )

            Spacer()

            ConnectionModeBadge()
        }
        .padding(.horizontal, 24)
        .padding(.vertical, 12)
        .background(.ultraThinMaterial)
        .clipShape(RoundedRectangle(cornerRadius: 12))
        .padding(.horizontal, 32)
    }
}

struct StatusItem: View {
    let icon: String
    let value: String
    let label: String
    var color: Color?

    var body: some View {
        HStack(spacing: 6) {
            Image(systemName: icon)
                .font(.system(size: 12))
                .foregroundStyle(color ?? .secondary)

            Text(value)
                .font(.system(size: 14, weight: .semibold))

            Text(label)
                .font(.system(size: 12))
                .foregroundStyle(.secondary)
        }
    }
}

struct ConnectionModeBadge: View {
    @EnvironmentObject var appState: AppState

    var body: some View {
        HStack(spacing: 4) {
            Circle()
                .fill(modeColor)
                .frame(width: 8, height: 8)

            Text(appState.connectionMode.rawValue)
                .font(.system(size: 11, weight: .medium))
                .foregroundStyle(.secondary)
        }
        .padding(.horizontal, 10)
        .padding(.vertical, 4)
        .background(modeColor.opacity(0.15))
        .clipShape(Capsule())
    }

    var modeColor: Color {
        switch appState.connectionMode {
        case .lite: return .blue
        case .standard: return .purple
        case .pro: return .orange
        }
    }
}

// MARK: - Theme Manager

@Observable
class ThemeManager {
    var accentColor: Color = .blue
    var useSystemAppearance = true
}

// MARK: - New Group View

struct NewGroupView: View {
    @EnvironmentObject var appState: AppState
    @Environment(\.dismiss) private var dismiss

    @State private var name = ""
    @State private var selectedColor: String? = "#45B7D1"

    let colorOptions = [
        "#FF6B6B", "#4ECDC4", "#45B7D1", "#96CEB4",
        "#FFEAA7", "#DDA0DD", "#98D8C8", "#F7DC6F"
    ]

    var body: some View {
        VStack(spacing: 20) {
            Text("New Group")
                .font(.headline)

            Form {
                TextField("Group Name", text: $name)
                    .textFieldStyle(.roundedBorder)

                VStack(alignment: .leading, spacing: 8) {
                    Text("Color")
                        .font(.caption)
                        .foregroundStyle(.secondary)

                    LazyVGrid(columns: [GridItem(.adaptive(minimum: 32))], spacing: 8) {
                        ForEach(colorOptions, id: \.self) { color in
                            Circle()
                                .fill(Color(hex: color))
                                .frame(width: 32, height: 32)
                                .overlay(
                                    Circle()
                                        .stroke(Color.white, lineWidth: selectedColor == color ? 3 : 0)
                                )
                                .onTapGesture {
                                    selectedColor = color
                                }
                        }
                    }
                }
            }
            .formStyle(.grouped)

            HStack {
                Button("Cancel") {
                    dismiss()
                }
                .buttonStyle(.bordered)

                Button("Create") {
                    Task {
                        try? await appState.addGroup(name: name, color: selectedColor)
                        dismiss()
                    }
                }
                .buttonStyle(.borderedProminent)
                .disabled(name.isEmpty)
            }
        }
        .padding()
        .frame(width: 320)
    }
}

// MARK: - Color Extension

extension Color {
    init(hex: String) {
        let hex = hex.trimmingCharacters(in: CharacterSet.alphanumerics.inverted)
        var int: UInt64 = 0
        Scanner(string: hex).scanHexInt64(&int)
        let a, r, g, b: UInt64
        switch hex.count {
        case 3: // RGB (12-bit)
            (a, r, g, b) = (255, (int >> 8) * 17, (int >> 4 & 0xF) * 17, (int & 0xF) * 17)
        case 6: // RGB (24-bit)
            (a, r, g, b) = (255, int >> 16, int >> 8 & 0xFF, int & 0xFF)
        case 8: // ARGB (32-bit)
            (a, r, g, b) = (int >> 24, int >> 16 & 0xFF, int >> 8 & 0xFF, int & 0xFF)
        default:
            (a, r, g, b) = (1, 1, 1, 0)
        }

        self.init(
            .sRGB,
            red: Double(r) / 255,
            green: Double(g) / 255,
            blue: Double(b) / 255,
            opacity: Double(a) / 255
        )
    }
}
