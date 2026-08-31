import Darwin
import Foundation
import FyAgentPrivilegedProtocol
@testable import FyAgentPrivilegedTransaction

struct TransactionTests {
    static func testFreshInstallToInjectedParent() throws {
        let env = try makeEnv()
        defer { try? FileManager.default.removeItem(at: env.root) }
        let source = try FakeApp.make(
            in: env.root.appendingPathComponent("source"),
            basename: "OpenCode.app",
            bundleId: "ai.opencode.desktop",
            version: "1.2.3",
            executable: "OpenCode"
        )
        let identity = try readIdentity(source, product: .openCodeDesktop)
        let fd = try openDir(source)
        defer { _ = Darwin.close(fd) }
        let result = try SystemCommit.commit(
            CommitRequest(
                operationId: UUID(),
                action: .freshInstall,
                product: .openCodeDesktop,
                targetSlot: 1,
                expectedSourceRevision: identity.revision,
                expectedTargetRevision: Data(repeating: 0, count: 32),
                sourceDirectoryFD: fd
            ),
            environment: env.transaction
        )
        expect(result.outcome == .committed)
        let installed = env.apps.appendingPathComponent("OpenCode.app")
        expect(FileManager.default.fileExists(atPath: installed.path))
        let installedIdentity = try readIdentity(installed, product: .openCodeDesktop)
        expect(installedIdentity.revision == identity.revision)
    }

    static func testUpdateExactSlot() throws {
        let env = try makeEnv()
        defer { try? FileManager.default.removeItem(at: env.root) }
        let existing = try FakeApp.make(
            in: env.apps,
            basename: "OpenCode.app",
            bundleId: "ai.opencode.desktop",
            version: "1.0.0",
            executable: "OpenCode"
        )
        let oldIdentity = try readIdentity(existing, product: .openCodeDesktop)
        let source = try FakeApp.make(
            in: env.root.appendingPathComponent("source"),
            basename: "OpenCode.app",
            bundleId: "ai.opencode.desktop",
            version: "2.0.0",
            executable: "OpenCode"
        )
        let newIdentity = try readIdentity(source, product: .openCodeDesktop)
        let fd = try openDir(source)
        defer { _ = Darwin.close(fd) }
        let result = try SystemCommit.commit(
            CommitRequest(
                operationId: UUID(),
                action: .updateExisting,
                product: .openCodeDesktop,
                targetSlot: 1,
                expectedSourceRevision: newIdentity.revision,
                expectedTargetRevision: oldIdentity.revision,
                sourceDirectoryFD: fd
            ),
            environment: env.transaction
        )
        expect(result.outcome == .committed)
        let installed = try readIdentity(existing, product: .openCodeDesktop)
        expect(installed.version == "2.0.0")
        let leftovers = try FileManager.default.contentsOfDirectory(atPath: env.apps.path)
            .filter { $0.hasPrefix(".fyagent-system-") }
        expect(leftovers == [])
    }

    static func testVerificationFailureRestoresBackup() throws {
        let env = try makeEnv()
        defer { try? FileManager.default.removeItem(at: env.root) }
        let existing = try FakeApp.make(
            in: env.apps,
            basename: "WorkBuddy.app",
            bundleId: "com.workbuddy.workbuddy",
            version: "5.0.0",
            executable: "WorkBuddy"
        )
        let oldIdentity = try readIdentity(existing, product: .workBuddy)
        let source = try FakeApp.make(
            in: env.root.appendingPathComponent("source"),
            basename: "WorkBuddy.app",
            bundleId: "com.workbuddy.workbuddy",
            version: "5.1.0",
            executable: "WorkBuddy"
        )
        let newIdentity = try readIdentity(source, product: .workBuddy)
        let fd = try openDir(source)
        defer { _ = Darwin.close(fd) }
        let result = try SystemCommit.commit(
            CommitRequest(
                operationId: UUID(),
                action: .updateExisting,
                product: .workBuddy,
                targetSlot: 1,
                expectedSourceRevision: newIdentity.revision,
                expectedTargetRevision: oldIdentity.revision,
                sourceDirectoryFD: fd
            ),
            environment: env.transaction,
            hooks: TransactionHooks(forceVerificationFailure: true)
        )
        expect(result.outcome == .rollbackRestored)
        let restored = try readIdentity(existing, product: .workBuddy)
        expect(restored.version == "5.0.0")
    }

