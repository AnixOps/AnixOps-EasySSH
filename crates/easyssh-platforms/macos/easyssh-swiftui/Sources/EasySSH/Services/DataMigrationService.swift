import Foundation
import SwiftData

/// Service for migrating data from UserDefaults/JSON to SwiftData
@MainActor
final class DataMigrationService {
    static let shared = DataMigrationService()

    private let swiftDataService = SwiftDataService.shared
    private let userDefaults = UserDefaults.standard
    private let migrationKey = "com.anixops.easyssh.migration.completed"
    private let schemaVersionKey = "com.anixops.easyssh.migration.schemaVersion"

    // Legacy keys for UserDefaults storage
    private enum LegacyKeys {
        static let servers = "com.anixops.easyssh.servers"
        static let groups = "com.anixops.easyssh.groups"
        static let settings = "com.anixops.easyssh.settings"
        static let lastSelectedServer = "lastSelectedServer"
        static let connectionMode = "connectionMode"
        static let sidebarVisible = "sidebarVisible"
    }

    var isMigrationNeeded: Bool {
        // Check if legacy data exists and migration hasn't been completed
        guard !userDefaults.bool(forKey: migrationKey) else { return false }
        return userDefaults.object(forKey: LegacyKeys.servers) != nil ||
               userDefaults.object(forKey: LegacyKeys.groups) != nil
    }

    var currentSchemaVersion: DataSchemaVersion {
        let storedVersion = userDefaults.integer(forKey: schemaVersionKey)
        return DataSchemaVersion(rawValue: storedVersion) ?? .v1
    }

    // MARK: - Migration Execution

    /// Perform full data migration
    func performMigration() async throws -> MigrationResult {
        let startTime = Date()
        var result = MigrationResult()

        do {
            // Step 1: Migrate groups (must be done before servers)
            let groupsResult = try await migrateGroups()
            result.groupsMigrated = groupsResult.count
            result.groupErrors = groupsResult.errors

            // Step 2: Migrate servers
            let serversResult = try await migrateServers()
            result.serversMigrated = serversResult.count
            result.serverErrors = serversResult.errors

            // Step 3: Migrate settings
            try migrateSettings()

            // Step 4: Mark migration complete
            markMigrationComplete()

            result.duration = Date().timeIntervalSince(startTime)
            result.success = true

        } catch {
            result.success = false
            result.errorMessage = error.localizedDescription
        }

        return result
    }

    /// Migrate with progress callback
    func performMigrationWithProgress(
        progressHandler: @escaping (MigrationProgress) -> Void
    ) async throws -> MigrationResult {
        let startTime = Date()
        var result = MigrationResult()

        progressHandler(MigrationProgress(stage: .starting, percentComplete: 0))

        do {
            // Migrate groups
            progressHandler(MigrationProgress(stage: .migratingGroups, percentComplete: 10))
            let groupsResult = try await migrateGroups()
            result.groupsMigrated = groupsResult.count

            // Migrate servers
            progressHandler(MigrationProgress(stage: .migratingServers, percentComplete: 40))
            let serversResult = try await migrateServers()
            result.serversMigrated = serversResult.count

            // Migrate settings
            progressHandler(MigrationProgress(stage: .migratingSettings, percentComplete: 80))
            try migrateSettings()

            // Cleanup
            progressHandler(MigrationProgress(stage: .cleaningUp, percentComplete: 90))
            cleanupLegacyData()

            // Complete
            progressHandler(MigrationProgress(stage: .complete, percentComplete: 100))
            markMigrationComplete()

            result.duration = Date().timeIntervalSince(startTime)
            result.success = true

        } catch {
            result.success = false
            result.errorMessage = error.localizedDescription
            progressHandler(MigrationProgress(
                stage: .failed,
                percentComplete: 0,
                error: error
            ))
        }

        return result
    }

    // MARK: - Specific Migration Steps

