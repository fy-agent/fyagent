// swift-tools-version: 5.9

import Foundation
import PackageDescription

let packageDirectory = Context.packageDirectory
let environment = ProcessInfo.processInfo.environment
let privilegedVariant = environment["FYAGENT_PRIVILEGED_VARIANT"] ?? "production"
let isDevelopmentVariant = privilegedVariant == "development"
let helperInfoPlist = environment["FYAGENT_PRIVILEGED_HELPER_INFO_PLIST"]
    ?? "\(packageDirectory)/Resources/helper-info.plist"
let helperLaunchdPlist = environment["FYAGENT_PRIVILEGED_HELPER_LAUNCHD_PLIST"]
    ?? "\(packageDirectory)/Resources/helper-launchd.plist"
let privilegedVariantSettings: [SwiftSetting] = isDevelopmentVariant
    ? [.define("FYAGENT_PRIVILEGED_DEVELOPMENT")]
    : []

let package = Package(
    name: "FyAgentPrivilegedHelper",
    platforms: [.macOS(.v12)],
    products: [
        .library(name: "FyAgentPrivilegedProtocol", targets: ["FyAgentPrivilegedProtocol"]),
        .library(name: "FyAgentPrivilegedTransaction", targets: ["FyAgentPrivilegedTransaction"]),
        .library(
            name: "FyAgentPrivilegedClient",
            type: .dynamic,
            targets: ["FyAgentPrivilegedClient"]
        ),
        .executable(name: "com.fyagent.desktop.system-commit-helper", targets: ["FyAgentPrivilegedHelper"]),
        .executable(name: "PrivilegedHelperTests", targets: ["PrivilegedHelperTests"]),
    ],
    dependencies: [
        .package(url: "https://github.com/trilemma-dev/Blessed.git", exact: "0.6.0"),
        .package(url: "https://github.com/trilemma-dev/Authorized.git", exact: "1.0.0"),
        .package(
            url: "https://github.com/trilemma-dev/SecureXPC.git",
            revision: "1cece54562c7626d042f007d2f38cfe325565850"
        ),
        .package(url: "https://github.com/trilemma-dev/EmbeddedPropertyList.git", exact: "2.0.2"),
        .package(url: "https://github.com/trilemma-dev/Required.git", exact: "0.1.1"),
    ],
    targets: [
        .target(
            name: "CFyAgentPrivilegedBridge",
            path: "include",
            publicHeadersPath: "."
        ),
        .target(
            name: "FyAgentPrivilegedProtocol",
            dependencies: [
                .product(name: "Authorized", package: "Authorized"),
                .product(name: "SecureXPC", package: "SecureXPC"),
            ],
            path: "Sources/FyAgentPrivilegedProtocol",
            swiftSettings: privilegedVariantSettings
        ),
        .target(
            name: "FyAgentPrivilegedTransaction",
            dependencies: ["FyAgentPrivilegedProtocol"],
            path: "Sources/FyAgentPrivilegedTransaction",
            swiftSettings: privilegedVariantSettings
        ),
        .target(
            name: "FyAgentPrivilegedClient",
            dependencies: [
                "CFyAgentPrivilegedBridge",
                "FyAgentPrivilegedProtocol",
                "FyAgentPrivilegedTransaction",
                .product(name: "Authorized", package: "Authorized"),
                .product(name: "Blessed", package: "Blessed"),
                .product(name: "EmbeddedPropertyList", package: "EmbeddedPropertyList"),
                .product(name: "Required", package: "Required"),
                .product(name: "SecureXPC", package: "SecureXPC"),
            ],
            path: "Sources/FyAgentPrivilegedClient",
            swiftSettings: privilegedVariantSettings
        ),
        .executableTarget(
            name: "FyAgentPrivilegedHelper",
            dependencies: [
                "FyAgentPrivilegedProtocol",
                "FyAgentPrivilegedTransaction",
                .product(name: "Authorized", package: "Authorized"),
                .product(name: "SecureXPC", package: "SecureXPC"),
            ],
            path: "Sources/FyAgentPrivilegedHelper",
            swiftSettings: privilegedVariantSettings,
            linkerSettings: [
                .unsafeFlags([
                    "-Xlinker", "-sectcreate",
                    "-Xlinker", "__TEXT",
                    "-Xlinker", "__info_plist",
                    "-Xlinker", helperInfoPlist,
                    "-Xlinker", "-sectcreate",
                    "-Xlinker", "__TEXT",
                    "-Xlinker", "__launchd_plist",
                    "-Xlinker", helperLaunchdPlist,
                ])
            ]
        ),
        .executableTarget(
            name: "PrivilegedHelperTests",
            dependencies: [
                "CFyAgentPrivilegedBridge",
                "FyAgentPrivilegedClient",
                "FyAgentPrivilegedProtocol",
                "FyAgentPrivilegedTransaction",
            ],
            path: "Tests"
        ),
    ],
    swiftLanguageVersions: [.v5]
)
