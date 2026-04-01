// XCTest utilities for EasySSH
// This file provides test helpers and extensions

import XCTest

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
        authType: Server.AuthType = .password,
        groupId: String? = nil,
        tags: [String] = []
    ) -> Server {
        Server(
            id: id,
            name: name,
            host: host,
            port: port,
            username: username,
            authType: authType,
            groupId: groupId,
            tags: tags
        )
    }

    static func makeGroup(
        id: String = UUID().uuidString,
        name: String = "Test Group",
        serverIds: [String] = []
    ) -> ServerGroup {
        ServerGroup(
            id: id,
            name: name,
            serverIds: serverIds
        )
    }
}

// MARK: - Mock Types for Testing

#if DEBUG
class MockSSHSession: SSHSessionProtocol {
    var isConnected = false
    var lastCommand: String?
    var mockOutput = ""

    func connect(host: String, port: Int, username: String, password: String?) throws {
        isConnected = true
    }

    func disconnect() {
        isConnected = false
    }

    func execute(_ command: String) throws -> String {
        lastCommand = command
        return mockOutput
    }
}

protocol SSHSessionProtocol {
    var isConnected: Bool { get }
    func connect(host: String, port: Int, username: String, password: String?) throws
    func disconnect()
    func execute(_ command: String) throws -> String
}
#endif