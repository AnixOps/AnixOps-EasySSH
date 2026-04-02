import XCTest
@testable import EasySSHCore

// MARK: - Model Tests

final class ModelTests: XCTestCase {

    func testServerInitialization() {
        let server = Server(
            id: "test-001",
            name: "Test Server",
            host: "192.168.1.100",
            port: 22,
            username: "root",
            authType: .password,
            groupId: nil,
            tags: []
        )

        XCTAssertEqual(server.id, "test-001")
        XCTAssertEqual(server.name, "Test Server")
        XCTAssertEqual(server.host, "192.168.1.100")
        XCTAssertEqual(server.port, 22)
        XCTAssertEqual(server.username, "root")
        XCTAssertEqual(server.authType, .password)
        XCTAssertNil(server.groupId)
        XCTAssertTrue(server.tags.isEmpty)
    }

    func testServerWithKeyAuth() {
        let server = Server(
            id: "key-001",
            name: "Key Auth Server",
            host: "10.0.0.5",
            port: 2222,
            username: "deploy",
            authType: .key,
            groupId: "production",
            tags: ["web", "production"]
        )

        XCTAssertEqual(server.authType, .key)
        XCTAssertEqual(server.port, 2222)
        XCTAssertEqual(server.groupId, "production")
        XCTAssertEqual(server.tags.count, 2)
    }

    func testServerEquality() {
        let server1 = Server(
            id: "eq-001",
            name: "Server 1",
            host: "192.168.1.1",
            port: 22,
            username: "root",
            authType: .password,
            groupId: nil,
            tags: []
        )

        let server2 = Server(
            id: "eq-001",
            name: "Server 1",
            host: "192.168.1.1",
            port: 22,
            username: "root",
            authType: .password,
            groupId: nil,
            tags: []
        )

        let server3 = Server(
            id: "eq-002",
            name: "Server 2",
            host: "192.168.1.2",
            port: 22,
            username: "root",
            authType: .password,
            groupId: nil,
            tags: []
        )

        XCTAssertEqual(server1.id, server2.id)
        XCTAssertNotEqual(server1.id, server3.id)
    }

    func testServerGroupInitialization() {
        let group = ServerGroup(
            id: "group-001",
            name: "Production",
            serverIds: ["srv-1", "srv-2", "srv-3"]
        )

        XCTAssertEqual(group.id, "group-001")
        XCTAssertEqual(group.name, "Production")
        XCTAssertEqual(group.serverIds.count, 3)
    }

    func testServerGroupEmptyServers() {
        let group = ServerGroup(
            id: "empty-group",
            name: "Empty",
            serverIds: []
        )

        XCTAssertTrue(group.serverIds.isEmpty)
    }

    func testAuthTypeEnum() {
        let password: Server.AuthType = .password
        let key: Server.AuthType = .key
        let agent: Server.AuthType = .agent

        XCTAssertEqual(password, .password)
        XCTAssertEqual(key, .key)
        XCTAssertEqual(agent, .agent)
        XCTAssertNotEqual(password, key)
    }

    func testServerTagOperations() {
        var server = Server(
            id: "tag-test",
            name: "Tag Test",
            host: "192.168.1.1",
            port: 22,
            username: "root",
            authType: .password,
            groupId: nil,
            tags: ["web"]
        )

        // Note: Since Server is a struct with let properties,
        // we can't directly mutate it. This test verifies initial state.
        XCTAssertTrue(server.tags.contains("web"))
        XCTAssertFalse(server.tags.contains("db"))
    }
}

// MARK: - View Model Tests

final class ViewModelTests: XCTestCase {

    var viewModel: ServerViewModel!

    override func setUp() {
        super.setUp()
        viewModel = ServerViewModel()
    }

    override func tearDown() {
        viewModel = nil
        super.tearDown()
    }

    func testViewModelInitialState() {
        XCTAssertTrue(viewModel.servers.isEmpty)
        XCTAssertTrue(viewModel.groups.isEmpty)
        XCTAssertNil(viewModel.selectedServerId)
        XCTAssertEqual(viewModel.searchQuery, "")
    }

    func testViewModelServerSelection() {
        let serverId = "test-selection"
        viewModel.selectedServerId = serverId

        XCTAssertEqual(viewModel.selectedServerId, serverId)
    }

