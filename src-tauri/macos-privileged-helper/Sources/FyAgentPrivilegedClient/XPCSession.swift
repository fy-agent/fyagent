import Foundation
import FyAgentPrivilegedProtocol
import SecureXPC

/// The synchronous C ABI has to wait for SecureXPC replies, but each route has
/// a materially different execution profile. In particular, a commit timeout
/// cannot be reported as "helper unavailable": the root helper may still own
/// the transferred directory descriptor and continue the transaction.
enum HelperXPCOperation: Equatable {
    case status
    case commit
    case remove

    var timeoutMilliseconds: Int {
        switch self {
        case .status:
            // Includes launchd cold-start and the authenticated status round trip.
            return 5_000
        case .remove:
            // Removal is bounded, but it mutates launchd/helper-owned state.
            return 60_000
        case .commit:
            // Large application bundles can take minutes to copy and verify.
            return 30 * 60 * 1_000
        }
    }

    var timeoutFailure: BridgeFailure {
        switch self {
        case .status:
            return .helperUnavailable
        case .commit, .remove:
            return .operationOutcomeUnknown
        }
    }

    /// Once a mutating message has been handed to SecureXPC, every callback
    /// failure except an authenticated peer rejection is ambiguous: the helper
    /// may have committed and then lost/failed to encode the reply. Ambiguous
    /// failures must enter recovery semantics instead of being retried.
    func responseFailureHasUnknownOutcome(_ error: XPCError) -> Bool {
        guard self == .commit || self == .remove else {
            return false
        }
        if case .insecure = error {
            return false
        }
        return true
    }
}

enum HelperXPCClient {
    static func make() throws -> XPCClient {
        let identifierRequirement = try secRequirement(
            "identifier \"\(PrivilegedIdentifiers.helperIdentifier)\" and certificate leaf[subject.OU] = \"\(PrivilegedIdentifiers.teamIdentifier)\""
        )
        let requirement = try XPCClient.ServerRequirement.teamIdentifier(PrivilegedIdentifiers.teamIdentifier)
            && XPCClient.ServerRequirement.secRequirement(identifierRequirement)
        return XPCClient.forMachService(
            named: PrivilegedIdentifiers.machService,
            withServerRequirement: requirement
        )
    }

    static func send<E: Encodable, D: Decodable>(
        _ message: E,
        to route: XPCRouteWithMessageWithReply<E, D>,
        operation: HelperXPCOperation
    ) throws -> D {
        let client = try make()
        let box = ReplyBox<D>()
        let lock = DispatchSemaphore(value: 0)
        client.sendMessage(message, to: route, withResponse: { result in
            box.store(result)
            lock.signal()
        })
        let timeout = DispatchTime.now() + .milliseconds(operation.timeoutMilliseconds)
        if lock.wait(timeout: timeout) == .timedOut {
            throw operation.timeoutFailure
        }
        switch box.load() {
        case .success(let value):
            return value
        case .failure(let error):
            if operation.responseFailureHasUnknownOutcome(error) {
                throw BridgeFailure.operationOutcomeUnknown
            }
            throw error
        case .none:
            throw operation.timeoutFailure
        }
    }

    private static func secRequirement(_ string: String) throws -> SecRequirement {
        var requirement: SecRequirement?
        let status = SecRequirementCreateWithString(string as CFString, SecCSFlags(), &requirement)
        guard status == errSecSuccess, let requirement else {
            throw BridgeFailure.helperUnavailable
        }
        return requirement
    }
}

private final class ReplyBox<D> {
    private let lock = NSLock()
    private var result: Result<D, XPCError>?

    func store(_ result: Result<D, XPCError>) {
        lock.lock()
        self.result = result
        lock.unlock()
    }

    func load() -> Result<D, XPCError>? {
        lock.lock()
        defer { lock.unlock() }
        return result
    }
}

enum BridgeFailure: Error, Equatable {
    case helperUnavailable
    /// A mutating request crossed the trust boundary, but no authoritative
    /// reply arrived before the bounded wait. Callers must enter recovery
    /// semantics and must not retry it as an ordinary transport failure.
    case operationOutcomeUnknown
    case authorizationCancelled
    case invalidRequest
}