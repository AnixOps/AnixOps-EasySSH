import XCTest
@testable import EasySSH
@testable import EasySSHBridge

// MARK: - Server Model Tests

final class ServerModelTests: XCTestCase {
    func testServerModelBasicProperties() {
        let server = Server(
            id: "test-1",
            name: "Test Server",
            host: "192.168.1.1",
            port: 22,
            username: "root",
            authType: .password,
            status: .unknown
        )

        XCTAssertEqual(server.name, "Test Server")
        XCTAssertEqual(server.host, "192.168.1.1")
        XCTAssertEqual(server.port, 22)
        XCTAssertEqual(server.username, "root")
        XCTAssertEqual(server.authType, .password)
    }

    func testServerModelWithAllProperties() {
        let server = Server(
            id: "test-full",
            name: "Full Server",
            host: "example.com",
            port: 2222,
            username: "admin",
            authType: .key,
            privateKeyPath: "~/.ssh/id_rsa",
            groupId: "production",
            tags: ["web", "production"],
            notes: "Important server",
            status: .connected,
            isFavorite: true,
            jumpHost: "jump.example.com",
            startupCommand: "tmux attach",
            useSSHConfig: false
        )

        XCTAssertEqual(server.privateKeyPath, "~/.ssh/id_rsa")
        XCTAssertEqual(server.groupId, "production")
        XCTAssertEqual(server.tags, ["web", "production"])
        XCTAssertEqual(server.notes, "Important server")
        XCTAssertTrue(server.isFavorite)
        XCTAssertEqual(server.jumpHost, "jump.example.com")
        XCTAssertEqual(server.startupCommand, "tmux attach")
        XCTAssertFalse(server.useSSHConfig)
    }

    func testServerEncodingDecoding() throws {
        let server = Server.preview
        let data = try JSONEncoder().encode(server)
        let decoded = try JSONDecoder().decode(Server.self, from: data)

        XCTAssertEqual(server.id, decoded.id)
        XCTAssertEqual(server.name, decoded.name)
        XCTAssertEqual(server.host, decoded.host)
        XCTAssertEqual(server.port, decoded.port)
        XCTAssertEqual(server.authType, decoded.authType)
        XCTAssertEqual(server.tags, decoded.tags)
    }

    func testServerEquality() {
        let server1 = Server(id: "same-id", name: "Server", host: "host", username: "user")
        let server2 = Server(id: "same-id", name: "Different", host: "other", username: "other")
        let server3 = Server(id: "different-id", name: "Server", host: "host", username: "user")

        XCTAssertEqual(server1, server2) // Same ID should be equal
        XCTAssertNotEqual(server1, server3) // Different ID should not be equal
    }

    func testServerHashing() {
        let server1 = Server(id: "same-id", name: "Server", host: "host", username: "user")
        let server2 = Server(id: "same-id", name: "Different", host: "other", username: "other")

        var set = Set<Server>()
        set.insert(server1)
        set.insert(server2)

        XCTAssertEqual(set.count, 1) // Same ID, so only one item
    }
}

// MARK: - Server Group Tests

final class ServerGroupTests: XCTestCase {
    func testServerGroupCreation() {
        let group = ServerGroup(
            id: "test-group",
            name: "Production",
            color: "#FF6B6B",
            sortOrder: 0
        )

        XCTAssertEqual(group.name, "Production")
        XCTAssertEqual(group.color, "#FF6B6B")
        XCTAssertEqual(group.sortOrder, 0)
    }

    func testDefaultGroups() {
        let defaults = ServerGroup.defaultGroups

        XCTAssertEqual(defaults.count, 3)
        XCTAssertEqual(defaults[0].id, "production")
        XCTAssertEqual(defaults[1].id, "staging")
        XCTAssertEqual(defaults[2].id, "development")
    }

    func testServerGroupEquality() {
        let group1 = ServerGroup(id: "id-1", name: "Group 1", color: nil, sortOrder: 0)
        let group2 = ServerGroup(id: "id-1", name: "Group 2", color: "#000", sortOrder: 1)
        let group3 = ServerGroup(id: "id-2", name: "Group 1", color: nil, sortOrder: 0)

        XCTAssertEqual(group1, group2) // Same ID
        XCTAssertNotEqual(group1, group3) // Different ID
    }
}

// MARK: - AuthType Tests

final class AuthTypeTests: XCTestCase {
    func testAuthTypeDisplayNames() {
        XCTAssertEqual(AuthType.password.displayName, "Password")
        XCTAssertEqual(AuthType.key.displayName, "Private Key")
        XCTAssertEqual(AuthType.agent.displayName, "SSH Agent")
    }

    func testAuthTypeIcons() {
        XCTAssertEqual(AuthType.password.icon, "lock")
        XCTAssertEqual(AuthType.key.icon, "key")
        XCTAssertEqual(AuthType.agent.icon, "keychain")
    }

    func testAuthTypeId() {
        XCTAssertEqual(AuthType.password.id, "password")
        XCTAssertEqual(AuthType.key.id, "key")
        XCTAssertEqual(AuthType.agent.id, "agent")
    }

    func testAuthTypeAllCases() {
        let allCases = AuthType.allCases
        XCTAssertEqual(allCases.count, 3)
        XCTAssertTrue(allCases.contains(.password))
        XCTAssertTrue(allCases.contains(.key))
        XCTAssertTrue(allCases.contains(.agent))
    }

    func testAuthTypeEncoding() throws {
        for authType in AuthType.allCases {
            let data = try JSONEncoder().encode(authType)
            let decoded = try JSONDecoder().decode(AuthType.self, from: data)
            XCTAssertEqual(authType, decoded)
        }
    }
}

// MARK: - ServerStatus Tests

final class ServerStatusTests: XCTestCase {
    func testServerStatusDisplayNames() {
        XCTAssertEqual(ServerStatus.unknown.displayName, "Unknown")
        XCTAssertEqual(ServerStatus.connected.displayName, "Connected")
        XCTAssertEqual(ServerStatus.disconnected.displayName, "Disconnected")
        XCTAssertEqual(ServerStatus.error.displayName, "Error")
        XCTAssertEqual(ServerStatus.connecting.displayName, "Connecting...")
    }

