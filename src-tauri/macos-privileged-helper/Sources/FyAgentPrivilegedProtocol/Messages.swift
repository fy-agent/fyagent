import Foundation

public enum RevisionBytes {
    public static let size = 32

    public static func parse(_ data: Data, allowZero: Bool) throws -> Data {
        guard data.count == size else {
            throw ProtocolError.invalidRevision
        }
        if !allowZero && data.allSatisfy({ $0 == 0 }) {
            throw ProtocolError.invalidRevision
        }
        return data
    }

    public static func fromTuple(_ bytes: (
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8
    )) -> Data {
        Data([
            bytes.0, bytes.1, bytes.2, bytes.3, bytes.4, bytes.5, bytes.6, bytes.7,
            bytes.8, bytes.9, bytes.10, bytes.11, bytes.12, bytes.13, bytes.14, bytes.15,
            bytes.16, bytes.17, bytes.18, bytes.19, bytes.20, bytes.21, bytes.22, bytes.23,
            bytes.24, bytes.25, bytes.26, bytes.27, bytes.28, bytes.29, bytes.30, bytes.31,
        ])
    }

    public static func fromCArray(_ bytes: (UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8)) -> Data {
        fromTuple(bytes)
    }
}

public enum OperationUUID {
    public static func fromBytes(_ bytes: (
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8
    )) throws -> UUID {
        let uuid = uuid_t(
            bytes.0, bytes.1, bytes.2, bytes.3, bytes.4, bytes.5, bytes.6, bytes.7,
            bytes.8, bytes.9, bytes.10, bytes.11, bytes.12, bytes.13, bytes.14, bytes.15
        )
        return UUID(uuid: uuid)
    }

    public static func toBytes(_ uuid: UUID) -> (
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
        UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8
    ) {
        let u = uuid.uuid
        return (
            u.0, u.1, u.2, u.3, u.4, u.5, u.6, u.7,
            u.8, u.9, u.10, u.11, u.12, u.13, u.14, u.15
        )
    }
}

public struct ClosedCommitFields: Equatable {
    public var protocolVersion: UInt32
    public var operationId: UUID
    public var action: CommitAction
    public var product: KnownProduct
    public var targetSlot: UInt32
    public var expectedTargetRevision: Data
    public var expectedSourceRevision: Data
    public var reserved: UInt32

    public init(
        protocolVersion: UInt32,
        operationId: UUID,
        action: CommitAction,
        product: KnownProduct,
        targetSlot: UInt32,
        expectedTargetRevision: Data,
        expectedSourceRevision: Data,
        reserved: UInt32
    ) throws {
        self.protocolVersion = protocolVersion
        self.operationId = operationId
        self.action = action
        self.product = product
        self.targetSlot = targetSlot
        self.expectedTargetRevision = expectedTargetRevision
        self.expectedSourceRevision = expectedSourceRevision
        self.reserved = reserved
        try validate()
    }

    public func validate() throws {
        if protocolVersion != PrivilegedIdentifiers.protocolVersion {
            throw ProtocolError.protocolIncompatible
        }
        if reserved != 0 {
            throw ProtocolError.reservedNonzero
        }
        if action == .none {
            throw ProtocolError.unknownAction
        }
        _ = try KnownApplicationPolicyTable.resolve(
            product: product,
            slot: targetSlot,
            action: action
        )
        _ = try RevisionBytes.parse(expectedSourceRevision, allowZero: false)
        _ = try RevisionBytes.parse(expectedTargetRevision, allowZero: action == .freshInstall)
    }
}

extension ClosedCommitFields: Codable {
    public enum CodingKeys: String, CodingKey, CaseIterable {
        case protocolVersion
        case operationId
        case action
        case product
        case targetSlot
        case expectedTargetRevision
        case expectedSourceRevision
        case reserved
    }

    public init(from decoder: Decoder) throws {
        try ForbiddenWireKeys.assertAllowedCodingKeys(CodingKeys.allCases.map(\.rawValue))
        try StrictDecoder.rejectUnknownKeys(decoder, allowed: CodingKeys.self)
        let container = try decoder.container(keyedBy: CodingKeys.self)
        protocolVersion = try container.decode(UInt32.self, forKey: .protocolVersion)
        operationId = try container.decode(UUID.self, forKey: .operationId)
        let actionRaw = try container.decode(UInt32.self, forKey: .action)
        action = try CommitAction(validating: actionRaw)
        let productRaw = try container.decode(UInt32.self, forKey: .product)
        product = try KnownProduct(validating: productRaw)
        targetSlot = try container.decode(UInt32.self, forKey: .targetSlot)
        expectedTargetRevision = try container.decode(Data.self, forKey: .expectedTargetRevision)
        expectedSourceRevision = try container.decode(Data.self, forKey: .expectedSourceRevision)
        reserved = try container.decode(UInt32.self, forKey: .reserved)
        try validate()
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        try container.encode(protocolVersion, forKey: .protocolVersion)
        try container.encode(operationId, forKey: .operationId)
        try container.encode(action.rawValue, forKey: .action)
        try container.encode(product.rawValue, forKey: .product)
        try container.encode(targetSlot, forKey: .targetSlot)
        try container.encode(expectedTargetRevision, forKey: .expectedTargetRevision)
        try container.encode(expectedSourceRevision, forKey: .expectedSourceRevision)
        try container.encode(reserved, forKey: .reserved)
    }
}

