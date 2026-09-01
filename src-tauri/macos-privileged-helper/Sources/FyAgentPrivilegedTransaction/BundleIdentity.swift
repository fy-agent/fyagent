import CommonCrypto
import Foundation
import FyAgentPrivilegedProtocol

public struct BundleIdentity: Equatable {
    public var bundleIdentifier: String
    public var version: String
    public var executable: String

    public var revision: Data {
        let canonical = "bundleId=\(bundleIdentifier)\nversion=\(version)\nexecutable=\(executable)\n"
        return Self.sha256(canonical)
    }

    public static func sha256(_ string: String) -> Data {
        let bytes = Array(string.utf8)
        var digest = [UInt8](repeating: 0, count: Int(CC_SHA256_DIGEST_LENGTH))
        bytes.withUnsafeBytes { raw in
            _ = CC_SHA256(raw.baseAddress, CC_LONG(bytes.count), &digest)
        }
        return Data(digest)
    }
}

public enum BundleIdentityReader {
    static let maxPlistBytes = 256 * 1024

    public static func read(fromBundleFD bundleFD: Int32, policy: KnownApplicationPolicy) throws -> BundleIdentity {
        try DirectoryFD.requireDirectory(bundleFD)
        let contentsFD = try DirectoryFD.openAtDirectory(bundleFD, "Contents")
        defer { DirectoryFD.close(contentsFD) }

        let plistData = try DirectoryFD.readFileAt(contentsFD, "Info.plist", limit: maxPlistBytes)
        let plist = try PropertyListSerialization.propertyList(from: plistData, options: [], format: nil)
        guard let info = plist as? [String: Any] else {
            throw TransactionError.sourceCapabilityInvalid
        }
        guard let bundleId = info["CFBundleIdentifier"] as? String,
              bundleId == policy.bundleIdentifier else {
            throw TransactionError.sourceCapabilityInvalid
        }
        guard let executable = info["CFBundleExecutable"] as? String, !executable.isEmpty else {
            throw TransactionError.sourceCapabilityInvalid
        }

        let macosFD = try DirectoryFD.openAtDirectory(contentsFD, "MacOS")
        defer { DirectoryFD.close(macosFD) }
        let executableFD = try DirectoryFD.openAtFile(macosFD, executable)
        DirectoryFD.close(executableFD)

        let version: String
        switch policy.versionSource {
        case .infoPlist:
            if let short = info["CFBundleShortVersionString"] as? String, !short.isEmpty {
                version = short
            } else if let build = info["CFBundleVersion"] as? String, !build.isEmpty {
                version = build
            } else {
                throw TransactionError.sourceCapabilityInvalid
            }
        case .traeProductJSON:
            version = try readTraeVersion(fromContents: contentsFD)
        }

        return BundleIdentity(bundleIdentifier: bundleId, version: version, executable: executable)
    }

    private static func readTraeVersion(fromContents contentsFD: Int32) throws -> String {
        let resourcesFD = try DirectoryFD.openAtDirectory(contentsFD, "Resources")
        defer { DirectoryFD.close(resourcesFD) }
        let appFD = try DirectoryFD.openAtDirectory(resourcesFD, "app")
        defer { DirectoryFD.close(appFD) }
        let json = try DirectoryFD.readFileAt(appFD, "product.json", limit: maxPlistBytes)
        let object = try JSONSerialization.jsonObject(with: json)
        guard let dict = object as? [String: Any],
              let version = dict["tronBuildVersion"] as? String,
              !version.isEmpty else {
            throw TransactionError.sourceCapabilityInvalid
        }
        return version
    }
}
