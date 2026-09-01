import Darwin
import Foundation
import FyAgentPrivilegedProtocol

public enum RecoveryStatus: Equatable {
    case idle
    case recovered
    case blocked
}

enum Recovery {
    static func recover(environment: TransactionEnvironment) throws -> RecoveryStatus {
        let receipts: [TransactionReceipt]
        do {
            receipts = try ReceiptStore.loadAll(in: environment.receiptDirectory)
        } catch {
            throw TransactionError.recoveryRequired
        }
        if receipts.isEmpty {
            return .idle
        }
        if receipts.count != 1 {
            throw TransactionError.recoveryRequired
        }
        let receipt = receipts[0]
        let parentFD = try DirectoryFD.openDirectory(at: environment.applicationsParent)
        defer { DirectoryFD.close(parentFD) }

        let product: KnownProduct
        do {
            product = try KnownProduct(validating: receipt.product)
        } catch {
            throw TransactionError.recoveryRequired
        }
        let policy = KnownApplicationPolicyTable.policy(for: product)
        let slot: TargetSlotPolicy
        do {
            slot = try policy.slot(receipt.targetSlot)
        } catch {
            throw TransactionError.recoveryRequired
        }

        switch receipt.phase {
        case .preparing, .readyToCommit:
            switch receipt.action {
            case .freshInstall:
                try recoverFreshBeforeCommit(
                    receipt: receipt,
                    parentFD: parentFD,
                    policy: policy,
                    slot: slot
                )
            case .updateExisting:
                try recoverUpdateBeforeBackup(
                    receipt: receipt,
                    parentFD: parentFD,
                    policy: policy,
                    slot: slot
                )
            case .none:
                throw TransactionError.recoveryRequired
            }
            try ReceiptStore.remove(receipt, in: environment.receiptDirectory)
            return .recovered
        case .backupCreated:
            try recoverBackupCreated(
                receipt: receipt,
                parentFD: parentFD,
                policy: policy,
                slot: slot
            )
            try ReceiptStore.remove(receipt, in: environment.receiptDirectory)
            return .recovered
        case .replacementCommitted:
            try recoverReplacementCommitted(
                receipt: receipt,
                parentFD: parentFD,
                policy: policy,
                slot: slot
            )
            try ReceiptStore.remove(receipt, in: environment.receiptDirectory)
            return .recovered
        }
    }

    private static func recoverFreshBeforeCommit(
        receipt: TransactionReceipt,
        parentFD: Int32,
        policy: KnownApplicationPolicy,
        slot: TargetSlotPolicy
    ) throws {
        let stageExists = try DirectoryFD.exists(parentFD, receipt.stageName)
        let targetExists = try DirectoryFD.exists(parentFD, slot.basename)
        if stageExists, targetExists {
            throw TransactionError.recoveryRequired
        }
        if targetExists {
            try requireRevision(
                parentFD: parentFD,
                name: slot.basename,
                policy: policy,
                expectedRevision: receipt.sourceRevision
            )
            return
        }
        if stageExists {
            try cleanupOwnedStage(receipt: receipt, parentFD: parentFD, policy: policy)
        }
    }

    private static func recoverUpdateBeforeBackup(
        receipt: TransactionReceipt,
        parentFD: Int32,
        policy: KnownApplicationPolicy,
        slot: TargetSlotPolicy
    ) throws {
        guard try DirectoryFD.exists(parentFD, slot.basename),
              let targetRevision = receipt.targetRevision else {
            throw TransactionError.recoveryRequired
        }
        try requireRevision(
            parentFD: parentFD,
            name: slot.basename,
            policy: policy,
            expectedRevision: targetRevision
        )
        if let backup = receipt.backupName, try DirectoryFD.exists(parentFD, backup) {
            throw TransactionError.recoveryRequired
        }
        try cleanupOwnedStage(receipt: receipt, parentFD: parentFD, policy: policy)
    }

    private static func cleanupOwnedStage(
        receipt: TransactionReceipt,
        parentFD: Int32,
        policy: KnownApplicationPolicy
    ) throws {
        guard try DirectoryFD.exists(parentFD, receipt.stageName) else {
            return
        }
        let stageFD = try DirectoryFD.openAtDirectory(parentFD, receipt.stageName)
        let identity: BundleIdentity
        do {
            identity = try BundleIdentityReader.read(fromBundleFD: stageFD, policy: policy)
        } catch {
            DirectoryFD.close(stageFD)
            throw error
        }
        DirectoryFD.close(stageFD)
        if identity.revision != receipt.sourceRevision {
            throw TransactionError.recoveryRequired
        }
        try removeTree(parentFD: parentFD, name: receipt.stageName)
    }

    private static func recoverBackupCreated(
        receipt: TransactionReceipt,
        parentFD: Int32,
        policy: KnownApplicationPolicy,
        slot: TargetSlotPolicy
    ) throws {
        let targetExists = try DirectoryFD.exists(parentFD, slot.basename)
        let backupName = receipt.backupName
        let backupExists = try backupName.map { try DirectoryFD.exists(parentFD, $0) } ?? false
        guard let backupName, backupExists else {
            throw TransactionError.recoveryRequired
        }
        if targetExists {
            if try DirectoryFD.exists(parentFD, receipt.stageName) {
                throw TransactionError.recoveryRequired
            }
            try requireRevision(
                parentFD: parentFD,
                name: slot.basename,
                policy: policy,
                expectedRevision: receipt.sourceRevision
            )
            try removeOwnedBackup(
                parentFD: parentFD,
                backupName: backupName,
                policy: policy,
                expectedRevision: receipt.targetRevision
            )
            return
        }
        try cleanupOwnedStage(receipt: receipt, parentFD: parentFD, policy: policy)
        try restoreBackup(
            parentFD: parentFD,
            backupName: backupName,
            targetName: slot.basename,
            policy: policy,
            expectedRevision: receipt.targetRevision
        )
    }

