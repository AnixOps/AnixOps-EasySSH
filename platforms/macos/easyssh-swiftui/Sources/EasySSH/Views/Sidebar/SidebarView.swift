import SwiftUI

/// Main sidebar view with search, groups, and server list
struct SidebarView: View {
    @EnvironmentObject var appState: AppState
    @State private var selectedSection: SidebarSection = .all
    @State private var expandedGroups: Set<String> = []
    @State private var showingSearchScope = false

    enum SidebarSection: String, CaseIterable {
        case all = "All Servers"
        case recent = "Recent"
        case favorites = "Favorites"
        case connected = "Connected"

        var icon: String {
            switch self {
            case .all: return "server.rack"
            case .recent: return "clock"
            case .favorites: return "star.fill"
            case .connected: return "link"
            }
        }
    }

    var body: some View {
        VStack(spacing: 0) {
            // Search bar
            SearchBar(
                text: $appState.searchText,
                scope: $appState.selectedGroupFilter,
                groups: appState.groups
            )
            .padding(.horizontal, 12)
            .padding(.vertical, 8)

            // Section picker
            SectionPicker(selected: $selectedSection)
                .padding(.horizontal, 12)
                .padding(.bottom, 8)

            Divider()

            // Content list
            List(selection: $appState.selectedServer) {
                // Quick actions
                if appState.searchText.isEmpty {
                    QuickActionsSection()
                        .listRowSeparator(.hidden)
                        .listRowInsets(EdgeInsets(top: 4, leading: 12, bottom: 4, trailing: 12))
                }

                // Servers section
                ServersSection(
                    servers: filteredServersForSection,
                    selectedSection: selectedSection
                )

                // Groups section
                if appState.searchText.isEmpty && !appState.groups.isEmpty {
                    GroupsSection(
                        groups: appState.groups,
                        servers: appState.servers,
                        expandedGroups: $expandedGroups
                    )
                }
            }
            .listStyle(.sidebar)
            .alternatingRowBackgrounds(.enabled)
        }
        .navigationTitle("EasySSH")
    }

    private var filteredServersForSection: [Server] {
        let base = appState.filteredServers

        switch selectedSection {
        case .all:
            return base
        case .recent:
            return base.filter { $0.lastConnected != nil }
                .sorted { ($0.lastConnected ?? Date.distantPast) > ($1.lastConnected ?? Date.distantPast) }
        case .favorites:
            return base.filter { $0.isFavorite }
        case .connected:
            return base.filter { $0.status == .connected }
        }
    }
}

// MARK: - Search Bar

struct SearchBar: View {
    @Binding var text: String
    @Binding var scope: String?
    let groups: [ServerGroup]
    @State private var isFocused = false

    var body: some View {
        HStack(spacing: 8) {
            Image(systemName: "magnifyingglass")
                .font(.system(size: 13))
                .foregroundStyle(.secondary)

            TextField("Search servers...", text: $text)
                .textFieldStyle(.plain)
                .font(.system(size: 13))

            if !text.isEmpty {
                Button {
                    text = ""
                } label: {
                    Image(systemName: "xmark.circle.fill")
                        .font(.system(size: 14))
                        .foregroundStyle(.secondary)
                }
                .buttonStyle(.plain)
            }

            // Scope picker
            if !groups.isEmpty {
                Menu {
                    Button {
                        scope = nil
                    } label: {
                        Label("All Groups", systemImage: scope == nil ? "checkmark" : "")
                    }

                    Divider()

                    ForEach(groups) { group in
                        Button {
                            scope = group.id
                        } label: {
                            Label(
                                group.name,
                                systemImage: scope == group.id ? "checkmark" : "folder"
                            )
                        }
                    }
                } label: {
                    Image(systemName: "line.3.horizontal.decrease.circle")
                        .font(.system(size: 13))
                        .foregroundStyle(scope != nil ? .accentColor : .secondary)
                }
                .menuStyle(.borderlessButton)
            }
        }
        .padding(.horizontal, 10)
        .padding(.vertical, 6)
        .background(.quaternary.opacity(0.3))
        .clipShape(RoundedRectangle(cornerRadius: 8))
    }
}