    static func testUnknownProductRejectedBeforeWrite() throws {
        let env = try makeEnv()
        defer { try? FileManager.default.removeItem(at: env.root) }
        let source = try FakeApp.make(
            in: env.root.appendingPathComponent("source"),
            basename: "OpenCode.app",
            bundleId: "ai.opencode.desktop",
            version: "1.0.0",
            executable: "OpenCode"
        )
        let fd = try openDir(source)
        defer { _ = Darwin.close(fd) }
        expectThrowsAny {
            _ = try SystemCommit.commit(
                CommitRequest(
                    operationId: UUID(),
                    action: .freshInstall,
                    product: KnownProduct(rawValue: 1)!,
                    targetSlot: 99,
                    expectedSourceRevision: Data(repeating: 1, count: 32),
                    expectedTargetRevision: Data(repeating: 0, count: 32),
                    sourceDirectoryFD: fd
                ),
                environment: env.transaction
            )
        }
        let leftoverApps = try FileManager.default.contentsOfDirectory(atPath: env.apps.path)
            .filter { $0.hasSuffix(".app") }
        expect(leftoverApps.isEmpty)
    }

    static func testCodexAppSlotCannotBeUsedForFreshInstall() throws {
        let env = try makeEnv()
        defer { try? FileManager.default.removeItem(at: env.root) }
        let source = try FakeApp.make(
            in: env.root.appendingPathComponent("source"),
            basename: "ChatGPT.app",
            bundleId: "com.openai.codex",
            version: "1.0.0",
            executable: "ChatGPT"
        )
        let identity = try readIdentity(source, product: .codexDesktop)
        let fd = try openDir(source)
        defer { _ = Darwin.close(fd) }
        expectThrows(TransactionError.targetSlotInvalid) {
            _ = try SystemCommit.commit(
                CommitRequest(
                    operationId: UUID(),
                    action: .freshInstall,
                    product: .codexDesktop,
                    targetSlot: 2,
                    expectedSourceRevision: identity.revision,
                    expectedTargetRevision: Data(repeating: 0, count: 32),
                    sourceDirectoryFD: fd
                ),
                environment: env.transaction
            )
        }
        expect(!(FileManager.default.fileExists(atPath: env.apps.appendingPathComponent("Codex.app").path)))
        expect(!(FileManager.default.fileExists(atPath: env.apps.appendingPathComponent("ChatGPT.app").path)))
    }

    static func testFDPointingAtFileIsRejected() throws {
        let env = try makeEnv()
        defer { try? FileManager.default.removeItem(at: env.root) }
        let file = env.root.appendingPathComponent("not-a-bundle")
        try Data("nope".utf8).write(to: file)
        let fd = file.path.withCString { open($0, O_RDONLY | O_CLOEXEC) }
        expect(fd >= 0)
        defer { _ = Darwin.close(fd) }
        expectThrowsAny {
            _ = try SystemCommit.commit(
                CommitRequest(
                    operationId: UUID(),
                    action: .freshInstall,
                    product: .openCodeDesktop,
                    targetSlot: 1,
                    expectedSourceRevision: Data(repeating: 1, count: 32),
                    expectedTargetRevision: Data(repeating: 0, count: 32),
                    sourceDirectoryFD: fd
                ),
                environment: env.transaction
            )
        }
    }

