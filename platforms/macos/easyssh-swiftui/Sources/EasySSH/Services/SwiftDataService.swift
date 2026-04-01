import Foundation
import SwiftData
import CryptoKit
import Combine

// MARK: - Encryption Service

/// Handles encryption for sensitive SwiftData fields
final class SwiftDataEncryptionService {
    static let shared = SwiftDataEncryptionService()

    private let keychainKey = "com.anixops.easyssh.swiftdata.encryption.key"
    private var symmetricKey: SymmetricKey?

    private init() {
        symmetricKey = loadOrCreateKey()
    }

    /// Load existing key from Keychain or create a new one
    private func loadOrCreateKey() -> SymmetricKey? {
        // Try to load existing key from Keychain
        if let existingKey = try? KeychainService.shared.getData(key: keychainKey) {
            return SymmetricKey(data: existingKey)
        }

        // Generate new key
        let key = SymmetricKey(size: .bits256)
        let keyData = key.withUnsafeBytes { Data($0) }

        // Store in Keychain
        do {
            try KeychainService.shared.saveData(keyData, key: keychainKey)
            return key
        } catch {
            print("Failed to save encryption key to Keychain: \(error)")
            return nil
        }
    }

    /// Encrypt data using AES-GCM
    func encrypt(_ data: Data) -> Data? {
        guard let key = symmetricKey else { return nil }

        do {
            let sealedBox = try AES.GCM.seal(data, using: key)
            return sealedBox.combined
        } catch {
            print("Encryption failed: \(error)")
            return nil
        }
    }

    /// Decrypt data using AES-GCM
    func decrypt(_ data: Data) -> Data? {
        guard let key = symmetricKey else { return nil }

        do {
            let sealedBox = try AES.GCM.SealedBox(combined: data)
            let decryptedData = try AES.GCM.open(sealedBox, using: key)
            return decryptedData
        } catch {
            print("Decryption failed: \(error)")
            return nil
        }
    }

    /// Encrypt a string value
    func encryptString(_ value: String) -> Data? {
        guard let data = value.data(using: .utf8) else { return nil }
        return encrypt(data)
    }

    /// Decrypt to string
    func decryptString(_ data: Data) -> String? {
        guard let decrypted = decrypt(data) else { return nil }
        return String(data: decrypted, encoding: .utf8)
    }
}

// MARK: - SwiftData Service

/// Main service for SwiftData operations with iCloud sync
@MainActor
final class SwiftDataService: ObservableObject {
    static let shared = SwiftDataService()

    let container: ModelContainer
    private let encryptionService = SwiftDataEncryptionService.shared

    @Published var syncStatus: CloudSyncStatus = .unknown
    @Published var lastSyncError: Error?

    // MARK: - Initialization

    init() {
        do {
            container = try ModelContainer(
                for: ServerModel.self,
                ServerGroupModel.self,
                SSHIdentityModel.self,
                ConnectionProfileModel.self,
                SessionRecordModel.self,
                SnippetModel.self,
                AppSettingsModel.self,
                configurations: [EasySSHSchema.modelConfiguration]
            )

            // Setup CloudKit sync monitoring
            setupCloudSyncMonitoring()
        } catch {
            fatalError("Failed to create ModelContainer: \(error)")
        }
    }

    // MARK: - Context Helper

    var mainContext: ModelContext {
        container.mainContext
    }

    func newBackgroundContext() -> ModelContext {
        ModelContext(container)
    }

    // MARK: - Cloud Sync Monitoring

    private func setupCloudSyncMonitoring() {
        // Subscribe to NSPersistentStoreRemoteChange notifications for CloudKit sync status
        NotificationCenter.default.addObserver(
            self,
            selector: #selector(handleCloudKitEvent),
            name: NSPersistentStoreRemoteChangeNotification,
            object: nil
        )
    }

    @objc private func handleCloudKitEvent(_ notification: Notification) {
        // Update sync status based on CloudKit events
        Task { @MainActor in
            self.syncStatus = .synced
        }
    }