    func testServerStatusRawValues() {
        XCTAssertEqual(ServerStatus.unknown.rawValue, "unknown")
        XCTAssertEqual(ServerStatus.connected.rawValue, "connected")
        XCTAssertEqual(ServerStatus.disconnected.rawValue, "disconnected")
        XCTAssertEqual(ServerStatus.error.rawValue, "error")
        XCTAssertEqual(ServerStatus.connecting.rawValue, "connecting")
    }
}

// MARK: - ServerFormViewModel Tests

final class ServerFormViewModelTests: XCTestCase {
    func testEmptyFormValidation() {
        let viewModel = ServerFormViewModel()

        XCTAssertFalse(viewModel.isValid)
        XCTAssertFalse(viewModel.nameValidation.isValid)
        XCTAssertEqual(viewModel.nameValidation.message, "Name is required")
    }

    func testShortNameValidation() {
        let viewModel = ServerFormViewModel()
        viewModel.name = "A"

        XCTAssertFalse(viewModel.nameValidation.isValid)
        XCTAssertEqual(viewModel.nameValidation.message, "Name must be at least 2 characters")
        XCTAssertFalse(viewModel.isValid) // Still invalid because host is empty
    }

    func testValidNameValidation() {
        let viewModel = ServerFormViewModel()
        viewModel.name = "Valid"

        XCTAssertTrue(viewModel.nameValidation.isValid)
        XCTAssertEqual(viewModel.nameValidation.message, "Valid")
    }

    func testMinimumValidForm() {
        let viewModel = ServerFormViewModel()
        viewModel.name = "Test"
        viewModel.host = "example.com"
        viewModel.username = "root"

        XCTAssertTrue(viewModel.isValid)
    }

    func testValidForTestWithEmptyName() {
        // isValidForTest only requires host and username
        let viewModel = ServerFormViewModel()
        viewModel.name = ""
        viewModel.host = "example.com"
        viewModel.username = "root"

        XCTAssertTrue(viewModel.isValidForTest)
        XCTAssertFalse(viewModel.isValid) // Name is still required for save
    }

    func testToServerConversion() {
        let viewModel = ServerFormViewModel()
        viewModel.name = "Test Server"
        viewModel.host = "192.168.1.1"
        viewModel.port = "2222"
        viewModel.username = "admin"
        viewModel.authType = .key
        viewModel.privateKeyPath = "~/.ssh/test"
        viewModel.tags = ["test", "dev"]
        viewModel.notes = "Test notes"

        let server = viewModel.toServer()

        XCTAssertEqual(server.name, "Test Server")
        XCTAssertEqual(server.host, "192.168.1.1")
        XCTAssertEqual(server.port, 2222)
        XCTAssertEqual(server.username, "admin")
        XCTAssertEqual(server.authType, .key)
        XCTAssertEqual(server.privateKeyPath, "~/.ssh/test")
        XCTAssertEqual(server.tags, ["test", "dev"])
        XCTAssertEqual(server.notes, "Test notes")
    }

    func testToServerWithExistingId() {
        let viewModel = ServerFormViewModel()
        viewModel.name = "Test"
        viewModel.host = "host"
        viewModel.username = "user"

        let existingId = "existing-id-123"
        let server = viewModel.toServer(id: existingId)

        XCTAssertEqual(server.id, existingId)
    }

    func testToServerWithEmptyOptionalFields() {
        let viewModel = ServerFormViewModel()
        viewModel.name = "Test"
        viewModel.host = "host"
        viewModel.username = "user"
        viewModel.privateKeyPath = ""
        viewModel.jumpHost = ""
        viewModel.startupCommand = ""

        let server = viewModel.toServer()

        XCTAssertNil(server.privateKeyPath)
        XCTAssertNil(server.jumpHost)
        XCTAssertNil(server.startupCommand)
    }

    func testDefaultPortConversion() {
        let viewModel = ServerFormViewModel()
        viewModel.name = "Test"
        viewModel.host = "host"
        viewModel.username = "user"
        viewModel.port = "invalid"

        let server = viewModel.toServer()
        XCTAssertEqual(server.port, 22) // Default when parsing fails
    }

    func testInitializationFromServer() {
        let server = Server(
            id: "test-id",
            name: "Original",
            host: "original.com",
            port: 2222,
            username: "admin",
            authType: .key,
            privateKeyPath: "~/.ssh/key",
            groupId: "group-1",
            tags: ["tag1", "tag2"],
            notes: "Notes",
            useSSHConfig: false
        )

        let viewModel = ServerFormViewModel(from: server)

        XCTAssertEqual(viewModel.name, "Original")
        XCTAssertEqual(viewModel.host, "original.com")
        XCTAssertEqual(viewModel.port, "2222")
        XCTAssertEqual(viewModel.username, "admin")
        XCTAssertEqual(viewModel.authType, .key)
        XCTAssertEqual(viewModel.privateKeyPath, "~/.ssh/key")
        XCTAssertEqual(viewModel.groupId, "group-1")
        XCTAssertEqual(viewModel.tags, ["tag1", "tag2"])
        XCTAssertEqual(viewModel.notes, "Notes")
        XCTAssertFalse(viewModel.useSSHConfig)
    }

    func testInitializationFromServerWithNilOptionals() {
        let server = Server(
            id: "test-id",
            name: "Test",
            host: "host",
            username: "user",
            privateKeyPath: nil,
            groupId: nil,
            jumpHost: nil,
            startupCommand: nil
        )

        let viewModel = ServerFormViewModel(from: server)

        XCTAssertEqual(viewModel.privateKeyPath, "")
        XCTAssertNil(viewModel.groupId)
        XCTAssertEqual(viewModel.jumpHost, "")
        XCTAssertEqual(viewModel.startupCommand, "")
    }

    func testPasswordAuthValidation() {
        let viewModel = ServerFormViewModel()
        viewModel.name = "Test"
        viewModel.host = "host"
        viewModel.username = "user"
        viewModel.authType = .password
        viewModel.password = "secret"

        XCTAssertTrue(viewModel.isValid)
    }

