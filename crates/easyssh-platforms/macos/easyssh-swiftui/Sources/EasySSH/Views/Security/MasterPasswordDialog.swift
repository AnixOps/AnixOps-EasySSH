import SwiftUI
import Security
import LocalAuthentication

/// Master password dialog modes
enum MasterPasswordMode {
    case setup      // First time setup
    case verify     // Unlock on startup
    case change     // Change password
    case reset      // Reset warning
}

/// Password strength levels
enum PasswordStrength: String {
    case veryWeak = "Very Weak"
    case weak = "Weak"
    case fair = "Fair"
    case good = "Good"
    case strong = "Strong"

    var color: Color {
        switch self {
        case .veryWeak: return .red
        case .weak: return .orange
        case .fair: return .yellow
        case .good: return .green.opacity(0.7)
        case .strong: return .green
        }
    }

    var score: Double {
        switch self {
        case .veryWeak: return 0.2
        case .weak: return 0.4
        case .fair: return 0.6
        case .good: return 0.8
        case .strong: return 1.0
        }
    }
}

/// Result of master password dialog
enum MasterPasswordResult {
    case cancelled
    case setPassword(String)
    case verify(String)
    case changePassword(old: String, new: String)
    case resetConfirmed
    case forgotPassword
    case biometricAuthenticated
}

/// Master Password Dialog View for macOS
struct MasterPasswordDialog: View {
    @Binding var isPresented: Bool
    let mode: MasterPasswordMode
    var failedAttempts: Int = 0
    let onComplete: (MasterPasswordResult) -> Void

    @State private var password = ""
    @State private var confirmPassword = ""
    @State private var oldPassword = ""
    @State private var showPassword = false
    @State private var showConfirmPassword = false
    @State private var showOldPassword = false
    @State private var passwordStrength: PasswordStrength = .veryWeak
    @State private var errorMessage: String?
    @State private var resetConfirmation = ""
    @State private var useBiometric = false
    @State private var isBiometricAvailable = false

    // Argon2id parameters (64MB memory, 3 iterations, 2 parallelism)
    private let argon2MemoryKB = 65536
    private let argon2Iterations = 3
    private let argon2Parallelism = 2

    var body: some View {
        VStack(spacing: 0) {
            // Header
            headerView
                .padding(.horizontal, 24)
                .padding(.top, 24)
                .padding(.bottom, 16)

            ScrollView {
                VStack(spacing: 20) {
                    switch mode {
                    case .setup:
                        setupContent
                    case .verify:
                        verifyContent
                    case .change:
                        changeContent
                    case .reset:
                        resetContent
                    }
                }
                .padding(.horizontal, 24)
                .padding(.bottom, 24)
            }

            // Footer with buttons
            footerView
                .padding(.horizontal, 24)
                .padding(.vertical, 16)
                .background(.ultraThinMaterial)
        }
        .frame(width: 450, height: heightForMode)
        .onAppear {
            checkBiometricAvailability()
        }
    }

    // MARK: - Header

    private var headerView: some View {
        HStack(spacing: 12) {
            Image(systemName: iconForMode)
                .font(.system(size: 32))
                .foregroundStyle(colorForMode)

            VStack(alignment: .leading, spacing: 4) {
                Text(titleForMode)
                    .font(.headline)

                Text(subtitleForMode)
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .lineLimit(2)
            }

            Spacer()
        }
    }

    // MARK: - Setup Content

