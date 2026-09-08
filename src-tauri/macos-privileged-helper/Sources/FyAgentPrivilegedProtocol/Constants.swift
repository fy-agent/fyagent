import Foundation

public enum PrivilegedIdentifiers {
    #if FYAGENT_PRIVILEGED_DEVELOPMENT
    public static let appIdentifier = "com.fyagent.desktop.dev"
    public static let helperIdentifier = "com.fyagent.desktop.dev.system-commit-helper"
    public static let machService = "com.fyagent.desktop.dev.system-commit-helper"
    public static let commitRightName = "com.fyagent.desktop.dev.system-application.commit"
    public static let removeRightName = "com.fyagent.desktop.dev.privileged-helper.remove"
    public static let productionReceiptDirectory = "/Library/Application Support/FyAgent/DevelopmentSystemCommit/v1"
    #else
    public static let appIdentifier = "com.fyagent.desktop"
    public static let helperIdentifier = "com.fyagent.desktop.system-commit-helper"
    public static let machService = "com.fyagent.desktop.system-commit-helper"
    public static let commitRightName = "com.fyagent.desktop.system-application.commit"
    public static let removeRightName = "com.fyagent.desktop.privileged-helper.remove"
    public static let productionReceiptDirectory = "/Library/Application Support/FyAgent/SystemCommit/v1"
    #endif
    public static let teamIdentifier = "HY446996QX"
    public static let productionApplicationsParent = "/Applications"
    public static let protocolVersion: UInt32 = 1
    public static var helperBundleVersion: String {
        Bundle.main.infoDictionary?["CFBundleVersion"] as? String ?? minimumClientVersion
    }
    public static let minimumClientVersion = "0.4.2"
}

public enum AnyCodingKey: CodingKey, Hashable {
    case name(String)

    public var stringValue: String {
        switch self {
        case .name(let value):
            return value
        }
    }

    public var intValue: Int? { nil }

    public init?(stringValue: String) {
        self = .name(stringValue)
    }

    public init?(intValue: Int) {
        self = .name(String(intValue))
    }
}

public enum ProtocolError: Error, Equatable {
    case unexpectedField(String)
    case reservedNonzero
    case protocolIncompatible
    case unknownProduct
    case targetSlotInvalid
    case unknownOperation
    case unknownAction
    case invalidRevision
    case invalidOperationId
}

public enum StrictDecoder {
    public static func rejectUnknownKeys<Key: CodingKey & CaseIterable>(
        _ decoder: Decoder,
        allowed: Key.Type
    ) throws {
        let container = try decoder.container(keyedBy: AnyCodingKey.self)
        let allowedNames = Set(allowed.allCases.map(\.stringValue))
        for key in container.allKeys where !allowedNames.contains(key.stringValue) {
            throw ProtocolError.unexpectedField(key.stringValue)
        }
    }
}

public enum ForbiddenWireKeys {
    public static let names: Set<String> = [
        "path", "url", "command", "argv", "destination", "hash", "token",
        "teamId", "team_id", "bypass", "packageFormat", "sourcePath", "targetPath",
    ]

    public static func assertAllowedCodingKeys(_ names: [String]) throws {
        for name in names {
            if namesLooksForbidden(name) {
                throw ProtocolError.unexpectedField(name)
            }
        }
    }

    public static func namesLooksForbidden(_ name: String) -> Bool {
        let lowered = name.lowercased()
        return names.contains(name)
            || lowered.contains("path")
            || lowered.contains("url")
            || lowered.contains("command")
            || lowered.contains("argv")
            || lowered == "destination"
    }
}
