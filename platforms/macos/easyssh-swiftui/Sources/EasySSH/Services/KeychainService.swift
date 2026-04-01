import Foundation
import Security

/// macOS Keychain service for secure credential storage
public final class KeychainService {
    public static let shared = KeychainService()
    private let service = "com.anixops.easyssh"

    private init() {}

    // MARK: - Password Operations

    public func savePassword(_ password: String, for serverId: String) throws {
        guard let passwordData = password.data(using: .utf8) else {
            throw KeychainError.invalidData
        }

        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: serverId,
            kSecValueData as String: passwordData,
            kSecAttrAccessible as String: kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly
        ]

        // Delete any existing item first
        SecItemDelete(query as CFDictionary)

        let status = SecItemAdd(query as CFDictionary, nil)
        guard status == errSecSuccess else {
            throw KeychainError.saveFailed(status: status)
        }
    }

    public func getPassword(for serverId: String) throws -> String? {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: serverId,
            kSecReturnData as String: true,
            kSecMatchLimit as String: kSecMatchLimitOne
        ]

        var result: AnyObject?
        let status = SecItemCopyMatching(query as CFDictionary, &result)

        guard status == errSecSuccess else {
            if status == errSecItemNotFound {
                return nil
            }
            throw KeychainError.readFailed(status: status)
        }

        guard let data = result as? Data,
              let password = String(data: data, encoding: .utf8) else {
            throw KeychainError.invalidData
        }

        return password
    }

    public func deletePassword(for serverId: String) throws {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: serverId
        ]

        let status = SecItemDelete(query as CFDictionary)
        guard status == errSecSuccess || status == errSecItemNotFound else {
            throw KeychainError.deleteFailed(status: status)
        }
    }

    public func updatePassword(_ password: String, for serverId: String) throws {
        guard let passwordData = password.data(using: .utf8) else {
            throw KeychainError.invalidData
        }

        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: serverId
        ]

        let attributes: [String: Any] = [
            kSecValueData as String: passwordData
        ]

        let status = SecItemUpdate(query as CFDictionary, attributes as CFDictionary)

        if status == errSecItemNotFound {
            // Item doesn't exist, create it
            try savePassword(password, for: serverId)
        } else if status != errSecSuccess {
            throw KeychainError.updateFailed(status: status)
        }
    }

    // MARK: - SSH Key Operations

    public func saveSSHKey(_ keyData: Data, name: String, isPrivate: Bool) throws {
        let keyType = isPrivate ? "private" : "public"
        let account = "\(name).\(keyType)"

        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: "\(service).keys",
            kSecAttrAccount as String: account,
            kSecValueData as String: keyData,
            kSecAttrAccessible as String: kSecAttrAccessibleWhenUnlockedThisDeviceOnly
        ]

        SecItemDelete(query as CFDictionary)

        let status = SecItemAdd(query as CFDictionary, nil)
        guard status == errSecSuccess else {
            throw KeychainError.saveFailed(status: status)
        }
    }

    public func getSSHKey(name: String, isPrivate: Bool) throws -> Data? {
        let keyType = isPrivate ? "private" : "public"
        let account = "\(name).\(keyType)"

        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: "\(service).keys",
            kSecAttrAccount as String: account,
            kSecReturnData as String: true,
            kSecMatchLimit as String: kSecMatchLimitOne
        ]

        var result: AnyObject?
        let status = SecItemCopyMatching(query as CFDictionary, &result)

        guard status == errSecSuccess else {
            if status == errSecItemNotFound {
                return nil
            }
            throw KeychainError.readFailed(status: status)
        }

        return result as? Data
    }

    // MARK: - Generic Data Storage

    public func saveData(_ data: Data, key: String) throws {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: key,
            kSecValueData as String: data,
            kSecAttrAccessible as String: kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly
        ]

        SecItemDelete(query as CFDictionary)

        let status = SecItemAdd(query as CFDictionary, nil)
        guard status == errSecSuccess else {
            throw KeychainError.saveFailed(status: status)
        }
    }

    public func getData(key: String) throws -> Data? {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: key,
            kSecReturnData as String: true,
            kSecMatchLimit as String: kSecMatchLimitOne
        ]

        var result: AnyObject?
        let status = SecItemCopyMatching(query as CFDictionary, &result)

        guard status == errSecSuccess else {
            if status == errSecItemNotFound {
                return nil
            }
            throw KeychainError.readFailed(status: status)
        }

        return result as? Data
    }

    // MARK: - Bulk Operations

    public func deleteAllCredentials() throws {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service
        ]

        let status = SecItemDelete(query as CFDictionary)
        guard status == errSecSuccess || status == errSecItemNotFound else {
            throw KeychainError.deleteFailed(status: status)
        }

        // Also delete SSH keys
        let keyQuery: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: "\(service).keys"
        ]

        SecItemDelete(keyQuery as CFDictionary)
    }

    public func listAllStoredItems() throws -> [String] {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecReturnAttributes as String: true,
            kSecMatchLimit as String: kSecMatchLimitAll
        ]

        var result: AnyObject?
        let status = SecItemCopyMatching(query as CFDictionary, &result)

        guard status == errSecSuccess else {
            if status == errSecItemNotFound {
                return []
            }
            throw KeychainError.readFailed(status: status)
        }

        guard let items = result as? [[String: Any]] else {
            return []
        }

        return items.compactMap { $0[kSecAttrAccount as String] as? String }
    }
}

// MARK: - Errors

public enum KeychainError: Error, LocalizedError {
    case invalidData
    case saveFailed(status: OSStatus)
    case readFailed(status: OSStatus)
    case updateFailed(status: OSStatus)
    case deleteFailed(status: OSStatus)

    public var errorDescription: String? {
        switch self {
        case .invalidData:
            return "Invalid data format"
        case .saveFailed(let status):
            return "Failed to save to Keychain (status: \(status))"
        case .readFailed(let status):
            return "Failed to read from Keychain (status: \(status))"
        case .updateFailed(let status):
            return "Failed to update Keychain item (status: \(status))"
        case .deleteFailed(let status):
            return "Failed to delete Keychain item (status: \(status))"
        }
    }
}

// MARK: - Convenience Extensions

extension KeychainService {
    /// Check if a password exists for a server
    public func hasPassword(for serverId: String) -> Bool {
        (try? getPassword(for: serverId)) != nil
    }

    /// Save with automatic error handling (returns success/failure)
    @discardableResult
    public func savePasswordSafely(_ password: String, for serverId: String) -> Bool {
        do {
            try savePassword(password, for: serverId)
            return true
        } catch {
            print("Keychain save failed: \(error)")
            return false
        }
    }
}