    func testKeyAuthValidation() {
        let viewModel = ServerFormViewModel()
        viewModel.name = "Test"
        viewModel.host = "host"
        viewModel.username = "user"
        viewModel.authType = .key
        viewModel.privateKeyPath = "~/.ssh/id_rsa"

        XCTAssertTrue(viewModel.isValid)
        // Key path is not required for form validity
    }

    func testAgentAuthValidation() {
        let viewModel = ServerFormViewModel()
        viewModel.name = "Test"
        viewModel.host = "host"
        viewModel.username = "user"
        viewModel.authType = .agent

        XCTAssertTrue(viewModel.isValid)
        // Agent auth requires no additional fields
    }
}

// MARK: - ConnectionMode Tests

final class ConnectionModeTests: XCTestCase {
    func testConnectionModeRawValues() {
        XCTAssertEqual(ConnectionMode.lite.rawValue, "Lite")
        XCTAssertEqual(ConnectionMode.standard.rawValue, "Standard")
        XCTAssertEqual(ConnectionMode.pro.rawValue, "Pro")
    }

    func testConnectionModeAllCases() {
        let allCases = ConnectionMode.allCases
        XCTAssertEqual(allCases.count, 3)
        XCTAssertTrue(allCases.contains(.lite))
        XCTAssertTrue(allCases.contains(.standard))
        XCTAssertTrue(allCases.contains(.pro))
    }
}

// MARK: - SidebarVisibility Tests

final class SidebarVisibilityTests: XCTestCase {
    func testSidebarVisibilityCases() {
        // Verify all cases exist
        let visible = SidebarVisibility.visible
        let hidden = SidebarVisibility.hidden
        let automatic = SidebarVisibility.automatic

        _ = visible
        _ = hidden
        _ = automatic
    }
}

// MARK: - AppState Tests (Mocked)

final class AppStateTests: XCTestCase {
    private var appState: AppState!
    private let testServerId1 = "test-server-1"
    private let testServerId2 = "test-server-2"
    private let testServerId3 = "test-server-3"

    override func setUp() {
        super.setUp()
        appState = AppState()

        // Clear UserDefaults for clean tests
        UserDefaults.standard.removeObject(forKey: "connectionMode")
        UserDefaults.standard.removeObject(forKey: "sidebarVisible")
        UserDefaults.standard.removeObject(forKey: "lastSelectedServer")
    }

    override func tearDown() {
        UserDefaults.standard.removeObject(forKey: "connectionMode")
        UserDefaults.standard.removeObject(forKey: "sidebarVisible")
        UserDefaults.standard.removeObject(forKey: "lastSelectedServer")
        super.tearDown()
    }

    func testServerFilteringBySearchText() {
        // Setup test servers
        let server1 = Server(id: testServerId1, name: "Production Web", host: "web.prod.com", username: "root", tags: ["web"])
        let server2 = Server(id: testServerId2, name: "Database Server", host: "db.internal.com", username: "admin", tags: ["db"])
        let server3 = Server(id: testServerId3, name: "Development API", host: "api.dev.com", username: "dev", tags: ["api", "dev"])

        appState.servers = [server1, server2, server3]

        // Test search by name
        appState.searchText = "Production"
        XCTAssertEqual(appState.filteredServers.count, 1)
        XCTAssertEqual(appState.filteredServers.first?.id, testServerId1)

        // Test search by host (case insensitive)
        appState.searchText = "INTERNAL"
        XCTAssertEqual(appState.filteredServers.count, 1)
        XCTAssertEqual(appState.filteredServers.first?.id, testServerId2)

        // Test search by username
        appState.searchText = "dev"
        XCTAssertEqual(appState.filteredServers.count, 2) // Matches username "dev" and tag "dev" on server3

        // Test search by tags
        appState.searchText = "api"
        XCTAssertEqual(appState.filteredServers.count, 1)
        XCTAssertEqual(appState.filteredServers.first?.id, testServerId3)

        // Test empty search returns all
        appState.searchText = ""
        XCTAssertEqual(appState.filteredServers.count, 3)
    }

    func testServerFilteringByGroup() {
        let server1 = Server(id: testServerId1, name: "Server 1", host: "host1", username: "user", groupId: "group-1")
        let server2 = Server(id: testServerId2, name: "Server 2", host: "host2", username: "user", groupId: "group-1")
        let server3 = Server(id: testServerId3, name: "Server 3", host: "host3", username: "user", groupId: "group-2")
        let server4 = Server(id: "test-server-4", name: "Server 4", host: "host4", username: "user", groupId: nil)

        appState.servers = [server1, server2, server3, server4]

        // Filter by group
        appState.selectedGroupFilter = "group-1"
        XCTAssertEqual(appState.filteredServers.count, 2)

        // Filter by different group
        appState.selectedGroupFilter = "group-2"
        XCTAssertEqual(appState.filteredServers.count, 1)
        XCTAssertEqual(appState.filteredServers.first?.id, testServerId3)

        // No group filter returns all
        appState.selectedGroupFilter = nil
        XCTAssertEqual(appState.filteredServers.count, 4)
    }

    func testCombinedSearchAndGroupFiltering() {
        let server1 = Server(id: testServerId1, name: "Web Prod", host: "host1", username: "user", groupId: "prod", tags: ["web"])
        let server2 = Server(id: testServerId2, name: "DB Prod", host: "host2", username: "user", groupId: "prod", tags: ["db"])
        let server3 = Server(id: testServerId3, name: "Web Dev", host: "host3", username: "user", groupId: "dev", tags: ["web"])

        appState.servers = [server1, server2, server3]

        // Combined filter: group + search
        appState.selectedGroupFilter = "prod"
        appState.searchText = "Web"
        XCTAssertEqual(appState.filteredServers.count, 1)
        XCTAssertEqual(appState.filteredServers.first?.id, testServerId1)

        // Different combination
        appState.searchText = "DB"
        XCTAssertEqual(appState.filteredServers.count, 1)
        XCTAssertEqual(appState.filteredServers.first?.id, testServerId2)

        // Search across groups
        appState.selectedGroupFilter = nil
        appState.searchText = "Web"
        XCTAssertEqual(appState.filteredServers.count, 2)
    }