public struct HelperStatusRequest: Codable, Equatable {
    public var protocolVersion: UInt32
    public var reserved: UInt32

    public enum CodingKeys: String, CodingKey, CaseIterable {
        case protocolVersion
        case reserved
    }

    public init(protocolVersion: UInt32 = PrivilegedIdentifiers.protocolVersion, reserved: UInt32 = 0) throws {
        self.protocolVersion = protocolVersion
        self.reserved = reserved
        try validate()
    }

    public init(from decoder: Decoder) throws {
        try ForbiddenWireKeys.assertAllowedCodingKeys(CodingKeys.allCases.map(\.rawValue))
        try StrictDecoder.rejectUnknownKeys(decoder, allowed: CodingKeys.self)
        let container = try decoder.container(keyedBy: CodingKeys.self)
        protocolVersion = try container.decode(UInt32.self, forKey: .protocolVersion)
        reserved = try container.decode(UInt32.self, forKey: .reserved)
        try validate()
    }

    public func validate() throws {
        if protocolVersion != PrivilegedIdentifiers.protocolVersion {
            throw ProtocolError.protocolIncompatible
        }
        if reserved != 0 {
            throw ProtocolError.reservedNonzero
        }
    }
}

public struct HelperStatusReply: Codable, Equatable {
    public var protocolVersion: UInt32
    public var helperVersion: String
    public var minimumClientVersion: String
    public var state: HelperState
    public var reason: HelperReason
    public var activeRecovery: Bool

    public enum CodingKeys: String, CodingKey, CaseIterable {
        case protocolVersion
        case helperVersion
        case minimumClientVersion
        case state
        case reason
        case activeRecovery
    }

    public init(
        protocolVersion: UInt32 = PrivilegedIdentifiers.protocolVersion,
        helperVersion: String = PrivilegedIdentifiers.helperBundleVersion,
        minimumClientVersion: String = PrivilegedIdentifiers.minimumClientVersion,
        state: HelperState,
        reason: HelperReason = .none,
        activeRecovery: Bool = false
    ) {
        self.protocolVersion = protocolVersion
        self.helperVersion = helperVersion
        self.minimumClientVersion = minimumClientVersion
        self.state = state
        self.reason = reason
        self.activeRecovery = activeRecovery
    }

    public init(from decoder: Decoder) throws {
        try ForbiddenWireKeys.assertAllowedCodingKeys(CodingKeys.allCases.map(\.rawValue))
        try StrictDecoder.rejectUnknownKeys(decoder, allowed: CodingKeys.self)
        let container = try decoder.container(keyedBy: CodingKeys.self)
        protocolVersion = try container.decode(UInt32.self, forKey: .protocolVersion)
        helperVersion = try container.decode(String.self, forKey: .helperVersion)
        minimumClientVersion = try container.decode(String.self, forKey: .minimumClientVersion)
        state = try container.decode(HelperState.self, forKey: .state)
        reason = try container.decode(HelperReason.self, forKey: .reason)
        activeRecovery = try container.decode(Bool.self, forKey: .activeRecovery)
    }
}

public struct KnownApplicationCommitResult: Codable, Equatable {
    public var protocolVersion: UInt32
    public var operationId: UUID
    public var outcome: CommitOutcome
    public var reason: HelperReason
    public var reserved: UInt32

    public enum CodingKeys: String, CodingKey, CaseIterable {
        case protocolVersion
        case operationId
        case outcome
        case reason
        case reserved
    }

    public init(
        protocolVersion: UInt32 = PrivilegedIdentifiers.protocolVersion,
        operationId: UUID,
        outcome: CommitOutcome,
        reason: HelperReason,
        reserved: UInt32 = 0
    ) {
        self.protocolVersion = protocolVersion
        self.operationId = operationId
        self.outcome = outcome
        self.reason = reason
        self.reserved = reserved
    }

    public init(from decoder: Decoder) throws {
        try ForbiddenWireKeys.assertAllowedCodingKeys(CodingKeys.allCases.map(\.rawValue))
        try StrictDecoder.rejectUnknownKeys(decoder, allowed: CodingKeys.self)
        let container = try decoder.container(keyedBy: CodingKeys.self)
        protocolVersion = try container.decode(UInt32.self, forKey: .protocolVersion)
        operationId = try container.decode(UUID.self, forKey: .operationId)
        outcome = try container.decode(CommitOutcome.self, forKey: .outcome)
        reason = try container.decode(HelperReason.self, forKey: .reason)
        reserved = try container.decode(UInt32.self, forKey: .reserved)
        if reserved != 0 {
            throw ProtocolError.reservedNonzero
        }
    }

