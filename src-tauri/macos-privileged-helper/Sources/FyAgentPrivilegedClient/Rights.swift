import Authorized
import Foundation
import FyAgentPrivilegedProtocol

enum PrivilegedRights {
    static let commit = AuthorizationRight(name: PrivilegedIdentifiers.commitRightName)
    static let remove = AuthorizationRight(name: PrivilegedIdentifiers.removeRightName)

    static func ensureDefined() throws {
        if try !commit.isDefined() {
            try commit.createOrUpdateDefinition(
                rules: [CannedAuthorizationRightRules.authenticateAsAdmin],
                descriptionKey: "FyAgent needs administrator authorization to install or update a known application in /Applications.",
                comment: "Per-operation right for known-application system commit."
            )
        }
        if try !remove.isDefined() {
            try remove.createOrUpdateDefinition(
                rules: [CannedAuthorizationRightRules.authenticateAsAdmin],
                descriptionKey: "FyAgent needs administrator authorization to remove its privileged helper.",
                comment: "Per-operation right for privileged helper removal."
            )
        }
    }

    static func requestCommit() throws -> Authorization {
        try ensureDefined()
        let authorization = try Authorization()
        _ = try authorization.requestRights(
            [commit],
            environment: [],
            options: [.interactionAllowed, .extendRights]
        )
        return authorization
    }

    static func requestRemove() throws -> Authorization {
        try ensureDefined()
        let authorization = try Authorization()
        _ = try authorization.requestRights(
            [remove],
            environment: [],
            options: [.interactionAllowed, .extendRights]
        )
        return authorization
    }

    static func recheck(_ authorization: Authorization, right: AuthorizationRight) throws {
        _ = try authorization.requestRights(
            [right],
            environment: [],
            options: [.extendRights]
        )
    }
}
