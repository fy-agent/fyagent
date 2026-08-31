import Authorized
import Foundation
import FyAgentPrivilegedProtocol
import FyAgentPrivilegedTransaction
import SecureXPC

enum HelperServer {
    static func run() throws {
        _ = try SystemCommit.recover(environment: .production)
        let server = try XPCServer.forMachService()
        register(on: server)
        server.startAndBlock()
    }

    static func register(on server: XPCServer) {
        server.registerRoute(PrivilegedRoutes.status, handler: handleStatus)
        server.registerRoute(PrivilegedRoutes.commit, handler: handleCommit)
        server.registerRoute(PrivilegedRoutes.remove, handler: handleRemove)
    }

    private static func handleStatus(_ request: HelperStatusRequest) throws -> HelperStatusReply {
        try request.validate()
        let recovery: RecoveryStatus
        do {
            recovery = try SystemCommit.recover(environment: .production)
        } catch {
            return HelperStatusReply(state: .recoveryRequired, reason: .recoveryRequired, activeRecovery: true)
        }
        if recovery == .blocked {
            return HelperStatusReply(state: .recoveryRequired, reason: .recoveryRequired, activeRecovery: true)
        }
        return HelperStatusReply(state: .ready, reason: .none, activeRecovery: false)
    }

    private static func handleCommit(_ request: KnownApplicationCommitRequest) throws -> KnownApplicationCommitResult {
        try recheck(request.authorization, rightName: PrivilegedIdentifiers.commitRightName)
        defer { try? request.authorization.destroyRights() }

        let sourceFD = request.sourceDirectory.wrappedValue.fileDescriptor
        let commit = CommitRequest(
            operationId: request.fields.operationId,
            action: request.fields.action,
            product: request.fields.product,
            targetSlot: request.fields.targetSlot,
            expectedSourceRevision: request.fields.expectedSourceRevision,
            expectedTargetRevision: request.fields.expectedTargetRevision,
            sourceDirectoryFD: sourceFD,
            reserved: request.fields.reserved
        )
        do {
            let result = try SystemCommit.commit(commit, environment: .production)
            return KnownApplicationCommitResult(
                operationId: request.fields.operationId,
                outcome: result.outcome,
                reason: result.reason
            )
        } catch let error as TransactionError {
            if error == .recoveryRequired {
                return KnownApplicationCommitResult(
                    operationId: request.fields.operationId,
                    outcome: .recoveryRequired,
                    reason: .recoveryRequired
                )
            }
            return KnownApplicationCommitResult(
                operationId: request.fields.operationId,
                outcome: .failed,
                reason: error.reason
            )
        }
    }

    private static func handleRemove(_ request: AuthorizedRemoveHelperRequest) throws -> RemoveHelperResult {
        try request.request.validate()
        try recheck(request.authorization, rightName: PrivilegedIdentifiers.removeRightName)
        defer { try? request.authorization.destroyRights() }

        do {
            let recovery = try SystemCommit.recover(environment: .production)
            if recovery == .blocked {
                return RemoveHelperResult(
                    operationId: request.request.operationId,
                    outcome: .recoveryRequired,
                    reason: .recoveryRequired
                )
            }
        } catch {
            return RemoveHelperResult(
                operationId: request.request.operationId,
                outcome: .recoveryRequired,
                reason: .recoveryRequired
            )
        }

        do {
            try removeOwnedHelperArtifacts()
            return RemoveHelperResult(
                operationId: request.request.operationId,
                outcome: .ready,
                reason: .none
            )
        } catch {
            return RemoveHelperResult(
                operationId: request.request.operationId,
                outcome: .failed,
                reason: .helperRemovalFailed
            )
        }
    }

    private static func recheck(_ authorization: Authorization, rightName: String) throws {
        _ = try authorization.requestRights(
            [AuthorizationRight(name: rightName)],
            environment: [],
            options: [.extendRights]
        )
    }

    private static func removeOwnedHelperArtifacts() throws {
        let helper = URL(fileURLWithPath: "/Library/PrivilegedHelperTools/\(PrivilegedIdentifiers.helperIdentifier)")
        let plist = URL(fileURLWithPath: "/Library/LaunchDaemons/\(PrivilegedIdentifiers.helperIdentifier).plist")
        let receipts = URL(fileURLWithPath: PrivilegedIdentifiers.productionReceiptDirectory, isDirectory: true)
        try unlinkIfExists(helper)
        try unlinkIfExists(plist)
        if FileManager.default.fileExists(atPath: receipts.path) {
            try FileManager.default.removeItem(at: receipts)
        }
    }

    private static func unlinkIfExists(_ url: URL) throws {
        if FileManager.default.fileExists(atPath: url.path) {
            try FileManager.default.removeItem(at: url)
        }
    }
}