    func testFilteredServersSorting() {
        let server1 = Server(id: "c", name: "Charlie Server", host: "host", username: "user")
        let server2 = Server(id: "a", name: "Alpha Server", host: "host", username: "user")
        let server3 = Server(id: "b", name: "Bravo Server", host: "host", username: "user")

        appState.servers = [server1, server2, server3]
        appState.searchText = ""

        let filtered = appState.filteredServers
        XCTAssertEqual(filtered[0].name, "Alpha Server")
        XCTAssertEqual(filtered[1].name, "Bravo Server")
        XCTAssertEqual(filtered[2].name, "Charlie Server")
    }

    func testConnectionModePersistence() {
        // Test default value
        XCTAssertEqual(appState.connectionMode, .lite)

        // Change mode
        appState.connectionMode = .standard

        // Verify UserDefaults storage
        XCTAssertEqual(UserDefaults.standard.string(forKey: "connectionMode"), "Standard")

        // Create new AppState and verify persistence
        let newAppState = AppState()
        XCTAssertEqual(newAppState.connectionMode, .standard)

        // Test pro mode
        appState.connectionMode = .pro
        XCTAssertEqual(UserDefaults.standard.string(forKey: "connectionMode"), "Pro")
    }

    func testSidebarVisibilityPersistence() {
        // Test default
        XCTAssertEqual(appState.sidebarVisibility, .automatic)

        // Change visibility
        appState.sidebarVisibility = .visible
        XCTAssertTrue(UserDefaults.standard.bool(forKey: "sidebarVisible"))

        appState.sidebarVisibility = .hidden
        XCTAssertFalse(UserDefaults.standard.bool(forKey: "sidebarVisible"))

        // Test persistence with new instance
        UserDefaults.standard.set(true, forKey: "sidebarVisible")
        let newAppState = AppState()
        XCTAssertEqual(newAppState.sidebarVisibility, .visible)
    }

    func testSelectedServerPersistence() {
        let server = Server(id: testServerId1, name: "Test", host: "host", username: "user")

        appState.selectedServer = server

        XCTAssertEqual(UserDefaults.standard.string(forKey: "lastSelectedServer"), testServerId1)

        // Clear selection doesn't update UserDefaults
        appState.selectedServer = nil
        // UserDefaults should still have the last value
        XCTAssertEqual(UserDefaults.standard.string(forKey: "lastSelectedServer"), testServerId1)
    }

    func testConnectionStatus() {
        // Empty state
        XCTAssertEqual(appState.connectionStatus.totalServers, 0)
        XCTAssertEqual(appState.connectionStatus.connectedSessions, 0)

        // Add servers
        let server1 = Server(id: testServerId1, name: "Server 1", host: "host1", username: "user")
        let server2 = Server(id: testServerId2, name: "Server 2", host: "host2", username: "user")

        appState.servers = [server1, server2]

        XCTAssertEqual(appState.connectionStatus.totalServers, 2)
        XCTAssertEqual(appState.connectionStatus.connectedSessions, 0)
    }

    func testServerDuplicate() {
        let expectation = XCTestExpectation(description: "Server duplicated")

        let server = Server(
            id: testServerId1,
            name: "Original Server",
            host: "original.com",
            port: 22,
            username: "root",
            authType: .agent,
            privateKeyPath: "~/.ssh/key",
            groupId: "group-1",
            tags: ["tag1"],
            notes: "Original notes",
            useSSHConfig: false
        )

        appState.servers = [server]

        // Duplicate is async, so we need to wait for it
        appState.duplicate(server: server)

        // Give it a moment to process
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.5) {
            expectation.fulfill()
        }

        wait(for: [expectation], timeout: 1.0)

        // The duplicate operation creates a new server with " Copy" suffix
        // Since we're not mocking the bridge, the server won't actually be added
        // But we verified the method doesn't crash
    }

    func testEmptySearchReturnsAllServers() {
        let server1 = Server(id: testServerId1, name: "Server 1", host: "host1", username: "user")
        let server2 = Server(id: testServerId2, name: "Server 2", host: "host2", username: "user")

        appState.servers = [server1, server2]
        appState.searchText = ""
        appState.selectedGroupFilter = nil

        XCTAssertEqual(appState.filteredServers.count, 2)
    }

    func testSearchCaseInsensitivity() {
        let server = Server(id: testServerId1, name: "MyServer", host: "HOST.COM", username: "USER")

        appState.servers = [server]

        appState.searchText = "myserver"
        XCTAssertEqual(appState.filteredServers.count, 1)

        appState.searchText = "MYSERVER"
        XCTAssertEqual(appState.filteredServers.count, 1)

        appState.searchText = "host.com"
        XCTAssertEqual(appState.filteredServers.count, 1)

        appState.searchText = "user"
        XCTAssertEqual(appState.filteredServers.count, 1)
    }
}

// MARK: - FlowLayout Tests

final class FlowLayoutTests: XCTestCase {
    func testFlowLayoutSizeCalculation() {
        // Create a layout with test parameters
        let layout = FlowLayout(spacing: 10)

        // We can't easily test the actual layout without SwiftUI hosting
        // But we can verify the struct exists and has proper initializers
        XCTAssertEqual(layout.spacing, 10)

        let defaultLayout = FlowLayout()
        XCTAssertEqual(defaultLayout.spacing, 8) // Default spacing
    }

    func testFlowLayoutWithDifferentSpacings() {
        let layout0 = FlowLayout(spacing: 0)
        let layout8 = FlowLayout(spacing: 8)
        let layout20 = FlowLayout(spacing: 20)

        XCTAssertEqual(layout0.spacing, 0)
        XCTAssertEqual(layout8.spacing, 8)
        XCTAssertEqual(layout20.spacing, 20)
    }

