import Darwin
import EmbeddedPropertyList
import Foundation
import FyAgentPrivilegedProtocol

/// Strict numeric CFBundleVersion representation used only for the bundled
/// privileged helper lifecycle. Keeping comparison here avoids attempting an
/// equal-version SMJobBless and makes downgrade handling explicit.
struct HelperBundleVersion: Comparable, Equatable {
    let rawValue: String
    private let major: UInt32
    private let minor: UInt32
    private let patch: UInt32

    init?(_ rawValue: String) {
        let fields = rawValue.split(separator: ".", omittingEmptySubsequences: false)
        guard (1 ... 3).contains(fields.count) else {
            return nil
        }
        var values = [UInt32](repeating: 0, count: 3)
        for (index, field) in fields.enumerated() {
            let maximumDigits = index == 0 ? 4 : 2
            guard !field.isEmpty,
                  field.count <= maximumDigits,
                  field.allSatisfy({ $0.isASCII && $0.isNumber }),
                  let value = UInt32(field) else {
                return nil
            }
            values[index] = value
        }
        self.rawValue = rawValue
        major = values[0]
        minor = values[1]
        patch = values[2]
    }

    static func < (lhs: HelperBundleVersion, rhs: HelperBundleVersion) -> Bool {
        if lhs.major != rhs.major {
            return lhs.major < rhs.major
        }
        if lhs.minor != rhs.minor {
            return lhs.minor < rhs.minor
        }
        return lhs.patch < rhs.patch
    }

    static func == (lhs: HelperBundleVersion, rhs: HelperBundleVersion) -> Bool {
        lhs.major == rhs.major && lhs.minor == rhs.minor && lhs.patch == rhs.patch
    }
}

enum HelperLifecycleDecision: Equatable {
    case ready
    case installOrUpdate
    case failed(state: HelperState, reason: HelperReason)
}

enum HelperLifecycleError: Error, Equatable {
    case appIdentityInvalid
    case helperMissing
    case helperNotRegular
    case helperMetadataInvalid
}

struct BundledHelperDescriptor: Equatable {
    let version: HelperBundleVersion
}

enum HelperLifecycle {
    static func clientVersion(in bundle: Bundle = .main) throws -> HelperBundleVersion {
        guard bundle.bundleIdentifier == PrivilegedIdentifiers.appIdentifier,
              let rawVersion = bundle.infoDictionary?["CFBundleVersion"] as? String,
              let version = HelperBundleVersion(rawVersion) else {
            throw HelperLifecycleError.appIdentityInvalid
        }
        return version
    }

    static func bundledHelper(in bundle: Bundle = .main) throws -> BundledHelperDescriptor {
        let helper = bundle.bundleURL
            .appendingPathComponent("Contents", isDirectory: true)
            .appendingPathComponent("Library", isDirectory: true)
            .appendingPathComponent("LaunchServices", isDirectory: true)
            .appendingPathComponent(PrivilegedIdentifiers.helperIdentifier, isDirectory: false)
        var metadata = stat()
        guard lstat(helper.path, &metadata) == 0 else {
            throw HelperLifecycleError.helperMissing
        }
        guard metadata.st_mode & S_IFMT == S_IFREG else {
            throw HelperLifecycleError.helperNotRegular
        }
        let data: Data
        do {
            data = try EmbeddedPropertyListReader.info.readExternal(from: helper)
        } catch {
            throw HelperLifecycleError.helperMetadataInvalid
        }
        let propertyList: Any
        do {
            propertyList = try PropertyListSerialization.propertyList(
                from: data,
                options: [],
                format: nil
            )
        } catch {
            throw HelperLifecycleError.helperMetadataInvalid
        }
        guard let info = propertyList as? [String: Any],
              info["CFBundleIdentifier"] as? String == PrivilegedIdentifiers.helperIdentifier,
              let rawVersion = info["CFBundleVersion"] as? String,
              let version = HelperBundleVersion(rawVersion) else {
            throw HelperLifecycleError.helperMetadataInvalid
        }
        return BundledHelperDescriptor(version: version)
    }

    static func decide(
        status: HelperStatusReply,
        bundledVersion: HelperBundleVersion,
        clientVersion: HelperBundleVersion
    ) -> HelperLifecycleDecision {
        guard let installedVersion = HelperBundleVersion(status.helperVersion),
              let minimumClientVersion = HelperBundleVersion(status.minimumClientVersion) else {
            return .failed(state: .incompatible, reason: .helperProtocolIncompatible)
        }

        if status.activeRecovery
            || status.state == .recoveryRequired
            || status.reason == .recoveryRequired {
            guard status.activeRecovery,
                  status.state == .recoveryRequired,
                  status.reason == .recoveryRequired else {
                return .failed(state: .incompatible, reason: .helperProtocolIncompatible)
            }
            return .failed(state: .recoveryRequired, reason: .recoveryRequired)
        }

        if minimumClientVersion > clientVersion {
            return .failed(state: .incompatible, reason: .helperProtocolIncompatible)
        }

        if status.protocolVersion != PrivilegedIdentifiers.protocolVersion {
            if installedVersion < bundledVersion {
                return .installOrUpdate
            }
            let reason: HelperReason = installedVersion > bundledVersion
                ? .helperDowngradeRejected
                : .helperProtocolIncompatible
            return .failed(state: .incompatible, reason: reason)
        }

        if installedVersion < bundledVersion {
            return .installOrUpdate
        }

        if status.state == .ready, status.reason == .none, !status.activeRecovery {
            return .ready
        }

        if installedVersion > bundledVersion,
           status.state == .updateRequired || status.state == .incompatible {
            return .failed(state: .incompatible, reason: .helperDowngradeRejected)
        }

        if status.state == .updateRequired || status.reason == .helperUpdateRequired {
            return .failed(state: .updateRequired, reason: .helperUpdateRequired)
        }

        if status.state == .incompatible {
            return .failed(state: .incompatible, reason: .helperProtocolIncompatible)
        }

        return .failed(state: .incompatible, reason: .helperProtocolIncompatible)
    }
}
