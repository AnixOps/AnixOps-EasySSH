import SwiftUI
import SwiftData
import Combine

/// Main application state managing all global state with SwiftData
@MainActor
@Observable
class AppState {
    // MARK: - Published Properties

    /// All servers (loaded from SwiftData)
    var servers: [Server] = []

    /// Server groups
    var groups: [ServerGroup] = []

    /// Currently selected server
    var selectedServer: Server? {
        didSet {
            if let server = selectedServer {
                UserDefaults.standard.set(server.id, forKey: "lastSelectedServer")
            }
        }
    }

    /// Active SSH sessions
    var activeSessions: [String: SSHSession] = [:]

    /// Connection mode
    var connectionMode: ConnectionMode = .lite {
        didSet {
            UserDefaults.standard.set(connectionMode.rawValue, forKey: "connectionMode")
        }
    }

    /// Sidebar visibility
    var sidebarVisibility: SidebarVisibility = .automatic {
        didSet {
            UserDefaults.standard.set(sidebarVisibility == .visible, forKey: "sidebarVisible")
        }
    }

    /// UI States
    var showingAddServer = false
    var editingServer: Server? = nil
    var serverToDelete: Server? = nil
    var showingGroupManager = false

    /// Search and filter
    var searchText = ""
    var selectedGroupFilter: String? = nil
    var favoritesOnly: Bool = false

    /// Cloud sync status
    var cloudSyncStatus: CloudSyncStatus = .unknown

    // MARK: - Services

    private let swiftDataService = SwiftDataService.shared
    private let keychainService = KeychainService.shared
    private var cancellables = Set<AnyCancellable>()

    // MARK: - SwiftData Models (for binding)

    private var serverModels: [ServerModel] = []
    private var groupModels: [ServerGroupModel] = []

    // MARK: - Computed Properties

    var filteredServers: [Server] {
        var filtered = servers

        // Apply search filter
        if !searchText.isEmpty {
            filtered = filtered.filter { server in
                server.name.localizedCaseInsensitiveContains(searchText) ||
                server.host.localizedCaseInsensitiveContains(searchText) ||
                server.username.localizedCaseInsensitiveContains(searchText) ||
                server.tags.contains { $0.localizedCaseInsensitiveContains(searchText) }
            }
        }

        // Apply group filter
        if let groupId = selectedGroupFilter {
            filtered = filtered.filter { $0.groupId == groupId }
        }

        // Apply favorites filter
        if favoritesOnly {
            filtered = filtered.filter { $0.isFavorite }
        }

        return filtered.sorted { $0.name < $1.name }
    }

    var connectionStatus: ConnectionStatus {
        let connected = activeSessions.values.filter { $0.isConnected }.count
        return ConnectionStatus(
            totalServers: servers.count,
            connectedSessions: connected,
            activeTransfers: 0
        )
    }

    // MARK: - Initialization

    init() {
        loadPreferences()
        loadData()
        setupNotifications()
        setupCloudSyncMonitoring()
    }

    // MARK: - Setup

    private func loadPreferences() {
        if let savedMode = UserDefaults.standard.string(forKey: "connectionMode"),
           let mode = ConnectionMode(rawValue: savedMode) {
            connectionMode = mode
        }

        sidebarVisibility = UserDefaults.standard.bool(forKey: "sidebarVisible") ? .visible : .automatic
    }

