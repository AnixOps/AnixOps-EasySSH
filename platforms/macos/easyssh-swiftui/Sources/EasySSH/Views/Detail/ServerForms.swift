import SwiftUI

/// Container for add server sheet
struct AddServerContainer: View {
    @EnvironmentObject var appState: AppState
    @Environment(\.dismiss) private var dismiss

    var body: some View {
        AddServerView { server, password in
            Task {
                try? await appState.addServer(server, password: password)
                dismiss()
            }
        }
    }
}

/// Add new server form
struct AddServerView: View {
    let onSave: (Server, String?) -> Void

    @Environment(\.dismiss) private var dismiss
    @State private var viewModel = ServerFormViewModel()
    @State private var showingKeyPicker = false
    @State private var testingConnection = false
    @State private var testResult: TestResult?

    enum TestResult {
        case success
        case failure(String)
    }

    var body: some View {
        NavigationStack {
            Form {
                // Basic Info Section
                Section {
                    FormRow(icon: "tag", title: "Name") {
                        TextField("My Server", text: $viewModel.name)
                            .textFieldStyle(.roundedBorder)
                    }

                    FormRow(icon: "network", title: "Host") {
                        HStack {
                            TextField("192.168.1.1 or example.com", text: $viewModel.host)
                                .textFieldStyle(.roundedBorder)

                            Text(":")
                                .foregroundStyle(.secondary)

                            TextField("22", text: $viewModel.port)
                                .textFieldStyle(.roundedBorder)
                                .frame(width: 60)
                                .multilineTextAlignment(.center)
                        }
                    }
                } header: {
                    Text("Server Information")
                } footer: {
                    if !viewModel.nameValidation.isValid {
                        ValidationMessage(viewModel.nameValidation.message, isError: true)
                    }
                }

                // Authentication Section
                Section {
                    FormRow(icon: "person", title: "Username") {
                        TextField("root", text: $viewModel.username)
                            .textFieldStyle(.roundedBorder)
                    }

                    Picker("Authentication", selection: $viewModel.authType) {
                        ForEach(AuthType.allCases) { type in
                            Label(type.displayName, systemImage: type.icon)
                                .tag(type)
                        }
                    }
                    .pickerStyle(.segmented)
                    .padding(.vertical, 4)

                    // Dynamic auth fields
                    switch viewModel.authType {
                    case .password:
                        FormRow(icon: "lock", title: "Password") {
                            SecureField("Optional - will prompt if empty", text: $viewModel.password)
                                .textFieldStyle(.roundedBorder)
                        }

                    case .key:
                        FormRow(icon: "key", title: "Private Key") {
                            HStack {
                                Text(viewModel.privateKeyPath.isEmpty ? "Select key file..." : viewModel.privateKeyPath)
                                    .font(.system(size: 13))
                                    .foregroundStyle(viewModel.privateKeyPath.isEmpty ? .secondary : .primary)
                                    .lineLimit(1)

                                Spacer()

                                Button("Browse...") {
                                    showingKeyPicker = true
                                }
                                .buttonStyle(.borderless)
                            }
                        }

                        if !viewModel.privateKeyPath.isEmpty {
                            FormRow(icon: "lock.shield", title: "Key Passphrase") {
                                SecureField("Optional", text: $viewModel.keyPassphrase)
                                    .textFieldStyle(.roundedBorder)
                            }
                        }

                    case .agent:
                        HStack {
                            Image(systemName: "checkmark.shield")
                                .foregroundStyle(.green)
                            Text("Will use SSH agent or local keys")
                                .font(.system(size: 13))
                                .foregroundStyle(.secondary)
                        }
                        .padding(.vertical, 4)
                    }
                } header: {
                    Text("Authentication")
                }

                // Advanced Section
                Section("Advanced Options") {
                    FormRow(icon: "arrow.forward.circle", title: "Jump Host") {
                        TextField("Optional proxy/jump host", text: $viewModel.jumpHost)
                            .textFieldStyle(.roundedBorder)
                    }

                    FormRow(icon: "terminal", title: "Startup Command") {
                        TextField("Command to run after connect", text: $viewModel.startupCommand)
                            .textFieldStyle(.roundedBorder)
                    }

                    Toggle("Use SSH config file options", isOn: $viewModel.useSSHConfig)
                }

                // Group & Tags Section
                Section("Organization") {
                    Picker("Group", selection: $viewModel.groupId) {
                        Text("None").tag(nil as String?)
                        // Groups would be populated here
                    }

                    FormRow(icon: "tag", title: "Tags") {
                        TagInputView(tags: $viewModel.tags)
                    }

                    FormRow(icon: "text.alignleft", title: "Notes") {
                        TextEditor(text: $viewModel.notes)
                            .font(.system(size: 13))
                            .frame(height: 60)
                    }
                }

                // Connection Test Section
                if testingConnection {
                    Section {
                        HStack {
                            Spacer()
                            ProgressView("Testing connection...")
                            Spacer()
                        }
                    }
                } else if let result = testResult {
                    Section {
                        HStack {
                            Spacer()

                            switch result {
                            case .success:
                                HStack(spacing: 6) {
                                    Image(systemName: "checkmark.circle.fill")
                                        .foregroundStyle(.green)
                                    Text("Connection successful!")
                                        .foregroundStyle(.green)
                                }
                            case .failure(let message):
                                HStack(spacing: 6) {
                                    Image(systemName: "exclamationmark.triangle.fill")
                                        .foregroundStyle(.red)
                                    Text(message)
                                        .foregroundStyle(.red)
                                        .font(.system(size: 13))
                                }
                            }

                            Spacer()
                        }
                    }
                }
            }
            .formStyle(.grouped)
            .navigationTitle("Add Server")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Cancel") { dismiss() }
                }

                ToolbarItem(placement: .principal) {
                    // Connection test button
                    Button {
                        testConnection()
                    } label: {
                        HStack(spacing: 4) {
                            Image(systemName: "bolt")
                            Text("Test")
                        }
                    }
                    .buttonStyle(.bordered)
                    .disabled(!viewModel.isValidForTest)
                }

                ToolbarItem(placement: .confirmationAction) {
                    Button("Save") {
                        saveServer()
                    }
                    .disabled(!viewModel.isValid)
                }
            }
            .fileImporter(
                isPresented: $showingKeyPicker,
                allowedContentTypes: [.data],
                allowsMultipleSelection: false
            ) { result in
                switch result {
                case .success(let urls):
                    if let url = urls.first {
                        viewModel.privateKeyPath = url.path
                    }
                case .failure:
                    break
                }
            }
        }
        .frame(width: 500, minHeight: 600, maxHeight: 800)
    }

    private func testConnection() {
        testingConnection = true
        testResult = nil

        Task {
            // Simulate connection test
            try? await Task.sleep(nanoseconds: 1_500_000_000)

            await MainActor.run {
                testingConnection = false
                // In real implementation, this would actually test the connection
                testResult = .success
            }
        }
    }

    private func saveServer() {
        let server = viewModel.toServer()
        let password = viewModel.password.isEmpty ? nil : viewModel.password
        onSave(server, password)
    }
}