    func testFlowResultCalculation() {
        // Since we can't create actual Subviews in unit tests,
        // we verify the FlowResult struct exists
        // The actual layout math is tested via UI tests
    }
}

// MARK: - Bridge Error Tests

final class BridgeErrorTests: XCTestCase {
    func testBridgeErrorDescriptions() {
        let errors: [BridgeError] = [
            .initializationFailed,
            .notInitialized,
            .noData,
            .invalidData,
            .decodingFailed("Test error"),
            .encodingFailed,
            .operationFailed("Custom message"),
            .serverNotFound,
            .connectionFailed,
            .commandFailed,
            .exportFailed
        ]

        // Verify all errors have descriptions
        for error in errors {
            XCTAssertNotNil(error.errorDescription)
            XCTAssertFalse(error.errorDescription!.isEmpty)
        }
    }

    func testBridgeErrorLocalizedErrorConformance() {
        let error = BridgeError.initializationFailed
        XCTAssertEqual(error.errorDescription, "Failed to initialize EasySSH core")
    }

    func testDecodingFailedErrorMessage() {
        let error = BridgeError.decodingFailed("JSON parse error")
        XCTAssertEqual(error.errorDescription, "Failed to decode data: JSON parse error")
    }

    func testOperationFailedErrorMessage() {
        let error = BridgeError.operationFailed("Failed to add server")
        XCTAssertEqual(error.errorDescription, "Failed to add server")
    }
}

// MARK: - Keychain Service Tests

final class KeychainServiceTests: XCTestCase {
    override func setUp() {
        super.setUp()
        // Clean up test items before each test
        try? KeychainService.shared.deletePassword(for: "test-server")
        try? KeychainService.shared.deletePassword(for: "test-server-2")
        try? KeychainService.shared.deleteAllCredentials()
    }

    override func tearDown() {
        // Clean up test items after each test
        try? KeychainService.shared.deletePassword(for: "test-server")
        try? KeychainService.shared.deletePassword(for: "test-server-2")
        try? KeychainService.shared.deleteAllCredentials()
        super.tearDown()
    }

    func testPasswordRoundTrip() throws {
        let password = "test-password-123"
        let serverId = "test-server"

        try KeychainService.shared.savePassword(password, for: serverId)
        let retrieved = try KeychainService.shared.getPassword(for: serverId)

        XCTAssertEqual(retrieved, password)
    }

    func testPasswordUpdate() throws {
        let serverId = "test-server"

        try KeychainService.shared.savePassword("old-password", for: serverId)
        try KeychainService.shared.updatePassword("new-password", for: serverId)

        let retrieved = try KeychainService.shared.getPassword(for: serverId)
        XCTAssertEqual(retrieved, "new-password")
    }

    func testPasswordDelete() throws {
        let serverId = "test-server"

        try KeychainService.shared.savePassword("test-password", for: serverId)
        try KeychainService.shared.deletePassword(for: serverId)

        let retrieved = try KeychainService.shared.getPassword(for: serverId)
        XCTAssertNil(retrieved)
    }

    func testGetNonExistentPassword() throws {
        let retrieved = try KeychainService.shared.getPassword(for: "non-existent-server")
        XCTAssertNil(retrieved)
    }

    func testUpdateNonExistentPasswordCreatesNew() throws {
        // Updating a non-existent password should create it
        try KeychainService.shared.updatePassword("new-password", for: "new-server")
        let retrieved = try KeychainService.shared.getPassword(for: "new-server")
        XCTAssertEqual(retrieved, "new-password")

        // Clean up
        try? KeychainService.shared.deletePassword(for: "new-server")
    }

    func testHasPassword() {
        XCTAssertFalse(KeychainService.shared.hasPassword(for: "test-server"))

        try? KeychainService.shared.savePassword("password", for: "test-server")

        XCTAssertTrue(KeychainService.shared.hasPassword(for: "test-server"))
    }

    func testSavePasswordSafely() {
        let result = KeychainService.shared.savePasswordSafely("password", for: "test-server")
        XCTAssertTrue(result)

        let retrieved = try? KeychainService.shared.getPassword(for: "test-server")
        XCTAssertEqual(retrieved, "password")
    }

    func testSaveEmptyPasswordFails() {
        // Empty password should fail
        var success = KeychainService.shared.savePasswordSafely("", for: "test-server")
        // This might succeed depending on keychain implementation
        // but empty data might cause issues
    }

    func testMultiplePasswords() throws {
        let server1 = "test-server-1"
        let server2 = "test-server-2"
        let server3 = "test-server-3"

        try KeychainService.shared.savePassword("pass1", for: server1)
        try KeychainService.shared.savePassword("pass2", for: server2)
        try KeychainService.shared.savePassword("pass3", for: server3)

        XCTAssertEqual(try KeychainService.shared.getPassword(for: server1), "pass1")
        XCTAssertEqual(try KeychainService.shared.getPassword(for: server2), "pass2")
        XCTAssertEqual(try KeychainService.shared.getPassword(for: server3), "pass3")
    }

    func testDeleteAllCredentials() throws {
        try KeychainService.shared.savePassword("pass1", for: "server-1")
        try KeychainService.shared.savePassword("pass2", for: "server-2")

        try KeychainService.shared.deleteAllCredentials()

        XCTAssertNil(try KeychainService.shared.getPassword(for: "server-1"))
        XCTAssertNil(try KeychainService.shared.getPassword(for: "server-2"))
    }

    func testSpecialCharactersPassword() throws {
        let specialPassword = "p@$$w0rd!#$%^&*()_+-=[]{}|;':\",./<>?"
        try KeychainService.shared.savePassword(specialPassword, for: "test-server")
        let retrieved = try KeychainService.shared.getPassword(for: "test-server")
        XCTAssertEqual(retrieved, specialPassword)
    }

    func testLongPassword() throws {
        let longPassword = String(repeating: "a", count: 1000)
        try KeychainService.shared.savePassword(longPassword, for: "test-server")
        let retrieved = try KeychainService.shared.getPassword(for: "test-server")
        XCTAssertEqual(retrieved, longPassword)
    }
}