// MARK: - Section Picker

struct SectionPicker: View {
    @Binding var selected: SidebarView.SidebarSection

    var body: some View {
        ScrollView(.horizontal, showsIndicators: false) {
            HStack(spacing: 4) {
                ForEach(SidebarView.SidebarSection.allCases, id: \.self) { section in
                    SectionButton(
                        section: section,
                        isSelected: selected == section
                    ) {
                        selected = section
                    }
                }
            }
            .padding(.horizontal, 4)
        }
    }
}

struct SectionButton: View {
    let section: SidebarView.SidebarSection
    let isSelected: Bool
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            HStack(spacing: 4) {
                Image(systemName: section.icon)
                    .font(.system(size: 11))
                Text(section.rawValue)
                    .font(.system(size: 11, weight: isSelected ? .semibold : .regular))
            }
            .padding(.horizontal, 10)
            .padding(.vertical, 5)
            .background(isSelected ? Color.accentColor : Color.clear)
            .foregroundStyle(isSelected ? .white : .primary)
            .clipShape(Capsule())
        }
        .buttonStyle(.plain)
    }
}

// MARK: - Quick Actions

struct QuickActionsSection: View {
    @EnvironmentObject var appState: AppState

    var body: some View {
        Section {
            Button {
                // Quick connect dialog
            } label: {
                Label("Quick Connect", systemImage: "bolt.fill")
                    .font(.system(size: 13))
            }
            .buttonStyle(.plain)
        }
        .listRowBackground(Color.clear)
    }
}

// MARK: - Servers Section

struct ServersSection: View {
    let servers: [Server]
    let selectedSection: SidebarView.SidebarSection

    var body: some View {
        Section {
            if servers.isEmpty {
                EmptyServersView(section: selectedSection)
            } else {
                ForEach(servers) { server in
                    ServerRow(server: server)
                        .tag(server)
                }
            }
        } header: {
            if !servers.isEmpty {
                Text("Servers")
                    .font(.system(size: 11, weight: .medium))
                    .foregroundStyle(.secondary)
                    .textCase(nil)
            }
        }
    }
}

struct EmptyServersView: View {
    let section: SidebarView.SidebarSection

    var body: some View {
        VStack(spacing: 8) {
            Image(systemName: "server.rack")
                .font(.system(size: 24))
                .foregroundStyle(.tertiary)

            Text(emptyMessage)
                .font(.system(size: 12))
                .foregroundStyle(.secondary)
                .multilineTextAlignment(.center)
        }
        .frame(maxWidth: .infinity)
        .padding(.vertical, 32)
        .listRowBackground(Color.clear)
    }

    private var emptyMessage: String {
        switch section {
        case .all:
            return "No servers yet.\nAdd one to get started."
        case .recent:
            return "No recent connections"
        case .favorites:
            return "No favorite servers"
        case .connected:
            return "No active connections"
        }
    }
}

// MARK: - Server Row

struct ServerRow: View {
    let server: Server
    @State private var isHovered = false
    @State private var showingQuickActions = false

    var body: some View {
        HStack(spacing: 10) {
            // Status indicator
            ZStack {
                Circle()
                    .fill(statusColor.opacity(0.15))
                    .frame(width: 28, height: 28)

                Image(systemName: statusIcon)
                    .font(.system(size: 12))
                    .foregroundStyle(statusColor)
            }

            // Server info
            VStack(alignment: .leading, spacing: 2) {
                Text(server.name)
                    .font(.system(size: 13, weight: .medium))
                    .lineLimit(1)

                HStack(spacing: 4) {
                    Text(server.username)
                        .font(.system(size: 11))
                        .foregroundStyle(.secondary)

                    Text("@")
                        .font(.system(size: 11))
                        .foregroundStyle(.tertiary)

                    Text(server.host)
                        .font(.system(size: 11))
                        .foregroundStyle(.secondary)
                        .lineLimit(1)
                }
            }

            Spacer()

            // Quick actions on hover
            if isHovered {
                HStack(spacing: 4) {
                    Button {
                        // Connect
                    } label: {
                        Image(systemName: "play.fill")
                            .font(.system(size: 10))
                    }
                    .buttonStyle(.borderless)
                    .help("Connect")

                    if server.isFavorite {
                        Image(systemName: "star.fill")
                            .font(.system(size: 10))
                            .foregroundStyle(.yellow)
                    }
                }
                .transition(.opacity)
            }
        }
        .padding(.vertical, 4)
        .contentShape(Rectangle())
        .onHover { hovering in
            withAnimation(.easeInOut(duration: 0.15)) {
                isHovered = hovering
            }
        }
        .contextMenu {
            ServerContextMenu(server: server)
        }
    }