    // MARK: - Server Operations

    /// Fetch all servers with optional sorting and filtering
    func fetchServers(
        group: ServerGroupModel? = nil,
        searchText: String? = nil,
        favoritesOnly: Bool = false,
        sortBy: ServerSortOption = .name
    ) throws -> [ServerModel] {
        let descriptor = FetchDescriptor<ServerModel>(
            predicate: buildServerPredicate(
                group: group,
                searchText: searchText,
                favoritesOnly: favoritesOnly
            ),
            sortBy: sortBy.sortDescriptors
        )

        return try mainContext.fetch(descriptor)
    }

    /// Get server by ID
    func fetchServer(byId id: UUID) throws -> ServerModel? {
        let descriptor = FetchDescriptor<ServerModel>(
            predicate: #Predicate { $0.id == id }
        )
        return try mainContext.fetch(descriptor).first
    }

    /// Save a new server
    func saveServer(_ server: ServerModel) throws {
        server.touch()
        mainContext.insert(server)
        try mainContext.save()

        // Trigger CloudKit sync
        triggerCloudSync()
    }

    /// Update existing server
    func updateServer(_ server: ServerModel) throws {
        server.touch()
        try mainContext.save()
        triggerCloudSync()
    }

    /// Delete server
    func deleteServer(_ server: ServerModel) throws {
        // Remove associated passwords from Keychain
        try? KeychainService.shared.deletePassword(for: server.id.uuidString)

        mainContext.delete(server)
        try mainContext.save()
        triggerCloudSync()
    }

    /// Record connection event
    func recordConnection(for server: ServerModel, sessionType: SessionType = .terminal) throws -> SessionRecordModel {
        server.lastConnected = Date()
        server.connectionCount += 1

        let record = SessionRecordModel(
            server: server,
            serverName: server.name,
            startTime: Date(),
            sessionType: sessionType
        )

        mainContext.insert(record)
        try mainContext.save()

        return record
    }

    /// End session recording
    func endSession(_ session: SessionRecordModel, commandsExecuted: Int = 0, filesTransferred: Int = 0) throws {
        session.endTime = Date()
        session.commandsExecuted = commandsExecuted
        session.filesTransferred = filesTransferred

        // Update server stats
        if let server = session.server {
            let duration = session.endTime?.timeIntervalSince(session.startTime) ?? 0
            server.totalConnectionTime += duration
        }

        try mainContext.save()
    }

    // MARK: - Server Group Operations

    func fetchGroups(parent: ServerGroupModel? = nil) throws -> [ServerGroupModel] {
        let descriptor = FetchDescriptor<ServerGroupModel>(
            predicate: #Predicate { $0.parent == parent },
            sortBy: [SortDescriptor(\.displayOrder), SortDescriptor(\.name)]
        )
        return try mainContext.fetch(descriptor)
    }

    func saveGroup(_ group: ServerGroupModel) throws {
        group.touch()
        mainContext.insert(group)
        try mainContext.save()
        triggerCloudSync()
    }

    func updateGroup(_ group: ServerGroupModel) throws {
        group.touch()
        try mainContext.save()
        triggerCloudSync()
    }

    func deleteGroup(_ group: ServerGroupModel) throws {
        // Note: Related servers will be handled by deleteRule
        mainContext.delete(group)
        try mainContext.save()
        triggerCloudSync()
    }

    // MARK: - SSH Identity Operations

    func fetchIdentities() throws -> [SSHIdentityModel] {
        let descriptor = FetchDescriptor<SSHIdentityModel>(
            sortBy: [SortDescriptor(\.name)]
        )
        return try mainContext.fetch(descriptor)
    }

    func saveIdentity(_ identity: SSHIdentityModel) throws {
        identity.updatedAt = Date()
        mainContext.insert(identity)
        try mainContext.save()
        triggerCloudSync()
    }

    func deleteIdentity(_ identity: SSHIdentityModel) throws {
        mainContext.delete(identity)
        try mainContext.save()
        triggerCloudSync()
    }

