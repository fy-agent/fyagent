import CFyAgentPrivilegedBridge
@testable import FyAgentPrivilegedClient
import FyAgentPrivilegedProtocol

struct BridgeABITests {
    static func requestAndReplySizesAreStable() {
        expect(MemoryLayout<FyAgentPrivilegedRequest>.size == 120)
        expect(MemoryLayout<FyAgentPrivilegedReply>.size == 44)
        expect(FYAGENT_PRIVILEGED_ABI_VERSION == 1)
    }

    static func nullPointersFailClosed() {
        expect(fyagent_privileged_invoke(nil, nil) == -1)
    }

    static func wrongAbiVersionFailsClosed() {
        var request = FyAgentPrivilegedRequest()
        request.abi_version = 99
        request.size = UInt32(MemoryLayout<FyAgentPrivilegedRequest>.size)
        var reply = FyAgentPrivilegedReply()
        expect(fyagent_privileged_invoke(&request, &reply) == -1)
    }

    static func nonzeroReservedIsRejected() {
        var request = FyAgentPrivilegedRequest()
        request.abi_version = UInt32(FYAGENT_PRIVILEGED_ABI_VERSION)
        request.size = UInt32(MemoryLayout<FyAgentPrivilegedRequest>.size)
        request.protocol_version = PrivilegedIdentifiers.protocolVersion
        request.operation = UInt32(FYAGENT_PRIVILEGED_OPERATION_STATUS)
        request.reserved0 = 1
        var reply = FyAgentPrivilegedReply()
        expect(fyagent_privileged_invoke(&request, &reply) == 0)
        expect(reply.reason == UInt32(FYAGENT_PRIVILEGED_REASON_RESERVED_NONZERO))
        expect(reply.outcome == UInt32(FYAGENT_PRIVILEGED_OUTCOME_FAILED))
    }

    static func unknownOperationIsRejected() {
        var request = FyAgentPrivilegedRequest()
        request.abi_version = UInt32(FYAGENT_PRIVILEGED_ABI_VERSION)
        request.size = UInt32(MemoryLayout<FyAgentPrivilegedRequest>.size)
        request.protocol_version = PrivilegedIdentifiers.protocolVersion
        request.operation = 99
        var reply = FyAgentPrivilegedReply()
        expect(fyagent_privileged_invoke(&request, &reply) == 0)
        expect(reply.outcome == UInt32(FYAGENT_PRIVILEGED_OUTCOME_FAILED))
        expect(reply.reason == UInt32(FYAGENT_PRIVILEGED_REASON_TARGET_SLOT_INVALID))
    }

    static func statusWithoutHelperReportsMissing() {
        var request = FyAgentPrivilegedRequest()
        request.abi_version = UInt32(FYAGENT_PRIVILEGED_ABI_VERSION)
        request.size = UInt32(MemoryLayout<FyAgentPrivilegedRequest>.size)
        request.protocol_version = PrivilegedIdentifiers.protocolVersion
        request.operation = UInt32(FYAGENT_PRIVILEGED_OPERATION_STATUS)
        request.source_directory_fd = -1
        var reply = FyAgentPrivilegedReply()
        expect(fyagent_privileged_invoke(&request, &reply) == 0)
        expect(reply.helper_state == UInt32(FYAGENT_PRIVILEGED_HELPER_STATE_MISSING))
        expect(reply.reason == UInt32(FYAGENT_PRIVILEGED_REASON_HELPER_NOT_PACKAGED))
    }

    static func ensureHelperWithoutPackagingReportsMissing() {
        var request = FyAgentPrivilegedRequest()
        request.abi_version = UInt32(FYAGENT_PRIVILEGED_ABI_VERSION)
        request.size = UInt32(MemoryLayout<FyAgentPrivilegedRequest>.size)
        request.protocol_version = PrivilegedIdentifiers.protocolVersion
        request.operation = UInt32(FYAGENT_PRIVILEGED_OPERATION_ENSURE_HELPER)
        request.source_directory_fd = -1
        var reply = FyAgentPrivilegedReply()
        expect(fyagent_privileged_invoke(&request, &reply) == 0)
        expect(reply.helper_state == UInt32(FYAGENT_PRIVILEGED_HELPER_STATE_MISSING))
        expect(reply.reason == UInt32(FYAGENT_PRIVILEGED_REASON_HELPER_NOT_PACKAGED))
    }

    static func xpcWaitPolicyIsOperationSpecific() {
        expect(HelperXPCOperation.status.timeoutMilliseconds == 5_000)
        expect(HelperXPCOperation.remove.timeoutMilliseconds == 60_000)
        expect(HelperXPCOperation.commit.timeoutMilliseconds == 30 * 60 * 1_000)
        expect(
            HelperXPCOperation.status.timeoutMilliseconds
                < HelperXPCOperation.remove.timeoutMilliseconds
        )
        expect(
            HelperXPCOperation.remove.timeoutMilliseconds
                < HelperXPCOperation.commit.timeoutMilliseconds
        )
    }

    static func mutatingTimeoutsRequireRecoveryInsteadOfRetry() {
        expect(HelperXPCOperation.status.timeoutFailure == .helperUnavailable)
        expect(HelperXPCOperation.commit.timeoutFailure == .operationOutcomeUnknown)
        expect(HelperXPCOperation.remove.timeoutFailure == .operationOutcomeUnknown)
    }

    static func clientCommitRightsPreauthorizeHelperRecheck() {
        expect(PrivilegedRights.clientRequestOptions.contains(.preAuthorize))
        expect(PrivilegedRights.clientRequestOptions.contains(.interactionAllowed))
        expect(PrivilegedRights.clientRequestOptions.contains(.extendRights))
        expect(PrivilegedRights.helperRecheckOptions.contains(.extendRights))
        expect(!PrivilegedRights.helperRecheckOptions.contains(.interactionAllowed))
        expect(!PrivilegedRights.helperRecheckOptions.contains(.preAuthorize))
        expect(PrivilegedRights.cachedCredentialTimeoutSeconds >= 1)
    }

    static func mutatingTransportLossRequiresRecoveryExceptPeerRejection() {
        expect(
            HelperXPCOperation.commit.responseFailureHasUnknownOutcome(.connectionInterrupted)
        )
        expect(
            HelperXPCOperation.commit.responseFailureHasUnknownOutcome(
                .decodingError(description: "bad reply")
            )
        )
        expect(
            HelperXPCOperation.remove.responseFailureHasUnknownOutcome(.connectionInvalid)
        )
        expect(!HelperXPCOperation.commit.responseFailureHasUnknownOutcome(.insecure))
        expect(!HelperXPCOperation.status.responseFailureHasUnknownOutcome(.connectionInterrupted))
    }
}
