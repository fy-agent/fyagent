import Foundation
import FyAgentPrivilegedProtocol

struct ProtocolTests {
    static func rejectsUnknownProduct() throws {
        expectThrows(ProtocolError.unknownProduct) {
            _ = try KnownProduct(validating: 99)
        }
        expectThrows(ProtocolError.unknownProduct) {
            _ = try decodeCommit(mutating: { $0["product"] = 99 })
        }
    }

    static func rejectsUnknownSlotBeforeMutationSemantics() throws {
        expectThrows(ProtocolError.targetSlotInvalid) {
            _ = try KnownApplicationPolicyTable.resolve(
                product: .openCodeDesktop,
                slot: 2,
                action: .freshInstall
            )
        }
        expectThrows(ProtocolError.targetSlotInvalid) {
            _ = try decodeCommit(mutating: { $0["targetSlot"] = 9 })
        }
    }

    static func rejectsUnknownAction() {
        expectThrows(ProtocolError.unknownAction) {
            _ = try decodeCommit(mutating: { $0["action"] = 7 })
        }
    }

    static func rejectsNonzeroReserved() {
        expectThrows(ProtocolError.reservedNonzero) {
            _ = try decodeCommit(mutating: { $0["reserved"] = 1 })
        }
    }

    static func rejectsExtraFields() {
        expectThrows(ProtocolError.unexpectedField("extra")) {
            _ = try decodeCommit(mutating: { $0["extra"] = "nope" })
        }
        expectThrows(ProtocolError.unexpectedField("path")) {
            _ = try decodeCommit(mutating: { $0["path"] = "/tmp/app.app" })
        }
    }

    static func rejectsUnknownOperation() {
        expectThrows(ProtocolError.unknownOperation) {
            _ = try PrivilegedOperation(validating: 99)
        }
    }

    static func codexExistingOnlySlotCannotFreshInstall() throws {
        expectThrows(ProtocolError.targetSlotInvalid) {
            _ = try KnownApplicationPolicyTable.resolve(
                product: .codexDesktop,
                slot: 2,
                action: .freshInstall
            )
        }
        _ = try KnownApplicationPolicyTable.resolve(
            product: .codexDesktop,
            slot: 2,
            action: .updateExisting
        )
    }

    static func codingKeysContainNoPathUrlOrCommand() throws {
        let keys =
            ClosedCommitFields.CodingKeys.allCases.map(\.rawValue)
            + HelperStatusRequest.CodingKeys.allCases.map(\.rawValue)
            + HelperStatusReply.CodingKeys.allCases.map(\.rawValue)
            + KnownApplicationCommitResult.CodingKeys.allCases.map(\.rawValue)
            + RemoveHelperRequest.CodingKeys.allCases.map(\.rawValue)
            + RemoveHelperResult.CodingKeys.allCases.map(\.rawValue)
            + KnownApplicationCommitRequest.CodingKeys.allCases.map(\.rawValue)
            + AuthorizedRemoveHelperRequest.CodingKeys.allCases.map(\.rawValue)
        try ForbiddenWireKeys.assertAllowedCodingKeys(keys)
    }

    static func protocolSourcesContainNoPathUrlCommandKeys() throws {
        let root = packageRoot().appendingPathComponent("Sources/FyAgentPrivilegedProtocol")
        let files = try FileManager.default.contentsOfDirectory(at: root, includingPropertiesForKeys: nil)
            .filter { $0.pathExtension == "swift" }
        expect(!files.isEmpty)
        for file in files {
            let source = try String(contentsOf: file, encoding: .utf8)
            for forbidden in [
                "case path", "case url", "case command", "case argv",
                "case destination", "case sourcePath", "case targetPath",
            ] {
                expect(!source.contains(forbidden), "\(file.lastPathComponent) contains \(forbidden)")
            }
        }
    }

