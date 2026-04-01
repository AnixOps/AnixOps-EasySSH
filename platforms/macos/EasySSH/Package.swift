// swift-tools-version:5.9
import PackageDescription

let package = Package(
    name: "EasySSH",
    platforms: [
        .macOS(.v13)
    ],
    products: [
        .executable(
            name: "EasySSH",
            targets: ["EasySSH"]
        ),
        .library(
            name: "EasySSHCore",
            targets: ["EasySSHCore"]
        )
    ],
    dependencies: [
        .package(url: "https://github.com/apple/swift-nio.git", from: "2.0.0"),
    ],
    targets: [
        .executableTarget(
            name: "EasySSH",
            dependencies: [
                "EasySSHCore"
            ]
        ),
        .target(
            name: "EasySSHCore",
            dependencies: [],
            swiftSettings: [
                .unsafeFlags(["-I", "../../core/target/include"]),
                .unsafeFlags(["-L", "../../core/target/release"]),
                .unsafeFlags(["-leasyssh_core"])
            ]
        ),
        .testTarget(
            name: "EasySSHTests",
            dependencies: ["EasySSHCore"],
            path: "Tests/EasySSHTests",
            exclude: ["EasySSHCoreBridgeTests.m"],
            swiftSettings: [
                .unsafeFlags(["-I", "../../core/target/include"]),
                .unsafeFlags(["-L", "../../core/target/release"]),
                .unsafeFlags(["-leasyssh_core"])
            ]
        )
    ]
)
