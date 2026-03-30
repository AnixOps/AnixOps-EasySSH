import Foundation

// FFI函数声明
@_silgen_name("easyssh_init")
func easyssh_init() -> OpaquePointer?

@_silgen_name("easyssh_destroy")
func easyssh_destroy(_ handle: OpaquePointer?)

@_silgen_name("easyssh_get_servers")
func easyssh_get_servers(_ handle: OpaquePointer?) -> UnsafeMutablePointer<CChar>?

@_silgen_name("easyssh_add_server")
func easyssh_add_server(_ handle: OpaquePointer?, _ json: UnsafePointer<CChar>?) -> Int32

@_silgen_name("easyssh_delete_server")
func easyssh_delete_server(_ handle: OpaquePointer?, _ id: UnsafePointer<CChar>?) -> Int32

@_silgen_name("easyssh_get_groups")
func easyssh_get_groups(_ handle: OpaquePointer?) -> UnsafeMutablePointer<CChar>?

@_silgen_name("easyssh_add_group")
func easyssh_add_group(_ handle: OpaquePointer?, _ json: UnsafePointer<CChar>?) -> Int32

@_silgen_name("easyssh_connect_native")
func easyssh_connect_native(_ handle: OpaquePointer?, _ id: UnsafePointer<CChar>?) -> Int32

@_silgen_name("easyssh_free_string")
func easyssh_free_string(_ s: UnsafeMutablePointer<CChar>?)

@_silgen_name("easyssh_version")
func easyssh_version() -> UnsafePointer<CChar>?

/// Bridge between Swift UI and Rust core library
public class EasySSHCoreBridge {
    private var coreHandle: OpaquePointer?

    public init() {
        coreHandle = easyssh_init()
    }

    deinit {
        if let handle = coreHandle {
            easyssh_destroy(handle)
        }
    }

    public func getServers() -> [Server] {
        guard let handle = coreHandle else { return [] }
        guard let cString = easyssh_get_servers(handle) else { return [] }
        defer { easyssh_free_string(cString) }

        guard let jsonString = String(validatingUTF8: cString) else { return [] }

        do {
            let data = jsonString.data(using: .utf8)!
            return try JSONDecoder().decode([Server].self, from: data)
        } catch {
            print("Failed to decode servers: \(error)")
            return []
        }
    }

    public func getGroups() -> [ServerGroup] {
        guard let handle = coreHandle else { return [] }
        guard let cString = easyssh_get_groups(handle) else { return [] }
        defer { easyssh_free_string(cString) }

        guard let jsonString = String(validatingUTF8: cString) else { return [] }

        do {
            let data = jsonString.data(using: .utf8)!
            return try JSONDecoder().decode([ServerGroup].self, from: data)
        } catch {
            print("Failed to decode groups: \(error)")
            return []
        }
    }

    public func addServer(_ server: Server) -> Bool {
        guard let handle = coreHandle else { return false }

        do {
            let data = try JSONEncoder().encode(server)
            guard let jsonString = String(data: data, encoding: .utf8) else { return false }

            let result = jsonString.withCString { cStr in
                easyssh_add_server(handle, cStr)
            }
            return result == 0
        } catch {
            return false
        }
    }

    public func deleteServer(id: String) -> Bool {
        guard let handle = coreHandle else { return false }

        let result = id.withCString { cId in
            easyssh_delete_server(handle, cId)
        }
        return result == 0
    }

    public func connectNative(server: Server) {
        guard let handle = coreHandle else { return }

        let result = server.id.withCString { cId in
            easyssh_connect_native(handle, cId)
        }

        if result != 0 {
            print("Failed to connect to server")
        }
    }
}