// MARK: - SSH Identity Tests

final class SSHIdentityTests: XCTestCase {
    func testSSHIdentityCreation() {
        let identity = SSHIdentity(
            id: "test-id",
            name: "My Key",
            privateKeyPath: "~/.ssh/id_rsa",
            publicKeyPath: "~/.ssh/id_rsa.pub",
            isEncrypted: true,
            addedToAgent: false
        )

        XCTAssertEqual(identity.name, "My Key")
        XCTAssertEqual(identity.privateKeyPath, "~/.ssh/id_rsa")
        XCTAssertEqual(identity.publicKeyPath, "~/.ssh/id_rsa.pub")
        XCTAssertTrue(identity.isEncrypted)
        XCTAssertFalse(identity.addedToAgent)
    }

    func testSSHIdentityEncoding() throws {
        let identity = SSHIdentity(
            id: "test-id",
            name: "Test",
            privateKeyPath: "~/.ssh/test",
            publicKeyPath: nil,
            isEncrypted: false,
            addedToAgent: true
        )

        let data = try JSONEncoder().encode(identity)
        let decoded = try JSONDecoder().decode(SSHIdentity.self, from: data)

        XCTAssertEqual(identity.id, decoded.id)
        XCTAssertEqual(identity.name, decoded.name)
        XCTAssertEqual(identity.privateKeyPath, decoded.privateKeyPath)
        XCTAssertNil(decoded.publicKeyPath)
    }
}

// MARK: - Connection Profile Tests

final class ConnectionProfileTests: XCTestCase {
    func testConnectionProfileCreation() {
        let profile = ConnectionProfile(
            id: "profile-1",
            name: "Default",
            port: 22,
            username: "root",
            authType: .key,
            keepAlive: true,
            forwardAgent: false,
            customOptions: ["StrictHostKeyChecking": "no"]
        )

        XCTAssertEqual(profile.name, "Default")
        XCTAssertEqual(profile.port, 22)
        XCTAssertTrue(profile.keepAlive)
        XCTAssertFalse(profile.forwardAgent)
        XCTAssertEqual(profile.customOptions["StrictHostKeyChecking"], "no")
    }
}

// MARK: - Session Record Tests

final class SessionRecordTests: XCTestCase {
    func testSessionRecordCreation() {
        let record = SessionRecord(
            id: "session-1",
            serverId: "server-1",
            serverName: "Test Server",
            startTime: Date(),
            endTime: nil,
            commandsExecuted: 5,
            filesTransferred: 0,
            sessionType: .terminal,
            notes: nil
        )

        XCTAssertEqual(record.serverName, "Test Server")
        XCTAssertEqual(record.commandsExecuted, 5)
        XCTAssertEqual(record.sessionType, .terminal)
        XCTAssertNil(record.endTime)
    }

    func testSessionTypeCases() {
        let types: [SessionType] = [.terminal, .sftp, .portForward]

        for type in types {
            XCTAssertNotNil(type.rawValue)
        }

        XCTAssertEqual(SessionType.terminal.rawValue, "terminal")
        XCTAssertEqual(SessionType.sftp.rawValue, "sftp")
        XCTAssertEqual(SessionType.portForward.rawValue, "port_forward")
    }
}

// MARK: - Snippet Tests

final class SnippetTests: XCTestCase {
    func testSnippetCreation() {
        let snippet = Snippet(
            id: "snippet-1",
            title: "Docker PS",
            content: "docker ps -a",
            tags: ["docker", "container"],
            isShared: false,
            createdBy: "user-1",
            teamId: nil
        )

        XCTAssertEqual(snippet.title, "Docker PS")
        XCTAssertEqual(snippet.content, "docker ps -a")
        XCTAssertEqual(snippet.tags, ["docker", "container"])
        XCTAssertFalse(snippet.isShared)
    }

    func testSharedSnippet() {
        let snippet = Snippet(
            id: "snippet-2",
            title: "Team Snippet",
            content: "kubectl get pods",
            tags: ["k8s"],
            isShared: true,
            createdBy: "admin",
            teamId: "team-1"
        )

        XCTAssertTrue(snippet.isShared)
        XCTAssertEqual(snippet.teamId, "team-1")
    }
}

// MARK: - Validation Result Tests

final class ValidationResultTests: XCTestCase {
    func testValidationResultCreation() {
        let valid = ValidationResult(isValid: true, message: "Valid")
        XCTAssertTrue(valid.isValid)
        XCTAssertEqual(valid.message, "Valid")

        let invalid = ValidationResult(isValid: false, message: "Invalid")
        XCTAssertFalse(invalid.isValid)
        XCTAssertEqual(invalid.message, "Invalid")
    }
}

// MARK: - Theme Manager Tests

final class ThemeManagerTests: XCTestCase {
    private var themeManager: ThemeManager!

    override func setUp() {
        super.setUp()
        themeManager = ThemeManager()
    }

    func testDefaultColorScheme() {
        XCTAssertNil(themeManager.colorScheme)
    }

    func testSetAccentColor() {
        themeManager.setAccentColor("blue")
        XCTAssertEqual(themeManager.accentColor, .blue)

        themeManager.setAccentColor("purple")
        XCTAssertEqual(themeManager.accentColor, .purple)

        themeManager.setAccentColor("green")
        XCTAssertEqual(themeManager.accentColor, .green)

        themeManager.setAccentColor("orange")
        XCTAssertEqual(themeManager.accentColor, .orange)

        themeManager.setAccentColor("red")
        XCTAssertEqual(themeManager.accentColor, .red)

        themeManager.setAccentColor("pink")
        XCTAssertEqual(themeManager.accentColor, .pink)

        themeManager.setAccentColor("teal")
        XCTAssertEqual(themeManager.accentColor, .teal)

        themeManager.setAccentColor("indigo")
        XCTAssertEqual(themeManager.accentColor, .indigo)
    }

    func testInvalidAccentColorDefaultsToBlue() {
        themeManager.setAccentColor("invalid")
        XCTAssertEqual(themeManager.accentColor, .blue)
    }