    static func validCommitFieldsRoundTrip() throws {
        let fields = try validFields()
        let data = try JSONEncoder().encode(fields)
        let decoded = try JSONDecoder().decode(ClosedCommitFields.self, from: data)
        expect(decoded == fields)
    }

    static func commitReplyEnvelopeAndOutcomeAreBoundToTheRequest() throws {
        let operationId = UUID()
        let valid = KnownApplicationCommitResult(
            operationId: operationId,
            outcome: .committed,
            reason: .none
        )
        try valid.validate(expectedOperationId: operationId)

        expectThrows(ProtocolError.invalidOperationId) {
            try valid.validate(expectedOperationId: UUID())
        }
        expectThrows(ProtocolError.protocolIncompatible) {
            try KnownApplicationCommitResult(
                operationId: operationId,
                outcome: .committed,
                reason: .commitFailed
            ).validate(expectedOperationId: operationId)
        }
        expectThrows(ProtocolError.protocolIncompatible) {
            try KnownApplicationCommitResult(
                operationId: operationId,
                outcome: .ready,
                reason: .none
            ).validate(expectedOperationId: operationId)
        }
        try KnownApplicationCommitResult(
            operationId: operationId,
            outcome: .failed,
            reason: .sourceChanged
        ).validate(expectedOperationId: operationId)
        try KnownApplicationCommitResult(
            operationId: operationId,
            outcome: .failed,
            reason: .operationAuthorizationInvalid
        ).validate(expectedOperationId: operationId)
        try KnownApplicationCommitResult(
            operationId: operationId,
            outcome: .failed,
            reason: .operationAuthorizationCancelled
        ).validate(expectedOperationId: operationId)
    }

    static func removeReplyAcceptsOnlyClosedTerminalPairs() throws {
        let operationId = UUID()
        try RemoveHelperResult(
            operationId: operationId,
            outcome: .ready,
            reason: .none
        ).validate(expectedOperationId: operationId)
        try RemoveHelperResult(
            operationId: operationId,
            outcome: .recoveryRequired,
            reason: .recoveryRequired
        ).validate(expectedOperationId: operationId)
        try RemoveHelperResult(
            operationId: operationId,
            outcome: .failed,
            reason: .helperRemovalFailed
        ).validate(expectedOperationId: operationId)
        expectThrows(ProtocolError.protocolIncompatible) {
            try RemoveHelperResult(
                operationId: operationId,
                outcome: .committed,
                reason: .none
            ).validate(expectedOperationId: operationId)
        }
        expectThrows(ProtocolError.protocolIncompatible) {
            try RemoveHelperResult(
                operationId: operationId,
                outcome: .failed,
                reason: .commitFailed
            ).validate(expectedOperationId: operationId)
        }
    }
}

private func validFields() throws -> ClosedCommitFields {
    try ClosedCommitFields(
        protocolVersion: 1,
        operationId: UUID(),
        action: .freshInstall,
        product: .openCodeDesktop,
        targetSlot: 1,
        expectedTargetRevision: Data(repeating: 0, count: 32),
        expectedSourceRevision: Data(repeating: 7, count: 32),
        reserved: 0
    )
}

private func decodeCommit(mutating: (inout [String: Any]) -> Void) throws -> ClosedCommitFields {
    var object: [String: Any] = [
        "protocolVersion": 1,
        "operationId": UUID().uuidString,
        "action": 1,
        "product": 2,
        "targetSlot": 1,
        "expectedTargetRevision": Data(repeating: 0, count: 32).base64EncodedString(),
        "expectedSourceRevision": Data(repeating: 7, count: 32).base64EncodedString(),
        "reserved": 0,
    ]
    mutating(&object)
    let data = try JSONSerialization.data(withJSONObject: object)
    return try JSONDecoder().decode(ClosedCommitFields.self, from: data)
}

private func packageRoot() -> URL {
    URL(fileURLWithPath: #filePath)
        .deletingLastPathComponent()
        .deletingLastPathComponent()
        .deletingLastPathComponent()
}