    private var statusIcon: String {
        switch server.status {
        case .connected:
            return "checkmark.circle.fill"
        case .disconnected:
            return "circle"
        case .error:
            return "exclamationmark.triangle.fill"
        case .unknown:
            return "questionmark.circle"
        case .connecting:
            return "arrow.clockwise"
        }
    }

    private var statusColor: Color {
        switch server.status {
        case .connected:
            return .green
        case .disconnected:
            return .gray
        case .error:
            return .red
        case .unknown:
            return .orange
        case .connecting:
            return .blue
        }
    }
}

// MARK: - Context Menu

struct ServerContextMenu: View {
    @EnvironmentObject var appState: AppState
    let server: Server

    var body: some View {
        Button {
            appState.connect(to: server)
        } label: {
            Label("Connect", systemImage: "play")
        }

        Button {
            // SFTP
        } label: {
            Label("Open SFTP", systemImage: "folder")
        }

        Divider()

        Button {
            appState.editingServer = server
        } label: {
            Label("Edit", systemImage: "pencil")
        }

        Button {
            appState.duplicate(server: server)
        } label: {
            Label("Duplicate", systemImage: "doc.on.doc")
        }

        Divider()

        Button {
            // Toggle favorite
        } label: {
            Label(server.isFavorite ? "Remove Favorite" : "Add to Favorites",
                  systemImage: server.isFavorite ? "star.slash" : "star")
        }

        Divider()

        Button(role: .destructive) {
            appState.serverToDelete = server
        } label: {
            Label("Delete", systemImage: "trash")
        }
    }
}

// MARK: - Groups Section

struct GroupsSection: View {
    let groups: [ServerGroup]
    let servers: [Server]
    @Binding var expandedGroups: Set<String>

    var body: some View {
        Section {
            ForEach(groups) { group in
                GroupRow(
                    group: group,
                    servers: servers,
                    isExpanded: expandedGroups.contains(group.id)
                ) {
                    if expandedGroups.contains(group.id) {
                        expandedGroups.remove(group.id)
                    } else {
                        expandedGroups.insert(group.id)
                    }
                }
            }
        } header: {
            Text("Groups")
                .font(.system(size: 11, weight: .medium))
                .foregroundStyle(.secondary)
                .textCase(nil)
        }
    }
}

struct GroupRow: View {
    let group: ServerGroup
    let servers: [Server]
    let isExpanded: Bool
    let toggle: () -> Void

    var groupServers: [Server] {
        servers.filter { $0.groupId == group.id }
            .sorted { $0.name < $1.name }
    }

    var body: some View {
        DisclosureGroup(isExpanded: $isExpanded) {
            ForEach(groupServers) { server in
                ServerRow(server: server)
                    .padding(.leading, 8)
                    .tag(server)
            }
        } label: {
            HStack(spacing: 6) {
                Image(systemName: isExpanded ? "folder.fill" : "folder")
                    .font(.system(size: 14))
                    .foregroundStyle(.accentColor)

                Text(group.name)
                    .font(.system(size: 13, weight: .medium))

                Spacer()

                Text("\(groupServers.count)")
                    .font(.system(size: 11))
                    .foregroundStyle(.secondary)
                    .padding(.horizontal, 6)
                    .padding(.vertical, 2)
                    .background(.secondary.opacity(0.1))
                    .clipShape(Capsule())
            }
        }
        .disclosureGroupStyle(.standard)
    }
}