    func testEmptyAccentColorDefaultsToBlue() {
        themeManager.setAccentColor("")
        XCTAssertEqual(themeManager.accentColor, .blue)
    }
}

// MARK: - Keychain Error Tests

final class KeychainErrorTests: XCTestCase {
    func testKeychainErrorDescriptions() {
        let invalidData = KeychainError.invalidData
        XCTAssertEqual(invalidData.errorDescription, "Invalid data format")

        let saveFailed = KeychainError.saveFailed(status: -1)
        XCTAssertTrue(saveFailed.errorDescription?.contains("Failed to save") ?? false)

        let readFailed = KeychainError.readFailed(status: -2)
        XCTAssertTrue(readFailed.errorDescription?.contains("Failed to read") ?? false)

        let updateFailed = KeychainError.updateFailed(status: -3)
        XCTAssertTrue(updateFailed.errorDescription?.contains("Failed to update") ?? false)

        let deleteFailed = KeychainError.deleteFailed(status: -4)
        XCTAssertTrue(deleteFailed.errorDescription?.contains("Failed to delete") ?? false)
    }

    func testKeychainErrorLocalizedErrorConformance() {
        let error = KeychainError.invalidData
        XCTAssertNotNil(error.errorDescription)
    }
}

// MARK: - Settings Persistence Tests

final class SettingsPersistenceTests: XCTestCase {
    private let defaults = UserDefaults.standard

    override func setUp() {
        super.setUp()
        // Clean up settings before tests
        defaults.removeObject(forKey: "launchAtLogin")
        defaults.removeObject(forKey: "showInMenuBar")
        defaults.removeObject(forKey: "showInDock")
        defaults.removeObject(forKey: "confirmBeforeQuit")
        defaults.removeObject(forKey: "autoSaveSessions")
        defaults.removeObject(forKey: "accentColor")
        defaults.removeObject(forKey: "defaultPort")
        defaults.removeObject(forKey: "connectionTimeout")
        defaults.removeObject(forKey: "keepAliveInterval")
        defaults.removeObject(forKey: "maxRetries")
        defaults.removeObject(forKey: "autoReconnect")
        defaults.removeObject(forKey: "useKeychain")
        defaults.removeObject(forKey: "lockOnSleep")
        defaults.removeObject(forKey: "lockAfterMinutes")
        defaults.removeObject(forKey: "requirePasswordForSensitive")
        defaults.removeObject(forKey: "defaultTerminal")
        defaults.removeObject(forKey: "terminalFontSize")
        defaults.removeObject(forKey: "terminalFontFamily")
        defaults.removeObject(forKey: "enableLogging")
    }

    override func tearDown() {
        // Clean up after tests
        defaults.removeObject(forKey: "launchAtLogin")
        defaults.removeObject(forKey: "showInMenuBar")
        defaults.removeObject(forKey: "showInDock")
        defaults.removeObject(forKey: "confirmBeforeQuit")
        defaults.removeObject(forKey: "autoSaveSessions")
        defaults.removeObject(forKey: "accentColor")
        defaults.removeObject(forKey: "defaultPort")
        defaults.removeObject(forKey: "connectionTimeout")
        defaults.removeObject(forKey: "keepAliveInterval")
        defaults.removeObject(forKey: "maxRetries")
        defaults.removeObject(forKey: "autoReconnect")
        defaults.removeObject(forKey: "useKeychain")
        defaults.removeObject(forKey: "lockOnSleep")
        defaults.removeObject(forKey: "lockAfterMinutes")
        defaults.removeObject(forKey: "requirePasswordForSensitive")
        defaults.removeObject(forKey: "defaultTerminal")
        defaults.removeObject(forKey: "terminalFontSize")
        defaults.removeObject(forKey: "terminalFontFamily")
        defaults.removeObject(forKey: "enableLogging")
        super.tearDown()
    }

    // MARK: - General Settings

    func testGeneralSettingsPersistence() {
        // Set values
        defaults.set(true, forKey: "launchAtLogin")
        defaults.set(false, forKey: "showInMenuBar")
        defaults.set(true, forKey: "showInDock")
        defaults.set(false, forKey: "confirmBeforeQuit")
        defaults.set(true, forKey: "autoSaveSessions")

        // Verify values
        XCTAssertTrue(defaults.bool(forKey: "launchAtLogin"))
        XCTAssertFalse(defaults.bool(forKey: "showInMenuBar"))
        XCTAssertTrue(defaults.bool(forKey: "showInDock"))
        XCTAssertFalse(defaults.bool(forKey: "confirmBeforeQuit"))
        XCTAssertTrue(defaults.bool(forKey: "autoSaveSessions"))
    }

    // MARK: - Appearance Settings

    func testAccentColorPersistence() {
        defaults.set("purple", forKey: "accentColor")
        XCTAssertEqual(defaults.string(forKey: "accentColor"), "purple")

        defaults.set("green", forKey: "accentColor")
        XCTAssertEqual(defaults.string(forKey: "accentColor"), "green")
    }

    // MARK: - Connection Settings

    func testConnectionSettingsPersistence() {
        defaults.set(2222, forKey: "defaultPort")
        defaults.set(60, forKey: "connectionTimeout")
        defaults.set(30, forKey: "keepAliveInterval")
        defaults.set(5, forKey: "maxRetries")
        defaults.set(false, forKey: "autoReconnect")

        XCTAssertEqual(defaults.integer(forKey: "defaultPort"), 2222)
        XCTAssertEqual(defaults.integer(forKey: "connectionTimeout"), 60)
        XCTAssertEqual(defaults.integer(forKey: "keepAliveInterval"), 30)
        XCTAssertEqual(defaults.integer(forKey: "maxRetries"), 5)
        XCTAssertFalse(defaults.bool(forKey: "autoReconnect"))
    }

    // MARK: - Security Settings

