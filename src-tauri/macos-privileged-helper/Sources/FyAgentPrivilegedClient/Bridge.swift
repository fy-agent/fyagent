import Authorized
import Blessed
import CFyAgentPrivilegedBridge
import Darwin
import Foundation
import FyAgentPrivilegedProtocol
import SecureXPC

@_cdecl("fyagent_privileged_invoke")
public func fyagent_privileged_invoke(
    request: UnsafePointer<FyAgentPrivilegedRequest>?,
    reply: UnsafeMutablePointer<FyAgentPrivilegedReply>?
) -> Int32 {
    guard let request, let reply else {
        return -1
    }
    let expectedSize = UInt32(MemoryLayout<FyAgentPrivilegedRequest>.size)
    if request.pointee.abi_version != UInt32(FYAGENT_PRIVILEGED_ABI_VERSION) || request.pointee.size != expectedSize {
        return -1
    }
    var output = FyAgentPrivilegedReply()
    output.abi_version = UInt32(FYAGENT_PRIVILEGED_ABI_VERSION)
    output.size = UInt32(MemoryLayout<FyAgentPrivilegedReply>.size)
    output.protocol_version = PrivilegedIdentifiers.protocolVersion
    output.operation_id = request.pointee.operation_id
    output.reserved0 = 0

    if request.pointee.reserved0 != 0 || request.pointee.reserved1 != 0 {
        output.outcome = UInt32(FYAGENT_PRIVILEGED_OUTCOME_FAILED)
        output.reason = UInt32(FYAGENT_PRIVILEGED_REASON_RESERVED_NONZERO)
        output.helper_state = UInt32(FYAGENT_PRIVILEGED_HELPER_STATE_MISSING)
        reply.pointee = output
        return 0
    }
    if request.pointee.protocol_version != PrivilegedIdentifiers.protocolVersion {
        output.outcome = UInt32(FYAGENT_PRIVILEGED_OUTCOME_FAILED)
        output.reason = UInt32(FYAGENT_PRIVILEGED_REASON_HELPER_PROTOCOL_INCOMPATIBLE)
        output.helper_state = UInt32(FYAGENT_PRIVILEGED_HELPER_STATE_INCOMPATIBLE)
        reply.pointee = output
        return 0
    }

    do {
        let operation = try PrivilegedOperation(validating: request.pointee.operation)
        switch operation {
        case .status:
            output = try performStatus(template: output)
        case .ensureHelper:
            output = try performEnsureHelper(template: output)
        case .commit:
            output = try performCommit(request.pointee, template: output)
        case .removeHelper:
            output = try performRemove(request.pointee, template: output)
        }
    } catch let error as ProtocolError {
        output.outcome = UInt32(FYAGENT_PRIVILEGED_OUTCOME_FAILED)
        output.reason = UInt32(reason(from: error).rawValue)
        output.helper_state = UInt32(FYAGENT_PRIVILEGED_HELPER_STATE_MISSING)
    } catch {
        output.outcome = UInt32(FYAGENT_PRIVILEGED_OUTCOME_FAILED)
        output.reason = UInt32(mapTransportReason(error).rawValue)
        output.helper_state = UInt32(FYAGENT_PRIVILEGED_HELPER_STATE_MISSING)
    }
    reply.pointee = output
    return 0
}

private func performStatus(template: FyAgentPrivilegedReply) throws -> FyAgentPrivilegedReply {
    var output = template
    let bundled: BundledHelperDescriptor
    let clientVersion: HelperBundleVersion
    do {
        bundled = try HelperLifecycle.bundledHelper()
        clientVersion = try HelperLifecycle.clientVersion()
    } catch let error as HelperLifecycleError {
        return lifecyclePackagingFailure(error, template: output)
    }
    do {
        let status = try queryHelperStatus()
        return lifecycleReply(
            HelperLifecycle.decide(
                status: status,
                bundledVersion: bundled.version,
                clientVersion: clientVersion
            ),
            template: output
        )
    } catch {
        return transportFailureReply(error, template: output)
    }
}