    private func migrateGroups() async throws -> (count: Int, errors: [String]) {
        guard let groupsData = userDefaults.data(forKey: LegacyKeys.groups) else {
            // Create default groups if none exist
            let defaults = [
                ServerGroupModel(id: UUID(), name: "Production", colorHex: "#FF6B6B", displayOrder: 0),
                ServerGroupModel(id: UUID(), name: "Staging", colorHex: "#4ECDC4", displayOrder: 1),
                ServerGroupModel(id: UUID(), name: "Development", colorHex: "#45B7D1", displayOrder: 2)
            ]

            for group in defaults {
                try swiftDataService.saveGroup(group)
            }

            return (count: defaults.count, errors: [])
        }

        let decoder = JSONDecoder()
        let legacyGroups = try decoder.decode([LegacyGroup].self, from: groupsData)

        var errors: [String] = []
        var migratedCount = 0

        for legacyGroup in legacyGroups {
            do {
                // Check if group already exists
                let existing = try swiftDataService.fetchGroups().first { $0.name == legacyGroup.name }
                if existing != nil {
                    continue // Skip duplicate
                }

                let group = ServerGroupModel(
                    id: UUID(uuidString: legacyGroup.id) ?? UUID(),
                    name: legacyGroup.name,
                    colorHex: legacyGroup.color,
                    displayOrder: legacyGroup.sortOrder
                )

                try swiftDataService.saveGroup(group)
                migratedCount += 1

            } catch {
                errors.append("Failed to migrate group '\(legacyGroup.name)': \(error.localizedDescription)")
            }
        }

        return (count: migratedCount, errors: errors)
    }

    private func migrateServers() async throws -> (count: Int, errors: [String]) {
        guard let serversData = userDefaults.data(forKey: LegacyKeys.servers) else {
            return (count: 0, errors: [])
        }

        let decoder = JSONDecoder()
        let legacyServers = try decoder.decode([LegacyServer].self, from: serversData)

        var errors: [String] = []
        var migratedCount = 0

        // Get all groups for reference matching
        let groups = try swiftDataService.fetchGroups()

        for legacyServer in legacyServers {
            do {
                let serverId = UUID(uuidString: legacyServer.id) ?? UUID()

                // Check if server already exists
                if let _ = try swiftDataService.fetchServer(byId: serverId) {
                    continue // Skip duplicate
                }

                // Find matching group
                var group: ServerGroupModel? = nil
                if let groupId = legacyServer.groupId,
                   let uuid = UUID(uuidString: groupId) {
                    group = groups.first { $0.id == uuid }
                }

                let server = ServerModel(
                    id: serverId,
                    name: legacyServer.name,
                    host: legacyServer.host,
                    port: legacyServer.port,
                    username: legacyServer.username,
                    authType: AuthType(rawValue: legacyServer.authType) ?? .agent,
                    privateKeyPath: legacyServer.privateKeyPath,
                    group: group,
                    tags: legacyServer.tags,
                    notes: legacyServer.notes,
                    jumpHost: legacyServer.jumpHost,
                    startupCommand: legacyServer.startupCommand,
                    useSSHConfig: legacyServer.useSSHConfig,
                    isFavorite: legacyServer.isFavorite,
                    displayOrder: migratedCount
                )

                server.lastConnected = legacyServer.lastConnected
                server.createdAt = Date() // We don't know original creation date

                try swiftDataService.saveServer(server)
                migratedCount += 1

                // Handle password migration if needed
                if legacyServer.authType == "password" {
                    // Passwords should already be in Keychain, linked by server ID
                    // The keychain service uses server.id.uuidString as the account
                    // Legacy passwords might use the old ID format
                }

            } catch {
                errors.append("Failed to migrate server '\(legacyServer.name)': \(error.localizedDescription)")
            }
        }

        return (count: migratedCount, errors: errors)
    }