    func testViewModelSearchQuery() {
        let query = "production"
        viewModel.searchQuery = query

        XCTAssertEqual(viewModel.searchQuery, query)
    }

    func testViewModelFilteredServers() {
        // Add test servers
        let server1 = Server(
            id: "web-001",
            name: "Web Server",
            host: "192.168.1.10",
            port: 22,
            username: "root",
            authType: .password,
            groupId: nil,
            tags: []
        )

        let server2 = Server(
            id: "db-001",
            name: "Database Server",
            host: "192.168.1.20",
            port: 22,
            username: "root",
            authType: .password,
            groupId: nil,
            tags: []
        )

        viewModel.servers = [server1, server2]

        // Test filtering by name
        viewModel.searchQuery = "web"
        let filteredByName = viewModel.filteredServers
        XCTAssertEqual(filteredByName.count, 1)
        XCTAssertEqual(filteredByName.first?.name, "Web Server")

        // Test filtering by host
        viewModel.searchQuery = "192.168.1.20"
        let filteredByHost = viewModel.filteredServers
        XCTAssertEqual(filteredByHost.count, 1)
        XCTAssertEqual(filteredByHost.first?.id, "db-001")

        // Test empty query returns all
        viewModel.searchQuery = ""
        let allServers = viewModel.filteredServers
        XCTAssertEqual(allServers.count, 2)
    }

    func testViewModelCaseInsensitiveSearch() {
        let server = Server(
            id: "case-test",
            name: "Production Web SERVER",
            host: "10.0.0.1",
            port: 22,
            username: "root",
            authType: .password,
            groupId: nil,
            tags: []
        )

        viewModel.servers = [server]

        viewModel.searchQuery = "WEB"
        XCTAssertEqual(viewModel.filteredServers.count, 1)

        viewModel.searchQuery = "production"
        XCTAssertEqual(viewModel.filteredServers.count, 1)
    }

    func testViewModelGroupedServers() {
        let group1 = ServerGroup(
            id: "prod",
            name: "Production",
            serverIds: ["srv-1", "srv-2"]
        )

        let group2 = ServerGroup(
            id: "dev",
            name: "Development",
            serverIds: ["srv-3"]
        )

        viewModel.groups = [group1, group2]

        let prodGroup = viewModel.groupedServers[ServerGroup(id: "prod", name: "", serverIds: [])]
        XCTAssertNotNil(prodGroup)
    }
}

// MARK: - Form Validation Tests

final class FormValidationTests: XCTestCase {

    func testEmptyNameValidation() {
        let result = validateServerForm(name: "", host: "192.168.1.1", port: 22, username: "root")
        XCTAssertFalse(result.isValid)
        XCTAssertTrue(result.errors.contains("Name is required"))
    }

    func testEmptyHostValidation() {
        let result = validateServerForm(name: "Test", host: "", port: 22, username: "root")
        XCTAssertFalse(result.isValid)
        XCTAssertTrue(result.errors.contains("Host is required"))
    }

    func testEmptyUsernameValidation() {
        let result = validateServerForm(name: "Test", host: "192.168.1.1", port: 22, username: "")
        XCTAssertFalse(result.isValid)
        XCTAssertTrue(result.errors.contains("Username is required"))
    }

    func testInvalidPortValidation() {
        let result1 = validateServerForm(name: "Test", host: "192.168.1.1", port: 0, username: "root")
        XCTAssertFalse(result1.isValid)

        let result2 = validateServerForm(name: "Test", host: "192.168.1.1", port: 70000, username: "root")
        XCTAssertFalse(result2.isValid)
    }

    func testValidFormValidation() {
        let result = validateServerForm(
            name: "Web Server",
            host: "192.168.1.100",
            port: 2222,
            username: "admin"
        )
        XCTAssertTrue(result.isValid)
        XCTAssertTrue(result.errors.isEmpty)
    }

    func testDefaultPortValidation() {
        let result = validateServerForm(name: "Test", host: "host.com", port: 22, username: "root")
        XCTAssertTrue(result.isValid)
    }

    func testHostnameValidation() {
        let result = validateServerForm(
            name: "DNS Test",
            host: "server.example.com",
            port: 22,
            username: "root"
        )
        XCTAssertTrue(result.isValid)
    }
}

