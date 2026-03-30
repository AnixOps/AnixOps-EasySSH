import SwiftUI

struct ServerSidebar: View {
    let servers: [Server]
    let groups: [ServerGroup]
    @Binding var selectedServer: Server?
    @Binding var searchText: String

    var body: some View {
        List(selection: $selectedServer) {
            Section("Quick Connect") {
                Button(action: { /* Quick connect action */ }) {
                    Label("Quick Connect", systemImage: "bolt.fill")
                }
            }

            Section("Servers") {
                ForEach(servers) { server in
                    ServerRow(server: server)
                        .tag(server)
                }
            }

            if !groups.isEmpty {
                Section("Groups") {
                    ForEach(groups) { group in
                        GroupRow(group: group, servers: servers)
                    }
                }
            }
        }
        .searchable(text: $searchText, prompt: "Search servers...")
        .navigationTitle("EasySSH")
    }
}

struct ServerRow: View {
    let server: Server

    var body: some View {
        HStack {
            Image(systemName: statusIcon)
                .foregroundStyle(statusColor)

            VStack(alignment: .leading, spacing: 2) {
                Text(server.name)
                    .fontWeight(.medium)
                Text("\(server.username)@\(server.host)")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }

            Spacer()
        }
        .padding(.vertical, 4)
    }

    var statusIcon: String {
        switch server.status {
        case .connected: return "circle.fill"
        case .disconnected: return "circle"
        case .error: return "exclamationmark.triangle.fill"
        case .unknown: return "questionmark.circle"
        }
    }

    var statusColor: Color {
        switch server.status {
        case .connected: return .green
        case .disconnected: return .gray
        case .error: return .red
        case .unknown: return .orange
        }
    }
}

struct GroupRow: View {
    let group: ServerGroup
    let servers: [Server]

    var body: some View {
        DisclosureGroup {
            ForEach(group.servers.compactMap { id in
                servers.first { $0.id == id }
            }) { server in
                ServerRow(server: server)
                    .padding(.leading)
            }
        } label: {
            Label(group.name, systemImage: "folder.fill")
        }
    }
}

struct ServerDetailView: View {
    let server: Server
    @EnvironmentObject var appState: AppState
    @State private var showingEditSheet = false

    var body: some View {
        VStack(spacing: 20) {
            // Header
            HStack {
                VStack(alignment: .leading, spacing: 4) {
                    Text(server.name)
                        .font(.largeTitle)
                        .fontWeight(.bold)

                    HStack {
                        Text("\(server.username)@\(server.host):")
                            .font(.title3)
                            .foregroundStyle(.secondary)
                        Text("\(server.port)")
                            .font(.title3)
                            .foregroundStyle(.secondary)
                    }
                }

                Spacer()

                // Status indicator
                HStack {
                    Circle()
                        .fill(statusColor)
                        .frame(width: 10, height: 10)
                    Text(server.status.rawValue.capitalized)
                        .foregroundStyle(.secondary)
                }
            }

            Divider()

            // Connection actions
            HStack(spacing: 16) {
                Button("Connect (Native Terminal)") {
                    appState.connect(to: server)
                }
                .buttonStyle(.borderedProminent)
                .controlSize(.large)

                Button("Connect (Embedded)") {
                    // TODO: Embedded terminal for Standard/Pro mode
                }
                .buttonStyle(.bordered)
                .controlSize(.large)
                .disabled(true) // Disabled in Lite mode

                Spacer()

                Button("Edit") {
                    showingEditSheet = true
                }
                .buttonStyle(.bordered)
                .controlSize(.regular)
            }

            Divider()

            // Server details
            Form {
                Section("Connection Details") {
                    LabeledContent("Authentication", value: server.authType.rawValue.capitalized)
                    LabeledContent("Last Connected", value: "Never")
                }
            }
            .formStyle(.grouped)

            Spacer()
        }
        .padding()
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        .sheet(isPresented: $showingEditSheet) {
            EditServerView(server: server)
        }
    }

    var statusColor: Color {
        switch server.status {
        case .connected: return .green
        case .disconnected: return .gray
        case .error: return .red
        case .unknown: return .orange
        }
    }
}