private func performEnsureHelper(template: FyAgentPrivilegedReply) throws -> FyAgentPrivilegedReply {
    var output = template
    guard let executables = Bundle.main.infoDictionary?["SMPrivilegedExecutables"] as? [String: String],
          executables.count == 1,
          executables[PrivilegedIdentifiers.helperIdentifier] != nil else {
        output.outcome = UInt32(FYAGENT_PRIVILEGED_OUTCOME_FAILED)
        output.reason = UInt32(FYAGENT_PRIVILEGED_REASON_HELPER_NOT_PACKAGED)
        output.helper_state = UInt32(FYAGENT_PRIVILEGED_HELPER_STATE_MISSING)
        return output
    }
    let bundled: BundledHelperDescriptor
    let clientVersion: HelperBundleVersion
    do {
        bundled = try HelperLifecycle.bundledHelper()
        clientVersion = try HelperLifecycle.clientVersion()
    } catch let error as HelperLifecycleError {
        return lifecyclePackagingFailure(error, template: output)
    }

    do {
        let status = try queryHelperStatus()
        let decision = HelperLifecycle.decide(
            status: status,
            bundledVersion: bundled.version,
            clientVersion: clientVersion
        )
        switch decision {
        case .ready:
            return lifecycleReply(decision, template: output)
        case .installOrUpdate:
            break
        case .failed:
            return lifecycleReply(decision, template: output)
        }
    } catch let error as XPCError {
        switch error {
        case .connectionInvalid:
            // A non-installed Mach service is the only status failure that
            // authorizes installation. Other transport/protocol failures may
            // describe an existing helper and must not trigger overwrite.
            break
        default:
            return transportFailureReply(error, template: output)
        }
    } catch {
        return transportFailureReply(error, template: output)
    }

    do {
        try PrivilegedHelperManager.shared.authorizeAndBless()
    } catch let error as AuthorizationError {
        output.outcome = UInt32(FYAGENT_PRIVILEGED_OUTCOME_FAILED)
        output.helper_state = UInt32(FYAGENT_PRIVILEGED_HELPER_STATE_MISSING)
        if case .canceled = error {
            output.reason = UInt32(FYAGENT_PRIVILEGED_REASON_HELPER_INSTALL_AUTHORIZATION_CANCELLED)
        } else {
            output.reason = UInt32(FYAGENT_PRIVILEGED_REASON_HELPER_INSTALL_FAILED)
        }
        return output
    } catch {
        // A launchd cold-start race or an equal-version helper can make Bless
        // report failure after a usable trusted helper becomes reachable.
        if let status = try? queryHelperStatus() {
            let decision = HelperLifecycle.decide(
                status: status,
                bundledVersion: bundled.version,
                clientVersion: clientVersion
            )
            if decision == .ready {
                return lifecycleReply(decision, template: output)
            }
        }
        output.outcome = UInt32(FYAGENT_PRIVILEGED_OUTCOME_FAILED)
        output.reason = UInt32(FYAGENT_PRIVILEGED_REASON_HELPER_INSTALL_FAILED)
        output.helper_state = UInt32(FYAGENT_PRIVILEGED_HELPER_STATE_MISSING)
        return output
    }

    do {
        let status = try queryHelperStatusAfterBless()
        return lifecycleReply(
            HelperLifecycle.decide(
                status: status,
                bundledVersion: bundled.version,
                clientVersion: clientVersion
            ),
            template: output
        )
    } catch {
        return transportFailureReply(error, template: output)
    }
}

private func queryHelperStatus() throws -> HelperStatusReply {
    try HelperXPCClient.send(
        HelperStatusRequest(),
        to: PrivilegedRoutes.status,
        operation: .status
    )
}

private func queryHelperStatusAfterBless() throws -> HelperStatusReply {
    var lastError: Error = BridgeFailure.helperUnavailable
    for attempt in 0 ..< 10 {
        do {
            return try queryHelperStatus()
        } catch let error as XPCError {
            switch error {
            case .connectionInvalid, .connectionInterrupted, .terminationImminent:
                lastError = error
            default:
                throw error
            }
        } catch {
            throw error
        }
        if attempt < 9 {
            usleep(100_000)
        }
    }
    throw lastError
}

private func lifecycleReply(
    _ decision: HelperLifecycleDecision,
    template: FyAgentPrivilegedReply
) -> FyAgentPrivilegedReply {
    var output = template
    switch decision {
    case .ready:
        output.outcome = UInt32(FYAGENT_PRIVILEGED_OUTCOME_READY)
        output.reason = UInt32(FYAGENT_PRIVILEGED_REASON_NONE)
        output.helper_state = UInt32(FYAGENT_PRIVILEGED_HELPER_STATE_READY)
    case .installOrUpdate:
        output.outcome = UInt32(FYAGENT_PRIVILEGED_OUTCOME_FAILED)
        output.reason = UInt32(FYAGENT_PRIVILEGED_REASON_HELPER_UPDATE_REQUIRED)
        output.helper_state = UInt32(FYAGENT_PRIVILEGED_HELPER_STATE_UPDATE_REQUIRED)
    case .failed(let state, let reason):
        output.outcome = state == .recoveryRequired
            ? UInt32(FYAGENT_PRIVILEGED_OUTCOME_RECOVERY_REQUIRED)
            : UInt32(FYAGENT_PRIVILEGED_OUTCOME_FAILED)
        output.reason = reason.rawValue
        output.helper_state = state.rawValue
    }
    return output
}

