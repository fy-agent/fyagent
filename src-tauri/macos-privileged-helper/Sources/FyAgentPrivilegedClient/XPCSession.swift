import Foundation
import FyAgentPrivilegedProtocol
import SecureXPC

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
        to route: XPCRouteWithMessageWithReply<E, D>
    ) throws -> D {
        let client = try make()
        let box = ReplyBox<D>()
        let lock = DispatchSemaphore(value: 0)
        client.sendMessage(message, to: route, withResponse: { result in
            box.result = result
            lock.signal()
        })
        if lock.wait(timeout: .now() + 0.4) == .timedOut {
            throw BridgeFailure.helperUnavailable
        }
        switch box.result {
        case .success(let value):
            return value
        case .failure(let error):
            throw error
        case .none:
            throw BridgeFailure.helperUnavailable
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
    var result: Result<D, XPCError>?
}

enum BridgeFailure: Error {
    case helperUnavailable
    case authorizationCancelled
    case invalidRequest
}
