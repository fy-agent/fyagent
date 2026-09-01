import Darwin
import Foundation
import FyAgentPrivilegedProtocol

public struct TransactionEnvironment: Equatable {
    public var applicationsParent: URL
    public var receiptDirectory: URL
    public var productionLocked: Bool

    public static let production = TransactionEnvironment(
        applicationsParent: URL(fileURLWithPath: PrivilegedIdentifiers.productionApplicationsParent, isDirectory: true),
        receiptDirectory: URL(fileURLWithPath: PrivilegedIdentifiers.productionReceiptDirectory, isDirectory: true),
        productionLocked: true
    )

    public static func testing(applicationsParent: URL, receiptDirectory: URL) -> TransactionEnvironment {
        TransactionEnvironment(
            applicationsParent: applicationsParent,
            receiptDirectory: receiptDirectory,
            productionLocked: false
        )
    }

    public func validate() throws {
        if productionLocked {
            let parent = applicationsParent.standardizedFileURL.path
            if parent != PrivilegedIdentifiers.productionApplicationsParent {
                throw TransactionError.permissionDenied
            }
            if receiptDirectory.standardizedFileURL.path != PrivilegedIdentifiers.productionReceiptDirectory {
                throw TransactionError.permissionDenied
            }
        }
    }
}

public struct CommitRequest {
    public var operationId: UUID
    public var action: CommitAction
    public var product: KnownProduct
    public var targetSlot: UInt32
    public var expectedSourceRevision: Data
    public var expectedTargetRevision: Data
    public var sourceDirectoryFD: Int32
    public var reserved: UInt32

    public init(
        operationId: UUID,
        action: CommitAction,
        product: KnownProduct,
        targetSlot: UInt32,
        expectedSourceRevision: Data,
        expectedTargetRevision: Data,
        sourceDirectoryFD: Int32,
        reserved: UInt32 = 0
    ) {
        self.operationId = operationId
        self.action = action
        self.product = product
        self.targetSlot = targetSlot
        self.expectedSourceRevision = expectedSourceRevision
        self.expectedTargetRevision = expectedTargetRevision
        self.sourceDirectoryFD = sourceDirectoryFD
        self.reserved = reserved
    }
}

public enum SystemCommit {
    public static func recover(environment: TransactionEnvironment) throws -> RecoveryStatus {
        try environment.validate()
        return try Recovery.recover(environment: environment)
    }