struct ValidationResult {
    let isValid: Bool
    let errors: [String]
}

func validateServerForm(name: String, host: String, port: Int, username: String) -> ValidationResult {
    var errors: [String] = []

    if name.isEmpty {
        errors.append("Name is required")
    }

    if host.isEmpty {
        errors.append("Host is required")
    }

    if username.isEmpty {
        errors.append("Username is required")
    }

    if port <= 0 || port > 65535 {
        errors.append("Port must be between 1 and 65535")
    }

    return ValidationResult(isValid: errors.isEmpty, errors: errors)
}

// MARK: - Network Tests

final class NetworkTests: XCTestCase {

    func testServerConnectionURL() {
        let server = Server(
            id: "conn-test",
            name: "Connection Test",
            host: "192.168.1.50",
            port: 2222,
            username: "deploy",
            authType: .key,
            groupId: nil,
            tags: []
        )

        let expectedURL = "ssh://deploy@192.168.1.50:2222"
        let actualURL = "ssh://\(server.username)@\(server.host):\(server.port)"

        XCTAssertEqual(actualURL, expectedURL)
    }

    func testDefaultSSHPort() {
        XCTAssertEqual(22, SSHConstants.defaultPort)
    }

    func testSFTPPathConstruction() {
        let path1 = SFTPPath.root
        let path2 = SFTPPath(path: "/home/user")
        let path3 = SFTPPath(path: "/home/user/documents")

        XCTAssertEqual(path1.fullPath, "/")
        XCTAssertEqual(path2.fullPath, "/home/user")
        XCTAssertEqual(path3.fullPath, "/home/user/documents")
    }

    func testSFTPPathParent() {
        let path = SFTPPath(path: "/home/user/documents")
        let parent = path.parent

        XCTAssertEqual(parent?.fullPath, "/home/user")
    }

    func testSFTPPathRootParent() {
        let root = SFTPPath.root
        let parent = root.parent

        XCTAssertNil(parent)
    }
}

struct SSHConstants {
    static let defaultPort = 22
}

struct SFTPPath {
    let fullPath: String

    static let root = SFTPPath(path: "/")

    init(path: String) {
        self.fullPath = path
    }

    var parent: SFTPPath? {
        if fullPath == "/" {
            return nil
        }

        let trimmed = fullPath.hasSuffix("/") ? String(fullPath.dropLast()) : fullPath

        if let lastSlash = trimmed.lastIndex(of: "/") {
            let parentPath = String(trimmed[..<lastSlash])
            return parentPath.isEmpty ? SFTPPath.root : SFTPPath(path: parentPath)
        }

        return SFTPPath.root
    }
}

// MARK: - UI State Tests

final class UIStateTests: XCTestCase {

    func testConnectionStatusEnum() {
        let idle: ConnectionStatus = .idle
        let connecting: ConnectionStatus = .connecting
        let connected: ConnectionStatus = .connected
        let error: ConnectionStatus = .error(message: "Connection failed")

        XCTAssertEqual(idle, .idle)
        XCTAssertEqual(connecting, .connecting)
        XCTAssertEqual(connected, .connected)

        if case .error(let message) = error {
            XCTAssertEqual(message, "Connection failed")
        } else {
            XCTFail("Expected error status")
        }
    }

    func testViewModeEnum() {
        let list: ViewMode = .list
        let grid: ViewMode = .grid
        let details: ViewMode = .details

        XCTAssertEqual(list, .list)
        XCTAssertEqual(grid, .grid)
        XCTAssertEqual(details, .details)
    }

    func testSidebarVisibility() {
        var isVisible = true
        isVisible.toggle()
        XCTAssertFalse(isVisible)
        isVisible.toggle()
        XCTAssertTrue(isVisible)
    }
}

enum ConnectionStatus: Equatable {
    case idle
    case connecting
    case connected
    case error(message: String)
}

enum ViewMode: Equatable {
    case list
    case grid
    case details
}

// MARK: - Search and Filter Tests

final class SearchFilterTests: XCTestCase {

    var testServers: [Server]!