    private var setupContent: some View {
        VStack(spacing: 16) {
            // Welcome message
            Text("Welcome to EasySSH Lite!")
                .font(.title2)
                .fontWeight(.semibold)

            Text("Create a master password to secure your SSH configurations. This password will encrypt all your sensitive data using AES-256-GCM with Argon2id key derivation.")
                .font(.callout)
                .foregroundStyle(.secondary)
                .multilineTextAlignment(.center)

            // Password field
            VStack(alignment: .leading, spacing: 8) {
                Label("Password", systemImage: "lock.fill")
                    .font(.caption)
                    .foregroundStyle(.secondary)

                HStack {
                    if showPassword {
                        TextField("Enter password", text: $password)
                            .textFieldStyle(.roundedBorder)
                            .onChange(of: password) { updateStrength() }
                    } else {
                        SecureField("Enter password", text: $password)
                            .textFieldStyle(.roundedBorder)
                            .onChange(of: password) { updateStrength() }
                    }

                    Button {
                        showPassword.toggle()
                    } label: {
                        Image(systemName: showPassword ? "eye.slash" : "eye")
                    }
                    .buttonStyle(.plain)
                }

                // Strength indicator
                HStack {
                    Text("Strength: \(passwordStrength.rawValue)")
                        .font(.caption)
                        .foregroundStyle(passwordStrength.color)

                    Spacer()

                    ProgressView(value: passwordStrength.score)
                        .tint(passwordStrength.color)
                        .frame(width: 100)
                }
            }

            // Confirm password field
            VStack(alignment: .leading, spacing: 8) {
                Label("Confirm Password", systemImage: "lock.fill")
                    .font(.caption)
                    .foregroundStyle(.secondary)

                HStack {
                    if showConfirmPassword {
                        TextField("Confirm password", text: $confirmPassword)
                            .textFieldStyle(.roundedBorder)
                    } else {
                        SecureField("Confirm password", text: $confirmPassword)
                            .textFieldStyle(.roundedBorder)
                    }

                    Button {
                        showConfirmPassword.toggle()
                    } label: {
                        Image(systemName: showConfirmPassword ? "eye.slash" : "eye")
                    }
                    .buttonStyle(.plain)
                }
            }

            // Biometric option
            if isBiometricAvailable {
                Toggle("Enable Touch ID for quick unlock", isOn: $useBiometric)
                    .font(.callout)
            }

            // Requirements
            VStack(alignment: .leading, spacing: 6) {
                Text("Password Requirements:")
                    .font(.caption)
                    .fontWeight(.medium)

                RequirementRow(
                    isMet: password.count >= 8,
                    text: "At least 8 characters"
                )
                RequirementRow(
                    isMet: password.rangeOfCharacter(from: .uppercaseLetters) != nil,
                    text: "Uppercase letters (A-Z)"
                )
                RequirementRow(
                    isMet: password.rangeOfCharacter(from: .lowercaseLetters) != nil,
                    text: "Lowercase letters (a-z)"
                )
                RequirementRow(
                    isMet: password.rangeOfCharacter(from: .decimalDigits) != nil,
                    text: "Numbers (0-9)"
                )
                RequirementRow(
                    isMet: password.rangeOfCharacter(from: CharacterSet.alphanumerics.inverted) != nil,
                    text: "Special characters (!@#$...)"
                )
            }
            .font(.caption)

            // Error message
            if let error = errorMessage {
                Text(error)
                    .font(.caption)
                    .foregroundStyle(.red)
                    .multilineTextAlignment(.center)
            }
        }
    }

    // MARK: - Verify Content

    private var verifyContent: some View {
        VStack(spacing: 16) {
            Image(systemName: "lock.shield.fill")
                .font(.system(size: 48))
                .foregroundStyle(.accent)

            Text("Unlock EasySSH")
                .font(.title2)
                .fontWeight(.semibold)

            Text("Enter your master password to access your encrypted server configurations.")
                .font(.callout)
                .foregroundStyle(.secondary)
                .multilineTextAlignment(.center)

            // Failed attempts warning
            if failedAttempts > 0 {
                let remaining = max(0, 5 - failedAttempts)
                Label(
                    "Warning: \(failedAttempts) failed attempts. \(remaining) attempts remaining.",
                    systemImage: "exclamationmark.triangle.fill"
                )
                .font(.caption)
                .foregroundStyle(.orange)
                .padding(.horizontal, 12)
                .padding(.vertical, 8)
                .background(Color.orange.opacity(0.1))
                .clipShape(RoundedRectangle(cornerRadius: 8))
            }

            // Biometric button
            if isBiometricAvailable {
                Button {
                    authenticateWithBiometric()
                } label: {
                    Label("Unlock with Touch ID", systemImage: "touchid")
                        .font(.callout)
                }
                .buttonStyle(.bordered)
            }

            // Password field
            VStack(alignment: .leading, spacing: 8) {
                Label("Master Password", systemImage: "lock.fill")
                    .font(.caption)
                    .foregroundStyle(.secondary)

                HStack {
                    if showPassword {
                        TextField("Enter password", text: $password)
                            .textFieldStyle(.roundedBorder)
                    } else {
                        SecureField("Enter password", text: $password)
                            .textFieldStyle(.roundedBorder)
                    }

                    Button {
                        showPassword.toggle()
                    } label: {
                        Image(systemName: showPassword ? "eye.slash" : "eye")
                    }
                    .buttonStyle(.plain)
                }
            }

            // Forgot password link
            Button("Forgot password?") {
                onComplete(.forgotPassword)
                isPresented = false
            }
            .buttonStyle(.plain)
            .font(.callout)
            .foregroundStyle(.accent)

            // Error message
            if let error = errorMessage {
                Text(error)
                    .font(.caption)
                    .foregroundStyle(.red)
            }
        }
    }