    private func setupNotifications() {
        // Listen for commands from menu bar
        NotificationCenter.default.publisher(for: .showAddServer)
            .receive(on: DispatchQueue.main)
            .sink { [weak self] _ in
                self?.showingAddServer = true
            }
            .store(in: &cancellables)

        NotificationCenter.default.publisher(for: .connectSelectedServer)
            .receive(on: DispatchQueue.main)
            .sink { [weak self] _ in
                if let server = self?.selectedServer {
                    self?.connect(to: server)
                }
            }
            .store(in: &cancellables)

        NotificationCenter.default.publisher(for: .disconnectSelectedServer)
            .receive(on: DispatchQueue.main)
            .sink { [weak self] _ in
                if let server = self?.selectedServer {
                    self?.disconnect(from: server)
                }
            }
            .store(in: &cancellables)

        NotificationCenter.default.publisher(for: .editSelectedServer)
            .receive(on: DispatchQueue.main)
            .sink { [weak self] _ in
                self?.editingServer = self?.selectedServer
            }
            .store(in: &cancellables)

        NotificationCenter.default.publisher(for: .duplicateSelectedServer)
            .receive(on: DispatchQueue.main)
            .sink { [weak self] _ in
                if let server = self?.selectedServer {
                    self?.duplicate(server: server)
                }
            }
            .store(in: &cancellables)

        NotificationCenter.default.publisher(for: .deleteSelectedServer)
            .receive(on: DispatchQueue.main)
            .sink { [weak self] _ in
                self?.serverToDelete = self?.selectedServer
            }
            .store(in: &cancellables)

        NotificationCenter.default.publisher(for: .toggleSidebar)
            .receive(on: DispatchQueue.main)
            .sink { [weak self] _ in
                self?.toggleSidebar()
            }
            .store(in: &cancellables)

        NotificationCenter.default.publisher(for: .importSSHConfig)
            .receive(on: DispatchQueue.main)
            .sink { [weak self] _ in
                self?.importSSHConfig()
            }
            .store(in: &cancellables)

        NotificationCenter.default.publisher(for: .exportData)
            .receive(on: DispatchQueue.main)
            .sink { [weak self] _ in
                self?.exportAllData()
            }
            .store(in: &cancellables)

        NotificationCenter.default.publisher(for: .importData)
            .receive(on: DispatchQueue.main)
            .sink { [weak self] _ in
                self?.importAllData()
            }
            .store(in: &cancellables)
    }

    private func setupCloudSyncMonitoring() {
        swiftDataService.$syncStatus
            .receive(on: DispatchQueue.main)
            .sink { [weak self] status in
                self?.cloudSyncStatus = status
            }
            .store(in: &cancellables)
    }

    // MARK: - Data Loading

    func loadData() {
        Task {
            do {
                // Load from SwiftData
                serverModels = try swiftDataService.fetchServers(sortBy: .name)
                groupModels = try swiftDataService.fetchGroups()

                // Convert to view models
                servers = serverModels.map { Server(from: $0) }
                groups = groupModels.map { ServerGroup(from: $0) }

                // Restore last selected server
                if let lastId = UserDefaults.standard.string(forKey: "lastSelectedServer"),
                   let server = servers.first(where: { $0.id == lastId }) {
                    selectedServer = server
                }

            } catch {
                print("Failed to load data: \(error)")
            }
        }
    }

    func refreshData() {
        loadData()
    }

    // MARK: - Server Management

    func addServer(_ server: Server, password: String? = nil) async throws {
        // Create SwiftData model
        let group = server.groupId.flatMap { groupId in
            groupModels.first { $0.id.uuidString == groupId }
        }

        let model = server.toSwiftDataModel(group: group)
        try swiftDataService.saveServer(model)

        // Store password in keychain if provided
        if let password = password, server.authType == .password {
            try? keychainService.savePassword(password, for: server.id)
        }

        // Update local state
        await MainActor.run {
            serverModels.append(model)
            servers.append(server)
        }
    }

    func updateServer(_ server: Server, password: String? = nil) async throws {
        // Find existing model
        guard let model = try swiftDataService.fetchServer(byId: UUID(uuidString: server.id) ?? UUID()) else {
            throw AppStateError.serverNotFound
        }

        // Update fields
        model.name = server.name
        model.host = server.host
        model.port = server.port
        model.username = server.username
        model.authTypeRaw = server.authType.rawValue
        model.privateKeyPath = server.privateKeyPath
        model.group = server.groupId.flatMap { groupId in
            groupModels.first { $0.id.uuidString == groupId }
        }
        model.tags = server.tags
        model.notes = server.notes
        model.jumpHost = server.jumpHost
        model.startupCommand = server.startupCommand
        model.useSSHConfig = server.useSSHConfig
        model.isFavorite = server.isFavorite

        try swiftDataService.updateServer(model)

        // Update password if provided
        if let password = password {
            try? keychainService.savePassword(password, for: server.id)
        }

        // Update local state
        await MainActor.run {
            if let index = servers.firstIndex(where: { $0.id == server.id }) {
                servers[index] = server
                if selectedServer?.id == server.id {
                    selectedServer = server
                }
            }
        }
    }