    // MARK: - Session History Operations

    func fetchSessionHistory(
        for server: ServerModel? = nil,
        limit: Int = 100
    ) throws -> [SessionRecordModel] {
        var descriptor = FetchDescriptor<SessionRecordModel>(
            sortBy: [SortDescriptor(\.startTime, order: .reverse)]
        )
        descriptor.fetchLimit = limit

        if let server = server {
            descriptor.predicate = #Predicate { $0.server == server }
        }

        return try mainContext.fetch(descriptor)
    }

    func deleteSessionHistory(olderThan date: Date) throws {
        let descriptor = FetchDescriptor<SessionRecordModel>(
            predicate: #Predicate { $0.startTime < date }
        )
        let oldSessions = try mainContext.fetch(descriptor)

        for session in oldSessions {
            mainContext.delete(session)
        }

        try mainContext.save()
    }

    // MARK: - Snippet Operations (Pro)

    func fetchSnippets(teamId: String? = nil, searchText: String? = nil) throws -> [SnippetModel] {
        var predicates: [Predicate<SnippetModel>] = []

        if let teamId = teamId {
            predicates.append(#Predicate { $0.teamId == teamId })
        }

        let descriptor = FetchDescriptor<SnippetModel>(
            sortBy: [SortDescriptor(\.useCount, order: .reverse), SortDescriptor(\.title)]
        )

        return try mainContext.fetch(descriptor)
    }

    func saveSnippet(_ snippet: SnippetModel) throws {
        snippet.updatedAt = Date()
        mainContext.insert(snippet)
        try mainContext.save()
        triggerCloudSync()
    }

    func recordSnippetUsage(_ snippet: SnippetModel) throws {
        snippet.useCount += 1
        snippet.lastUsed = Date()
        try mainContext.save()
    }

    // MARK: - App Settings

    func fetchSettings() throws -> AppSettingsModel {
        let descriptor = FetchDescriptor<AppSettingsModel>()
        if let settings = try mainContext.fetch(descriptor).first {
            return settings
        }

        // Create default settings
        let settings = AppSettingsModel()
        mainContext.insert(settings)
        try mainContext.save()
        return settings
    }

    func updateSettings(_ settings: AppSettingsModel) throws {
        settings.updatedAt = Date()
        try mainContext.save()
        triggerCloudSync()
    }

    // MARK: - Cloud Sync Operations

    func triggerCloudSync() {
        guard syncStatus != .syncing else { return }

        syncStatus = .syncing

        // CloudKit sync is automatic, but we can monitor status
        // In a real implementation, you'd use NSPersistentCloudKitContainer events
        Task {
            try? await Task.sleep(nanoseconds: 500_000_000) // Simulate brief sync
            await MainActor.run {
                self.syncStatus = .synced
            }
        }
    }

    func disableCloudSync() throws {
        // Disable CloudKit sync and switch to local-only
        let settings = try fetchSettings()
        settings.enableCloudSync = false
        try mainContext.save()
    }

    func exportAllData() throws -> Data {
        // Export all data for backup
        let servers = try fetchServers()
        let groups = try fetchGroups()
        let identities = try fetchIdentities()
        let snippets = try fetchSnippets()
        let settings = try fetchSettings()

        let exportData = DataExport(
            servers: servers.map { ServerExport(from: $0) },
            groups: groups.map { GroupExport(from: $0) },
            identities: identities.map { IdentityExport(from: $0) },
            snippets: snippets.map { SnippetExport(from: $0) },
            settings: SettingsExport(from: settings),
            exportDate: Date(),
            schemaVersion: DataSchemaVersion.current.rawValue
        )

        return try JSONEncoder().encode(exportData)
    }

    func importData(from data: Data, mergeStrategy: ImportMergeStrategy = .merge) throws {
        let exportData = try JSONDecoder().decode(DataExport.self, from: data)

        // Validate schema version
        guard exportData.schemaVersion <= DataSchemaVersion.current.rawValue else {
            throw SwiftDataServiceError.newerSchemaVersion
        }

        // Import based on merge strategy
        switch mergeStrategy {
        case .replaceAll:
            try deleteAllData()
            fallthrough
        case .merge:
            try performImport(exportData)
        case .skipExisting:
            try performImport(exportData, skipExisting: true)
        }
    }

    // MARK: - Helper Methods

    private func buildServerPredicate(
        group: ServerGroupModel?,
        searchText: String?,
        favoritesOnly: Bool
    ) -> Predicate<ServerModel>? {
        // Build compound predicate based on filters
        // Note: SwiftData predicates have limitations, so we may need to filter in memory for complex cases

        if let group = group {
            return #Predicate { $0.group == group }
        }

        if favoritesOnly {
            return #Predicate { $0.isFavorite == true }
        }

        return nil
    }

    private func deleteAllData() throws {
        let serverDescriptor = FetchDescriptor<ServerModel>()
        let servers = try mainContext.fetch(serverDescriptor)
        servers.forEach { mainContext.delete($0) }

        let groupDescriptor = FetchDescriptor<ServerGroupModel>()
        let groups = try mainContext.fetch(groupDescriptor)
        groups.forEach { mainContext.delete($0) }

        try mainContext.save()
    }

    private func performImport(_ exportData: DataExport, skipExisting: Bool = false) throws {
        // Import groups first (servers reference them)
        for groupExport in exportData.groups {
            if skipExisting {
                let descriptor = FetchDescriptor<ServerGroupModel>(
                    predicate: #Predicate { $0.id == groupExport.id }
                )
                if try mainContext.fetch(descriptor).first != nil {
                    continue
                }
            }

            let group = ServerGroupModel(
                id: groupExport.id,
                name: groupExport.name,
                colorHex: groupExport.color,
                displayOrder: groupExport.displayOrder
            )
            mainContext.insert(group)
        }

        // Import servers
        for serverExport in exportData.servers {
            if skipExisting {
                let descriptor = FetchDescriptor<ServerModel>(
                    predicate: #Predicate { $0.id == serverExport.id }
                )
                if try mainContext.fetch(descriptor).first != nil {
                    continue
                }
            }

            // Find group if specified
            var group: ServerGroupModel? = nil
            if let groupId = serverExport.groupId {
                let groupDescriptor = FetchDescriptor<ServerGroupModel>(
                    predicate: #Predicate { $0.id == groupId }
                )
                group = try mainContext.fetch(groupDescriptor).first
            }

            let server = ServerModel(
                id: serverExport.id,
                name: serverExport.name,
                host: serverExport.host,
                port: serverExport.port,
                username: serverExport.username,
                authType: serverExport.authType,
                privateKeyPath: serverExport.privateKeyPath,
                group: group,
                tags: serverExport.tags,
                notes: serverExport.notes,
                jumpHost: serverExport.jumpHost,
                startupCommand: serverExport.startupCommand,
                useSSHConfig: serverExport.useSSHConfig,
                isFavorite: serverExport.isFavorite,
                displayOrder: serverExport.displayOrder
            )

            server.lastConnected = serverExport.lastConnected
            server.connectionCount = serverExport.connectionCount
            server.totalConnectionTime = serverExport.totalConnectionTime

            mainContext.insert(server)
        }

        try mainContext.save()
    }
}

// MARK: - Supporting Types

enum CloudSyncStatus: Equatable {
    case unknown
    case synced
    case syncing
    case error(String)
    case disabled
}

enum ServerSortOption {
    case name
    case lastConnected
    case connectionCount
    case displayOrder
    case dateAdded