    public static func commit(
        _ request: CommitRequest,
        environment: TransactionEnvironment,
        hooks: TransactionHooks = TransactionHooks()
    ) throws -> CommitResult {
        try environment.validate()
        if request.reserved != 0 {
            throw TransactionError.reservedNonzero
        }
        let fields: ClosedCommitFields
        do {
            fields = try ClosedCommitFields(
                protocolVersion: PrivilegedIdentifiers.protocolVersion,
                operationId: request.operationId,
                action: request.action,
                product: request.product,
                targetSlot: request.targetSlot,
                expectedTargetRevision: request.expectedTargetRevision,
                expectedSourceRevision: request.expectedSourceRevision,
                reserved: request.reserved
            )
        } catch let error as ProtocolError {
            throw TransactionError.from(error)
        }

        do {
            let status = try recover(environment: environment)
            if status == .blocked {
                throw TransactionError.recoveryRequired
            }
        } catch {
            throw TransactionError.recoveryRequired
        }

        let (policy, slot) = try KnownApplicationPolicyTable.resolve(
            product: fields.product,
            slot: fields.targetSlot,
            action: fields.action
        )

        do {
            try DirectoryFD.requireDirectory(request.sourceDirectoryFD)
        } catch {
            throw TransactionError.sourceCapabilityInvalid
        }
        let sourceIdentity = try BundleIdentityReader.read(
            fromBundleFD: request.sourceDirectoryFD,
            policy: policy
        )
        if sourceIdentity.revision != fields.expectedSourceRevision {
            throw TransactionError.sourceChanged
        }

        let parentFD = try DirectoryFD.openDirectory(at: environment.applicationsParent)
        defer { DirectoryFD.close(parentFD) }

        if try DirectoryFD.exists(parentFD, slot.basename) {
            if fields.action == .freshInstall {
                throw TransactionError.targetChanged
            }
        } else if fields.action == .updateExisting {
            throw TransactionError.targetChanged
        }

        if fields.action == .updateExisting {
            if isRunning(basename: slot.basename, parent: environment.applicationsParent, hooks: hooks) {
                throw TransactionError.applicationRunning
            }
            let targetFD = try DirectoryFD.openAtDirectory(parentFD, slot.basename)
            let targetIdentity: BundleIdentity
            do {
                targetIdentity = try BundleIdentityReader.read(fromBundleFD: targetFD, policy: policy)
            } catch {
                DirectoryFD.close(targetFD)
                throw error
            }
            DirectoryFD.close(targetFD)
            if targetIdentity.revision != fields.expectedTargetRevision {
                throw TransactionError.targetChanged
            }
        }

        let stageName = GeneratedNames.stageName(for: fields.operationId)
        let backupName = GeneratedNames.backupName(for: fields.operationId)
        if try DirectoryFD.exists(parentFD, stageName) || DirectoryFD.exists(parentFD, backupName) {
            throw TransactionError.commitFailed("preexisting generated name")
        }

        var receipt = TransactionReceipt(
            operationId: fields.operationId,
            product: fields.product.rawValue,
            targetSlot: fields.targetSlot,
            action: fields.action,
            phase: .preparing,
            stageName: stageName,
            backupName: fields.action == .updateExisting ? backupName : nil,
            sourceRevision: sourceIdentity.revision,
            targetRevision: fields.action == .updateExisting ? fields.expectedTargetRevision : nil
        )
        try ReceiptStore.write(receipt, in: environment.receiptDirectory)

        do {
            try OpenAtCopier.copyBundle(
                fromSourceFD: request.sourceDirectoryFD,
                toParentFD: parentFD,
                stageName: stageName
            )
            let stageFD = try DirectoryFD.openAtDirectory(parentFD, stageName)
            defer { DirectoryFD.close(stageFD) }
            let stageIdentity = try BundleIdentityReader.read(fromBundleFD: stageFD, policy: policy)
            if stageIdentity.revision != sourceIdentity.revision {
                throw TransactionError.sourceChanged
            }
            try DirectoryFD.fsync(parentFD)

            receipt.phase = .readyToCommit
            try ReceiptStore.write(receipt, in: environment.receiptDirectory)

            if fields.action == .updateExisting {
                let liveTargetFD = try DirectoryFD.openAtDirectory(parentFD, slot.basename)
                let liveIdentity: BundleIdentity
                do {
                    liveIdentity = try BundleIdentityReader.read(fromBundleFD: liveTargetFD, policy: policy)
                } catch {
                    DirectoryFD.close(liveTargetFD)
                    throw error
                }
                DirectoryFD.close(liveTargetFD)
                if liveIdentity.revision != fields.expectedTargetRevision {
                    throw TransactionError.targetChanged
                }
                try DirectoryFD.renameAt(parentFD, from: slot.basename, to: backupName)
                try DirectoryFD.fsync(parentFD)
                receipt.phase = .backupCreated
                try ReceiptStore.write(receipt, in: environment.receiptDirectory)
            }

            try DirectoryFD.renameAt(parentFD, from: stageName, to: slot.basename)
            try DirectoryFD.fsync(parentFD)
            receipt.phase = .replacementCommitted
            try ReceiptStore.write(receipt, in: environment.receiptDirectory)

            if let hook = hooks.afterReplacementCommitted {
                try hook()
            }

            let installedFD = try DirectoryFD.openAtDirectory(parentFD, slot.basename)
            defer { DirectoryFD.close(installedFD) }
            let installedIdentity = try BundleIdentityReader.read(fromBundleFD: installedFD, policy: policy)
            if installedIdentity.revision != sourceIdentity.revision || hooks.forceVerificationFailure {
                return try rollback(
                    receipt: receipt,
                    parentFD: parentFD,
                    slot: slot,
                    policy: policy,
                    environment: environment
                )
            }

            if let backup = receipt.backupName, try DirectoryFD.exists(parentFD, backup) {
                try Recovery.removeOwnedBackup(
                    parentFD: parentFD,
                    backupName: backup,
                    policy: policy,
                    expectedRevision: receipt.targetRevision
                )
            }
            try ReceiptStore.remove(receipt, in: environment.receiptDirectory)
            try DirectoryFD.fsync(parentFD)
            return CommitResult(outcome: .committed, reason: .none, installedRevision: installedIdentity.revision)
        } catch let error as TransactionError {
            return try handleFailure(
                error,
                receipt: receipt,
                parentFD: parentFD,
                slot: slot,
                policy: policy,
                environment: environment
            )
        } catch {
            return try handleFailure(
                .commitFailed("copy"),
                receipt: receipt,
                parentFD: parentFD,
                slot: slot,
                policy: policy,
                environment: environment
            )
        }
    }

