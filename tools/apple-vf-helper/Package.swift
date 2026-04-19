// swift-tools-version: 5.9
// World Compute — Apple Virtualization.framework helper binary
// Requires macOS 13+ and Xcode with Virtualization framework.

import PackageDescription

let package = Package(
    name: "wc-apple-vf-helper",
    platforms: [
        .macOS(.v13)
    ],
    targets: [
        .executableTarget(
            name: "wc-apple-vf-helper",
            path: "Sources",
            linkerSettings: [
                .linkedFramework("Virtualization")
            ]
        )
    ]
)