    static func testFDPointingAtSymlinkIsRejected() throws {
        let env = try makeEnv()
        defer { try? FileManager.default.removeItem(at: env.root) }
        let real = try FakeApp.make(
            in: env.root.appendingPathComponent("real"),
            basename: "OpenCode.app",
            bundleId: "ai.opencode.desktop",
            version: "1.0.0",
            executable: "OpenCode"
        )
        let link = env.root.appendingPathComponent("link.app")
        try FileManager.default.createSymbolicLink(at: link, withDestinationURL: real)
        let fd = link.path.withCString { open($0, O_RDONLY | O_SYMLINK | O_CLOEXEC) }
        expect(fd >= 0)
        defer { _ = Darwin.close(fd) }
        expectThrowsAny {
            _ = try SystemCommit.commit(
                CommitRequest(
                    operationId: UUID(),
                    action: .freshInstall,
                    product: .openCodeDesktop,
                    targetSlot: 1,
                    expectedSourceRevision: Data(repeating: 1, count: 32),
                    expectedTargetRevision: Data(repeating: 0, count: 32),
                    sourceDirectoryFD: fd
                ),
                environment: env.transaction
            )
        }
    }

    static func testWrongBundleIdIsRejected() throws {
        let env = try makeEnv()
        defer { try? FileManager.default.removeItem(at: env.root) }
        let source = try FakeApp.make(
            in: env.root.appendingPathComponent("source"),
            basename: "OpenCode.app",
            bundleId: "com.example.wrong",
            version: "1.0.0",
            executable: "OpenCode"
        )
        let fd = try openDir(source)
        defer { _ = Darwin.close(fd) }
        expectThrowsAny {
            _ = try SystemCommit.commit(
                CommitRequest(
                    operationId: UUID(),
                    action: .freshInstall,
                    product: .openCodeDesktop,
                    targetSlot: 1,
                    expectedSourceRevision: Data(repeating: 1, count: 32),
                    expectedTargetRevision: Data(repeating: 0, count: 32),
                    sourceDirectoryFD: fd
                ),
                environment: env.transaction
            )
        }
        expect(!(FileManager.default.fileExists(atPath: env.apps.appendingPathComponent("OpenCode.app").path)))
    }

    static func testTOCTOUPathReplacementDoesNotChangeOpenedFD() throws {
        let env = try makeEnv()
        defer { try? FileManager.default.removeItem(at: env.root) }
        let original = try FakeApp.make(
            in: env.root.appendingPathComponent("original"),
            basename: "OpenCode.app",
            bundleId: "ai.opencode.desktop",
            version: "9.9.9",
            executable: "OpenCode"
        )
        let livePath = env.root.appendingPathComponent("live.app")
        try FileManager.default.copyItem(at: original, to: livePath)
        let fd = try openDir(livePath)
        defer { _ = Darwin.close(fd) }
        try FileManager.default.moveItem(at: livePath, to: env.root.appendingPathComponent("aside.app"))
        _ = try FakeApp.make(
            in: env.root,
            basename: "live.app",
            bundleId: "com.example.attacker",
            version: "0.0.1",
            executable: "OpenCode"
        )
        let identity = try BundleIdentityReader.read(
            fromBundleFD: fd,
            policy: KnownApplicationPolicyTable.policy(for: .openCodeDesktop)
        )
        expect(identity.bundleIdentifier == "ai.opencode.desktop")
        expect(identity.version == "9.9.9")
        let result = try SystemCommit.commit(
            CommitRequest(
                operationId: UUID(),
                action: .freshInstall,
                product: .openCodeDesktop,
                targetSlot: 1,
                expectedSourceRevision: identity.revision,
                expectedTargetRevision: Data(repeating: 0, count: 32),
                sourceDirectoryFD: fd
            ),
            environment: env.transaction
        )
        expect(result.outcome == .committed)
        let installed = try readIdentity(env.apps.appendingPathComponent("OpenCode.app"), product: .openCodeDesktop)
        expect(installed.bundleIdentifier == "ai.opencode.desktop")
        expect(installed.version == "9.9.9")
    }
}