    private func migrateSettings() throws {
        let settings = try swiftDataService.fetchSettings()

        // Migrate connection mode
        if let savedMode = userDefaults.string(forKey: LegacyKeys.connectionMode),
           let mode = ConnectionMode(rawValue: savedMode) {
            settings.connectionMode = mode
        }

        // Migrate sidebar visibility
        let sidebarVisible = userDefaults.bool(forKey: LegacyKeys.sidebarVisible)
        settings.sidebarVisibility = sidebarVisible ? .visible : .automatic

        // Migrate startup behavior based on previous state
        settings.startupBehavior = .showWindow

        try swiftDataService.updateSettings(settings)
    }

    // MARK: - Schema Migration

    /// Handle migration between SwiftData schema versions
    func migrateSchemaIfNeeded() async throws {
        let currentVersion = currentSchemaVersion

        // Skip if already at latest
        if currentVersion == .current {
            return
        }

        // Perform incremental migrations
        for version in DataSchemaVersion.allCases {
            if version.rawValue > currentVersion.rawValue && version != .v1 {
                try await migrateToSchema(version)
            }
        }

        // Update stored version
        userDefaults.set(DataSchemaVersion.current.rawValue, forKey: schemaVersionKey)
    }

    private func migrateToSchema(_ version: DataSchemaVersion) async throws {
        switch version {
        case .v1:
            // Initial UserDefaults - handled by main migration
            break
        case .v2:
            // SwiftData migration - handled by main migration
            break
        case .v3:
            // Add iCloud sync and encryption fields
            try await migrateToV3()
        default:
            break
        }
    }

    private func migrateToV3() async throws {
        // Migrate all existing models to include cloud sync fields
        let servers = try swiftDataService.fetchServers()
        for server in servers {
            server.isSyncedToCloud = false
        }

        let groups = try swiftDataService.fetchGroups()
        for group in groups {
            group.isSyncedToCloud = false
        }

        let settings = try swiftDataService.fetchSettings()
        settings.enableCloudSync = true

        try swiftDataService.mainContext.save()
    }

    // MARK: - Cleanup

    private func markMigrationComplete() {
        userDefaults.set(true, forKey: migrationKey)
        userDefaults.set(DataSchemaVersion.current.rawValue, forKey: schemaVersionKey)
    }

    private func cleanupLegacyData() {
        // Remove legacy UserDefaults data
        userDefaults.removeObject(forKey: LegacyKeys.servers)
        userDefaults.removeObject(forKey: LegacyKeys.groups)
        userDefaults.removeObject(forKey: LegacyKeys.settings)
        // Keep lastSelectedServer and other UI state
    }

    /// Rollback migration (for emergency recovery)
    func rollbackMigration() async throws {
        // Delete all SwiftData content
        let descriptor = FetchDescriptor<ServerModel>()
        let servers = try swiftDataService.mainContext.fetch(descriptor)
        servers.forEach { swiftDataService.mainContext.delete($0) }

        let groupDescriptor = FetchDescriptor<ServerGroupModel>()
        let groups = try swiftDataService.mainContext.fetch(groupDescriptor)
        groups.forEach { swiftDataService.mainContext.delete($0) }

        try swiftDataService.mainContext.save()

        // Reset migration flag
        userDefaults.set(false, forKey: migrationKey)
        userDefaults.set(1, forKey: schemaVersionKey)
    }

    // MARK: - Legacy Models (for migration only)

    private struct LegacyServer: Codable {
        let id: String
        let name: String
        let host: String
        let port: Int
        let username: String
        let authType: String
        let privateKeyPath: String?
        let groupId: String?
        let tags: [String]
        let notes: String
        let jumpHost: String?
        let startupCommand: String?
        let useSSHConfig: Bool
        let isFavorite: Bool
        let lastConnected: Date?

