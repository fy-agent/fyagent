import Foundation

public enum KnownProduct: UInt32, Codable, CaseIterable, Equatable {
    case codexDesktop = 1
    case openCodeDesktop = 2
    case qoderWork = 3
    case traeWork = 4
    case workBuddy = 5

    public init(validating rawValue: UInt32) throws {
        guard let value = KnownProduct(rawValue: rawValue) else {
            throw ProtocolError.unknownProduct
        }
        self = value
    }
}

public enum CommitAction: UInt32, Codable, CaseIterable, Equatable {
    case none = 0
    case freshInstall = 1
    case updateExisting = 2

    public init(validating rawValue: UInt32) throws {
        guard let value = CommitAction(rawValue: rawValue) else {
            throw ProtocolError.unknownAction
        }
        self = value
    }
}

public enum PrivilegedOperation: UInt32, Codable, CaseIterable, Equatable {
    case status = 1
    case ensureHelper = 2
    case commit = 3
    case removeHelper = 4

    public init(validating rawValue: UInt32) throws {
        guard let value = PrivilegedOperation(rawValue: rawValue) else {
            throw ProtocolError.unknownOperation
        }
        self = value
    }
}

public enum CommitOutcome: UInt32, Codable, CaseIterable, Equatable {
    case committed = 1
    case rollbackRestored = 2
    case recoveryRequired = 3
    case ready = 4
    case failed = 5
}

public enum HelperState: UInt32, Codable, CaseIterable, Equatable {
    case ready = 1
    case updateRequired = 2
    case incompatible = 3
    case recoveryRequired = 4
    case missing = 5
}

public enum HelperReason: UInt32, Codable, CaseIterable, Equatable {
    case none = 0
    case helperNotPackaged = 1
    case helperSignatureInvalid = 2
    case helperInstallAuthorizationCancelled = 3
    case helperInstallFailed = 4
    case helperUpdateRequired = 5
    case helperDowngradeRejected = 6
    case helperProtocolIncompatible = 7
    case helperPeerRejected = 8
    case operationAuthorizationCancelled = 9
    case operationAuthorizationInvalid = 10
    case sourceCapabilityInvalid = 11
    case sourceChanged = 12
    case targetSlotInvalid = 13
    case helperRemovalFailed = 14
    case targetChanged = 15
    case applicationRunning = 16
    case permissionDenied = 17
    case commitFailed = 18
    case rollbackRestored = 19
    case recoveryRequired = 20
    case unexpectedField = 21
    case reservedNonzero = 22
}

public enum VersionSource: Equatable {
    case infoPlist
    case traeProductJSON
}
