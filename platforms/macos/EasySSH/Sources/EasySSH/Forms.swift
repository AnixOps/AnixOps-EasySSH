import SwiftUI

struct AddServerView: View {
    @Environment(\.dismiss) var dismiss
    @State private var name = ""
    @State private var host = ""
    @State private var port = "22"
    @State private var username = ""
    @State private var authType: AuthType = .agent

    var body: some View {
        NavigationStack {
            Form {
                Section("Basic Info") {
                    TextField("Name", text: $name)
                        .textFieldStyle(.roundedBorder)
                    TextField("Host", text: $host)
                        .textFieldStyle(.roundedBorder)
                }

                Section("Connection") {
                    TextField("Port", text: $port)
                        .textFieldStyle(.roundedBorder)
                    TextField("Username", text: $username)
                        .textFieldStyle(.roundedBorder)
                }

                Section("Authentication") {
                    Picker("Auth Type", selection: $authType) {
                        Text("SSH Agent").tag(AuthType.agent)
                        Text("Private Key").tag(AuthType.key)
                        Text("Password").tag(AuthType.password)
                    }
                    .pickerStyle(.segmented)
                }
            }
            .formStyle(.grouped)
            .navigationTitle("Add Server")
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Cancel") { dismiss() }
                }
                ToolbarItem(placement: .confirmationAction) {
                    Button("Save") {
                        saveServer()
                        dismiss()
                    }
                    .disabled(!isValid)
                }
            }
        }
        .frame(width: 400, height: 350)
    }

    var isValid: Bool {
        !name.isEmpty && !host.isEmpty && !username.isEmpty
    }

    func saveServer() {
        // TODO: Save via EasySSHCoreBridge
    }
}

struct EditServerView: View {
    let server: Server
    @Environment(\.dismiss) var dismiss
    @State private var name: String
    @State private var host: String
    @State private var port: String
    @State private var username: String
    @State private var authType: AuthType

    init(server: Server) {
        self.server = server
        _name = State(initialValue: server.name)
        _host = State(initialValue: server.host)
        _port = State(initialValue: String(server.port))
        _username = State(initialValue: server.username)
        _authType = State(initialValue: server.authType)
    }

    var body: some View {
        NavigationStack {
            Form {
                Section("Basic Info") {
                    TextField("Name", text: $name)
                        .textFieldStyle(.roundedBorder)
                    TextField("Host", text: $host)
                        .textFieldStyle(.roundedBorder)
                }

                Section("Connection") {
                    TextField("Port", text: $port)
                        .textFieldStyle(.roundedBorder)
                    TextField("Username", text: $username)
                        .textFieldStyle(.roundedBorder)
                }

                Section("Authentication") {
                    Picker("Auth Type", selection: $authType) {
                        Text("SSH Agent").tag(AuthType.agent)
                        Text("Private Key").tag(AuthType.key)
                        Text("Password").tag(AuthType.password)
                    }
                    .pickerStyle(.segmented)
                }
            }
            .formStyle(.grouped)
            .navigationTitle("Edit Server")
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Cancel") { dismiss() }
                }
                ToolbarItem(placement: .confirmationAction) {
                    Button("Save") {
                        updateServer()
                        dismiss()
                    }
                }
            }
        }
        .frame(width: 400, height: 350)
    }

    func updateServer() {
        // TODO: Update via EasySSHCoreBridge
    }
}