// MARK: - Edit Server Container

struct EditServerContainer: View {
    @EnvironmentObject var appState: AppState
    @Environment(\.dismiss) private var dismiss
    let server: Server

    var body: some View {
        EditServerView(server: server) { updated, password in
            Task {
                try? await appState.updateServer(updated, password: password)
                dismiss()
            }
        }
    }
}

/// Edit existing server form
struct EditServerView: View {
    let server: Server
    let onSave: (Server, String?) -> Void

    @Environment(\.dismiss) private var dismiss
    @State private var viewModel: ServerFormViewModel
    @State private var showingKeyPicker = false

    init(server: Server, onSave: @escaping (Server, String?) -> Void) {
        self.server = server
        self.onSave = onSave
        _viewModel = State(initialValue: ServerFormViewModel(from: server))
    }

    var body: some View {
        NavigationStack {
            Form {
                // Same sections as AddServerView but pre-populated
                Section {
                    FormRow(icon: "tag", title: "Name") {
                        TextField("My Server", text: $viewModel.name)
                            .textFieldStyle(.roundedBorder)
                    }

                    FormRow(icon: "network", title: "Host") {
                        HStack {
                            TextField("192.168.1.1", text: $viewModel.host)
                                .textFieldStyle(.roundedBorder)
                                .disabled(true) // Host usually shouldn't change

                            Text(":")
                                .foregroundStyle(.secondary)

                            TextField("22", text: $viewModel.port)
                                .textFieldStyle(.roundedBorder)
                                .frame(width: 60)
                                .multilineTextAlignment(.center)
                        }
                    }
                } header: {
                    Text("Server Information")
                }

                Section {
                    FormRow(icon: "person", title: "Username") {
                        TextField("root", text: $viewModel.username)
                            .textFieldStyle(.roundedBorder)
                    }

                    Picker("Authentication", selection: $viewModel.authType) {
                        ForEach(AuthType.allCases) { type in
                            Label(type.displayName, systemImage: type.icon)
                                .tag(type)
                        }
                    }
                    .pickerStyle(.segmented)

                    switch viewModel.authType {
                    case .password:
                        FormRow(icon: "lock", title: "Password") {
                            SecureField("Leave empty to keep current", text: $viewModel.password)
                                .textFieldStyle(.roundedBorder)
                        }

                    case .key:
                        FormRow(icon: "key", title: "Private Key") {
                            HStack {
                                Text(viewModel.privateKeyPath.isEmpty ? "No key selected" : viewModel.privateKeyPath)
                                    .font(.system(size: 13))
                                    .lineLimit(1)
                                Spacer()
                                Button("Browse...") { showingKeyPicker = true }
                                    .buttonStyle(.borderless)
                            }
                        }

                    case .agent:
                        EmptyView()
                    }
                } header: {
                    Text("Authentication")
                }

                Section("Organization") {
                    FormRow(icon: "folder", title: "Group") {
                        Picker("", selection: $viewModel.groupId) {
                            Text("None").tag(nil as String?)
                        }
                        .pickerStyle(.menu)
                    }

                    FormRow(icon: "tag", title: "Tags") {
                        TagInputView(tags: $viewModel.tags)
                    }

                    FormRow(icon: "text.alignleft", title: "Notes") {
                        TextEditor(text: $viewModel.notes)
                            .font(.system(size: 13))
                            .frame(height: 80)
                    }
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
                        let updated = viewModel.toServer(id: server.id)
                        let password = viewModel.password.isEmpty ? nil : viewModel.password
                        onSave(updated, password)
                    }
                }
            }
        }
        .frame(width: 500, minHeight: 550, maxHeight: 700)
    }
}