    var sortDescriptors: [SortDescriptor<ServerModel>] {
        switch self {
        case .name:
            return [SortDescriptor(\.name)]
        case .lastConnected:
            return [SortDescriptor(\.lastConnected, order: .reverse)]
        case .connectionCount:
            return [SortDescriptor(\.connectionCount, order: .reverse)]
        case .displayOrder:
            return [SortDescriptor(\.displayOrder), SortDescriptor(\.name)]
        case .dateAdded:
            return [SortDescriptor(\.createdAt, order: .reverse)]
        }
    }
}

enum ImportMergeStrategy {
    case replaceAll    // Delete all existing data
    case merge         // Merge with existing (update if exists)
    case skipExisting  // Skip items that already exist
}

enum SwiftDataServiceError: Error, LocalizedError {
    case newerSchemaVersion
    case importFailed(String)
    case encryptionFailed

    var errorDescription: String? {
        switch self {
        case .newerSchemaVersion:
            return "Cannot import data from a newer app version"
        case .importFailed(let reason):
            return "Import failed: \(reason)"
        case .encryptionFailed:
            return "Failed to encrypt sensitive data"
        }
    }
}

// MARK: - Export Models

struct DataExport: Codable {
    let servers: [ServerExport]
    let groups: [GroupExport]
    let identities: [IdentityExport]
    let snippets: [SnippetExport]
    let settings: SettingsExport
    let exportDate: Date
    let schemaVersion: Int
}

struct ServerExport: Codable {
    let id: UUID
    let name: String
    let host: String
    let port: Int
    let username: String
    let authType: AuthType
    let privateKeyPath: String?
    let groupId: UUID?
    let tags: [String]
    let notes: String
    let jumpHost: String?
    let startupCommand: String?
    let useSSHConfig: Bool
    let isFavorite: Bool
    let displayOrder: Int
    let lastConnected: Date?
    let connectionCount: Int
    let totalConnectionTime: TimeInterval
    let createdAt: Date

