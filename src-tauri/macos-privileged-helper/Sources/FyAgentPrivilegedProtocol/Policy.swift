import Foundation

public struct TargetSlotPolicy: Equatable {
    public let slot: UInt32
    public let basename: String
    public let allowsFreshInstall: Bool
}

public struct KnownApplicationPolicy: Equatable {
    public let product: KnownProduct
    public let bundleIdentifier: String
    public let versionSource: VersionSource
    public let slots: [TargetSlotPolicy]

    public func slot(_ slot: UInt32) throws -> TargetSlotPolicy {
        guard let match = slots.first(where: { $0.slot == slot }) else {
            throw ProtocolError.targetSlotInvalid
        }
        return match
    }

    public func resolve(slot: UInt32, action: CommitAction) throws -> TargetSlotPolicy {
        let policy = try self.slot(slot)
        if action == .freshInstall && !policy.allowsFreshInstall {
            throw ProtocolError.targetSlotInvalid
        }
        if action == .none {
            throw ProtocolError.unknownAction
        }
        return policy
    }
}

public enum KnownApplicationPolicyTable {
    public static let all: [KnownApplicationPolicy] = [
        KnownApplicationPolicy(
            product: .codexDesktop,
            bundleIdentifier: "com.openai.codex",
            versionSource: .infoPlist,
            slots: [
                TargetSlotPolicy(slot: 1, basename: "ChatGPT.app", allowsFreshInstall: true),
                TargetSlotPolicy(slot: 2, basename: "Codex.app", allowsFreshInstall: false),
            ]
        ),
        KnownApplicationPolicy(
            product: .openCodeDesktop,
            bundleIdentifier: "ai.opencode.desktop",
            versionSource: .infoPlist,
            slots: [
                TargetSlotPolicy(slot: 1, basename: "OpenCode.app", allowsFreshInstall: true),
            ]
        ),
        KnownApplicationPolicy(
            product: .qoderWork,
            bundleIdentifier: "com.qoder.work.cn",
            versionSource: .infoPlist,
            slots: [
                TargetSlotPolicy(slot: 1, basename: "QoderWork CN.app", allowsFreshInstall: true),
            ]
        ),
        KnownApplicationPolicy(
            product: .traeWork,
            bundleIdentifier: "cn.trae.solo.app",
            versionSource: .traeProductJSON,
            slots: [
                TargetSlotPolicy(slot: 1, basename: "TRAE SOLO CN.app", allowsFreshInstall: true),
            ]
        ),
        KnownApplicationPolicy(
            product: .workBuddy,
            bundleIdentifier: "com.workbuddy.workbuddy",
            versionSource: .infoPlist,
            slots: [
                TargetSlotPolicy(slot: 1, basename: "WorkBuddy.app", allowsFreshInstall: true),
            ]
        ),
    ]

    public static func policy(for product: KnownProduct) -> KnownApplicationPolicy {
        all.first { $0.product == product }!
    }

    public static func resolve(
        product: KnownProduct,
        slot: UInt32,
        action: CommitAction
    ) throws -> (KnownApplicationPolicy, TargetSlotPolicy) {
        let policy = policy(for: product)
        let slotPolicy = try policy.resolve(slot: slot, action: action)
        return (policy, slotPolicy)
    }
}
