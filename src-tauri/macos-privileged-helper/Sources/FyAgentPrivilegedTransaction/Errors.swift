import Foundation
import FyAgentPrivilegedProtocol

public enum TransactionError: Error, Equatable {
    case protocolIncompatible
    case unknownProduct
    case targetSlotInvalid
    case sourceCapabilityInvalid
    case sourceChanged
    case targetChanged
    case applicationRunning
    case permissionDenied
    case commitFailed(String)
    case recoveryRequired
    case reservedNonzero
    case unexpectedField
    case invalidRevision

    public var reason: HelperReason {
        switch self {
        case .protocolIncompatible:
            return .helperProtocolIncompatible
        case .unknownProduct, .targetSlotInvalid:
            return .targetSlotInvalid
        case .sourceCapabilityInvalid:
            return .sourceCapabilityInvalid
        case .sourceChanged:
            return .sourceChanged
        case .targetChanged:
            return .targetChanged
        case .applicationRunning:
            return .applicationRunning
        case .permissionDenied:
            return .permissionDenied
        case .commitFailed:
            return .commitFailed
        case .recoveryRequired:
            return .recoveryRequired
        case .reservedNonzero:
            return .reservedNonzero
        case .unexpectedField:
            return .unexpectedField
        case .invalidRevision:
            return .sourceCapabilityInvalid
        }
    }

    public static func from(_ error: ProtocolError) -> TransactionError {
        switch error {
        case .unexpectedField:
            return .unexpectedField
        case .reservedNonzero:
            return .reservedNonzero
        case .protocolIncompatible:
            return .protocolIncompatible
        case .unknownProduct:
            return .unknownProduct
        case .targetSlotInvalid:
            return .targetSlotInvalid
        case .unknownOperation, .unknownAction:
            return .targetSlotInvalid
        case .invalidRevision, .invalidOperationId:
            return .invalidRevision
        }
    }
}

public struct CommitResult: Equatable {
    public var outcome: CommitOutcome
    public var reason: HelperReason
    public var installedRevision: Data?

    public init(outcome: CommitOutcome, reason: HelperReason, installedRevision: Data? = nil) {
        self.outcome = outcome
        self.reason = reason
        self.installedRevision = installedRevision
    }
}

public struct TransactionHooks {
    public var afterReplacementCommitted: (() throws -> Void)?
    public var isApplicationRunning: ((String) -> Bool)?
    public var forceVerificationFailure: Bool

    public init(
        afterReplacementCommitted: (() throws -> Void)? = nil,
        isApplicationRunning: ((String) -> Bool)? = nil,
        forceVerificationFailure: Bool = false
    ) {
        self.afterReplacementCommitted = afterReplacementCommitted
        self.isApplicationRunning = isApplicationRunning
        self.forceVerificationFailure = forceVerificationFailure
    }
}