    private static func handleFailure(
        _ error: TransactionError,
        receipt: TransactionReceipt,
        parentFD: Int32,
        slot: TargetSlotPolicy,
        policy: KnownApplicationPolicy,
        environment: TransactionEnvironment
    ) throws -> CommitResult {
        if receipt.phase == .preparing || receipt.phase == .readyToCommit {
            if try DirectoryFD.exists(parentFD, receipt.stageName) {
                try? Recovery.removeTree(parentFD: parentFD, name: receipt.stageName)
            }
            try? ReceiptStore.remove(receipt, in: environment.receiptDirectory)
            throw error
        }
        if receipt.phase == .backupCreated || receipt.phase == .replacementCommitted {
            return try rollback(
                receipt: receipt,
                parentFD: parentFD,
                slot: slot,
                policy: policy,
                environment: environment
            )
        }
        throw error
    }

    private static func rollback(
        receipt: TransactionReceipt,
        parentFD: Int32,
        slot: TargetSlotPolicy,
        policy: KnownApplicationPolicy,
        environment: TransactionEnvironment
    ) throws -> CommitResult {
        do {
            if try DirectoryFD.exists(parentFD, slot.basename) {
                try Recovery.removeOwnedReplacement(
                    parentFD: parentFD,
                    name: slot.basename,
                    policy: policy,
                    expectedRevision: receipt.sourceRevision
                )
            }
            if let backupName = receipt.backupName {
                try Recovery.restoreBackup(
                    parentFD: parentFD,
                    backupName: backupName,
                    targetName: slot.basename,
                    policy: policy,
                    expectedRevision: receipt.targetRevision
                )
                let restoredFD = try DirectoryFD.openAtDirectory(parentFD, slot.basename)
                defer { DirectoryFD.close(restoredFD) }
                let restored = try BundleIdentityReader.read(fromBundleFD: restoredFD, policy: policy)
                if let expected = receipt.targetRevision, restored.revision != expected {
                    throw TransactionError.recoveryRequired
                }
            }
            if try DirectoryFD.exists(parentFD, receipt.stageName) {
                try Recovery.removeTree(parentFD: parentFD, name: receipt.stageName)
            }
            try ReceiptStore.remove(receipt, in: environment.receiptDirectory)
            try DirectoryFD.fsync(parentFD)
            return CommitResult(outcome: .rollbackRestored, reason: .rollbackRestored)
        } catch {
            throw TransactionError.recoveryRequired
        }
    }

    private static func isRunning(basename: String, parent: URL, hooks: TransactionHooks) -> Bool {
        if let hook = hooks.isApplicationRunning {
            return hook(basename)
        }
        let path = parent.appendingPathComponent(basename).path
        return ApplicationRunning.isRunning(at: path)
    }
}

enum ApplicationRunning {
    static func isRunning(at path: String) -> Bool {
        var pids = [pid_t](repeating: 0, count: 64)
        let filled = pids.withUnsafeMutableBufferPointer { buffer -> Int32 in
            path.withCString { cPath in
                proc_listpidspath(
                    UInt32(PROC_ALL_PIDS),
                    0,
                    cPath,
                    0,
                    buffer.baseAddress,
                    Int32(buffer.count * MemoryLayout<pid_t>.size)
                )
            }
        }
        return filled > 0
    }
}
