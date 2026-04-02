// swift-tools-version:5.9
import PackageDescription
import Foundation

// MARK: - Build Mode Detection

/// Detect if we're building for App Store distribution
/// Set APP_STORE_BUILD=1 for App Store submission (uses XCFramework)
let isAppStoreBuild = ProcessInfo.processInfo.environment["APP_STORE_BUILD"] == "1"

/// Path to Rust core for local development
let rustCorePath = ProcessInfo.processInfo.environment["EASYSSH_CORE_PATH"] ?? "../../../core"

// MARK: - Target Definitions

/// System library target for local development
let ceasysshSystemTarget: Target = .systemLibrary(
    name: "CEasySSHCore",
    path: "Sources/CEasySSHCore",
    pkgConfig: nil,
    providers: nil
)

/// Binary target for App Store distribution
let ceasysshBinaryTarget: Target = .binaryTarget(
    name: "CEasySSHCore",
    path: "Frameworks/CEasySSHCore.xcframework"
)

/// EasySSH executable with appropriate linker settings
func makeEasySSHExecutable() -> Target {
    if isAppStoreBuild {
        // App Store: No custom linker settings, everything from XCFramework
        return .executableTarget(
            name: "EasySSH",
            dependencies: [
                "EasySSHBridge",
                .product(name: "NIO", package: "swift-nio"),
            ],
            swiftSettings: [
                .enableExperimentalFeature("StrictConcurrency")
            ]
        )
    } else {
        // Development: Linker paths to local Rust build
        return .executableTarget(
            name: "EasySSH",
            dependencies: [
                "EasySSHBridge",
                .product(name: "NIO", package: "swift-nio"),
            ],
            swiftSettings: [
                .enableExperimentalFeature("StrictConcurrency")
            ],
            linkerSettings: [
                // Library search paths - linker-only, not compiler flags
                .unsafeFlags(
                    ["-L", "\(rustCorePath)/target/release"],
                    .when(platforms: [.macOS], configuration: .release)
                ),
                .unsafeFlags(
                    ["-L", "\(rustCorePath)/target/debug"],
                    .when(platforms: [.macOS], configuration: .debug)
                ),
            ]
        )
    }
}

/// CEasySSHCore target selector
let ceasysshTarget: Target = isAppStoreBuild ? ceasysshBinaryTarget : ceasysshSystemTarget

// MARK: - Package

let package = Package(
    name: "EasySSH",
    platforms: [
        .macOS(.v14)
    ],
    products: [
        .executable(
            name: "EasySSH",
            targets: ["EasySSH"]
        ),
        .library(
            name: "EasySSHBridge",
            targets: ["EasySSHBridge"]
        )
    ],
    dependencies: [
        .package(url: "https://github.com/apple/swift-nio.git", from: "2.0.0"),
        .package(url: "https://github.com/sushichop/swift-crypto.git", from: "0.1.0"),
    ],
    targets: [
        // C/Rust Core - systemLibrary or binaryTarget based on build mode
        ceasysshTarget,

        // Main executable
        makeEasySSHExecutable(),

        // Bridge library
        .target(
            name: "EasySSHBridge",
            dependencies: ["CEasySSHCore"],
            swiftSettings: [
                .enableExperimentalFeature("StrictConcurrency")
            ]
        ),

        // Tests
        .testTarget(
            name: "EasySSHTests",
            dependencies: ["EasySSH", "EasySSHBridge"]
        )
    ]
)

// MARK: - Build Instructions

/*
 ============================================================================
 BUILD MODES
 ============================================================================

 1. LOCAL DEVELOPMENT (Default)
    ---------------------------
    Environment variables:
      - EASYSSH_CORE_PATH (optional): Absolute path to core directory

    Steps:
      cd core && cargo build --release
      cd platforms/macos/easyssh-swiftui
      swift build

    Or use the helper script:
      ./scripts/build-rust-core.sh
      swift build

 2. CI/CD BUILD
    ------------
    Environment variables:
      - EASYSSH_CORE_PATH: Must be set to absolute path in CI workspace
      - CI: Set to "true" (optional, for logging)

    Steps:
      export EASYSSH_CORE_PATH=$GITHUB_WORKSPACE/core
      cd platforms/macos/easyssh-swiftui
      swift build

 3. APP STORE SUBMISSION (XCFramework)
    ------------------------------------
    Environment variables:
      - APP_STORE_BUILD: Set to "1"

    Prerequisites:
      1. Build XCFramework:
         ./scripts/build-xcframework.sh

      2. Place XCFramework:
         Frameworks/CEasySSHCore.xcframework

      3. Sign XCFramework:
         codesign --sign "Developer ID" Frameworks/CEasySSHCore.xcframework

    Steps:
      export APP_STORE_BUILD=1
      swift build

    Note: In this mode, the Package.swift uses NO unsafe flags.
    The library is completely self-contained in the XCFramework.

 ============================================================================
 WHY SYSTEMLIBRARY + MODULEMAP?
 ============================================================================

 The systemLibrary target with modulemap is App Store compliant because:

 1. No compiler unsafeFlags (like -I, -L, -l in the target that uses them)
 2. Standard Swift Package Manager pattern for C library integration
 3. Clear separation between Swift code and C FFI
 4. The modulemap provides the interface definition

 The only unsafeFlags used are in linkerSettings for local development,
 and these are NOT included in the App Store build path.

 ============================================================================
 TROUBLESHOOTING
 ============================================================================

 Q: "No such module: CEasySSHCore"
 A: The modulemap isn't being found. Check:
    - Sources/CEasySSHCore/include/module.modulemap exists
    - Headers are in Sources/CEasySSHCore/include/

 Q: "Library not found for -leasyssh_core"
 A: Rust library not built or path incorrect:
    - cd core && cargo build --release
    - export EASYSSH_CORE_PATH=/absolute/path/to/core

 Q: "Undefined symbols for architecture"
 A: Architecture mismatch. Build universal binary:
    - ./scripts/build-rust-core.sh

 Q: App Store rejection for unsafeFlags
 A: Make sure APP_STORE_BUILD=1 and XCFramework exists:
    - ./scripts/build-xcframework.sh
    - export APP_STORE_BUILD=1
    - swift build
*/
