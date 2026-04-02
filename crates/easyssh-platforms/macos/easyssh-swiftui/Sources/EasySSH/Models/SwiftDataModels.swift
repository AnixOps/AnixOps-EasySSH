import Foundation
import SwiftData
import CryptoKit

// MARK: - Data Schema Versioning

/// Schema version for data migration
enum DataSchemaVersion: Int, CaseIterable {
    case v1 = 1  // Initial JSON/UserDefaults storage
    case v2 = 2  // SwiftData migration
    case v3 = 3  // iCloud sync + encryption

    static var current: DataSchemaVersion { .v3 }
}

// MARK: - SwiftData Models

/// Server model for SwiftData persistence
@Model
final class ServerModel {
    // MARK: - Identifiers
    @Attribute(.unique) var id: UUID
    var createdAt: Date
    var updatedAt: Date
    var schemaVersion: Int

    // MARK: - Server Configuration
    var name: String
    var host: String
    var port: Int
    var username: String
    var authTypeRaw: String

    // MARK: - Authentication (Sensitive - Encrypted)
    @Attribute(.encrypted) var privateKeyPath: String?
    var encryptedKeyData: Data?  // For embedded keys
    var keyPassphraseHint: String?  // Hint only, not the actual passphrase

    // MARK: - Organization
    @Relationship(deleteRule: .nullify) var group: ServerGroupModel?
    var tagsData: Data  // Stored as JSON array
    var notes: String

    // MARK: - Connection Settings
    var jumpHost: String?
    var startupCommand: String?
    var useSSHConfig: Bool

    // MARK: - User Preferences
    var isFavorite: Bool
    var displayOrder: Int

    // MARK: - Connection History
    var lastConnected: Date?
    var connectionCount: Int
    var totalConnectionTime: TimeInterval

    // MARK: - Sync & Cloud
    var isSyncedToCloud: Bool
    var cloudRecordID: String?
    var lastSyncDate: Date?

    // MARK: - Computed Properties

    var authType: AuthType {
        get { AuthType(rawValue: authTypeRaw) ?? .agent }
        set { authTypeRaw = newValue.rawValue }
    }

    var tags: [String] {
        get {
            guard let decoded = try? JSONDecoder().decode([String].self, from: tagsData) else {
                return []
            }
            return decoded
        }
        set {
            tagsData = (try? JSONEncoder().encode(newValue)) ?? Data()
        }
    }

    var status: ServerStatus {
        // Transient property - not persisted
        get { .unknown }
        set { }
    }

    // MARK: - Initialization

    init(
        id: UUID = UUID(),
        name: String,
        host: String,
        port: Int = 22,
        username: String,
        authType: AuthType = .agent,
        privateKeyPath: String? = nil,
        group: ServerGroupModel? = nil,
        tags: [String] = [],
        notes: String = "",
        jumpHost: String? = nil,
        startupCommand: String? = nil,
        useSSHConfig: Bool = true,
        isFavorite: Bool = false,
        displayOrder: Int = 0
    ) {
        self.id = id
        self.createdAt = Date()
        self.updatedAt = Date()
        self.schemaVersion = DataSchemaVersion.current.rawValue

        self.name = name
        self.host = host
        self.port = port
        self.username = username
        self.authTypeRaw = authType.rawValue
        self.privateKeyPath = privateKeyPath
        self.group = group
        self.tagsData = (try? JSONEncoder().encode(tags)) ?? Data()
        self.notes = notes
        self.jumpHost = jumpHost
        self.startupCommand = startupCommand
        self.useSSHConfig = useSSHConfig
        self.isFavorite = isFavorite
        self.displayOrder = displayOrder

        self.connectionCount = 0
        self.totalConnectionTime = 0
        self.isSyncedToCloud = false
    }

    // MARK: - Update Timestamp

    func touch() {
        updatedAt = Date()
    }
}

// MARK: - Server Group Model

@Model
final class ServerGroupModel {
    @Attribute(.unique) var id: UUID
    var createdAt: Date
    var updatedAt: Date
    var schemaVersion: Int

    var name: String
    var colorHex: String?
    var displayOrder: Int
    var isExpanded: Bool  // For UI state persistence

    @Relationship(deleteRule: .cascade, inverse: \ServerModel.group)
    var servers: [ServerModel]?

    @Relationship(deleteRule: .nullify)
    var parent: ServerGroupModel?

    @Relationship(deleteRule: .cascade, inverse: \ServerGroupModel.parent)
    var children: [ServerGroupModel]?

    // Sync
    var isSyncedToCloud: Bool
    var cloudRecordID: String?