        // Decode from struct format
        init(from decoder: Decoder) throws {
            let container = try decoder.container(keyedBy: CodingKeys.self)
            id = try container.decode(String.self, forKey: .id)
            name = try container.decode(String.self, forKey: .name)
            host = try container.decode(String.self, forKey: .host)
            port = try container.decode(Int.self, forKey: .port)
            username = try container.decode(String.self, forKey: .username)
            authType = try container.decode(String.self, forKey: .authType)
            privateKeyPath = try container.decodeIfPresent(String.self, forKey: .privateKeyPath)
            groupId = try container.decodeIfPresent(String.self, forKey: .groupId)
            tags = try container.decode([String].self, forKey: .tags)
            notes = try container.decode(String.self, forKey: .notes)
            jumpHost = try container.decodeIfPresent(String.self, forKey: .jumpHost)
            startupCommand = try container.decodeIfPresent(String.self, forKey: .startupCommand)
            useSSHConfig = try container.decode(Bool.self, forKey: .useSSHConfig)
            isFavorite = try container.decode(Bool.self, forKey: .isFavorite)
            lastConnected = try container.decodeIfPresent(Date.self, forKey: .lastConnected)
        }

        enum CodingKeys: String, CodingKey {
            case id, name, host, port, username
            case authType, privateKeyPath, groupId, tags, notes
            case jumpHost, startupCommand, useSSHConfig
            case isFavorite, lastConnected
        }
    }

    private struct LegacyGroup: Codable {
        let id: String
        let name: String
        let color: String?
        let sortOrder: Int

        init(from decoder: Decoder) throws {
            let container = try decoder.container(keyedBy: CodingKeys.self)
            id = try container.decode(String.self, forKey: .id)
            name = try container.decode(String.self, forKey: .name)
            color = try container.decodeIfPresent(String.self, forKey: .color)
            sortOrder = try container.decode(Int.self, forKey: .sortOrder)
        }

        enum CodingKeys: String, CodingKey {
            case id, name, color, sortOrder
        }
    }
}

// MARK: - Migration Result Types

struct MigrationResult {
    var success: Bool = false
    var serversMigrated: Int = 0
    var groupsMigrated: Int = 0
    var serverErrors: [String] = []
    var groupErrors: [String] = []
    var duration: TimeInterval = 0
    var errorMessage: String? = nil

    var hasErrors: Bool {
        !serverErrors.isEmpty || !groupErrors.isEmpty
    }

    var summary: String {
        if success {
            var parts = [
                "Migration completed in \(String(format: "%.2f", duration))s",
                "Servers: \(serversMigrated) migrated"
            ]
            if groupsMigrated > 0 {
                parts.append("Groups: \(groupsMigrated) migrated")
            }
            if hasErrors {
                parts.append("Warnings: \(serverErrors.count + groupErrors.count)")
            }
            return parts.joined(separator: "\n")
        } else {
            return "Migration failed: \(errorMessage ?? "Unknown error")"
        }
    }
}

struct MigrationProgress {
    let stage: MigrationStage
    let percentComplete: Double
    let error: Error?

    init(stage: MigrationStage, percentComplete: Double, error: Error? = nil) {
        self.stage = stage
        self.percentComplete = percentComplete
        self.error = error
    }
}

enum MigrationStage: String {
    case starting = "Starting migration..."
    case migratingGroups = "Migrating server groups..."
    case migratingServers = "Migrating servers..."
    case migratingSettings = "Migrating settings..."
    case cleaningUp = "Cleaning up..."
    case complete = "Migration complete!"
    case failed = "Migration failed"
}

// MARK: - Encryption Migration

extension DataMigrationService {
    /// Re-encrypt all sensitive data with a new key (for key rotation)
    func reencryptAllData() async throws {
        let servers = try swiftDataService.fetchServers()

        for server in servers {
            // Re-encrypt private key path if present
            if let keyPath = server.privateKeyPath {
                // The value is already encrypted in SwiftData via @Attribute(.encrypted)
                // This would trigger a re-save with new encryption
                server.privateKeyPath = keyPath
            }
        }

        try swiftDataService.mainContext.save()
    }
}