    // MARK: - Change Content

    private var changeContent: some View {
        VStack(spacing: 16) {
            Text("Change Master Password")
                .font(.title2)
                .fontWeight(.semibold)

            Text("Change your master password. All existing encrypted data will be re-encrypted with the new password.")
                .font(.callout)
                .foregroundStyle(.secondary)
                .multilineTextAlignment(.center)

            // Current password
            VStack(alignment: .leading, spacing: 8) {
                Label("Current Password", systemImage: "lock.fill")
                    .font(.caption)
                    .foregroundStyle(.secondary)

                HStack {
                    if showOldPassword {
                        TextField("Enter current password", text: $oldPassword)
                            .textFieldStyle(.roundedBorder)
                    } else {
                        SecureField("Enter current password", text: $oldPassword)
                            .textFieldStyle(.roundedBorder)
                    }

                    Button {
                        showOldPassword.toggle()
                    } label: {
                        Image(systemName: showOldPassword ? "eye.slash" : "eye")
                    }
                    .buttonStyle(.plain)
                }
            }

            // New password
            VStack(alignment: .leading, spacing: 8) {
                Label("New Password", systemImage: "lock.fill")
                    .font(.caption)
                    .foregroundStyle(.secondary)

                HStack {
                    if showPassword {
                        TextField("Enter new password", text: $password)
                            .textFieldStyle(.roundedBorder)
                            .onChange(of: password) { updateStrength() }
                    } else {
                        SecureField("Enter new password", text: $password)
                            .textFieldStyle(.roundedBorder)
                            .onChange(of: password) { updateStrength() }
                    }

                    Button {
                        showPassword.toggle()
                    } label: {
                        Image(systemName: showPassword ? "eye.slash" : "eye")
                    }
                    .buttonStyle(.plain)
                }

                // Strength indicator
                HStack {
                    Text("Strength: \(passwordStrength.rawValue)")
                        .font(.caption)
                        .foregroundStyle(passwordStrength.color)

                    Spacer()

                    ProgressView(value: passwordStrength.score)
                        .tint(passwordStrength.color)
                        .frame(width: 100)
                }
            }

            // Confirm new password
            VStack(alignment: .leading, spacing: 8) {
                Label("Confirm New Password", systemImage: "lock.fill")
                    .font(.caption)
                    .foregroundStyle(.secondary)

                HStack {
                    if showConfirmPassword {
                        TextField("Confirm new password", text: $confirmPassword)
                            .textFieldStyle(.roundedBorder)
                    } else {
                        SecureField("Confirm new password", text: $confirmPassword)
                            .textFieldStyle(.roundedBorder)
                    }

                    Button {
                        showConfirmPassword.toggle()
                    } label: {
                        Image(systemName: showConfirmPassword ? "eye.slash" : "eye")
                    }
                    .buttonStyle(.plain)
                }
            }

            // Error message
            if let error = errorMessage {
                Text(error)
                    .font(.caption)
                    .foregroundStyle(.red)
            }
        }
    }

    // MARK: - Reset Content