// MARK: - Supporting Views

struct FormRow<Content: View>: View {
    let icon: String
    let title: String
    @ViewBuilder let content: Content

    var body: some View {
        HStack(alignment: .firstTextBaseline, spacing: 12) {
            Image(systemName: icon)
                .font(.system(size: 14))
                .foregroundStyle(.secondary)
                .frame(width: 20, alignment: .center)

            Text(title)
                .font(.system(size: 13))
                .foregroundStyle(.secondary)
                .frame(width: 100, alignment: .leading)

            content
                .frame(maxWidth: .infinity, alignment: .leading)
        }
        .padding(.vertical, 2)
    }
}

struct ValidationMessage: View {
    let message: String
    let isError: Bool

    init(_ message: String, isError: Bool = false) {
        self.message = message
        self.isError = isError
    }

    var body: some View {
        HStack(spacing: 4) {
            Image(systemName: isError ? "exclamationmark.circle" : "checkmark.circle")
                .font(.system(size: 11))
            Text(message)
                .font(.system(size: 11))
        }
        .foregroundStyle(isError ? .red : .green)
    }
}

struct TagInputView: View {
    @Binding var tags: [String]
    @State private var newTag = ""

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            FlowLayout(spacing: 6) {
                ForEach(tags, id: \.self) { tag in
                    TagChip(tag: tag) {
                        tags.removeAll { $0 == tag }
                    }
                }
            }

