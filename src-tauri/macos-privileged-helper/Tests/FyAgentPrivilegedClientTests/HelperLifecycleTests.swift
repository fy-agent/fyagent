import FyAgentPrivilegedProtocol
@testable import FyAgentPrivilegedClient

enum HelperLifecycleTests {
    static func bundleVersionParsingIsStrictAndComparable() {
        expect(HelperBundleVersion("0.4.2") != nil)
        expect(HelperBundleVersion("350.56.31") != nil)
        expect(HelperBundleVersion("20260831.100245") == nil)
        expect(HelperBundleVersion("1.100") == nil)
        expect(HelperBundleVersion("1.2.3.4") == nil)
        expect(HelperBundleVersion("1.beta") == nil)
        expect(HelperBundleVersion("1.2")! == HelperBundleVersion("1.2.0")!)
        expect(HelperBundleVersion("1.2.1")! > HelperBundleVersion("1.2")!)
    }

    static func readyInstalledHelperAvoidsEqualVersionBless() {
        let status = HelperStatusReply(
            helperVersion: "350.56.31",
            minimumClientVersion: "0.4.2",
            state: .ready
        )
        let decision = HelperLifecycle.decide(
            status: status,
            bundledVersion: HelperBundleVersion("350.56.31")!,
            clientVersion: HelperBundleVersion("0.4.2")!
        )
        expect(decision == .ready)
    }

    static func olderInstalledHelperRequiresOneUpgrade() {
        let status = HelperStatusReply(
            helperVersion: "350.56.30",
            minimumClientVersion: "0.4.2",
            state: .ready
        )
        let decision = HelperLifecycle.decide(
            status: status,
            bundledVersion: HelperBundleVersion("350.56.31")!,
            clientVersion: HelperBundleVersion("0.4.2")!
        )
        expect(decision == .installOrUpdate)
    }

    static func newerIncompatibleHelperCannotBeDowngraded() {
        let status = HelperStatusReply(
            protocolVersion: 2,
            helperVersion: "350.56.32",
            minimumClientVersion: "0.4.2",
            state: .incompatible,
            reason: .helperProtocolIncompatible
        )
        let decision = HelperLifecycle.decide(
            status: status,
            bundledVersion: HelperBundleVersion("350.56.31")!,
            clientVersion: HelperBundleVersion("0.4.2")!
        )
        expect(
            decision == .failed(
                state: .incompatible,
                reason: .helperDowngradeRejected
            )
        )
    }

    static func recoveryAndMinimumClientVersionFailClosed() {
        let recovery = HelperLifecycle.decide(
            status: HelperStatusReply(
                helperVersion: "350.56.31",
                minimumClientVersion: "0.4.2",
                state: .recoveryRequired,
                reason: .recoveryRequired,
                activeRecovery: true
            ),
            bundledVersion: HelperBundleVersion("350.56.31")!,
            clientVersion: HelperBundleVersion("0.4.2")!
        )
        expect(
            recovery == .failed(
                state: .recoveryRequired,
                reason: .recoveryRequired
            )
        )

        let clientTooOld = HelperLifecycle.decide(
            status: HelperStatusReply(
                helperVersion: "350.56.31",
                minimumClientVersion: "0.5.0",
                state: .ready
            ),
            bundledVersion: HelperBundleVersion("350.56.31")!,
            clientVersion: HelperBundleVersion("0.4.2")!
        )
        expect(
            clientTooOld == .failed(
                state: .incompatible,
                reason: .helperProtocolIncompatible
            )
        )
    }

    static func malformedOrInconsistentStatusFailsClosed() {
        let malformed = HelperLifecycle.decide(
            status: HelperStatusReply(
                helperVersion: "20260831.100245",
                minimumClientVersion: "0.4.2",
                state: .ready
            ),
            bundledVersion: HelperBundleVersion("350.56.31")!,
            clientVersion: HelperBundleVersion("0.4.2")!
        )
        expect(
            malformed == .failed(
                state: .incompatible,
                reason: .helperProtocolIncompatible
            )
        )

        let inconsistentRecovery = HelperLifecycle.decide(
            status: HelperStatusReply(
                helperVersion: "350.56.31",
                minimumClientVersion: "0.4.2",
                state: .ready,
                reason: .recoveryRequired,
                activeRecovery: false
            ),
            bundledVersion: HelperBundleVersion("350.56.31")!,
            clientVersion: HelperBundleVersion("0.4.2")!
        )
        expect(
            inconsistentRecovery == .failed(
                state: .incompatible,
                reason: .helperProtocolIncompatible
            )
        )
    }
}
