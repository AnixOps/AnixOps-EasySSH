import Foundation
import CEasySSHCore

/// Comprehensive bridge to EasySSH Rust core
public actor EasySSHCoreBridge {
    private var coreHandle: OpaquePointer?
    private var isInitialized = false

    // MARK: - Initialization

    public init() {
        // Defer actual initialization to async context
    }

    /// Initialize the core connection
    public func initialize() throws {
        guard !isInitialized else { return }

        coreHandle = easyssh_init()
        guard coreHandle != nil else {
            throw BridgeError.initializationFailed
        }

        isInitialized = true
    }

    deinit {
        if let handle = coreHandle {
            easyssh_destroy(handle)
        }
    }

    // MARK: - Server Operations

    public func getServers() async throws -> [Server] {
        try ensureInitialized()

        guard let handle = coreHandle else {
            throw BridgeError.notInitialized
        }

        return try await withCheckedThrowingContinuation { continuation in
            DispatchQueue.global(qos: .userInitiated).async {
                guard let cString = easyssh_get_servers(handle) else {
                    continuation.resume(throwing: BridgeError.noData)
                    return
                }
                defer { easyssh_free_string(cString) }

                guard let jsonString = String(validatingUTF8: cString) else {
                    continuation.resume(throwing: BridgeError.invalidData)
                    return
                }

                do {
                    let data = jsonString.data(using: .utf8)!
                    let decoder = JSONDecoder()
                    decoder.dateDecodingStrategy = .iso8601
                    let servers = try decoder.decode([Server].self, from: data)
                    continuation.resume(returning: servers)
                } catch {
                    continuation.resume(throwing: BridgeError.decodingFailed(error.localizedDescription))
                }
            }
        }
    }

    public func getServer(id: String) async throws -> Server {
        try ensureInitialized()

        guard let handle = coreHandle else {
            throw BridgeError.notInitialized
        }

        return try await withCheckedThrowingContinuation { continuation in
            id.withCString { cId in
                guard let cString = easyssh_get_server(handle, cId) else {
                    continuation.resume(throwing: BridgeError.serverNotFound)
                    return
                }
                defer { easyssh_free_string(cString) }

                guard let jsonString = String(validatingUTF8: cString) else {
                    continuation.resume(throwing: BridgeError.invalidData)
                    return
                }

                do {
                    let data = jsonString.data(using: .utf8)!
                    let server = try JSONDecoder().decode(Server.self, from: data)
                    continuation.resume(returning: server)
                } catch {
                    continuation.resume(throwing: BridgeError.decodingFailed(error.localizedDescription))
                }
            }
        }
    }

    public func addServer(_ server: Server) async throws {
        try ensureInitialized()

        guard let handle = coreHandle else {
            throw BridgeError.notInitialized
        }

        let jsonData = try JSONEncoder().encode(server)
        guard let jsonString = String(data: jsonData, encoding: .utf8) else {
            throw BridgeError.encodingFailed
        }

        return try await withCheckedThrowingContinuation { continuation in
            jsonString.withCString { cJson in
                let result = easyssh_add_server(handle, cJson)
                if result == 0 {
                    continuation.resume()
                } else {
                    continuation.resume(throwing: BridgeError.operationFailed("Failed to add server"))
                }
            }
        }
    }

    public func updateServer(_ server: Server) async throws {
        try ensureInitialized()

        guard let handle = coreHandle else {
            throw BridgeError.notInitialized
        }

        let jsonData = try JSONEncoder().encode(server)
        guard let jsonString = String(data: jsonData, encoding: .utf8) else {
            throw BridgeError.encodingFailed
        }

        return try await withCheckedThrowingContinuation { continuation in
            jsonString.withCString { cJson in
                let result = easyssh_update_server(handle, cJson)
                if result == 0 {
                    continuation.resume()
                } else {
                    continuation.resume(throwing: BridgeError.operationFailed("Failed to update server"))
                }
            }
        }
    }

    public func deleteServer(id: String) async -> Bool {
        guard isInitialized, let handle = coreHandle else { return false }

        return await withCheckedContinuation { continuation in
            id.withCString { cId in
                let result = easyssh_delete_server(handle, cId)
                continuation.resume(returning: result == 0)
            }
        }
    }

    // MARK: - Group Operations

    public func getGroups() async throws -> [ServerGroup] {
        try ensureInitialized()

        guard let handle = coreHandle else {
            throw BridgeError.notInitialized
        }

        return try await withCheckedThrowingContinuation { continuation in
            DispatchQueue.global(qos: .userInitiated).async {
                guard let cString = easyssh_get_groups(handle) else {
                    continuation.resume(returning: [])
                    return
                }
                defer { easyssh_free_string(cString) }

                guard let jsonString = String(validatingUTF8: cString) else {
                    continuation.resume(returning: [])
                    return
                }

                do {
                    let data = jsonString.data(using: .utf8)!
                    let groups = try JSONDecoder().decode([ServerGroup].self, from: data)
                    continuation.resume(returning: groups)
                } catch {
                    continuation.resume(returning: [])
                }
            }
        }
    }

    public func addGroup(_ group: ServerGroup) async throws {
        try ensureInitialized()

        guard let handle = coreHandle else {
            throw BridgeError.notInitialized
        }

        let jsonData = try JSONEncoder().encode(group)
        guard let jsonString = String(data: jsonData, encoding: .utf8) else {
            throw BridgeError.encodingFailed
        }

        return try await withCheckedThrowingContinuation { continuation in
            jsonString.withCString { cJson in
                let result = easyssh_add_group(handle, cJson)
                if result == 0 {
                    continuation.resume()
                } else {
                    continuation.resume(throwing: BridgeError.operationFailed("Failed to add group"))
                }
            }
        }
    }

    // MARK: - Connection Operations

    /// Connect using native terminal (Lite mode)
    public func connectNative(server: Server, password: String? = nil) async throws {
        try ensureInitialized()

        guard let handle = coreHandle else {
            throw BridgeError.notInitialized
        }

        return try await withCheckedThrowingContinuation { continuation in
            server.id.withCString { cId in
                let result: Int32
                if let password = password {
                    result = password.withCString { cPassword in
                        easyssh_connect_native_with_password(handle, cId, cPassword)
                    }
                } else {
                    result = easyssh_connect_native(handle, cId)
                }

                if result == 0 {
                    continuation.resume()
                } else {
                    continuation.resume(throwing: BridgeError.connectionFailed)
                }
            }
        }
    }

    /// Connect SSH session (Standard/Pro mode)
    public func sshConnect(server: Server, password: String? = nil) async throws -> SessionMetadata {
        try ensureInitialized()

        guard let handle = coreHandle else {
            throw BridgeError.notInitialized
        }

        return try await withCheckedThrowingContinuation { continuation in
            server.id.withCString { cId in
                let result: UnsafeMutablePointer<CChar>?
                if let password = password {
                    result = password.withCString { cPassword in
                        easyssh_ssh_connect_with_password(handle, cId, cPassword)
                    }
                } else {
                    result = easyssh_ssh_connect(handle, cId)
                }

                guard let cString = result else {
                    continuation.resume(throwing: BridgeError.connectionFailed)
                    return
                }
                defer { easyssh_free_string(cString) }

                guard let jsonString = String(validatingUTF8: cString) else {
                    continuation.resume(throwing: BridgeError.invalidData)
                    return
                }

                do {
                    let data = jsonString.data(using: .utf8)!
                    let metadata = try JSONDecoder().decode(SessionMetadata.self, from: data)
                    continuation.resume(returning: metadata)
                } catch {
                    continuation.resume(throwing: BridgeError.decodingFailed(error.localizedDescription))
                }
            }
        }
    }

    public func sshDisconnect(sessionId: String) async {
        guard isInitialized, let handle = coreHandle else { return }

        await withCheckedContinuation { continuation in
            sessionId.withCString { cId in
                easyssh_ssh_disconnect(handle, cId)
                continuation.resume()
            }
        }
    }

    public func sshExecute(sessionId: String, command: String) async throws -> String {
        try ensureInitialized()

        guard let handle = coreHandle else {
            throw BridgeError.notInitialized
        }

        return try await withCheckedThrowingContinuation { continuation in
            sessionId.withCString { cSessionId in
                command.withCString { cCommand in
                    guard let cResult = easyssh_ssh_execute(handle, cSessionId, cCommand) else {
                        continuation.resume(throwing: BridgeError.commandFailed)
                        return
                    }
                    defer { easyssh_free_string(cResult) }

                    if let result = String(validatingUTF8: cResult) {
                        continuation.resume(returning: result)
                    } else {
                        continuation.resume(throwing: BridgeError.invalidData)
                    }
                }
            }
        }
    }

    // MARK: - Import/Export

    public func importSSHConfig() async throws -> Int {
        try ensureInitialized()

        guard let handle = coreHandle else {
            throw BridgeError.notInitialized
        }

        return try await withCheckedThrowingContinuation { continuation in
            let count = easyssh_import_ssh_config(handle)
            continuation.resume(returning: Int(count))
        }
    }

    public func exportServers(to url: URL) async throws {
        try ensureInitialized()

        guard let handle = coreHandle else {
            throw BridgeError.notInitialized
        }

        return try await withCheckedThrowingContinuation { continuation in
            url.path.withCString { cPath in
                let result = easyssh_export_servers(handle, cPath)
                if result == 0 {
                    continuation.resume()
                } else {
                    continuation.resume(throwing: BridgeError.exportFailed)
                }
            }
        }
    }

    // MARK: - Helpers

    private func ensureInitialized() throws {
        if !isInitialized {
            try initialize()
        }
    }
}