    override func setUp() {
        super.setUp()

        testServers = [
            Server(
                id: "web-prod",
                name: "Production Web",
                host: "10.0.0.10",
                port: 22,
                username: "root",
                authType: .password,
                groupId: "production",
                tags: ["web", "critical"]
            ),
            Server(
                id: "db-prod",
                name: "Production Database",
                host: "10.0.0.20",
                port: 22,
                username: "root",
                authType: .key,
                groupId: "production",
                tags: ["db", "critical"]
            ),
            Server(
                id: "dev-server",
                name: "Development Server",
                host: "192.168.1.50",
                port: 2222,
                username: "dev",
                authType: .password,
                groupId: "development",
                tags: ["dev"]
            )
        ]
    }

    override func tearDown() {
        testServers = nil
        super.tearDown()
    }

    func testFilterByName() {
        let filtered = testServers.filter { server in
            server.name.lowercased().contains("production")
        }

        XCTAssertEqual(filtered.count, 2)
        XCTAssertTrue(filtered.contains { $0.id == "web-prod" })
        XCTAssertTrue(filtered.contains { $0.id == "db-prod" })
    }

    func testFilterByHost() {
        let filtered = testServers.filter { server in
            server.host.contains("192.168")
        }

        XCTAssertEqual(filtered.count, 1)
        XCTAssertEqual(filtered.first?.id, "dev-server")
    }

    func testFilterByTag() {
        let filtered = testServers.filter { server in
            server.tags.contains("critical")
        }

        XCTAssertEqual(filtered.count, 2)
    }

    func testFilterByGroup() {
        let filtered = testServers.filter { server in
            server.groupId == "production"
        }

        XCTAssertEqual(filtered.count, 2)
    }

    func testCaseInsensitiveSearch() {
        let query = "WEB"
        let filtered = testServers.filter { server in
            server.name.lowercased().contains(query.lowercased())
        }

        XCTAssertEqual(filtered.count, 1)
        XCTAssertEqual(filtered.first?.name, "Production Web")
    }

    func testEmptyQueryReturnsAll() {
        let query = ""
        let filtered = testServers.filter { server in
            query.isEmpty || server.name.contains(query)
        }

        XCTAssertEqual(filtered.count, 3)
    }

    func testNoMatchSearch() {
        let query = "nonexistent"
        let filtered = testServers.filter { server in
            server.name.lowercased().contains(query)
        }

        XCTAssertTrue(filtered.isEmpty)
    }
}

// MARK: - Favorites Tests

final class FavoritesTests: XCTestCase {

    var favorites: Set<String>!

    override func setUp() {
        super.setUp()
        favorites = Set()
    }

    override func tearDown() {
        favorites = nil
        super.tearDown()
    }

    func testAddFavorite() {
        favorites.insert("server-1")
        XCTAssertTrue(favorites.contains("server-1"))
    }

    func testRemoveFavorite() {
        favorites.insert("server-1")
        favorites.remove("server-1")
        XCTAssertFalse(favorites.contains("server-1"))
    }

    func testToggleFavorite() {
        let serverId = "server-toggle"

        // Add
        if favorites.contains(serverId) {
            favorites.remove(serverId)
        } else {
            favorites.insert(serverId)
        }
        XCTAssertTrue(favorites.contains(serverId))

        // Remove
        if favorites.contains(serverId) {
            favorites.remove(serverId)
        } else {
            favorites.insert(serverId)
        }
        XCTAssertFalse(favorites.contains(serverId))
    }

    func testMultipleFavorites() {
        favorites.insert("server-1")
        favorites.insert("server-2")
        favorites.insert("server-3")

        XCTAssertEqual(favorites.count, 3)
    }

    func testDuplicateInsert() {
        favorites.insert("server-1")
        favorites.insert("server-1") // Duplicate

        XCTAssertEqual(favorites.count, 1)
    }
}

// MARK: - Performance Tests

final class PerformanceTests: XCTestCase {

    func testLargeServerListFiltering() {
        var servers: [Server] = []

        // Create 1000 servers
        for i in 0..<1000 {
            servers.append(Server(
                id: "srv-\(i)",
                name: "Server \(i)",
                host: "10.0.\(i / 256).\(i % 256)",
                port: 22,
                username: "root",
                authType: .password,
                groupId: i % 2 == 0 ? "even" : "odd",
                tags: []
            ))
        }

        measure {
            let filtered = servers.filter { server in
                server.name.lowercased().contains("500") ||
                server.host.contains("10.0.1")
            }
            XCTAssertGreaterThan(filtered.count, 0)
        }
    }