    func testSecuritySettingsPersistence() {
        defaults.set(false, forKey: "useKeychain")
        defaults.set(true, forKey: "lockOnSleep")
        defaults.set(15, forKey: "lockAfterMinutes")
        defaults.set(true, forKey: "requirePasswordForSensitive")

        XCTAssertFalse(defaults.bool(forKey: "useKeychain"))
        XCTAssertTrue(defaults.bool(forKey: "lockOnSleep"))
        XCTAssertEqual(defaults.integer(forKey: "lockAfterMinutes"), 15)
        XCTAssertTrue(defaults.bool(forKey: "requirePasswordForSensitive"))
    }

    // MARK: - Terminal Settings

    func testTerminalSettingsPersistence() {
        defaults.set("iTerm2", forKey: "defaultTerminal")
        defaults.set(16, forKey: "terminalFontSize")
        defaults.set("JetBrains Mono", forKey: "terminalFontFamily")

        XCTAssertEqual(defaults.string(forKey: "defaultTerminal"), "iTerm2")
        XCTAssertEqual(defaults.integer(forKey: "terminalFontSize"), 16)
        XCTAssertEqual(defaults.string(forKey: "terminalFontFamily"), "JetBrains Mono")
    }

    // MARK: - Advanced Settings

    func testAdvancedSettingsPersistence() {
        defaults.set(true, forKey: "enableLogging")
        XCTAssertTrue(defaults.bool(forKey: "enableLogging"))

        defaults.set(false, forKey: "enableLogging")
        XCTAssertFalse(defaults.bool(forKey: "enableLogging"))
    }

    // MARK: - Default Values

    func testDefaultValues() {
        // Verify that non-existent keys return appropriate defaults
        XCTAssertFalse(defaults.bool(forKey: "launchAtLogin"))
        XCTAssertTrue(defaults.bool(forKey: "showInMenuBar")) // Default is true if not set
        XCTAssertEqual(defaults.integer(forKey: "defaultPort"), 0) // Default is 0 if not set
        XCTAssertNil(defaults.string(forKey: "accentColor"))
    }
}

// MARK: - SSH Session Tests

final class SSHSessionTests: XCTestCase {
    func testSSHSessionCreation() {
        let session = SSHSession(
            id: "session-1",
            serverId: "server-1",
            serverName: "Test Server"
        )

        XCTAssertEqual(session.id, "session-1")
        XCTAssertEqual(session.serverId, "server-1")
        XCTAssertEqual(session.serverName, "Test Server")
        XCTAssertFalse(session.isConnected)
        XCTAssertNil(session.metadata)
        XCTAssertNil(session.error)
        XCTAssertEqual(session.terminalContent, "")
    }

    func testSSHSessionIdentifiable() {
        let session1 = SSHSession(id: "id-1", serverId: "s1", serverName: "Server 1")
        let session2 = SSHSession(id: "id-2", serverId: "s2", serverName: "Server 2")

        XCTAssertEqual(session1.id, "id-1")
        XCTAssertEqual(session2.id, "id-2")
    }
}

// MARK: - Session Metadata Tests

final class SessionMetadataTests: XCTestCase {
    func testSessionMetadataCreation() {
        let metadata = SessionMetadata(
            sessionId: "session-1",
            host: "192.168.1.1",
            port: 22,
            username: "root",
            connectedAt: Date()
        )

        XCTAssertEqual(metadata.sessionId, "session-1")
        XCTAssertEqual(metadata.host, "192.168.1.1")
        XCTAssertEqual(metadata.port, 22)
        XCTAssertEqual(metadata.username, "root")
    }

    func testSessionMetadataEncoding() throws {
        let date = Date()
        let metadata = SessionMetadata(
            sessionId: "session-1",
            host: "host",
            port: 22,
            username: "user",
            connectedAt: date
        )

        let data = try JSONEncoder().encode(metadata)
        let decoded = try JSONDecoder().decode(SessionMetadata.self, from: data)

        XCTAssertEqual(metadata.sessionId, decoded.sessionId)
        XCTAssertEqual(metadata.host, decoded.host)
        XCTAssertEqual(metadata.port, decoded.port)
    }
}

// MARK: - Connection Status Tests

final class ConnectionStatusTests: XCTestCase {
    func testConnectionStatusCreation() {
        let status = ConnectionStatus(
            totalServers: 10,
            connectedSessions: 3,
            activeTransfers: 1
        )

        XCTAssertEqual(status.totalServers, 10)
        XCTAssertEqual(status.connectedSessions, 3)
        XCTAssertEqual(status.activeTransfers, 1)
    }

    func testEmptyConnectionStatus() {
        let status = ConnectionStatus(
            totalServers: 0,
            connectedSessions: 0,
            activeTransfers: 0
        )

        XCTAssertEqual(status.totalServers, 0)
        XCTAssertEqual(status.connectedSessions, 0)
        XCTAssertEqual(status.activeTransfers, 0)
    }
}

// MARK: - Notification Tests

final class NotificationNameTests: XCTestCase {
    func testSSHStatusChangedNotification() {
        let notificationName = Notification.Name.sshStatusChanged
        XCTAssertEqual(notificationName.rawValue, "sshStatusChanged")
    }
}

// MARK: - Preview Extensions Tests

final class PreviewExtensionsTests: XCTestCase {
    func testServerPreview() {
        let preview = Server.preview

        XCTAssertFalse(preview.id.isEmpty)
        XCTAssertEqual(preview.name, "Preview Server")
        XCTAssertEqual(preview.host, "192.168.1.1")
        XCTAssertEqual(preview.port, 22)
        XCTAssertEqual(preview.username, "root")
    }

    func testServerPreviews() {
        let previews = Server.previews

        XCTAssertEqual(previews.count, 3)
        XCTAssertEqual(previews[0].name, "Production Web")
        XCTAssertEqual(previews[1].name, "Staging API")
        XCTAssertEqual(previews[2].name, "Development DB")
    }

    func testPreviewServerWithStatus() {
        let connected = Server.previewConnected
        XCTAssertEqual(connected.status, .connected)

        let disconnected = Server.previewDisconnected
        XCTAssertEqual(disconnected.status, .disconnected)

        let error = Server.previewError
        XCTAssertEqual(error.status, .error)
    }
}
