import Foundation
import FyAgentPrivilegedProtocol

public enum ReceiptPhase: String, Codable, Equatable {
    case preparing
    case readyToCommit = "ready_to_commit"
    case backupCreated = "backup_created"
    case replacementCommitted = "replacement_committed"
}

public struct TransactionReceipt: Codable, Equatable {
    public var version: UInt32
    public var operationId: UUID
    public var product: UInt32
    public var targetSlot: UInt32
    public var phase: ReceiptPhase
    public var stageName: String
    public var backupName: String?
    public var sourceRevision: Data
    public var targetRevision: Data?

    public enum CodingKeys: String, CodingKey, CaseIterable {
        case version
        case operationId
        case product
        case targetSlot
        case phase
        case stageName
        case backupName
        case sourceRevision
        case targetRevision
    }

    public init(
        version: UInt32 = 1,
        operationId: UUID,
        product: UInt32,
        targetSlot: UInt32,
        phase: ReceiptPhase,
        stageName: String,
        backupName: String? = nil,
        sourceRevision: Data,
        targetRevision: Data? = nil
    ) {
        self.version = version
        self.operationId = operationId
        self.product = product
        self.targetSlot = targetSlot
        self.phase = phase
        self.stageName = stageName
        self.backupName = backupName
        self.sourceRevision = sourceRevision
        self.targetRevision = targetRevision
    }

    public init(from decoder: Decoder) throws {
        try StrictDecoder.rejectUnknownKeys(decoder, allowed: CodingKeys.self)
        let container = try decoder.container(keyedBy: CodingKeys.self)
        version = try container.decode(UInt32.self, forKey: .version)
        operationId = try container.decode(UUID.self, forKey: .operationId)
        product = try container.decode(UInt32.self, forKey: .product)
        targetSlot = try container.decode(UInt32.self, forKey: .targetSlot)
        phase = try container.decode(ReceiptPhase.self, forKey: .phase)
        stageName = try container.decode(String.self, forKey: .stageName)
        backupName = try container.decodeIfPresent(String.self, forKey: .backupName)
        sourceRevision = try container.decode(Data.self, forKey: .sourceRevision)
        targetRevision = try container.decodeIfPresent(Data.self, forKey: .targetRevision)
    }

    var fileName: String {
        "\(operationId.uuidString.lowercased()).receipt"
    }
}

public enum GeneratedNames {
    public static func stageName(for operationId: UUID) -> String {
        ".fyagent-system-stage-\(operationId.uuidString.lowercased()).app"
    }

    public static func backupName(for operationId: UUID) -> String {
        ".fyagent-system-backup-\(operationId.uuidString.lowercased()).backup"
    }

    public static func isGeneratedStage(_ name: String) -> Bool {
        name.hasPrefix(".fyagent-system-stage-") && name.hasSuffix(".app") && !name.contains("/")
    }

    public static func isGeneratedBackup(_ name: String) -> Bool {
        name.hasPrefix(".fyagent-system-backup-") && name.hasSuffix(".backup") && !name.contains("/")
    }
}

public enum ReceiptStore {
    static let currentVersion: UInt32 = 1
    static let maxReceipts = 8

    static func ensureDirectory(_ url: URL) throws {
        try FileManager.default.createDirectory(at: url, withIntermediateDirectories: true, attributes: [
            .posixPermissions: 0o700,
        ])
    }

    public static func write(_ receipt: TransactionReceipt, in directory: URL) throws {
        if receipt.version != currentVersion {
            throw TransactionError.recoveryRequired
        }
        try ensureDirectory(directory)
        let data = try JSONEncoder().encode(receipt)
        let url = directory.appendingPathComponent(receipt.fileName)
        let temporary = url.appendingPathExtension("tmp")
        try data.write(to: temporary, options: .atomic)
        let fd = try DirectoryFD.openDirectory(at: directory)
        defer { DirectoryFD.close(fd) }
        if FileManager.default.fileExists(atPath: url.path) {
            try FileManager.default.removeItem(at: url)
        }
        try FileManager.default.moveItem(at: temporary, to: url)
        try DirectoryFD.fsync(fd)
    }

    static func remove(_ receipt: TransactionReceipt, in directory: URL) throws {
        let url = directory.appendingPathComponent(receipt.fileName)
        if FileManager.default.fileExists(atPath: url.path) {
            try FileManager.default.removeItem(at: url)
        }
        if FileManager.default.fileExists(atPath: directory.path) {
            let fd = try DirectoryFD.openDirectory(at: directory)
            defer { DirectoryFD.close(fd) }
            try DirectoryFD.fsync(fd)
        }
    }

    static func loadAll(in directory: URL) throws -> [TransactionReceipt] {
        guard FileManager.default.fileExists(atPath: directory.path) else {
            return []
        }
        let items = try FileManager.default.contentsOfDirectory(at: directory, includingPropertiesForKeys: nil)
        let receiptURLs = items.filter { $0.pathExtension == "receipt" }
        if receiptURLs.count > maxReceipts {
            throw TransactionError.recoveryRequired
        }
        return try receiptURLs.map { url in
            let data = try Data(contentsOf: url)
            let receipt = try JSONDecoder().decode(TransactionReceipt.self, from: data)
            if receipt.version != currentVersion {
                throw TransactionError.recoveryRequired
            }
            if !GeneratedNames.isGeneratedStage(receipt.stageName) {
                throw TransactionError.recoveryRequired
            }
            if let backup = receipt.backupName, !GeneratedNames.isGeneratedBackup(backup) {
                throw TransactionError.recoveryRequired
            }
            return receipt
        }
    }
}
