import SwiftUI
import AppKit

/// AppDelegate for handling application-level events
/// Manages dock visibility, menu bar integration, and app lifecycle
class EasySSHAppDelegate: NSObject, NSApplicationDelegate, ObservableObject {
    @AppStorage("showInDock") private var showInDock = true
    @AppStorage("showInMenuBar") private var showInMenuBar = true

    func applicationDidFinishLaunching(_ notification: Notification) {
        // Set activation policy based on settings
        updateActivationPolicy()

        // Setup notification observers for settings changes
        NotificationCenter.default.addObserver(
            self,
            selector: #selector(handleSettingsChanged),
            name: UserDefaults.didChangeNotification,
            object: nil
        )
    }

    func applicationShouldHandleReopen(_ sender: NSApplication, hasVisibleWindows flag: Bool) -> Bool {
        // When user clicks the dock icon, show main window
        if !flag {
            showMainWindow()
        }
        return true
    }

    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool {
        // Keep app running in menu bar if enabled
        return !showInMenuBar
    }

    func application(_ application: NSApplication, open urls: [URL]) {
        // Handle URL scheme (easyssh://host)
        for url in urls {
            if url.scheme == "easyssh", let host = url.host {
                handleDeepLink(host: host, path: url.path)
            }
        }
    }

    // MARK: - Private Methods

    private func updateActivationPolicy() {
        if showInMenuBar && !showInDock {
            // Menu bar only mode
            NSApp.setActivationPolicy(.accessory)
        } else {
            // Regular app mode (dock + menu bar or dock only)
            NSApp.setActivationPolicy(.regular)
        }
    }

    @objc private func handleSettingsChanged() {
        // Update activation policy when settings change
        DispatchQueue.main.async { [weak self] in
            self?.updateActivationPolicy()
        }
    }

    private func showMainWindow() {
        NSApp.unhide(nil)
        NSApp.activate(ignoringOtherApps: true)

        // Create and show main window if it doesn't exist
        if NSApp.mainWindow == nil {
            // The WindowGroup should handle this automatically
            // Just ensure app is visible
        }
    }

    private func handleDeepLink(host: String, path: String) {
        // Handle easyssh://host connection requests
        // This could be triggered from browser or other apps
        NotificationCenter.default.post(
            name: .init("EasySSHDeepLink"),
            object: nil,
            userInfo: ["host": host, "path": path]
        )
    }
}

// MARK: - Menu Bar Icon Customization

/// Custom menu bar icon view with connection status indicator
struct MenuBarIconView: View {
    let isConnected: Bool
    let connectionCount: Int

    var body: some View {
        ZStack {
            Image(systemName: "terminal.fill")
                .font(.system(size: 16))

            // Connection indicator dot
            if isConnected {
                Circle()
                    .fill(Color.green)
                    .frame(width: 8, height: 8)
                    .offset(x: 6, y: -6)
                    .overlay(
                        Circle()
                            .stroke(Color.white, lineWidth: 1)
                            .frame(width: 8, height: 8)
                            .offset(x: 6, y: -6)
                    )
            }

            // Connection count badge
            if connectionCount > 1 {
                Text("\(min(connectionCount, 9))")
                    .font(.system(size: 9, weight: .bold))
                    .foregroundColor(.white)
                    .frame(width: 12, height: 12)
                    .background(Color.red)
                    .clipShape(Circle())
                    .offset(x: 8, y: 8)
            }
        }
    }
}

// MARK: - SwiftUI Environment Extensions

extension EnvironmentValues {
    var appDelegate: EasySSHAppDelegate? {
        get { self[AppDelegateKey.self] }
        set { self[AppDelegateKey.self] = newValue }
    }
}

private struct AppDelegateKey: EnvironmentKey {
    static let defaultValue: EasySSHAppDelegate? = nil
}
