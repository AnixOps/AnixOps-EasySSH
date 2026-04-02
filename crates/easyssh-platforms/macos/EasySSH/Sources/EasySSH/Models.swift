import Foundation

struct Server: Identifiable, Codable {
    let id: String
    var name: String
    var host: String
    var port: Int
    var username: String
    var authType: AuthType
    var groupId: String?
    var status: ServerStatus
}

struct ServerGroup: Identifiable, Codable {
    let id: String
    var name: String
    var servers: [String] // Server IDs
}

enum AuthType: String, Codable {
    case password
    case key
    case agent
}

enum ServerStatus: String, Codable {
    case unknown
    case connected
    case disconnected
    case error
}

extension Server {
    static var preview: Server {
        Server(
            id: "1",
            name: "Production Server",
            host: "prod.example.com",
            port: 22,
            username: "admin",
            authType: .agent,
            groupId: nil,
            status: .unknown
        )
    }
}