    init(from model: ServerModel) {
        self.id = model.id
        self.name = model.name
        self.host = model.host
        self.port = model.port
        self.username = model.username
        self.authType = model.authType
        self.privateKeyPath = model.privateKeyPath
        self.groupId = model.group?.id
        self.tags = model.tags
        self.notes = model.notes
        self.jumpHost = model.jumpHost
        self.startupCommand = model.startupCommand
        self.useSSHConfig = model.useSSHConfig
        self.isFavorite = model.isFavorite
        self.displayOrder = model.displayOrder
        self.lastConnected = model.lastConnected
        self.connectionCount = model.connectionCount
        self.totalConnectionTime = model.totalConnectionTime
        self.createdAt = model.createdAt
    }
}

struct GroupExport: Codable {
    let id: UUID
    let name: String
    let color: String?
    let displayOrder: Int
    let parentId: UUID?

    init(from model: ServerGroupModel) {
        self.id = model.id
        self.name = model.name
        self.color = model.colorHex
        self.displayOrder = model.displayOrder
        self.parentId = model.parent?.id
    }
}

struct IdentityExport: Codable {
    let id: UUID
    let name: String
    let keyType: SSHKeyType
    let isEncrypted: Bool
    let keyFingerprint: String
    let keyComment: String?

    init(from model: SSHIdentityModel) {
        self.id = model.id
        self.name = model.name
        self.keyType = model.keyType
        self.isEncrypted = model.isEncrypted
        self.keyFingerprint = model.keyFingerprint
        self.keyComment = model.keyComment
    }
}

struct SnippetExport: Codable {
    let id: UUID
    let title: String
    let content: String
    let tags: [String]
    let isShared: Bool
    let teamId: String?
    let useCount: Int

    init(from model: SnippetModel) {
        self.id = model.id
        self.title = model.title
        self.content = model.content
        self.tags = model.tags
        self.isShared = model.isShared
        self.teamId = model.teamId
        self.useCount = model.useCount
    }
}

struct SettingsExport: Codable {
    let connectionMode: ConnectionMode
    let startupBehavior: StartupBehavior
    let sidebarVisibility: SidebarVisibility
    let enableCloudSync: Bool

    init(from model: AppSettingsModel) {
        self.connectionMode = model.connectionMode
        self.startupBehavior = model.startupBehavior
        self.sidebarVisibility = model.sidebarVisibility
        self.enableCloudSync = model.enableCloudSync
    }
}