struct RecoveryTests {
    static func testPreparingCleansStageLeavesTarget() throws {
        try runPhase(.preparing, expectTargetVersion: "1.0.0", expectStageGone: true)
    }

    static func testReadyToCommitCleansStageLeavesTarget() throws {
        try runPhase(.readyToCommit, expectTargetVersion: "1.0.0", expectStageGone: true)
    }

    static func testBackupCreatedRestoresBackupWhenTargetAbsent() throws {
        let env = try makeEnv()
        defer { try? FileManager.default.removeItem(at: env.root) }
        let operationId = UUID()
        let old = try FakeApp.make(
            in: env.apps,
            basename: GeneratedNames.backupName(for: operationId),
            bundleId: "ai.opencode.desktop",
            version: "1.0.0",
            executable: "OpenCode"
        )
        let oldIdentity = try readIdentity(old, product: .openCodeDesktop)
        let stage = try FakeApp.make(
            in: env.apps,
            basename: GeneratedNames.stageName(for: operationId),
            bundleId: "ai.opencode.desktop",
            version: "2.0.0",
            executable: "OpenCode"
        )
        let stageIdentity = try readIdentity(stage, product: .openCodeDesktop)
        try writeReceipt(
            env: env,
            operationId: operationId,
            phase: .backupCreated,
            source: stageIdentity.revision,
            target: oldIdentity.revision
        )
        let status = try SystemCommit.recover(environment: env.transaction)
        expect(status == .recovered)
        expect(FileManager.default.fileExists(atPath: env.apps.appendingPathComponent("OpenCode.app").path))
        expect(try readIdentity(env.apps.appendingPathComponent("OpenCode.app"), product: .openCodeDesktop).version == "1.0.0")
        expect(!(FileManager.default.fileExists(atPath: stage.path)))
    }

    static func testReplacementCommittedKeepsValidTarget() throws {
        let env = try makeEnv()
        defer { try? FileManager.default.removeItem(at: env.root) }
        let operationId = UUID()
        let installed = try FakeApp.make(
            in: env.apps,
            basename: "OpenCode.app",
            bundleId: "ai.opencode.desktop",
            version: "2.0.0",
            executable: "OpenCode"
        )
        let newIdentity = try readIdentity(installed, product: .openCodeDesktop)
        let backup = try FakeApp.make(
            in: env.apps,
            basename: GeneratedNames.backupName(for: operationId),
            bundleId: "ai.opencode.desktop",
            version: "1.0.0",
            executable: "OpenCode"
        )
        let oldIdentity = try readIdentity(backup, product: .openCodeDesktop)
        try writeReceipt(
            env: env,
            operationId: operationId,
            phase: .replacementCommitted,
            source: newIdentity.revision,
            target: oldIdentity.revision
        )
        let status = try SystemCommit.recover(environment: env.transaction)
        expect(status == .recovered)
        expect(try readIdentity(installed, product: .openCodeDesktop).version == "2.0.0")
        expect(!(FileManager.default.fileExists(atPath: backup.path)))
    }

    static func runPhase(_ phase: ReceiptPhase, expectTargetVersion: String, expectStageGone: Bool) throws {
        let env = try makeEnv()
        defer { try? FileManager.default.removeItem(at: env.root) }
        let operationId = UUID()
        let target = try FakeApp.make(
            in: env.apps,
            basename: "OpenCode.app",
            bundleId: "ai.opencode.desktop",
            version: "1.0.0",
            executable: "OpenCode"
        )
        let stage = try FakeApp.make(
            in: env.apps,
            basename: GeneratedNames.stageName(for: operationId),
            bundleId: "ai.opencode.desktop",
            version: "2.0.0",
            executable: "OpenCode"
        )
        let sourceIdentity = try readIdentity(stage, product: .openCodeDesktop)
        try writeReceipt(
            env: env,
            operationId: operationId,
            phase: phase,
            source: sourceIdentity.revision,
            target: nil
        )
        let status = try SystemCommit.recover(environment: env.transaction)
        expect(status == .recovered)
        expect(try readIdentity(target, product: .openCodeDesktop).version == expectTargetVersion)
        expect(FileManager.default.fileExists(atPath: stage.path) == !expectStageGone)
    }
}