    private static func requireRevision(
        parentFD: Int32,
        name: String,
        policy: KnownApplicationPolicy,
        expectedRevision: Data
    ) throws {
        let fd = try DirectoryFD.openAtDirectory(parentFD, name)
        let identity: BundleIdentity
        do {
            identity = try BundleIdentityReader.read(fromBundleFD: fd, policy: policy)
        } catch {
            DirectoryFD.close(fd)
            throw error
        }
        DirectoryFD.close(fd)
        if identity.revision != expectedRevision {
            throw TransactionError.recoveryRequired
        }
    }

    private static func recoverReplacementCommitted(
        receipt: TransactionReceipt,
        parentFD: Int32,
        policy: KnownApplicationPolicy,
        slot: TargetSlotPolicy
    ) throws {
        if try DirectoryFD.exists(parentFD, slot.basename) {
            let targetFD = try DirectoryFD.openAtDirectory(parentFD, slot.basename)
            defer { DirectoryFD.close(targetFD) }
            if let identity = try? BundleIdentityReader.read(fromBundleFD: targetFD, policy: policy),
               identity.revision == receipt.sourceRevision {
                if let backupName = receipt.backupName, try DirectoryFD.exists(parentFD, backupName) {
                    try removeOwnedBackup(
                        parentFD: parentFD,
                        backupName: backupName,
                        policy: policy,
                        expectedRevision: receipt.targetRevision
                    )
                }
                return
            }
        }
        guard let backupName = receipt.backupName else {
            throw TransactionError.recoveryRequired
        }
        if try DirectoryFD.exists(parentFD, slot.basename) {
            try removeOwnedReplacement(
                parentFD: parentFD,
                name: slot.basename,
                policy: policy,
                expectedRevision: receipt.sourceRevision
            )
        }
        try restoreBackup(
            parentFD: parentFD,
            backupName: backupName,
            targetName: slot.basename,
            policy: policy,
            expectedRevision: receipt.targetRevision
        )
    }

    static func restoreBackup(
        parentFD: Int32,
        backupName: String,
        targetName: String,
        policy: KnownApplicationPolicy,
        expectedRevision: Data?
    ) throws {
        guard GeneratedNames.isGeneratedBackup(backupName) else {
            throw TransactionError.recoveryRequired
        }
        let backupFD = try DirectoryFD.openAtDirectory(parentFD, backupName)
        let identity: BundleIdentity
        do {
            identity = try BundleIdentityReader.read(fromBundleFD: backupFD, policy: policy)
        } catch {
            DirectoryFD.close(backupFD)
            throw error
        }
        DirectoryFD.close(backupFD)
        if let expectedRevision, identity.revision != expectedRevision {
            throw TransactionError.recoveryRequired
        }
        try DirectoryFD.renameAt(parentFD, from: backupName, to: targetName)
        try DirectoryFD.fsync(parentFD)
    }

    static func removeOwnedReplacement(
        parentFD: Int32,
        name: String,
        policy: KnownApplicationPolicy,
        expectedRevision: Data
    ) throws {
        let fd = try DirectoryFD.openAtDirectory(parentFD, name)
        let identity: BundleIdentity
        do {
            identity = try BundleIdentityReader.read(fromBundleFD: fd, policy: policy)
        } catch {
            DirectoryFD.close(fd)
            throw error
        }
        DirectoryFD.close(fd)
        if identity.revision != expectedRevision {
            throw TransactionError.recoveryRequired
        }
        try removeTree(parentFD: parentFD, name: name)
    }

    static func removeOwnedBackup(
        parentFD: Int32,
        backupName: String,
        policy: KnownApplicationPolicy,
        expectedRevision: Data?
    ) throws {
        guard GeneratedNames.isGeneratedBackup(backupName) else {
            throw TransactionError.recoveryRequired
        }
        let fd = try DirectoryFD.openAtDirectory(parentFD, backupName)
        let identity: BundleIdentity
        do {
            identity = try BundleIdentityReader.read(fromBundleFD: fd, policy: policy)
        } catch {
            DirectoryFD.close(fd)
            throw error
        }
        DirectoryFD.close(fd)
        if let expectedRevision, identity.revision != expectedRevision {
            throw TransactionError.recoveryRequired
        }
        try removeTree(parentFD: parentFD, name: backupName)
    }

    static func removeTree(parentFD: Int32, name: String) throws {
        if name.contains("/") || name == "." || name == ".." {
            throw TransactionError.commitFailed("invalid name")
        }
        var buffer = [CChar](repeating: 0, count: Int(PATH_MAX))
        if fcntl(parentFD, F_GETPATH, &buffer) != 0 {
            throw TransactionError.commitFailed("parent path")
        }
        let parentPath = String(cString: buffer)
        let url = URL(fileURLWithPath: parentPath, isDirectory: true).appendingPathComponent(name)
        try FileManager.default.removeItem(at: url)
        try DirectoryFD.fsync(parentFD)
    }
}