    func deleteServer(_ server: Server) async {
        guard let model = try swiftDataService.fetchServer(byId: UUID(uuidString: server.id) ?? UUID()) else {
            return
        }

        do {
            try swiftDataService.deleteServer(model)

            // Remove password from keychain
            try? keychainService.deletePassword(for: server.id)

            // Update local state
            await MainActor.run {
                servers.removeAll { $0.id == server.id }
                serverModels.removeAll { $0.id.uuidString == server.id }
                if selectedServer?.id == server.id {
                    selectedServer = nil
                }
            }
        } catch {
            print("Failed to delete server: \(error)")
        }
    }

    func duplicate(server: Server) {
        let newServer = Server(
            id: UUID().uuidString,
            name: "\(server.name) Copy",
            host: server.host,
            port: server.port,
            username: server.username,
            authType: server.authType,
            privateKeyPath: server.privateKeyPath,
            groupId: server.groupId,
            tags: server.tags,
            notes: server.notes,
            status: .unknown,
            lastConnected: nil
        )

        Task {
            try? await addServer(newServer)
        }
    }

    func toggleFavorite(_ server: Server) {
        guard let index = servers.firstIndex(where: { $0.id == server.id }) else { return }

        var updated = server
        updated.isFavorite = !server.isFavorite
        servers[index] = updated

        if selectedServer?.id == server.id {
            selectedServer = updated
        }

        Task {
            try? await updateServer(updated)
        }
    }

    // MARK: - Group Management

    func addGroup(name: String, color: String?) async throws {
        let group = ServerGroupModel(
            name: name,
            colorHex: color,
            displayOrder: groupModels.count
        )

        try swiftDataService.saveGroup(group)

        await MainActor.run {
            groupModels.append(group)
            groups.append(ServerGroup(from: group))
        }
    }

    func deleteGroup(_ group: ServerGroup) async {
        guard let model = groupModels.first(where: { $0.id.uuidString == group.id }) else {
            return
        }

        do {
            try swiftDataService.deleteGroup(model)

            await MainActor.run {
                groups.removeAll { $0.id == group.id }
                groupModels.removeAll { $0.id.uuidString == group.id }

                // Update servers that were in this group
                for (index, server) in servers.enumerated() where server.groupId == group.id {
                    var updated = server
                    updated.groupId = nil
                    servers[index] = updated
                }
            }
        } catch {
            print("Failed to delete group: \(error)")
        }
    }

    // MARK: - Connection Management

    func connect(to server: Server) {
        switch connectionMode {
        case .lite:
            connectNative(to: server)
        case .standard, .pro:
            connectEmbedded(to: server)
        }
    }

    private func connectNative(to server: Server) {
        Task {
            do {
                // Get password from keychain if needed
                var password: String? = nil
                if server.authType == .password {
                    password = try? keychainService.getPassword(for: server.id)
                }

                // Create native terminal command
                var command = "ssh"

                if let port = server.port, port != 22 {
                    command += " -p \(port)"
                }

                if let privateKey = server.privateKeyPath {
                    command += " -i '\(privateKey)'"
                }

                if let jumpHost = server.jumpHost {
                    command += " -J '\(jumpHost)'"
                }

                command += " '\(server.username)@\(server.host)'"

                // Execute in native terminal
                let task = Process()
                task.launchPath = "/usr/bin/open"
                task.arguments = ["-a", "Terminal", command]
                try task.run()

                // Record session
                if let model = try? swiftDataService.fetchServer(byId: UUID(uuidString: server.id) ?? UUID()) {
                    _ = try? swiftDataService.recordConnection(for: model)
                }

                await MainActor.run {
                    updateServerStatus(serverId: server.id, status: .connected)
                }

            } catch {
                await MainActor.run {
                    updateServerStatus(serverId: server.id, status: .error)
                }
            }
        }
    }

    private func connectEmbedded(to server: Server) {
        // For Standard/Pro mode - would initialize embedded terminal
        let session = SSHSession(
            id: UUID().uuidString,
            serverId: server.id,
            serverName: server.name
        )

        activeSessions[session.id] = session

        Task {
            // Simulate connection for now
            try? await Task.sleep(nanoseconds: 1_000_000_000)

            await MainActor.run {
                session.isConnected = true
                updateServerStatus(serverId: server.id, status: .connected)
            }
        }
    }