// MARK: - Bridge Errors

public enum BridgeError: Error, LocalizedError {
    case initializationFailed
    case notInitialized
    case noData
    case invalidData
    case decodingFailed(String)
    case encodingFailed
    case operationFailed(String)
    case serverNotFound
    case connectionFailed
    case commandFailed
    case exportFailed

    public var errorDescription: String? {
        switch self {
        case .initializationFailed:
            return "Failed to initialize EasySSH core"
        case .notInitialized:
            return "Bridge not initialized"
        case .noData:
            return "No data returned from core"
        case .invalidData:
            return "Invalid data format"
        case .decodingFailed(let message):
            return "Failed to decode data: \(message)"
        case .encodingFailed:
            return "Failed to encode data"
        case .operationFailed(let message):
            return message
        case .serverNotFound:
            return "Server not found"
        case .connectionFailed:
            return "Failed to establish connection"
        case .commandFailed:
            return "Command execution failed"
        case .exportFailed:
            return "Export failed"
        }
    }
}

// MARK: - FFI Declarations

// Core lifecycle
@_silgen_name("easyssh_init")
func easyssh_init() -> OpaquePointer?

@_silgen_name("easyssh_destroy")
func easyssh_destroy(_ handle: OpaquePointer?)

@_silgen_name("easyssh_free_string")
func easyssh_free_string(_ s: UnsafeMutablePointer<CChar>?)