            HStack {
                TextField("Add tag...", text: $newTag)
                    .textFieldStyle(.roundedBorder)
                    .font(.system(size: 13))
                    .onSubmit {
                        addTag()
                    }

                Button {
                    addTag()
                } label: {
                    Image(systemName: "plus")
                }
                .buttonStyle(.borderless)
                .disabled(newTag.isEmpty)
            }
        }
    }

    private func addTag() {
        let trimmed = newTag.trimmingCharacters(in: .whitespaces)
        if !trimmed.isEmpty && !tags.contains(trimmed) {
            tags.append(trimmed)
        }
        newTag = ""
    }
}

struct TagChip: View {
    let tag: String
    let onRemove: () -> Void

    var body: some View {
        HStack(spacing: 4) {
            Text(tag)
                .font(.system(size: 12))

            Button {
                onRemove()
            } label: {
                Image(systemName: "xmark")
                    .font(.system(size: 10))
            }
            .buttonStyle(.borderless)
        }
        .padding(.horizontal, 8)
        .padding(.vertical, 3)
        .background(Color.accentColor.opacity(0.15))
        .foregroundStyle(.accentColor)
        .clipShape(Capsule())
    }
}

// MARK: - Quick Connect View

struct QuickConnectView: View {
    @EnvironmentObject var appState: AppState
    @Environment(\.dismiss) private var dismiss
    @State private var connectionString = ""
    @State private var username = ""
    @State private var parsedHost = ""

    var body: some View {
        NavigationStack {
            Form {
                Section {
                    TextField("user@hostname:port", text: $connectionString)
                        .font(.system(size: 14, design: .monospaced))
                        .textFieldStyle(.roundedBorder)
                        .onChange(of: connectionString) { parseConnectionString() }
                } header: {
                    Text("Connection String")
                } footer: {
                    Text("Format: username@hostname:port (port optional)")
                        .font(.system(size: 11))
                }

                if !parsedHost.isEmpty {
                    Section("Preview") {
                        HStack {
                            Text("Host:")
                                .foregroundStyle(.secondary)
                            Text(parsedHost)
                        }
                        HStack {
                            Text("User:")
                                .foregroundStyle(.secondary)
                            Text(username)
                        }
                    }
                }
            }
            .formStyle(.grouped)
            .navigationTitle("Quick Connect")
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Cancel") { dismiss() }
                }

                ToolbarItem(placement: .confirmationAction) {
                    Button("Connect") {
                        performQuickConnect()
                    }
                    .disabled(!isValid)
                }
            }
        }
        .frame(width: 400, height: 250)
    }

    private var isValid: Bool {
        connectionString.contains("@") && !parsedHost.isEmpty
    }

    private func parseConnectionString() {
        // Simple parsing - would be more robust in production
        let parts = connectionString.components(separatedBy: "@")
        if parts.count == 2 {
            username = parts[0]
            let hostParts = parts[1].components(separatedBy: ":")
            parsedHost = hostParts[0]
        }
    }

    private func performQuickConnect() {
        // Create temporary server and connect
        let tempServer = Server(
            id: UUID().uuidString,
            name: parsedHost,
            host: parsedHost,
            port: 22,
            username: username,
            authType: .agent,
            status: .unknown
        )

        appState.connect(to: tempServer)
        dismiss()
    }
}

struct NewGroupView: View {
    @Environment(\.dismiss) private var dismiss
    @State private var name = ""
    @State private var color = Color.blue

    var body: some View {
        NavigationStack {
            Form {
                TextField("Group Name", text: $name)
                    .textFieldStyle(.roundedBorder)

                ColorPicker("Color", selection: $color)
            }
            .formStyle(.grouped)
            .padding()
            .navigationTitle("New Group")
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Cancel") { dismiss() }
                }
                ToolbarItem(placement: .confirmationAction) {
                    Button("Create") {
                        // Create group
                        dismiss()
                    }
                    .disabled(name.isEmpty)
                }
            }
        }
        .frame(width: 300, height: 200)
    }
}