    func disconnect(from server: Server) {
        // Find and disconnect active session for this server
        if let session = activeSessions.values.first(where: { $0.serverId == server.id }) {
            disconnectSession(session)
        }

        updateServerStatus(serverId: server.id, status: .disconnected)
    }

    func disconnectSession(_ session: SSHSession) {
        Task {
            await MainActor.run {
                session.isConnected = false
                activeSessions.removeValue(forKey: session.id)
            }
        }
    }

    func reconnect(session: SSHSession) {
        guard let server = servers.first(where: { $0.id == session.serverId }) else { return }
        connectEmbedded(to: server)
    }

    // MARK: - Import/Export

    func importSSHConfig() {
        Task {
            do {
                // Read ~/.ssh/config
                let home = FileManager.default.homeDirectoryForCurrentUser
                let configPath = home.appendingPathComponent(".ssh/config")

                guard FileManager.default.fileExists(atPath: configPath.path) else {
                    print("SSH config not found")
                    return
                }

                let content = try String(contentsOf: configPath, encoding: .utf8)
                let parsedServers = SSHConfigParser.parse(content)

                for parsedServer in parsedServers {
                    let server = Server(
                        id: UUID().uuidString,
                        name: parsedServer.host,
                        host: parsedServer.hostname,
                        port: parsedServer.port ?? 22,
                        username: parsedServer.user ?? "root",
                        authType: parsedServer.identityFile != nil ? .key : .agent,
                        privateKeyPath: parsedServer.identityFile,
                        useSSHConfig: true
                    )

                    try? await addServer(server)
                }

                // Reload data
                refreshData()

            } catch {
                print("Failed to import SSH config: \(error)")
            }
        }
    }

    func exportAllData() {
        let panel = NSSavePanel()
        panel.allowedContentTypes = [.json]
        panel.nameFieldStringValue = "easyssh-backup-\(Date().ISO8601Format()).json"

        if panel.runModal() == .OK, let url = panel.url {
            Task {
                do {
                    let data = try swiftDataService.exportAllData()
                    try data.write(to: url)
                    print("Data exported to \(url.path)")
                } catch {
                    print("Export failed: \(error)")
                }
            }
        }
    }

    func importAllData() {
        let panel = NSOpenPanel()
        panel.allowedContentTypes = [.json]
        panel.allowsMultipleSelection = false

        if panel.runModal() == .OK, let url = panel.url {
            Task {
                do {
                    let data = try Data(contentsOf: url)
                    try swiftDataService.importData(from: data, mergeStrategy: .merge)

                    // Reload data
                    refreshData()

                    print("Data imported successfully")
                } catch {
                    print("Import failed: \(error)")
                }
            }
        }
    }

    func exportServers() {
        exportAllData()
    }

    // MARK: - Helpers

    private func updateServerStatus(sessionId: String, status: ServerStatus) {
        // Find server by session
        if let session = activeSessions[sessionId],
           let index = servers.firstIndex(where: { $0.id == session.serverId }) {
            var updated = servers[index]
            updated.status = status
            if status == .connected {
                updated.lastConnected = Date()
            }
            servers[index] = updated

            if selectedServer?.id == updated.id {
                selectedServer = updated
            }
        }
    }

    private func updateServerStatus(serverId: String, status: ServerStatus) {
        if let index = servers.firstIndex(where: { $0.id == serverId }) {
            var updated = servers[index]
            updated.status = status
            if status == .connected {
                updated.lastConnected = Date()
            }
            servers[index] = updated

            if selectedServer?.id == updated.id {
                selectedServer = updated
            }
        }
    }

    private func toggleSidebar() {
        switch sidebarVisibility {
        case .visible:
            sidebarVisibility = .hidden
        case .hidden, .automatic:
            sidebarVisibility = .visible
        }
    }
}

// MARK: - Conversion Extensions

extension Server {
    init(from model: ServerModel) {
        self.id = model.id.uuidString
        self.name = model.name
        self.host = model.host
        self.port = model.port
        self.username = model.username
        self.authType = model.authType
        self.privateKeyPath = model.privateKeyPath
        self.groupId = model.group?.id.uuidString
        self.tags = model.tags
        self.notes = model.notes
        self.status = .unknown
        self.lastConnected = model.lastConnected
        self.isFavorite = model.isFavorite
        self.jumpHost = model.jumpHost
        self.startupCommand = model.startupCommand
        self.useSSHConfig = model.useSSHConfig
    }