// Server operations
@_silgen_name("easyssh_get_servers")
func easyssh_get_servers(_ handle: OpaquePointer?) -> UnsafeMutablePointer<CChar>?

@_silgen_name("easyssh_get_server")
func easyssh_get_server(_ handle: OpaquePointer?, _ id: UnsafePointer<CChar>?) -> UnsafeMutablePointer<CChar>?

@_silgen_name("easyssh_add_server")
func easyssh_add_server(_ handle: OpaquePointer?, _ json: UnsafePointer<CChar>?) -> Int32

@_silgen_name("easyssh_update_server")
func easyssh_update_server(_ handle: OpaquePointer?, _ json: UnsafePointer<CChar>?) -> Int32

@_silgen_name("easyssh_delete_server")
func easyssh_delete_server(_ handle: OpaquePointer?, _ id: UnsafePointer<CChar>?) -> Int32

// Group operations
@_silgen_name("easyssh_get_groups")
func easyssh_get_groups(_ handle: OpaquePointer?) -> UnsafeMutablePointer<CChar>?

@_silgen_name("easyssh_add_group")
func easyssh_add_group(_ handle: OpaquePointer?, _ json: UnsafePointer<CChar>?) -> Int32

// Connection operations
@_silgen_name("easyssh_connect_native")
func easyssh_connect_native(_ handle: OpaquePointer?, _ id: UnsafePointer<CChar>?) -> Int32

@_silgen_name("easyssh_connect_native_with_password")
func easyssh_connect_native_with_password(_ handle: OpaquePointer?, _ id: UnsafePointer<CChar>?, _ password: UnsafePointer<CChar>?) -> Int32

@_silgen_name("easyssh_ssh_connect")
func easyssh_ssh_connect(_ handle: OpaquePointer?, _ id: UnsafePointer<CChar>?) -> UnsafeMutablePointer<CChar>?

@_silgen_name("easyssh_ssh_connect_with_password")
func easyssh_ssh_connect_with_password(_ handle: OpaquePointer?, _ id: UnsafePointer<CChar>?, _ password: UnsafePointer<CChar>?) -> UnsafeMutablePointer<CChar>?

@_silgen_name("easyssh_ssh_disconnect")
func easyssh_ssh_disconnect(_ handle: OpaquePointer?, _ sessionId: UnsafePointer<CChar>?)

@_silgen_name("easyssh_ssh_execute")
func easyssh_ssh_execute(_ handle: OpaquePointer?, _ sessionId: UnsafePointer<CChar>?, _ command: UnsafePointer<CChar>?) -> UnsafeMutablePointer<CChar>?

// Import/Export
@_silgen_name("easyssh_import_ssh_config")
func easyssh_import_ssh_config(_ handle: OpaquePointer?) -> Int32

@_silgen_name("easyssh_export_servers")
func easyssh_export_servers(_ handle: OpaquePointer?, _ path: UnsafePointer<CChar>?) -> Int32

@_silgen_name("easyssh_version")
func easyssh_version() -> UnsafePointer<CChar>?
