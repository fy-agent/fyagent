import Darwin
import Foundation

let cases: [(String, () throws -> Void)] = [
    ("protocol.rejectsUnknownProduct", ProtocolTests.rejectsUnknownProduct),
    ("protocol.rejectsUnknownSlot", ProtocolTests.rejectsUnknownSlotBeforeMutationSemantics),
    ("protocol.rejectsUnknownAction", ProtocolTests.rejectsUnknownAction),
    ("protocol.rejectsNonzeroReserved", ProtocolTests.rejectsNonzeroReserved),
    ("protocol.rejectsExtraFields", ProtocolTests.rejectsExtraFields),
    ("protocol.rejectsUnknownOperation", ProtocolTests.rejectsUnknownOperation),
    ("protocol.codexExistingOnlySlot", ProtocolTests.codexExistingOnlySlotCannotFreshInstall),
    ("protocol.codingKeysForbidden", ProtocolTests.codingKeysContainNoPathUrlOrCommand),
    ("protocol.sourceScanForbiddenKeys", ProtocolTests.protocolSourcesContainNoPathUrlCommandKeys),
    ("protocol.roundTrip", ProtocolTests.validCommitFieldsRoundTrip),
    ("transaction.freshInstall", TransactionTests.testFreshInstallToInjectedParent),
    ("transaction.updateExactSlot", TransactionTests.testUpdateExactSlot),
    ("transaction.verificationFailureRestoresBackup", TransactionTests.testVerificationFailureRestoresBackup),
    ("transaction.unknownSlotBeforeWrite", TransactionTests.testUnknownProductRejectedBeforeWrite),
    ("transaction.codexFreshSlotRejected", TransactionTests.testCodexAppSlotCannotBeUsedForFreshInstall),
    ("transaction.fileFDRejected", TransactionTests.testFDPointingAtFileIsRejected),
    ("transaction.symlinkFDRejected", TransactionTests.testFDPointingAtSymlinkIsRejected),
    ("transaction.wrongBundleIdRejected", TransactionTests.testWrongBundleIdIsRejected),
    ("transaction.tocTOU", TransactionTests.testTOCTOUPathReplacementDoesNotChangeOpenedFD),
    ("recovery.preparing", RecoveryTests.testPreparingCleansStageLeavesTarget),
    ("recovery.readyToCommit", RecoveryTests.testReadyToCommitCleansStageLeavesTarget),
    ("recovery.backupCreated", RecoveryTests.testBackupCreatedRestoresBackupWhenTargetAbsent),
    ("recovery.replacementCommitted", RecoveryTests.testReplacementCommittedKeepsValidTarget),
    ("abi.sizes", BridgeABITests.requestAndReplySizesAreStable),
    ("abi.nullPointers", BridgeABITests.nullPointersFailClosed),
    ("abi.wrongVersion", BridgeABITests.wrongAbiVersionFailsClosed),
    ("abi.reserved", BridgeABITests.nonzeroReservedIsRejected),
    ("abi.unknownOperation", BridgeABITests.unknownOperationIsRejected),
    ("abi.statusMissing", BridgeABITests.statusWithoutHelperReportsMissing),
    ("abi.ensureMissing", BridgeABITests.ensureHelperWithoutPackagingReportsMissing),
    ("scan.helperForbiddenSurface", ForbiddenSurfaceTests.helperSourcesForbidProcessNetworkAndShell),
    ("scan.headerHasNoAuthBytes", ForbiddenSurfaceTests.clientDoesNotPutAuthorizationBytesInCABI),
    ("scan.helperSources", HelperSourceScanTests.helperMainAndServerHaveNoGenericRoutes),
]
for (name, body) in cases {
    TestRuntime.run(name, body)
}
if TestRuntime.failures > 0 {
    fputs("\(TestRuntime.failures) failed / \(TestRuntime.ran) ran\n", stderr)
    exit(1)
}
fputs("All \(TestRuntime.ran) tests passed\n", stdout)