private func lifecyclePackagingFailure(
    _ error: HelperLifecycleError,
    template: FyAgentPrivilegedReply
) -> FyAgentPrivilegedReply {
    var output = template
    output.outcome = UInt32(FYAGENT_PRIVILEGED_OUTCOME_FAILED)
    output.helper_state = UInt32(FYAGENT_PRIVILEGED_HELPER_STATE_MISSING)
    switch error {
    case .helperMissing:
        output.reason = UInt32(FYAGENT_PRIVILEGED_REASON_HELPER_NOT_PACKAGED)
    case .appIdentityInvalid, .helperNotRegular, .helperMetadataInvalid:
        output.reason = UInt32(FYAGENT_PRIVILEGED_REASON_HELPER_SIGNATURE_INVALID)
    }
    return output
}

private func protocolFailureReply(
    template: FyAgentPrivilegedReply
) -> FyAgentPrivilegedReply {
    var output = template
    output.outcome = UInt32(FYAGENT_PRIVILEGED_OUTCOME_FAILED)
    output.reason = UInt32(FYAGENT_PRIVILEGED_REASON_HELPER_PROTOCOL_INCOMPATIBLE)
    output.helper_state = UInt32(FYAGENT_PRIVILEGED_HELPER_STATE_INCOMPATIBLE)
    return output
}

private func transportFailureReply(
    _ error: Error,
    template: FyAgentPrivilegedReply
) -> FyAgentPrivilegedReply {
    var output = template
    let reason = mapTransportReason(error)
    output.outcome = UInt32(FYAGENT_PRIVILEGED_OUTCOME_FAILED)
    output.reason = reason.rawValue
    switch reason {
    case .helperPeerRejected, .helperProtocolIncompatible, .helperSignatureInvalid:
        output.helper_state = UInt32(FYAGENT_PRIVILEGED_HELPER_STATE_INCOMPATIBLE)
    default:
        output.helper_state = UInt32(FYAGENT_PRIVILEGED_HELPER_STATE_MISSING)
    }
    return output
}

private func performCommit(
    _ request: FyAgentPrivilegedRequest,
    template: FyAgentPrivilegedReply
) throws -> FyAgentPrivilegedReply {
    var output = template
    let action = try CommitAction(validating: request.action)
    let product = try KnownProduct(validating: request.product)
    let operationId = uuidFromC(request.operation_id)
    let sourceRevision = dataFromC32(request.expected_source_revision)
    let targetRevision = dataFromC32(request.expected_target_revision)
    let fields = try ClosedCommitFields(
        protocolVersion: request.protocol_version,
        operationId: operationId,
        action: action,
        product: product,
        targetSlot: request.target_slot,
        expectedTargetRevision: targetRevision,
        expectedSourceRevision: sourceRevision,
        reserved: 0
    )
    if request.source_directory_fd < 0 {
        output.outcome = UInt32(FYAGENT_PRIVILEGED_OUTCOME_FAILED)
        output.reason = UInt32(FYAGENT_PRIVILEGED_REASON_SOURCE_CAPABILITY_INVALID)
        output.helper_state = UInt32(FYAGENT_PRIVILEGED_HELPER_STATE_MISSING)
        return output
    }
    let duplicated = dup(request.source_directory_fd)
    if duplicated < 0 {
        output.outcome = UInt32(FYAGENT_PRIVILEGED_OUTCOME_FAILED)
        output.reason = UInt32(FYAGENT_PRIVILEGED_REASON_SOURCE_CAPABILITY_INVALID)
        output.helper_state = UInt32(FYAGENT_PRIVILEGED_HELPER_STATE_MISSING)
        return output
    }
    let handle = FileHandle(fileDescriptor: duplicated, closeOnDealloc: true)
    let source = FileHandleForXPC(wrappedValue: handle, closeOnEncode: true)

    let authorization: Authorization
    do {
        authorization = try PrivilegedRights.requestCommit()
    } catch let error as AuthorizationError {
        output.outcome = UInt32(FYAGENT_PRIVILEGED_OUTCOME_FAILED)
        output.helper_state = UInt32(FYAGENT_PRIVILEGED_HELPER_STATE_MISSING)
        if case .canceled = error {
            output.reason = UInt32(FYAGENT_PRIVILEGED_REASON_OPERATION_AUTHORIZATION_CANCELLED)
        } else {
            output.reason = UInt32(FYAGENT_PRIVILEGED_REASON_OPERATION_AUTHORIZATION_INVALID)
        }
        return output
    }

    let message = KnownApplicationCommitRequest(
        fields: fields,
        sourceDirectory: source,
        authorization: authorization
    )
    defer { try? authorization.destroyRights() }
    do {
        let result = try HelperXPCClient.send(
            message,
            to: PrivilegedRoutes.commit,
            operation: .commit
        )
        do {
            try result.validate(expectedOperationId: operationId)
        } catch {
            return protocolFailureReply(template: output)
        }
        output.outcome = result.outcome.rawValue
        output.reason = result.reason.rawValue
        output.helper_state = UInt32(FYAGENT_PRIVILEGED_HELPER_STATE_READY)
        return output
    } catch BridgeFailure.operationOutcomeUnknown {
        output.outcome = UInt32(FYAGENT_PRIVILEGED_OUTCOME_RECOVERY_REQUIRED)
        output.reason = UInt32(FYAGENT_PRIVILEGED_REASON_RECOVERY_REQUIRED)
        output.helper_state = UInt32(FYAGENT_PRIVILEGED_HELPER_STATE_RECOVERY_REQUIRED)
        return output
    } catch {
        return transportFailureReply(error, template: output)
    }
}