    private var resetContent: some View {
        VStack(spacing: 16) {
            Image(systemName: "exclamationmark.triangle.fill")
                .font(.system(size: 48))
                .foregroundStyle(.red)

            Text("Reset Master Password")
                .font(.title2)
                .fontWeight(.semibold)

            Text("WARNING: This action cannot be undone!")
                .font(.callout)
                .fontWeight(.semibold)
                .foregroundStyle(.red)

            VStack(alignment: .leading, spacing: 8) {
                Text("Resetting your master password will permanently delete:")
                    .font(.callout)

                VStack(alignment: .leading, spacing: 4) {
                    Label("All stored SSH passwords", systemImage: "xmark.circle.fill")
                    Label("All encrypted server configurations", systemImage: "xmark.circle.fill")
                    Label("All secure vault items", systemImage: "xmark.circle.fill")
                    Label("Your encrypted keychain data", systemImage: "xmark.circle.fill")
                }
                .font(.caption)
                .foregroundStyle(.secondary)
            }

            Text("You will need to re-add all your servers manually.")
                .font(.callout)
                .foregroundStyle(.orange)

            // Confirmation
            VStack(alignment: .leading, spacing: 8) {
                Text("Type \"DELETE\" to confirm:")
                    .font(.caption)
                    .fontWeight(.medium)

                TextField("DELETE", text: $resetConfirmation)
                    .textFieldStyle(.roundedBorder)
            }

            // Error message
            if let error = errorMessage {
                Text(error)
                    .font(.caption)
                    .foregroundStyle(.red)
            }
        }
    }

    // MARK: - Footer

    private var footerView: some View {
        HStack {
            Button("Cancel") {
                onComplete(.cancelled)
                isPresented = false
            }
            .keyboardShortcut(.cancelAction)

            Spacer()

            Button {
                handleAction()
            } label: {
                Text(actionButtonTitle)
                    .fontWeight(.semibold)
            }
            .keyboardShortcut(.defaultAction)
            .buttonStyle(.borderedProminent)
            .tint(mode == .reset ? .red : .accentColor)
        }
    }

    // MARK: - Helpers

    private var titleForMode: String {
        switch mode {
        case .setup: return "Set Master Password"
        case .verify: return "Unlock EasySSH"
        case .change: return "Change Master Password"
        case .reset: return "Reset Master Password"
        }
    }

    private var subtitleForMode: String {
        switch mode {
        case .setup: return "Create a secure password to encrypt your data"
        case .verify: return "Enter your master password to continue"
        case .change: return "Update your master password"
        case .reset: return "Warning: All encrypted data will be lost"
        }
    }

    private var iconForMode: String {
        switch mode {
        case .setup: return "lock.shield.fill"
        case .verify: return "lock.open.fill"
        case .change: return "arrow.clockwise.lock.fill"
        case .reset: return "exclamationmark.triangle.fill"
        }
    }

    private var colorForMode: Color {
        switch mode {
        case .setup: return .accentColor
        case .verify: return .accentColor
        case .change: return .accentColor
        case .reset: return .red
        }
    }

    private var heightForMode: CGFloat {
        switch mode {
        case .setup: return 580
        case .verify: return 380
        case .change: return 600
        case .reset: return 480
        }
    }

    private var actionButtonTitle: String {
        switch mode {
        case .setup: return "Set Password"
        case .verify: return "Unlock"
        case .change: return "Change Password"
        case .reset: return "Reset"
        }
    }

    private func updateStrength() {
        let (score, _) = calculatePasswordStrength(password)
        passwordStrength = score
    }

    private func calculatePasswordStrength(_ password: String) -> (PasswordStrength, [String]) {
        var score = 0
        var feedback: [String] = []

        // Length
        if password.count >= 8 {
            score += 20
        } else {
            feedback.append("Password must be at least 8 characters")
        }
        if password.count >= 12 { score += 10 }
        if password.count >= 16 { score += 10 }

        // Character variety
        if password.rangeOfCharacter(from: .lowercaseLetters) != nil {
            score += 15
        } else {
            feedback.append("Add lowercase letters")
        }

        if password.rangeOfCharacter(from: .uppercaseLetters) != nil {
            score += 15
        } else {
            feedback.append("Add uppercase letters")
        }

        if password.rangeOfCharacter(from: .decimalDigits) != nil {
            score += 15
        } else {
            feedback.append("Add numbers")
        }

        if password.rangeOfCharacter(from: CharacterSet.alphanumerics.inverted) != nil {
            score += 15
        } else {
            feedback.append("Add special characters")
        }

        let strength: PasswordStrength
        switch score {
        case 0...20: strength = .veryWeak
        case 21...40: strength = .weak
        case 41...60: strength = .fair
        case 61...80: strength = .good
        default: strength = .strong
        }

        return (strength, feedback)
    }