    public func validate(expectedOperationId: UUID) throws {
        if protocolVersion != PrivilegedIdentifiers.protocolVersion {
            throw ProtocolError.protocolIncompatible
        }
        if operationId != expectedOperationId {
            throw ProtocolError.invalidOperationId
        }
        if reserved != 0 {
            throw ProtocolError.reservedNonzero
        }
        switch (outcome, reason) {
        case (.committed, .none),
             (.rollbackRestored, .rollbackRestored),
             (.recoveryRequired, .recoveryRequired):
            return
        case (.failed, let failureReason) where Self.allowedFailureReasons.contains(failureReason):
            return
        case (.ready, _),
             (.committed, _),
             (.rollbackRestored, _),
             (.recoveryRequired, _),
             (.failed, _):
            throw ProtocolError.protocolIncompatible
        }
    }

    private static let allowedFailureReasons: Set<HelperReason> = [
        .helperProtocolIncompatible,
        .operationAuthorizationCancelled,
        .operationAuthorizationInvalid,
        .sourceCapabilityInvalid,
        .sourceChanged,
        .targetSlotInvalid,
        .targetChanged,
        .applicationRunning,
        .permissionDenied,
        .commitFailed,
        .unexpectedField,
        .reservedNonzero,
    ]
}

public struct RemoveHelperRequest: Codable, Equatable {
    public var protocolVersion: UInt32
    public var operationId: UUID
    public var reserved: UInt32

    public enum CodingKeys: String, CodingKey, CaseIterable {
        case protocolVersion
        case operationId
        case reserved
    }

    public init(
        protocolVersion: UInt32 = PrivilegedIdentifiers.protocolVersion,
        operationId: UUID,
        reserved: UInt32 = 0
    ) throws {
        self.protocolVersion = protocolVersion
        self.operationId = operationId
        self.reserved = reserved
        try validate()
    }

    public init(from decoder: Decoder) throws {
        try ForbiddenWireKeys.assertAllowedCodingKeys(CodingKeys.allCases.map(\.rawValue))
        try StrictDecoder.rejectUnknownKeys(decoder, allowed: CodingKeys.self)
        let container = try decoder.container(keyedBy: CodingKeys.self)
        protocolVersion = try container.decode(UInt32.self, forKey: .protocolVersion)
        operationId = try container.decode(UUID.self, forKey: .operationId)
        reserved = try container.decode(UInt32.self, forKey: .reserved)
        try validate()
    }

    public func validate() throws {
        if protocolVersion != PrivilegedIdentifiers.protocolVersion {
            throw ProtocolError.protocolIncompatible
        }
        if reserved != 0 {
            throw ProtocolError.reservedNonzero
        }
    }
}

public struct RemoveHelperResult: Codable, Equatable {
    public var protocolVersion: UInt32
    public var operationId: UUID
    public var outcome: CommitOutcome
    public var reason: HelperReason
    public var reserved: UInt32

    public enum CodingKeys: String, CodingKey, CaseIterable {
        case protocolVersion
        case operationId
        case outcome
        case reason
        case reserved
    }

    public init(
        protocolVersion: UInt32 = PrivilegedIdentifiers.protocolVersion,
        operationId: UUID,
        outcome: CommitOutcome,
        reason: HelperReason,
        reserved: UInt32 = 0
    ) {
        self.protocolVersion = protocolVersion
        self.operationId = operationId
        self.outcome = outcome
        self.reason = reason
        self.reserved = reserved
    }

    public init(from decoder: Decoder) throws {
        try ForbiddenWireKeys.assertAllowedCodingKeys(CodingKeys.allCases.map(\.rawValue))
        try StrictDecoder.rejectUnknownKeys(decoder, allowed: CodingKeys.self)
        let container = try decoder.container(keyedBy: CodingKeys.self)
        protocolVersion = try container.decode(UInt32.self, forKey: .protocolVersion)
        operationId = try container.decode(UUID.self, forKey: .operationId)
        outcome = try container.decode(CommitOutcome.self, forKey: .outcome)
        reason = try container.decode(HelperReason.self, forKey: .reason)
        reserved = try container.decode(UInt32.self, forKey: .reserved)
        if reserved != 0 {
            throw ProtocolError.reservedNonzero
        }
    }

    public func validate(expectedOperationId: UUID) throws {
        if protocolVersion != PrivilegedIdentifiers.protocolVersion {
            throw ProtocolError.protocolIncompatible
        }
        if operationId != expectedOperationId {
            throw ProtocolError.invalidOperationId
        }
        if reserved != 0 {
            throw ProtocolError.reservedNonzero
        }
        switch (outcome, reason) {
        case (.ready, .none),
             (.recoveryRequired, .recoveryRequired),
             (.failed, .helperRemovalFailed),
             (.failed, .operationAuthorizationCancelled),
             (.failed, .operationAuthorizationInvalid):
            return
        case (.committed, _),
             (.rollbackRestored, _),
             (.recoveryRequired, _),
             (.ready, _),
             (.failed, _):
            throw ProtocolError.protocolIncompatible
        }
    }
}