    init(
        id: UUID = UUID(),
        name: String,
        colorHex: String? = nil,
        displayOrder: Int = 0,
        parent: ServerGroupModel? = nil
    ) {
        self.id = id
        self.createdAt = Date()
        self.updatedAt = Date()
        self.schemaVersion = DataSchemaVersion.current.rawValue
        self.name = name
        self.colorHex = colorHex
        self.displayOrder = displayOrder
        self.isExpanded = true
        self.parent = parent
        self.isSyncedToCloud = false
    }

    func touch() {
        updatedAt = Date()
    }
}

// MARK: - SSH Identity Model

@Model
final class SSHIdentityModel {
    @Attribute(.unique) var id: UUID
    var createdAt: Date
    var updatedAt: Date

    var name: String
    var keyTypeRaw: String
    var isEncrypted: Bool
    var keyComment: String?

    // Encrypted key storage
    @Attribute(.encrypted) var privateKeyReference: String?  // Keychain reference
    var keyFingerprint: String  // For identification without accessing full key

    // Sync
    var isSyncedToCloud: Bool
    var cloudRecordID: String?

    init(
        id: UUID = UUID(),
        name: String,
        keyType: SSHKeyType = .rsa,
        isEncrypted: Bool = false,
        keyFingerprint: String,
        keyComment: String? = nil
    ) {
        self.id = id
        self.createdAt = Date()
        self.updatedAt = Date()
        self.name = name
        self.keyTypeRaw = keyType.rawValue
        self.isEncrypted = isEncrypted
        self.keyFingerprint = keyFingerprint
        self.keyComment = keyComment
        self.isSyncedToCloud = false
    }

    var keyType: SSHKeyType {
        get { SSHKeyType(rawValue: keyTypeRaw) ?? .rsa }
        set { keyTypeRaw = newValue.rawValue }
    }
}

// MARK: - Connection Profile Model

@Model
final class ConnectionProfileModel {
    @Attribute(.unique) var id: UUID
    var createdAt: Date
    var updatedAt: Date

    var name: String
    var port: Int
    var username: String
    var authTypeRaw: String
    var keepAlive: Bool
    var forwardAgent: Bool

    // SSH Options stored as JSON
    var customOptionsData: Data

    // Sync
    var isSyncedToCloud: Bool
    var isDefault: Bool  // Mark as default profile

    init(
        id: UUID = UUID(),
        name: String,
        port: Int = 22,
        username: String,
        authType: AuthType = .agent,
        keepAlive: Bool = true,
        forwardAgent: Bool = false,
        customOptions: [String: String] = [:],
        isDefault: Bool = false
    ) {
        self.id = id
        self.createdAt = Date()
        self.updatedAt = Date()
        self.name = name
        self.port = port
        self.username = username
        self.authTypeRaw = authType.rawValue
        self.keepAlive = keepAlive
        self.forwardAgent = forwardAgent
        self.customOptionsData = (try? JSONEncoder().encode(customOptions)) ?? Data()
        self.isDefault = isDefault
        self.isSyncedToCloud = false
    }

    var authType: AuthType {
        get { AuthType(rawValue: authTypeRaw) ?? .agent }
        set { authTypeRaw = newValue.rawValue }
    }

    var customOptions: [String: String] {
        get {
            guard let decoded = try? JSONDecoder().decode([String: String].self, from: customOptionsData) else {
                return [:]
            }
            return decoded
        }
        set {
            customOptionsData = (try? JSONEncoder().encode(newValue)) ?? Data()
        }
    }
}

// MARK: - Session History Model

@Model
final class SessionRecordModel {
    @Attribute(.unique) var id: UUID
    var createdAt: Date

    @Relationship(deleteRule: .nullify)
    var server: ServerModel?

    var serverName: String  // Cached for history even if server deleted
    var startTime: Date
    var endTime: Date?
    var commandsExecuted: Int
    var filesTransferred: Int
    var sessionTypeRaw: String
    var notes: String?
    var exitCode: Int?

    // Performance metrics
    var latencyMs: Double?
    var bytesTransferred: Int64?

    init(
        id: UUID = UUID(),
        server: ServerModel?,
        serverName: String,
        startTime: Date,
        sessionType: SessionType = .terminal
    ) {
        self.id = id
        self.createdAt = Date()
        self.server = server
        self.serverName = serverName
        self.startTime = startTime
        self.sessionTypeRaw = sessionType.rawValue
        self.commandsExecuted = 0
        self.filesTransferred = 0
    }

    var sessionType: SessionType {
        get { SessionType(rawValue: sessionTypeRaw) ?? .terminal }
        set { sessionTypeRaw = newValue.rawValue }
    }

    var duration: TimeInterval? {
        guard let endTime = endTime else { return nil }
        return endTime.timeIntervalSince(startTime)
    }
}

// MARK: - Snippet Model (Pro Feature)

@Model
final class SnippetModel {
    @Attribute(.unique) var id: UUID
    var createdAt: Date
    var updatedAt: Date

