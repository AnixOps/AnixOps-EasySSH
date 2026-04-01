import SwiftUI

/// Quick Connect dialog for immediate SSH connections
/// Allows connecting to a server without saving it to the database
struct QuickConnectView: View {
    @EnvironmentObject var appState: AppState
    @Environment(\.dismiss) private var dismiss
    @State private var host = ""
    @State private var username = ""
    @State private var port = "22"
    @State private var authType: AuthType = .agent
    @State private var password = ""
    @State private var isConnecting = false

    var body: some View {
        VStack(spacing: 24) {
            // Header
            VStack(spacing: 8) {
                Image(systemName: "bolt.fill")
                    .font(.system(size: 32))
                    .foregroundStyle(.accent)

                Text("Quick Connect")
                    .font(.title2)
                    .fontWeight(.bold)

                Text("Connect to a server without saving it")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }

            // Form
            VStack(spacing: 16) {
                // Host
                VStack(alignment: .leading, spacing: 4) {
                    Text("Host")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                    TextField("example.com", text: $host)
                        .textFieldStyle(.roundedBorder)
                }

                // Username and Port
                HStack(spacing: 12) {
                    VStack(alignment: .leading, spacing: 4) {
                        Text("Username")
                            .font(.caption)
                            .foregroundStyle(.secondary)
                        TextField("root", text: $username)
                            .textFieldStyle(.roundedBorder)
                    }

                    VStack(alignment: .leading, spacing: 4) {
                        Text("Port")
                            .font(.caption)
                            .foregroundStyle(.secondary)
                        TextField("22", text: $port)
                            .textFieldStyle(.roundedBorder)
                            .frame(width: 70)
                    }
                }

                // Auth Type
                VStack(alignment: .leading, spacing: 4) {
                    Text("Authentication")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                    Picker("Auth Type", selection: $authType) {
                        ForEach(AuthType.allCases) { type in
                            Label(type.displayName, systemImage: type.icon)
                                .tag(type)
                        }
                    }
                    .pickerStyle(.segmented)
                    .labelsHidden()
                }

                // Password field if needed
                if authType == .password {
                    VStack(alignment: .leading, spacing: 4) {
                        Text("Password")
                            .font(.caption)
                            .foregroundStyle(.secondary)
                        SecureField("Enter password", text: $password)
                            .textFieldStyle(.roundedBorder)
                    }
                }
            }
            .frame(width: 280)

            Spacer()

            // Actions
            HStack {
                Button("Cancel") {
                    dismiss()
                }
                .keyboardShortcut(.escape, modifiers: [])

                Spacer()

                Button {
                    performQuickConnect()
                } label: {
                    if isConnecting {
                        HStack(spacing: 4) {
                            ProgressView()
                                .scaleEffect(0.6)
                                .frame(width: 16, height: 16)
                            Text("Connecting...")
                        }
                    } else {
                        Text("Connect")
                    }
                }
                .buttonStyle(.borderedProminent)
                .disabled(!canConnect || isConnecting)
                .keyboardShortcut(.return, modifiers: [])
            }
        }
        .padding()
        .frame(width: 340, height: 380)
    }

    private var canConnect: Bool {
        !host.isEmpty && !username.isEmpty
    }

    private func performQuickConnect() {
        isConnecting = true

        let server = Server(
            id: UUID().uuidString,
            name: "\(username)@\(host)",
            host: host,
            port: Int(port) ?? 22,
            username: username,
            authType: authType
        )

        // Temporarily add to servers list so connection works
        appState.servers.append(server)

        // Connect
        appState.connect(to: server)

        dismiss()
    }
}

// MARK: - Preview

#Preview {
    QuickConnectView()
        .environmentObject(AppState())
}