    private func validatePassword() -> Bool {
        // Minimum length
        if password.count < 8 {
            errorMessage = "Password must be at least 8 characters long"
            return false
        }

        // Character requirements
        let hasLower = password.rangeOfCharacter(from: .lowercaseLetters) != nil
        let hasUpper = password.rangeOfCharacter(from: .uppercaseLetters) != nil
        let hasDigit = password.rangeOfCharacter(from: .decimalDigits) != nil
        let hasSpecial = password.rangeOfCharacter(from: CharacterSet.alphanumerics.inverted) != nil

        if !hasLower || !hasUpper || !hasDigit || !hasSpecial {
            errorMessage = "Password must contain uppercase, lowercase, numbers, and special characters"
            return false
        }

        // Strength check
        let (strength, _) = calculatePasswordStrength(password)
        if strength == .veryWeak || strength == .weak {
            errorMessage = "Password is too weak. Please use a stronger password."
            return false
        }

        return true
    }

    private func handleAction() {
        errorMessage = nil

        switch mode {
        case .setup:
            if !validatePassword() { return }
            if password != confirmPassword {
                errorMessage = "Passwords do not match"
                return
            }
            onComplete(.setPassword(password))

        case .verify:
            if password.isEmpty {
                errorMessage = "Please enter your master password"
                return
            }
            onComplete(.verify(password))

        case .change:
            if oldPassword.isEmpty {
                errorMessage = "Please enter your current password"
                return
            }
            if !validatePassword() { return }
            if password != confirmPassword {
                errorMessage = "New passwords do not match"
                return
            }
            if oldPassword == password {
                errorMessage = "New password must be different from current password"
                return
            }
            onComplete(.changePassword(old: oldPassword, new: password))

        case .reset:
            if resetConfirmation != "DELETE" {
                errorMessage = "Please type DELETE to confirm"
                return
            }
            onComplete(.resetConfirmed)
        }

        isPresented = false
    }

    private func checkBiometricAvailability() {
        let context = LAContext()
        var error: NSError?
        isBiometricAvailable = context.canEvaluatePolicy(
            .deviceOwnerAuthenticationWithBiometrics,
            error: &error
        )
    }

    private func authenticateWithBiometric() {
        let context = LAContext()
        context.localizedReason = "Unlock EasySSH"

        context.evaluatePolicy(
            .deviceOwnerAuthenticationWithBiometrics,
            localizedReason: "Unlock EasySSH"
        ) { success, error in
            DispatchQueue.main.async {
                if success {
                    onComplete(.biometricAuthenticated)
                    isPresented = false
                } else {
                    errorMessage = "Biometric authentication failed"
                }
            }
        }
    }
}

// MARK: - Requirement Row

struct RequirementRow: View {
    let isMet: Bool
    let text: String

    var body: some View {
        HStack(spacing: 6) {
            Image(systemName: isMet ? "checkmark.circle.fill" : "circle")
                .font(.caption)
                .foregroundStyle(isMet ? .green : .secondary)

            Text(text)
                .foregroundStyle(isMet ? .primary : .secondary)

            Spacer()
        }
    }
}

// MARK: - View Extensions

extension View {
    func masterPasswordDialog(
        isPresented: Binding<Bool>,
        mode: MasterPasswordMode,
        failedAttempts: Int = 0,
        onComplete: @escaping (MasterPasswordResult) -> Void
    ) -> some View {
        self.sheet(isPresented: isPresented) {
            MasterPasswordDialog(
                isPresented: isPresented,
                mode: mode,
                failedAttempts: failedAttempts,
                onComplete: onComplete
            )
        }
    }
}

// MARK: - Preview

#Preview("Setup") {
    MasterPasswordDialog(
        isPresented: .constant(true),
        mode: .setup,
        onComplete: { _ in }
    )
}

#Preview("Verify") {
    MasterPasswordDialog(
        isPresented: .constant(true),
        mode: .verify,
        failedAttempts: 2,
        onComplete: { _ in }
    )
}

#Preview("Change") {
    MasterPasswordDialog(
        isPresented: .constant(true),
        mode: .change,
        onComplete: { _ in }
    )
}

#Preview("Reset") {
    MasterPasswordDialog(
        isPresented: .constant(true),
        mode: .reset,
        onComplete: { _ in }
    )
}
