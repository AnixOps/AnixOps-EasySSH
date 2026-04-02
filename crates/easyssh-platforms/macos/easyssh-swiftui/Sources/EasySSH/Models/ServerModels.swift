import Foundation

// MARK: - Server Model

struct Server: Identifiable, Codable, Equatable, Hashable {
    let id: String
    var name: String
    var host: String
    var port: Int
    var username: String
    var authType: AuthType
    var privateKeyPath: String?
    var groupId: String?
    var tags: [String]
    var notes: String
    var status: ServerStatus
    var lastConnected: Date?
    var isFavorite: Bool
    var jumpHost: String?
    var startupCommand: String?
    var useSSHConfig: Bool

    init(
        id: String,
        name: String,
        host: String,
        port: Int = 22,
        username: String,
        authType: AuthType = .agent,
        privateKeyPath: String? = nil,
        groupId: String? = nil,
        tags: [String] = [],
        notes: String = "",
        status: ServerStatus = .unknown,
        lastConnected: Date? = nil,
        isFavorite: Bool = false,
        jumpHost: String? = nil,
        startupCommand: String? = nil,
        useSSHConfig: Bool = true
    ) {
        self.id = id
        self.name = name
        self.host = host
        self.port = port
        self.username = username
        self.authType = authType
        self.privateKeyPath = privateKeyPath
        self.groupId = groupId
        self.tags = tags
        self.notes = notes
        self.status = status
        self.lastConnected = lastConnected
        self.isFavorite = isFavorite
        self.jumpHost = jumpHost
        self.startupCommand = startupCommand
        self.useSSHConfig = useSSHConfig
    }
}

// MARK: - Server Group

struct ServerGroup: Identifiable, Codable, Equatable {
    let id: String
    var name: String
    var color: String?  // Hex color string
    var sortOrder: Int

    static let defaultGroups = [
        ServerGroup(id: "production", name: "Production", color: "#FF6B6B", sortOrder: 0),
        ServerGroup(id: "staging", name: "Staging", color: "#4ECDC4", sortOrder: 1),
        ServerGroup(id: "development", name: "Development", color: "#45B7D1", sortOrder: 2)
    ]
}

// MARK: - Authentication Types

enum AuthType: String, Codable, CaseIterable, Identifiable {
    case password = "password"
    case key = "key"
    case agent = "agent"

    var id: String { rawValue }

    var displayName: String {
        switch self {
        case .password: return "Password"
        case .key: return "Private Key"
        case .agent: return "SSH Agent"
        }
    }

    var icon: String {
        switch self {
        case .password: return "lock"
        case .key: return "key"
        case .agent: return "keychain"
        }
    }
}

// MARK: - Server Status

enum ServerStatus: String, Codable {
    case unknown = "unknown"
    case connected = "connected"
    case disconnected = "disconnected"
    case error = "error"
    case connecting = "connecting"

    var displayName: String {
        switch self {
        case .unknown: return "Unknown"
        case .connected: return "Connected"
        case .disconnected: return "Disconnected"
        case .error: return "Error"
        case .connecting: return "Connecting..."
        }
    }
}

// MARK: - SSH Identity

struct SSHIdentity: Identifiable, Codable {
    let id: String
    var name: String
    var privateKeyPath: String
    var publicKeyPath: String?
    var isEncrypted: Bool
    var addedToAgent: Bool
}

// MARK: - Connection Profile

struct ConnectionProfile: Identifiable, Codable {
    let id: String
    var name: String
    var port: Int
    var username: String
    var authType: AuthType
    var keepAlive: Bool
    var forwardAgent: Bool
    var customOptions: [String: String]
}

// MARK: - Session History

struct SessionRecord: Identifiable, Codable {
    let id: String
    let serverId: String
    let serverName: String
    let startTime: Date
    var endTime: Date?
    var commandsExecuted: Int
    var filesTransferred: Int
    var sessionType: SessionType
    var notes: String?
}

enum SessionType: String, Codable {
    case terminal = "terminal"
    case sftp = "sftp"
    case portForward = "port_forward"
}

// MARK: - Snippet (Pro Feature)

struct Snippet: Identifiable, Codable {
    let id: String
    var title: String
    var content: String
    var tags: [String]
    var isShared: Bool
    var createdBy: String?
    var teamId: String?
}

// MARK: - Form View Model

@Observable
class ServerFormViewModel {
    var name = ""
    var host = ""
    var port = "22"
    var username = ""
    var authType: AuthType = .agent
    var password = ""
    var privateKeyPath = ""
    var keyPassphrase = ""
    var groupId: String? = nil
    var tags: [String] = []
    var notes = ""
    var jumpHost = ""
    var startupCommand = ""
    var useSSHConfig = true

    var nameValidation: ValidationResult {
        if name.isEmpty {
            return ValidationResult(isValid: false, message: "Name is required")
        }
        if name.count < 2 {
            return ValidationResult(isValid: false, message: "Name must be at least 2 characters")
        }
        return ValidationResult(isValid: true, message: "Valid")
    }

    var isValid: Bool {
        !name.isEmpty && !host.isEmpty && !username.isEmpty
    }

    var isValidForTest: Bool {
        !host.isEmpty && !username.isEmpty
    }

    init() {}

    init(from server: Server) {
        self.name = server.name
        self.host = server.host
        self.port = String(server.port)
        self.username = server.username
        self.authType = server.authType
        self.privateKeyPath = server.privateKeyPath ?? ""
        self.groupId = server.groupId
        self.tags = server.tags
        self.notes = server.notes
        self.jumpHost = server.jumpHost ?? ""
        self.startupCommand = server.startupCommand ?? ""
        self.useSSHConfig = server.useSSHConfig
    }

    func toServer(id: String? = nil) -> Server {
        Server(
            id: id ?? UUID().uuidString,
            name: name,
            host: host,
            port: Int(port) ?? 22,
            username: username,
            authType: authType,
            privateKeyPath: privateKeyPath.isEmpty ? nil : privateKeyPath,
            groupId: groupId,
            tags: tags,
            notes: notes,
            status: .unknown,
            jumpHost: jumpHost.isEmpty ? nil : jumpHost,
            startupCommand: startupCommand.isEmpty ? nil : startupCommand,
            useSSHConfig: useSSHConfig
        )
    }
}

struct ValidationResult {
    let isValid: Bool
    let message: String
}