private func performRemove(
    _ request: FyAgentPrivilegedRequest,
    template: FyAgentPrivilegedReply
) throws -> FyAgentPrivilegedReply {
    var output = template
    let operationId = uuidFromC(request.operation_id)
    let removeRequest = try RemoveHelperRequest(operationId: operationId)
    let authorization: Authorization
    do {
        authorization = try PrivilegedRights.requestRemove()
    } catch let error as AuthorizationError {
        output.outcome = UInt32(FYAGENT_PRIVILEGED_OUTCOME_FAILED)
        output.helper_state = UInt32(FYAGENT_PRIVILEGED_HELPER_STATE_MISSING)
        if case .canceled = error {
            output.reason = UInt32(FYAGENT_PRIVILEGED_REASON_OPERATION_AUTHORIZATION_CANCELLED)
        } else {
            output.reason = UInt32(FYAGENT_PRIVILEGED_REASON_OPERATION_AUTHORIZATION_INVALID)
        }
        return output
    }
    defer { try? authorization.destroyRights() }
    let message = AuthorizedRemoveHelperRequest(request: removeRequest, authorization: authorization)
    do {
        let result = try HelperXPCClient.send(
            message,
            to: PrivilegedRoutes.remove,
            operation: .remove
        )
        do {
            try result.validate(expectedOperationId: operationId)
        } catch {
            return protocolFailureReply(template: output)
        }
        output.outcome = result.outcome.rawValue
        output.reason = result.reason.rawValue
        output.helper_state = UInt32(FYAGENT_PRIVILEGED_HELPER_STATE_MISSING)
        return output
    } catch BridgeFailure.operationOutcomeUnknown {
        output.outcome = UInt32(FYAGENT_PRIVILEGED_OUTCOME_RECOVERY_REQUIRED)
        output.reason = UInt32(FYAGENT_PRIVILEGED_REASON_RECOVERY_REQUIRED)
        output.helper_state = UInt32(FYAGENT_PRIVILEGED_HELPER_STATE_RECOVERY_REQUIRED)
        return output
    } catch {
        output.outcome = UInt32(FYAGENT_PRIVILEGED_OUTCOME_FAILED)
        output.reason = UInt32(FYAGENT_PRIVILEGED_REASON_HELPER_REMOVAL_FAILED)
        output.helper_state = UInt32(FYAGENT_PRIVILEGED_HELPER_STATE_MISSING)
        return output
    }
}

private func reason(from error: ProtocolError) -> HelperReason {
    switch error {
    case .unexpectedField:
        return .unexpectedField
    case .reservedNonzero:
        return .reservedNonzero
    case .protocolIncompatible:
        return .helperProtocolIncompatible
    case .unknownProduct, .targetSlotInvalid, .unknownOperation, .unknownAction:
        return .targetSlotInvalid
    case .invalidRevision, .invalidOperationId:
        return .sourceCapabilityInvalid
    }
}

private func mapTransportReason(_ error: Error) -> HelperReason {
    if let error = error as? AuthorizationError, case .canceled = error {
        return .operationAuthorizationCancelled
    }
    if let error = error as? XPCError {
        switch error {
        case .insecure:
            return .helperPeerRejected
        case .decodingError, .routeNotRegistered, .routeMismatch, .handlerError:
            return .helperProtocolIncompatible
        default:
            return .helperNotPackaged
        }
    }
    return .helperNotPackaged
}

private func uuidFromC(_ bytes: (
    UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
    UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8
)) -> UUID {
    UUID(uuid: uuid_t(
        bytes.0, bytes.1, bytes.2, bytes.3, bytes.4, bytes.5, bytes.6, bytes.7,
        bytes.8, bytes.9, bytes.10, bytes.11, bytes.12, bytes.13, bytes.14, bytes.15
    ))
}

private func dataFromC32(_ bytes: (
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
