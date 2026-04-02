import SwiftUI
import SwiftData

/// View shown during data migration from UserDefaults to SwiftData
struct MigrationView: View {
    @StateObject private var migrationService = DataMigrationService.shared
    @State private var progress: MigrationProgress?
    @State private var result: MigrationResult?
    @State private var isMigrating = false

    var onComplete: () -> Void

    var body: some View {
        VStack(spacing: 20) {
            // Header
            VStack(spacing: 8) {
                Image(systemName: "arrow.down.doc.fill")
                    .font(.system(size: 48))
                    .foregroundColor(.accentColor)

                Text("Data Migration")
                    .font(.title)
                    .fontWeight(.semibold)

                Text("Upgrading to SwiftData with iCloud sync")
                    .font(.subheadline)
                    .foregroundColor(.secondary)
            }
            .padding(.top, 20)

            // Progress or Result
            if let result = result {
                migrationResultView(result)
            } else if isMigrating, let progress = progress {
                migrationProgressView(progress)
            } else {
                preMigrationView
            }

            Spacer()
        }
        .frame(width: 400, height: 300)
        .padding()
    }

    // MARK: - Subviews

    private var preMigrationView: some View {
        VStack(spacing: 16) {
            VStack(alignment: .leading, spacing: 12) {
                FeatureRow(
                    icon: "cloud",
                    title: "iCloud Sync",
                    description: "Your servers will sync across all your devices"
                )

                FeatureRow(
                    icon: "lock.shield",
                    title: "Enhanced Security",
                    description: "Sensitive data is encrypted with AES-256"
                )

                FeatureRow(
                    icon: "bolt",
                    title: "Better Performance",
                    description: "Faster queries and improved reliability"
                )
            }
            .padding()
            .background(Color.secondary.opacity(0.1))
            .cornerRadius(8)

            Button("Start Migration") {
                startMigration()
            }
            .buttonStyle(.borderedProminent)
            .controlSize(.large)
        }
    }

    private func migrationProgressView(_ progress: MigrationProgress) -> some View {
        VStack(spacing: 16) {
            ProgressView(value: progress.percentComplete, total: 100)
                .progressViewStyle(.linear)
                .frame(width: 300)

            Text(progress.stage.rawValue)
                .font(.subheadline)
                .foregroundColor(.secondary)

            Text("\(Int(progress.percentComplete))%")
                .font(.caption)
                .foregroundColor(.secondary)
        }
        .padding()
    }

    private func migrationResultView(_ result: MigrationResult) -> some View {
        VStack(spacing: 16) {
            Image(systemName: result.success ? "checkmark.circle.fill" : "exclamationmark.triangle.fill")
                .font(.system(size: 48))
                .foregroundColor(result.success ? .green : .orange)

            if result.success {
                VStack(spacing: 8) {
                    Text("Migration Complete!")
                        .font(.headline)

                    Text("Migrated \(result.serversMigrated) servers and \(result.groupsMigrated) groups")
                        .font(.subheadline)
                        .foregroundColor(.secondary)
                        .multilineTextAlignment(.center)
                }

                if result.hasErrors {
                    VStack(alignment: .leading, spacing: 4) {
                        Text("Warnings:")
                            .font(.caption)
                            .fontWeight(.medium)

                        ScrollView {
                            VStack(alignment: .leading, spacing: 4) {
                                ForEach(result.serverErrors + result.groupErrors, id: \.self) { error in
                                    Text("• \(error)")
                                        .font(.caption)
                                        .foregroundColor(.secondary)
                                }
                            }
                        }
                        .frame(maxHeight: 60)
                    }
                    .padding()
                    .background(Color.orange.opacity(0.1))
                    .cornerRadius(6)
                }

                Button("Continue") {
                    onComplete()
                }
                .buttonStyle(.borderedProminent)
                .controlSize(.large)
            } else {
                VStack(spacing: 8) {
                    Text("Migration Failed")
                        .font(.headline)

                    Text(result.errorMessage ?? "Unknown error")
                        .font(.subheadline)
                        .foregroundColor(.secondary)
                        .multilineTextAlignment(.center)
                }

                HStack(spacing: 12) {
                    Button("Retry") {
                        startMigration()
                    }
                    .buttonStyle(.borderedProminent)

                    Button("Skip") {
                        onComplete()
                    }
                    .buttonStyle(.bordered)
                }
            }
        }
        .padding()
    }

    // MARK: - Actions

    private func startMigration() {
        isMigrating = true
        result = nil

        Task {
            do {
                let migrationResult = try await migrationService.performMigrationWithProgress { progress in
                    Task { @MainActor in
                        self.progress = progress
                    }
                }

                await MainActor.run {
                    self.result = migrationResult
                    self.isMigrating = false
                }
            } catch {
                await MainActor.run {
                    self.result = MigrationResult(
                        success: false,
                        errorMessage: error.localizedDescription
                    )
                    self.isMigrating = false
                }
            }
        }
    }
}

// MARK: - Feature Row

struct FeatureRow: View {
    let icon: String
    let title: String
    let description: String

    var body: some View {
        HStack(spacing: 12) {
            Image(systemName: icon)
                .font(.system(size: 20))
                .foregroundColor(.accentColor)
                .frame(width: 24)

            VStack(alignment: .leading, spacing: 2) {
                Text(title)
                    .font(.subheadline)
                    .fontWeight(.medium)

                Text(description)
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
        }
    }
}

// MARK: - Migration Result Extension

extension MigrationResult {
    init(success: Bool, errorMessage: String) {
        self.success = success
        self.serversMigrated = 0
        self.groupsMigrated = 0
        self.serverErrors = []
        self.groupErrors = []
        self.duration = 0
        self.errorMessage = errorMessage
    }
}

// MARK: - Sync Status View

struct CloudSyncStatusView: View {
    let status: CloudSyncStatus

    var body: some View {
        HStack(spacing: 6) {
            syncIcon
            Text(syncText)
                .font(.caption)
        }
        .foregroundColor(syncColor)
        .padding(.horizontal, 8)
        .padding(.vertical, 4)
        .background(syncColor.opacity(0.1))
        .cornerRadius(4)
    }

    private var syncIcon: some View {
        switch status {
        case .synced:
            return Image(systemName: "checkmark.circle.fill")
        case .syncing:
            return Image(systemName: "arrow.clockwise")
        case .error:
            return Image(systemName: "exclamationmark.triangle.fill")
        case .disabled:
            return Image(systemName: "icloud.slash")
        case .unknown:
            return Image(systemName: "icloud")
        }
    }

    private var syncText: String {
        switch status {
        case .synced:
            return "Synced"
        case .syncing:
            return "Syncing..."
        case .error(let message):
            return "Sync Error: \(message)"
        case .disabled:
            return "iCloud Disabled"
        case .unknown:
            return "Checking..."
        }
    }

    private var syncColor: Color {
        switch status {
        case .synced:
            return .green
        case .syncing:
            return .blue
        case .error:
            return .orange
        case .disabled, .unknown:
            return .secondary
        }
    }
}

// MARK: - Preview

#Preview("Migration - Pre") {
    MigrationView(onComplete: {})
}

#Preview("Migration - Progress") {
    MigrationView(onComplete: {})
        .onAppear {
            // Simulate in-progress state
        }
}

#Preview("Migration - Success") {
    let result = MigrationResult(
        success: true,
        serversMigrated: 5,
        groupsMigrated: 2,
        serverErrors: [],
        groupErrors: [],
        duration: 2.5
    )

    return MigrationView(onComplete: {})
        .onAppear {
            // Show success state
        }
}