struct TestEnv {
    var root: URL
    var apps: URL
    var receipts: URL
    var transaction: TransactionEnvironment
}

func makeEnv() throws -> TestEnv {
    let root = FileManager.default.temporaryDirectory.appendingPathComponent("fyagent-helper-\(UUID().uuidString)", isDirectory: true)
    let apps = root.appendingPathComponent("Applications", isDirectory: true)
    let receipts = root.appendingPathComponent("receipts", isDirectory: true)
    try FileManager.default.createDirectory(at: apps, withIntermediateDirectories: true)
    try FileManager.default.createDirectory(at: receipts, withIntermediateDirectories: true)
    return TestEnv(
        root: root,
        apps: apps,
        receipts: receipts,
        transaction: .testing(applicationsParent: apps, receiptDirectory: receipts)
    )
}

func openDir(_ url: URL) throws -> Int32 {
    let fd = url.path.withCString { open($0, O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC) }
    if fd < 0 {
        throw NSError(domain: NSPOSIXErrorDomain, code: Int(errno))
    }
    return fd
}

func readIdentity(_ url: URL, product: KnownProduct) throws -> BundleIdentity {
    let fd = try openDir(url)
    defer { _ = Darwin.close(fd) }
    return try BundleIdentityReader.read(fromBundleFD: fd, policy: KnownApplicationPolicyTable.policy(for: product))
}

func writeReceipt(
    env: TestEnv,
    operationId: UUID,
    phase: ReceiptPhase,
    source: Data,
    target: Data?
) throws {
    let receipt = TransactionReceipt(
        operationId: operationId,
        product: KnownProduct.openCodeDesktop.rawValue,
        targetSlot: 1,
        phase: phase,
        stageName: GeneratedNames.stageName(for: operationId),
        backupName: phase == .preparing || phase == .readyToCommit ? nil : GeneratedNames.backupName(for: operationId),
        sourceRevision: source,
        targetRevision: target
    )
    try ReceiptStore.write(receipt, in: env.receipts)
}

enum FakeApp {
    static func make(
        in directory: URL,
        basename: String,
        bundleId: String,
        version: String,
        executable: String
    ) throws -> URL {
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        let app = directory.appendingPathComponent(basename)
        let contents = app.appendingPathComponent("Contents")
        let macos = contents.appendingPathComponent("MacOS")
        try FileManager.default.createDirectory(at: macos, withIntermediateDirectories: true)
        let plist = """
        <?xml version="1.0" encoding="UTF-8"?>
        <!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
        <plist version="1.0">
        <dict>
            <key>CFBundleIdentifier</key>
            <string>\(bundleId)</string>
            <key>CFBundleName</key>
            <string>\(executable)</string>
            <key>CFBundleExecutable</key>
            <string>\(executable)</string>
            <key>CFBundleShortVersionString</key>
            <string>\(version)</string>
            <key>CFBundleVersion</key>
            <string>\(version)</string>
        </dict>
        </plist>
        """
        try plist.data(using: .utf8)!.write(to: contents.appendingPathComponent("Info.plist"))
        try Data("exe".utf8).write(to: macos.appendingPathComponent(executable))
        if bundleId == "cn.trae.solo.app" {
            let appRes = contents.appendingPathComponent("Resources/app")
            try FileManager.default.createDirectory(at: appRes, withIntermediateDirectories: true)
            let json = "{\"tronBuildVersion\":\"\(version)\"}"
            try json.data(using: .utf8)!.write(to: appRes.appendingPathComponent("product.json"))
        }
        return app
    }
}