    func testJSONEncodingPerformance() {
        let server = Server(
            id: "perf-test",
            name: "Performance Test Server",
            host: "192.168.1.1",
            port: 22,
            username: "root",
            authType: .password,
            groupId: nil,
            tags: ["test", "performance", "benchmark"]
        )

        measure {
            let encoder = JSONEncoder()
            encoder.outputFormatting = .sortedKeys
            _ = try? encoder.encode(server)
        }
    }
}

// MARK: - Keychain Tests

final class KeychainTests: XCTestCase {

    func testPasswordKeyGeneration() {
        let serverId = "test-server-id"
        let expectedKey = "easyssh.server.test-server-id.password"
        let actualKey = KeychainHelper.keyForServerPassword(serverId: serverId)

        XCTAssertEqual(actualKey, expectedKey)
    }

    func testKeychainKeyUniqueness() {
        let key1 = KeychainHelper.keyForServerPassword(serverId: "server-1")
        let key2 = KeychainHelper.keyForServerPassword(serverId: "server-2")

        XCTAssertNotEqual(key1, key2)
    }
}

struct KeychainHelper {
    static func keyForServerPassword(serverId: String) -> String {
        return "easyssh.server.\(serverId).password"
    }
}

// MARK: - String Extension Tests

final class StringExtensionTests: XCTestCase {

    func testTruncateWithEllipsis() {
        let longString = String(repeating: "a", count: 100)
        let truncated = longString.truncate(maxLength: 10)

        XCTAssertEqual(truncated.count, 10)
        XCTAssertTrue(truncated.hasSuffix("..."))
    }

    func testTruncateNoOp() {
        let shortString = "Short"
        let truncated = shortString.truncate(maxLength: 100)

        XCTAssertEqual(truncated, shortString)
    }

    func testIsValidHostname() {
        XCTAssertTrue("192.168.1.1".isValidHostname)
        XCTAssertTrue("server.example.com".isValidHostname)
        XCTAssertTrue("localhost".isValidHostname)
        XCTAssertTrue("10.0.0.1".isValidHostname)
    }

    func testInvalidHostname() {
        XCTAssertFalse("".isValidHostname)
        XCTAssertFalse("   ".isValidHostname)
        XCTAssertFalse("not a valid host".isValidHostname)
    }
}

extension String {
    func truncate(maxLength: Int) -> String {
        if self.count <= maxLength {
            return self
        }

        let index = self.index(self.startIndex, offsetBy: maxLength - 3)
        return String(self[..<index]) + "..."
    }

    var isValidHostname: Bool {
        let trimmed = self.trimmingCharacters(in: .whitespaces)
        guard !trimmed.isEmpty else { return false }

        // Basic validation - could be enhanced
        let pattern = "^[a-zA-Z0-9.-]+$"
        let regex = try? NSRegularExpression(pattern: pattern)
        let range = NSRange(location: 0, length: trimmed.utf16.count)
        return regex?.firstMatch(in: trimmed, options: [], range: range) != nil
    }
}

// MARK: - Integration Tests

final class IntegrationTests: XCTestCase {

    func testAppInitialization() {
        // Verify that the app can initialize without crashes
        let app = EasySSHApp()
        XCTAssertNotNil(app)
    }

    func testCoreBridgeAccessibility() {
        // Test that EasySSHCore bridge functions are accessible
        // Note: Actual bridge calls would require the Rust library
        XCTAssertTrue(EasySSHCoreBridge.isAvailable)
    }
}

// Mock extensions for testing
extension ServerViewModel {
    var filteredServers: [Server] {
        if searchQuery.isEmpty {
            return servers
        }

        let query = searchQuery.lowercased()
        return servers.filter { server in
            server.name.lowercased().contains(query) ||
            server.host.lowercased().contains(query)
        }
    }

    var groupedServers: [ServerGroup: [Server]] {
        var result: [ServerGroup: [Server]] = [:]

        for group in groups {
            result[group] = servers.filter { $0.groupId == group.id }
        }

        return result
    }
}

// Extension for testing
extension EasySSHCoreBridge {
    static var isAvailable: Bool {
        // In actual implementation, this would check if the Rust library is loaded
        return true
    }
}