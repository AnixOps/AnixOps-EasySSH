// XCTest utilities for EasySSH
// This file provides test helpers and extensions

import XCTest
@testable import EasySSH

// MARK: - XCTestCase Extensions

extension XCTestCase {

    /// Wait for a condition to become true
    func waitForCondition(
        timeout: TimeInterval = 5.0,
        condition: () -> Bool
    ) -> Bool {
        let expectation = XCTestExpectation(description: "Wait for condition")
        var result = false

        let timer = Timer.scheduledTimer(withTimeInterval: 0.1, repeats: true) { _ in
            if condition() {
                result = true
                expectation.fulfill()
            }
        }

        wait(for: [expectation], timeout: timeout)
        timer.invalidate()
        return result
    }

    /// Assert async operation completes
    func XCTAssertAsyncCompletes(
        timeout: TimeInterval = 5.0,
        operation: @escaping (@escaping () -> Void) -> Void,
        file: StaticString = #file,
        line: UInt = #line
    ) {
        let expectation = XCTestExpectation(description: "Async operation")

        operation {
            expectation.fulfill()
        }

        wait(for: [expectation], timeout: timeout)
    }
}

// MARK: - Test Data Builders

enum TestDataBuilder {

    static func makeServer(
        id: String = UUID().uuidString,
        name: String = "Test Server",
        host: String = "192.168.1.1",
        port: Int = 22,
        username: String = "root",
        authType: AuthType = .password,
        privateKeyPath: String? = nil,
        groupId: String? = nil,
        tags: [String] = [],
        notes: String = "",
        status: ServerStatus = .unknown,
        isFavorite: Bool = false,
        jumpHost: String? = nil,
        startupCommand: String? = nil,
        useSSHConfig: Bool = true
    ) -> Server {
        Server(
            id: id,
            name: name,
            host: host,
            port: port,
            username: username,
            authType: authType,
            privateKeyPath: privateKeyPath,
            groupId: groupId,
            tags: tags,
            notes: notes,
            status: status,
            isFavorite: isFavorite,
            jumpHost: jumpHost,
            startupCommand: startupCommand,
            useSSHConfig: useSSHConfig
        )
    }

    static func makeGroup(
        id: String = UUID().uuidString,
        name: String = "Test Group",
        color: String? = "#4ECDC4",
        sortOrder: Int = 0
    ) -> ServerGroup {
        ServerGroup(
            id: id,
            name: name,
            color: color,
            sortOrder: sortOrder
        )
    }

    static func makeSSHSession(
        id: String = UUID().uuidString,
        serverId: String = "server-1",
        serverName: String = "Test Server",
        isConnected: Bool = false
    ) -> SSHSession {
        let session = SSHSession(
            id: id,
            serverId: serverId,
            serverName: serverName
        )
        session.isConnected = isConnected
        return session
    }

    static func makeSessionMetadata(
        sessionId: String = UUID().uuidString,
        host: String = "192.168.1.1",
        port: Int = 22,
        username: String = "root",
        connectedAt: Date = Date()
    ) -> SessionMetadata {
        SessionMetadata(
            sessionId: sessionId,
            host: host,
            port: port,
            username: username,
            connectedAt: connectedAt
        )
    }
}

// MARK: - Mock Types for Testing

#if DEBUG
/// Mock implementation of EasySSHCoreBridge for testing
actor MockEasySSHCoreBridge {
    var servers: [Server] = []
    var groups: [ServerGroup] = []
    var shouldFailOperations = false
    var mockSessionId = "mock-session-123"

    func getServers() async throws -> [Server] {
        if shouldFailOperations {
            throw BridgeError.noData
        }
        return servers
    }

    func getServer(id: String) async throws -> Server {
        guard let server = servers.first(where: { $0.id == id }) else {
            throw BridgeError.serverNotFound
        }
        return server
    }

    func addServer(_ server: Server) async throws {
        if shouldFailOperations {
            throw BridgeError.operationFailed("Failed to add server")
        }
        servers.append(server)
    }

    func updateServer(_ server: Server) async throws {
        if shouldFailOperations {
            throw BridgeError.operationFailed("Failed to update server")
        }
        if let index = servers.firstIndex(where: { $0.id == server.id }) {
            servers[index] = server
        } else {
            throw BridgeError.serverNotFound
        }
    }

    func deleteServer(id: String) async -> Bool {
        if shouldFailOperations {
            return false
        }
        servers.removeAll { $0.id == id }
        return true
    }

    func getGroups() async throws -> [ServerGroup] {
        if shouldFailOperations {
            return []
        }
        return groups
    }

    func addGroup(_ group: ServerGroup) async throws {
        if shouldFailOperations {
            throw BridgeError.operationFailed("Failed to add group")
        }
        groups.append(group)
    }

    func connectNative(server: Server, password: String?) async throws {
        if shouldFailOperations {
            throw BridgeError.connectionFailed
        }
    }

    func sshConnect(server: Server, password: String?) async throws -> SessionMetadata {
        if shouldFailOperations {
            throw BridgeError.connectionFailed
        }
        return TestDataBuilder.makeSessionMetadata(
            sessionId: mockSessionId,
            host: server.host,
            port: server.port,
            username: server.username
        )
    }

    func sshDisconnect(sessionId: String) async {
        // Mock disconnect
    }

    func sshExecute(sessionId: String, command: String) async throws -> String {
        if shouldFailOperations {
            throw BridgeError.commandFailed
        }
        return "Mock output for: \(command)"
    }

    func importSSHConfig() async throws -> Int {
        if shouldFailOperations {
            throw BridgeError.operationFailed("Import failed")
        }
        return 3 // Mock 3 servers imported
    }

    func exportServers(to url: URL) async throws {
        if shouldFailOperations {
            throw BridgeError.exportFailed
        }
    }
}

/// Mock KeychainService for testing
class MockKeychainService {
    static let shared = MockKeychainService()
    private var storage: [String: String] = [:]

    private init() {}

    func savePassword(_ password: String, for serverId: String) throws {
        storage[serverId] = password
    }

    func getPassword(for serverId: String) throws -> String? {
        return storage[serverId]
    }

    func deletePassword(for serverId: String) throws {
        storage.removeValue(forKey: serverId)
    }

    func updatePassword(_ password: String, for serverId: String) throws {
        storage[serverId] = password
    }

    func deleteAllCredentials() throws {
        storage.removeAll()
    }

    func hasPassword(for serverId: String) -> Bool {
        return storage[serverId] != nil
    }
}
#endif