    func toSwiftDataModel(group: ServerGroupModel?) -> ServerModel {
        ServerModel(
            id: UUID(uuidString: id) ?? UUID(),
            name: name,
            host: host,
            port: port,
            username: username,
            authType: authType,
            privateKeyPath: privateKeyPath,
            group: group,
            tags: tags,
            notes: notes,
            jumpHost: jumpHost,
            startupCommand: startupCommand,
            useSSHConfig: useSSHConfig,
            isFavorite: isFavorite,
            displayOrder: 0
        )
    }
}

extension ServerGroup {
    init(from model: ServerGroupModel) {
        self.id = model.id.uuidString
        self.name = model.name
        self.color = model.colorHex
        self.sortOrder = model.displayOrder
    }
}

// MARK: - Supporting Types

struct ConnectionStatus {
    let totalServers: Int
    let connectedSessions: Int
    let activeTransfers: Int
}

class SSHSession: ObservableObject, Identifiable {
    let id: String
    let serverId: String
    let serverName: String
    @Published var isConnected = false
    @Published var metadata: SessionMetadata?
    @Published var error: String?
    @Published var terminalContent: String = ""

    init(id: String, serverId: String, serverName: String) {
        self.id = id
        self.serverId = serverId
        self.serverName = serverName
    }
}

struct SessionMetadata: Codable {
    let sessionId: String
    let host: String
    let port: Int
    let username: String
    let connectedAt: Date
}

enum AppStateError: Error, LocalizedError {
    case serverNotFound

    var errorDescription: String? {
        switch self {
        case .serverNotFound:
            return "Server not found in database"
        }
    }
}

// MARK: - SSH Config Parser

struct SSHConfigParser {
    struct ParsedServer {
        let host: String
        let hostname: String
        let port: Int?
        let user: String?
        let identityFile: String?
    }

    static func parse(_ content: String) -> [ParsedServer] {
        var servers: [ParsedServer] = []
        var currentHost: String?
        var currentHostname: String?
        var currentPort: Int?
        var currentUser: String?
        var currentIdentityFile: String?

        let lines = content.components(separatedBy: .newlines)

        for line in lines {
            let trimmed = line.trimmingCharacters(in: .whitespaces)

            // Skip comments and empty lines
            if trimmed.isEmpty || trimmed.hasPrefix("#") {
                continue
            }

            let parts = trimmed.components(separatedBy: .whitespaces)
            guard parts.count >= 2 else { continue }

            let key = parts[0].lowercased()
            let value = parts[1...].joined(separator: " ")

            switch key {
            case "host":
                // Save previous host if exists
                if let host = currentHost, let hostname = currentHostname {
                    servers.append(ParsedServer(
                        host: host,
                        hostname: hostname,
                        port: currentPort,
                        user: currentUser,
                        identityFile: currentIdentityFile
                    ))
                }

                // Skip wildcards
                if value.contains("*") {
                    currentHost = nil
                    currentHostname = nil
                } else {
                    currentHost = value
                    currentHostname = nil
                    currentPort = nil
                    currentUser = nil
                    currentIdentityFile = nil
                }

            case "hostname":
                if currentHost != nil {
                    currentHostname = value
                }

            case "port":
                if currentHost != nil {
                    currentPort = Int(value)
                }

            case "user":
                if currentHost != nil {
                    currentUser = value
                }

            case "identityfile":
                if currentHost != nil {
                    // Expand ~ to home directory
                    let expandedPath = value.replacingOccurrences(
                        of: "~",
                        with: FileManager.default.homeDirectoryForCurrentUser.path
                    )
                    currentIdentityFile = expandedPath
                }

            default:
                break
            }
        }

        // Don't forget the last host
        if let host = currentHost, let hostname = currentHostname {
            servers.append(ParsedServer(
                host: host,
                hostname: hostname,
                port: currentPort,
                user: currentUser,
                identityFile: currentIdentityFile
            ))
        }

        return servers
    }
}