    var title: String
    var content: String
    var tagsData: Data
    var isShared: Bool
    var teamId: String?
    var createdBy: String?

    // Usage tracking
    var useCount: Int
    var lastUsed: Date?

    // Sync
    var isSyncedToCloud: Bool
    var cloudRecordID: String?

    init(
        id: UUID = UUID(),
        title: String,
        content: String,
        tags: [String] = [],
        isShared: Bool = false,
        teamId: String? = nil,
        createdBy: String? = nil
    ) {
        self.id = id
        self.createdAt = Date()
        self.updatedAt = Date()
        self.title = title
        self.content = content
        self.tagsData = (try? JSONEncoder().encode(tags)) ?? Data()
        self.isShared = isShared
        self.teamId = teamId
        self.createdBy = createdBy
        self.useCount = 0
        self.isSyncedToCloud = false
    }

    var tags: [String] {
        get {
            guard let decoded = try? JSONDecoder().decode([String].self, from: tagsData) else {
                return []
            }
            return decoded
        }
        set {
            tagsData = (try? JSONEncoder().encode(newValue)) ?? Data()
        }
    }
}

// MARK: - App Settings Model

@Model
final class AppSettingsModel {
    @Attribute(.unique) var id: UUID
    var updatedAt: Date

    // General Settings
    var connectionModeRaw: String
    var startupBehaviorRaw: String
    var sidebarVisibilityRaw: String

    // Terminal Settings
    var defaultTerminalTheme: String
    var fontFamily: String
    var fontSize: Double
    var enableWebGL: Bool

    // Security Settings
    var requireUnlockOnLaunch: Bool
    var lockAfterMinutes: Int
    var clearClipboardOnExit: Bool

    // Sync Settings
    var enableCloudSync: Bool
    var lastSyncDate: Date?
    var syncConflictResolutionRaw: String

    // Data Migration
    var schemaVersion: Int
    var migrationCompletedAt: Date?

    init() {
        self.id = UUID()
        self.updatedAt = Date()
        self.connectionModeRaw = ConnectionMode.lite.rawValue
        self.startupBehaviorRaw = StartupBehavior.showWindow.rawValue
        self.sidebarVisibilityRaw = SidebarVisibility.automatic.rawValue
        self.defaultTerminalTheme = "dark"
        self.fontFamily = "SF Mono"
        self.fontSize = 13.0
        self.enableWebGL = true
        self.requireUnlockOnLaunch = false
        self.lockAfterMinutes = 5
        self.clearClipboardOnExit = true
        self.enableCloudSync = true
        self.syncConflictResolutionRaw = SyncConflictResolution.newestWins.rawValue
        self.schemaVersion = DataSchemaVersion.current.rawValue
    }

    var connectionMode: ConnectionMode {
        get { ConnectionMode(rawValue: connectionModeRaw) ?? .lite }
        set { connectionModeRaw = newValue.rawValue }
    }

    var startupBehavior: StartupBehavior {
        get { StartupBehavior(rawValue: startupBehaviorRaw) ?? .showWindow }
        set { startupBehaviorRaw = newValue.rawValue }
    }

    var sidebarVisibility: SidebarVisibility {
        get { SidebarVisibility(rawValue: sidebarVisibilityRaw) ?? .automatic }
        set { sidebarVisibilityRaw = newValue.rawValue }
    }

    var syncConflictResolution: SyncConflictResolution {
        get { SyncConflictResolution(rawValue: syncConflictResolutionRaw) ?? .newestWins }
        set { syncConflictResolutionRaw = newValue.rawValue }
    }
}

// MARK: - Supporting Types

enum SSHKeyType: String, Codable {
    case rsa = "rsa"
    case ed25519 = "ed25519"
    case ecdsa = "ecdsa"
    case dsa = "dsa"
}

enum StartupBehavior: String, Codable {
    case showWindow = "showWindow"
    case showMenuBarOnly = "showMenuBarOnly"
    case restoreSessions = "restoreSessions"
}

enum SyncConflictResolution: String, Codable {
    case newestWins = "newestWins"
    case localWins = "localWins"
    case cloudWins = "cloudWins"
    case askUser = "askUser"
}

// MARK: - SwiftData Schema Configuration

/// Schema configuration for SwiftData container
enum EasySSHSchema {
    static var schema: Schema {
        Schema([
            ServerModel.self,
            ServerGroupModel.self,
            SSHIdentityModel.self,
            ConnectionProfileModel.self,
            SessionRecordModel.self,
            SnippetModel.self,
            AppSettingsModel.self
        ])
    }

    static var modelConfiguration: ModelConfiguration {
        let configuration = ModelConfiguration(
            schema: schema,
            isStoredInMemoryOnly: false,
            cloudKitDatabase: .automatic
        )
        return configuration
    }
}
