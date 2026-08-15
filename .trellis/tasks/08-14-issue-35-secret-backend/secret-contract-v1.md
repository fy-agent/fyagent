# secret-contract/v1 — design-contract review candidate

Status: `PROPOSED_FOR_REVIEW`. This file is not a freeze receipt and is not consumable by #55 or #41 until the authority documents adopt it and all three reviewers re-read the same immutable SHA.

## 1. Normative boundary

The words MUST, MUST NOT, SHOULD and MAY are normative.

1. The JSON contract version is exactly `secret-contract/v1`. Every #35 renderer command envelope and every versioned top-level aggregate/report/operation DTO has the literal `schemaVersion: 1`, represented in Rust by `SchemaVersionV1` rather than `u8`. Nested value objects inherit that enclosing version and do not add a second version field. The independently owned main-integration resume request body is exactly `{stageId,expectedResumeCas}` and its result data is the exact five-field DTO below; version and command id remain in the common envelope rather than either body.
2. Every Rust wire struct/struct-variant that implements `Deserialize` uses `deny_unknown_fields`, including request-reachable nested objects. Rust enums use closed discriminants. Newtypes use `#[serde(transparent)]` with validating `Deserialize`; deriving an unchecked string deserializer is forbidden. The TypeScript adapter uses generated strict decoders at every object node (no passthrough/catch-all/`Record<string, unknown>`), and never reconstructs a union by spreading an unvalidated base object.
3. Public command success is `SecretCommandSuccess<T>`. Public command failure is `SecretCommandError`. No public command accepts or returns material, a material-derived digest, a backend locator, an OS error, a path, or an arbitrary error message.
4. `SecretOwnerKind.agent` is wire-reserved so a future v2 does not need a discriminant rewrite. v1 runtime accepts only `provider/codex + codexApiKey + primaryApiKey`. Every concrete Agent request MUST return `SECRET_OWNER_KIND_UNSUPPORTED` with `effect=none`.
5. Acceptance scope is `codex_feature_runtime`, not repository-global. The closed call graph is: startup bootstrap/backup gate; Codex Provider persistence/public projection/delete-detach; Codex mutation and staged import/restore/sync/deep-link branches; live backfill and Provider switch/apply; proxy traffic; usage/balance and the fixed primary coding-plan adapter; model fetch; typed rejection of Provider terminal; Codex UniversalProvider/failover branches; Codex export/backup/diagnostic/log surfaces. Existing WebDAV/S3 credentials, OAuth managers, independent ZenMux/AK-SK/login credentials and non-Codex Providers remain reported pre-existing debt. A report MUST NOT claim that the repository is globally secret-free.
6. `SecretRefAggregate` is the stable ref-level object and may contain many bindings. `SecretOwnerCredentialSummary` is the owner-level object and can represent legacy/unbound states that have no ref, backend or creation time.
7. A stable summary MUST NOT contain `confirmationRequired`, an operation id, a confirmation step or a one-shot capability. Those exist only in an operation-scoped readiness/prepare response.
8. Hardware is contract-capable but runtime-hidden unless a real adapter instance is registered. An unregistered hardware adapter MUST NOT appear as an Add/Replace option. An already-bound hardware record remains visible and becomes `unavailable`; it MUST NOT fall back to `osKeyring`.
9. All create/replace/rotate/reconcile captures create a verified staged candidate. A candidate changes no binding, Provider record or live target. Only a consumed, immutable #55 Change Plan may activate it.
10. A one-shot prepared capability contains no `SecretMaterial`. It is bound to the approved plan, operation, owner binding, ref, record revision, binding-set CAS, backend instance/generation, device-binding generation, capability revision, consumer, sink and expiry. `resolve_for_apply` consumes it atomically, revalidates every binding immediately before material acquisition, then invokes exactly one native writer.
11. The device-local authority root is exactly `app_local_data_dir/device-local/secrets/v1`. Records, refs, bindings, owner-binding tombstone revisions, candidates, operation journal, audit and recovery state live only below that root and are excluded from database sync. Provider SQLite rows contain non-secret Provider configuration only; owner summaries join the Provider id to the device-local binding projection at read time.
12. Issue #35 adds no SQLite table, column, trigger or `PRAGMA user_version` transition and does not allocate or share schema version `v17`. Prompt/Memory SQLite schema ownership remains outside #35; every #35 authority object is device-local file state under the root above.

## 2. Strict scalar contract

All numeric revisions are JSON safe positive integers (`1..=9_007_199_254_740_991`). All string parsers reject NUL, control characters and surrounding whitespace.

| Type | Exact wire grammar / provenance |
| --- | --- |
| `SecretRef` | `^sec_[0-9a-f]{12}4[0-9a-f]{3}[89ab][0-9a-f]{15}$`; native-generated UUIDv4 simple form |
| `SecretCandidateId` | `^scd_[0-9a-f]{12}4[0-9a-f]{3}[89ab][0-9a-f]{15}$`; native-generated |
| `SecretOperationId` | `^sop_[0-9a-f]{12}4[0-9a-f]{3}[89ab][0-9a-f]{15}$`; native-generated |
| `SecretCommandId` | `^scm_[0-9a-f]{12}4[0-9a-f]{3}[89ab][0-9a-f]{15}$`; native-generated even for failures |
| `SecretAuditEventId` | `^sae_[0-9a-f]{12}4[0-9a-f]{3}[89ab][0-9a-f]{15}$`; native-generated |
| `SecretConfirmationStepId` | `^scs_[0-9a-f]{12}4[0-9a-f]{3}[89ab][0-9a-f]{15}$`; native-generated |
| `SecretBackendInstanceId` | `^sbi_[0-9a-f]{12}4[0-9a-f]{3}[89ab][0-9a-f]{15}$`; device-local random adapter identity |
| `DeviceInstanceId` | `^dev_[0-9a-f]{12}4[0-9a-f]{3}[89ab][0-9a-f]{15}$`; durable device authority identity, generated once and persisted below the device-local root; distinct from the process-local `DeviceSecretStoreInstanceId` |
| `SecretRecoveryId` | `^src_[0-9a-f]{12}4[0-9a-f]{3}[89ab][0-9a-f]{15}$`; native-generated recovery identity |
| `SecretCaptureIntentId` | `^sci_[0-9a-f]{12}4[0-9a-f]{3}[89ab][0-9a-f]{15}$`; short-lived process-local capture-flow identity minted only by `list_secret_backend_options` |
| `ImportStageId` | `^ist_[0-9a-f]{12}4[0-9a-f]{3}[89ab][0-9a-f]{15}$`; ImportCoordinator-generated staging identity |
| `ProviderDeleteImpactId` | `^pdi_[0-9a-f]{12}4[0-9a-f]{3}[89ab][0-9a-f]{15}$`; main-integration preview registry identity |
| `SecretSummaryCursor` | `^ssc_[0-9a-f]{32}$`; opaque, native-generated, expires |
| `SecretAuditCursor` | `^sac_[0-9a-f]{32}$`; opaque, native-generated, expires |
| `SecretMigrationReportId` | `^smr_[0-9a-f]{12}4[0-9a-f]{3}[89ab][0-9a-f]{15}$` |
| `LegacySourceLocationId` | `^lsl_[0-9a-f]{32}$`; native structural-location identity, never value-derived |
| `ChangePlanId` | canonical lowercase hyphenated UUIDv4 |
| `ChangePlanDigest`, `BindingSetDigest`, `SecretRecoveryDigest`, `SecretProjectionDigest`, `RecoveryStructureDigest`, `StagedImportResumeDigest` | exactly 64 lowercase hexadecimal SHA-256 characters; `RecoveryStructureDigest` covers only credential-free structural rows and `StagedImportResumeDigest` covers the closed operation-bound five-phase internal staged-resume preimage |
| `OwnerId` | `^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$`; an existing stable Provider id, never a name/path/value |
| `SecretOwnerNamespace` | `^[a-z][a-z0-9-]{0,31}$`; v1 accepts `codex` only after kind validation |
| `SafeDisplayText` | 1–80 Unicode scalar values; trim-stable, with no control/newline, absolute path or credential-shaped content; every input/output construction validates |
| `UtcTimestamp` | canonical RFC 3339 UTC with `Z` and millisecond precision |
| revision newtypes | positive JS-safe integer; `SecretRecordRevision`, `SecretStoreRevision` (native only), `SecretCandidateRevision`, `SecretBindingRevision`, `SecretOwnerBindingRevision`, `SecretBindingSetRevision`, `SecretRecoveryRevision`, `StagedImportResumeRevision`, `StagedRowRevision`, `ProviderRowRevision`, `LegacySourceStructuralRevision`, `CodexLiveStructuralRevision`, backend/device/capability revisions are all distinct types |
| `ConfirmationTimeoutSeconds` | integer `1..=300`; backend-declared and native-clamped |
| `SchemaVersionV1` / `SecretContractVersionV1` | exact literals `1` / `secret-contract/v1`; other values are `SECRET_REQUEST_INVALID` |

`secretRefDisplay` is response-only and derived as `sec_…` plus the final four ref characters. It is never accepted as identity.

All nominally credential-free strings use the same `credential_shaped` rejection set. `SafeDisplayText`, `OwnerId`, `CodexModelId` and `CodexModelProviderId` reject credential prefixes/assignments anywhere allowed by their grammar. `ValidatedUrl` additionally rejects userinfo/query/fragment, percent-encoded paths, non-ASCII/control/trim drift, paths over 512 bytes, path characters outside `[A-Za-z0-9/._~-]` and credential-shaped path segments. A type name never overrides this validation.

Wire `OwnerId` is only a lookup key. A renderer can submit its syntax but cannot mint existence: before any #35 operation, `crate::database::dao::providers` must resolve the complete `SecretOwner` to a private `ExistingSecretOwnerToken` or return `SECRET_OWNER_NOT_FOUND/effect=none`. Server-generated refs/candidate/operation/recovery/cursor ids likewise require their issuing authority/registry row when accepted back; syntax-valid fabricated ids never create authority.

## 3. Complete TypeScript wire contract

The following block is the renderer/consumer contract. Branded values are created only by validators in the data adapter.

```ts
export type Brand<T, Name extends string> = T & { readonly __brand: Name };

export type SecretRef = Brand<string, "SecretRef">;
export type SecretRefDisplay = Brand<string, "SecretRefDisplay">;
export type SecretCandidateId = Brand<string, "SecretCandidateId">;
export type SecretOperationId = Brand<string, "SecretOperationId">;
export type SecretCommandId = Brand<string, "SecretCommandId">;
export type SecretAuditEventId = Brand<string, "SecretAuditEventId">;
export type SecretConfirmationStepId = Brand<string, "SecretConfirmationStepId">;
export type SecretBackendInstanceId = Brand<string, "SecretBackendInstanceId">;
export type DeviceInstanceId = Brand<string, "DeviceInstanceId">;
export type SecretRecoveryId = Brand<string, "SecretRecoveryId">;
export type SecretCaptureIntentId = Brand<string, "SecretCaptureIntentId">;
export type ImportStageId = Brand<string, "ImportStageId">;
export type ProviderDeleteImpactId = Brand<string, "ProviderDeleteImpactId">;
export type SecretSummaryCursor = Brand<string, "SecretSummaryCursor">;
export type SecretAuditCursor = Brand<string, "SecretAuditCursor">;
export type SecretMigrationReportId = Brand<string, "SecretMigrationReportId">;
export type LegacySourceLocationId = Brand<string, "LegacySourceLocationId">;
export type ChangePlanId = Brand<string, "ChangePlanId">;
export type ChangePlanDigest = Brand<string, "ChangePlanDigest">;
export type BindingSetDigest = Brand<string, "BindingSetDigest">;
export type SecretRecoveryDigest = Brand<string, "SecretRecoveryDigest">;
export type SecretProjectionDigest = Brand<string, "SecretProjectionDigest">;
export type RecoveryStructureDigest = Brand<string, "RecoveryStructureDigest">;
export type StagedImportResumeDigest = Brand<string, "StagedImportResumeDigest">;
export type OwnerId = Brand<string, "OwnerId">;
export type SafeDisplayText = Brand<string, "SafeDisplayText">;
export type ValidatedUrl = Brand<string, "ValidatedUrl">;
export type CodexModelId = Brand<string, "CodexModelId">;
export type CodexModelProviderId = Brand<string, "CodexModelProviderId">;
export type UtcTimestamp = Brand<string, "UtcTimestamp">;
export type PageLimit = Brand<number, "PageLimit">;
export type SecretRecordRevision = Brand<number, "SecretRecordRevision">;
export type SecretCandidateRevision = Brand<number, "SecretCandidateRevision">;
export type SecretBindingRevision = Brand<number, "SecretBindingRevision">;
export type SecretOwnerBindingRevision = Brand<number, "SecretOwnerBindingRevision">;
export type SecretBindingSetRevision = Brand<number, "SecretBindingSetRevision">;
export type SecretRecoveryRevision = Brand<number, "SecretRecoveryRevision">;
export type StagedImportResumeRevision = Brand<number, "StagedImportResumeRevision">;
export type StagedRowRevision = Brand<number, "StagedRowRevision">;
export type ProviderRowRevision = Brand<number, "ProviderRowRevision">;
export type LegacySourceStructuralRevision =
  Brand<number, "LegacySourceStructuralRevision">;
export type CodexLiveStructuralRevision =
  Brand<number, "CodexLiveStructuralRevision">;
export type SecretBackendGeneration = Brand<number, "SecretBackendGeneration">;
export type DeviceBindingGeneration = Brand<number, "DeviceBindingGeneration">;
export type CapabilityRevision = Brand<number, "CapabilityRevision">;
export type ConfirmationTimeoutSeconds = Brand<number, "ConfirmationTimeoutSeconds">;
export type SchemaVersionV1 = 1;
export type SecretContractVersionV1 = "secret-contract/v1";

const FORBIDDEN_SEMANTIC_FIELDS_V1: ReadonlySet<string> = new Set([
  "secret", "secretvalue", "value", "apikey", "openaiapikey",
  "experimentalbearertoken", "token", "accesstoken", "refreshtoken",
  "authorization", "accesskey", "secretkey", "password", "credential",
  "privatekey", "credentialblob", "backendlocator", "rawerror",
  "rawmessage", "rawconfig", "providersettings", "livesettings",
  "absolutepath", "materialdigest",
]);

const CREDENTIAL_PREFIX_MARKERS_V1 = [
  "sk-", "ghp_", "github_pat_", "glpat-", "akia", "aiza", "ya29.",
  "npm_", "pypi-", "hf_", "xoxb-", "xoxp-", "xoxa-", "eyj",
  "bearer ", "bearer%20",
] as const;

// This exact table is mirrored by Rust. It deliberately does not use `\s`:
// U+0009 TAB, U+000A LF, U+000B VT, U+000C FF, U+000D CR, U+0020 SPACE,
// # & , . / : ; = ? @ \, U+00A0 NBSP and U+2003 EM SPACE.
const CREDENTIAL_SEPARATOR_CODE_POINTS_V1: readonly number[] = [
  0x0009, 0x000a, 0x000b, 0x000c, 0x000d, 0x0020,
  0x0023, 0x0026, 0x002c, 0x002e, 0x002f, 0x003a,
  0x003b, 0x003d, 0x003f, 0x0040, 0x005c, 0x00a0, 0x2003,
] as const;
const CREDENTIAL_SEPARATOR_SET_V1: ReadonlySet<number> =
  new Set<number>(CREDENTIAL_SEPARATOR_CODE_POINTS_V1);

const isCredentialSeparatorV1 = (value: string): boolean => {
  const codePoint = value.codePointAt(0);
  return codePoint !== undefined && CREDENTIAL_SEPARATOR_SET_V1.has(codePoint);
};

const asciiLowerV1 = (value: string): string =>
  [...value].map((ch) => {
    const cp = ch.codePointAt(0)!;
    return cp >= 0x41 && cp <= 0x5a
      ? String.fromCodePoint(cp + 0x20)
      : ch;
  }).join("");

const isAsciiV1 = (value: string): boolean =>
  [...value].every((ch) => ch.codePointAt(0)! <= 0x7f);

// SafeDisplayText may contain Unicode. Its credential scanner treats every
// non-ASCII scalar as a hard token boundary and never Unicode-case-folds it.
// ASCII-only scalar/key validators reject non-ASCII before calling this scan.
const credentialSemanticPartsV1 = (
  value: string,
  unicodeBoundary: boolean,
): readonly string[] => {
  const parts: string[] = [];
  let current = "";
  for (const ch of value) {
    if (isCredentialSeparatorV1(ch)
        || (unicodeBoundary && ch.codePointAt(0)! > 0x7f)) {
      if (current.length > 0) parts.push(current);
      current = "";
    } else {
      current += ch;
    }
  }
  if (current.length > 0) parts.push(current);
  return parts;
};

const canonicalSemanticKeyV1 = (value: string): string => {
  if (!isAsciiV1(value)) throw new Error("non-ASCII semantic key");
  return asciiLowerV1(
    [...value].filter((ch) => /[A-Za-z0-9]/.test(ch)).join(""),
  );
};

const hasTokenBoundaryMarkerV1 = (value: string, marker: string): boolean => {
  for (let from = 0; from <= value.length - marker.length; from += 1) {
    const index = value.indexOf(marker, from);
    if (index < 0) return false;
    if (index === 0 || !/[A-Za-z0-9]/.test(value.charAt(index - 1))) return true;
    from = index;
  }
  return false;
};

const credentialShapedTokenStreamV1 = (
  value: string,
  unicodeBoundary: boolean,
): boolean => {
  const lower = asciiLowerV1(value);
  const semantic = credentialSemanticPartsV1(lower, unicodeBoundary).some((part) => {
      const canonical = canonicalSemanticKeyV1(part);
      return FORBIDDEN_SEMANTIC_FIELDS_V1.has(canonical)
        || canonical === "bearer";
  });
  return semantic || CREDENTIAL_PREFIX_MARKERS_V1.some(
    (marker) => hasTokenBoundaryMarkerV1(lower, marker),
  );
};

export const credentialShapedAsciiV1 = (value: string): boolean =>
  !isAsciiV1(value) || credentialShapedTokenStreamV1(value, false);

export const credentialShapedDisplayV1 = (value: string): boolean =>
  credentialShapedTokenStreamV1(value, true);

export type SecretOwnerKind = "provider" | "agent";
export type SecretOwnerNamespace = Brand<string, "SecretOwnerNamespace">;
export type SecretPurpose = "codexApiKey";
export type SecretSlot = "primaryApiKey";

export interface SecretOwner {
  kind: SecretOwnerKind;
  namespace: SecretOwnerNamespace;
  ownerId: OwnerId;
  slot: SecretSlot;
}

export type SecretBackendKind = "osKeyring" | "hardware";
export type SecretBackendAvailability = "available" | "unavailable";
export type SecretPresence = "present" | "missing" | "unknown";
export type SecretStableAvailability =
  | "ready"
  | "missing"
  | "locked"
  | "denied"
  | "stale"
  | "revoked"
  | "unavailable";
export type SecretLockSource = "fyAgentPolicy" | "backend";
export type SecretRevocationSource =
  | "userDelete"
  | "centralBackend"
  | "deviceAdministration"
  | "supersededByRotation";
export type BackendObservedRevocationSource =
  | "centralBackend"
  | "deviceAdministration";
export type SecretBackendUnavailableReason =
  | "hardwareUnregistered"
  | "hardwareDisconnected"
  | "osStoreUnavailable"
  | "centralServiceUnavailable";
export type DeviceBinding = "hostUser" | "hardwareDevice";
export type PhysicalConfirmation = "never" | "optional" | "required";
export type HardwarePromptKey = "secret.hardware.confirmTouch";
export type StorageResidency = "osProtectedStore" | "hardwareOnly";
// Generic transport enums retain future-reserved literals so the command
// validator can return a typed rejection instead of silently accepting them.
export type SecretConsumer =
  | "changePlanApply"
  | "proxyRequest"
  | "usageProbe"
  | "codingPlanUsageProbe"
  | "modelFetch"
  | "providerTerminal";
export type ApplyTargetSink =
  | "processMemory"
  | "externalConfigFile"
  | "childProcessEnvironment";
export type SecretRuntimeConsumer =
  | "changePlanApply"
  | "proxyRequest"
  | "usageProbe"
  | "codingPlanUsageProbe"
  | "modelFetch";
export type SecretRuntimeSink = "processMemory" | "externalConfigFile";
export type SecretChangePlanApplyConsumer = "changePlanApply";
export type SecretChangePlanApplySink = "externalConfigFile";
export type CodexLiveSecretSinkId =
  | "codexAuthJsonOpenAiApiKey"
  | "codexConfigTomlExperimentalBearerToken";
export type ProxyRequestConsumer = "proxyRequest";
export type UsageProbeConsumer = "usageProbe";
export type CodingPlanUsageProbeConsumer = "codingPlanUsageProbe";
export type ModelFetchConsumer = "modelFetch";
export type ProcessMemorySink = "processMemory";
export type SecretBackendOperation =
  | "captureVerify"
  | "validate"
  | "resolveForApply"
  | "delete"
  | "revoke";
// Exactly five hardware-policy operations. Every fresh-missing readback uses
// the "validate" policy; its typed authorization/confirmation slot and
// durable delete-applied checkpoint nevertheless remain independent.

export interface SecretDeviceDisplay {
  displayName: SafeDisplayText;
  deviceClass: "osAccount" | "securityKey" | "secureElement" | "unknown";
  transport: "platform" | "usb" | "nfc" | "ble" | "unknown";
}

interface SecretBackendInstanceViewWire {
  kind: SecretBackendKind;
  instanceId: SecretBackendInstanceId;
  generation: SecretBackendGeneration;
  availability: SecretBackendAvailability;
  device?: SecretDeviceDisplay;
}

export type SecretBackendInstanceView = Brand<
  SecretBackendInstanceViewWire,
  "SecretBackendInstanceView"
>;

export interface SecretOperationConfirmationCapabilities {
  captureVerify: PhysicalConfirmation;
  validate: PhysicalConfirmation;
  resolveForApply: PhysicalConfirmation;
  delete: PhysicalConfirmation;
  revoke: PhysicalConfirmation;
}

export type BackendRevocationObservationCapability =
  | "unsupported"
  | "sourceAndTime";

interface SecretRecordCapabilitiesWire {
  schemaVersion: SchemaVersionV1;
  capabilityRevision: CapabilityRevision;
  backendKind: SecretBackendKind;
  backendInstanceId: SecretBackendInstanceId;
  backendGeneration: SecretBackendGeneration;
  deviceBindingGeneration: DeviceBindingGeneration;
  deviceBinding: DeviceBinding;
  storageResidency: StorageResidency;
  operationConfirmation: SecretOperationConfirmationCapabilities;
  allowedConsumers: readonly SecretRuntimeConsumer[];
  allowedSinks: readonly SecretRuntimeSink[];
  persistentTargetProjection: boolean;
  centralRevocation: boolean;
  revocationObservation: BackendRevocationObservationCapability;
  silentFallback: false;
}

export type SecretRecordCapabilities = Brand<
  SecretRecordCapabilitiesWire,
  "SecretRecordCapabilities"
>;

export interface SecretBindingSetCas {
  revision: SecretBindingSetRevision;
  digest: BindingSetDigest;
  count: number;
}

export interface SecretRecoveryCas {
  revision: SecretRecoveryRevision;
  digest: SecretRecoveryDigest;
}

export interface SecretOwnerBindingSummary {
  owner: SecretOwner;
  purpose: SecretPurpose;
  bindingRevision: SecretBindingRevision;
  createdAt: UtcTimestamp;
  updatedAt: UtcTimestamp;
}

export interface SecretLockView {
  source: SecretLockSource;
  lockedAt: UtcTimestamp;
}

export interface SecretRevocationView {
  source: SecretRevocationSource;
  revokedAt: UtcTimestamp;
}

export interface SecretIssueView {
  code: SecretErrorCode;
  retryable: boolean;
  action: SecretUserAction;
  lockSource?: SecretLockSource;
  revocationSource?: SecretRevocationSource;
  backendUnavailableReason?: SecretBackendUnavailableReason;
  recovery?: SecretRecoveryPointer;
}

export type SecretRecoveryKind =
  | "activationCleanup"
  | "captureCompensation"
  | "deleteFinalization"
  | "ownerDetachFinalization";

export interface SecretRecoveryPointer {
  recoveryId: SecretRecoveryId;
  kind: SecretRecoveryKind;
  recoveryCas: SecretRecoveryCas;
}

export interface SecretRefAggregate {
  schemaVersion: SchemaVersionV1;
  secretRef: SecretRef;
  secretRefDisplay: SecretRefDisplay;
  purpose: SecretPurpose;
  recordRevision: SecretRecordRevision;
  bindingSetCas: SecretBindingSetCas;
  backend: SecretBackendInstanceView;
  capabilities: SecretRecordCapabilities;
  bindings: readonly SecretOwnerBindingSummary[];
  presence: SecretPresence;
  availability: SecretStableAvailability;
  lock?: SecretLockView;
  revocation?: SecretRevocationView;
  issue?: SecretIssueView;
  createdAt: UtcTimestamp;
  rotatedAt?: UtcTimestamp;
  lastValidatedAt?: UtcTimestamp;
}

export type LegacySourceCategory =
  | "providerAuthJson"
  | "providerConfigTomlTopLevel"
  | "providerConfigTomlActiveTable"
  | "providerConfigTomlInactiveTable"
  | "providerConfigTomlInlineTable"
  | "providerUsageScriptApiKey"
  | "providerNonCanonicalProxyAlias";
export type LegacySourceOrigin =
  | "providerRow"
  | "liveAuth"
  | "liveConfig"
  | "sqlImportStaging"
  | "dbRestoreStaging"
  | "syncDownloadStaging";
export interface LegacySourceRef {
  locationId: LegacySourceLocationId;
  category: LegacySourceCategory;
  origin: LegacySourceOrigin;
}

// Supplemental inventory is deliberately no-value and non-addressable. It
// reports adjacent sources that block a clean startup/delete decision but are
// not approved current scrub targets and therefore never become LegacySourceRef.
export type SupplementalLegacySourceCategory =
  | "processEnvironment"
  | "windowsRegistryCurrentUser"
  | "windowsRegistryLocalMachine"
  | "shellStartupFile"
  | "commonConfigJson"
  | "commonConfigBackup"
  | "commonConfigMigrated"
  | "commonConfigSqlite"
  | "rendererLocalStorage"
  | "liveConfigMerge";

export interface AdjacentBlockedLegacySourceObservation {
  state: "adjacentBlocked";
  category: SupplementalLegacySourceCategory;
}

export type CurrentScrubbableLegacySourceCoverageView =
  | { state: "none"; sourceCount: 0; categories: readonly [] }
  | {
      state: "currentSourcesPresent";
      sourceCount: number;
      categories: readonly [LegacySourceCategory, ...LegacySourceCategory[]];
    };

export type AdjacentBlockedLegacySourceCoverageView =
  | { state: "none"; observationCount: 0; observations: readonly [] }
  | {
      state: "adjacentBlockedSourcesPresent";
      observationCount: number;
      observations: readonly [
        AdjacentBlockedLegacySourceObservation,
        ...AdjacentBlockedLegacySourceObservation[],
      ];
    };

export type LegacySourceCoverageView =
  | {
      state: "clear";
      currentScrubbable: Extract<
        CurrentScrubbableLegacySourceCoverageView,
        { state: "none" }
      >;
      adjacentBlocked: Extract<
        AdjacentBlockedLegacySourceCoverageView,
        { state: "none" }
      >;
    }
  | {
      state: "blockingSourcesPresent";
      currentScrubbable: Extract<
        CurrentScrubbableLegacySourceCoverageView,
        { state: "currentSourcesPresent" }
      >;
      adjacentBlocked: AdjacentBlockedLegacySourceCoverageView;
    }
  | {
      state: "blockingSourcesPresent";
      currentScrubbable: Extract<
        CurrentScrubbableLegacySourceCoverageView,
        { state: "none" }
      >;
      adjacentBlocked: Extract<
        AdjacentBlockedLegacySourceCoverageView,
        { state: "adjacentBlockedSourcesPresent" }
      >;
    };

export interface LegacySourceExpectation {
  source: LegacySourceRef;
  structuralRevision: LegacySourceStructuralRevision;
}
export type CurrentLegacySourceExpectations = Brand<
  readonly LegacySourceExpectation[],
  "CurrentLegacySourceExpectations"
>;
export type StagedLegacySourceExpectations = Brand<
  readonly [LegacySourceExpectation, ...LegacySourceExpectation[]],
  "StagedLegacySourceExpectations"
>;
export type LegacyOwnerState =
  | "singleValuePending"
  | "sourcesConflict"
  | "sourceInvalid"
  | "bindingComparisonPending"
  | "bindingConflict"
  | "approvalRequired";

export interface BoundOwnerBindingState {
  state: "bound";
  secretRef: SecretRef;
  secretRefDisplay: SecretRefDisplay;
  bindingRevision: SecretBindingRevision;
}

export interface LegacyOwnerBindingState {
  state: "legacy";
  legacyState: LegacyOwnerState;
  sources: readonly LegacySourceRef[];
  sourceCount: number;
  action: SecretUserAction;
  candidateId?: SecretCandidateId;
  lastError?: SecretIssueView;
}

export interface UnboundOwnerBindingState {
  state: "unbound";
}

export type OwnerBindingState =
  | BoundOwnerBindingState
  | LegacyOwnerBindingState
  | UnboundOwnerBindingState;

export interface SecretOwnerCredentialSummary {
  schemaVersion: SchemaVersionV1;
  owner: SecretOwner;
  purpose: SecretPurpose;
  ownerBindingRevision: SecretOwnerBindingRevision;
  bindingState: OwnerBindingState;
  legacySourceCoverage: LegacySourceCoverageView;
}

export type SortedAffectedOwners = Brand<
  readonly [SecretOwnerBindingSummary, ...SecretOwnerBindingSummary[]],
  "SortedAffectedOwners"
>;
export type SortedOwnerSummaries = Brand<
  readonly [SecretOwnerCredentialSummary, ...SecretOwnerCredentialSummary[]],
  "SortedOwnerSummaries"
>;
export type SortedSecretOwners = Brand<
  readonly SecretOwner[],
  "SortedSecretOwners"
>;

export interface ListSecretSummariesResult {
  owners: readonly SecretOwnerCredentialSummary[];
  refs: readonly SecretRefAggregate[];
  nextCursor?: SecretSummaryCursor;
}

export interface SecretBackendOption {
  backend: SecretBackendInstanceView;
  capabilitiesForNewRecord: SecretRecordCapabilities;
}

export type BeginCaptureIntent =
  | "newBinding"
  | "replaceBinding"
  | "legacyReconcile";

export type SecretCaptureBindingView =
  | { state: "unbound" }
  | {
      state: "bound";
      secretRefDisplay: SecretRefDisplay;
      bindingRevision: SecretBindingRevision;
    }
  | {
      state: "legacy";
      legacyState: LegacyOwnerState;
      sourceCount: number;
    };

export interface SecretCaptureIntentView {
  schemaVersion: SchemaVersionV1;
  captureIntentId: SecretCaptureIntentId;
  owner: SecretOwner;
  purpose: SecretPurpose;
  intent: BeginCaptureIntent;
  currentBinding: SecretCaptureBindingView;
  legacySourceCoverage: LegacySourceCoverageView;
  expiresAt: UtcTimestamp;
}

export interface ListSecretBackendOptionsResult {
  captureIntent: SecretCaptureIntentView;
  options: readonly SecretBackendOption[];
}

export type SecretCandidateKind =
  | "newBinding"
  | "replaceBinding"
  | "rotateBindingSet"
  | "legacyReconcile"
  | "legacyScrubExistingBinding";
export type LegacyActivationComparisonPolicy =
  | "candidateEquality"
  | "explicitReplacement";
export type LegacyActivationComparisonImpact =
  | {
      policy: "candidateEquality";
      userMeaning: "verifySameValueMigration";
    }
  | {
      policy: "explicitReplacement";
      userMeaning: "replaceExistingCredential";
      affectedSourceCount: number;
      replacesBoundBinding: boolean;
    };
export type SecretCandidateState =
  | "verifiedPendingPlan"
  | "activated"
  | "discarded"
  | "cleanupRequired"
  | "expired";
export type CandidateTerminalState = "discarded" | "expired";

export interface ExpectedUnboundOwner {
  state: "unbound";
  owner: SecretOwner;
  ownerBindingRevision: SecretOwnerBindingRevision;
}

export interface ExpectedBoundOwner {
  state: "bound";
  owner: SecretOwner;
  secretRef: SecretRef;
  ownerBindingRevision: SecretOwnerBindingRevision;
  bindingRevision: SecretBindingRevision;
  sourceBindingSet: SecretBindingSetCas;
}

export type OwnerBindingExpectation =
  | ExpectedUnboundOwner
  | ExpectedBoundOwner;

export interface SecretCandidateSummary {
  schemaVersion: SchemaVersionV1;
  candidateId: SecretCandidateId;
  candidateRevision: SecretCandidateRevision;
  kind: SecretCandidateKind;
  comparisonPolicy: LegacyActivationComparisonPolicy;
  comparisonImpact: LegacyActivationComparisonImpact;
  state: SecretCandidateState;
  secretRef: SecretRef;
  secretRefDisplay: SecretRefDisplay;
  purpose: SecretPurpose;
  recordRevision: SecretRecordRevision;
  backend: SecretBackendInstanceView;
  capabilities: SecretRecordCapabilities;
  targetOwners: readonly SecretOwner[];
  expectedBindings: readonly OwnerBindingExpectation[];
  legacySourcesToScrub: CurrentLegacySourceExpectations;
  createdAt: UtcTimestamp;
  expiresAt: UtcTimestamp;
  pendingTerminalDisposition?: CandidateTerminalState;
  issue?: SecretIssueView;
}

export type ActivationOldRecordDeleteOperation = "delete";
export type ActivationOldRecordPostBindingState = "noBindings";

export type SecretActivationOldRecordDeleteExpectation =
  | {
      kind: "notApplicable";
    }
  | {
      kind: "deleteAfterActivation";
      operation: ActivationOldRecordDeleteOperation;
      oldSecretRef: SecretRef;
      expectedRecordRevision: SecretRecordRevision;
      expectedPreActivationBindingSet: SecretBindingSetCas;
      requiredPostActivationBindingState: ActivationOldRecordPostBindingState;
      backendInstanceId: SecretBackendInstanceId;
      backendGeneration: SecretBackendGeneration;
      deviceBindingGeneration: DeviceBindingGeneration;
      capabilityRevision: CapabilityRevision;
      deleteConfirmation: PhysicalConfirmation;
      missingReadbackOperation: "validate";
      missingReadbackScope: "activationOldRecordMissingReadback";
      missingReadbackConfirmation: PhysicalConfirmation;
    };

export interface SecretActivationCandidateReadExpectation {
  operation: ActivationCandidateReadOperation;
  scope: ActivationCandidateReadScope;
  backendInstanceId: SecretBackendInstanceId;
  backendGeneration: SecretBackendGeneration;
  deviceBindingGeneration: DeviceBindingGeneration;
  capabilityRevision: CapabilityRevision;
  confirmation: PhysicalConfirmation;
}

export interface SecretCandidateActivationProjection {
  contractVersion: SecretContractVersionV1;
  operation: SecretCandidateActivationOperation;
  candidateId: SecretCandidateId;
  candidateRevision: SecretCandidateRevision;
  kind: SecretCandidateKind;
  comparisonPolicy: LegacyActivationComparisonPolicy;
  comparisonImpact: LegacyActivationComparisonImpact;
  secretRef: SecretRef;
  purpose: SecretPurpose;
  recordRevision: SecretRecordRevision;
  backendInstanceId: SecretBackendInstanceId;
  backendGeneration: SecretBackendGeneration;
  deviceBindingGeneration: DeviceBindingGeneration;
  capabilityRevision: CapabilityRevision;
  targetOwners: readonly SecretOwner[];
  expectedBindings: readonly OwnerBindingExpectation[];
  legacySourcesToScrub: CurrentLegacySourceExpectations;
  candidateRead: SecretActivationCandidateReadExpectation;
  oldRecordDelete: SecretActivationOldRecordDeleteExpectation;
  projectionDigest: SecretProjectionDigest;
}

export type SecretCandidateActivationOperation = "secretCandidateActivation";
export type StagedSecretImportActivationOperation =
  "stagedSecretImportActivation";
export type CodexProviderApplyOperation = "codexProviderApply";

export interface StagedSourceSetCas {
  stagedRowRevision: StagedRowRevision;
  structureDigest: RecoveryStructureDigest;
  sourceCount: number;
}

export interface StagedSecretImportActivationProjection {
  contractVersion: SecretContractVersionV1;
  operation: StagedSecretImportActivationOperation;
  stageId: ImportStageId;
  owner: SecretOwner;
  stagedSourceSetCas: StagedSourceSetCas;
  sourceExpectations: StagedLegacySourceExpectations;
  candidateId: SecretCandidateId;
  candidateRevision: SecretCandidateRevision;
  comparisonPolicy: LegacyActivationComparisonPolicy;
  comparisonImpact: LegacyActivationComparisonImpact;
  secretRef: SecretRef;
  recordRevision: SecretRecordRevision;
  backendInstanceId: SecretBackendInstanceId;
  backendGeneration: SecretBackendGeneration;
  deviceBindingGeneration: DeviceBindingGeneration;
  capabilityRevision: CapabilityRevision;
  expectedLiveBinding: OwnerBindingExpectation;
  projectionDigest: SecretProjectionDigest;
}

export interface StagedImportResumeCas {
  revision: StagedImportResumeRevision;
  digest: StagedImportResumeDigest;
}

export interface ResumeStagedImportCutoverRequest {
  stageId: ImportStageId;
  expectedResumeCas: StagedImportResumeCas;
}

export type StagedSecretImportActivationResultDto =
  | {
      status: "activated" | "alreadyActivated";
      schemaVersion: SchemaVersionV1;
      stageId: ImportStageId;
      candidateId: SecretCandidateId;
      ownerSummary: SecretOwnerCredentialSummary;
      auditEventId: SecretAuditEventId;
    }
  | {
      status: "cutoverRecoveryRequired";
      schemaVersion: SchemaVersionV1;
      stageId: ImportStageId;
      action: "resumeStagedImportCutover";
      currentResumeCas: StagedImportResumeCas;
      auditEventId: SecretAuditEventId;
    };

// The public resume handler intentionally has a separate no-value result.
// Unlike first-run activation, each arm has the same exact five data fields;
// no arm can return schema/audit/candidate/owner/ref/summary.
export type ResumeStagedImportCutoverResultDto =
  | {
      status: "activated";
      stageId: ImportStageId;
      currentResumeCas: StagedImportResumeCas;
      action: "none";
      issue: null;
    }
  | {
      status: "alreadyActivated";
      stageId: ImportStageId;
      currentResumeCas: StagedImportResumeCas;
      action: "none";
      issue: null;
    }
  | {
      status: "cutoverRecoveryRequired";
      stageId: ImportStageId;
      currentResumeCas: StagedImportResumeCas;
      action: "resumeStagedImportCutover";
      issue: SecretIssueView;
    };

export interface SecretMutationImpact {
  schemaVersion: SchemaVersionV1;
  secretRef: SecretRef;
  secretRefDisplay: SecretRefDisplay;
  recordRevision: SecretRecordRevision;
  bindingSetCas: SecretBindingSetCas;
  affectedOwners: readonly SecretOwnerBindingSummary[];
  effect: "allBindingsAffected" | "oneBindingAffected";
  noFallback: true;
}

export interface SecretDeleteReadinessContext {
  schemaVersion: SchemaVersionV1;
  operationId: SecretOperationId;
  operation: "delete";
  secretRef: SecretRef;
  recordRevision: SecretRecordRevision;
  bindingSetCas: SecretBindingSetCas;
  checkedAt: UtcTimestamp;
  expiresAt: UtcTimestamp;
}

export type SecretDeleteReadiness =
  | {
      status: "ready";
      context: SecretDeleteReadinessContext;
    }
  | {
      status: "confirmationRequired";
      context: SecretDeleteReadinessContext;
      confirmation: SecretConfirmationRequirementView;
    }
  | {
      status: "blocked";
      context: SecretDeleteReadinessContext;
      error: SecretIssueView;
    };

export interface SecretDeleteImpact {
  impact: SecretMutationImpact;
  readiness: SecretDeleteReadiness;
}

export type SecretRecoveryStepKind =
  | "finalizeLegacyScrub"
  | "deleteOldRecord"
  | "verifyOldRecordMissing"
  | "deleteUncommittedRecord"
  | "verifyUncommittedRecordMissing"
  | "finalizeCaptureCompensation"
  | "deleteAdmittedRecord"
  | "verifyDeletedRecordMissing"
  | "finalizeDeletedRecord"
  | "finalizeOwnerDetach";

export type SortedRecoverySteps = Brand<
  readonly SecretRecoveryStepKind[],
  "SortedRecoverySteps"
>;
export type NonEmptySortedRecoverySteps = Brand<
  readonly [SecretRecoveryStepKind, ...SecretRecoveryStepKind[]],
  "NonEmptySortedRecoverySteps"
>;

export type SecretRecoveryStepImpact =
  | {
      kind: "finalizeLegacyScrub" | "deleteOldRecord"
        | "verifyOldRecordMissing"
        | "deleteUncommittedRecord" | "verifyUncommittedRecordMissing"
        | "deleteAdmittedRecord" | "verifyDeletedRecordMissing";
      backendKind: SecretBackendKind;
      backendInstanceId: SecretBackendInstanceId;
      confirmation: PhysicalConfirmation;
    }
  | {
      kind: "finalizeCaptureCompensation" | "finalizeDeletedRecord"
        | "finalizeOwnerDetach";
      confirmation: "never";
    };

export type NonEmptySortedRecoveryStepImpacts = Brand<
  readonly [SecretRecoveryStepImpact, ...SecretRecoveryStepImpact[]],
  "NonEmptySortedRecoveryStepImpacts"
>;

export type OwnerDetachRecoveryBindingState =
  | {
      state: "bound";
      secretRefDisplay: SecretRefDisplay;
      bindingRevision: SecretBindingRevision;
      bindingSetCas: SecretBindingSetCas;
    }
  | { state: "unbound" };

export interface SecretRecoveryReadinessContext {
  schemaVersion: SchemaVersionV1;
  operationId: SecretOperationId;
  operation: "recovery";
  recoveryId: SecretRecoveryId;
  recoveryKind: SecretRecoveryKind;
  recoveryCas: SecretRecoveryCas;
  checkedAt: UtcTimestamp;
  expiresAt: UtcTimestamp;
}

export type SecretRecoveryReadiness =
  | { status: "ready"; context: SecretRecoveryReadinessContext }
  | {
      status: "confirmationRequired";
      context: SecretRecoveryReadinessContext;
      confirmation: SecretConfirmationRequirementView;
    }
  | {
      status: "blocked";
      context: SecretRecoveryReadinessContext;
      error: SecretIssueView;
    };

type SecretRecoveryImpactWire =
  | {
      kind: "activationCleanup";
      impact: {
        schemaVersion: SchemaVersionV1;
        recoveryId: SecretRecoveryId;
        recoveryCas: SecretRecoveryCas;
        candidateId: SecretCandidateId;
        affectedOwners: SortedAffectedOwners;
        secretRefDisplay: SecretRefDisplay;
        pendingSteps: NonEmptySortedRecoveryStepImpacts;
        readiness: SecretRecoveryReadiness;
      };
    }
  | {
      kind: "captureCompensation";
      impact: {
        schemaVersion: SchemaVersionV1;
        recoveryId: SecretRecoveryId;
        recoveryCas: SecretRecoveryCas;
        candidateId: SecretCandidateId;
        secretRefDisplay: SecretRefDisplay;
        pendingSteps: NonEmptySortedRecoveryStepImpacts;
        readiness: SecretRecoveryReadiness;
      };
    }
  | {
      kind: "deleteFinalization";
      impact: {
        schemaVersion: SchemaVersionV1;
        recoveryId: SecretRecoveryId;
        recoveryCas: SecretRecoveryCas;
        affectedOwners: SortedAffectedOwners;
        secretRefDisplay: SecretRefDisplay;
        pendingSteps: NonEmptySortedRecoveryStepImpacts;
        readiness: SecretRecoveryReadiness;
      };
    }
  | {
      kind: "ownerDetachFinalization";
      impact: {
        schemaVersion: SchemaVersionV1;
        recoveryId: SecretRecoveryId;
        recoveryCas: SecretRecoveryCas;
        detachedOwner: SecretOwner;
        remainingOwners: SortedSecretOwners;
        bindingState: OwnerDetachRecoveryBindingState;
        pendingSteps: NonEmptySortedRecoveryStepImpacts;
        readiness: SecretRecoveryReadiness;
      };
    };

export type SecretRecoveryImpact = Brand<
  SecretRecoveryImpactWire,
  "SecretRecoveryImpact"
>;

type ActivationRecoveryResult =
  | {
      status: "complete" | "alreadyComplete";
      schemaVersion: SchemaVersionV1;
      recoveryId: SecretRecoveryId;
      recoveryCas: SecretRecoveryCas;
      completedSteps: SortedRecoverySteps;
      remainingSteps: readonly [];
      ownerSummaries: SortedOwnerSummaries;
      aggregate: SecretRefAggregate;
      candidate: SecretCandidateSummary;
      auditEventId: SecretAuditEventId;
    }
  | {
      status: "recoveryRequired";
      schemaVersion: SchemaVersionV1;
      recoveryId: SecretRecoveryId;
      recoveryCas: SecretRecoveryCas;
      completedSteps: SortedRecoverySteps;
      remainingSteps: NonEmptySortedRecoverySteps;
      ownerSummaries: SortedOwnerSummaries;
      aggregate: SecretRefAggregate;
      candidate: SecretCandidateSummary;
      issue: SecretIssueView;
      auditEventId: SecretAuditEventId;
    };
type CaptureCompensationRecoveryResult =
  | {
      status: "complete" | "alreadyComplete";
      schemaVersion: SchemaVersionV1;
      recoveryId: SecretRecoveryId;
      recoveryCas: SecretRecoveryCas;
      completedSteps: SortedRecoverySteps;
      remainingSteps: readonly [];
      candidateId: SecretCandidateId;
      secretRefDisplay: SecretRefDisplay;
      terminalCandidateState: "discarded";
      auditEventId: SecretAuditEventId;
    }
  | {
      status: "recoveryRequired";
      schemaVersion: SchemaVersionV1;
      recoveryId: SecretRecoveryId;
      recoveryCas: SecretRecoveryCas;
      completedSteps: SortedRecoverySteps;
      remainingSteps: NonEmptySortedRecoverySteps;
      candidateId: SecretCandidateId;
      secretRefDisplay: SecretRefDisplay;
      issue: SecretIssueView;
      auditEventId: SecretAuditEventId;
    };
type LocalRecoveryOutcome =
  | {
      status: "complete" | "alreadyComplete";
      schemaVersion: SchemaVersionV1;
      recoveryId: SecretRecoveryId;
      recoveryCas: SecretRecoveryCas;
      completedSteps: SortedRecoverySteps;
      remainingSteps: readonly [];
      auditEventId: SecretAuditEventId;
    }
  | {
      status: "recoveryRequired";
      schemaVersion: SchemaVersionV1;
      recoveryId: SecretRecoveryId;
      recoveryCas: SecretRecoveryCas;
      completedSteps: SortedRecoverySteps;
      remainingSteps: NonEmptySortedRecoverySteps;
      issue: SecretIssueView;
      auditEventId: SecretAuditEventId;
    };
type DeleteFinalizationRecoveryResult = {
  ownerSummaries: SortedOwnerSummaries;
  aggregate: SecretRefAggregate;
  outcome: LocalRecoveryOutcome;
};
type OwnerDetachFinalizationRecoveryResult = {
  detachedOwner: SecretOwner;
  remainingOwners: SortedSecretOwners;
  outcome: LocalRecoveryOutcome;
};
type SecretRecoveryResultWire =
  | { kind: "activationCleanup"; result: ActivationRecoveryResult }
  | { kind: "captureCompensation"; result: CaptureCompensationRecoveryResult }
  | { kind: "deleteFinalization"; result: DeleteFinalizationRecoveryResult }
  | {
      kind: "ownerDetachFinalization";
      result: OwnerDetachFinalizationRecoveryResult;
    };

export type SecretRecoveryResult = Brand<
  SecretRecoveryResultWire,
  "SecretRecoveryResult"
>;

export interface StageSecretCandidateResult {
  status: "staged";
  candidate: SecretCandidateSummary;
  activationProjection: SecretCandidateActivationProjection;
  impact: SecretMutationImpact | null;
  auditEventId: SecretAuditEventId;
}

export interface SecretCandidateWithProjection {
  candidate: SecretCandidateSummary;
  activationProjection: SecretCandidateActivationProjection;
}

export interface ListSecretCandidatesResult {
  candidates: readonly SecretCandidateWithProjection[];
}

export type CandidateDiscardConfirmationSlot =
  | "recordDelete"
  | "recordMissingReadback";
export type CandidateDiscardDeleteScope = "candidateDiscardRecordDelete";
export type CandidateDiscardMissingReadbackScope =
  "candidateDiscardRecordMissingReadback";

export interface SecretCandidateDiscardDeleteHardwareConfirmStep {
  schemaVersion: SchemaVersionV1;
  stepId: SecretConfirmationStepId;
  operationId: SecretOperationId;
  operation: "delete";
  scope: CandidateDiscardDeleteScope;
  backendInstanceId: SecretBackendInstanceId;
  device: SecretDeviceDisplay;
  promptKey: HardwarePromptKey;
  expiresAt: UtcTimestamp;
}

export interface SecretCandidateDiscardMissingHardwareConfirmStep {
  schemaVersion: SchemaVersionV1;
  stepId: SecretConfirmationStepId;
  operationId: SecretOperationId;
  operation: "validate";
  scope: CandidateDiscardMissingReadbackScope;
  backendInstanceId: SecretBackendInstanceId;
  device: SecretDeviceDisplay;
  promptKey: HardwarePromptKey;
  expiresAt: UtcTimestamp;
}

export type SecretCandidateDiscardHardwareConfirmStep =
  | {
      slot: "recordDelete";
      confirmation: SecretCandidateDiscardDeleteHardwareConfirmStep;
    }
  | {
      slot: "recordMissingReadback";
      confirmation: SecretCandidateDiscardMissingHardwareConfirmStep;
    };

type SecretCandidateDiscardPreparationViewWire =
  | {
      status: "prepared";
      schemaVersion: SchemaVersionV1;
      operationId: SecretOperationId;
      expiresAt: UtcTimestamp;
    }
  | {
      status: "confirmationRequired";
      schemaVersion: SchemaVersionV1;
      operationId: SecretOperationId;
      step: SecretCandidateDiscardHardwareConfirmStep;
    };

// Native-only strict decoder mirror. It is not a command result and never
// adds a renderer command or exposes a capability/pending id.
export type SecretCandidateDiscardPreparationView = Brand<
  SecretCandidateDiscardPreparationViewWire,
  "SecretCandidateDiscardPreparationView"
>;

// Internal durable-decoder mirror. These types never enter a command DTO;
// role brands prevent equal-shaped candidate/activation/recovery checkpoints
// from being assigned across roles after strict validation.
type BackendDeleteAppliedRevision = Brand<
  number,
  "BackendDeleteAppliedRevision"
>;
interface BackendDeleteAppliedCas {
  revision: BackendDeleteAppliedRevision;
  digest: RecoveryStructureDigest;
}
type CandidateDiscardDeleteCheckpoint = Brand<
  {
    deleteDisposition: "deleted" | "alreadyMissing";
    backendCompletedAt: UtcTimestamp;
    deleteAppliedCas: BackendDeleteAppliedCas;
  },
  "CandidateDiscardDeleteCheckpoint"
>;
type ActivationOldRecordDeleteCheckpoint = Brand<
  {
    deleteDisposition: "deleted" | "alreadyMissing";
    backendCompletedAt: UtcTimestamp;
    deleteAppliedCas: BackendDeleteAppliedCas;
  },
  "ActivationOldRecordDeleteCheckpoint"
>;
type RecoveryOldRecordDeleteCheckpoint = Brand<
  {
    deleteDisposition: "deleted" | "alreadyMissing";
    backendCompletedAt: UtcTimestamp;
    deleteAppliedCas: BackendDeleteAppliedCas;
  },
  "RecoveryOldRecordDeleteCheckpoint"
>;
type ActivationOldRecordDurableCheckpoint =
  | { state: "none" }
  | {
      state: "oldRecordDeleteApplied";
      deleteDisposition: "deleted" | "alreadyMissing";
      backendCompletedAt: UtcTimestamp;
      deleteAppliedCas: BackendDeleteAppliedCas;
    };
type DiscardCandidateRecoveryCheckpointWire =
  | { state: "intent" }
  | { state: "backendApplied"; checkpoint: CandidateDiscardDeleteCheckpoint }
  | {
      state: "missingReadbackVerified";
      checkpoint: CandidateDiscardDeleteCheckpoint;
      missingCheckedAt: UtcTimestamp;
    };
type DiscardCandidateJournalPhaseWire =
  | DiscardCandidateRecoveryCheckpointWire
  | {
      state: "recoveryRequired";
      lastErrorCode: SecretErrorCode;
      checkpoint: DiscardCandidateRecoveryCheckpointWire;
    }
  | { state: "terminal"; terminalDisposition: CandidateTerminalState };

export type DiscardSecretCandidateResult =
  | {
      status: "discarded" | "alreadyDiscarded";
      terminalState: "discarded";
      candidateId: SecretCandidateId;
      auditEventId: SecretAuditEventId;
    }
  | {
      status: "expired" | "alreadyExpired";
      terminalState: "expired";
      candidateId: SecretCandidateId;
      action: "refreshSummary";
      auditEventId: SecretAuditEventId;
    };

export type SortedLegacySourceRefs = Brand<
  readonly LegacySourceRef[],
  "SortedLegacySourceRefs"
>;
export type NonEmptySortedLegacySourceRefs = Brand<
  readonly [LegacySourceRef, ...LegacySourceRef[]],
  "NonEmptySortedLegacySourceRefs"
>;

export type SecretLegacyCleanupTerminal =
  | { status: "notApplicable" }
  | {
      status: "complete";
      scrubbedSources: SortedLegacySourceRefs;
    };

export type SecretLegacyCleanupPending =
  | {
      status: "partial";
      scrubbedSources: SortedLegacySourceRefs;
      retainedSources: NonEmptySortedLegacySourceRefs;
      issue: SecretIssueView;
    }
  | {
      status: "blocked";
      retainedSources: NonEmptySortedLegacySourceRefs;
      issue: SecretIssueView;
    };

export type SecretOldRecordCleanupTerminal =
  | { status: "notApplicable" }
  | {
      status: "deleted";
      oldSecretRefDisplay: SecretRefDisplay;
      supersession: {
        source: "supersededByRotation";
        revokedAt: UtcTimestamp;
      };
    }
  | {
      status: "alreadyMissing";
      oldSecretRefDisplay: SecretRefDisplay;
      supersession: {
        source: "supersededByRotation";
        revokedAt: UtcTimestamp;
      };
    };

export interface SecretOldRecordCleanupPending {
  status: "cleanupRequired";
  oldSecretRefDisplay: SecretRefDisplay;
  issue: SecretIssueView;
}

export interface SecretOldRecordNotAttempted {
  status: "notAttempted";
}

export interface SecretActivationCompleteCleanup {
  kind: "complete";
  legacy: SecretLegacyCleanupTerminal;
  oldRecord: SecretOldRecordCleanupTerminal;
}

export type SecretActivationPendingCleanup =
  | {
      kind: "legacyScrubPending";
      legacy: SecretLegacyCleanupPending;
      oldRecord: SecretOldRecordNotAttempted;
      recovery: SecretRecoveryPointer;
    }
  | {
      kind: "oldRecordDeletePending";
      legacy: SecretLegacyCleanupTerminal;
      oldRecord: SecretOldRecordCleanupPending;
      recovery: SecretRecoveryPointer;
    };

export type ActivationCandidateReadOperation = "resolveForApply";
export type ActivationCandidateReadScope = "activationCandidateCompare";

export interface SecretActivationReadHardwareConfirmStep {
  schemaVersion: SchemaVersionV1;
  stepId: SecretConfirmationStepId;
  operationId: SecretOperationId;
  operation: ActivationCandidateReadOperation;
  scope: ActivationCandidateReadScope;
  backendInstanceId: SecretBackendInstanceId;
  device: SecretDeviceDisplay;
  promptKey: HardwarePromptKey;
  expiresAt: UtcTimestamp;
}

export interface SecretActivationDeleteHardwareConfirmStep {
  schemaVersion: SchemaVersionV1;
  stepId: SecretConfirmationStepId;
  operationId: SecretOperationId;
  operation: ActivationOldRecordDeleteOperation;
  scope: ActivationOldRecordDeleteScope;
  backendInstanceId: SecretBackendInstanceId;
  device: SecretDeviceDisplay;
  promptKey: HardwarePromptKey;
  expiresAt: UtcTimestamp;
}

export type ActivationOldRecordDeleteScope = "activationOldRecordDelete";
export type ActivationOldRecordMissingReadbackScope =
  "activationOldRecordMissingReadback";

export interface SecretActivationOldRecordMissingHardwareConfirmStep {
  schemaVersion: SchemaVersionV1;
  stepId: SecretConfirmationStepId;
  operationId: SecretOperationId;
  operation: "validate";
  scope: ActivationOldRecordMissingReadbackScope;
  backendInstanceId: SecretBackendInstanceId;
  device: SecretDeviceDisplay;
  promptKey: HardwarePromptKey;
  expiresAt: UtcTimestamp;
}

export type SecretActivationHardwareConfirmStep =
  | SecretActivationReadHardwareConfirmStep
  | SecretActivationDeleteHardwareConfirmStep
  | SecretActivationOldRecordMissingHardwareConfirmStep;

type SecretActivationPreparationViewWire =
  | {
      schemaVersion: SchemaVersionV1;
      status: "prepared";
      operationId: SecretOperationId;
      expiresAt: UtcTimestamp;
    }
  | {
      schemaVersion: SchemaVersionV1;
      status: "confirmationRequired";
      operationId: SecretOperationId;
      step: SecretActivationHardwareConfirmStep;
    };

export type SecretActivationPreparationView = Brand<
  SecretActivationPreparationViewWire,
  "SecretActivationPreparationView"
>;

type SecretActivationResultDtoWire =
  | {
      schemaVersion: SchemaVersionV1;
      status: "activated" | "alreadyActivated";
      candidateId: SecretCandidateId;
      planId: ChangePlanId;
      aggregate: SecretRefAggregate;
      affectedOwners: SortedAffectedOwners;
      cleanup: SecretActivationCompleteCleanup;
      targetProjection: "notPerformedByActivation";
      auditEventId: SecretAuditEventId;
    }
  | {
      schemaVersion: SchemaVersionV1;
      status: "activatedCleanupPending";
      candidateId: SecretCandidateId;
      planId: ChangePlanId;
      aggregate: SecretRefAggregate;
      affectedOwners: SortedAffectedOwners;
      cleanup: SecretActivationPendingCleanup;
      targetProjection: "notPerformedByActivation";
      auditEventId: SecretAuditEventId;
    };

export type SecretActivationResultDto = Brand<
  SecretActivationResultDtoWire,
  "SecretActivationResultDto"
>;

export type SecretApplyRole = "target" | "rollback";

export interface SecretApplyTargetProjection {
  role: "target";
  consumer: SecretChangePlanApplyConsumer;
  targetSink: SecretChangePlanApplySink;
  liveSinkId: CodexLiveSecretSinkId;
  owner: SecretOwner;
  secretRef: SecretRef;
  ownerBindingRevision: SecretOwnerBindingRevision;
  bindingRevision: SecretBindingRevision;
  recordRevision: SecretRecordRevision;
  bindingSetCas: SecretBindingSetCas;
  backendInstanceId: SecretBackendInstanceId;
  backendGeneration: SecretBackendGeneration;
  deviceBindingGeneration: DeviceBindingGeneration;
  capabilityRevision: CapabilityRevision;
}

export interface SecretApplyRollbackProjection {
  role: "rollback";
  consumer: SecretChangePlanApplyConsumer;
  targetSink: SecretChangePlanApplySink;
  liveSinkId: CodexLiveSecretSinkId;
  owner: SecretOwner;
  secretRef: SecretRef;
  ownerBindingRevision: SecretOwnerBindingRevision;
  bindingRevision: SecretBindingRevision;
  recordRevision: SecretRecordRevision;
  bindingSetCas: SecretBindingSetCas;
  backendInstanceId: SecretBackendInstanceId;
  backendGeneration: SecretBackendGeneration;
  deviceBindingGeneration: DeviceBindingGeneration;
  capabilityRevision: CapabilityRevision;
}

export type SecretApplyCredentialProjection =
  | SecretApplyTargetProjection
  | SecretApplyRollbackProjection;

export interface SecretApplyPlanProjection {
  contractVersion: SecretContractVersionV1;
  operation: CodexProviderApplyOperation;
  target: SecretApplyTargetProjection;
  rollback?: SecretApplyRollbackProjection;
  projectionDigest: SecretProjectionDigest;
}

export interface SecretConfirmationRequirementView {
  operation: SecretBackendOperation;
  device: SecretDeviceDisplay;
  timeoutSeconds: ConfirmationTimeoutSeconds;
  promptKey: HardwarePromptKey;
}

export interface SecretApplyReadinessContext {
  schemaVersion: SchemaVersionV1;
  operationId: SecretOperationId;
  projection: SecretApplyCredentialProjection;
  checkedAt: UtcTimestamp;
  expiresAt: UtcTimestamp;
}

export type SecretApplyReadiness =
  | {
      status: "ready";
      context: SecretApplyReadinessContext;
    }
  | {
      status: "confirmationRequired";
      context: SecretApplyReadinessContext;
      confirmation: SecretConfirmationRequirementView;
    }
  | {
      status: "blocked";
      context: SecretApplyReadinessContext;
      error: SecretIssueView;
    };

export interface SecretApplyHardwareConfirmStep {
  schemaVersion: SchemaVersionV1;
  stepId: SecretConfirmationStepId;
  operationId: SecretOperationId;
  operation: "resolveForApply";
  role: SecretApplyRole;
  backendInstanceId: SecretBackendInstanceId;
  device: SecretDeviceDisplay;
  promptKey: HardwarePromptKey;
  expiresAt: UtcTimestamp;
}

export interface SecretNonApplyHardwareConfirmStep {
  schemaVersion: SchemaVersionV1;
  stepId: SecretConfirmationStepId;
  operationId: SecretOperationId;
  operation: "captureVerify" | "validate" | "delete" | "revoke";
  backendInstanceId: SecretBackendInstanceId;
  device: SecretDeviceDisplay;
  promptKey: HardwarePromptKey;
  expiresAt: UtcTimestamp;
}

export type HardwareConfirmStep =
  | SecretApplyHardwareConfirmStep
  | SecretNonApplyHardwareConfirmStep;

export type SecretApplyPreparationView =
  | {
      schemaVersion: SchemaVersionV1;
      status: "prepared";
      operationId: SecretOperationId;
      expiresAt: UtcTimestamp;
    }
  | {
      schemaVersion: SchemaVersionV1;
      status: "confirmationRequired";
      operationId: SecretOperationId;
      step: SecretApplyHardwareConfirmStep;
    };

export type SecretWriterReceiptDto =
  | {
      status: "succeeded";
      writerCode: "readbackMatched";
      targetEffect: "changed";
    }
  | {
      status: "failedBeforeMutation";
      writerCode: "writerFailed";
      targetEffect: "none";
    }
  | {
      status: "failedAfterMutation";
      writerCode: "writerFailed";
      targetEffect: "changedUnknown";
    }
  | {
      status: "readbackMismatch";
      writerCode: "readbackMismatch";
      targetEffect: "changed";
    }
  | {
      status: "readbackUnavailable";
      writerCode: "readbackUnavailable";
      targetEffect: "changedUnknown";
    };

export interface SecretApplyResultDto {
  schemaVersion: SchemaVersionV1;
  operationId: SecretOperationId;
  role: SecretApplyRole;
  status: "writerReturned";
  writer: SecretWriterReceiptDto;
  consumedRecordRevision: SecretRecordRevision;
  consumedBindingSetRevision: SecretBindingSetRevision;
  consumedBackendGeneration: SecretBackendGeneration;
  auditEventId: SecretAuditEventId;
}

export type SecretValidationOutcome = "valid" | "missing" | "blocked";
export interface SecretValidationResult {
  outcome: SecretValidationOutcome;
  aggregate: SecretRefAggregate;
  auditEventId: SecretAuditEventId;
}

export interface SecretMutationResult {
  aggregate: SecretRefAggregate;
  auditEventId: SecretAuditEventId;
}

export interface SecretDeleteResult {
  status: "revoked" | "alreadyRevoked";
  aggregate: SecretRefAggregate;
  auditEventId: SecretAuditEventId;
}

export type LegacyMigrationOwnerStatus =
  | "noCredential"
  | "alreadyMigrated"
  | "candidateStaged"
  | "cleanupCandidateStaged"
  | "conflict"
  | "sourceInvalid"
  | "comparisonPending"
  | "blocked"
  | "failed";

export interface LegacyMigrationOwnerResult {
  owner: SecretOwner;
  status: LegacyMigrationOwnerStatus;
  sources: readonly LegacySourceRef[];
  candidateId?: SecretCandidateId;
  activationProjection?: SecretCandidateActivationProjection;
  planId?: ChangePlanId;
  action: SecretUserAction;
  issue?: SecretIssueView;
}

export type HistoricalArtifactCategory =
  | "historicalProviderSnapshot"
  | "appPrivateCache"
  | "managedDiagnostic"
  | "managedBackup"
  | "userOwnedExport";

export interface SecretArtifactScanReport {
  status: "notRun" | "complete" | "partial" | "blocked";
  enumeratedCategories: readonly HistoricalArtifactCategory[];
  scannedCount: number;
  findingCount: number;
  reportOnlyCount: number;
  unreadableCount: number;
}

export interface SecretMigrationReport {
  schemaVersion: SchemaVersionV1;
  reportId: SecretMigrationReportId;
  status: "noChanges" | "staged" | "approvalRequired" | "partial" | "blocked";
  owners: readonly LegacyMigrationOwnerResult[];
  artifactScan: SecretArtifactScanReport;
  startedAt: UtcTimestamp;
  completedAt: UtcTimestamp;
}

export type SecretAuditAction =
  | "captureCandidate"
  | "discardCandidate"
  | "activateCandidate"
  | "validate"
  | "rotateCandidate"
  | "lock"
  | "unlock"
  | "delete"
  | "revoke"
  | "checkReadiness"
  | "prepareApply"
  | "confirmHardware"
  | "resolveApply"
  | "migrateLegacy"
  | "reconcileLegacy"
  | "reconcileRecovery"
  | "retryCleanup"
  | "cancelConfirmation";
export type SecretApplyAuditAction =
  | "prepareApply"
  | "confirmHardware"
  | "resolveApply";
export type SecretGeneralAuditAction = Exclude<
  SecretAuditAction,
  SecretApplyAuditAction
>;
export type SecretAuditOutcome =
  | "success"
  | "blocked"
  | "failed"
  | "partial"
  | "recovered";
export type SecretEffect =
  | "none"
  | "candidateStaged"
  | "bindingChanged"
  | "policyChanged"
  | "recordRevoked"
  | "targetWriterInvoked"
  | "cleanupPending";

export type SecretAuditScope =
  | {
      kind: "general";
      action: SecretGeneralAuditAction;
    }
  | {
      kind: "apply";
      action: SecretApplyAuditAction;
      role: SecretApplyRole;
    };

export interface SecretAuditEvent {
  schemaVersion: SchemaVersionV1;
  eventId: SecretAuditEventId;
  occurredAt: UtcTimestamp;
  operationId: SecretOperationId;
  scope: SecretAuditScope;
  outcome: SecretAuditOutcome;
  effect: SecretEffect;
  owner?: SecretOwner;
  secretRefDisplay?: SecretRefDisplay;
  backendKind?: SecretBackendKind;
  backendInstanceId?: SecretBackendInstanceId;
  errorCode?: SecretErrorCode;
}

export interface SecretAuditPage {
  events: readonly SecretAuditEvent[];
  nextCursor?: SecretAuditCursor;
}

export type SecretUserAction =
  | "none"
  | "retryCapture"
  | "retryRotation"
  | "retryProxyRequest"
  | "retryUsageProbe"
  | "retryCodingPlanUsageProbe"
  | "retryModelFetch"
  | "unlockFyAgent"
  | "unlockBackend"
  | "requestPermission"
  | "captureReplacement"
  | "chooseBackend"
  | "confirmDevice"
  | "refreshSummary"
  | "refreshDeleteImpact"
  | "refreshRecoveryImpact"
  | "reopenChangePlan"
  | "resolveLegacyConflict"
  | "discardCandidate"
  | "completeRecovery"
  | "resumeStagedImportCutover"
  | "reconnectDevice"
  | "openBackendSettings"
  | "contactAdministrator";

export type SecretCommandName =
  | "list_secret_summaries"
  | "list_secret_backend_options"
  | "begin_secret_capture"
  | "rotate_secret"
  | "list_secret_candidates"
  | "discard_secret_candidate"
  | "set_secret_locked"
  | "get_secret_delete_impact"
  | "delete_secret"
  | "get_secret_cleanup_impact"
  | "retry_secret_cleanup"
  | "validate_secret"
  | "check_secret_apply_readiness"
  | "migrate_legacy_codex_secrets"
  | "list_secret_audit";

export type SecretMainIntegrationCommandName =
  | "resume_staged_import_cutover";

export type SecretPostGuidanceDestination =
  | { kind: "none" }
  | { kind: "refreshSummary"; command: "list_secret_summaries" };

export type SecretActionDestination =
  | { kind: "none" }
  | { kind: "secretCommand"; command: SecretCommandName }
  | {
      kind: "freshSecretCommand";
      command: SecretCommandName;
      operationIdPolicy: "serverGeneratedNew";
    }
  | {
      kind: "secretCaptureFlow";
      intent: BeginCaptureIntent;
      listOptions: "list_secret_backend_options";
      selection: "registeredBackendOption";
      beginCapture: "begin_secret_capture";
      operationIdPolicy: "serverGeneratedNew";
    }
  | {
      kind: "fixedRuntimeFlow";
      entry:
        | "proxyRequest"
        | "usageProbe"
        | "codingPlanUsageProbe"
        | "modelFetch";
      operationIdPolicy: "serverGeneratedNew";
    }
  | {
      kind: "mainIntegrationCommand";
      command: SecretMainIntegrationCommandName;
      operationIdPolicy: "serverGeneratedNew";
    }
  | {
      kind: "secretCommandFlow";
      commands: readonly [SecretCommandName, SecretCommandName];
      operationIdPolicy: "serverGeneratedNew";
    }
  | { kind: "nativeConfirmationContinuation" }
  | {
      kind: "externalGuidance";
      guidance:
        | "unlockBackend"
        | "grantPermission"
        | "reconnectDevice"
        | "openBackendSettings"
        | "openChangePlan"
        | "contactAdministrator";
      after: SecretPostGuidanceDestination;
    };

export const SECRET_ACTION_DESTINATIONS_V1 = {
  none: { kind: "none" },
  retryCapture: {
    kind: "secretCaptureFlow",
    intent: "newBinding",
    listOptions: "list_secret_backend_options",
    selection: "registeredBackendOption",
    beginCapture: "begin_secret_capture",
    operationIdPolicy: "serverGeneratedNew",
  },
  retryRotation: {
    kind: "freshSecretCommand",
    command: "rotate_secret",
    operationIdPolicy: "serverGeneratedNew",
  },
  retryProxyRequest: {
    kind: "fixedRuntimeFlow",
    entry: "proxyRequest",
    operationIdPolicy: "serverGeneratedNew",
  },
  retryUsageProbe: {
    kind: "fixedRuntimeFlow",
    entry: "usageProbe",
    operationIdPolicy: "serverGeneratedNew",
  },
  retryCodingPlanUsageProbe: {
    kind: "fixedRuntimeFlow",
    entry: "codingPlanUsageProbe",
    operationIdPolicy: "serverGeneratedNew",
  },
  retryModelFetch: {
    kind: "fixedRuntimeFlow",
    entry: "modelFetch",
    operationIdPolicy: "serverGeneratedNew",
  },
  unlockFyAgent: { kind: "secretCommand", command: "set_secret_locked" },
  unlockBackend: {
    kind: "externalGuidance",
    guidance: "unlockBackend",
    after: { kind: "refreshSummary", command: "list_secret_summaries" },
  },
  requestPermission: {
    kind: "externalGuidance",
    guidance: "grantPermission",
    after: { kind: "refreshSummary", command: "list_secret_summaries" },
  },
  captureReplacement: {
    kind: "secretCaptureFlow",
    intent: "replaceBinding",
    listOptions: "list_secret_backend_options",
    selection: "registeredBackendOption",
    beginCapture: "begin_secret_capture",
    operationIdPolicy: "serverGeneratedNew",
  },
  chooseBackend: {
    kind: "secretCaptureFlow",
    intent: "newBinding",
    listOptions: "list_secret_backend_options",
    selection: "registeredBackendOption",
    beginCapture: "begin_secret_capture",
    operationIdPolicy: "serverGeneratedNew",
  },
  confirmDevice: { kind: "nativeConfirmationContinuation" },
  refreshSummary: { kind: "secretCommand", command: "list_secret_summaries" },
  refreshDeleteImpact: {
    kind: "freshSecretCommand",
    command: "get_secret_delete_impact",
    operationIdPolicy: "serverGeneratedNew",
  },
  refreshRecoveryImpact: {
    kind: "freshSecretCommand",
    command: "get_secret_cleanup_impact",
    operationIdPolicy: "serverGeneratedNew",
  },
  reopenChangePlan: {
    kind: "externalGuidance",
    guidance: "openChangePlan",
    after: { kind: "none" },
  },
  resolveLegacyConflict: {
    kind: "secretCaptureFlow",
    intent: "legacyReconcile",
    listOptions: "list_secret_backend_options",
    selection: "registeredBackendOption",
    beginCapture: "begin_secret_capture",
    operationIdPolicy: "serverGeneratedNew",
  },
  discardCandidate: {
    kind: "freshSecretCommand",
    command: "discard_secret_candidate",
    operationIdPolicy: "serverGeneratedNew",
  },
  completeRecovery: {
    kind: "secretCommandFlow",
    commands: ["get_secret_cleanup_impact", "retry_secret_cleanup"],
    operationIdPolicy: "serverGeneratedNew",
  },
  resumeStagedImportCutover: {
    kind: "mainIntegrationCommand",
    command: "resume_staged_import_cutover",
    operationIdPolicy: "serverGeneratedNew",
  },
  reconnectDevice: {
    kind: "externalGuidance",
    guidance: "reconnectDevice",
    after: { kind: "refreshSummary", command: "list_secret_summaries" },
  },
  openBackendSettings: {
    kind: "externalGuidance",
    guidance: "openBackendSettings",
    after: { kind: "refreshSummary", command: "list_secret_summaries" },
  },
  contactAdministrator: {
    kind: "externalGuidance",
    guidance: "contactAdministrator",
    after: { kind: "refreshSummary", command: "list_secret_summaries" },
  },
} as const satisfies Record<SecretUserAction, SecretActionDestination>;

export type SecretErrorCode =
  | "SECRET_REQUEST_INVALID"
  | "SECRET_REF_INVALID"
  | "SECRET_OWNER_KIND_UNSUPPORTED"
  | "SECRET_OWNER_NAMESPACE_UNSUPPORTED"
  | "SECRET_OWNER_NOT_FOUND"
  | "SECRET_OWNER_CONFLICT"
  | "SECRET_OPERATION_BUSY"
  | "SECRET_UNSUPPORTED_PURPOSE"
  | "SECRET_CONSUMER_UNSUPPORTED"
  | "SECRET_INPUT_CANCELLED"
  | "SECRET_INPUT_INVALID"
  | "SECRET_CANDIDATE_NOT_FOUND"
  | "SECRET_CANDIDATE_EXPIRED"
  | "SECRET_CANDIDATE_CONSUMED"
  | "SECRET_CHANGE_PLAN_REQUIRED"
  | "SECRET_CHANGE_PLAN_INVALID"
  | "SECRET_CHANGE_PLAN_STALE"
  | "SECRET_MIGRATION_REQUIRED"
  | "SECRET_LEGACY_SOURCE_INVALID"
  | "SECRET_LEGACY_CONFLICT"
  | "SECRET_LEGACY_COMPARISON_PENDING"
  | "SECRET_MIGRATION_FAILED"
  | "SECRET_MISSING"
  | "SECRET_LOCKED"
  | "SECRET_PERMISSION_DENIED"
  | "SECRET_BACKEND_UNAVAILABLE"
  | "SECRET_STALE"
  | "SECRET_REVOKED"
  | "SECRET_CONFIRMATION_REQUIRED"
  | "SECRET_CONFIRMATION_CANCELLED"
  | "SECRET_CONFIRMATION_EXPIRED"
  | "SECRET_CONFIRMATION_REPLAYED"
  | "SECRET_DEVICE_MISMATCH"
  | "SECRET_WRITE_FAILED"
  | "SECRET_READ_FAILED"
  | "SECRET_DELETE_FAILED"
  | "SECRET_VERIFY_FAILED"
  | "SECRET_PROJECTION_FORBIDDEN"
  | "SECRET_DEPENDENCY_CHANGED"
  | "SECRET_RECORD_CHANGED"
  | "SECRET_BACKEND_CHANGED"
  | "SECRET_CAPABILITY_EXPIRED"
  | "SECRET_CAPABILITY_CONSUMED"
  | "SECRET_RECOVERY_NOT_FOUND"
  | "SECRET_RECOVERY_CHANGED"
  | "SECRET_OPERATION_RECOVERY_REQUIRED"
  | "SECRET_INTERNAL";

export interface SecretErrorView {
  code: SecretErrorCode;
  retryable: boolean;
  action: SecretUserAction;
  effect: SecretEffect;
  auditEventId?: SecretAuditEventId;
  owner?: SecretOwner;
  secretRefDisplay?: SecretRefDisplay;
  lockSource?: SecretLockSource;
  revocationSource?: SecretRevocationSource;
  backendUnavailableReason?: SecretBackendUnavailableReason;
  recovery?: SecretRecoveryPointer;
}

export interface SecretCommandSuccess<T> {
  contractVersion: SecretContractVersionV1;
  schemaVersion: SchemaVersionV1;
  commandId: SecretCommandId;
  data: T;
}

export interface SecretCommandError {
  contractVersion: SecretContractVersionV1;
  schemaVersion: SchemaVersionV1;
  commandId: SecretCommandId;
  error: SecretErrorView;
}

export interface ListSecretSummariesRequest {
  schemaVersion: SchemaVersionV1;
  owner?: SecretOwner;
  secretRef?: SecretRef;
  availability?: readonly SecretStableAvailability[];
  includeUnboundOwners: boolean;
  cursor?: SecretSummaryCursor;
  limit: PageLimit;
}

export interface ListSecretBackendOptionsRequest {
  schemaVersion: SchemaVersionV1;
  owner: SecretOwner;
  purpose: SecretPurpose;
  intent: BeginCaptureIntent;
}

export interface BeginSecretCaptureRequest {
  schemaVersion: SchemaVersionV1;
  captureIntentId: SecretCaptureIntentId;
  backendInstanceId: SecretBackendInstanceId;
}

export interface RotateSecretRequest {
  schemaVersion: SchemaVersionV1;
  secretRef: SecretRef;
  backendInstanceId: SecretBackendInstanceId;
  expectedRecordRevision: SecretRecordRevision;
  expectedBindingSet: SecretBindingSetCas;
}

export interface ListSecretCandidatesRequest {
  schemaVersion: SchemaVersionV1;
  owner?: SecretOwner;
  includeTerminal: boolean;
}

export interface DiscardSecretCandidateRequest {
  schemaVersion: SchemaVersionV1;
  candidateId: SecretCandidateId;
  expectedCandidateRevision: SecretCandidateRevision;
}

export interface SetSecretLockedRequest {
  schemaVersion: SchemaVersionV1;
  secretRef: SecretRef;
  locked: boolean;
  expectedRecordRevision: SecretRecordRevision;
  expectedBindingSet: SecretBindingSetCas;
}

export interface GetSecretDeleteImpactRequest {
  schemaVersion: SchemaVersionV1;
  secretRef: SecretRef;
}

export interface DeleteSecretRequest {
  schemaVersion: SchemaVersionV1;
  operationId: SecretOperationId;
  secretRef: SecretRef;
  expectedRecordRevision: SecretRecordRevision;
  expectedBindingSet: SecretBindingSetCas;
}

export interface ValidateSecretRequest {
  schemaVersion: SchemaVersionV1;
  secretRef: SecretRef;
  expectedRecordRevision: SecretRecordRevision;
}

export type CheckSecretApplyReadinessRequest =
  | {
      schemaVersion: SchemaVersionV1;
      role: "target";
      owner: SecretOwner;
      consumer: SecretConsumer;
      targetSink: ApplyTargetSink;
      liveSinkId: CodexLiveSecretSinkId;
    }
  | {
      schemaVersion: SchemaVersionV1;
      role: "rollback";
      owner: SecretOwner;
      consumer: SecretConsumer;
      targetSink: ApplyTargetSink;
      liveSinkId: CodexLiveSecretSinkId;
    };

export interface GetSecretCleanupImpactRequest {
  schemaVersion: SchemaVersionV1;
  recoveryId: SecretRecoveryId;
  recoveryKind: SecretRecoveryKind;
}

export interface RetrySecretCleanupRequest {
  schemaVersion: SchemaVersionV1;
  operationId: SecretOperationId;
  recoveryId: SecretRecoveryId;
  recoveryKind: SecretRecoveryKind;
  expectedRecoveryCas: SecretRecoveryCas;
}

export interface MigrateLegacyCodexSecretsRequest {
  schemaVersion: SchemaVersionV1;
  owner?: SecretOwner;
}

export interface ListSecretAuditRequest {
  schemaVersion: SchemaVersionV1;
  owner?: SecretOwner;
  secretRef?: SecretRef;
  actions?: readonly SecretAuditAction[];
  outcomes?: readonly SecretAuditOutcome[];
  cursor?: SecretAuditCursor;
  limit: PageLimit;
}

export type CodexWireApi = "responses" | "chatCompletions";

export interface CodexProviderConfigurationSummary {
  baseUrl?: ValidatedUrl;
  model?: CodexModelId;
  modelProviderId?: CodexModelProviderId;
  wireApi?: CodexWireApi;
  enabled: boolean;
}

export interface CodexProviderPublicDto {
  id: OwnerId;
  name: SafeDisplayText;
  configuration: CodexProviderConfigurationSummary;
  ownerBindingSummary: SecretOwnerCredentialSummary;
}

export type ProviderDeleteExistingBindingView =
  | {
      state: "bound";
      secretRefDisplay: SecretRefDisplay;
      bindingRevision: SecretBindingRevision;
      bindingSetCas: SecretBindingSetCas;
      remainingOwners: SortedSecretOwners;
      becomesOrphan: boolean;
    }
  | {
      state: "unbound";
      remainingOwners: readonly [];
      becomesOrphan: false;
    };

export type ProviderDeleteReadyImpact =
  | {
      bindingState: "bound";
      providerDeleteImpactId: ProviderDeleteImpactId;
      providerRowRevision: ProviderRowRevision;
      ownerBindingRevision: SecretOwnerBindingRevision;
      previewedAt: UtcTimestamp;
      expiresAt: UtcTimestamp;
      owner: SecretOwner;
      existingBinding: Extract<
        ProviderDeleteExistingBindingView,
        { state: "bound" }
      >;
      legacySourceCoverage: Extract<LegacySourceCoverageView, { state: "clear" }>;
      deleteAllowed: true;
      effect: "none";
      secretRetained: true;
      separateSecretDeleteAction: "get_secret_delete_impact";
    }
  | {
      bindingState: "unbound";
      providerDeleteImpactId: ProviderDeleteImpactId;
      providerRowRevision: ProviderRowRevision;
      ownerBindingRevision: SecretOwnerBindingRevision;
      previewedAt: UtcTimestamp;
      expiresAt: UtcTimestamp;
      owner: SecretOwner;
      existingBinding: Extract<
        ProviderDeleteExistingBindingView,
        { state: "unbound" }
      >;
      legacySourceCoverage: Extract<LegacySourceCoverageView, { state: "clear" }>;
      deleteAllowed: true;
      effect: "none";
      separateSecretDeleteAction: "notApplicable";
    };

export type ProviderDeleteBlockedLegacyImpact =
  | {
      bindingState: "bound";
      providerRowRevision: ProviderRowRevision;
      ownerBindingRevision: SecretOwnerBindingRevision;
      checkedAt: UtcTimestamp;
      owner: SecretOwner;
      existingBinding: Extract<
        ProviderDeleteExistingBindingView,
        { state: "bound" }
      >;
      legacySourceCoverage: Extract<
        LegacySourceCoverageView,
        { state: "blockingSourcesPresent" }
      >;
      deleteAllowed: false;
      effect: "none";
      action: "resolveLegacyConflict";
    }
  | {
      bindingState: "unbound";
      providerRowRevision: ProviderRowRevision;
      ownerBindingRevision: SecretOwnerBindingRevision;
      checkedAt: UtcTimestamp;
      owner: SecretOwner;
      existingBinding: Extract<
        ProviderDeleteExistingBindingView,
        { state: "unbound" }
      >;
      legacySourceCoverage: Extract<
        LegacySourceCoverageView,
        { state: "blockingSourcesPresent" }
      >;
      deleteAllowed: false;
      effect: "none";
      action: "resolveLegacyConflict";
    };

export type CodexProviderDeleteImpactDto =
  | {
      schemaVersion: SchemaVersionV1;
      status: "ready";
      impact: ProviderDeleteReadyImpact;
    }
  | {
      schemaVersion: SchemaVersionV1;
      status: "blockedLegacyResolutionRequired";
      blocked: ProviderDeleteBlockedLegacyImpact;
    };

export interface CodexProviderDeleteConfirmRequestDto {
  schemaVersion: SchemaVersionV1;
  providerDeleteImpactId: ProviderDeleteImpactId;
}

export type CodexProviderDeleteResultDto =
  | {
      schemaVersion: SchemaVersionV1;
      status: "providerDeletedBoundSecretRetained";
      consumedImpactId: ProviderDeleteImpactId;
      owner: SecretOwner;
      bindingState: "bound";
      remainingOwners: SortedSecretOwners;
      becomesOrphan: boolean;
      secretRetained: true;
      separateSecretDeleteAction: "get_secret_delete_impact";
    }
  | {
      schemaVersion: SchemaVersionV1;
      status: "providerDeletedUnbound";
      consumedImpactId: ProviderDeleteImpactId;
      owner: SecretOwner;
      bindingState: "unbound";
      remainingOwners: readonly [];
      becomesOrphan: false;
      separateSecretDeleteAction: "notApplicable";
    }
  | {
      schemaVersion: SchemaVersionV1;
      status: "providerDeletedBoundDetachRecoveryRequired";
      consumedImpactId: ProviderDeleteImpactId;
      owner: SecretOwner;
      bindingState: "bound";
      remainingOwners: SortedSecretOwners;
      becomesOrphan: boolean;
      secretRetained: true;
      separateSecretDeleteAction: "get_secret_cleanup_impact";
      recovery: {
        recoveryId: SecretRecoveryId;
        kind: "ownerDetachFinalization";
        recoveryCas: SecretRecoveryCas;
      };
    }
  | {
      schemaVersion: SchemaVersionV1;
      status: "providerDeletedUnboundDetachRecoveryRequired";
      consumedImpactId: ProviderDeleteImpactId;
      owner: SecretOwner;
      bindingState: "unbound";
      remainingOwners: readonly [];
      becomesOrphan: false;
      separateSecretDeleteAction: "get_secret_cleanup_impact";
      recovery: {
        recoveryId: SecretRecoveryId;
        kind: "ownerDetachFinalization";
        recoveryCas: SecretRecoveryCas;
      };
    };

export type CodexProviderMutationDto =
  | {
      operation: "create";
      name: SafeDisplayText;
      configuration: CodexProviderConfigurationSummary;
    }
  | {
      operation: "update";
      id: OwnerId;
      name: SafeDisplayText;
      configuration: CodexProviderConfigurationSummary;
    };

export interface CodexLiveStructuralProjection {
  contractVersion: SecretContractVersionV1;
  schemaVersion: SchemaVersionV1;
  owner: SecretOwner;
  structuralRevision: CodexLiveStructuralRevision;
  configuration: CodexProviderConfigurationSummary;
  bindingRequired: true;
}
```

### TypeScript invariants

- `owners[].bindingState.state=legacy` never fabricates a ref/backend/capability. It is the sole representation for legacy plaintext without a verified binding. Its cached action mapping is total and has no invocation-dependent `retry`: `singleValuePending → refreshSummary`, `sourcesConflict|sourceInvalid → resolveLegacyConflict`, `bindingComparisonPending → refreshSummary`, `bindingConflict → captureReplacement`, and `approvalRequired → reopenChangePlan`. Only `approvalRequired` may carry `candidateId`, and it must carry one; every other legacy state forbids it. No private action vocabulary is permitted.
- Owner validation order is fixed: validate scalar grammar, then reject `kind=agent` with `SECRET_OWNER_KIND_UNSUPPORTED` regardless of namespace, then require `provider.namespace == "codex"` or return `SECRET_OWNER_NAMESPACE_UNSUPPORTED`.
- Legacy `sourceCount == sources.length`, `locationId` values are unique within one owner scan, and ordinary `legacySourcesToScrub[].source` is an exact subset of the scan snapshot restricted to current `providerRow|liveAuth|liveConfig` occurrences. Each expectation preserves the exact locator/category/origin plus the occurrence's structural revision in the candidate projection. `sqlImportStaging|dbRestoreStaging|syncDownloadStaging` remain distinct staging origins and are never normalized to `providerRow`; they may be scrub expectations only inside the dedicated staged-import projection bound to the temp-DB live-object/CAS.
- `LegacySourceCoverageReceipt` is the canonical no-value supplemental inventory receipt. It is minted only through `CodexLegacySourceInventoryBridge`, whose private `CompleteLegacySourceInventoryAuthority` proves one fresh, complete inventory over exactly eleven fixed domains: `currentProviderLiveScrubbable` plus the ten supplemental domains. The opaque receipt's exact private fields are its non-value-derived `inventoryRevision`, `CompleteLegacySourceCoverageIdentity`, `currentScrubbable` and `adjacentBlocked`. The complete identity binds, for every named domain, its structural revision, presence and count; it contains no value, path, locator or value digest. `currentScrubbable` contains only exact current `LegacySourceRef` expectations approved for the ordinary scrub flow; their typed non-value-derived `LegacySourceLocationId` is permitted, but a raw locator/path/value/value-derived digest is not. `adjacentBlocked` contains only category/state observations for process environment, Windows HKCU/HKLM, shell-file category, common-config JSON/backup/migrated/SQLite, renderer localStorage and live merge. An adjacent observation has no location id, path, value, value digest or conversion to `LegacySourceRef`; it blocks startup/Provider deletion and appears in capture/owner coverage only. Startup reconciliation, each owner-summary projection, capture-intent mint and claimed-intent revalidation, and both Provider-delete preview and confirm revalidation must each obtain a new receipt from that bridge rather than reuse a receipt or run a divergent/partial inventory. Empty observation sets authorize startup `Clean` only when the consumed receipt still carries the checked complete eleven-domain identity.
- Candidate `targetOwners/expectedBindings` are non-empty, unique and sorted by `kind, namespace, ownerId, slot`; their owner sets are identical.
- Activation `oldRecordDelete.kind=notApplicable` is required unless an old record is explicitly deleted after binding. `deleteAfterActivation` must name the exact old ref/record/pre-activation binding-set/backend/device/capability snapshot implied by all bound expectations, require the post-activation old ref state `noBindings`, and carry independent delete and fresh-missing-readback confirmation policies. The binding checkpoint proves the admitted pre-set changed to that exact empty post-state before delete revalidation. The complete expectation is inside `projectionDigest`; the prepared bundle has distinct delete and missing-readback authorizations/CAS, and only a typed missing receipt may mint supersession or terminal cleanup. No post-approval inference may add/change it.
- Activation `candidateRead` is mandatory and must exactly repeat the candidate record's backend instance/generation, device-binding generation, capability revision and current `operationConfirmation.resolveForApply`, with literals `operation=resolveForApply` and `scope=activationCandidateCompare`. It is inside `projectionDigest` independently of `oldRecordDelete`; prepare/confirm cannot substitute one authorization or confirmation for the other.
- `refs[]` contains one row per logical ref; its `bindings` are sorted by `kind, namespace, ownerId, slot`. No field named `owner` appears singularly on a ref aggregate.
- `SecretRefAggregate.bindings.length == bindingSetCas.count` and the digest/revision correspond to that exact sorted set.
- `SecretStableAvailability` has no `migrationRequired` or `confirmationRequired`. Migration is owner-level; confirmation is operation-level.
- `SecretRefAggregate.issue.code` is limited to `SECRET_MISSING, SECRET_LOCKED, SECRET_PERMISSION_DENIED, SECRET_BACKEND_UNAVAILABLE, SECRET_STALE, SECRET_REVOKED, SECRET_DEVICE_MISMATCH, SECRET_OPERATION_RECOVERY_REQUIRED`. It is absent when availability is `ready`.
- `LegacyOwnerBindingState.lastError.code` is limited to `SECRET_MIGRATION_REQUIRED, SECRET_LEGACY_SOURCE_INVALID, SECRET_LEGACY_CONFLICT, SECRET_LEGACY_COMPARISON_PENDING, SECRET_MIGRATION_FAILED, SECRET_LOCKED, SECRET_PERMISSION_DENIED, SECRET_BACKEND_UNAVAILABLE`.
- A blocked readiness may use the stable codes plus `SECRET_CONSUMER_UNSUPPORTED, SECRET_PROJECTION_FORBIDDEN, SECRET_DEPENDENCY_CHANGED, SECRET_RECORD_CHANGED, SECRET_BACKEND_CHANGED, SECRET_CHANGE_PLAN_STALE`. `ready` has neither error nor confirmation; `confirmationRequired` has confirmation and no error.
- Apply-readiness `operationId` is a non-executable correlation id and is not hashed into the plan or accepted back from the renderer; `prepare_for_apply` creates a fresh native operation id. Delete/recovery readiness is registered in the process-local `SecretReadinessRegistry`: its textual `operationId` is only a lookup key for an opaque row bound to the complete delete identity or `recoveryKind + recoveryCas` and expiry. `delete_secret` / `retry_secret_cleanup` atomically claim that row before backend preparation; consumed, expired, cancelled, missing or already-claimed ids never authorize work and are replay-safe terminal states.
- `SecretApplyPlanProjection.target` is a `SecretApplyTargetProjection` and optional `rollback` is a `SecretApplyRollbackProjection`; neither role can be decoded as the other. Each role fixes `consumer=changePlanApply`, `targetSink=externalConfigFile` and one closed `CodexLiveSecretSinkId`. The bundle projection digest, #55 final baseline, prepared backend authorization and exact target/rollback writer all cover that same sink id. #41 may not synthesize rollback from target or from backup bytes, substitute a sink id, or supply an absolute path.
- Every readiness `context.projection` is role-discriminated. The generic request enums are decoded to the role-specific projection through `SecretRuntimeConsumer/SecretRuntimeSink`; `providerTerminal` and `childProcessEnvironment` reach that validator and produce typed `SECRET_CONSUMER_UNSUPPORTED` / `SECRET_PROJECTION_FORBIDDEN` rather than becoming an apply projection.
- Apply readiness requires the command/coordinator owner to resolve request `owner` to an owner-module-minted `ExistingSecretOwnerToken`; #35 requires the token and request owner to match before reading the device-local binding and requires `bindingState=bound`. It never accepts a candidate id/ref/projection, and it cannot turn `verifiedPendingPlan` into an apply projection.
- Prepared-bundle `expiresAt` is the earlier of target/rollback capability expiry. An apply `HardwareConfirmStep.role` is required and names the one role being confirmed; confirming it never authorizes the other role. Non-apply capture/delete/validate/revoke steps omit `role`.
- Audit `scope.kind=apply` is required for `prepareApply/confirmHardware/resolveApply` and carries a mandatory role; `scope.kind=general` cannot carry one. The TypeScript and Rust discriminated unions are identical.
- Non-apply hardware confirmation is audited under its parent general action (`captureCandidate/activateCandidate/validate/delete/revoke/retryCleanup`); `confirmHardware` is reserved for role-specific #41 apply confirmation, so no role-free `confirmHardware` event exists.
- `providerTerminal` and `childProcessEnvironment` are wire-reserved. v1 readiness for `providerTerminal` is always `status=blocked` with `SECRET_CONSUMER_UNSUPPORTED`, and neither value may appear in a record's `allowedConsumers/allowedSinks`.
- Record `allowedConsumers/allowedSinks` are sorted, duplicate-free arrays of the strict runtime enums. `changePlanApply` requires `externalConfigFile` plus `persistentTargetProjection=true`; `proxyRequest/usageProbe/codingPlanUsageProbe/modelFetch` require `processMemory`. A disagreement is `SECRET_BACKEND_CHANGED`, not a permissive fallback.
- `SecretBackendInstanceView` is a refined TypeScript brand and Rust private-field/custom-Deserialize wrapper. `kind=hardware` requires a present sanitized non-OS/non-platform device; `osKeyring` allows no device or exactly `osAccount/platform` and forbids hardware-only classes/transports. A registered instance's `kind/instanceId/generation/device` is one authority-issued tuple, and an unavailable previously-bound hardware instance remains that same tuple rather than becoming an OS option.
- `SecretRecordCapabilities` is a refined TypeScript brand and a Rust private-field wrapper over `SecretRecordCapabilitiesRepr`. Its custom `Deserialize` reruns the entire sorted/unique and consumer/sink/persistence/device/residency/revocation matrix. `centralRevocation` is true iff `revocationObservation=sourceAndTime`; no deserialized capability may claim one without the other. `osKeyring` is exact: all five confirmations are `never`, all five strict consumers (`changePlanApply/proxyRequest/usageProbe/codingPlanUsageProbe/modelFetch`) and both strict sinks are present in rank order, binding is `hostUser`, residency is `osProtectedStore`, persistent projection is true, central revocation is false and observation capability is `unsupported`. Hardware uses `hardwareDevice/hardwareOnly`, an explicit strict subset/matrix and no fallback. `try_new` copies kind/instance/generation and the sealed platform observation capability from the registered backend instead of accepting those identities as caller arguments. `PlatformBackendPort::capabilities_for_record` and `capabilities_for_new_record` return only the validated wrapper; `BackendInstanceHandle` rechecks outer instance/generation before exposing it.
- `issue.code=SECRET_LOCKED` requires `lockSource`. `availability=revoked` requires `revocation`. A logical unlock removes only `fyAgentPolicy` lock state, then performs a fresh backend probe; it never reports the backend unlocked.
- `issue/error.code=SECRET_OPERATION_RECOVERY_REQUIRED` requires a `SecretRecoveryPointer { recoveryId, kind, recoveryCas }`, except the closed candidate-terminal-cleanup form `SecretCandidateSummary{state=verifiedPendingPlan,pendingTerminalDisposition=discarded|expired,issue.action=discardCandidate}`. That form forbids `recovery`, exists only while the exact nonterminal `discardCandidate` journal has the same immutable disposition, and resumes through `discard_secret_candidate`, never `get_secret_cleanup_impact`. All other codes forbid a recovery pointer. `GetSecretCleanupImpactRequest.recoveryKind` and `RetrySecretCleanupRequest.recoveryKind` must equal the pointer/durable row or fail with `SECRET_RECOVERY_CHANGED/effect=none`.
- `persistentTargetProjection=false` means `externalConfigFile` is absent from `allowedSinks`. Both conditions are checked; disagreement is `SECRET_BACKEND_CHANGED`.
- Stable/public command result DTOs never contain a live `HardwareConfirmStep`. Only native coordinator state (`SecretApplyPreparationView`, `SecretActivationPreparationView`, `PrepareSecretRecovery::ConfirmationRequired` or `PrepareStagedImport::ConfirmationRequired`) carries its role/scope-specific step; no renderer DTO contains a capability id/token.
- `StageSecretCandidateResult.impact` is always present on the wire: `null` iff every expected owner is unbound and no current ref/binding is affected; otherwise a `SecretMutationImpact` object. Rust uses the required `NullableSecretMutationImpact` wrapper, so null is serialized but omission is rejected.
- `SecretRecoveryImpact`/`SecretRecoveryResult` are outer-tagged by exactly one `SecretRecoveryKind`. `activationCleanup` alone carries candidate/aggregate/shared-owner data and the suffix `finalizeLegacyScrub → deleteOldRecord → verifyOldRecordMissing`; `captureCompensation` alone carries candidate identity and the suffix `deleteUncommittedRecord → verifyUncommittedRecordMissing → finalizeCaptureCompensation`; `deleteFinalization` alone carries the admitted deleted ref/affected owners and the suffix `deleteAdmittedRecord → verifyDeletedRecordMissing → finalizeDeletedRecord`; `ownerDetachFinalization` alone carries detached/remaining owners and `finalizeOwnerDetach`. Pending steps are the exact phase-derived non-empty suffix, sorted and duplicate-free. A terminal outcome requires `remainingSteps=[]` and no issue; `recoveryRequired` requires a non-empty suffix plus an issue with the same pointer kind/id/CAS. Completed/remaining sets are disjoint and together match the impact snapshot's kind-specific step algebra.
- `ownerDetachFinalization.bindingState` is exact: `bound` requires `secretRefDisplay + bindingRevision + bindingSetCas` and canonical sorted-unique `remainingOwners`; `unbound` forbids those binding fields and requires no fabricated ref. Any `blockingSourcesPresent` coverage—current-scrubbable or adjacent-blocked—blocks Provider deletion before an impact id or detach journal exists. The durable detach journal/recovery step carries the required single literal `legacySourceCoverageState=clear` plus the same binding arm and distinct `ownerBindingRevision` tombstone expectation; coverage drift makes the Provider impact stale before either row can be written.
- For `activationCleanup`, `affectedOwners` and terminal/pending `ownerSummaries` are non-empty, duplicate-free and sorted by `kind,namespace,ownerId,slot`; both equal the recovery CAS owner set, so a shared ref is never collapsed to one owner. Rust serializes them through non-deserializable checked newtypes. A pending activation cleanup returns candidate `cleanupRequired`, every affected owner still bound, and active ref `stale`; other recovery kinds never synthesize that activation-only candidate state.
- `SecretRecoveryImpact`, `SecretRecoveryResult`, `SecretActivationPreparationView`, `SecretActivationResultDto`, issue/aggregate/owner-state/migration/result/audit DTOs deliberately do not implement Rust `Deserialize`. Their fields/private reprs are constructed only by checked authority factories. Projection/durable input types use private repr plus validating `Deserialize`. The TypeScript adapter decodes the same nested discriminants and exact per-variant key sets; no intersection/spread/passthrough decoder is permitted.
- `activationCleanup.finalizeLegacyScrub` carries the committed active record's backend identity and its read/compare confirmation policy; `deleteOldRecord` and `verifyOldRecordMissing` carry the old record's independent delete and fresh-missing policies. `confirmationRequired` is legal only for the next prepared slot and the typed operation/scope is respectively `resolveForApply/cleanupActiveRecordCompare`, `delete/cleanupOldRecordDelete`, or `validate/cleanupOldRecordMissingReadback`. Other recovery kinds cannot decode those slots.
- `SecretOldRecordCleanupTerminal.status=deleted|alreadyMissing` requires `supersession={source:supersededByRotation,revokedAt}` in both TypeScript and Rust; `notApplicable` forbids it. Normal activation, activation recovery, and any `RecoveryRequired` continuation retain the complete `{deleteDisposition,backendCompletedAt,deleteAppliedCas}` old-record checkpoint; its digest/preimage consumes all three fields. The fresh missing receipt and supersession/terminal write are atomic, and `revokedAt` is exactly `backendCompletedAt`, never request/plan/missing-check time.
- Candidate discard/expiry has no false terminal state. `discarded|alreadyDiscarded` requires the durable terminal target `discarded`; `expired|alreadyExpired` requires target `expired`. `expired` is written only after backend deleted/already-missing, a fresh missing readback and durable candidate-state finalization. Its native-only TypeScript/Rust preparation mirrors are the same strict two-arm algebra: `recordDelete` is `operation=delete/scope=candidateDiscardRecordDelete`, and `recordMissingReadback` is `operation=validate/scope=candidateDiscardRecordMissingReadback`; unknown or mixed slot/operation/scope fields reject, and neither view is a public command result. `BackendApplied` retains exact `{deleteDisposition,backendCompletedAt,deleteAppliedCas}`, while `MissingReadbackVerified` retains that same triple plus `missingCheckedAt`. Any delete/readback ambiguity leaves `verifiedPendingPlan` with the journal's checked immutable `pendingTerminalDisposition=discarded|expired`, `SECRET_OPERATION_RECOVERY_REQUIRED/action=discardCandidate` and the reachable `discardCandidate` journal; it never creates a general recovery row or the activation-only `cleanupRequired` mapping. `SECRET_CANDIDATE_EXPIRED` is reserved for a durably terminal `state=expired` candidate rejected by a non-discard operation; it is non-retryable with `refreshSummary` and is never the pending-cleanup signal.
- `comparisonPolicy` and `comparisonImpact.policy` must match in candidate durable state, activation projection, #55 admission, projection digest, final-baseline receipt and journal. Automatic migration and `legacyScrubExistingBinding` are exactly `candidateEquality/verifySameValueMigration`; explicit native capture from source/binding conflict, replace, reconcile and rotate are exactly `explicitReplacement/replaceExistingCredential`. Both re-resolve the complete admitted source set and revisions. Only `candidateEquality` constant-time-compares every old occurrence with the candidate; `explicitReplacement` validates the approved replacement impact and scrubs without old==candidate. Any missing/extra/retyped/relocated/revision drift is effect-none in both modes.
- Ordinary activation accepts only `CurrentLegacySourceExpectations`. `StagedSecretImportActivationProjection` accepts only staging origins and its token is unusable by live readiness/apply/runtime. Its comparison policy has the same two branches, but validation/scrub/readback/cutover run only through the main-integration-owned `ImportCutoverCoordinatorContext`; #41 is not an import coordinator.
- `SecretCandidateSummary.pendingTerminalDisposition` and `issue` are both absent except for `state=verifiedPendingPlan` with a durable nonterminal discard/expiry delete journal. That form requires the journal-matching disposition plus `SECRET_OPERATION_RECOVERY_REQUIRED/retryable=true/action=discardCandidate` and has no activation recovery pointer. `state=expired|discarded|activated|cleanupRequired` forbids both fields; terminal finalization removes the pending disposition in the same durable transition.
- Strict decoders reject an unknown key at every nested object boundary before semantic validation. No decoder uses passthrough, `flatten` or a permissive catch-all. Only a successfully decoded generic consumer/sink is converted to its strict runtime subtype.
- Every TypeScript `field?: T` in this contract is absent-only: omission is canonical and a present JSON `null` is `SECRET_REQUEST_INVALID` (or a response/fixture decoder failure). Rust pairs every deserializable absent-only `Option<T>` with `default + deserialize_absent_only`; serializers omit `None`. The sole explicit-null field is required `StageSecretCandidateResult.impact: SecretMutationImpact | null`; no other field has dual absent/null encodings.
- Before candidate activation, every `LegacySourceExpectation.source` is resolved against its exact origin/category/location, its current structural revision must equal `structuralRevision`, and the current complete source set must equal the admitted set. `candidateEquality` additionally constant-time-compares every current value to a fresh candidate backend read; `explicitReplacement` instead verifies the candidate authority plus approved replacement impact and intentionally permits old-value inequality. Missing, extra, retyped, relocated or revision-drifted occurrences return `SECRET_DEPENDENCY_CHANGED/effect=none` in both modes; value drift does so only in equality mode.

## 4. Binding-set CAS

`SecretBindingSetCas` is not a dependency count check. The device-local authority increments `revision` on every bind, unbind or binding revision change for a ref. `digest` is SHA-256 over these exact UTF-8 bytes:

```text
fyagent.secret.binding-set.v1\n
<secretRef>\n
<kind>\0<namespace>\0<ownerId>\0<slot>\0<bindingRevision>\n
... one line per binding in ascending byte order ...
```

`count` is display metadata only. Rotate, lock and delete compare `revision + digest + exact affected binding rows` inside the same mutation critical section. Any difference returns `SECRET_DEPENDENCY_CHANGED`, `effect=none`. Equal counts never authorize a changed owner set.

`SecretRecoveryCas` is likewise `revision + digest`, never a row count. The authority increments it on every recovery phase, remaining step, affected owner, source expectation, or record/store/binding/backend/device/capability/confirmation identity change. The digest preimage is exactly the concatenation below; literals, order, NUL and LF bytes are part of the grammar:

```text
fyagent.secret.recovery.v1\n
recovery\0activationCleanup\0<recoveryId>\0<phase>\n
time\0<createdAt>\0<updatedAt>\n
candidate\0<candidateId>\0<candidateRevision>\0<activeSecretRef>\0<activeRecordRevision>\n
owner\0<kind>\0<namespace>\0<ownerId>\0<slot>\0<ownerBindingRevision>\0<activeSecretRef>\0<bindingRevision>\n
... all affected owner rows in SecretOwner byte order ...
step\0finalizeLegacyScrub\0<activeSecretRef>\0<activeRecordRevision>\0<expectedStoreRevision>\0<bindingSetRevision>\0<bindingSetDigest>\0<bindingSetCount>\0<backendInstanceId>\0<backendGeneration>\0<deviceBindingGeneration>\0<capabilityRevision>\0<readConfirmation>\0<structureDigest>\n
source\0<locationId>\0<category>\0<origin>\0<structuralRevision>\n
... all source rows in LegacySourceRef byte order ...
step\0deleteOldRecord\0<oldSecretRef>\0<oldRecordRevision>\0<expectedStoreRevision>\0<bindingSetRevision>\0<bindingSetDigest>\0<bindingSetCount>\0<backendInstanceId>\0<backendGeneration>\0<deviceBindingGeneration>\0<capabilityRevision>\0<deleteConfirmation>\0noBindings\n
checkpoint\0<none|deleteApplied|stateFinalized>\n
deleteReceipt\0<deleted|alreadyMissing>\0<backendCompletedAt>\0<deleteAppliedCasRevision>\0<deleteAppliedCasDigest>\n
step\0verifyOldRecordMissing\0<readConfirmation>\n
oldRecordTerminal\0notApplicable\n
# OR, when old-record deletion was planned and the missing receipt has been consumed:
oldRecordTerminal\0<deleted|alreadyMissing>\n
supersession\0supersededByRotation\0<backendCompletedAt>\n

# OR captureCompensation, instead of the activation block:
recovery\0captureCompensation\0<recoveryId>\0<phase>\n
time\0<createdAt>\0<updatedAt>\n
candidate\0<candidateId>\0<candidateRevision>\0<secretRef>\0<recordRevision>\0<expectedStoreRevision>\n
bindingSet\0<bindingSetRevision>\0<bindingSetDigest>\00\n
backend\0<backendInstanceId>\0<backendGeneration>\0<deviceBindingGeneration>\0<capabilityRevision>\n
checkpoint\0<none|deleteApplied|missingReadbackVerified|stateFinalized>\n
deleteReceipt\0<deleted|alreadyMissing>\0<backendCompletedAt>\0<deleteAppliedCasRevision>\0<deleteAppliedCasDigest>\n
missingReceipt\0<missingCheckedAt>\n
finalized\0discarded\n
step\0deleteUncommittedRecord\0<deleteConfirmation>\n
step\0verifyUncommittedRecordMissing\0<readConfirmation>\n
step\0finalizeCaptureCompensation\0noBindings\0discarded\0absent\n

# OR deleteFinalization:
recovery\0deleteFinalization\0<recoveryId>\0<phase>\n
time\0<createdAt>\0<updatedAt>\n
admission\0<admissionIdHex>\0<readinessOperationId>\0<admittedAt>\0userDelete\n
record\0<secretRef>\0<recordRevision>\0<expectedStoreRevision>\0<bindingSetRevision>\0<bindingSetDigest>\0<bindingSetCount>\n
owner\0<kind>\0<namespace>\0<ownerId>\0<slot>\0<ownerBindingRevision>\0<secretRef>\0<bindingRevision>\n
... all affected owner rows in SecretOwner byte order ...
backend\0<backendInstanceId>\0<backendGeneration>\0<deviceBindingGeneration>\0<capabilityRevision>\n
checkpoint\0<none|deleteApplied|missingReadbackVerified|stateFinalized>\n
deleteReceipt\0<deleted|alreadyMissing>\0<backendCompletedAt>\0<deleteAppliedCasRevision>\0<deleteAppliedCasDigest>\n
missingReceipt\0<missingCheckedAt>\n
revocation\0userDelete\0<revokedAt>\n
step\0deleteAdmittedRecord\0<deleteConfirmation>\n
step\0verifyDeletedRecordMissing\0<readConfirmation>\n
step\0finalizeDeletedRecord\0retainedTombstones\0userDelete\n

# OR ownerDetachFinalization:
recovery\0ownerDetachFinalization\0<recoveryId>\0<phase>\n
time\0<createdAt>\0<updatedAt>\n
provider\0<providerDeleteImpactId>\0<providerRowRevision>\0<providerDetachTransactionIdHex>\0<providerDetachCommitIdHex>\n
detach\0<kind>\0<namespace>\0<ownerId>\0<slot>\0<expectedOwnerBindingRevision>\0<expectedStoreRevision>\n
legacy\0none\n
binding\0bound\0<secretRef>\0<bindingRevision>\0<bindingSetRevision>\0<bindingSetDigest>\0<bindingSetCount>\n
remainingOwner\0<kind>\0<namespace>\0<ownerId>\0<slot>\n
... all remaining owner rows in SecretOwner byte order ...
step\0finalizeOwnerDetach\0never\0backendMutationForbidden\n

# OR the mutually-exclusive unbound continuation:
binding\0unbound\n
step\0finalizeOwnerDetach\0never\0backendMutationForbidden\n
```

Each actual preimage contains exactly one kind block and only the receipt and remaining-step rows allowed by that arm's current phase. Activation encodes only the remaining suffix ranked `finalizeLegacyScrub < deleteOldRecord < verifyOldRecordMissing`. Its only crash-visible old-record checkpoints are `none`, `deleteApplied`, and terminal `stateFinalized`: `deleteApplied` carries `deleteReceipt + verifyOldRecordMissing`; a successful missing receipt is consumed inside the atomic terminal transaction and is never encoded as a standalone checkpoint/row. Activation terminal replaces the delete receipt and empty suffix with `oldRecordTerminal\0notApplicable` or `oldRecordTerminal\0<deleted|alreadyMissing>` plus `supersession\0supersededByRotation\0<backendCompletedAt>`. Capture and delete encode the exact suffix ranked delete → missing-readback → state-finalize; for those two kinds `deleteReceipt` includes the exact `deleteAppliedCasRevision + deleteAppliedCasDigest` and appears only for `deleteApplied|missingReadbackVerified` or the matching `recoveryRequired.checkpoint`, `missingReceipt` appears only for `missingReadbackVerified` or its matching checkpoint, and `stateFinalized|terminal` replaces intermediate receipts with `checkpoint\0stateFinalized` plus the exact `finalized`/`revocation` row. Thus every missing-readback authorization is bound by both typed state and recovery-CAS preimage to the same durable backend-applied checkpoint. Owner detach encodes exactly one `bound|unbound` continuation and always includes `legacy\0none`; no legacy arm exists. The journal typed `RecoveryRequired` link must equal the selected row's `recoveryId + kind + recoveryCas`.

Unsigned revisions/counts use minimal base-10 ASCII without sign/leading zero; timestamps are canonical RFC3339 UTC; digests and opaque receipt/transaction/commit identifiers are fixed lowercase hex. Every scalar forbids NUL/LF, so there is no escaping or Unicode normalization. `RecoveryAffectedOwner` is the sole durable owner source and binds owner-binding ABA, exact ref and binding revision. Old-record and capture-compensation binding-set counts are exactly zero. `RecoveryStructureDigest` covers only the activation scrub's identical sorted structural source rows. The preimage never contains a backend locator, path, material, value, material digest or value-derived digest. Any phase/checkpoint/receipt/remaining-step/identity change increments `SecretRecoveryCas.revision` before recomputing the digest.

`StagedImportResumeCas` has a separate credential-free canonical preimage. It binds the common journal `operationId` and the exact five-arm `StagedImportResumePhase`; it never substitutes a three-arm last-checkpoint enum or an optional receipt bag. The fixed prefix and row order are:

```text
fyagent.secret.staged-import-resume.v1\n
operation\0<operationId>\n
phase\0<intent|sourcesScrubbed|cutoverCommitted|liveOwnerMinted|localBindingFinalized>\n
stage\0<tempDatabaseDurableObjectIdHex>\0<processNonceHex>\0<stageId>\0<ownerKind>\0<ownerNamespace>\0<ownerId>\0<ownerSlot>\0<stagedRowRevision>\n
sourceSet\0<stagedRowRevision>\0<structureDigest>\0<sourceCount>\n
source\0<locationId>\0<category>\0<origin>\0<structuralRevision>\n
... all staged source rows in LegacySourceRef byte order ...
plan\0<admissionIdHex>\0<planId>\0<planDigest>\0<projectionDigest>\n
candidate\0<candidateId>\0<candidateRevision>\0<comparisonPolicy>\n
comparison\0<candidateEquality|explicitReplacement>\0<verifySameValueMigration|replaceExistingCredential>\0<affectedSourceCount-or-none>\0<replacesBoundBinding-or-none>\n
record\0<secretRef>\0<recordRevision>\0<expectedStoreRevision>\0<bindingSetRevision>\0<bindingSetDigest>\0<bindingSetCount>\n
backend\0<backendInstanceId>\0<backendGeneration>\0<deviceBindingGeneration>\0<capabilityRevision>\n
expectedLiveBinding\0<unbound|bound>\0<ownerBindingRevision>\0<secretRef-or-none>\0<bindingRevision-or-none>\0<sourceBindingSetRevision-or-none>\0<sourceBindingSetDigest-or-none>\0<sourceBindingSetCount-or-none>\n
sourcesScrubbed\0<stagedRowRevisionAfterScrub>\0<structureDigest>\00\n
cutover\0<cutoverReceiptIdHex>\n
promotedOwner\0<kind>\0<namespace>\0<ownerId>\0<slot>\0<providerRowRevision>\0<ownerBindingRevision>\n
```

`operationId` is copied from the same `DurableSecretOperationJournal.operation_id`; it is immutable for that journal and is never reconstructed from `stageId` or a fresh attempt. Any mismatch rejects rather than creating another revision of that row. `comparison` is exactly `candidateEquality\0verifySameValueMigration\0none\0none` or `explicitReplacement\0replaceExistingCredential\0<count>\0<true|false>` after its row key; each `none` is a literal rather than an empty field. `expectedLiveBinding` similarly uses the displayed `none` literals for every forbidden value in the unbound arm. The phase arm fixes the exact suffix of that grammar. `intent` forbids `sourcesScrubbed`, `cutover` and `promotedOwner`; `sourcesScrubbed` requires only its same-named row; `cutoverCommitted` requires `sourcesScrubbed + cutover`; `liveOwnerMinted` and `localBindingFinalized` each require all three, with the latter distinguished by its phase literal. Terminal projects its current resume CAS as `localBindingFinalized` with the same three cumulative rows. No arm permits an omitted cumulative receipt, extra row or promoted owner in an earlier phase. Every phase, process nonce, admission, source/CAS, receipt, promoted owner or other mutable preimage identity change first increments `StagedImportResumeCas.revision` and then recomputes its typed 64-lowerhex digest; changing any such byte under the old revision is stale and zero-write.

The canonical digest-fixture plan contains exactly these five positive fixtures, plus one-field mutation negatives for every row they admit:

| Fixture / adjacent crash point | Required phase rows | Forbidden phase rows / resumable suffix |
| --- | --- | --- |
| `staged_resume_intent_v1` (intent durable, before scrub) | `phase=intent` | forbid all three; resume scrub onward |
| `staged_resume_sources_scrubbed_v1` (scrub readback durable, before cutover) | `phase=sourcesScrubbed`, `sourcesScrubbed` | forbid `cutover,promotedOwner`; resume cutover onward |
| `staged_resume_cutover_committed_v1` (cutover receipt durable, before live owner) | `phase=cutoverCommitted`, `sourcesScrubbed`, `cutover` | forbid `promotedOwner`; resume live-owner mint onward |
| `staged_resume_live_owner_minted_v1` (owner readback durable, before local binding) | `phase=liveOwnerMinted`, all three cumulative rows | resume local binding only |
| `staged_resume_local_binding_finalized_v1` (local CAS durable, before terminal) | `phase=localBindingFinalized`, all three cumulative rows | resume terminal only |

Each fixture freezes the exact UTF-8 preimage, revision and expected lowercase SHA-256, round-trips only through the matching Rust phase arm, and pairs with the adjacent crash case that continues only the listed suffix. Each asserts that a changed `operationId` is not the same journal, while changing phase, nonce, admission, source/CAS, a cumulative receipt or promoted owner requires `revision + 1` and a different digest; the prior phase CAS, old nonce and old admission are stale/effect-none. Missing/extra phase rows and same-revision mutations must fail before resume authority is reopened.

`DurableSecretRecoveryRecord` is kind-tagged with no common optional bag. `RecoveryProviderProjection` can be constructed only from `activationCleanup/finalizeLegacyScrub`; #41 is forbidden to decode or infer from `SecretRecoveryAuthoritySnapshot`. Capture/delete are local-only after pre-action hardware preparation. Owner detach requires main integration's already-held `OwnerDetachCoordinatorContext`. Impact, readiness, bundle, coordinator and retry request all match `recoveryKind + recoveryCas` inside one mutation critical section or return `SECRET_RECOVERY_CHANGED/effect=none`.

In the preimage above, `\n` means one LF byte and `\0` means one NUL byte. `SecretProjectionDigest` is non-self-referential: serialize the full projection with `projectionDigest` omitted using RFC 8785 JSON canonicalization, including comparison policy/impact and every source expectation, then prefix with exactly `fyagent.secret.candidate-activation.v1\n`, `fyagent.secret.staged-import-activation.v1\n`, or `fyagent.secret.apply-projection.v1\n` by operation. SHA-256/lowercase-hex the result. #55 admission/final baseline store and compare that complete digest; no later policy/source/impact inference is allowed.

## 5. Staged candidate and Change Plan activation

### 5.1 Candidate rules

1. Before any backend write, native code persists a material-free operation intent containing the generated operation/candidate/ref/backend/owner identities.
2. Any backend confirmation for `captureVerify` completes before native secure input opens. The material is then captured, written and verified without waiting for Change Plan or Provider lease.
3. A successful backend write plus constant-time verify produces `verifiedPendingPlan`. It does not change bindings, scrub legacy fields, mutate current Provider or write a live target.
4. #55 hashes the complete `SecretCandidateActivationProjection` into an immutable plan. It MUST reject any projection containing an unknown field or a mismatched `projectionDigest`.
5. `activate_candidate_from_change_plan` re-reads the candidate, comparison policy/impact, every `OwnerBindingExpectation`, record/backend/capability generations and admitted plan. Before binding CAS it fresh-reads the candidate record and re-resolves the complete exact `LegacySourceRef` set/revisions under the lease. `candidateEquality` then constant-time-compares every source value to the candidate and yields the equality receipt. `explicitReplacement` consumes the candidate read into a fixed verification receipt, validates the approved replacement impact and yields a replacement receipt without old-value equality. Either receipt is policy-tagged and consumed by binding commit. Missing/extra/structural drift is `SECRET_DEPENDENCY_CHANGED/effect=none` in both modes; value drift is additionally effect-none for `candidateEquality`.
6. Historical snapshots, user-owned exports/backups and managed historical cache/diagnostic/backup artifacts are permanently scan/report-only in v1. They never enter `legacySourcesToScrub`, a candidate projection, recovery steps or `activatedCleanupPending`, and v1 has no approved-artifact rewrite/delete item. Only current `providerRow/liveAuth/liveConfig` occurrences may be exact activation scrub expectations.
7. Rotation deletes/revokes the old backend record only after the new binding set is active. Old-delete failure produces `activatedCleanupPending` and leaves the new binding active plus an old stale recovery item.
8. `list_secret_backend_options` atomically reads the existing owner token, purpose, requested intent, current owner-binding, current legacy-source expectation/hidden binding and exact registered backend option set, then mints one short-lived single-use `SecretCaptureIntentId`. `newBinding` requires the stored unbound snapshot; `replaceBinding` requires the stored bound snapshot; `legacyReconcile` additionally requires the stored current legacy snapshot and hidden binding. `begin_secret_capture` accepts only that id plus one returned `backendInstanceId`; the registry claims and revalidates the exact snapshot before material input or writes. The renderer never submits `OwnerBindingExpectation`. `rotateBindingSet` targets exactly every binding in the confirmed `SecretBindingSetCas` and cannot add/remove owners at activation.
9. Candidate persistence has no admission state. Admission is an ephemeral, consuming native #55 object; the persisted candidate transitions only `verifiedPendingPlan → activated|cleanupRequired` or `verifiedPendingPlan → discarded|expired`. `cleanupRequired` is legal only after activation binding CAS committed; a stale/invalid/cancelled admission or failed discard leaves `verifiedPendingPlan`.
10. Expiry is not a metadata-only state flip and introduces no journal operation kind. Explicit discard and the startup/list expiry sweep both journal `operationKind=discardCandidate` with a material-free immutable `terminalDisposition`; explicit discard fixes `discarded`, while the expiry sweep fixes `expired`. The exact progress phases are `intent → backendApplied{deleteDisposition,backendCompletedAt,deleteAppliedCas} → missingReadbackVerified{deleteDisposition,backendCompletedAt,deleteAppliedCas,missingCheckedAt} → terminal{terminalDisposition}`; `RecoveryRequired` retains one exact `DiscardCandidateRecoveryCheckpoint` arm, and there is no candidate-discard `stateFinalized` phase. Preparation has exactly two operation-specific slots, `CandidateDiscardConfirmationSlot::RecordDelete` (`Delete`) and `CandidateDiscardConfirmationSlot::RecordMissingReadback` (`Validate`), each with an independent one-shot authorization. Both may be confirmed before mutation, but the missing-readback authorization is unusable until the exact durable `BackendApplied` transition fulfills its operation-owned `BackendDeleteAppliedCasReservation`. Delete/already-missing alone never proves terminal state; the fresh missing readback is its own durable checkpoint. The startup/list sweeper may advance an OS-keyring `confirmation=never` intent automatically. For `confirmation=optional|required` it MUST NOT open a background prompt or pending hardware session; it only creates/preserves the reachable intent/issue, and solely an explicit `discard_secret_candidate` prepare/confirm flow may advance it. Retry loads the existing intent and cannot relabel `expired` as `discarded` or the reverse. Any confirmation cancellation/expiry, lock, permission, device/backend/generation drift, ambiguous delete or readback failure keeps `state=verifiedPendingPlan`, emits the checked `pendingTerminalDisposition` equal to that journal target and exposes `issue={code: SECRET_OPERATION_RECOVERY_REQUIRED, retryable: true, action: discardCandidate}`; the same journal remains reachable through a fresh `discard_secret_candidate(candidateId,expectedCandidateRevision)` invocation and the startup/list expiry sweeper. Terminal finalization atomically removes the unbound record and pending disposition while writing the exact candidate state/audit and terminal journal phase. An `expired|alreadyExpired` result has no pending issue, always returns `action=refreshSummary`, never reuses its old candidate, and cannot directly emit `retryCapture`; after refreshing owner/candidate state, only a newly minted capture intent or rotation flow may continue. No backend entry may survive behind an unreachable terminal candidate.
11. Staged SQL import/restore/sync never enters this ordinary current-owner activation. Main integration mints `StagedSecretOwnerToken` from one open temp DB object's structural inventory and obtains the dedicated #55 staged admission. It calls #35 staged prepare/confirm only to prepare authorization, with no candidate/staged read, then constructs its `ImportCutoverCoordinatorContext`. Only through that context may it fresh-validate exact staged source CAS/policy and values, scrub/read back those temp sources, perform sanitized cutover, receive the cutover receipt, mint the live DAO `ExistingSecretOwnerToken`, and finally finalize the local binding. Pre-cutover failure leaves main DB/live/local bindings unchanged; post-cutover failure persists staged-import recovery and blocks consumers until finalization. The staged token cannot authorize ordinary readiness/apply/runtime.

### 5.2 Change Plan projection rules

- #55 plan identity contains the complete projection and `projectionDigest` computed by `4`, never presence, material or material-derived data.
- #55 admission is single-consume and validates `planId + planDigest + projectionDigest + plan operation`.
- The activation projection always contains mandatory comparison policy/impact, `candidateRead` and closed `oldRecordDelete` expectations. `candidateRead` hashes the candidate backend/device/capability identity and exact read confirmation policy. Non-rotation uses old-delete `kind=notApplicable`; rotation additionally hashes the old ref/record/pre-activation binding-set/backend/device/capability revisions, literal post-state `noBindings`, exact delete policy and exact fresh-missing-readback policy. `prepare_candidate_activation/confirm_candidate_activation/cancel_candidate_activation` may authorize only those three independently planned slots.
- `StagedSecretImportActivationProjection.operation=stagedSecretImportActivation` is a separate #55 operation/admission and is never decoded as `secretCandidateActivation` or `codexProviderApply`. It hashes stage id, staged-row/source-set CAS, exact staging expectations, candidate/backend identities, comparison policy/impact and expected post-cutover live binding. Only main integration may consume it through the staged APIs/context.
- Candidate activation is not a target projection. `SecretActivationResultDto.targetProjection` is always `notPerformedByActivation`. Its #55 admission, Provider lease and #35 activation bundle are consumed/terminal and the Provider lease is released before any live-apply preview begins.
- Only after the owner is durably `bound` and not `cleanupRequired` may #55 create a new, separately hashed/approved `codexProviderApply` plan. `CheckSecretApplyReadinessRequest` and `SecretApplyPlanProjection` contain no `candidateId`; unknown candidate fields are rejected, and an unbound/candidate-only ref returns blocked `SECRET_CHANGE_PLAN_REQUIRED/effect=none`. The new plan follows its own `prepare/confirm/resolve` flow below.

## 6. Rust wire mirror

This is the canonical Rust shape. Request types use `deny_unknown_fields` so a caller cannot smuggle a value through an ignored key. All public response/error types are serializable; material/native authorization types in `7` deliberately are not.

### 6.1 Validating newtypes

```rust
use chrono::{DateTime, SecondsFormat, Utc};
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use std::{fmt, hash::Hash};
use uuid::{Variant, Version, Uuid};

const JS_SAFE_INTEGER_MAX: u64 = 9_007_199_254_740_991;

// Pair with #[serde(default, deserialize_with = "deserialize_absent_only")].
// Missing field -> None through default; a present JSON null is passed here
// and rejected because T (not Option<T>) must deserialize.
fn deserialize_absent_only<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(deserializer).map(Some)
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct WireValidationError(&'static str);

impl fmt::Display for WireValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

impl fmt::Debug for WireValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("WireValidationError(redacted)")
    }
}

impl std::error::Error for WireValidationError {}

fn valid_prefixed_uuid_v4(value: &str, prefix: &str) -> bool {
    let Some(raw) = value.strip_prefix(prefix) else {
        return false;
    };
    raw.len() == 32
        && raw.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        && Uuid::parse_str(raw).is_ok_and(|uuid| {
            uuid.get_version() == Some(Version::Random)
                && uuid.get_variant() == Variant::RFC4122
                && uuid.simple().to_string() == raw
        })
}

fn valid_change_plan_id(value: &str) -> bool {
    Uuid::parse_str(value).is_ok_and(|uuid| {
        uuid.get_version() == Some(Version::Random)
            && uuid.get_variant() == Variant::RFC4122
            && uuid.hyphenated().to_string() == value
    })
}

fn valid_hex_64(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn contains_token_boundary_marker(value: &str, marker: &str) -> bool {
    value.match_indices(marker).any(|(index, _)| {
        index == 0
            || !value.as_bytes()[index - 1].is_ascii_alphanumeric()
    })
}

// Separator-insensitive canonical semantic keys. This is the sole Rust set;
// §12.3 is the sole source-spelling list from which it is generated.
const FORBIDDEN_SEMANTIC_FIELDS_V1: &[&str] = &[
    "secret",
    "secretvalue",
    "value",
    "apikey",
    "openaiapikey",
    "experimentalbearertoken",
    "token",
    "accesstoken",
    "refreshtoken",
    "authorization",
    "accesskey",
    "secretkey",
    "password",
    "credential",
    "privatekey",
    "credentialblob",
    "backendlocator",
    "rawerror",
    "rawmessage",
    "rawconfig",
    "providersettings",
    "livesettings",
    "absolutepath",
    "materialdigest",
];

fn canonical_semantic_key_ascii(value: &str) -> Option<String> {
    value.is_ascii().then(|| {
        value
            .bytes()
            .filter(|byte| byte.is_ascii_alphanumeric())
            .map(|byte| byte.to_ascii_lowercase() as char)
            .collect()
    })
}

// Exact mirror of CREDENTIAL_SEPARATOR_CODE_POINTS_V1. Do not replace this
// with char::is_whitespace/is_ascii_whitespace or a regex character class.
const CREDENTIAL_SEPARATOR_CODE_POINTS_V1: [char; 19] = [
    '\t', '\n', '\u{000b}', '\u{000c}', '\r', ' ', '#', '&', ',', '.', '/',
    ':', ';', '=', '?', '@', '\\', '\u{00a0}', '\u{2003}',
];

fn is_credential_separator_v1(value: char) -> bool {
    CREDENTIAL_SEPARATOR_CODE_POINTS_V1.contains(&value)
}

fn credential_shaped_token_stream(value: &str, unicode_boundary: bool) -> bool {
    let lower = value.to_ascii_lowercase();
    let has_forbidden_key = lower
        .split(|ch| {
            is_credential_separator_v1(ch)
                || (unicode_boundary && !ch.is_ascii())
        })
        .any(|part| {
            canonical_semantic_key_ascii(part).is_some_and(|compact| {
                FORBIDDEN_SEMANTIC_FIELDS_V1.contains(&compact.as_str())
                    || compact == "bearer"
            })
        });
    const MARKERS: &[&str] = &[
        "sk-",
        "ghp_",
        "github_pat_",
        "glpat-",
        "akia",
        "aiza",
        "ya29.",
        "npm_",
        "pypi-",
        "hf_",
        "xoxb-",
        "xoxp-",
        "xoxa-",
        "eyj",
        "bearer ",
        "bearer%20",
    ];
    has_forbidden_key
        || MARKERS
            .iter()
            .any(|marker| contains_token_boundary_marker(&lower, marker))
}

pub(in crate::secret) fn credential_shaped_ascii(value: &str) -> bool {
    !value.is_ascii() || credential_shaped_token_stream(value, false)
}

pub(in crate::secret) fn credential_shaped_display(value: &str) -> bool {
    credential_shaped_token_stream(value, true)
}

fn valid_owner_id(value: &str) -> bool {
    let bytes = value.as_bytes();
    (1..=128).contains(&bytes.len())
        && bytes[0].is_ascii_alphanumeric()
        && bytes.iter().all(|b| {
            b.is_ascii_alphanumeric() || matches!(*b, b'.' | b'_' | b':' | b'-')
        })
        && !credential_shaped_ascii(value)
}

fn valid_owner_namespace(value: &str) -> bool {
    let bytes = value.as_bytes();
    (1..=32).contains(&bytes.len())
        && bytes[0].is_ascii_lowercase()
        && bytes
            .iter()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || *b == b'-')
}

fn valid_opaque_cursor(value: &str, prefix: &str) -> bool {
    value.strip_prefix(prefix).is_some_and(|raw| {
        raw.len() == 32
            && raw
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    })
}

fn valid_timestamp(value: &str) -> bool {
    DateTime::parse_from_rfc3339(value).is_ok_and(|parsed| {
        parsed.offset().local_minus_utc() == 0
            && parsed
                .with_timezone(&Utc)
                .to_rfc3339_opts(SecondsFormat::Millis, true)
                == value
    })
}

fn valid_safe_display(value: &str) -> bool {
    let count = value.chars().count();
    (1..=80).contains(&count)
        && value.trim() == value
        && !value.chars().any(char::is_control)
        && !value.starts_with('/')
        && !value.starts_with("\\\\")
        && !value.as_bytes().get(1).is_some_and(|b| *b == b':')
        && !credential_shaped_display(value)
}

fn valid_url(value: &str) -> bool {
    (1..=2048).contains(&value.len())
        && value.trim() == value
        && !value.chars().any(char::is_control)
        && url::Url::parse(value).is_ok_and(|parsed| {
            let path = parsed.path();
            let safe_path = path.len() <= 512
                && path.is_ascii()
                && !path.contains('%')
                && path.bytes().all(|byte| {
                    byte.is_ascii_alphanumeric()
                        || matches!(byte, b'/' | b'.' | b'_' | b'~' | b'-')
                })
                && path
                    .split('/')
                    .filter(|segment| !segment.is_empty())
                    .all(|segment| !credential_shaped_ascii(segment));
            matches!(parsed.scheme(), "http" | "https")
                && parsed.username().is_empty()
                && parsed.password().is_none()
                && parsed.query().is_none()
                && parsed.fragment().is_none()
                && parsed
                    .host_str()
                    .is_some_and(|host| {
                        host.split('.')
                            .all(|label| !credential_shaped_ascii(label))
                    })
                && safe_path
                && parsed.as_str() == value
        })
}

fn valid_codex_model_id(value: &str) -> bool {
    let bytes = value.as_bytes();
    (1..=128).contains(&bytes.len())
        && value.trim() == value
        && bytes[0].is_ascii_alphanumeric()
        && bytes.iter().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(*byte, b'.' | b'_' | b':' | b'/' | b'-')
        })
        && !credential_shaped_ascii(value)
}

macro_rules! validated_string_newtype {
    ($name:ident, $validate:expr, $label:literal) => {
        #[derive(Clone, PartialEq, Eq, Hash, Serialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            pub fn parse(value: String) -> Result<Self, WireValidationError> {
                if ($validate)(&value) {
                    Ok(Self(value))
                } else {
                    Err(WireValidationError($label))
                }
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                Self::parse(String::deserialize(deserializer)?)
                    .map_err(de::Error::custom)
            }
        }
    };
}

macro_rules! revision_newtype {
    ($name:ident) => {
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
        #[serde(transparent)]
        pub struct $name(u64);

        impl $name {
            pub fn parse(value: u64) -> Result<Self, WireValidationError> {
                if (1..=JS_SAFE_INTEGER_MAX).contains(&value) {
                    Ok(Self(value))
                } else {
                    Err(WireValidationError("invalid revision"))
                }
            }

            pub fn get(self) -> u64 {
                self.0
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                Self::parse(u64::deserialize(deserializer)?)
                    .map_err(de::Error::custom)
            }
        }
    };
}

validated_string_newtype!(SecretRef, |v: &str| valid_prefixed_uuid_v4(v, "sec_"), "invalid secret ref");
validated_string_newtype!(SecretCandidateId, |v: &str| valid_prefixed_uuid_v4(v, "scd_"), "invalid candidate id");
validated_string_newtype!(SecretOperationId, |v: &str| valid_prefixed_uuid_v4(v, "sop_"), "invalid operation id");
validated_string_newtype!(SecretCommandId, |v: &str| valid_prefixed_uuid_v4(v, "scm_"), "invalid command id");
validated_string_newtype!(SecretAuditEventId, |v: &str| valid_prefixed_uuid_v4(v, "sae_"), "invalid audit id");
validated_string_newtype!(SecretConfirmationStepId, |v: &str| valid_prefixed_uuid_v4(v, "scs_"), "invalid step id");
validated_string_newtype!(SecretBackendInstanceId, |v: &str| valid_prefixed_uuid_v4(v, "sbi_"), "invalid backend instance id");
validated_string_newtype!(DeviceInstanceId, |v: &str| valid_prefixed_uuid_v4(v, "dev_"), "invalid durable device instance id");
validated_string_newtype!(SecretRecoveryId, |v: &str| valid_prefixed_uuid_v4(v, "src_"), "invalid recovery id");
validated_string_newtype!(SecretCaptureIntentId, |v: &str| valid_prefixed_uuid_v4(v, "sci_"), "invalid capture intent id");
validated_string_newtype!(ImportStageId, |v: &str| valid_prefixed_uuid_v4(v, "ist_"), "invalid import stage id");
validated_string_newtype!(ProviderDeleteImpactId, |v: &str| valid_prefixed_uuid_v4(v, "pdi_"), "invalid Provider delete impact id");
validated_string_newtype!(SecretMigrationReportId, |v: &str| valid_prefixed_uuid_v4(v, "smr_"), "invalid report id");
validated_string_newtype!(LegacySourceLocationId, |v: &str| valid_opaque_cursor(v, "lsl_"), "invalid legacy source location id");
validated_string_newtype!(SecretSummaryCursor, |v: &str| valid_opaque_cursor(v, "ssc_"), "invalid summary cursor");
validated_string_newtype!(SecretAuditCursor, |v: &str| valid_opaque_cursor(v, "sac_"), "invalid audit cursor");
validated_string_newtype!(ChangePlanId, valid_change_plan_id, "invalid change plan id");
validated_string_newtype!(ChangePlanDigest, valid_hex_64, "invalid change plan digest");
validated_string_newtype!(BindingSetDigest, valid_hex_64, "invalid binding set digest");
validated_string_newtype!(SecretRecoveryDigest, valid_hex_64, "invalid recovery digest");
validated_string_newtype!(SecretProjectionDigest, valid_hex_64, "invalid projection digest");
validated_string_newtype!(RecoveryStructureDigest, valid_hex_64, "invalid recovery structure digest");
validated_string_newtype!(StagedImportResumeDigest, valid_hex_64, "invalid staged import resume digest");
validated_string_newtype!(OwnerId, valid_owner_id, "invalid owner id");
validated_string_newtype!(SecretOwnerNamespace, valid_owner_namespace, "invalid owner namespace");
validated_string_newtype!(SafeDisplayText, valid_safe_display, "invalid display text");
validated_string_newtype!(UtcTimestamp, valid_timestamp, "invalid UTC timestamp");
validated_string_newtype!(ValidatedUrl, valid_url, "invalid URL");
validated_string_newtype!(CodexModelId, valid_codex_model_id, "invalid Codex model id");
validated_string_newtype!(CodexModelProviderId, valid_codex_model_id, "invalid Codex model provider id");

revision_newtype!(SecretRecordRevision);
revision_newtype!(SecretCandidateRevision);
revision_newtype!(SecretBindingRevision);
revision_newtype!(SecretOwnerBindingRevision);
revision_newtype!(SecretBindingSetRevision);
revision_newtype!(SecretRecoveryRevision);
revision_newtype!(StagedImportResumeRevision);
revision_newtype!(StagedRowRevision);
revision_newtype!(ProviderRowRevision);
revision_newtype!(LegacySourceStructuralRevision);
revision_newtype!(CodexLiveStructuralRevision);
revision_newtype!(SecretBackendGeneration);
revision_newtype!(DeviceBindingGeneration);
revision_newtype!(CapabilityRevision);

#[derive(Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct PageLimit(u8);

impl PageLimit {
    pub fn parse(value: u8) -> Result<Self, WireValidationError> {
        if (1..=100).contains(&value) {
            Ok(Self(value))
        } else {
            Err(WireValidationError("page limit must be 1..=100"))
        }
    }
}

impl<'de> Deserialize<'de> for PageLimit {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::parse(u8::deserialize(deserializer)?).map_err(de::Error::custom)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct ConfirmationTimeoutSeconds(u16);

impl ConfirmationTimeoutSeconds {
    pub fn parse(value: u16) -> Result<Self, WireValidationError> {
        if (1..=300).contains(&value) {
            Ok(Self(value))
        } else {
            Err(WireValidationError("confirmation timeout must be 1..=300"))
        }
    }
}

impl<'de> Deserialize<'de> for ConfirmationTimeoutSeconds {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::parse(u16::deserialize(deserializer)?)
            .map_err(de::Error::custom)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct SchemaVersionV1;

impl Serialize for SchemaVersionV1 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u8(1)
    }
}

impl<'de> Deserialize<'de> for SchemaVersionV1 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        match u8::deserialize(deserializer)? {
            1 => Ok(Self),
            _ => Err(de::Error::custom("schemaVersion must be 1")),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct SecretRefDisplay(String);

impl SecretRefDisplay {
    pub(crate) fn derive_from(secret_ref: &SecretRef) -> Self {
        let value = secret_ref.as_str();
        Self(format!("sec_…{}", &value[value.len() - 4..]))
    }
}

// Output-only: no Deserialize/FromStr/TryFrom<String> implementation exists.
// Request DTOs accept SecretRef and derive this display only after authority
// lookup; response decoders verify it against the adjacent authoritative ref.

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct AlwaysFalse;

impl Serialize for AlwaysFalse {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_bool(false)
    }
}

impl<'de> Deserialize<'de> for AlwaysFalse {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        match bool::deserialize(deserializer)? {
            false => Ok(Self),
            true => Err(de::Error::custom("must be false")),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct AlwaysTrue;

impl Serialize for AlwaysTrue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_bool(true)
    }
}

impl<'de> Deserialize<'de> for AlwaysTrue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        match bool::deserialize(deserializer)? {
            true => Ok(Self),
            false => Err(de::Error::custom("must be true")),
        }
    }
}
```

### 6.2 Enums and stable DTOs

```rust
macro_rules! wire_enum {
    ($name:ident { $($variant:ident),+ $(,)? }) => {
        #[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
        #[serde(rename_all = "camelCase")]
        pub enum $name { $($variant),+ }
    };
}

wire_enum!(SecretOwnerKind { Provider, Agent });
wire_enum!(SecretPurpose { CodexApiKey });
wire_enum!(SecretSlot { PrimaryApiKey });
wire_enum!(SecretBackendKind { OsKeyring, Hardware });
wire_enum!(SecretBackendAvailability { Available, Unavailable });
wire_enum!(SecretPresence { Present, Missing, Unknown });
wire_enum!(SecretStableAvailability {
    Ready, Missing, Locked, Denied, Stale, Revoked, Unavailable
});
wire_enum!(SecretLockSource { FyAgentPolicy, Backend });
wire_enum!(SecretRevocationSource {
    UserDelete, CentralBackend, DeviceAdministration, SupersededByRotation
});
wire_enum!(BackendObservedRevocationSource { CentralBackend, DeviceAdministration });
wire_enum!(SecretBackendUnavailableReason {
    HardwareUnregistered, HardwareDisconnected, OsStoreUnavailable,
    CentralServiceUnavailable
});
wire_enum!(SecretRecoveryKind {
    ActivationCleanup, CaptureCompensation, DeleteFinalization,
    OwnerDetachFinalization
});
wire_enum!(DeviceBinding { HostUser, HardwareDevice });
wire_enum!(PhysicalConfirmation { Never, Optional, Required });
wire_enum!(StorageResidency { OsProtectedStore, HardwareOnly });
wire_enum!(SecretConsumer {
    ChangePlanApply, ProxyRequest, UsageProbe, CodingPlanUsageProbe,
    ModelFetch, ProviderTerminal
});
wire_enum!(ApplyTargetSink {
    ProcessMemory, ExternalConfigFile, ChildProcessEnvironment
});
wire_enum!(SecretRuntimeConsumer {
    ChangePlanApply, ProxyRequest, UsageProbe, CodingPlanUsageProbe, ModelFetch
});
wire_enum!(SecretRuntimeSink { ProcessMemory, ExternalConfigFile });
wire_enum!(SecretChangePlanApplyConsumer { ChangePlanApply });
wire_enum!(SecretChangePlanApplySink { ExternalConfigFile });
wire_enum!(CodexLiveSecretSinkId {
    CodexAuthJsonOpenAiApiKey,
    CodexConfigTomlExperimentalBearerToken
});
wire_enum!(SecretBackendOperation {
    CaptureVerify, Validate, ResolveForApply, Delete, Revoke
});
// There is no MissingReadback operation variant. All five typed missing-
// readback scopes map to Validate while retaining distinct slots, consuming
// authorizations and durable checkpoints.
wire_enum!(SecretCandidateKind {
    NewBinding, ReplaceBinding, RotateBindingSet, LegacyReconcile,
    LegacyScrubExistingBinding
});
wire_enum!(SecretCandidateState {
    VerifiedPendingPlan, Activated, Discarded, CleanupRequired, Expired
});
wire_enum!(LegacyActivationComparisonPolicy { CandidateEquality, ExplicitReplacement });

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "policy",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum LegacyActivationComparisonImpact {
    CandidateEquality {
        user_meaning: VerifySameValueMigrationMeaning,
    },
    ExplicitReplacement {
        user_meaning: ReplaceExistingCredentialMeaning,
        affected_source_count: u32,
        replaces_bound_binding: bool,
    },
}

wire_enum!(VerifySameValueMigrationMeaning { VerifySameValueMigration });
wire_enum!(ReplaceExistingCredentialMeaning { ReplaceExistingCredential });
wire_enum!(LegacySourceCategory {
    ProviderAuthJson, ProviderConfigTomlTopLevel,
    ProviderConfigTomlActiveTable, ProviderConfigTomlInactiveTable,
    ProviderConfigTomlInlineTable, ProviderUsageScriptApiKey,
    ProviderNonCanonicalProxyAlias
});
wire_enum!(LegacySourceOrigin {
    ProviderRow, LiveAuth, LiveConfig, SqlImportStaging,
    DbRestoreStaging, SyncDownloadStaging
});
wire_enum!(LegacyOwnerState {
    SingleValuePending, SourcesConflict, SourceInvalid, BindingComparisonPending,
    BindingConflict, ApprovalRequired
});

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LegacySourceRef {
    pub location_id: LegacySourceLocationId,
    pub category: LegacySourceCategory,
    pub origin: LegacySourceOrigin,
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SupplementalLegacySourceCategory {
    ProcessEnvironment,
    WindowsRegistryCurrentUser,
    WindowsRegistryLocalMachine,
    ShellStartupFile,
    CommonConfigJson,
    CommonConfigBackup,
    CommonConfigMigrated,
    CommonConfigSqlite,
    RendererLocalStorage,
    LiveConfigMerge,
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
enum AdjacentBlockedLegacySourceObservationState {
    AdjacentBlocked,
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdjacentBlockedLegacySourceObservation {
    state: AdjacentBlockedLegacySourceObservationState,
    category: SupplementalLegacySourceCategory,
}

impl AdjacentBlockedLegacySourceObservation {
    pub(crate) fn checked_from_codex_inventory_bridge(
        category: SupplementalLegacySourceCategory,
    ) -> Self {
        Self {
            state: AdjacentBlockedLegacySourceObservationState::AdjacentBlocked,
            category,
        }
    }
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LegacySourceExpectation {
    pub source: LegacySourceRef,
    pub structural_revision: LegacySourceStructuralRevision,
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct CurrentLegacySourceExpectations(Vec<LegacySourceExpectation>);

impl CurrentLegacySourceExpectations {
    fn validate(
        values: Vec<LegacySourceExpectation>,
    ) -> Result<Self, WireValidationError> {
        let current_only = values.iter().all(|expectation| {
            matches!(
                expectation.source.origin,
                LegacySourceOrigin::ProviderRow
                    | LegacySourceOrigin::LiveAuth
                    | LegacySourceOrigin::LiveConfig
            )
        });
        let sorted_unique = values.windows(2).all(|pair| {
            legacy_source_sort_key(&pair[0].source)
                < legacy_source_sort_key(&pair[1].source)
        });
        if current_only && sorted_unique {
            Ok(Self(values))
        } else {
            Err(WireValidationError(
                "legacy scrub expectations must be current/sorted/unique",
            ))
        }
    }

    pub(in crate::secret) fn as_slice(&self) -> &[LegacySourceExpectation] {
        &self.0
    }

    pub(crate) fn checked_from_codex_inventory_bridge(
        values: Vec<LegacySourceExpectation>,
    ) -> Result<Self, WireValidationError> {
        Self::validate(values)
    }
}

impl<'de> Deserialize<'de> for CurrentLegacySourceExpectations {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::validate(Vec::<LegacySourceExpectation>::deserialize(deserializer)?)
            .map_err(de::Error::custom)
    }
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "state",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
enum CurrentScrubbableLegacySourceCoverageView {
    None {
        source_count: u32,
        categories: [LegacySourceCategory; 0],
    },
    CurrentSourcesPresent {
        source_count: u32,
        categories: Vec<LegacySourceCategory>,
    },
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "state",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
enum AdjacentBlockedLegacySourceCoverageView {
    None {
        observation_count: u32,
        observations: [AdjacentBlockedLegacySourceObservation; 0],
    },
    AdjacentBlockedSourcesPresent {
        observation_count: u32,
        observations: Vec<AdjacentBlockedLegacySourceObservation>,
    },
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "state",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
enum LegacySourceCoverageRepr {
    Clear {
        current_scrubbable: CurrentScrubbableLegacySourceCoverageView,
        adjacent_blocked: AdjacentBlockedLegacySourceCoverageView,
    },
    BlockingSourcesPresent {
        current_scrubbable: CurrentScrubbableLegacySourceCoverageView,
        adjacent_blocked: AdjacentBlockedLegacySourceCoverageView,
    },
}

#[derive(Clone, PartialEq, Eq)]
pub struct LegacySourceCoverageView(LegacySourceCoverageRepr);

impl Serialize for LegacySourceCoverageView {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

// The following bridge/identity/authority types live in the crate-root
// main-integration module `crate::legacy_source_inventory`. The module is
// declared `pub(crate)` so `crate::store`, `crate::commands::provider` and the
// #35 secret module can name its opaque types. All fields and all authority /
// identity constructors remain private to that module.
#[derive(PartialEq, Eq)]
pub(crate) struct LegacySourceInventoryRevision(u64);

impl LegacySourceInventoryRevision {
    fn checked_from_structural_generation(
        revision: u64,
    ) -> Result<Self, SecretInternalError> {
        if revision == 0 || revision > 9_007_199_254_740_991 {
            return Err(todo!("checked internal invariant error"));
        }
        // Scanner permits the call only on structural-generation metadata and
        // rejects source bytes, values or digests as the argument origin.
        Ok(Self(revision))
    }
}

#[derive(PartialEq, Eq)]
enum LegacySourceDomainPresence {
    Absent,
    Present,
}

#[derive(PartialEq, Eq)]
struct LegacySourceDomainCoverageIdentity {
    structural_revision: LegacySourceInventoryRevision,
    presence: LegacySourceDomainPresence,
    source_count: u32,
}

impl LegacySourceDomainCoverageIdentity {
    fn checked_from_structural_inventory(
        structural_revision: LegacySourceInventoryRevision,
        presence: LegacySourceDomainPresence,
        source_count: u32,
    ) -> Result<Self, SecretInternalError> {
        let coherent = match presence {
            LegacySourceDomainPresence::Absent => source_count == 0,
            LegacySourceDomainPresence::Present => source_count > 0,
        };
        if !coherent {
            return Err(todo!("checked internal invariant error"));
        }
        Ok(Self {
            structural_revision,
            presence,
            source_count,
        })
    }
}

// Fixed named fields make omission, duplication, extension and category
// substitution unrepresentable. This identity never stores a LegacySourceRef,
// location id, path, source value, value-derived revision or value digest.
#[derive(PartialEq, Eq)]
pub(crate) struct CompleteLegacySourceCoverageIdentity {
    current_provider_live_scrubbable: LegacySourceDomainCoverageIdentity,
    process_environment: LegacySourceDomainCoverageIdentity,
    windows_registry_current_user: LegacySourceDomainCoverageIdentity,
    windows_registry_local_machine: LegacySourceDomainCoverageIdentity,
    shell_startup_file: LegacySourceDomainCoverageIdentity,
    common_config_json: LegacySourceDomainCoverageIdentity,
    common_config_backup: LegacySourceDomainCoverageIdentity,
    common_config_migrated: LegacySourceDomainCoverageIdentity,
    common_config_sqlite: LegacySourceDomainCoverageIdentity,
    renderer_local_storage: LegacySourceDomainCoverageIdentity,
    live_config_merge: LegacySourceDomainCoverageIdentity,
}

impl CompleteLegacySourceCoverageIdentity {
    fn checked_exact_eleven_domains(
        current_provider_live_scrubbable: LegacySourceDomainCoverageIdentity,
        process_environment: LegacySourceDomainCoverageIdentity,
        windows_registry_current_user: LegacySourceDomainCoverageIdentity,
        windows_registry_local_machine: LegacySourceDomainCoverageIdentity,
        shell_startup_file: LegacySourceDomainCoverageIdentity,
        common_config_json: LegacySourceDomainCoverageIdentity,
        common_config_backup: LegacySourceDomainCoverageIdentity,
        common_config_migrated: LegacySourceDomainCoverageIdentity,
        common_config_sqlite: LegacySourceDomainCoverageIdentity,
        renderer_local_storage: LegacySourceDomainCoverageIdentity,
        live_config_merge: LegacySourceDomainCoverageIdentity,
    ) -> Result<Self, SecretInternalError> {
        Ok(Self {
            current_provider_live_scrubbable,
            process_environment,
            windows_registry_current_user,
            windows_registry_local_machine,
            shell_startup_file,
            common_config_json,
            common_config_backup,
            common_config_migrated,
            common_config_sqlite,
            renderer_local_storage,
            live_config_merge,
        })
    }

    pub(crate) fn all_domains_absent(&self) -> bool {
        [
            &self.current_provider_live_scrubbable,
            &self.process_environment,
            &self.windows_registry_current_user,
            &self.windows_registry_local_machine,
            &self.shell_startup_file,
            &self.common_config_json,
            &self.common_config_backup,
            &self.common_config_migrated,
            &self.common_config_sqlite,
            &self.renderer_local_storage,
            &self.live_config_merge,
        ]
        .into_iter()
        .all(|domain| {
            matches!(domain.presence, LegacySourceDomainPresence::Absent)
                && domain.source_count == 0
        })
    }
}

pub(crate) struct CompleteLegacySourceInventoryAuthority {
    inventory_revision: LegacySourceInventoryRevision,
    coverage_identity: CompleteLegacySourceCoverageIdentity,
    current_scrubbable: CurrentLegacySourceExpectations,
    adjacent_blocked: Vec<AdjacentBlockedLegacySourceObservation>,
}

impl CompleteLegacySourceInventoryAuthority {
    fn checked_from_bridge(
        inventory_revision: LegacySourceInventoryRevision,
        coverage_identity: CompleteLegacySourceCoverageIdentity,
        current_scrubbable: CurrentLegacySourceExpectations,
        adjacent_blocked: Vec<AdjacentBlockedLegacySourceObservation>,
    ) -> Result<Self, SecretInternalError> {
        Ok(Self {
            inventory_revision,
            coverage_identity,
            current_scrubbable,
            adjacent_blocked,
        })
    }

    // Scanner-allowlisted only from
    // LegacySourceCoverageReceipt::checked_from_complete_inventory_authority.
    // The bridge never returns this authority to any sibling module.
    pub(crate) fn into_secret_checked_parts(
        self,
    ) -> (
        LegacySourceInventoryRevision,
        CompleteLegacySourceCoverageIdentity,
        CurrentLegacySourceExpectations,
        Vec<AdjacentBlockedLegacySourceObservation>,
    ) {
        (
            self.inventory_revision,
            self.coverage_identity,
            self.current_scrubbable,
            self.adjacent_blocked,
        )
    }
}

enum FreshLegacySourceInventoryTarget<'a> {
    Startup,
    OwnerSummary(&'a ExistingSecretOwnerToken),
    Capture(&'a ExistingSecretOwnerToken),
    ProviderDelete {
        owner: &'a ExistingSecretOwnerToken,
        provider_row_revision: &'a ProviderRowRevision,
    },
}

struct CodexLegacySourceStructuralInventoryPorts<'a> {
    _borrowed_main_integration_authorities: std::marker::PhantomData<&'a mut ()>,
}

struct FreshLegacySourceDomainInventory {
    structural_generation: u64,
    source_count: u32,
}

impl FreshLegacySourceDomainInventory {
    fn into_coverage_identity(
        self,
    ) -> Result<LegacySourceDomainCoverageIdentity, SecretInternalError> {
        let presence = if self.source_count == 0 {
            LegacySourceDomainPresence::Absent
        } else {
            LegacySourceDomainPresence::Present
        };
        LegacySourceDomainCoverageIdentity::checked_from_structural_inventory(
            LegacySourceInventoryRevision::checked_from_structural_generation(
                self.structural_generation,
            )?,
            presence,
            self.source_count,
        )
    }
}

struct FreshCompleteLegacySourceInventory {
    inventory_generation: u64,
    current_provider_live_scrubbable: FreshLegacySourceDomainInventory,
    process_environment: FreshLegacySourceDomainInventory,
    windows_registry_current_user: FreshLegacySourceDomainInventory,
    windows_registry_local_machine: FreshLegacySourceDomainInventory,
    shell_startup_file: FreshLegacySourceDomainInventory,
    common_config_json: FreshLegacySourceDomainInventory,
    common_config_backup: FreshLegacySourceDomainInventory,
    common_config_migrated: FreshLegacySourceDomainInventory,
    common_config_sqlite: FreshLegacySourceDomainInventory,
    renderer_local_storage: FreshLegacySourceDomainInventory,
    live_config_merge: FreshLegacySourceDomainInventory,
    current_scrubbable: CurrentLegacySourceExpectations,
    adjacent_blocked: Vec<AdjacentBlockedLegacySourceObservation>,
}

pub(crate) struct CodexLegacySourceInventoryBridge<'a> {
    ports: CodexLegacySourceStructuralInventoryPorts<'a>,
}

impl<'a> CodexLegacySourceInventoryBridge<'a> {
    // The factory binds the existing AppState/DB, Provider/live configuration,
    // process/OS/file/common-config and renderer-storage structural adapters.
    // No caller may inject a source list, path, locator, value or digest.
    pub(crate) fn from_app_state(
        state: &'a crate::store::AppState,
    ) -> Result<Self, SecretInternalError> {
        let _ = state;
        todo!("main-integration factory over the fixed eleven structural adapters")
    }

    pub(crate) fn fresh_startup_coverage(
        &mut self,
    ) -> Result<LegacySourceCoverageReceipt, SecretInternalError> {
        self.fresh_complete_coverage(FreshLegacySourceInventoryTarget::Startup)
    }

    pub(crate) fn fresh_owner_summary_coverage(
        &mut self,
        owner: &ExistingSecretOwnerToken,
    ) -> Result<LegacySourceCoverageReceipt, SecretInternalError> {
        self.fresh_complete_coverage(
            FreshLegacySourceInventoryTarget::OwnerSummary(owner),
        )
    }

    pub(crate) fn fresh_capture_coverage(
        &mut self,
        owner: &ExistingSecretOwnerToken,
    ) -> Result<LegacySourceCoverageReceipt, SecretInternalError> {
        self.fresh_complete_coverage(
            FreshLegacySourceInventoryTarget::Capture(owner),
        )
    }

    pub(crate) fn fresh_provider_delete_coverage(
        &mut self,
        owner: &ExistingSecretOwnerToken,
        provider_row_revision: &ProviderRowRevision,
    ) -> Result<LegacySourceCoverageReceipt, SecretInternalError> {
        self.fresh_complete_coverage(
            FreshLegacySourceInventoryTarget::ProviderDelete {
                owner,
                provider_row_revision,
            },
        )
    }

    fn fresh_complete_coverage(
        &mut self,
        target: FreshLegacySourceInventoryTarget<'_>,
    ) -> Result<LegacySourceCoverageReceipt, SecretInternalError> {
        // `collect_complete_inventory_authority` performs one fresh read of
        // all eleven fixed domains. It privately constructs the complete
        // identity and authority; no partial Vec/map is an accepted input.
        let authority = self.collect_complete_inventory_authority(target)?;
        LegacySourceCoverageReceipt::checked_from_complete_inventory_authority(
            authority,
        )
    }

    fn collect_complete_inventory_authority(
        &mut self,
        target: FreshLegacySourceInventoryTarget<'_>,
    ) -> Result<CompleteLegacySourceInventoryAuthority, SecretInternalError> {
        let inventory: FreshCompleteLegacySourceInventory = {
            let _ = (&mut self.ports, target);
            todo!("one fresh complete pass over the fixed eleven structural adapters with before/after generation fencing and drift rejection; output has only structural generations/counts, typed current expectations and category-only adjacent observations")
        };
        let FreshCompleteLegacySourceInventory {
            inventory_generation,
            current_provider_live_scrubbable,
            process_environment,
            windows_registry_current_user,
            windows_registry_local_machine,
            shell_startup_file,
            common_config_json,
            common_config_backup,
            common_config_migrated,
            common_config_sqlite,
            renderer_local_storage,
            live_config_merge,
            current_scrubbable,
            adjacent_blocked,
        } = inventory;
        let inventory_revision =
            LegacySourceInventoryRevision::checked_from_structural_generation(
                inventory_generation,
            )?;
        let coverage_identity =
            CompleteLegacySourceCoverageIdentity::checked_exact_eleven_domains(
                current_provider_live_scrubbable.into_coverage_identity()?,
                process_environment.into_coverage_identity()?,
                windows_registry_current_user.into_coverage_identity()?,
                windows_registry_local_machine.into_coverage_identity()?,
                shell_startup_file.into_coverage_identity()?,
                common_config_json.into_coverage_identity()?,
                common_config_backup.into_coverage_identity()?,
                common_config_migrated.into_coverage_identity()?,
                common_config_sqlite.into_coverage_identity()?,
                renderer_local_storage.into_coverage_identity()?,
                live_config_merge.into_coverage_identity()?,
            )?;
        CompleteLegacySourceInventoryAuthority::checked_from_bridge(
            inventory_revision,
            coverage_identity,
            current_scrubbable,
            adjacent_blocked,
        )
    }
}

// Opaque, no-value inventory receipt owned by the private child module
// crate::secret::legacy_source_coverage and re-exported pub(crate) by
// crate::secret. Store/Provider/other secret siblings can name, move and
// consume it but cannot access its fields. It implements no
// Clone/Serialize/Deserialize/Debug/Default.
// The exact fields are one non-value-derived inventory revision, one complete
// eleven-domain identity, current expectations and adjacent observations.
// Only current_scrubbable may retain exact current LegacySourceRef
// expectations, including its typed non-value-derived LegacySourceLocationId;
// no raw locator/path/value/value-derived digest is retained. adjacent_blocked
// retains category/state observations only and can never authorize
// parse/read/compare/scrub.
pub(crate) struct LegacySourceCoverageReceipt {
    inventory_revision: LegacySourceInventoryRevision,
    coverage_identity: CompleteLegacySourceCoverageIdentity,
    current_scrubbable: CurrentLegacySourceExpectations,
    adjacent_blocked: Vec<AdjacentBlockedLegacySourceObservation>,
}

impl LegacySourceCoverageReceipt {
    fn validate_complete_parts(
        inventory_revision: &LegacySourceInventoryRevision,
        coverage_identity: &CompleteLegacySourceCoverageIdentity,
        current_scrubbable: &CurrentLegacySourceExpectations,
        adjacent_blocked: &[AdjacentBlockedLegacySourceObservation],
    ) -> Result<(), SecretInternalError> {
        let _ = (
            inventory_revision,
            coverage_identity,
            current_scrubbable,
            adjacent_blocked,
        );
        todo!("current count/presence equals exact current expectations; each supplemental count/presence equals its canonical category-only observations; inventory/domain revisions are positive structural generations")
    }

    pub(crate) fn checked_from_complete_inventory_authority(
        authority: CompleteLegacySourceInventoryAuthority,
    ) -> Result<Self, SecretInternalError> {
        let (
            inventory_revision,
            coverage_identity,
            current_scrubbable,
            adjacent_blocked,
        ) = authority.into_secret_checked_parts();
        Self::validate_complete_parts(
            &inventory_revision,
            &coverage_identity,
            &current_scrubbable,
            &adjacent_blocked,
        )?;
        Ok(Self {
            inventory_revision,
            coverage_identity,
            current_scrubbable,
            adjacent_blocked,
        })
    }

    pub(crate) fn assert_complete_clear(
        &self,
    ) -> Result<(), SecretInternalError> {
        if !self.coverage_identity.all_domains_absent()
            || !self.current_scrubbable.as_slice().is_empty()
            || !self.adjacent_blocked.is_empty()
        {
            return Err(todo!("checked internal invariant error"));
        }
        Ok(())
    }

    pub(crate) fn assert_complete(
        &self,
    ) -> Result<(), SecretInternalError> {
        Self::validate_complete_parts(
            &self.inventory_revision,
            &self.coverage_identity,
            &self.current_scrubbable,
            &self.adjacent_blocked,
        )
    }

    pub(crate) fn assert_complete_blocking(
        &self,
    ) -> Result<(), SecretInternalError> {
        if self.coverage_identity.all_domains_absent()
            || (self.current_scrubbable.as_slice().is_empty()
                && self.adjacent_blocked.is_empty())
        {
            return Err(todo!("checked internal invariant error"));
        }
        Ok(())
    }

    pub(crate) fn assert_same_complete_coverage_as(
        &self,
        expected: &LegacySourceCoverageReceipt,
    ) -> Result<(), SecretInternalError> {
        if self.inventory_revision != expected.inventory_revision
            || self.coverage_identity != expected.coverage_identity
            || self.current_scrubbable != expected.current_scrubbable
            || self.adjacent_blocked != expected.adjacent_blocked
        {
            return Err(todo!("checked stale-coverage error"));
        }
        Ok(())
    }

}

impl LegacySourceCoverageView {
    pub(crate) fn checked_from_coverage_receipt(
        receipt: &LegacySourceCoverageReceipt,
    ) -> Result<Self, SecretInternalError> {
        let _ = receipt;
        todo!("derive, never accept, exact clear/blocking state, current category/count and adjacent category-only observation/count projection; no location id, path, value or value digest")
    }
}

// LegacySourceInventoryRevision, CompleteLegacySourceCoverageIdentity,
// CompleteLegacySourceInventoryAuthority, CodexLegacySourceInventoryBridge
// and LegacySourceCoverageReceipt implement no Clone/Serialize/Deserialize/
// Debug/Default. The revision constructors accept structural-generation
// counters only and reject zero; source bytes and value-derived hashes are not
// in their type surface. A LegacySourceRef is current-scrubbable; an
// AdjacentBlockedLegacySourceObservation is supplemental and never one.
wire_enum!(SecretAuditAction {
    CaptureCandidate, DiscardCandidate, ActivateCandidate, Validate,
    RotateCandidate, Lock, Unlock, Delete, Revoke, CheckReadiness,
    PrepareApply, ConfirmHardware, ResolveApply, MigrateLegacy,
    ReconcileLegacy, ReconcileRecovery, RetryCleanup, CancelConfirmation
});
wire_enum!(SecretAuditOutcome { Success, Blocked, Failed, Partial, Recovered });
wire_enum!(SecretEffect {
    None, CandidateStaged, BindingChanged, PolicyChanged, RecordRevoked,
    TargetWriterInvoked, CleanupPending
});
#[derive(Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SecretUserAction {
    None,
    RetryCapture,
    RetryRotation,
    RetryProxyRequest,
    RetryUsageProbe,
    RetryCodingPlanUsageProbe,
    RetryModelFetch,
    UnlockFyAgent,
    UnlockBackend,
    RequestPermission,
    CaptureReplacement,
    ChooseBackend,
    ConfirmDevice,
    RefreshSummary,
    RefreshDeleteImpact,
    RefreshRecoveryImpact,
    ReopenChangePlan,
    ResolveLegacyConflict,
    DiscardCandidate,
    CompleteRecovery,
    ResumeStagedImportCutover,
    ReconnectDevice,
    OpenBackendSettings,
    ContactAdministrator,
}

// Internal-only discriminator used by the checked error/issue factory when
// one stable code has more than one executable remediation. It is never wire
// data; the renderer receives the already-derived exact SecretUserAction.
pub(in crate::secret) enum SecretActionCondition {
    General,
    DeleteReadiness,
    RecoveryReadiness,
    CaptureFreshOperation,
    RotationFreshOperation,
    CandidateDiscardFreshOperation,
    ApplyOrActivationPlan,
    StagedImportResume,
    ValidationFreshOperation,
    RuntimeFreshOperation,
    CaptureBackendSelection,
    CandidateTerminalCleanupPending,
}

pub(in crate::secret) enum SecretDeleteReadinessDrift {
    Dependency,
    Record,
}

pub(in crate::secret) enum SecretTerminalOperationContext {
    Summary,
    Capture(BeginCaptureIntent),
    Rotation,
    CandidateDiscard,
    CandidateTerminalCleanupPending,
    Delete,
    Recovery,
    ApplyOrActivation,
    StagedImport,
    Validation,
    Runtime(FixedRuntimeConsumer),
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SecretCommandName {
    ListSecretSummaries,
    ListSecretBackendOptions,
    BeginSecretCapture,
    RotateSecret,
    ListSecretCandidates,
    DiscardSecretCandidate,
    SetSecretLocked,
    GetSecretDeleteImpact,
    DeleteSecret,
    GetSecretCleanupImpact,
    RetrySecretCleanup,
    ValidateSecret,
    CheckSecretApplyReadiness,
    MigrateLegacyCodexSecrets,
    ListSecretAudit,
}

pub enum SecretCaptureFlowSelection {
    RegisteredBackendOption,
}

pub enum SecretFixedRuntimeEntry {
    ProxyRequest,
    UsageProbe,
    CodingPlanUsageProbe,
    ModelFetch,
}

pub enum SecretOperationIdPolicy {
    ServerGeneratedNew,
}

pub enum SecretExternalGuidance {
    UnlockBackend,
    GrantPermission,
    ReconnectDevice,
    OpenBackendSettings,
    OpenChangePlan,
    ContactAdministrator,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SecretMainIntegrationCommandName {
    ResumeStagedImportCutover,
}

pub enum SecretPostGuidanceDestination {
    None,
    RefreshSummary(SecretCommandName),
}

pub enum SecretActionDestination {
    None,
    SecretCommand(SecretCommandName),
    FreshSecretCommand {
        command: SecretCommandName,
        operation_id_policy: SecretOperationIdPolicy,
    },
    SecretCaptureFlow {
        intent: BeginCaptureIntent,
        list_options: SecretCommandName,
        selection: SecretCaptureFlowSelection,
        begin_capture: SecretCommandName,
        operation_id_policy: SecretOperationIdPolicy,
    },
    FixedRuntimeFlow {
        entry: SecretFixedRuntimeEntry,
        operation_id_policy: SecretOperationIdPolicy,
    },
    MainIntegrationCommand {
        command: SecretMainIntegrationCommandName,
        operation_id_policy: SecretOperationIdPolicy,
    },
    SecretCommandFlow {
        commands: [SecretCommandName; 2],
        operation_id_policy: SecretOperationIdPolicy,
    },
    NativeConfirmationContinuation,
    ExternalGuidance {
        guidance: SecretExternalGuidance,
        after: SecretPostGuidanceDestination,
    },
}

pub fn secret_action_destination(action: SecretUserAction) -> SecretActionDestination {
    use SecretActionDestination as Destination;
    use SecretCommandName as Command;
    use SecretExternalGuidance as Guidance;
    use SecretFixedRuntimeEntry as Runtime;
    use SecretMainIntegrationCommandName as MainCommand;
    use SecretPostGuidanceDestination as After;
    match action {
        SecretUserAction::None => Destination::None,
        SecretUserAction::RetryCapture => Destination::SecretCaptureFlow {
            intent: BeginCaptureIntent::NewBinding,
            list_options: Command::ListSecretBackendOptions,
            selection: SecretCaptureFlowSelection::RegisteredBackendOption,
            begin_capture: Command::BeginSecretCapture,
            operation_id_policy: SecretOperationIdPolicy::ServerGeneratedNew,
        },
        SecretUserAction::RetryRotation => {
            Destination::FreshSecretCommand {
                command: Command::RotateSecret,
                operation_id_policy: SecretOperationIdPolicy::ServerGeneratedNew,
            }
        }
        SecretUserAction::RetryProxyRequest => {
            Destination::FixedRuntimeFlow {
                entry: Runtime::ProxyRequest,
                operation_id_policy: SecretOperationIdPolicy::ServerGeneratedNew,
            }
        }
        SecretUserAction::RetryUsageProbe => {
            Destination::FixedRuntimeFlow {
                entry: Runtime::UsageProbe,
                operation_id_policy: SecretOperationIdPolicy::ServerGeneratedNew,
            }
        }
        SecretUserAction::RetryCodingPlanUsageProbe => {
            Destination::FixedRuntimeFlow {
                entry: Runtime::CodingPlanUsageProbe,
                operation_id_policy: SecretOperationIdPolicy::ServerGeneratedNew,
            }
        }
        SecretUserAction::RetryModelFetch => {
            Destination::FixedRuntimeFlow {
                entry: Runtime::ModelFetch,
                operation_id_policy: SecretOperationIdPolicy::ServerGeneratedNew,
            }
        }
        SecretUserAction::UnlockFyAgent => {
            Destination::SecretCommand(Command::SetSecretLocked)
        }
        SecretUserAction::UnlockBackend => {
            Destination::ExternalGuidance {
                guidance: Guidance::UnlockBackend,
                after: After::RefreshSummary(Command::ListSecretSummaries),
            }
        }
        SecretUserAction::RequestPermission => {
            Destination::ExternalGuidance {
                guidance: Guidance::GrantPermission,
                after: After::RefreshSummary(Command::ListSecretSummaries),
            }
        }
        SecretUserAction::CaptureReplacement => Destination::SecretCaptureFlow {
            intent: BeginCaptureIntent::ReplaceBinding,
            list_options: Command::ListSecretBackendOptions,
            selection: SecretCaptureFlowSelection::RegisteredBackendOption,
            begin_capture: Command::BeginSecretCapture,
            operation_id_policy: SecretOperationIdPolicy::ServerGeneratedNew,
        },
        SecretUserAction::ChooseBackend => Destination::SecretCaptureFlow {
            intent: BeginCaptureIntent::NewBinding,
            list_options: Command::ListSecretBackendOptions,
            selection: SecretCaptureFlowSelection::RegisteredBackendOption,
            begin_capture: Command::BeginSecretCapture,
            operation_id_policy: SecretOperationIdPolicy::ServerGeneratedNew,
        },
        SecretUserAction::ConfirmDevice => Destination::NativeConfirmationContinuation,
        SecretUserAction::RefreshSummary => {
            Destination::SecretCommand(Command::ListSecretSummaries)
        }
        SecretUserAction::RefreshDeleteImpact => {
            Destination::FreshSecretCommand {
                command: Command::GetSecretDeleteImpact,
                operation_id_policy: SecretOperationIdPolicy::ServerGeneratedNew,
            }
        }
        SecretUserAction::RefreshRecoveryImpact => {
            Destination::FreshSecretCommand {
                command: Command::GetSecretCleanupImpact,
                operation_id_policy: SecretOperationIdPolicy::ServerGeneratedNew,
            }
        }
        SecretUserAction::ReopenChangePlan => {
            Destination::ExternalGuidance {
                guidance: Guidance::OpenChangePlan,
                after: After::None,
            }
        }
        SecretUserAction::ResolveLegacyConflict => Destination::SecretCaptureFlow {
            intent: BeginCaptureIntent::LegacyReconcile,
            list_options: Command::ListSecretBackendOptions,
            selection: SecretCaptureFlowSelection::RegisteredBackendOption,
            begin_capture: Command::BeginSecretCapture,
            operation_id_policy: SecretOperationIdPolicy::ServerGeneratedNew,
        },
        SecretUserAction::DiscardCandidate => {
            Destination::FreshSecretCommand {
                command: Command::DiscardSecretCandidate,
                operation_id_policy: SecretOperationIdPolicy::ServerGeneratedNew,
            }
        }
        SecretUserAction::CompleteRecovery => Destination::SecretCommandFlow {
            commands: [
                Command::GetSecretCleanupImpact,
                Command::RetrySecretCleanup,
            ],
            operation_id_policy: SecretOperationIdPolicy::ServerGeneratedNew,
        },
        SecretUserAction::ResumeStagedImportCutover => {
            Destination::MainIntegrationCommand {
                command: MainCommand::ResumeStagedImportCutover,
                operation_id_policy: SecretOperationIdPolicy::ServerGeneratedNew,
            }
        }
        SecretUserAction::ReconnectDevice => {
            Destination::ExternalGuidance {
                guidance: Guidance::ReconnectDevice,
                after: After::RefreshSummary(Command::ListSecretSummaries),
            }
        }
        SecretUserAction::OpenBackendSettings => {
            Destination::ExternalGuidance {
                guidance: Guidance::OpenBackendSettings,
                after: After::RefreshSummary(Command::ListSecretSummaries),
            }
        }
        SecretUserAction::ContactAdministrator => {
            Destination::ExternalGuidance {
                guidance: Guidance::ContactAdministrator,
                after: After::RefreshSummary(Command::ListSecretSummaries),
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SecretErrorCode {
    SecretRequestInvalid,
    SecretRefInvalid,
    SecretOwnerKindUnsupported,
    SecretOwnerNamespaceUnsupported,
    SecretOwnerNotFound,
    SecretOwnerConflict,
    SecretOperationBusy,
    SecretUnsupportedPurpose,
    SecretConsumerUnsupported,
    SecretInputCancelled,
    SecretInputInvalid,
    SecretCandidateNotFound,
    SecretCandidateExpired,
    SecretCandidateConsumed,
    SecretChangePlanRequired,
    SecretChangePlanInvalid,
    SecretChangePlanStale,
    SecretMigrationRequired,
    SecretLegacySourceInvalid,
    SecretLegacyConflict,
    SecretLegacyComparisonPending,
    SecretMigrationFailed,
    SecretMissing,
    SecretLocked,
    SecretPermissionDenied,
    SecretBackendUnavailable,
    SecretStale,
    SecretRevoked,
    SecretConfirmationRequired,
    SecretConfirmationCancelled,
    SecretConfirmationExpired,
    SecretConfirmationReplayed,
    SecretDeviceMismatch,
    SecretWriteFailed,
    SecretReadFailed,
    SecretDeleteFailed,
    SecretVerifyFailed,
    SecretProjectionForbidden,
    SecretDependencyChanged,
    SecretRecordChanged,
    SecretBackendChanged,
    SecretCapabilityExpired,
    SecretCapabilityConsumed,
    SecretRecoveryNotFound,
    SecretRecoveryChanged,
    SecretOperationRecoveryRequired,
    SecretInternal,
}

impl TryFrom<SecretConsumer> for SecretRuntimeConsumer {
    type Error = SecretErrorCode;

    fn try_from(value: SecretConsumer) -> Result<Self, Self::Error> {
        match value {
            SecretConsumer::ChangePlanApply => Ok(Self::ChangePlanApply),
            SecretConsumer::ProxyRequest => Ok(Self::ProxyRequest),
            SecretConsumer::UsageProbe => Ok(Self::UsageProbe),
            SecretConsumer::CodingPlanUsageProbe => Ok(Self::CodingPlanUsageProbe),
            SecretConsumer::ModelFetch => Ok(Self::ModelFetch),
            SecretConsumer::ProviderTerminal => {
                Err(SecretErrorCode::SecretConsumerUnsupported)
            }
        }
    }
}

impl TryFrom<ApplyTargetSink> for SecretRuntimeSink {
    type Error = SecretErrorCode;

    fn try_from(value: ApplyTargetSink) -> Result<Self, Self::Error> {
        match value {
            ApplyTargetSink::ProcessMemory => Ok(Self::ProcessMemory),
            ApplyTargetSink::ExternalConfigFile => Ok(Self::ExternalConfigFile),
            ApplyTargetSink::ChildProcessEnvironment => {
                Err(SecretErrorCode::SecretProjectionForbidden)
            }
        }
    }
}

fn validate_change_plan_apply_route(
    consumer: SecretConsumer,
    sink: ApplyTargetSink,
) -> Result<
    (SecretChangePlanApplyConsumer, SecretChangePlanApplySink),
    SecretErrorCode,
> {
    match (
        SecretRuntimeConsumer::try_from(consumer)?,
        SecretRuntimeSink::try_from(sink)?,
    ) {
        (
            SecretRuntimeConsumer::ChangePlanApply,
            SecretRuntimeSink::ExternalConfigFile,
        ) => Ok((
            SecretChangePlanApplyConsumer::ChangePlanApply,
            SecretChangePlanApplySink::ExternalConfigFile,
        )),
        (SecretRuntimeConsumer::ChangePlanApply, _) => {
            Err(SecretErrorCode::SecretProjectionForbidden)
        }
        _ => Err(SecretErrorCode::SecretConsumerUnsupported),
    }
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SecretOwner {
    pub kind: SecretOwnerKind,
    pub namespace: SecretOwnerNamespace,
    pub owner_id: OwnerId,
    pub slot: SecretSlot,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SecretDeviceDisplay {
    pub display_name: SafeDisplayText,
    pub device_class: SecretDeviceClass,
    pub transport: SecretDeviceTransport,
}

wire_enum!(SecretDeviceClass { OsAccount, SecurityKey, SecureElement, Unknown });
wire_enum!(SecretDeviceTransport { Platform, Usb, Nfc, Ble, Unknown });

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SecretBackendInstanceViewRepr {
    kind: SecretBackendKind,
    instance_id: SecretBackendInstanceId,
    generation: SecretBackendGeneration,
    availability: SecretBackendAvailability,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_absent_only"
    )]
    device: Option<SecretDeviceDisplay>,
}

#[derive(Clone, PartialEq, Eq)]
pub struct SecretBackendInstanceView(SecretBackendInstanceViewRepr);

impl SecretBackendInstanceView {
    fn validate_repr(
        repr: SecretBackendInstanceViewRepr,
    ) -> Result<Self, WireValidationError> {
        let valid_device = match (&repr.kind, repr.device.as_ref()) {
            (SecretBackendKind::Hardware, Some(device)) => {
                device.device_class != SecretDeviceClass::OsAccount
                    && device.transport != SecretDeviceTransport::Platform
            }
            (SecretBackendKind::Hardware, None) => false,
            (SecretBackendKind::OsKeyring, Some(device)) => {
                device.device_class == SecretDeviceClass::OsAccount
                    && device.transport == SecretDeviceTransport::Platform
            }
            (SecretBackendKind::OsKeyring, None) => true,
        };
        if !valid_device {
            return Err(WireValidationError("invalid backend device tuple"));
        }
        Ok(Self(repr))
    }

    // Only crate::secret::backend's registered-instance factory calls this.
    // Callers cannot submit or construct an instance identity tuple.
    pub(in crate::secret) fn try_registered(
        kind: SecretBackendKind,
        instance_id: SecretBackendInstanceId,
        generation: SecretBackendGeneration,
        availability: SecretBackendAvailability,
        device: Option<SecretDeviceDisplay>,
    ) -> Result<Self, SecretInternalError> {
        Self::validate_repr(SecretBackendInstanceViewRepr {
            kind,
            instance_id,
            generation,
            availability,
            device,
        })
        .map_err(|_| SecretInternalError::input_invalid())
    }

    pub fn kind(&self) -> SecretBackendKind { self.0.kind }
    pub fn instance_id(&self) -> &SecretBackendInstanceId { &self.0.instance_id }
    pub fn generation(&self) -> SecretBackendGeneration { self.0.generation }
    pub fn availability(&self) -> SecretBackendAvailability { self.0.availability }
    pub fn device(&self) -> Option<&SecretDeviceDisplay> { self.0.device.as_ref() }
}

impl Serialize for SecretBackendInstanceView {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for SecretBackendInstanceView {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::validate_repr(SecretBackendInstanceViewRepr::deserialize(deserializer)?)
            .map_err(de::Error::custom)
    }
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SecretOperationConfirmationCapabilities {
    pub capture_verify: PhysicalConfirmation,
    pub validate: PhysicalConfirmation,
    pub resolve_for_apply: PhysicalConfirmation,
    pub delete: PhysicalConfirmation,
    pub revoke: PhysicalConfirmation,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SecretRecordCapabilitiesRepr {
    schema_version: SchemaVersionV1,
    capability_revision: CapabilityRevision,
    backend_kind: SecretBackendKind,
    backend_instance_id: SecretBackendInstanceId,
    backend_generation: SecretBackendGeneration,
    device_binding_generation: DeviceBindingGeneration,
    device_binding: DeviceBinding,
    storage_residency: StorageResidency,
    operation_confirmation: SecretOperationConfirmationCapabilities,
    allowed_consumers: Vec<SecretRuntimeConsumer>,
    allowed_sinks: Vec<SecretRuntimeSink>,
    persistent_target_projection: bool,
    central_revocation: bool,
    revocation_observation: BackendRevocationObservationCapability,
    silent_fallback: AlwaysFalse,
}

#[derive(Clone, PartialEq, Eq)]
pub struct SecretRecordCapabilities(SecretRecordCapabilitiesRepr);

wire_enum!(BackendRevocationObservationCapability {
    Unsupported, SourceAndTime
});

fn runtime_consumer_rank(value: SecretRuntimeConsumer) -> u8 {
    match value {
        SecretRuntimeConsumer::ChangePlanApply => 0,
        SecretRuntimeConsumer::ProxyRequest => 1,
        SecretRuntimeConsumer::UsageProbe => 2,
        SecretRuntimeConsumer::CodingPlanUsageProbe => 3,
        SecretRuntimeConsumer::ModelFetch => 4,
    }
}

fn runtime_sink_rank(value: SecretRuntimeSink) -> u8 {
    match value {
        SecretRuntimeSink::ProcessMemory => 0,
        SecretRuntimeSink::ExternalConfigFile => 1,
    }
}

impl SecretRecordCapabilities {
    fn validate_repr(
        repr: SecretRecordCapabilitiesRepr,
    ) -> Result<Self, WireValidationError> {
        let consumers_sorted = repr.allowed_consumers.windows(2).all(|pair| {
            runtime_consumer_rank(pair[0]) < runtime_consumer_rank(pair[1])
        });
        let sinks_sorted = repr.allowed_sinks.windows(2).all(|pair| {
            runtime_sink_rank(pair[0]) < runtime_sink_rank(pair[1])
        });
        let change_plan = repr
            .allowed_consumers
            .contains(&SecretRuntimeConsumer::ChangePlanApply);
        let memory_consumers = repr.allowed_consumers.iter().any(|consumer| {
            matches!(
                consumer,
                SecretRuntimeConsumer::ProxyRequest
                    | SecretRuntimeConsumer::UsageProbe
                    | SecretRuntimeConsumer::CodingPlanUsageProbe
                    | SecretRuntimeConsumer::ModelFetch
            )
        });
        let process_memory = repr
            .allowed_sinks
            .contains(&SecretRuntimeSink::ProcessMemory);
        let external_config = repr
            .allowed_sinks
            .contains(&SecretRuntimeSink::ExternalConfigFile);
        let os_all_confirmations_never = [
            repr.operation_confirmation.capture_verify,
            repr.operation_confirmation.validate,
            repr.operation_confirmation.resolve_for_apply,
            repr.operation_confirmation.delete,
            repr.operation_confirmation.revoke,
        ]
        .into_iter()
        .all(|confirmation| confirmation == PhysicalConfirmation::Never);
        let os_all_consumers = repr.allowed_consumers.as_slice() == [
            SecretRuntimeConsumer::ChangePlanApply,
            SecretRuntimeConsumer::ProxyRequest,
            SecretRuntimeConsumer::UsageProbe,
            SecretRuntimeConsumer::CodingPlanUsageProbe,
            SecretRuntimeConsumer::ModelFetch,
        ];
        let os_all_sinks = repr.allowed_sinks.as_slice() == [
            SecretRuntimeSink::ProcessMemory,
            SecretRuntimeSink::ExternalConfigFile,
        ];
        let backend_matrix = match repr.backend_kind {
            SecretBackendKind::OsKeyring => {
                repr.device_binding == DeviceBinding::HostUser
                    && repr.storage_residency == StorageResidency::OsProtectedStore
                    && !repr.central_revocation
                    && repr.revocation_observation
                        == BackendRevocationObservationCapability::Unsupported
                    && os_all_confirmations_never
                    && os_all_consumers
                    && os_all_sinks
                    && repr.persistent_target_projection
            }
            SecretBackendKind::Hardware => {
                repr.device_binding == DeviceBinding::HardwareDevice
                    && repr.storage_residency == StorageResidency::HardwareOnly
            }
        };
        if !consumers_sorted
            || !sinks_sorted
            || repr.central_revocation
                != (repr.revocation_observation
                    == BackendRevocationObservationCapability::SourceAndTime)
            || change_plan != external_config
            || change_plan != repr.persistent_target_projection
            || memory_consumers != process_memory
            || !backend_matrix
        {
            return Err(WireValidationError("invalid record capability matrix"));
        }
        Ok(Self(repr))
    }

    // Only crate::secret::backend calls this constructor. It copies identity
    // from the registered backend instead of accepting caller-supplied ids.
    pub(in crate::secret) fn try_new(
        backend: &SecretBackendInstanceView,
        capability_revision: CapabilityRevision,
        device_binding_generation: DeviceBindingGeneration,
        device_binding: DeviceBinding,
        storage_residency: StorageResidency,
        operation_confirmation: SecretOperationConfirmationCapabilities,
        allowed_consumers: Vec<SecretRuntimeConsumer>,
        allowed_sinks: Vec<SecretRuntimeSink>,
        persistent_target_projection: bool,
        central_revocation: bool,
        revocation_observation: BackendRevocationObservationCapability,
    ) -> Result<Self, SecretInternalError> {
        let observation_matches = central_revocation
            == matches!(
                revocation_observation,
                BackendRevocationObservationCapability::SourceAndTime
            );
        if !observation_matches
            || (backend.kind() == SecretBackendKind::OsKeyring
                && central_revocation)
        {
            return Err(SecretInternalError::input_invalid());
        }
        Self::validate_repr(SecretRecordCapabilitiesRepr {
            schema_version: SchemaVersionV1,
            capability_revision,
            backend_kind: backend.kind(),
            backend_instance_id: backend.instance_id().clone(),
            backend_generation: backend.generation(),
            device_binding_generation,
            device_binding,
            storage_residency,
            operation_confirmation,
            allowed_consumers,
            allowed_sinks,
            persistent_target_projection,
            central_revocation,
            revocation_observation,
            silent_fallback: AlwaysFalse,
        })
        .map_err(|_| SecretInternalError::input_invalid())
    }

    pub fn backend_identity(
        &self,
    ) -> (&SecretBackendInstanceId, SecretBackendGeneration) {
        (&self.0.backend_instance_id, self.0.backend_generation)
    }

    pub fn allowed_consumers(&self) -> &[SecretRuntimeConsumer] {
        &self.0.allowed_consumers
    }

    pub fn allowed_sinks(&self) -> &[SecretRuntimeSink] {
        &self.0.allowed_sinks
    }

    pub fn operation_confirmation(
        &self,
    ) -> &SecretOperationConfirmationCapabilities {
        &self.0.operation_confirmation
    }

    pub(in crate::secret) fn central_revocation(&self) -> bool {
        self.0.central_revocation
    }

    pub fn capability_revision(&self) -> CapabilityRevision {
        self.0.capability_revision
    }

    pub fn device_binding_generation(&self) -> DeviceBindingGeneration {
        self.0.device_binding_generation
    }

    pub fn persistent_target_projection(&self) -> bool {
        self.0.persistent_target_projection
    }
}

impl Serialize for SecretRecordCapabilities {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for SecretRecordCapabilities {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let repr = SecretRecordCapabilitiesRepr::deserialize(deserializer)?;
        Self::validate_repr(repr).map_err(de::Error::custom)
    }
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SecretBindingSetCas {
    pub revision: SecretBindingSetRevision,
    pub digest: BindingSetDigest,
    pub count: u32,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SecretRecoveryCas {
    pub revision: SecretRecoveryRevision,
    pub digest: SecretRecoveryDigest,
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SecretOwnerBindingSummary {
    owner: SecretOwner,
    purpose: SecretPurpose,
    binding_revision: SecretBindingRevision,
    created_at: UtcTimestamp,
    updated_at: UtcTimestamp,
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SecretLockView {
    source: SecretLockSource,
    locked_at: UtcTimestamp,
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SecretRevocationView {
    source: SecretRevocationSource,
    revoked_at: UtcTimestamp,
}

pub(in crate::secret) struct PlatformRevocationObservation {
    source: BackendObservedRevocationSource,
    revoked_at: UtcTimestamp,
}

pub(in crate::secret) struct PlatformBackendRevocationHint {
    _private: (),
}

// Ordinary read/probe can surface this non-Clone, non-serde, non-persistable
// hint only. It has no source/time/ref getter and is not accepted by authority.
pub(crate) struct BackendRevocationHint {
    registered_backend: RegisteredBackendHandleBinding,
    device_store_instance_id: std::sync::Arc<DeviceSecretStoreInstanceId>,
    _private: (),
}

struct BackendRevocationObservationScope {
    authorization_scope: BackendAuthorizationScope,
    registered_backend: RegisteredBackendHandleBinding,
    device_store_instance_id: std::sync::Arc<DeviceSecretStoreInstanceId>,
    secret_ref: SecretRef,
    store_revision: SecretStoreRevision,
    record_revision: SecretRecordRevision,
    binding_set_cas: SecretBindingSetCas,
    backend_instance_id: SecretBackendInstanceId,
    backend_generation: SecretBackendGeneration,
    device_binding_generation: DeviceBindingGeneration,
    capability_revision: CapabilityRevision,
}

// Consuming native receipt: no Clone/Serialize/Deserialize/Debug. The raw
// platform observation cannot be persisted until the registered wrapper has
// proven SourceAndTime support plus the record's centralRevocation capability.
pub(crate) struct BackendRevocationObservation {
    scope: BackendRevocationObservationScope,
    source: BackendObservedRevocationSource,
    revoked_at: UtcTimestamp,
}

impl BackendRevocationObservation {
    fn checked_from_platform(
        backend: &BackendInstanceHandle,
        record: &BackendRecordHandle,
        capabilities: &SecretRecordCapabilities,
        authorization: ConsumedBackendAuthorization,
        raw: PlatformRevocationObservationResult,
    ) -> Result<Self, SecretInternalError> {
        backend.assert_record_identity(record)?;
        authorization.scope.require_revoke_observation()?;
        let source_and_time = matches!(
            backend.registered.platform.revocation_observation_capability(),
            BackendRevocationObservationCapability::SourceAndTime
        );
        let capability_matches = capabilities.central_revocation()
            && capabilities.backend_identity()
                == (
                    backend.registered.instance.instance_id(),
                    backend.registered.instance.generation(),
                )
            && capabilities.capability_revision() == record.capability_revision
            && capabilities.device_binding_generation()
                == record.device_binding_generation;
        let returned_generations_match = raw.backend_generation
            == record.backend_generation
            && raw.device_binding_generation
                == record.device_binding_generation;
        if !source_and_time || !capability_matches || !returned_generations_match {
            return Err(SecretInternalError::terminal_operation_failure(
                SecretSourceFreeErrorCode::DependencyChanged,
                authorization.scope.into_terminal_error_context(),
            ));
        }
        Ok(Self {
            scope: BackendRevocationObservationScope {
                authorization_scope: authorization.scope,
                registered_backend: RegisteredBackendHandleBinding::from_handle(backend),
                device_store_instance_id:
                    record.device_store_instance_id.clone(),
                secret_ref: record.secret_ref.clone(),
                store_revision: record.store_revision,
                record_revision: record.record_revision,
                binding_set_cas: record.binding_set_cas.clone(),
                backend_instance_id: record.instance_id.clone(),
                backend_generation: record.backend_generation,
                device_binding_generation: record.device_binding_generation,
                capability_revision: record.capability_revision,
            },
            source: raw.observation.source,
            revoked_at: raw.observation.revoked_at,
        })
    }
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SecretRecoveryPointer {
    recovery_id: SecretRecoveryId,
    kind: SecretRecoveryKind,
    recovery_cas: SecretRecoveryCas,
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SecretIssueView {
    code: SecretErrorCode,
    retryable: bool,
    action: SecretUserAction,
    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "deserialize_absent_only")]
    lock_source: Option<SecretLockSource>,
    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "deserialize_absent_only")]
    revocation_source: Option<SecretRevocationSource>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    backend_unavailable_reason: Option<SecretBackendUnavailableReason>,
    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "deserialize_absent_only")]
    recovery: Option<SecretRecoveryPointer>,
}

impl SecretIssueView {
    // Sole constructor in crate::secret::device_store::result. Arbitrary
    // code/action/source tuples are not accepted: the view can only project a
    // tuple already minted by SecretInternalError::checked.
    pub(super) fn checked_from_internal(error: &SecretInternalError) -> Self {
        Self {
            code: error.code,
            retryable: error.retryable,
            action: error.action,
            lock_source: error.lock_source,
            revocation_source: error.revocation_source,
            backend_unavailable_reason: error.backend_unavailable_reason,
            recovery: error.recovery.clone(),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SecretRefAggregate {
    schema_version: SchemaVersionV1,
    secret_ref: SecretRef,
    secret_ref_display: SecretRefDisplay,
    purpose: SecretPurpose,
    record_revision: SecretRecordRevision,
    binding_set_cas: SecretBindingSetCas,
    backend: SecretBackendInstanceView,
    capabilities: SecretRecordCapabilities,
    bindings: Vec<SecretOwnerBindingSummary>,
    presence: SecretPresence,
    availability: SecretStableAvailability,
    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "deserialize_absent_only")]
    lock: Option<SecretLockView>,
    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "deserialize_absent_only")]
    revocation: Option<SecretRevocationView>,
    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "deserialize_absent_only")]
    issue: Option<SecretIssueView>,
    created_at: UtcTimestamp,
    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "deserialize_absent_only")]
    rotated_at: Option<UtcTimestamp>,
    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "deserialize_absent_only")]
    last_validated_at: Option<UtcTimestamp>,
}

impl SecretRefAggregate {
    pub(super) fn checked_from_authority(
        aggregate: SecretRefAggregate,
    ) -> Result<Self, SecretInternalError> {
        todo!("ref/display/binding-set/backend-capability/presence/availability/lock/revocation/issue/timestamp matrix")
    }
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "state",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
enum OwnerBindingStateRepr {
    Bound {
        secret_ref: SecretRef,
        secret_ref_display: SecretRefDisplay,
        binding_revision: SecretBindingRevision,
    },
    Legacy {
        legacy_state: LegacyOwnerState,
        sources: Vec<LegacySourceRef>,
        source_count: u32,
        action: SecretUserAction,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        candidate_id: Option<SecretCandidateId>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        last_error: Option<SecretIssueView>,
    },
    Unbound,
}

#[derive(Clone, PartialEq, Eq)]
pub struct OwnerBindingState(OwnerBindingStateRepr);

impl OwnerBindingState {
    pub(super) fn checked_from_authority(
        repr: OwnerBindingStateRepr,
    ) -> Result<Self, SecretInternalError> {
        todo!("bound identity or exact legacy state/source/count/candidate/error/action mapping; cached state never emits Retry")
    }
}

impl Serialize for OwnerBindingState {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SecretOwnerCredentialSummary {
    schema_version: SchemaVersionV1,
    owner: SecretOwner,
    purpose: SecretPurpose,
    owner_binding_revision: SecretOwnerBindingRevision,
    binding_state: OwnerBindingState,
    legacy_source_coverage: LegacySourceCoverageView,
}

impl SecretOwnerCredentialSummary {
    pub(super) fn checked_from_authority(
        mut summary: SecretOwnerCredentialSummary,
        coverage: &LegacySourceCoverageReceipt,
    ) -> Result<Self, SecretInternalError> {
        summary.legacy_source_coverage =
            LegacySourceCoverageView::checked_from_coverage_receipt(coverage)?;
        let _ = summary;
        todo!("owner/purpose/tombstone revision, checked binding-state identity and LegacySourceCoverageView derived from this exact opaque coverage receipt")
    }
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "state",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum OwnerBindingExpectation {
    Unbound {
        owner: SecretOwner,
        owner_binding_revision: SecretOwnerBindingRevision,
    },
    Bound {
        owner: SecretOwner,
        secret_ref: SecretRef,
        owner_binding_revision: SecretOwnerBindingRevision,
        binding_revision: SecretBindingRevision,
        source_binding_set: SecretBindingSetCas,
    },
}
```

### 6.3 Candidate, readiness, migration, audit and envelope DTOs

```rust
#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SecretCandidateSummary {
    schema_version: SchemaVersionV1,
    candidate_id: SecretCandidateId,
    candidate_revision: SecretCandidateRevision,
    kind: SecretCandidateKind,
    comparison_policy: LegacyActivationComparisonPolicy,
    comparison_impact: LegacyActivationComparisonImpact,
    state: SecretCandidateState,
    secret_ref: SecretRef,
    secret_ref_display: SecretRefDisplay,
    purpose: SecretPurpose,
    record_revision: SecretRecordRevision,
    backend: SecretBackendInstanceView,
    capabilities: SecretRecordCapabilities,
    target_owners: Vec<SecretOwner>,
    expected_bindings: Vec<OwnerBindingExpectation>,
    legacy_sources_to_scrub: CurrentLegacySourceExpectations,
    created_at: UtcTimestamp,
    expires_at: UtcTimestamp,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pending_terminal_disposition: Option<CandidateTerminalState>,
    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "deserialize_absent_only")]
    issue: Option<SecretIssueView>,
}

impl SecretCandidateSummary {
    pub(super) fn checked_from_candidate_authority(
        summary: SecretCandidateSummary,
        journal: Option<&CandidateDeleteJournalRow>,
    ) -> Result<Self, SecretInternalError> {
        todo!(
            "pending disposition iff verifiedPendingPlan + nonterminal matching discard journal + OPERATION_RECOVERY_REQUIRED/discardCandidate; terminal forbids both fields"
        )
    }
}

wire_enum!(ActivationOldRecordDeleteOperation { Delete });
wire_enum!(ActivationOldRecordPostBindingState { NoBindings });
wire_enum!(ActivationOldRecordMissingReadbackOperation { Validate });
wire_enum!(ActivationOldRecordMissingReadbackScope {
    ActivationOldRecordMissingReadback
});

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum SecretActivationOldRecordDeleteExpectation {
    NotApplicable,
    DeleteAfterActivation {
        operation: ActivationOldRecordDeleteOperation,
        old_secret_ref: SecretRef,
        expected_record_revision: SecretRecordRevision,
        expected_pre_activation_binding_set: SecretBindingSetCas,
        required_post_activation_binding_state:
            ActivationOldRecordPostBindingState,
        backend_instance_id: SecretBackendInstanceId,
        backend_generation: SecretBackendGeneration,
        device_binding_generation: DeviceBindingGeneration,
        capability_revision: CapabilityRevision,
        delete_confirmation: PhysicalConfirmation,
        missing_readback_operation: ActivationOldRecordMissingReadbackOperation,
        missing_readback_scope: ActivationOldRecordMissingReadbackScope,
        missing_readback_confirmation: PhysicalConfirmation,
    },
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SecretActivationCandidateReadExpectation {
    pub operation: ActivationCandidateReadOperation,
    pub scope: ActivationCandidateReadScope,
    pub backend_instance_id: SecretBackendInstanceId,
    pub backend_generation: SecretBackendGeneration,
    pub device_binding_generation: DeviceBindingGeneration,
    pub capability_revision: CapabilityRevision,
    pub confirmation: PhysicalConfirmation,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SecretCandidateActivationProjectionRepr {
    contract_version: SecretContractVersionV1,
    operation: SecretCandidateActivationOperation,
    candidate_id: SecretCandidateId,
    candidate_revision: SecretCandidateRevision,
    kind: SecretCandidateKind,
    comparison_policy: LegacyActivationComparisonPolicy,
    comparison_impact: LegacyActivationComparisonImpact,
    secret_ref: SecretRef,
    purpose: SecretPurpose,
    record_revision: SecretRecordRevision,
    backend_instance_id: SecretBackendInstanceId,
    backend_generation: SecretBackendGeneration,
    device_binding_generation: DeviceBindingGeneration,
    capability_revision: CapabilityRevision,
    target_owners: Vec<SecretOwner>,
    expected_bindings: Vec<OwnerBindingExpectation>,
    legacy_sources_to_scrub: CurrentLegacySourceExpectations,
    candidate_read: SecretActivationCandidateReadExpectation,
    old_record_delete: SecretActivationOldRecordDeleteExpectation,
    projection_digest: SecretProjectionDigest,
}

#[derive(Clone, PartialEq, Eq)]
pub struct SecretCandidateActivationProjection(
    SecretCandidateActivationProjectionRepr,
);

impl SecretCandidateActivationProjection {
    fn validate_repr(
        repr: SecretCandidateActivationProjectionRepr,
    ) -> Result<Self, WireValidationError> {
        let owner_sets_match = todo!(
            "non-empty strict sorted target owners equal expected-binding owners"
        );
        let policy_matches = match repr.comparison_policy {
            LegacyActivationComparisonPolicy::CandidateEquality => {
                !repr.legacy_sources_to_scrub.as_slice().is_empty()
            }
            LegacyActivationComparisonPolicy::ExplicitReplacement => true,
        };
        let impact_matches = matches!(
            (&repr.comparison_policy, &repr.comparison_impact),
            (
                LegacyActivationComparisonPolicy::CandidateEquality,
                LegacyActivationComparisonImpact::CandidateEquality { .. },
            ) | (
                LegacyActivationComparisonPolicy::ExplicitReplacement,
                LegacyActivationComparisonImpact::ExplicitReplacement { .. },
            )
        );
        let fixed_scrub_policy = repr.kind
            != SecretCandidateKind::LegacyScrubExistingBinding
            || repr.comparison_policy
                == LegacyActivationComparisonPolicy::CandidateEquality;
        if owner_sets_match && policy_matches && impact_matches && fixed_scrub_policy {
            Ok(Self(repr))
        } else {
            Err(WireValidationError("invalid activation projection"))
        }
    }

    pub(in crate::secret) fn candidate_id(&self) -> &SecretCandidateId {
        &self.0.candidate_id
    }

    pub(in crate::secret) fn comparison_policy(
        &self,
    ) -> LegacyActivationComparisonPolicy {
        self.0.comparison_policy
    }

    pub(in crate::secret) fn projection_digest(&self) -> &SecretProjectionDigest {
        &self.0.projection_digest
    }

    pub(in crate::secret) fn legacy_sources(
        &self,
    ) -> &[LegacySourceExpectation] {
        self.0.legacy_sources_to_scrub.as_slice()
    }
}

impl Serialize for SecretCandidateActivationProjection {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for SecretCandidateActivationProjection {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::validate_repr(
            SecretCandidateActivationProjectionRepr::deserialize(deserializer)?,
        )
        .map_err(de::Error::custom)
    }
}

wire_enum!(SecretCandidateActivationOperation { SecretCandidateActivation });
wire_enum!(StagedSecretImportActivationOperation { StagedSecretImportActivation });
wire_enum!(CodexProviderApplyOperation { CodexProviderApply });

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct StagedLegacySourceExpectations(Vec<LegacySourceExpectation>);

impl StagedLegacySourceExpectations {
    fn validate(
        values: Vec<LegacySourceExpectation>,
    ) -> Result<Self, WireValidationError> {
        let staging_only = values.iter().all(|expectation| {
            matches!(
                expectation.source.origin,
                LegacySourceOrigin::SqlImportStaging
                    | LegacySourceOrigin::DbRestoreStaging
                    | LegacySourceOrigin::SyncDownloadStaging
            )
        });
        let sorted_unique = !values.is_empty() && values.windows(2).all(|pair| {
            legacy_source_sort_key(&pair[0].source)
                < legacy_source_sort_key(&pair[1].source)
        });
        if staging_only && sorted_unique {
            Ok(Self(values))
        } else {
            Err(WireValidationError(
                "staged import sources must be non-empty/staging/sorted/unique",
            ))
        }
    }
}

impl<'de> Deserialize<'de> for StagedLegacySourceExpectations {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::validate(Vec::<LegacySourceExpectation>::deserialize(deserializer)?)
            .map_err(de::Error::custom)
    }
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct StagedSourceSetCas {
    staged_row_revision: StagedRowRevision,
    structure_digest: RecoveryStructureDigest,
    source_count: u32,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct StagedSecretImportActivationProjectionRepr {
    contract_version: SecretContractVersionV1,
    operation: StagedSecretImportActivationOperation,
    stage_id: ImportStageId,
    owner: SecretOwner,
    staged_source_set_cas: StagedSourceSetCas,
    source_expectations: StagedLegacySourceExpectations,
    candidate_id: SecretCandidateId,
    candidate_revision: SecretCandidateRevision,
    comparison_policy: LegacyActivationComparisonPolicy,
    comparison_impact: LegacyActivationComparisonImpact,
    secret_ref: SecretRef,
    record_revision: SecretRecordRevision,
    backend_instance_id: SecretBackendInstanceId,
    backend_generation: SecretBackendGeneration,
    device_binding_generation: DeviceBindingGeneration,
    capability_revision: CapabilityRevision,
    expected_live_binding: OwnerBindingExpectation,
    projection_digest: SecretProjectionDigest,
}

#[derive(Clone, PartialEq, Eq)]
pub struct StagedSecretImportActivationProjection(
    StagedSecretImportActivationProjectionRepr,
);

impl StagedSecretImportActivationProjection {
    fn validate_repr(
        repr: StagedSecretImportActivationProjectionRepr,
    ) -> Result<Self, WireValidationError> {
        let exact_count = repr.staged_source_set_cas.source_count as usize
            == repr.source_expectations.0.len();
        let owner_matches = todo!("expected live binding owner equals projection owner");
        let impact_matches = matches!(
            (&repr.comparison_policy, &repr.comparison_impact),
            (
                LegacyActivationComparisonPolicy::CandidateEquality,
                LegacyActivationComparisonImpact::CandidateEquality { .. },
            ) | (
                LegacyActivationComparisonPolicy::ExplicitReplacement,
                LegacyActivationComparisonImpact::ExplicitReplacement { .. },
            )
        );
        if exact_count && owner_matches && impact_matches {
            Ok(Self(repr))
        } else {
            Err(WireValidationError("invalid staged import activation projection"))
        }
    }

    pub(in crate::secret) fn stage_id(&self) -> &ImportStageId {
        &self.0.stage_id
    }

    pub(in crate::secret) fn projection_digest(&self) -> &SecretProjectionDigest {
        &self.0.projection_digest
    }

    pub(crate) fn comparison_policy(&self) -> LegacyActivationComparisonPolicy {
        self.0.comparison_policy
    }
}

impl Serialize for StagedSecretImportActivationProjection {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for StagedSecretImportActivationProjection {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::validate_repr(
            StagedSecretImportActivationProjectionRepr::deserialize(deserializer)?,
        )
        .map_err(de::Error::custom)
    }
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StagedImportResumeCas {
    revision: StagedImportResumeRevision,
    digest: StagedImportResumeDigest,
}

wire_enum!(ResumeStagedImportCutoverAction { ResumeStagedImportCutover });

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResumeStagedImportCutoverRequest {
    stage_id: ImportStageId,
    expected_resume_cas: StagedImportResumeCas,
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "status",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
enum StagedSecretImportActivationResultRepr {
    Activated {
        schema_version: SchemaVersionV1,
        stage_id: ImportStageId,
        candidate_id: SecretCandidateId,
        owner_summary: SecretOwnerCredentialSummary,
        audit_event_id: SecretAuditEventId,
    },
    AlreadyActivated {
        schema_version: SchemaVersionV1,
        stage_id: ImportStageId,
        candidate_id: SecretCandidateId,
        owner_summary: SecretOwnerCredentialSummary,
        audit_event_id: SecretAuditEventId,
    },
    CutoverRecoveryRequired {
        schema_version: SchemaVersionV1,
        stage_id: ImportStageId,
        action: ResumeStagedImportCutoverAction,
        current_resume_cas: StagedImportResumeCas,
        audit_event_id: SecretAuditEventId,
    },
}

#[derive(Clone, PartialEq, Eq)]
pub struct StagedSecretImportActivationResultDto(
    StagedSecretImportActivationResultRepr,
);

impl StagedSecretImportActivationResultDto {
    fn checked_from_cutover_journal(
        repr: StagedSecretImportActivationResultRepr,
        journal: &DurableSecretOperationJournal,
    ) -> Result<Self, SecretInternalError> {
        todo!("initial terminal arm may project the verified candidate/owner summary; recovery arm exposes only stage/action/current CAS/audit while candidate/owner/checkpoint remain in the journal preimage")
    }
}

impl Serialize for StagedSecretImportActivationResultDto {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: Serializer {
        self.0.serialize(serializer)
    }
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "status",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
enum ResumeStagedImportCutoverResultRepr {
    Activated {
        stage_id: ImportStageId,
        current_resume_cas: StagedImportResumeCas,
        action: SecretUserAction,
        issue: Option<SecretIssueView>,
    },
    AlreadyActivated {
        stage_id: ImportStageId,
        current_resume_cas: StagedImportResumeCas,
        action: SecretUserAction,
        issue: Option<SecretIssueView>,
    },
    CutoverRecoveryRequired {
        stage_id: ImportStageId,
        current_resume_cas: StagedImportResumeCas,
        action: SecretUserAction,
        issue: Option<SecretIssueView>,
    },
}

#[derive(Clone, PartialEq, Eq)]
pub struct ResumeStagedImportCutoverResultDto(
    ResumeStagedImportCutoverResultRepr,
);

impl ResumeStagedImportCutoverResultDto {
    fn checked_from_resume_journal(
        repr: ResumeStagedImportCutoverResultRepr,
        journal: &DurableSecretOperationJournal,
    ) -> Result<Self, SecretInternalError> {
        let _ = journal;
        todo!("exact five fields in every arm: stageId/currentResumeCas/status/action/issue; terminal arms require action=none + issue=None serialized as null, recovery requires action=resumeStagedImportCutover + Some(checked issue); schema/audit/candidate/owner/ref/summary are structurally impossible")
    }
}

impl Serialize for ResumeStagedImportCutoverResultDto {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SecretContractVersionV1 {
    #[serde(rename = "secret-contract/v1")]
    V1,
}

impl SecretContractVersionV1 {
    pub const WIRE: &'static str = "secret-contract/v1";
}

wire_enum!(SecretApplyRole { Target, Rollback });
wire_enum!(SecretApplyTargetRole { Target });
wire_enum!(SecretApplyRollbackRole { Rollback });

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SecretApplyTargetProjectionRepr {
    role: SecretApplyTargetRole,
    consumer: SecretChangePlanApplyConsumer,
    target_sink: SecretChangePlanApplySink,
    live_sink_id: CodexLiveSecretSinkId,
    owner: SecretOwner,
    secret_ref: SecretRef,
    owner_binding_revision: SecretOwnerBindingRevision,
    binding_revision: SecretBindingRevision,
    record_revision: SecretRecordRevision,
    binding_set_cas: SecretBindingSetCas,
    backend_instance_id: SecretBackendInstanceId,
    backend_generation: SecretBackendGeneration,
    device_binding_generation: DeviceBindingGeneration,
    capability_revision: CapabilityRevision,
}

#[derive(Clone, PartialEq, Eq)]
pub struct SecretApplyTargetProjection(SecretApplyTargetProjectionRepr);

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SecretApplyRollbackProjectionRepr {
    role: SecretApplyRollbackRole,
    consumer: SecretChangePlanApplyConsumer,
    target_sink: SecretChangePlanApplySink,
    live_sink_id: CodexLiveSecretSinkId,
    owner: SecretOwner,
    secret_ref: SecretRef,
    owner_binding_revision: SecretOwnerBindingRevision,
    binding_revision: SecretBindingRevision,
    record_revision: SecretRecordRevision,
    binding_set_cas: SecretBindingSetCas,
    backend_instance_id: SecretBackendInstanceId,
    backend_generation: SecretBackendGeneration,
    device_binding_generation: DeviceBindingGeneration,
    capability_revision: CapabilityRevision,
}

#[derive(Clone, PartialEq, Eq)]
pub struct SecretApplyRollbackProjection(SecretApplyRollbackProjectionRepr);

fn validate_apply_projection_identity(
    owner: &SecretOwner,
    binding_set: &SecretBindingSetCas,
) -> Result<(), WireValidationError> {
    let _ = (owner, binding_set);
    todo!("provider/codex owner; nonzero exact binding-set; strict scalar matrix")
}

macro_rules! impl_apply_role_projection {
    ($public:ident, $repr:ident) => {
        impl $public {
            fn validate_repr(repr: $repr) -> Result<Self, WireValidationError> {
                validate_apply_projection_identity(&repr.owner, &repr.binding_set_cas)?;
                Ok(Self(repr))
            }

            pub(in crate::secret) fn live_sink_id(&self) -> CodexLiveSecretSinkId {
                self.0.live_sink_id
            }
        }

        impl Serialize for $public {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where S: Serializer {
                self.0.serialize(serializer)
            }
        }

        impl<'de> Deserialize<'de> for $public {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where D: Deserializer<'de> {
                Self::validate_repr($repr::deserialize(deserializer)?)
                    .map_err(de::Error::custom)
            }
        }
    };
}

impl_apply_role_projection!(SecretApplyTargetProjection, SecretApplyTargetProjectionRepr);
impl_apply_role_projection!(SecretApplyRollbackProjection, SecretApplyRollbackProjectionRepr);

// Readiness output only. Each wrapped projection already serializes its
// single-value role; untagged here cannot swallow fields because there is no
// Deserialize implementation for this enum.
#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum SecretApplyCredentialProjection {
    Target(SecretApplyTargetProjection),
    Rollback(SecretApplyRollbackProjection),
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SecretApplyPlanProjectionRepr {
    contract_version: SecretContractVersionV1,
    operation: CodexProviderApplyOperation,
    target: SecretApplyTargetProjection,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_absent_only"
    )]
    rollback: Option<SecretApplyRollbackProjection>,
    projection_digest: SecretProjectionDigest,
}

#[derive(Clone, PartialEq, Eq)]
pub struct SecretApplyPlanProjection(SecretApplyPlanProjectionRepr);

impl SecretApplyPlanProjection {
    fn validate_repr(
        repr: SecretApplyPlanProjectionRepr,
    ) -> Result<Self, WireValidationError> {
        todo!("operation literal, target/rollback role separation, full canonical digest")
    }
}

impl Serialize for SecretApplyPlanProjection {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: Serializer {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for SecretApplyPlanProjection {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where D: Deserializer<'de> {
        Self::validate_repr(SecretApplyPlanProjectionRepr::deserialize(deserializer)?)
            .map_err(de::Error::custom)
    }
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SecretApplyReadinessContext {
    schema_version: SchemaVersionV1,
    operation_id: SecretOperationId,
    projection: SecretApplyCredentialProjection,
    checked_at: UtcTimestamp,
    expires_at: UtcTimestamp,
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "status",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
enum SecretApplyReadinessRepr {
    Ready {
        context: SecretApplyReadinessContext,
    },
    ConfirmationRequired {
        context: SecretApplyReadinessContext,
        confirmation: SecretConfirmationRequirementView,
    },
    Blocked {
        context: SecretApplyReadinessContext,
        error: SecretIssueView,
    },
}

#[derive(Clone, PartialEq, Eq)]
pub struct SecretApplyReadiness(SecretApplyReadinessRepr);

impl SecretApplyReadiness {
    fn checked_from_authority(
        repr: SecretApplyReadinessRepr,
    ) -> Result<Self, SecretInternalError> {
        todo!("ready/confirmation/blocked exclusivity, context expiry, issue route")
    }
}

impl Serialize for SecretApplyReadiness {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: Serializer {
        self.0.serialize(serializer)
    }
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SecretConfirmationRequirementView {
    operation: SecretBackendOperation,
    device: SecretDeviceDisplay,
    timeout_seconds: ConfirmationTimeoutSeconds,
    prompt_key: HardwarePromptKey,
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HardwarePromptKey {
    #[serde(rename = "secret.hardware.confirmTouch")]
    ConfirmTouch,
}

wire_enum!(ResolveForApplyOperation { ResolveForApply });

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SecretApplyHardwareConfirmStep {
    pub schema_version: SchemaVersionV1,
    pub step_id: SecretConfirmationStepId,
    pub operation_id: SecretOperationId,
    pub operation: ResolveForApplyOperation,
    pub role: SecretApplyRole,
    pub backend_instance_id: SecretBackendInstanceId,
    pub device: SecretDeviceDisplay,
    pub prompt_key: HardwarePromptKey,
    pub expires_at: UtcTimestamp,
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "operation",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum HardwareConfirmStep {
    ResolveForApply {
        schema_version: SchemaVersionV1,
        step_id: SecretConfirmationStepId,
        operation_id: SecretOperationId,
        role: SecretApplyRole,
        backend_instance_id: SecretBackendInstanceId,
        device: SecretDeviceDisplay,
        prompt_key: HardwarePromptKey,
        expires_at: UtcTimestamp,
    },
    CaptureVerify {
        schema_version: SchemaVersionV1,
        step_id: SecretConfirmationStepId,
        operation_id: SecretOperationId,
        backend_instance_id: SecretBackendInstanceId,
        device: SecretDeviceDisplay,
        prompt_key: HardwarePromptKey,
        expires_at: UtcTimestamp,
    },
    Validate {
        schema_version: SchemaVersionV1,
        step_id: SecretConfirmationStepId,
        operation_id: SecretOperationId,
        backend_instance_id: SecretBackendInstanceId,
        device: SecretDeviceDisplay,
        prompt_key: HardwarePromptKey,
        expires_at: UtcTimestamp,
    },
    Delete {
        schema_version: SchemaVersionV1,
        step_id: SecretConfirmationStepId,
        operation_id: SecretOperationId,
        backend_instance_id: SecretBackendInstanceId,
        device: SecretDeviceDisplay,
        prompt_key: HardwarePromptKey,
        expires_at: UtcTimestamp,
    },
    Revoke {
        schema_version: SchemaVersionV1,
        step_id: SecretConfirmationStepId,
        operation_id: SecretOperationId,
        backend_instance_id: SecretBackendInstanceId,
        device: SecretDeviceDisplay,
        prompt_key: HardwarePromptKey,
        expires_at: UtcTimestamp,
    },
}

wire_enum!(WriterReadbackMatchedCode { ReadbackMatched });
wire_enum!(WriterFailedCode { WriterFailed });
wire_enum!(WriterReadbackMismatchCode { ReadbackMismatch });
wire_enum!(WriterReadbackUnavailableCode { ReadbackUnavailable });
wire_enum!(WriterTargetChanged { Changed });
wire_enum!(WriterTargetNone { None });
wire_enum!(WriterTargetChangedUnknown { ChangedUnknown });

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "status",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum SecretWriterReceiptDto {
    Succeeded {
        writer_code: WriterReadbackMatchedCode,
        target_effect: WriterTargetChanged,
    },
    FailedBeforeMutation {
        writer_code: WriterFailedCode,
        target_effect: WriterTargetNone,
    },
    FailedAfterMutation {
        writer_code: WriterFailedCode,
        target_effect: WriterTargetChangedUnknown,
    },
    ReadbackMismatch {
        writer_code: WriterReadbackMismatchCode,
        target_effect: WriterTargetChanged,
    },
    ReadbackUnavailable {
        writer_code: WriterReadbackUnavailableCode,
        target_effect: WriterTargetChangedUnknown,
    },
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SecretApplyResultDto {
    schema_version: SchemaVersionV1,
    operation_id: SecretOperationId,
    role: SecretApplyRole,
    status: SecretApplyResultStatus,
    writer: SecretWriterReceiptDto,
    consumed_record_revision: SecretRecordRevision,
    consumed_binding_set_revision: SecretBindingSetRevision,
    consumed_backend_generation: SecretBackendGeneration,
    audit_event_id: SecretAuditEventId,
}

pub(crate) struct ConsumedPreparedSecretCapabilityIdentity {
    _private: (),
}

impl SecretApplyResultDto {
    fn checked_from_consumed_capability(
        result: SecretApplyResultDto,
        capability: &ConsumedPreparedSecretCapabilityIdentity,
    ) -> Result<Self, SecretInternalError> {
        todo!("role/operation/revision/generation/writer/audit identity")
    }
}

wire_enum!(SecretApplyResultStatus { WriterReturned });

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SecretMutationImpact {
    schema_version: SchemaVersionV1,
    secret_ref: SecretRef,
    secret_ref_display: SecretRefDisplay,
    record_revision: SecretRecordRevision,
    binding_set_cas: SecretBindingSetCas,
    affected_owners: Vec<SecretOwnerBindingSummary>,
    effect: SecretImpactEffect,
    no_fallback: AlwaysTrue,
}

impl SecretMutationImpact {
    fn checked_from_candidate_snapshot(
        impact: SecretMutationImpact,
        snapshot: &SecretCandidateAuthoritySnapshot,
    ) -> Result<Self, SecretInternalError> {
        todo!("ref/revision/binding-set/affected-owner/effect identity")
    }
}

wire_enum!(SecretImpactEffect { AllBindingsAffected, OneBindingAffected });

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SecretDeleteReadinessContext {
    pub schema_version: SchemaVersionV1,
    pub operation_id: SecretOperationId,
    pub operation: SecretDeleteOperation,
    pub secret_ref: SecretRef,
    pub record_revision: SecretRecordRevision,
    pub binding_set_cas: SecretBindingSetCas,
    pub checked_at: UtcTimestamp,
    pub expires_at: UtcTimestamp,
}

wire_enum!(SecretDeleteOperation { Delete });

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "status",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum SecretDeleteReadiness {
    Ready {
        context: SecretDeleteReadinessContext,
    },
    ConfirmationRequired {
        context: SecretDeleteReadinessContext,
        confirmation: SecretConfirmationRequirementView,
    },
    Blocked {
        context: SecretDeleteReadinessContext,
        error: SecretIssueView,
    },
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SecretDeleteImpact {
    pub impact: SecretMutationImpact,
    pub readiness: SecretDeleteReadiness,
}

wire_enum!(ActivationCleanupStepKind {
    FinalizeLegacyScrub, DeleteOldRecord, VerifyOldRecordMissing
});
wire_enum!(SecretRecoveryOperation { Recovery });

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RotationSupersessionView {
    pub source: RotationSupersessionSource,
    pub revoked_at: UtcTimestamp,
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ActivationCleanupStepImpact {
    FinalizeLegacyScrub {
        backend_kind: SecretBackendKind,
        backend_instance_id: SecretBackendInstanceId,
        confirmation: PhysicalConfirmation,
    },
    DeleteOldRecord {
        backend_kind: SecretBackendKind,
        backend_instance_id: SecretBackendInstanceId,
        confirmation: PhysicalConfirmation,
    },
    VerifyOldRecordMissing {
        backend_kind: SecretBackendKind,
        backend_instance_id: SecretBackendInstanceId,
        confirmation: PhysicalConfirmation,
    },
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretRecoveryReadinessContext {
    pub schema_version: SchemaVersionV1,
    pub operation_id: SecretOperationId,
    pub operation: SecretRecoveryOperation,
    pub recovery_id: SecretRecoveryId,
    pub recovery_kind: SecretRecoveryKind,
    pub recovery_cas: SecretRecoveryCas,
    pub checked_at: UtcTimestamp,
    pub expires_at: UtcTimestamp,
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "status",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum SecretRecoveryReadiness {
    Ready {
        context: SecretRecoveryReadinessContext,
    },
    ConfirmationRequired {
        context: SecretRecoveryReadinessContext,
        confirmation: SecretConfirmationRequirementView,
    },
    Blocked {
        context: SecretRecoveryReadinessContext,
        error: SecretIssueView,
    },
}

fn secret_owner_sort_key(
    owner: &SecretOwner,
) -> (&'static str, &str, &str, &'static str) {
    let kind = match owner.kind {
        SecretOwnerKind::Provider => "provider",
        SecretOwnerKind::Agent => "agent",
    };
    let slot = match owner.slot {
        SecretSlot::PrimaryApiKey => "primaryApiKey",
    };
    (
        kind,
        owner.namespace.as_str(),
        owner.owner_id.as_str(),
        slot,
    )
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct SortedAffectedOwners(Vec<SecretOwnerBindingSummary>);

impl SortedAffectedOwners {
    pub(in crate::secret) fn try_from_sorted_nonempty(
        owners: Vec<SecretOwnerBindingSummary>,
    ) -> Result<Self, SecretInternalError> {
        let ordered = owners.windows(2).all(|pair| {
            secret_owner_sort_key(&pair[0].owner)
                < secret_owner_sort_key(&pair[1].owner)
        });
        if owners.is_empty() || !ordered {
            Err(SecretInternalError::input_invalid())
        } else {
            Ok(Self(owners))
        }
    }
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct SortedOwnerSummaries(Vec<SecretOwnerCredentialSummary>);

impl SortedOwnerSummaries {
    pub(in crate::secret) fn try_from_sorted_nonempty(
        owners: Vec<SecretOwnerCredentialSummary>,
    ) -> Result<Self, SecretInternalError> {
        let ordered = owners.windows(2).all(|pair| {
            secret_owner_sort_key(&pair[0].owner)
                < secret_owner_sort_key(&pair[1].owner)
        });
        if owners.is_empty() || !ordered {
            Err(SecretInternalError::input_invalid())
        } else {
            Ok(Self(owners))
        }
    }
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct SortedSecretOwners(Vec<SecretOwner>);

impl SortedSecretOwners {
    pub(in crate::secret) fn try_from_sorted_unique(
        owners: Vec<SecretOwner>,
    ) -> Result<Self, SecretInternalError> {
        let ordered = owners.windows(2).all(|pair| {
            secret_owner_sort_key(&pair[0]) < secret_owner_sort_key(&pair[1])
        });
        if ordered {
            Ok(Self(owners))
        } else {
            Err(SecretInternalError::input_invalid())
        }
    }
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct SortedActivationCleanupSteps(Vec<ActivationCleanupStepKind>);

impl SortedActivationCleanupSteps {
    fn try_from_sorted_unique(
        steps: Vec<ActivationCleanupStepKind>,
    ) -> Result<Self, SecretInternalError> {
        match steps.as_slice() {
            []
            | [ActivationCleanupStepKind::FinalizeLegacyScrub]
            | [ActivationCleanupStepKind::DeleteOldRecord]
            | [ActivationCleanupStepKind::VerifyOldRecordMissing]
            | [
                ActivationCleanupStepKind::FinalizeLegacyScrub,
                ActivationCleanupStepKind::DeleteOldRecord,
                ActivationCleanupStepKind::VerifyOldRecordMissing,
            ]
            | [
                ActivationCleanupStepKind::DeleteOldRecord,
                ActivationCleanupStepKind::VerifyOldRecordMissing,
            ]
            | [
                ActivationCleanupStepKind::FinalizeLegacyScrub,
                ActivationCleanupStepKind::DeleteOldRecord,
            ] => Ok(Self(steps)),
            _ => Err(SecretInternalError::input_invalid()),
        }
    }

    fn contains(&self, step: &ActivationCleanupStepKind) -> bool {
        self.0.contains(step)
    }

    fn iter(&self) -> impl Iterator<Item = &ActivationCleanupStepKind> {
        self.0.iter()
    }
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct NonEmptyActivationCleanupSteps(Vec<ActivationCleanupStepKind>);

impl NonEmptyActivationCleanupSteps {
    fn try_from_sorted_unique(
        steps: Vec<ActivationCleanupStepKind>,
    ) -> Result<Self, SecretInternalError> {
        match steps.as_slice() {
            [ActivationCleanupStepKind::FinalizeLegacyScrub]
            | [ActivationCleanupStepKind::DeleteOldRecord]
            | [ActivationCleanupStepKind::VerifyOldRecordMissing]
            | [
                ActivationCleanupStepKind::FinalizeLegacyScrub,
                ActivationCleanupStepKind::DeleteOldRecord,
                ActivationCleanupStepKind::VerifyOldRecordMissing,
            ]
            | [
                ActivationCleanupStepKind::DeleteOldRecord,
                ActivationCleanupStepKind::VerifyOldRecordMissing,
            ] => Ok(Self(steps)),
            _ => Err(SecretInternalError::input_invalid()),
        }
    }

    fn is_disjoint_from(&self, completed: &SortedActivationCleanupSteps) -> bool {
        self.0.iter().all(|step| !completed.contains(step))
    }


    fn iter(&self) -> impl Iterator<Item = &ActivationCleanupStepKind> {
        self.0.iter()
    }
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct NonEmptySortedActivationCleanupStepImpacts(
    Vec<ActivationCleanupStepImpact>,
);

impl NonEmptySortedActivationCleanupStepImpacts {
    fn try_from_sorted_unique(
        steps: Vec<ActivationCleanupStepImpact>,
    ) -> Result<Self, SecretInternalError> {
        match steps.as_slice() {
            [ActivationCleanupStepImpact::FinalizeLegacyScrub { .. }]
            | [ActivationCleanupStepImpact::DeleteOldRecord { .. }]
            | [ActivationCleanupStepImpact::VerifyOldRecordMissing { .. }]
            | [
                ActivationCleanupStepImpact::FinalizeLegacyScrub { .. },
                ActivationCleanupStepImpact::DeleteOldRecord { .. },
                ActivationCleanupStepImpact::VerifyOldRecordMissing { .. },
            ]
            | [
                ActivationCleanupStepImpact::DeleteOldRecord { .. },
                ActivationCleanupStepImpact::VerifyOldRecordMissing { .. },
            ] => Ok(Self(steps)),
            _ => Err(SecretInternalError::input_invalid()),
        }
    }


    fn contains_kind(&self, expected: &ActivationCleanupStepKind) -> bool {
        self.0.iter().any(|impact| {
            matches!(
                (impact, expected),
                (
                    ActivationCleanupStepImpact::FinalizeLegacyScrub { .. },
                    ActivationCleanupStepKind::FinalizeLegacyScrub,
                ) | (
                    ActivationCleanupStepImpact::DeleteOldRecord { .. },
                    ActivationCleanupStepKind::DeleteOldRecord,
                ) | (
                    ActivationCleanupStepImpact::VerifyOldRecordMissing { .. },
                    ActivationCleanupStepKind::VerifyOldRecordMissing,
                )
            )
        })
    }
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct ActivationCleanupImpactRepr {
    schema_version: SchemaVersionV1,
    recovery_id: SecretRecoveryId,
    recovery_cas: SecretRecoveryCas,
    candidate_id: SecretCandidateId,
    affected_owners: SortedAffectedOwners,
    secret_ref_display: SecretRefDisplay,
    pending_steps: NonEmptySortedActivationCleanupStepImpacts,
    readiness: SecretRecoveryReadiness,
}

#[derive(Clone, PartialEq, Eq)]
struct ActivationCleanupImpact(ActivationCleanupImpactRepr);

impl ActivationCleanupImpact {
    // Private device-authority factory; no public/product constructor.
    fn from_recovery_snapshot(
        repr: ActivationCleanupImpactRepr,
        snapshot: &SecretRecoveryAuthoritySnapshot,
    ) -> Result<Self, SecretInternalError> {
        let _ = snapshot;
        todo!("validate activation-cleanup impact against recovery snapshot");
        let context = match &repr.readiness {
            SecretRecoveryReadiness::Ready { context }
            | SecretRecoveryReadiness::ConfirmationRequired { context, .. }
            | SecretRecoveryReadiness::Blocked { context, .. } => context,
        };
        if context.recovery_id != repr.recovery_id
            || context.recovery_cas != repr.recovery_cas
        {
            return Err(SecretInternalError::input_invalid());
        }
        Ok(Self(repr))
    }
}

impl Serialize for ActivationCleanupImpact {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "status",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
enum ActivationCleanupResultRepr {
    Complete {
        schema_version: SchemaVersionV1,
        recovery_id: SecretRecoveryId,
        recovery_cas: SecretRecoveryCas,
        completed_steps: SortedActivationCleanupSteps,
        remaining_steps: [ActivationCleanupStepKind; 0],
        owner_summaries: SortedOwnerSummaries,
        aggregate: SecretRefAggregate,
        candidate: SecretCandidateSummary,
        audit_event_id: SecretAuditEventId,
    },
    AlreadyComplete {
        schema_version: SchemaVersionV1,
        recovery_id: SecretRecoveryId,
        recovery_cas: SecretRecoveryCas,
        completed_steps: SortedActivationCleanupSteps,
        remaining_steps: [ActivationCleanupStepKind; 0],
        owner_summaries: SortedOwnerSummaries,
        aggregate: SecretRefAggregate,
        candidate: SecretCandidateSummary,
        audit_event_id: SecretAuditEventId,
    },
    RecoveryRequired {
        schema_version: SchemaVersionV1,
        recovery_id: SecretRecoveryId,
        recovery_cas: SecretRecoveryCas,
        completed_steps: SortedActivationCleanupSteps,
        remaining_steps: NonEmptyActivationCleanupSteps,
        owner_summaries: SortedOwnerSummaries,
        aggregate: SecretRefAggregate,
        candidate: SecretCandidateSummary,
        issue: SecretIssueView,
        audit_event_id: SecretAuditEventId,
    },
}

#[derive(Clone, PartialEq, Eq)]
struct ActivationCleanupResult(ActivationCleanupResultRepr);

impl ActivationCleanupResult {
    // The three private owner-module factories populate one repr variant and
    // all call this gate before construction.
    fn validate_sets(
        completed: &SortedActivationCleanupSteps,
        remaining: Option<&NonEmptyActivationCleanupSteps>,
        issue: Option<&SecretIssueView>,
    ) -> Result<(), SecretInternalError> {
        match (remaining, issue) {
            (None, None) => Ok(()),
            (Some(remaining), Some(issue))
                if remaining.is_disjoint_from(completed)
                    && issue.code
                        == SecretErrorCode::SecretOperationRecoveryRequired =>
            {
                Ok(())
            }
            _ => Err(SecretInternalError::input_invalid()),
        }
    }

    fn from_authority_snapshot(
        repr: ActivationCleanupResultRepr,
        admitted_pending: &NonEmptySortedActivationCleanupStepImpacts,
        snapshot: &SecretRecoveryAuthoritySnapshot,
    ) -> Result<Self, SecretInternalError> {
        let _ = snapshot;
        todo!("validate activation-cleanup result against recovery snapshot");
        match &repr {
            ActivationCleanupResultRepr::Complete {
                completed_steps, ..
            }
            | ActivationCleanupResultRepr::AlreadyComplete {
                completed_steps, ..
            } => {
                Self::validate_sets(completed_steps, None, None)?;
                if !completed_steps
                    .iter()
                    .all(|step| admitted_pending.contains_kind(step))
                {
                    return Err(SecretInternalError::input_invalid());
                }
            }
            ActivationCleanupResultRepr::RecoveryRequired {
                recovery_id,
                recovery_cas,
                completed_steps,
                remaining_steps,
                issue,
                ..
            } => {
                Self::validate_sets(
                    completed_steps,
                    Some(remaining_steps),
                    Some(issue),
                )?;
                if !issue.recovery.as_ref().is_some_and(|pointer| {
                    &pointer.recovery_id == recovery_id
                        && &pointer.recovery_cas == recovery_cas
                }) {
                    return Err(SecretInternalError::input_invalid());
                }
                if !completed_steps
                    .iter()
                    .chain(remaining_steps.iter())
                    .all(|step| admitted_pending.contains_kind(step))
                {
                    return Err(SecretInternalError::input_invalid());
                }
            }
        }
        Ok(Self(repr))
    }
}

impl Serialize for ActivationCleanupResult {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SecretRecoveryStepKind {
    FinalizeLegacyScrub,
    DeleteOldRecord,
    VerifyOldRecordMissing,
    DeleteUncommittedRecord,
    VerifyUncommittedRecordMissing,
    FinalizeCaptureCompensation,
    DeleteAdmittedRecord,
    VerifyDeletedRecordMissing,
    FinalizeDeletedRecord,
    FinalizeOwnerDetach,
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum NeverPhysicalConfirmation { Never }

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
struct SortedRecoverySteps(Vec<SecretRecoveryStepKind>);

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
struct NonEmptySortedRecoverySteps(Vec<SecretRecoveryStepKind>);

impl SortedRecoverySteps {
    pub(in crate::secret) fn checked(
        values: Vec<SecretRecoveryStepKind>,
        kind: SecretRecoveryKind,
    ) -> Result<Self, SecretInternalError> {
        todo!("sorted unique completed subset of exact kind allowlist")
    }
}

impl NonEmptySortedRecoverySteps {
    pub(in crate::secret) fn checked(
        values: Vec<SecretRecoveryStepKind>,
        kind: SecretRecoveryKind,
    ) -> Result<Self, SecretInternalError> {
        todo!("nonempty sorted unique remaining subset of exact kind allowlist")
    }

    pub(in crate::secret) fn disjoint_from(&self, completed: &SortedRecoverySteps) -> bool {
        todo!("exact disjointness")
    }
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum SecretRecoveryStepImpact {
    FinalizeLegacyScrub {
        backend_kind: SecretBackendKind,
        backend_instance_id: SecretBackendInstanceId,
        confirmation: PhysicalConfirmation,
    },
    DeleteOldRecord {
        backend_kind: SecretBackendKind,
        backend_instance_id: SecretBackendInstanceId,
        confirmation: PhysicalConfirmation,
    },
    VerifyOldRecordMissing {
        backend_kind: SecretBackendKind,
        backend_instance_id: SecretBackendInstanceId,
        confirmation: PhysicalConfirmation,
    },
    DeleteUncommittedRecord {
        backend_kind: SecretBackendKind,
        backend_instance_id: SecretBackendInstanceId,
        confirmation: PhysicalConfirmation,
    },
    VerifyUncommittedRecordMissing {
        backend_kind: SecretBackendKind,
        backend_instance_id: SecretBackendInstanceId,
        confirmation: PhysicalConfirmation,
    },
    FinalizeCaptureCompensation {
        confirmation: NeverPhysicalConfirmation,
    },
    DeleteAdmittedRecord {
        backend_kind: SecretBackendKind,
        backend_instance_id: SecretBackendInstanceId,
        confirmation: PhysicalConfirmation,
    },
    VerifyDeletedRecordMissing {
        backend_kind: SecretBackendKind,
        backend_instance_id: SecretBackendInstanceId,
        confirmation: PhysicalConfirmation,
    },
    FinalizeDeletedRecord {
        confirmation: NeverPhysicalConfirmation,
    },
    FinalizeOwnerDetach {
        confirmation: NeverPhysicalConfirmation,
    },
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
struct NonEmptySortedRecoveryStepImpacts(Vec<SecretRecoveryStepImpact>);

impl NonEmptySortedRecoveryStepImpacts {
    fn checked(
        values: Vec<SecretRecoveryStepImpact>,
        kind: SecretRecoveryKind,
    ) -> Result<Self, SecretInternalError> {
        todo!("non-empty strict rank/unique and exact recovery-kind step allowlist")
    }
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct CaptureCompensationImpact {
    schema_version: SchemaVersionV1,
    recovery_id: SecretRecoveryId,
    recovery_cas: SecretRecoveryCas,
    candidate_id: SecretCandidateId,
    secret_ref_display: SecretRefDisplay,
    pending_steps: NonEmptySortedRecoveryStepImpacts,
    readiness: SecretRecoveryReadiness,
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct DeleteFinalizationImpact {
    schema_version: SchemaVersionV1,
    recovery_id: SecretRecoveryId,
    recovery_cas: SecretRecoveryCas,
    affected_owners: SortedAffectedOwners,
    secret_ref_display: SecretRefDisplay,
    pending_steps: NonEmptySortedRecoveryStepImpacts,
    readiness: SecretRecoveryReadiness,
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "state",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
enum OwnerDetachRecoveryBindingState {
    Bound {
        secret_ref_display: SecretRefDisplay,
        binding_revision: SecretBindingRevision,
        binding_set_cas: SecretBindingSetCas,
    },
    Unbound,
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct OwnerDetachFinalizationImpact {
    schema_version: SchemaVersionV1,
    recovery_id: SecretRecoveryId,
    recovery_cas: SecretRecoveryCas,
    detached_owner: SecretOwner,
    remaining_owners: SortedSecretOwners,
    binding_state: OwnerDetachRecoveryBindingState,
    pending_steps: NonEmptySortedRecoveryStepImpacts,
    readiness: SecretRecoveryReadiness,
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "kind",
    content = "impact",
    rename_all = "camelCase"
)]
enum SecretRecoveryImpactRepr {
    ActivationCleanup(ActivationCleanupImpactRepr),
    CaptureCompensation(CaptureCompensationImpact),
    DeleteFinalization(DeleteFinalizationImpact),
    OwnerDetachFinalization(OwnerDetachFinalizationImpact),
}

#[derive(Clone, PartialEq, Eq)]
pub struct SecretRecoveryImpact(SecretRecoveryImpactRepr);

impl SecretRecoveryImpact {
    fn from_authority_snapshot(
        repr: SecretRecoveryImpactRepr,
        snapshot: &SecretRecoveryAuthoritySnapshot,
    ) -> Result<Self, SecretInternalError> {
        snapshot.validate_recovery_impact_identity(&repr)?;
        todo!("validate outer kind equals readiness kind/CAS and exact step algebra")
    }
}

impl Serialize for SecretRecoveryImpact {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "status",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
enum LocalRecoveryOutcome {
    Complete {
        schema_version: SchemaVersionV1,
        recovery_id: SecretRecoveryId,
        recovery_cas: SecretRecoveryCas,
        completed_steps: SortedRecoverySteps,
        remaining_steps: [SecretRecoveryStepKind; 0],
        audit_event_id: SecretAuditEventId,
    },
    AlreadyComplete {
        schema_version: SchemaVersionV1,
        recovery_id: SecretRecoveryId,
        recovery_cas: SecretRecoveryCas,
        completed_steps: SortedRecoverySteps,
        remaining_steps: [SecretRecoveryStepKind; 0],
        audit_event_id: SecretAuditEventId,
    },
    RecoveryRequired {
        schema_version: SchemaVersionV1,
        recovery_id: SecretRecoveryId,
        recovery_cas: SecretRecoveryCas,
        completed_steps: SortedRecoverySteps,
        remaining_steps: NonEmptySortedRecoverySteps,
        issue: SecretIssueView,
        audit_event_id: SecretAuditEventId,
    },
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "status",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
enum CaptureCompensationRecoveryResult {
    Complete {
        schema_version: SchemaVersionV1,
        recovery_id: SecretRecoveryId,
        recovery_cas: SecretRecoveryCas,
        candidate_id: SecretCandidateId,
        secret_ref_display: SecretRefDisplay,
        completed_steps: SortedRecoverySteps,
        remaining_steps: [SecretRecoveryStepKind; 0],
        terminal_candidate_state: DiscardedCandidateTerminalState,
        audit_event_id: SecretAuditEventId,
    },
    AlreadyComplete {
        schema_version: SchemaVersionV1,
        recovery_id: SecretRecoveryId,
        recovery_cas: SecretRecoveryCas,
        candidate_id: SecretCandidateId,
        secret_ref_display: SecretRefDisplay,
        completed_steps: SortedRecoverySteps,
        remaining_steps: [SecretRecoveryStepKind; 0],
        terminal_candidate_state: DiscardedCandidateTerminalState,
        audit_event_id: SecretAuditEventId,
    },
    RecoveryRequired {
        schema_version: SchemaVersionV1,
        recovery_id: SecretRecoveryId,
        recovery_cas: SecretRecoveryCas,
        candidate_id: SecretCandidateId,
        secret_ref_display: SecretRefDisplay,
        completed_steps: SortedRecoverySteps,
        remaining_steps: NonEmptySortedRecoverySteps,
        issue: SecretIssueView,
        audit_event_id: SecretAuditEventId,
    },
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct DeleteFinalizationRecoveryResult {
    owner_summaries: SortedOwnerSummaries,
    aggregate: SecretRefAggregate,
    outcome: LocalRecoveryOutcome,
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct OwnerDetachRecoveryResult {
    detached_owner: SecretOwner,
    remaining_owners: SortedSecretOwners,
    outcome: LocalRecoveryOutcome,
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "kind",
    content = "result",
    rename_all = "camelCase"
)]
enum SecretRecoveryResultRepr {
    ActivationCleanup(ActivationCleanupResultRepr),
    CaptureCompensation(CaptureCompensationRecoveryResult),
    DeleteFinalization(DeleteFinalizationRecoveryResult),
    OwnerDetachFinalization(OwnerDetachRecoveryResult),
}

#[derive(Clone, PartialEq, Eq)]
pub struct SecretRecoveryResult(SecretRecoveryResultRepr);

impl SecretRecoveryResult {
    fn from_authority_snapshot(
        repr: SecretRecoveryResultRepr,
        snapshot: &SecretRecoveryAuthoritySnapshot,
    ) -> Result<Self, SecretInternalError> {
        snapshot.validate_recovery_result_identity(&repr)?;
        todo!("validate kind-specific terminal/pending rows and disjoint step sets")
    }
}

impl Serialize for SecretRecoveryResult {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct NullableSecretMutationImpact(
    Option<SecretMutationImpact>,
);

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StageSecretCandidateResult {
    status: SecretCandidateStageStatus,
    candidate: SecretCandidateSummary,
    activation_projection: SecretCandidateActivationProjection,
    // Required field: serializes as object or explicit null; omission fails.
    impact: NullableSecretMutationImpact,
    audit_event_id: SecretAuditEventId,
}

impl StageSecretCandidateResult {
    fn checked_from_candidate_snapshot(
        result: StageSecretCandidateResult,
        snapshot: &SecretCandidateAuthoritySnapshot,
    ) -> Result<Self, SecretInternalError> {
        todo!("candidate/projection/policy/impact/null/audit identity")
    }
}

wire_enum!(SecretCandidateStageStatus { Staged });

pub(in crate::secret) fn legacy_source_sort_key(
    source: &LegacySourceRef,
) -> (u8, u8, &str) {
    let origin = match source.origin {
        LegacySourceOrigin::ProviderRow => 0,
        LegacySourceOrigin::LiveAuth => 1,
        LegacySourceOrigin::LiveConfig => 2,
        LegacySourceOrigin::SqlImportStaging => 3,
        LegacySourceOrigin::DbRestoreStaging => 4,
        LegacySourceOrigin::SyncDownloadStaging => 5,
    };
    let category = match source.category {
        LegacySourceCategory::ProviderAuthJson => 0,
        LegacySourceCategory::ProviderConfigTomlTopLevel => 1,
        LegacySourceCategory::ProviderConfigTomlActiveTable => 2,
        LegacySourceCategory::ProviderConfigTomlInactiveTable => 3,
        LegacySourceCategory::ProviderConfigTomlInlineTable => 4,
        LegacySourceCategory::ProviderUsageScriptApiKey => 5,
        LegacySourceCategory::ProviderNonCanonicalProxyAlias => 6,
    };
    (origin, category, source.location_id.as_str())
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct SortedLegacySourceRefs(Vec<LegacySourceRef>);

impl SortedLegacySourceRefs {
    fn try_from_sorted_unique(
        sources: Vec<LegacySourceRef>,
    ) -> Result<Self, SecretInternalError> {
        let ordered = sources.windows(2).all(|pair| {
            legacy_source_sort_key(&pair[0]) < legacy_source_sort_key(&pair[1])
        });
        if ordered {
            Ok(Self(sources))
        } else {
            Err(SecretInternalError::input_invalid())
        }
    }

    fn is_disjoint_from(&self, retained: &NonEmptySortedLegacySourceRefs) -> bool {
        self.0.iter().all(|source| !retained.0.contains(source))
    }
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct NonEmptySortedLegacySourceRefs(Vec<LegacySourceRef>);

impl NonEmptySortedLegacySourceRefs {
    fn try_from_sorted_unique(
        sources: Vec<LegacySourceRef>,
    ) -> Result<Self, SecretInternalError> {
        if sources.is_empty() {
            return Err(SecretInternalError::input_invalid());
        }
        SortedLegacySourceRefs::try_from_sorted_unique(sources.clone())?;
        Ok(Self(sources))
    }
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "status",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum SecretLegacyCleanupTerminal {
    NotApplicable,
    Complete {
        scrubbed_sources: SortedLegacySourceRefs,
    },
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "status",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum SecretLegacyCleanupPending {
    Partial {
        scrubbed_sources: SortedLegacySourceRefs,
        retained_sources: NonEmptySortedLegacySourceRefs,
        issue: SecretIssueView,
    },
    Blocked {
        retained_sources: NonEmptySortedLegacySourceRefs,
        issue: SecretIssueView,
    },
}

impl SecretLegacyCleanupPending {
    fn validate(&self) -> Result<(), SecretInternalError> {
        match self {
            Self::Partial {
                scrubbed_sources,
                retained_sources,
                issue,
            } if scrubbed_sources.is_disjoint_from(retained_sources)
                && issue.code
                    == SecretErrorCode::SecretOperationRecoveryRequired =>
            {
                Ok(())
            }
            Self::Blocked { issue, .. }
                if issue.code
                    == SecretErrorCode::SecretOperationRecoveryRequired =>
            {
                Ok(())
            }
            _ => Err(SecretInternalError::input_invalid()),
        }
    }

    fn issue(&self) -> &SecretIssueView {
        match self {
            Self::Partial { issue, .. } | Self::Blocked { issue, .. } => issue,
        }
    }
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "status",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum SecretOldRecordCleanupTerminal {
    NotApplicable,
    Deleted {
        old_secret_ref_display: SecretRefDisplay,
        supersession: RotationSupersessionView,
    },
    AlreadyMissing {
        old_secret_ref_display: SecretRefDisplay,
        supersession: RotationSupersessionView,
    },
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretOldRecordCleanupPending {
    pub status: SecretOldRecordCleanupPendingStatus,
    pub old_secret_ref_display: SecretRefDisplay,
    pub issue: SecretIssueView,
}

wire_enum!(SecretOldRecordCleanupPendingStatus { CleanupRequired });
wire_enum!(SecretActivationCompleteKind { Complete });

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretActivationCompleteCleanup {
    pub kind: SecretActivationCompleteKind,
    pub legacy: SecretLegacyCleanupTerminal,
    pub old_record: SecretOldRecordCleanupTerminal,
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum SecretActivationPendingCleanup {
    LegacyScrubPending {
        legacy: SecretLegacyCleanupPending,
        old_record: SecretOldRecordNotAttempted,
        recovery: SecretRecoveryPointer,
    },
    OldRecordDeletePending {
        legacy: SecretLegacyCleanupTerminal,
        old_record: SecretOldRecordCleanupPending,
        recovery: SecretRecoveryPointer,
    },
}

impl SecretActivationPendingCleanup {
    fn validate(&self) -> Result<(), SecretInternalError> {
        match self {
            Self::LegacyScrubPending {
                legacy,
                recovery,
                ..
            } => {
                legacy.validate()?;
                if legacy.issue().recovery.as_ref() == Some(recovery) {
                    Ok(())
                } else {
                    Err(SecretInternalError::input_invalid())
                }
            }
            Self::OldRecordDeletePending {
                old_record,
                recovery,
                ..
            }
                if old_record.issue.code
                    == SecretErrorCode::SecretOperationRecoveryRequired
                    && old_record.issue.recovery.as_ref() == Some(recovery) =>
            {
                Ok(())
            }
            _ => Err(SecretInternalError::input_invalid()),
        }
    }
}

wire_enum!(SecretOldRecordNotAttemptedStatus { NotAttempted });

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretOldRecordNotAttempted {
    pub status: SecretOldRecordNotAttemptedStatus,
}

wire_enum!(ActivationOldRecordDeleteScope { ActivationOldRecordDelete });
wire_enum!(ActivationCandidateReadOperation { ResolveForApply });
wire_enum!(ActivationCandidateReadScope { ActivationCandidateCompare });

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretActivationReadHardwareConfirmStep {
    pub schema_version: SchemaVersionV1,
    pub step_id: SecretConfirmationStepId,
    pub operation_id: SecretOperationId,
    pub operation: ActivationCandidateReadOperation,
    pub scope: ActivationCandidateReadScope,
    pub backend_instance_id: SecretBackendInstanceId,
    pub device: SecretDeviceDisplay,
    pub prompt_key: HardwarePromptKey,
    pub expires_at: UtcTimestamp,
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretActivationDeleteHardwareConfirmStep {
    pub schema_version: SchemaVersionV1,
    pub step_id: SecretConfirmationStepId,
    pub operation_id: SecretOperationId,
    pub operation: ActivationOldRecordDeleteOperation,
    pub scope: ActivationOldRecordDeleteScope,
    pub backend_instance_id: SecretBackendInstanceId,
    pub device: SecretDeviceDisplay,
    pub prompt_key: HardwarePromptKey,
    pub expires_at: UtcTimestamp,
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretActivationOldRecordMissingHardwareConfirmStep {
    pub schema_version: SchemaVersionV1,
    pub step_id: SecretConfirmationStepId,
    pub operation_id: SecretOperationId,
    pub operation: ActivationOldRecordMissingReadbackOperation,
    pub scope: ActivationOldRecordMissingReadbackScope,
    pub backend_instance_id: SecretBackendInstanceId,
    pub device: SecretDeviceDisplay,
    pub prompt_key: HardwarePromptKey,
    pub expires_at: UtcTimestamp,
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum SecretActivationHardwareConfirmStep {
    CandidateRead(SecretActivationReadHardwareConfirmStep),
    OldRecordDelete(SecretActivationDeleteHardwareConfirmStep),
    OldRecordMissingReadback(SecretActivationOldRecordMissingHardwareConfirmStep),
}

impl SecretActivationHardwareConfirmStep {
    fn operation_id(&self) -> &SecretOperationId {
        match self {
            Self::CandidateRead(step) => &step.operation_id,
            Self::OldRecordDelete(step) => &step.operation_id,
            Self::OldRecordMissingReadback(step) => &step.operation_id,
        }
    }
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "status",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
enum SecretActivationPreparationViewRepr {
    Prepared {
        schema_version: SchemaVersionV1,
        operation_id: SecretOperationId,
        expires_at: UtcTimestamp,
    },
    ConfirmationRequired {
        schema_version: SchemaVersionV1,
        operation_id: SecretOperationId,
        step: SecretActivationHardwareConfirmStep,
    },
}

#[derive(Clone, PartialEq, Eq)]
pub struct SecretActivationPreparationView(SecretActivationPreparationViewRepr);

impl SecretActivationPreparationView {
    // Private to crate::secret::device_store::result.
    fn from_prepared(
        repr: SecretActivationPreparationViewRepr,
    ) -> Result<Self, SecretInternalError> {
        if let SecretActivationPreparationViewRepr::ConfirmationRequired {
            operation_id,
            step,
            ..
        } = &repr
        {
            if step.operation_id() != operation_id {
                return Err(SecretInternalError::input_invalid());
            }
        }
        Ok(Self(repr))
    }
}

impl Serialize for SecretActivationPreparationView {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "status",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
enum SecretActivationResultDtoRepr {
    Activated {
        schema_version: SchemaVersionV1,
        candidate_id: SecretCandidateId,
        plan_id: ChangePlanId,
        aggregate: SecretRefAggregate,
        affected_owners: SortedAffectedOwners,
        cleanup: SecretActivationCompleteCleanup,
        target_projection: SecretTargetProjectionStatus,
        audit_event_id: SecretAuditEventId,
    },
    AlreadyActivated {
        schema_version: SchemaVersionV1,
        candidate_id: SecretCandidateId,
        plan_id: ChangePlanId,
        aggregate: SecretRefAggregate,
        affected_owners: SortedAffectedOwners,
        cleanup: SecretActivationCompleteCleanup,
        target_projection: SecretTargetProjectionStatus,
        audit_event_id: SecretAuditEventId,
    },
    ActivatedCleanupPending {
        schema_version: SchemaVersionV1,
        candidate_id: SecretCandidateId,
        plan_id: ChangePlanId,
        aggregate: SecretRefAggregate,
        affected_owners: SortedAffectedOwners,
        cleanup: SecretActivationPendingCleanup,
        target_projection: SecretTargetProjectionStatus,
        audit_event_id: SecretAuditEventId,
    },
}

#[derive(Clone, PartialEq, Eq)]
pub struct SecretActivationResultDto(SecretActivationResultDtoRepr);

impl SecretActivationResultDto {
    // Private to crate::secret::device_store::result after identity/recovery
    // cross-checks against the committed authority snapshot.
    fn from_authority_snapshot(
        repr: SecretActivationResultDtoRepr,
        snapshot: &SecretCandidateAuthoritySnapshot,
    ) -> Result<Self, SecretInternalError> {
        snapshot.validate_activation_result_identity(&repr)?;
        if let SecretActivationResultDtoRepr::ActivatedCleanupPending {
            cleanup,
            ..
        } = &repr
        {
            cleanup.validate()?;
        }
        Ok(Self(repr))
    }
}

impl Serialize for SecretActivationResultDto {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

wire_enum!(SecretTargetProjectionStatus { NotPerformedByActivation });
wire_enum!(LegacyMigrationOwnerStatus {
    NoCredential, AlreadyMigrated, CandidateStaged, CleanupCandidateStaged,
    Conflict, SourceInvalid, ComparisonPending, Blocked, Failed
});
wire_enum!(HistoricalArtifactCategory {
    HistoricalProviderSnapshot, AppPrivateCache, ManagedDiagnostic,
    ManagedBackup, UserOwnedExport
});
wire_enum!(ArtifactScanStatus { NotRun, Complete, Partial, Blocked });
wire_enum!(SecretMigrationStatus {
    NoChanges, Staged, ApprovalRequired, Partial, Blocked
});

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LegacyMigrationOwnerResult {
    owner: SecretOwner,
    status: LegacyMigrationOwnerStatus,
    sources: Vec<LegacySourceRef>,
    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "deserialize_absent_only")]
    candidate_id: Option<SecretCandidateId>,
    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "deserialize_absent_only")]
    activation_projection: Option<SecretCandidateActivationProjection>,
    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "deserialize_absent_only")]
    plan_id: Option<ChangePlanId>,
    action: SecretUserAction,
    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "deserialize_absent_only")]
    issue: Option<SecretIssueView>,
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SecretArtifactScanReport {
    status: ArtifactScanStatus,
    enumerated_categories: Vec<HistoricalArtifactCategory>,
    scanned_count: u32,
    finding_count: u32,
    report_only_count: u32,
    unreadable_count: u32,
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SecretMigrationReport {
    schema_version: SchemaVersionV1,
    report_id: SecretMigrationReportId,
    status: SecretMigrationStatus,
    owners: Vec<LegacyMigrationOwnerResult>,
    artifact_scan: SecretArtifactScanReport,
    started_at: UtcTimestamp,
    completed_at: UtcTimestamp,
}

impl SecretMigrationReport {
    pub(super) fn checked_from_inventory(
        report: SecretMigrationReport,
    ) -> Result<Self, SecretInternalError> {
        todo!("owner status/candidate/projection/plan/action/issue and aggregate status matrix")
    }
}

wire_enum!(SecretApplyAuditAction {
    PrepareApply, ConfirmHardware, ResolveApply
});
wire_enum!(SecretGeneralAuditAction {
    CaptureCandidate, DiscardCandidate, ActivateCandidate, Validate,
    RotateCandidate, Lock, Unlock, Delete, Revoke, CheckReadiness,
    MigrateLegacy, ReconcileLegacy, ReconcileRecovery, RetryCleanup,
    CancelConfirmation
});

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum SecretAuditScope {
    General {
        action: SecretGeneralAuditAction,
    },
    Apply {
        action: SecretApplyAuditAction,
        role: SecretApplyRole,
    },
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SecretAuditEvent {
    schema_version: SchemaVersionV1,
    event_id: SecretAuditEventId,
    occurred_at: UtcTimestamp,
    operation_id: SecretOperationId,
    scope: SecretAuditScope,
    outcome: SecretAuditOutcome,
    effect: SecretEffect,
    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "deserialize_absent_only")]
    owner: Option<SecretOwner>,
    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "deserialize_absent_only")]
    secret_ref_display: Option<SecretRefDisplay>,
    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "deserialize_absent_only")]
    backend_kind: Option<SecretBackendKind>,
    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "deserialize_absent_only")]
    backend_instance_id: Option<SecretBackendInstanceId>,
    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "deserialize_absent_only")]
    error_code: Option<SecretErrorCode>,
}

impl SecretAuditEvent {
    // Sole device-store audit factory. It enforces §11.1 action/scope/role,
    // outcome/effect/error and optional owner/backend tuple constraints.
    pub(super) fn checked_from_operation(
        event: SecretAuditEvent,
    ) -> Result<Self, SecretInternalError> {
        todo!("complete audit matrix and material-free optional-field allowlist")
    }
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SecretErrorView {
    code: SecretErrorCode,
    retryable: bool,
    action: SecretUserAction,
    effect: SecretEffect,
    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "deserialize_absent_only")]
    audit_event_id: Option<SecretAuditEventId>,
    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "deserialize_absent_only")]
    owner: Option<SecretOwner>,
    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "deserialize_absent_only")]
    secret_ref_display: Option<SecretRefDisplay>,
    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "deserialize_absent_only")]
    lock_source: Option<SecretLockSource>,
    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "deserialize_absent_only")]
    revocation_source: Option<SecretRevocationSource>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    backend_unavailable_reason: Option<SecretBackendUnavailableReason>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    recovery: Option<SecretRecoveryPointer>,
}

impl SecretErrorView {
    fn checked_from_internal(
        error: SecretInternalError,
        audit_event_id: Option<SecretAuditEventId>,
        owner: Option<SecretOwner>,
        secret_ref_display: Option<SecretRefDisplay>,
    ) -> Self {
        Self {
            code: error.code,
            retryable: error.retryable,
            action: error.action,
            effect: error.effect,
            audit_event_id,
            owner,
            secret_ref_display,
            lock_source: error.lock_source,
            revocation_source: error.revocation_source,
            backend_unavailable_reason: error.backend_unavailable_reason,
            recovery: error.recovery,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretCommandSuccess<T> {
    pub contract_version: SecretContractVersionV1,
    pub schema_version: SchemaVersionV1,
    pub command_id: SecretCommandId,
    pub data: T,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretCommandError {
    pub contract_version: SecretContractVersionV1,
    pub schema_version: SchemaVersionV1,
    pub command_id: SecretCommandId,
    pub error: SecretErrorView,
}

pub type SecretCommandResult<T> =
    Result<SecretCommandSuccess<T>, SecretCommandError>;
```

### 6.4 Exact Rust request/result types

```rust
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ListSecretSummariesRequest {
    pub schema_version: SchemaVersionV1,
    #[serde(default, deserialize_with = "deserialize_absent_only")]
    pub owner: Option<SecretOwner>,
    #[serde(default, deserialize_with = "deserialize_absent_only")]
    pub secret_ref: Option<SecretRef>,
    #[serde(default, deserialize_with = "deserialize_absent_only")]
    pub availability: Option<Vec<SecretStableAvailability>>,
    pub include_unbound_owners: bool,
    #[serde(default, deserialize_with = "deserialize_absent_only")]
    pub cursor: Option<SecretSummaryCursor>,
    pub limit: PageLimit,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListSecretSummariesResult {
    owners: Vec<SecretOwnerCredentialSummary>,
    refs: Vec<SecretRefAggregate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    next_cursor: Option<SecretSummaryCursor>,
}

impl ListSecretSummariesResult {
    fn checked_from_authority(
        result: ListSecretSummariesResult,
    ) -> Result<Self, SecretInternalError> {
        todo!("sorted unique owners/refs, binding joins, cursor/page invariants")
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ListSecretBackendOptionsRequest {
    pub schema_version: SchemaVersionV1,
    pub owner: SecretOwner,
    pub purpose: SecretPurpose,
    pub intent: BeginCaptureIntent,
}

wire_enum!(BeginCaptureIntent { NewBinding, ReplaceBinding, LegacyReconcile });

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretBackendOption {
    backend: SecretBackendInstanceView,
    capabilities_for_new_record: SecretRecordCapabilities,
}

#[derive(Serialize)]
#[serde(
    tag = "state",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
enum SecretCaptureBindingView {
    Unbound,
    Bound {
        secret_ref_display: SecretRefDisplay,
        binding_revision: SecretBindingRevision,
    },
    Legacy {
        legacy_state: LegacyOwnerState,
        source_count: u32,
    },
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretCaptureIntentView {
    schema_version: SchemaVersionV1,
    capture_intent_id: SecretCaptureIntentId,
    owner: SecretOwner,
    purpose: SecretPurpose,
    intent: BeginCaptureIntent,
    current_binding: SecretCaptureBindingView,
    legacy_source_coverage: LegacySourceCoverageView,
    expires_at: UtcTimestamp,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListSecretBackendOptionsResult {
    capture_intent: SecretCaptureIntentView,
    options: Vec<SecretBackendOption>,
}

impl ListSecretBackendOptionsResult {
    fn checked_from_registry(
        result: ListSecretBackendOptionsResult,
    ) -> Result<Self, SecretInternalError> {
        todo!("output-only intent view is derived from the exact atomic owner/binding/coverage receipt; options are sorted unique registered instances with matching validated capabilities")
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BeginSecretCaptureRequest {
    pub schema_version: SchemaVersionV1,
    pub capture_intent_id: SecretCaptureIntentId,
    pub backend_instance_id: SecretBackendInstanceId,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RotateSecretRequest {
    pub schema_version: SchemaVersionV1,
    pub secret_ref: SecretRef,
    pub backend_instance_id: SecretBackendInstanceId,
    pub expected_record_revision: SecretRecordRevision,
    pub expected_binding_set: SecretBindingSetCas,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ListSecretCandidatesRequest {
    pub schema_version: SchemaVersionV1,
    #[serde(default, deserialize_with = "deserialize_absent_only")]
    pub owner: Option<SecretOwner>,
    pub include_terminal: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListSecretCandidatesResult {
    candidates: Vec<SecretCandidateWithProjection>,
}

impl ListSecretCandidatesResult {
    fn checked_from_authority(
        result: ListSecretCandidatesResult,
    ) -> Result<Self, SecretInternalError> {
        todo!("sorted unique candidate rows and exact projection pairing")
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretCandidateWithProjection {
    candidate: SecretCandidateSummary,
    activation_projection: SecretCandidateActivationProjection,
}

impl SecretCandidateWithProjection {
    fn checked_from_candidate_snapshot(
        value: SecretCandidateWithProjection,
        snapshot: &SecretCandidateAuthoritySnapshot,
    ) -> Result<Self, SecretInternalError> {
        todo!("candidate id/revision/policy and projection identity")
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DiscardSecretCandidateRequest {
    pub schema_version: SchemaVersionV1,
    pub candidate_id: SecretCandidateId,
    pub expected_candidate_revision: SecretCandidateRevision,
}

wire_enum!(CandidateDiscardDeleteOperation { Delete });
wire_enum!(CandidateDiscardDeleteScope { CandidateDiscardRecordDelete });
wire_enum!(CandidateDiscardMissingReadbackOperation { Validate });
wire_enum!(CandidateDiscardMissingReadbackScope {
    CandidateDiscardRecordMissingReadback
});

#[derive(Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum CandidateDiscardConfirmationSlot {
    RecordDelete,
    RecordMissingReadback,
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretCandidateDiscardDeleteHardwareConfirmStep {
    pub schema_version: SchemaVersionV1,
    pub step_id: SecretConfirmationStepId,
    pub operation_id: SecretOperationId,
    pub operation: CandidateDiscardDeleteOperation,
    pub scope: CandidateDiscardDeleteScope,
    pub backend_instance_id: SecretBackendInstanceId,
    pub device: SecretDeviceDisplay,
    pub prompt_key: HardwarePromptKey,
    pub expires_at: UtcTimestamp,
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretCandidateDiscardMissingHardwareConfirmStep {
    pub schema_version: SchemaVersionV1,
    pub step_id: SecretConfirmationStepId,
    pub operation_id: SecretOperationId,
    pub operation: CandidateDiscardMissingReadbackOperation,
    pub scope: CandidateDiscardMissingReadbackScope,
    pub backend_instance_id: SecretBackendInstanceId,
    pub device: SecretDeviceDisplay,
    pub prompt_key: HardwarePromptKey,
    pub expires_at: UtcTimestamp,
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "slot", content = "confirmation", rename_all = "camelCase")]
pub enum SecretCandidateDiscardHardwareConfirmStep {
    RecordDelete(SecretCandidateDiscardDeleteHardwareConfirmStep),
    RecordMissingReadback(SecretCandidateDiscardMissingHardwareConfirmStep),
}

impl SecretCandidateDiscardHardwareConfirmStep {
    fn operation_id(&self) -> &SecretOperationId {
        match self {
            Self::RecordDelete(step) => &step.operation_id,
            Self::RecordMissingReadback(step) => &step.operation_id,
        }
    }
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "status",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
enum SecretCandidateDiscardPreparationViewRepr {
    Prepared {
        schema_version: SchemaVersionV1,
        operation_id: SecretOperationId,
        expires_at: UtcTimestamp,
    },
    ConfirmationRequired {
        schema_version: SchemaVersionV1,
        operation_id: SecretOperationId,
        step: SecretCandidateDiscardHardwareConfirmStep,
    },
}

#[derive(Clone, PartialEq, Eq)]
pub struct SecretCandidateDiscardPreparationView(
    SecretCandidateDiscardPreparationViewRepr,
);

impl SecretCandidateDiscardPreparationView {
    fn checked(
        repr: SecretCandidateDiscardPreparationViewRepr,
    ) -> Result<Self, SecretInternalError> {
        if let SecretCandidateDiscardPreparationViewRepr::ConfirmationRequired {
            operation_id,
            step,
            ..
        } = &repr
        {
            if step.operation_id() != operation_id {
                return Err(SecretInternalError::input_invalid());
            }
        }
        Ok(Self(repr))
    }
}

impl Serialize for SecretCandidateDiscardPreparationView {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

wire_enum!(DiscardedCandidateTerminalState { Discarded });
wire_enum!(ExpiredCandidateTerminalState { Expired });
wire_enum!(RefreshSummaryAction { RefreshSummary });

#[derive(Serialize)]
#[serde(
    tag = "status",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
enum DiscardSecretCandidateResultRepr {
    Discarded {
        terminal_state: DiscardedCandidateTerminalState,
        candidate_id: SecretCandidateId,
        audit_event_id: SecretAuditEventId,
    },
    AlreadyDiscarded {
        terminal_state: DiscardedCandidateTerminalState,
        candidate_id: SecretCandidateId,
        audit_event_id: SecretAuditEventId,
    },
    Expired {
        terminal_state: ExpiredCandidateTerminalState,
        candidate_id: SecretCandidateId,
        action: RefreshSummaryAction,
        audit_event_id: SecretAuditEventId,
    },
    AlreadyExpired {
        terminal_state: ExpiredCandidateTerminalState,
        candidate_id: SecretCandidateId,
        action: RefreshSummaryAction,
        audit_event_id: SecretAuditEventId,
    },
}

pub struct DiscardSecretCandidateResult(DiscardSecretCandidateResultRepr);

impl DiscardSecretCandidateResult {
    fn checked_from_candidate_journal(
        repr: DiscardSecretCandidateResultRepr,
        journal: &CandidateDeleteJournalRow,
    ) -> Result<Self, SecretInternalError> {
        todo!("status/terminal state/candidate exactly match durable terminal journal; expired arms alone carry action=refreshSummary and never a pending issue")
    }
}

impl Serialize for DiscardSecretCandidateResult {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SetSecretLockedRequest {
    pub schema_version: SchemaVersionV1,
    pub secret_ref: SecretRef,
    pub locked: bool,
    pub expected_record_revision: SecretRecordRevision,
    pub expected_binding_set: SecretBindingSetCas,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GetSecretDeleteImpactRequest {
    pub schema_version: SchemaVersionV1,
    pub secret_ref: SecretRef,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeleteSecretRequest {
    pub schema_version: SchemaVersionV1,
    pub operation_id: SecretOperationId,
    pub secret_ref: SecretRef,
    pub expected_record_revision: SecretRecordRevision,
    pub expected_binding_set: SecretBindingSetCas,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ValidateSecretRequest {
    pub schema_version: SchemaVersionV1,
    pub secret_ref: SecretRef,
    pub expected_record_revision: SecretRecordRevision,
}

#[derive(Deserialize)]
#[serde(
    tag = "role",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum CheckSecretApplyReadinessRequest {
    Target {
        schema_version: SchemaVersionV1,
        owner: SecretOwner,
        consumer: SecretConsumer,
        target_sink: ApplyTargetSink,
        live_sink_id: CodexLiveSecretSinkId,
    },
    Rollback {
        schema_version: SchemaVersionV1,
        owner: SecretOwner,
        consumer: SecretConsumer,
        target_sink: ApplyTargetSink,
        live_sink_id: CodexLiveSecretSinkId,
    },
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GetSecretCleanupImpactRequest {
    pub schema_version: SchemaVersionV1,
    pub recovery_id: SecretRecoveryId,
    pub recovery_kind: SecretRecoveryKind,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RetrySecretCleanupRequest {
    pub schema_version: SchemaVersionV1,
    pub operation_id: SecretOperationId,
    pub recovery_id: SecretRecoveryId,
    pub recovery_kind: SecretRecoveryKind,
    pub expected_recovery_cas: SecretRecoveryCas,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MigrateLegacyCodexSecretsRequest {
    pub schema_version: SchemaVersionV1,
    #[serde(default, deserialize_with = "deserialize_absent_only")]
    pub owner: Option<SecretOwner>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ListSecretAuditRequest {
    pub schema_version: SchemaVersionV1,
    #[serde(default, deserialize_with = "deserialize_absent_only")]
    pub owner: Option<SecretOwner>,
    #[serde(default, deserialize_with = "deserialize_absent_only")]
    pub secret_ref: Option<SecretRef>,
    #[serde(default, deserialize_with = "deserialize_absent_only")]
    pub actions: Option<Vec<SecretAuditAction>>,
    #[serde(default, deserialize_with = "deserialize_absent_only")]
    pub outcomes: Option<Vec<SecretAuditOutcome>>,
    #[serde(default, deserialize_with = "deserialize_absent_only")]
    pub cursor: Option<SecretAuditCursor>,
    pub limit: PageLimit,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretAuditPage {
    events: Vec<SecretAuditEvent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    next_cursor: Option<SecretAuditCursor>,
}

wire_enum!(SecretValidationOutcome { Valid, Missing, Blocked });

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretValidationResult {
    outcome: SecretValidationOutcome,
    aggregate: SecretRefAggregate,
    audit_event_id: SecretAuditEventId,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretMutationResult {
    aggregate: SecretRefAggregate,
    audit_event_id: SecretAuditEventId,
}

wire_enum!(SecretDeleteStatus { Revoked, AlreadyRevoked });

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretDeleteResult {
    status: SecretDeleteStatus,
    aggregate: SecretRefAggregate,
    audit_event_id: SecretAuditEventId,
}

impl SecretAuditPage {
    fn checked_from_audit_store(page: SecretAuditPage) -> Result<Self, SecretInternalError> {
        todo!("ordered page, valid events and cursor")
    }
}

impl SecretValidationResult {
    fn checked_from_authority(
        result: SecretValidationResult,
    ) -> Result<Self, SecretInternalError> {
        todo!("outcome/aggregate/audit matrix")
    }
}

impl SecretMutationResult {
    fn checked_from_authority(
        result: SecretMutationResult,
    ) -> Result<Self, SecretInternalError> {
        todo!("aggregate/audit identity")
    }
}

impl SecretDeleteResult {
    fn checked_from_authority(
        result: SecretDeleteResult,
    ) -> Result<Self, SecretInternalError> {
        todo!("status/revocation source/aggregate/audit identity")
    }
}
```

Every request carries `SchemaVersionV1`, so `schemaVersion != 1` is rejected before command logic with `SECRET_REQUEST_INVALID`. The transport decoder first performs command-specific closed-shape/type decoding, then explicit newtype validation; it never classifies errors by parsing a serde error string. A wrong/missing schema, unknown field, wrong JSON type or invalid non-ref scalar maps only to `SECRET_REQUEST_INVALID`. `SECRET_REF_INVALID` is reachable only when a known `secretRef` property is a JSON string but fails the `SecretRef` grammar; the two-stage decoder retains that field-specific error kind. No other parse path emits `SECRET_REF_INVALID`.

The canonical Rust/TypeScript fixture gate generates, for every absent-only field, one omitted-valid sample and one present-null rejection sample at its exact nested path. It also proves `StageSecretCandidateResult.impact=null` is accepted while omission is rejected. Byte-canonical JSON fixtures MUST have one representation per semantic object; a nullable/omittable ambiguity fails `contract_schema`.

The shared negative corpus mutates every request-reachable nested object, union arm and collection element in turn: add one unknown key; add a sibling field belonging to another discriminant; replace/omit the discriminant; inject a forbidden semantic key; present `null` for an absent-only member; duplicate a canonicalized key; unsort/duplicate/disjoin an owner, step or capability set; mismatch ready/issue, status/remaining-steps, lock/source, revocation/source-time, `centralRevocation/revocationObservation`, recovery-kind/CAS, role/sink/live-sink, comparison-policy/impact or candidate-terminal-state tuples. Rust and TypeScript must reject the same fixture at the same structural boundary. TypeScript decoders are explicit exact-object/discriminated-union decoders—never intersection merge, object spread, passthrough or catch-all. Rust request/projection/durable decoders use nested `deny_unknown_fields` private reprs plus validating `TryFrom`/custom `Deserialize`; `flatten` is forbidden.

Output-only issue/aggregate/owner/candidate/result/audit/readiness types have private Rust fields or private repr wrappers, `Serialize` only and one checked factory in the authority owner; the compile-shape scanner rejects `Deserialize`, public struct literals, unchecked `From`, `Default` or alternate constructors. Durable journal/recovery and admitted projection wrappers may deserialize only through their private repr and must rerun every cross-field invariant above before construction. A JSON round-trip test of an output does not create a supported input surface.

## 7. Native-only backend and capability contract

### 7.1 Material and backend trait

There is no generic `expose_to<T>` and no by-ref public resolver. The only byte exposure is a private, synchronous, named sink adapter used by a backend implementation, #41 writer, or one dedicated Codex runtime executor. An exact authorized sink MAY make the minimum short-lived copy required by its target API (for example an HTTP authorization header or live-config writer buffer); that copy is single-operation, non-serializable, never cached/logged/retried/failover-reused, and its owning buffer is zeroized/dropped at terminal completion. Any other retention is forbidden.
For `codexApiKey` the captured UTF-8 material length is exactly `1..=2560` bytes and NUL is forbidden.

```rust
use subtle::ConstantTimeEq;
use zeroize::Zeroizing;

pub(crate) struct SecretInternalError {
    code: SecretErrorCode,
    retryable: bool,
    action: SecretUserAction,
    effect: SecretEffect,
    condition: SecretActionCondition,
    lock_source: Option<SecretLockSource>,
    revocation_source: Option<SecretRevocationSource>,
    backend_unavailable_reason: Option<SecretBackendUnavailableReason>,
    recovery: Option<SecretRecoveryPointer>,
}

struct SecretErrorSources {
    lock_source: Option<SecretLockSource>,
    revocation_source: Option<SecretRevocationSource>,
    backend_unavailable_reason: Option<SecretBackendUnavailableReason>,
    recovery: Option<SecretRecoveryPointer>,
}

#[derive(Debug)]
struct SecretErrorFactoryViolation;

// Closed input for source-free terminal failures. Codes that require a lock
// source, revocation source, backend-unavailable reason, or recovery pointer
// are intentionally unrepresentable here and have dedicated factories below.
pub(in crate::secret) enum SecretSourceFreeErrorCode {
    RequestInvalid,
    RefInvalid,
    OwnerKindUnsupported,
    OwnerNamespaceUnsupported,
    OwnerNotFound,
    OwnerConflict,
    OperationBusy,
    UnsupportedPurpose,
    ConsumerUnsupported,
    InputCancelled,
    InputInvalid,
    CandidateNotFound,
    CandidateExpired,
    CandidateConsumed,
    ChangePlanRequired,
    ChangePlanInvalid,
    ChangePlanStale,
    MigrationRequired,
    LegacySourceInvalid,
    LegacyConflict,
    LegacyComparisonPending,
    MigrationFailed,
    Missing,
    PermissionDenied,
    Stale,
    ConfirmationRequired,
    ConfirmationCancelled,
    ConfirmationExpired,
    ConfirmationReplayed,
    DeviceMismatch,
    WriteFailed,
    ReadFailed,
    DeleteFailed,
    VerifyFailed,
    ProjectionForbidden,
    DependencyChanged,
    RecordChanged,
    BackendChanged,
    CapabilityExpired,
    CapabilityConsumed,
    RecoveryNotFound,
    RecoveryChanged,
    Internal,
}

impl SecretSourceFreeErrorCode {
    fn stable_code(self) -> SecretErrorCode {
        match self {
            Self::RequestInvalid => SecretErrorCode::SecretRequestInvalid,
            Self::RefInvalid => SecretErrorCode::SecretRefInvalid,
            Self::OwnerKindUnsupported => SecretErrorCode::SecretOwnerKindUnsupported,
            Self::OwnerNamespaceUnsupported => SecretErrorCode::SecretOwnerNamespaceUnsupported,
            Self::OwnerNotFound => SecretErrorCode::SecretOwnerNotFound,
            Self::OwnerConflict => SecretErrorCode::SecretOwnerConflict,
            Self::OperationBusy => SecretErrorCode::SecretOperationBusy,
            Self::UnsupportedPurpose => SecretErrorCode::SecretUnsupportedPurpose,
            Self::ConsumerUnsupported => SecretErrorCode::SecretConsumerUnsupported,
            Self::InputCancelled => SecretErrorCode::SecretInputCancelled,
            Self::InputInvalid => SecretErrorCode::SecretInputInvalid,
            Self::CandidateNotFound => SecretErrorCode::SecretCandidateNotFound,
            Self::CandidateExpired => SecretErrorCode::SecretCandidateExpired,
            Self::CandidateConsumed => SecretErrorCode::SecretCandidateConsumed,
            Self::ChangePlanRequired => SecretErrorCode::SecretChangePlanRequired,
            Self::ChangePlanInvalid => SecretErrorCode::SecretChangePlanInvalid,
            Self::ChangePlanStale => SecretErrorCode::SecretChangePlanStale,
            Self::MigrationRequired => SecretErrorCode::SecretMigrationRequired,
            Self::LegacySourceInvalid => SecretErrorCode::SecretLegacySourceInvalid,
            Self::LegacyConflict => SecretErrorCode::SecretLegacyConflict,
            Self::LegacyComparisonPending => SecretErrorCode::SecretLegacyComparisonPending,
            Self::MigrationFailed => SecretErrorCode::SecretMigrationFailed,
            Self::Missing => SecretErrorCode::SecretMissing,
            Self::PermissionDenied => SecretErrorCode::SecretPermissionDenied,
            Self::Stale => SecretErrorCode::SecretStale,
            Self::ConfirmationRequired => SecretErrorCode::SecretConfirmationRequired,
            Self::ConfirmationCancelled => SecretErrorCode::SecretConfirmationCancelled,
            Self::ConfirmationExpired => SecretErrorCode::SecretConfirmationExpired,
            Self::ConfirmationReplayed => SecretErrorCode::SecretConfirmationReplayed,
            Self::DeviceMismatch => SecretErrorCode::SecretDeviceMismatch,
            Self::WriteFailed => SecretErrorCode::SecretWriteFailed,
            Self::ReadFailed => SecretErrorCode::SecretReadFailed,
            Self::DeleteFailed => SecretErrorCode::SecretDeleteFailed,
            Self::VerifyFailed => SecretErrorCode::SecretVerifyFailed,
            Self::ProjectionForbidden => SecretErrorCode::SecretProjectionForbidden,
            Self::DependencyChanged => SecretErrorCode::SecretDependencyChanged,
            Self::RecordChanged => SecretErrorCode::SecretRecordChanged,
            Self::BackendChanged => SecretErrorCode::SecretBackendChanged,
            Self::CapabilityExpired => SecretErrorCode::SecretCapabilityExpired,
            Self::CapabilityConsumed => SecretErrorCode::SecretCapabilityConsumed,
            Self::RecoveryNotFound => SecretErrorCode::SecretRecoveryNotFound,
            Self::RecoveryChanged => SecretErrorCode::SecretRecoveryChanged,
            Self::Internal => SecretErrorCode::SecretInternal,
        }
    }
}

impl SecretErrorSources {
    fn none() -> Self {
        Self {
            lock_source: None,
            revocation_source: None,
            backend_unavailable_reason: None,
            recovery: None,
        }
    }

    fn locked(source: SecretLockSource) -> Self {
        Self { lock_source: Some(source), ..Self::none() }
    }

    fn revoked(source: SecretRevocationSource) -> Self {
        Self { revocation_source: Some(source), ..Self::none() }
    }

    fn backend_unavailable(reason: SecretBackendUnavailableReason) -> Self {
        Self { backend_unavailable_reason: Some(reason), ..Self::none() }
    }

    fn recovery(pointer: SecretRecoveryPointer) -> Self {
        Self { recovery: Some(pointer), ..Self::none() }
    }
}

impl SecretTerminalOperationContext {
    fn fresh_action_and_condition(&self) -> (SecretUserAction, SecretActionCondition) {
        match self {
            Self::Summary => (
                SecretUserAction::RefreshSummary,
                SecretActionCondition::General,
            ),
            Self::Capture(BeginCaptureIntent::NewBinding) => (
                SecretUserAction::RetryCapture,
                SecretActionCondition::CaptureFreshOperation,
            ),
            Self::Capture(BeginCaptureIntent::ReplaceBinding) => (
                SecretUserAction::CaptureReplacement,
                SecretActionCondition::CaptureFreshOperation,
            ),
            Self::Capture(BeginCaptureIntent::LegacyReconcile) => (
                SecretUserAction::ResolveLegacyConflict,
                SecretActionCondition::CaptureFreshOperation,
            ),
            Self::Rotation => (SecretUserAction::RetryRotation, SecretActionCondition::RotationFreshOperation),
            Self::CandidateDiscard => (SecretUserAction::DiscardCandidate, SecretActionCondition::CandidateDiscardFreshOperation),
            Self::CandidateTerminalCleanupPending => (SecretUserAction::DiscardCandidate, SecretActionCondition::CandidateTerminalCleanupPending),
            Self::Delete => (SecretUserAction::RefreshDeleteImpact, SecretActionCondition::DeleteReadiness),
            Self::Recovery => (SecretUserAction::RefreshRecoveryImpact, SecretActionCondition::RecoveryReadiness),
            Self::ApplyOrActivation => (SecretUserAction::ReopenChangePlan, SecretActionCondition::ApplyOrActivationPlan),
            Self::StagedImport => (SecretUserAction::ResumeStagedImportCutover, SecretActionCondition::StagedImportResume),
            Self::Validation => (SecretUserAction::RefreshSummary, SecretActionCondition::ValidationFreshOperation),
            Self::Runtime(FixedRuntimeConsumer::ProxyRequest) => (SecretUserAction::RetryProxyRequest, SecretActionCondition::RuntimeFreshOperation),
            Self::Runtime(FixedRuntimeConsumer::UsageProbe) => (SecretUserAction::RetryUsageProbe, SecretActionCondition::RuntimeFreshOperation),
            Self::Runtime(FixedRuntimeConsumer::CodingPlanUsageProbe) => (SecretUserAction::RetryCodingPlanUsageProbe, SecretActionCondition::RuntimeFreshOperation),
            Self::Runtime(FixedRuntimeConsumer::ModelFetch) => (SecretUserAction::RetryModelFetch, SecretActionCondition::RuntimeFreshOperation),
        }
    }
}

impl SecretInternalError {
    // Sole constructor. The code match is exhaustive with no wildcard; adding
    // a stable code cannot compile until retry/action/effect handling is added.
    fn checked(
        code: SecretErrorCode,
        context: SecretTerminalOperationContext,
        sources: SecretErrorSources,
    ) -> Result<Self, SecretErrorFactoryViolation> {
        let mut retryable = match code {
            SecretErrorCode::SecretRequestInvalid
            | SecretErrorCode::SecretRefInvalid
            | SecretErrorCode::SecretOwnerKindUnsupported
            | SecretErrorCode::SecretOwnerNamespaceUnsupported
            | SecretErrorCode::SecretOwnerNotFound
            | SecretErrorCode::SecretUnsupportedPurpose
            | SecretErrorCode::SecretConsumerUnsupported
            | SecretErrorCode::SecretCandidateNotFound
            | SecretErrorCode::SecretCandidateExpired
            | SecretErrorCode::SecretCandidateConsumed
            | SecretErrorCode::SecretChangePlanInvalid
            | SecretErrorCode::SecretLegacySourceInvalid
            | SecretErrorCode::SecretLegacyConflict
            | SecretErrorCode::SecretMissing
            | SecretErrorCode::SecretRevoked
            | SecretErrorCode::SecretDeviceMismatch
            | SecretErrorCode::SecretRecoveryNotFound
            | SecretErrorCode::SecretProjectionForbidden => false,
            SecretErrorCode::SecretOwnerConflict
            | SecretErrorCode::SecretOperationBusy
            | SecretErrorCode::SecretInputCancelled
            | SecretErrorCode::SecretInputInvalid
            | SecretErrorCode::SecretChangePlanRequired
            | SecretErrorCode::SecretChangePlanStale
            | SecretErrorCode::SecretMigrationRequired
            | SecretErrorCode::SecretLegacyComparisonPending
            | SecretErrorCode::SecretMigrationFailed
            | SecretErrorCode::SecretLocked
            | SecretErrorCode::SecretPermissionDenied
            | SecretErrorCode::SecretBackendUnavailable
            | SecretErrorCode::SecretStale
            | SecretErrorCode::SecretConfirmationRequired
            | SecretErrorCode::SecretConfirmationCancelled
            | SecretErrorCode::SecretConfirmationExpired
            | SecretErrorCode::SecretConfirmationReplayed
            | SecretErrorCode::SecretWriteFailed
            | SecretErrorCode::SecretReadFailed
            | SecretErrorCode::SecretDeleteFailed
            | SecretErrorCode::SecretVerifyFailed
            | SecretErrorCode::SecretDependencyChanged
            | SecretErrorCode::SecretRecordChanged
            | SecretErrorCode::SecretBackendChanged
            | SecretErrorCode::SecretCapabilityExpired
            | SecretErrorCode::SecretCapabilityConsumed
            | SecretErrorCode::SecretRecoveryChanged
            | SecretErrorCode::SecretOperationRecoveryRequired
            | SecretErrorCode::SecretInternal => true,
        };
        let capture_selection = matches!(
            &context,
            SecretTerminalOperationContext::Capture(_)
                | SecretTerminalOperationContext::Rotation
        );
        let backend_selection_action = match &context {
            SecretTerminalOperationContext::Capture(BeginCaptureIntent::NewBinding) => SecretUserAction::ChooseBackend,
            SecretTerminalOperationContext::Capture(BeginCaptureIntent::ReplaceBinding) => SecretUserAction::CaptureReplacement,
            SecretTerminalOperationContext::Capture(BeginCaptureIntent::LegacyReconcile) => SecretUserAction::ResolveLegacyConflict,
            SecretTerminalOperationContext::Rotation => SecretUserAction::RetryRotation,
            SecretTerminalOperationContext::Summary
            | SecretTerminalOperationContext::CandidateDiscard
            | SecretTerminalOperationContext::CandidateTerminalCleanupPending
            | SecretTerminalOperationContext::Delete
            | SecretTerminalOperationContext::Recovery
            | SecretTerminalOperationContext::ApplyOrActivation
            | SecretTerminalOperationContext::StagedImport
            | SecretTerminalOperationContext::Validation
            | SecretTerminalOperationContext::Runtime(_) => SecretUserAction::RefreshSummary,
        };
        let (fresh_action, mut condition) = context.fresh_action_and_condition();
        let action = match code {
            SecretErrorCode::SecretRequestInvalid
            | SecretErrorCode::SecretOwnerKindUnsupported
            | SecretErrorCode::SecretOwnerNamespaceUnsupported
            | SecretErrorCode::SecretUnsupportedPurpose
            | SecretErrorCode::SecretConsumerUnsupported => SecretUserAction::None,
            SecretErrorCode::SecretCandidateExpired
            | SecretErrorCode::SecretCandidateNotFound
            | SecretErrorCode::SecretCandidateConsumed
            | SecretErrorCode::SecretRefInvalid
            | SecretErrorCode::SecretOwnerNotFound
            | SecretErrorCode::SecretOwnerConflict
            | SecretErrorCode::SecretStale
            | SecretErrorCode::SecretLegacyComparisonPending => SecretUserAction::RefreshSummary,
            SecretErrorCode::SecretChangePlanRequired
            | SecretErrorCode::SecretChangePlanInvalid
            | SecretErrorCode::SecretChangePlanStale
            | SecretErrorCode::SecretProjectionForbidden => SecretUserAction::ReopenChangePlan,
            SecretErrorCode::SecretMigrationRequired
            | SecretErrorCode::SecretLegacySourceInvalid
            | SecretErrorCode::SecretLegacyConflict
            | SecretErrorCode::SecretMigrationFailed => SecretUserAction::ResolveLegacyConflict,
            SecretErrorCode::SecretMissing => SecretUserAction::CaptureReplacement,
            SecretErrorCode::SecretRevoked => match sources.revocation_source {
                Some(SecretRevocationSource::UserDelete) => SecretUserAction::CaptureReplacement,
                Some(SecretRevocationSource::SupersededByRotation) => SecretUserAction::None,
                Some(SecretRevocationSource::CentralBackend) => SecretUserAction::ContactAdministrator,
                Some(SecretRevocationSource::DeviceAdministration) => SecretUserAction::OpenBackendSettings,
                None => return Err(SecretErrorFactoryViolation),
            },
            SecretErrorCode::SecretLocked => match sources.lock_source {
                Some(SecretLockSource::FyAgentPolicy) => SecretUserAction::UnlockFyAgent,
                Some(SecretLockSource::Backend) => SecretUserAction::UnlockBackend,
                None => return Err(SecretErrorFactoryViolation),
            },
            SecretErrorCode::SecretPermissionDenied => SecretUserAction::RequestPermission,
            SecretErrorCode::SecretBackendUnavailable => match sources.backend_unavailable_reason {
                Some(SecretBackendUnavailableReason::HardwareUnregistered) if capture_selection => backend_selection_action,
                Some(SecretBackendUnavailableReason::HardwareUnregistered) => SecretUserAction::OpenBackendSettings,
                Some(SecretBackendUnavailableReason::HardwareDisconnected) => SecretUserAction::ReconnectDevice,
                Some(SecretBackendUnavailableReason::OsStoreUnavailable) => SecretUserAction::OpenBackendSettings,
                Some(SecretBackendUnavailableReason::CentralServiceUnavailable) => SecretUserAction::ContactAdministrator,
                None => return Err(SecretErrorFactoryViolation),
            },
            SecretErrorCode::SecretConfirmationRequired => SecretUserAction::ConfirmDevice,
            SecretErrorCode::SecretDeviceMismatch => SecretUserAction::ReconnectDevice,
            SecretErrorCode::SecretBackendChanged if capture_selection => backend_selection_action,
            SecretErrorCode::SecretRecoveryNotFound
            | SecretErrorCode::SecretRecoveryChanged => SecretUserAction::RefreshRecoveryImpact,
            SecretErrorCode::SecretOperationRecoveryRequired => {
                if sources.recovery.is_some() { SecretUserAction::CompleteRecovery } else { fresh_action }
            }
            SecretErrorCode::SecretOperationBusy
            | SecretErrorCode::SecretInputCancelled
            | SecretErrorCode::SecretInputInvalid
            | SecretErrorCode::SecretConfirmationCancelled
            | SecretErrorCode::SecretConfirmationExpired
            | SecretErrorCode::SecretConfirmationReplayed
            | SecretErrorCode::SecretWriteFailed
            | SecretErrorCode::SecretReadFailed
            | SecretErrorCode::SecretDeleteFailed
            | SecretErrorCode::SecretVerifyFailed
            | SecretErrorCode::SecretDependencyChanged
            | SecretErrorCode::SecretRecordChanged
            | SecretErrorCode::SecretBackendChanged
            | SecretErrorCode::SecretCapabilityExpired
            | SecretErrorCode::SecretCapabilityConsumed
            | SecretErrorCode::SecretInternal => fresh_action,
        };
        let effect = if code == SecretErrorCode::SecretOperationRecoveryRequired
            && sources.recovery.is_some()
        {
            SecretEffect::CleanupPending
        } else {
            SecretEffect::None
        };
        if code == SecretErrorCode::SecretBackendUnavailable {
            retryable = match sources.backend_unavailable_reason {
                Some(SecretBackendUnavailableReason::HardwareUnregistered) => capture_selection,
                Some(SecretBackendUnavailableReason::HardwareDisconnected)
                | Some(SecretBackendUnavailableReason::OsStoreUnavailable) => true,
                Some(SecretBackendUnavailableReason::CentralServiceUnavailable) => false,
                None => return Err(SecretErrorFactoryViolation),
            };
        }
        if capture_selection
            && (code == SecretErrorCode::SecretBackendChanged
                || (code == SecretErrorCode::SecretBackendUnavailable
                    && sources.backend_unavailable_reason
                        == Some(SecretBackendUnavailableReason::HardwareUnregistered)))
        {
            condition = SecretActionCondition::CaptureBackendSelection;
        }
        let source_shape_valid = match code {
            SecretErrorCode::SecretLocked => sources.lock_source.is_some()
                && sources.revocation_source.is_none()
                && sources.backend_unavailable_reason.is_none()
                && sources.recovery.is_none(),
            SecretErrorCode::SecretRevoked => sources.lock_source.is_none()
                && sources.revocation_source.is_some()
                && sources.backend_unavailable_reason.is_none()
                && sources.recovery.is_none(),
            SecretErrorCode::SecretBackendUnavailable => sources.lock_source.is_none()
                && sources.revocation_source.is_none()
                && sources.backend_unavailable_reason.is_some()
                && sources.recovery.is_none(),
            SecretErrorCode::SecretOperationRecoveryRequired => sources.lock_source.is_none()
                && sources.revocation_source.is_none()
                && sources.backend_unavailable_reason.is_none()
                && match (&context, &sources.recovery) {
                    (
                        SecretTerminalOperationContext::CandidateTerminalCleanupPending,
                        None,
                    ) => true,
                    (
                        SecretTerminalOperationContext::CandidateTerminalCleanupPending,
                        Some(_),
                    ) => false,
                    (_, Some(_)) => true,
                    (_, None) => false,
                },
            SecretErrorCode::SecretRequestInvalid
            | SecretErrorCode::SecretRefInvalid
            | SecretErrorCode::SecretOwnerKindUnsupported
            | SecretErrorCode::SecretOwnerNamespaceUnsupported
            | SecretErrorCode::SecretOwnerNotFound
            | SecretErrorCode::SecretOwnerConflict
            | SecretErrorCode::SecretOperationBusy
            | SecretErrorCode::SecretUnsupportedPurpose
            | SecretErrorCode::SecretConsumerUnsupported
            | SecretErrorCode::SecretInputCancelled
            | SecretErrorCode::SecretInputInvalid
            | SecretErrorCode::SecretCandidateNotFound
            | SecretErrorCode::SecretCandidateExpired
            | SecretErrorCode::SecretCandidateConsumed
            | SecretErrorCode::SecretChangePlanRequired
            | SecretErrorCode::SecretChangePlanInvalid
            | SecretErrorCode::SecretChangePlanStale
            | SecretErrorCode::SecretMigrationRequired
            | SecretErrorCode::SecretLegacySourceInvalid
            | SecretErrorCode::SecretLegacyConflict
            | SecretErrorCode::SecretLegacyComparisonPending
            | SecretErrorCode::SecretMigrationFailed
            | SecretErrorCode::SecretMissing
            | SecretErrorCode::SecretPermissionDenied
            | SecretErrorCode::SecretStale
            | SecretErrorCode::SecretConfirmationRequired
            | SecretErrorCode::SecretConfirmationCancelled
            | SecretErrorCode::SecretConfirmationExpired
            | SecretErrorCode::SecretConfirmationReplayed
            | SecretErrorCode::SecretDeviceMismatch
            | SecretErrorCode::SecretWriteFailed
            | SecretErrorCode::SecretReadFailed
            | SecretErrorCode::SecretDeleteFailed
            | SecretErrorCode::SecretVerifyFailed
            | SecretErrorCode::SecretProjectionForbidden
            | SecretErrorCode::SecretDependencyChanged
            | SecretErrorCode::SecretRecordChanged
            | SecretErrorCode::SecretBackendChanged
            | SecretErrorCode::SecretCapabilityExpired
            | SecretErrorCode::SecretCapabilityConsumed
            | SecretErrorCode::SecretRecoveryNotFound
            | SecretErrorCode::SecretRecoveryChanged
            | SecretErrorCode::SecretInternal => sources.lock_source.is_none()
                && sources.revocation_source.is_none()
                && sources.backend_unavailable_reason.is_none()
                && sources.recovery.is_none(),
        };
        if !source_shape_valid { return Err(SecretErrorFactoryViolation); }
        Ok(Self {
            code,
            retryable,
            action,
            effect,
            condition,
            lock_source: sources.lock_source,
            revocation_source: sources.revocation_source,
            backend_unavailable_reason: sources.backend_unavailable_reason,
            recovery: sources.recovery,
        })
    }

    fn known(code: SecretSourceFreeErrorCode, context: SecretTerminalOperationContext) -> Self {
        Self::checked(code.stable_code(), context, SecretErrorSources::none())
            .expect("closed factory tuple")
    }

    pub(in crate::secret) fn input_invalid() -> Self {
        Self::known(SecretSourceFreeErrorCode::InputInvalid, SecretTerminalOperationContext::Summary)
    }
    pub(in crate::secret) fn recovery_changed() -> Self {
        Self::known(SecretSourceFreeErrorCode::RecoveryChanged, SecretTerminalOperationContext::Recovery)
    }
    pub(in crate::secret) fn dependency_changed() -> Self {
        Self::known(SecretSourceFreeErrorCode::DependencyChanged, SecretTerminalOperationContext::Summary)
    }
    pub(in crate::secret) fn capability_consumed() -> Self {
        Self::known(SecretSourceFreeErrorCode::CapabilityConsumed, SecretTerminalOperationContext::ApplyOrActivation)
    }

    pub(in crate::secret) fn terminal_operation_failure(
        code: SecretSourceFreeErrorCode,
        context: SecretTerminalOperationContext,
    ) -> Self {
        Self::known(code, context)
    }

    pub(in crate::secret) fn locked(
        context: SecretTerminalOperationContext,
        source: SecretLockSource,
    ) -> Self {
        Self::checked(
            SecretErrorCode::SecretLocked,
            context,
            SecretErrorSources::locked(source),
        )
        .expect("lock source is required and exact")
    }

    pub(in crate::secret) fn revoked(
        context: SecretTerminalOperationContext,
        source: SecretRevocationSource,
    ) -> Self {
        Self::checked(
            SecretErrorCode::SecretRevoked,
            context,
            SecretErrorSources::revoked(source),
        )
        .expect("revocation source is required and exact")
    }

    pub(in crate::secret) fn backend_unavailable(
        context: SecretTerminalOperationContext,
        reason: SecretBackendUnavailableReason,
    ) -> Self {
        Self::checked(
            SecretErrorCode::SecretBackendUnavailable,
            context,
            SecretErrorSources::backend_unavailable(reason),
        )
        .expect("backend-unavailable reason is required and exact")
    }

    pub(in crate::secret) fn operation_recovery_required(
        pointer: SecretRecoveryPointer,
    ) -> Self {
        Self::checked(
            SecretErrorCode::SecretOperationRecoveryRequired,
            SecretTerminalOperationContext::Recovery,
            SecretErrorSources::recovery(pointer),
        )
        .expect("general recovery requires exactly one typed pointer")
    }

    pub(in crate::secret) fn candidate_terminal_cleanup_pending() -> Self {
        Self::checked(
            SecretErrorCode::SecretOperationRecoveryRequired,
            SecretTerminalOperationContext::CandidateTerminalCleanupPending,
            SecretErrorSources::none(),
        )
        .expect("candidate terminal cleanup is the sole pointer-free recovery issue")
    }
}

// Compile-shape scanner rule: a `SecretInternalError` struct literal is allowed
// exactly once inside `SecretInternalError::checked`; fields have no getters and no module
// may re-export the type or create a literal. Error-to-wire conversion reads it
// only in the owner module and projects the §11-validated tuple without accepting
// any replacement code/action/source fields.

impl std::fmt::Debug for SecretInternalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SecretInternalError(stable-code-only)")
    }
}

impl std::fmt::Display for SecretInternalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("secret operation failed")
    }
}

impl std::error::Error for SecretInternalError {}

fn validate_material(
    bytes: &[u8],
    purpose: SecretPurpose,
) -> Result<(), SecretInternalError> {
    if purpose != SecretPurpose::CodexApiKey
        || bytes.is_empty()
        || bytes.len() > 2560
        || bytes.contains(&0)
        || std::str::from_utf8(bytes).is_err()
    {
        Err(SecretInternalError::input_invalid())
    } else {
        Ok(())
    }
}

// True owner: crate::secret::material. crate::secret does not re-export
// this type. Only capture and crate::secret::backend may construct/consume it.
pub(in crate::secret) struct SecretMaterial(Zeroizing<Vec<u8>>);

// This crate-private seal and public-in-secret callback trait are defined in
// crate::secret::backend. crate::secret::material imports only the callback
// trait for SecretMaterial::write_to_sealed_callback. backend.rs implements
// only its platform callback; each allowlisted lane adapter implements one
// seal/base/route triple without making the seal public outside the crate.
pub(crate) mod backend_material_callback_sealed {
    pub(crate) trait Sealed {}
}

pub(crate) trait BackendMaterialWriteCallback:
    backend_material_callback_sealed::Sealed
{
    // Every implementer and receipt type is listed in 7.1.1. No callback may
    // return bytes/String/header/material or store the borrow beyond this call.
    type Receipt;
    fn write_once(self, material: &[u8]) -> Self::Receipt;
}

// #35 core owns only these route traits. Concrete #41, main-integration and
// runtime types implement the seal + base callback + exactly one marker in
// their lane-owned adapter module. backend.rs therefore compiles without
// naming or constructing any lane's not-yet-landed concrete callback type.
pub(crate) trait ApplyMaterialAdapter: BackendMaterialWriteCallback {}
pub(crate) trait ActivationEqualityMaterialAdapter: BackendMaterialWriteCallback {}
pub(crate) trait RecoveryEqualityMaterialAdapter: BackendMaterialWriteCallback {}
pub(crate) trait StagedImportEqualityMaterialAdapter: BackendMaterialWriteCallback {}
pub(crate) trait MigrationEqualityMaterialAdapter: BackendMaterialWriteCallback {}
pub(crate) trait ProxyMaterialAdapter: BackendMaterialWriteCallback {}
pub(crate) trait UsageMaterialAdapter: BackendMaterialWriteCallback {}
pub(crate) trait CodingPlanMaterialAdapter: BackendMaterialWriteCallback {}
pub(crate) trait ModelFetchMaterialAdapter: BackendMaterialWriteCallback {}

impl SecretMaterial {
    pub(in crate::secret) fn from_native_input(
        bytes: Vec<u8>,
        purpose: SecretPurpose,
    ) -> Result<Self, SecretInternalError> {
        // Own and zeroize before any validation branch can fail.
        let bytes = Zeroizing::new(bytes);
        validate_material(bytes.as_slice(), purpose)?;
        Ok(Self(bytes))
    }

    pub(in crate::secret) fn ct_eq(&self, other: &Self) -> bool {
        bool::from(self.0.as_slice().ct_eq(other.0.as_slice()))
    }

    pub(in crate::secret) fn ct_eq_slice(&self, other: &[u8]) -> bool {
        bool::from(self.0.as_slice().ct_eq(other))
    }

    pub(in crate::secret) fn write_to_sealed_callback<C>(
        self,
        callback: C,
    ) -> C::Receipt
    where
        C: BackendMaterialWriteCallback,
    {
        callback.write_once(self.0.as_slice())
    }
}

// No Serialize, Deserialize, Clone or unrestricted Debug implementation.

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct SecretStoreRevision(u64);

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct BackendDeleteAppliedRevision(u64);

impl BackendDeleteAppliedRevision {
    fn checked(value: u64) -> Result<Self, SecretInternalError> {
        if (1..=JS_SAFE_INTEGER_MAX).contains(&value) {
            Ok(Self(value))
        } else {
            Err(SecretInternalError::input_invalid())
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct BackendDeleteAppliedCas {
    revision: BackendDeleteAppliedRevision,
    digest: RecoveryStructureDigest,
}

impl BackendDeleteAppliedCas {
    fn checked_from_durable_backend_applied(
        revision: BackendDeleteAppliedRevision,
        digest: RecoveryStructureDigest,
        journal: &DurableSecretOperationJournal,
    ) -> Result<Self, SecretInternalError> {
        let _ = journal;
        todo!("accept only the exact just-persisted backendApplied phase and its credential-free structural preimage")
    }
}

pub(crate) struct BackendDeleteAppliedCasReservation {
    operation_id: SecretOperationId,
    expected_revision: BackendDeleteAppliedRevision,
    _private: (),
}

// The broker reserves only the next operation-bound revision before any
// prompt; it cannot predict a receipt-derived digest. After delete is durably
// journaled, authority mints the actual CAS and the missing-readback authorize
// method must consume this reservation via consume_fulfilled_by before it can
// pass that actual CAS into AuthorizedBackendMissingReadback.

impl BackendDeleteAppliedCasReservation {
    fn consume_fulfilled_by(
        self,
        operation_id: &SecretOperationId,
        actual: &BackendDeleteAppliedCas,
    ) -> Result<(), SecretInternalError> {
        if &self.operation_id == operation_id
            && &self.expected_revision == &actual.revision
        {
            Ok(())
        } else {
            Err(SecretInternalError::dependency_changed())
        }
    }
}

impl SecretStoreRevision {
    pub(in crate::secret) fn parse(value: u64) -> Result<Self, SecretInternalError> {
        if (1..=JS_SAFE_INTEGER_MAX).contains(&value) {
            Ok(Self(value))
        } else {
            Err(SecretInternalError::input_invalid())
        }
    }


    pub(in crate::secret) fn get(self) -> u64 {
        self.0
    }
}

// Native-only: SecretStoreRevision has no Serialize/Deserialize implementation.

// The locator and handle definitions live in crate::secret::backend. This
// private locator type is not re-exported from that module.
struct BackendRecordLocator(String);

impl BackendRecordLocator {
    fn parse(value: String) -> Result<Self, SecretInternalError> {
        let bytes = value.as_bytes();
        let valid = (1..=128).contains(&bytes.len())
            && bytes[0].is_ascii_alphanumeric()
            && bytes.iter().all(|byte| {
                byte.is_ascii_alphanumeric()
                    || matches!(*byte, b'.' | b'_' | b':' | b'@' | b'=' | b'-')
            })
            && !credential_shaped_ascii(&value);
        if valid {
            Ok(Self(value))
        } else {
            Err(SecretInternalError::input_invalid())
        }
    }

    fn as_str(&self) -> &str {
        &self.0
    }
}
// This private repr is the complete authorization algebra. The wrapper has no
// Serialize/Deserialize/Clone/Debug and its sole factory is in this module.
// Consequently an operation owner can request preparation but cannot forge,
// narrow, widen or transplant the scope returned by the registered backend.
enum BackendAuthorizationScopeKind {
    Apply {
        role: SecretApplyRole,
        projection_digest: SecretProjectionDigest,
        owner: SecretOwner,
        owner_binding_revision: SecretOwnerBindingRevision,
        binding_revision: SecretBindingRevision,
        consumer: SecretChangePlanApplyConsumer,
        target_sink: SecretChangePlanApplySink,
        live_sink_id: CodexLiveSecretSinkId,
    },
    Runtime {
        consumer: FixedRuntimeConsumer,
        sink: FixedRuntimeSink,
        owner: SecretOwner,
        owner_binding_revision: SecretOwnerBindingRevision,
        binding_revision: SecretBindingRevision,
    },
    Activation {
        candidate_id: SecretCandidateId,
        candidate_revision: SecretCandidateRevision,
        projection_digest: SecretProjectionDigest,
        comparison_policy: LegacyActivationComparisonPolicy,
        slot: ActivationConfirmationSlot,
    },
    Recovery {
        recovery_id: SecretRecoveryId,
        recovery_kind: SecretRecoveryKind,
        recovery_cas: SecretRecoveryCas,
        slot: RecoveryConfirmationSlot,
    },
    Migration {
        report_id: SecretMigrationReportId,
        owner: SecretOwner,
        comparison_policy: LegacyActivationComparisonPolicy,
    },
    StagedImport {
        authority: StagedImportBackendAuthorityScope,
        candidate_id: SecretCandidateId,
        projection_digest: SecretProjectionDigest,
        comparison_policy: LegacyActivationComparisonPolicy,
        slot: StagedImportConfirmationSlot,
    },
    General {
        operation: SecretNonApplyBackendOperation,
        owner: SecretOwner,
    },
}

pub(crate) enum FixedRuntimeConsumer {
    ProxyRequest,
    UsageProbe,
    CodingPlanUsageProbe,
    ModelFetch,
}

impl FixedRuntimeConsumer {
    fn required_record_consumer(&self) -> SecretRuntimeConsumer {
        match self {
            Self::ProxyRequest => SecretRuntimeConsumer::ProxyRequest,
            Self::UsageProbe => SecretRuntimeConsumer::UsageProbe,
            Self::CodingPlanUsageProbe => {
                SecretRuntimeConsumer::CodingPlanUsageProbe
            }
            Self::ModelFetch => SecretRuntimeConsumer::ModelFetch,
        }
    }
}

pub(crate) enum FixedRuntimeSink {
    ProcessMemory,
}

struct BackendAuthorizationScopeRepr {
    registered_backend: RegisteredBackendHandleBinding,
    device_instance_id: DeviceInstanceId,
    device_store_instance_id: std::sync::Arc<DeviceSecretStoreInstanceId>,
    operation_id: SecretOperationId,
    kind: BackendAuthorizationScopeKind,
    terminal_error_context: SecretTerminalOperationContext,
    secret_ref: SecretRef,
    record_revision: SecretRecordRevision,
    store_revision: SecretStoreRevision,
    binding_set_cas: SecretBindingSetCas,
    backend_instance_id: SecretBackendInstanceId,
    backend_generation: SecretBackendGeneration,
    device_binding_generation: DeviceBindingGeneration,
    capability_revision: CapabilityRevision,
    confirmation: PhysicalConfirmation,
    expires_at: UtcTimestamp,
}

pub(in crate::secret) struct BackendAuthorizationScope(
    BackendAuthorizationScopeRepr,
);

impl BackendAuthorizationScope {
    // Private to crate::secret::backend. It copies the record identity and the
    // closed operation context after their equality has been checked.
    fn mint_from_context(
        backend: &BackendInstanceHandle,
        record: &BackendRecordHandle,
        context: BrokeredBackendOperationContext,
    ) -> Result<Self, SecretInternalError> {
        todo!("unwrap only inside backend; validate complete brokered context/record/registered-handle/store tuple and its exact closed terminal-error context; staged arm consumes live-authority match; mint sealed scope")
    }

    fn into_terminal_error_context(self) -> SecretTerminalOperationContext {
        self.0.terminal_error_context
    }

    fn matches(
        &self,
        backend: &BackendInstanceHandle,
        record: &BackendRecordHandle,
        operation_id: &SecretOperationId,
        now: &UtcTimestamp,
    ) -> bool {
        todo!("Arc identity plus instance/generation/record/store/binding/device/capability/operation/route/expiry comparison; never partial")
    }

    fn assert_registered_handle(
        &self,
        backend: &BackendInstanceHandle,
    ) -> Result<(), SecretInternalError> {
        self.0.registered_backend.assert_same(backend)
    }

    fn validate_confirmation_requirement(
        &self,
        backend: &BackendInstanceHandle,
        operation: SecretBackendOperation,
        confirmation: PhysicalConfirmation,
        requirement: &PlatformConfirmationRequirement,
    ) -> Result<(), SecretInternalError> {
        let _ = (backend, operation, confirmation, requirement);
        todo!("exact registered object/device/operation/policy/timeout/prompt/scope-expiry validation")
    }

    fn validate_pending_requirement(
        &self,
        backend: &BackendInstanceHandle,
        requirement: &BackendPendingRequirementIdentity,
        now: &UtcTimestamp,
        termination: Option<&PendingConfirmationTermination>,
    ) -> Result<(), SecretInternalError> {
        let _ = (backend, requirement, now, termination);
        todo!("same registered object/instance/generation/device/operation/confirmation/timeout/prompt; confirm requires unexpired, Expired termination requires elapsed deadline, other termination consumes exact row")
    }

    fn platform_requirement(
        &self,
    ) -> Result<PlatformOperationRequirement<'_>, SecretInternalError> {
        let operation = match &self.0.kind {
            BackendAuthorizationScopeKind::Apply { .. }
            | BackendAuthorizationScopeKind::Runtime { .. } => {
                SecretBackendOperation::ResolveForApply
            }
            BackendAuthorizationScopeKind::Activation { slot, .. } => match slot {
                ActivationConfirmationSlot::CandidateRead => {
                    SecretBackendOperation::ResolveForApply
                }
                ActivationConfirmationSlot::OldRecordDelete => {
                    SecretBackendOperation::Delete
                }
                ActivationConfirmationSlot::OldRecordMissingReadback => {
                    SecretBackendOperation::Validate
                }
            },
            BackendAuthorizationScopeKind::Recovery { slot, .. } => match slot {
                RecoveryConfirmationSlot::ActiveRecordRead => {
                    SecretBackendOperation::ResolveForApply
                }
                RecoveryConfirmationSlot::OldRecordDelete
                | RecoveryConfirmationSlot::UncommittedRecordDelete
                | RecoveryConfirmationSlot::AdmittedRecordDelete => {
                    SecretBackendOperation::Delete
                }
                RecoveryConfirmationSlot::OldRecordMissingReadback
                | RecoveryConfirmationSlot::UncommittedRecordMissingReadback
                | RecoveryConfirmationSlot::AdmittedRecordMissingReadback => {
                    SecretBackendOperation::Validate
                }
            },
            BackendAuthorizationScopeKind::Migration { .. }
            | BackendAuthorizationScopeKind::StagedImport { .. } => {
                SecretBackendOperation::ResolveForApply
            }
            BackendAuthorizationScopeKind::General { operation, .. } => match operation {
                SecretNonApplyBackendOperation::CaptureVerify => {
                    SecretBackendOperation::CaptureVerify
                }
                SecretNonApplyBackendOperation::Validate => SecretBackendOperation::Validate,
                SecretNonApplyBackendOperation::CandidateDiscard {
                    slot: CandidateDiscardConfirmationSlot::RecordDelete,
                    ..
                }
                | SecretNonApplyBackendOperation::DirectDelete => {
                    SecretBackendOperation::Delete
                }
                SecretNonApplyBackendOperation::CandidateDiscard {
                    slot: CandidateDiscardConfirmationSlot::RecordMissingReadback,
                    ..
                } => SecretBackendOperation::Validate,
                SecretNonApplyBackendOperation::Revoke => SecretBackendOperation::Revoke,
            },
        };
        Ok(PlatformOperationRequirement {
            scope: self,
            operation,
            confirmation: self.0.confirmation,
        })
    }

    fn require_route(&self, route: AuthorizedReadRoute) -> Result<(), SecretInternalError> {
        let matches = match (&self.0.kind, route) {
            (BackendAuthorizationScopeKind::Apply { .. }, AuthorizedReadRoute::Apply)
            | (BackendAuthorizationScopeKind::Activation { slot: ActivationConfirmationSlot::CandidateRead, .. }, AuthorizedReadRoute::Activation)
            | (BackendAuthorizationScopeKind::Recovery { slot: RecoveryConfirmationSlot::ActiveRecordRead, .. }, AuthorizedReadRoute::Recovery)
            | (BackendAuthorizationScopeKind::Migration { .. }, AuthorizedReadRoute::Migration)
            | (BackendAuthorizationScopeKind::StagedImport { .. }, AuthorizedReadRoute::StagedImport)
            | (BackendAuthorizationScopeKind::Runtime { consumer: FixedRuntimeConsumer::ProxyRequest, sink: FixedRuntimeSink::ProcessMemory, .. }, AuthorizedReadRoute::Proxy)
            | (BackendAuthorizationScopeKind::Runtime { consumer: FixedRuntimeConsumer::UsageProbe, sink: FixedRuntimeSink::ProcessMemory, .. }, AuthorizedReadRoute::Usage)
            | (BackendAuthorizationScopeKind::Runtime { consumer: FixedRuntimeConsumer::CodingPlanUsageProbe, sink: FixedRuntimeSink::ProcessMemory, .. }, AuthorizedReadRoute::CodingPlan)
            | (BackendAuthorizationScopeKind::Runtime { consumer: FixedRuntimeConsumer::ModelFetch, sink: FixedRuntimeSink::ProcessMemory, .. }, AuthorizedReadRoute::ModelFetch)
            | (BackendAuthorizationScopeKind::General { operation: SecretNonApplyBackendOperation::Validate, .. }, AuthorizedReadRoute::Validation) => true,
            _ => false,
        };
        if matches {
            Ok(())
        } else {
            Err(SecretInternalError::dependency_changed())
        }
    }

    fn require_delete_mode(
        &self,
        mode: BackendDeleteMode,
    ) -> Result<(), SecretInternalError> {
        let _ = mode;
        todo!("exact activation/recovery/general delete-or-revoke scope and mode; CandidateDiscard permits Delete only for RecordDelete and rejects RecordMissingReadback")
    }

    fn require_revoke_observation(&self) -> Result<(), SecretInternalError> {
        match &self.0.kind {
            BackendAuthorizationScopeKind::General {
                operation: SecretNonApplyBackendOperation::Revoke,
                ..
            } => Ok(()),
            _ => Err(SecretInternalError::dependency_changed()),
        }
    }

    fn require_missing_readback(&self) -> Result<(), SecretInternalError> {
        match &self.0.kind {
            BackendAuthorizationScopeKind::Activation {
                slot: ActivationConfirmationSlot::OldRecordMissingReadback,
                ..
            }
            | BackendAuthorizationScopeKind::Recovery {
                slot: RecoveryConfirmationSlot::OldRecordMissingReadback
                    | RecoveryConfirmationSlot::UncommittedRecordMissingReadback
                    | RecoveryConfirmationSlot::AdmittedRecordMissingReadback,
                ..
            }
            | BackendAuthorizationScopeKind::General {
                operation: SecretNonApplyBackendOperation::CandidateDiscard {
                    slot: CandidateDiscardConfirmationSlot::RecordMissingReadback,
                    ..
                },
                ..
            } => Ok(()),
            _ => Err(SecretInternalError::dependency_changed()),
        }
    }
}

#[derive(Clone, Copy)]
enum AuthorizedReadRoute {
    Apply,
    Activation,
    Recovery,
    Migration,
    StagedImport,
    Proxy,
    Usage,
    CodingPlan,
    ModelFetch,
    Validation,
}

pub(crate) struct BackendAuthorizationHandle {
    authorization_id: u128,
    scope: BackendAuthorizationScope,
}
pub(crate) struct BackendPendingConfirmation {
    pending_id: u128,
    scope: BackendAuthorizationScope,
    requirement: BackendPendingRequirementIdentity,
}

struct BackendPendingRequirementIdentity {
    backend_instance_id: SecretBackendInstanceId,
    backend_generation: SecretBackendGeneration,
    operation: SecretBackendOperation,
    confirmation: PhysicalConfirmation,
    device: SecretDeviceDisplay,
    timeout_seconds: ConfirmationTimeoutSeconds,
    prompt_key: HardwarePromptKey,
    expires_at: UtcTimestamp,
}

pub(in crate::secret) struct ConsumedBackendAuthorization {
    authorization_id: u128,
    scope: BackendAuthorizationScope,
}

impl BackendAuthorizationHandle {
    // Private to crate::secret::backend::authorization. Only the registered
    // backend wrapper mints after ready/confirmed platform evidence.
    fn mint(
        authorization_id: u128,
        scope: BackendAuthorizationScope,
    ) -> Self {
        Self {
            authorization_id,
            scope,
        }
    }

    fn consume(
        self,
        backend: &BackendInstanceHandle,
        record: &BackendRecordHandle,
        operation_id: &SecretOperationId,
        now: &UtcTimestamp,
    ) -> Result<ConsumedBackendAuthorization, SecretInternalError> {
        if !self.scope.matches(backend, record, operation_id, now) {
            return Err(SecretInternalError::terminal_operation_failure(
                SecretSourceFreeErrorCode::DependencyChanged,
                self.scope.into_terminal_error_context(),
            ));
        }
        Ok(ConsumedBackendAuthorization {
            authorization_id: self.authorization_id,
            scope: self.scope,
        })
    }
}

impl BackendPendingConfirmation {
    // Same owner/privacy as BackendAuthorizationHandle::mint.
    fn mint(
        pending_id: u128,
        scope: BackendAuthorizationScope,
        requirement: BackendPendingRequirementIdentity,
    ) -> Self {
        Self {
            pending_id,
            scope,
            requirement,
        }
    }
}

// Locator/auth/pending/consumed types have private fields and no
// Serialize/Deserialize/Clone/Debug. Registries store non-material ids/nonces
// only. Mint/consume calls are scanner-allowlisted to crate::secret::backend.

pub(crate) struct BackendRecordHandle {
    registered_backend: RegisteredBackendHandleBinding,
    device_instance_id: DeviceInstanceId,
    device_store_instance_id: std::sync::Arc<DeviceSecretStoreInstanceId>,
    secret_ref: SecretRef,
    purpose: SecretPurpose,
    record_revision: SecretRecordRevision,
    instance_id: SecretBackendInstanceId,
    backend_generation: SecretBackendGeneration,
    store_revision: SecretStoreRevision,
    binding_set_cas: SecretBindingSetCas,
    device_binding_generation: DeviceBindingGeneration,
    capability_revision: CapabilityRevision,
    locator: BackendRecordLocator,
}

struct RegisteredBackendHandleBinding {
    registered: std::sync::Arc<RegisteredSecretBackend>,
    device_instance_id: DeviceInstanceId,
    device_store_instance_id: std::sync::Arc<DeviceSecretStoreInstanceId>,
}

impl RegisteredBackendHandleBinding {
    fn from_handle(handle: &BackendInstanceHandle) -> Self {
        Self {
            registered: std::sync::Arc::clone(&handle.registered),
            device_instance_id: handle.registered.device_instance_id.clone(),
            device_store_instance_id: std::sync::Arc::clone(
                &handle.registered.device_store_instance_id,
            ),
        }
    }

    fn assert_same(
        &self,
        handle: &BackendInstanceHandle,
    ) -> Result<(), SecretInternalError> {
        let same_object = std::sync::Arc::ptr_eq(
            &self.registered,
            &handle.registered,
        );
        let same_instance = self.registered.instance.instance_id()
            == handle.registered.instance.instance_id()
            && self.registered.instance.generation()
                == handle.registered.instance.generation();
        let same_device = self.device_instance_id
            == handle.registered.device_instance_id;
        let same_store = std::sync::Arc::ptr_eq(
            &self.device_store_instance_id,
            &handle.registered.device_store_instance_id,
        );
        (same_object && same_instance && same_device && same_store)
            .then_some(())
            .ok_or_else(SecretInternalError::dependency_changed)
    }
}

impl BackendRecordHandle {
    // Private to crate::secret::backend; callers can receive but not forge it.
    fn from_backend_record(
        backend: &BackendInstanceHandle,
        device_instance_id: DeviceInstanceId,
        device_store_instance_id: std::sync::Arc<DeviceSecretStoreInstanceId>,
        secret_ref: SecretRef,
        purpose: SecretPurpose,
        record_revision: SecretRecordRevision,
        store_revision: SecretStoreRevision,
        binding_set_cas: SecretBindingSetCas,
        device_binding_generation: DeviceBindingGeneration,
        capability_revision: CapabilityRevision,
        locator: BackendRecordLocator,
    ) -> Result<Self, SecretInternalError> {
        if device_instance_id != backend.registered.device_instance_id
            || !std::sync::Arc::ptr_eq(
                &device_store_instance_id,
                &backend.registered.device_store_instance_id,
            )
        {
            return Err(SecretInternalError::dependency_changed());
        }
        Ok(Self {
            registered_backend: RegisteredBackendHandleBinding::from_handle(backend),
            device_instance_id,
            device_store_instance_id,
            secret_ref,
            purpose,
            record_revision,
            instance_id: backend.registered.instance.instance_id().clone(),
            backend_generation: backend.registered.instance.generation(),
            store_revision,
            binding_set_cas,
            device_binding_generation,
            capability_revision,
            locator,
        })
    }

    // Read-only view usable only inside the crate::secret subtree. It permits
    // platform adapters to address the native record without a raw/public
    // locator getter or the ability to forge/change record identity.
    pub(in crate::secret) fn view(&self) -> BackendRecordView<'_> {
        BackendRecordView {
            device_instance_id: &self.device_instance_id,
            device_store_instance_id: &self.device_store_instance_id,
            secret_ref: &self.secret_ref,
            instance_id: &self.instance_id,
            backend_generation: self.backend_generation,
            store_revision: self.store_revision,
            locator: &self.locator,
        }
    }
}

pub(in crate::secret) struct BackendRecordView<'a> {
    device_instance_id: &'a DeviceInstanceId,
    device_store_instance_id: &'a DeviceSecretStoreInstanceId,
    secret_ref: &'a SecretRef,
    instance_id: &'a SecretBackendInstanceId,
    backend_generation: SecretBackendGeneration,
    store_revision: SecretStoreRevision,
    locator: &'a BackendRecordLocator,
}

impl BackendRecordView<'_> {
    pub(in crate::secret) fn secret_ref(&self) -> &SecretRef {
        self.secret_ref
    }

    pub(in crate::secret) fn instance_id(&self) -> &SecretBackendInstanceId {
        self.instance_id
    }

    pub(in crate::secret) fn backend_generation(&self) -> SecretBackendGeneration {
        self.backend_generation
    }

    pub(in crate::secret) fn store_revision(&self) -> SecretStoreRevision {
        self.store_revision
    }

}

struct BackendApplyOperationContext {
    operation_id: SecretOperationId,
    role: SecretApplyRole,
    projection_digest: SecretProjectionDigest,
    owner: SecretOwner,
    owner_binding_revision: SecretOwnerBindingRevision,
    binding_revision: SecretBindingRevision,
    consumer: SecretChangePlanApplyConsumer,
    target_sink: SecretChangePlanApplySink,
    live_sink_id: CodexLiveSecretSinkId,
    record_revision: SecretRecordRevision,
    store_revision: SecretStoreRevision,
    binding_set_cas: SecretBindingSetCas,
    device_binding_generation: DeviceBindingGeneration,
    capability_revision: CapabilityRevision,
    confirmation: PhysicalConfirmation,
    expires_at: UtcTimestamp,
}

struct BackendRuntimeOperationContext {
    operation_id: SecretOperationId,
    consumer: FixedRuntimeConsumer,
    sink: FixedRuntimeSink,
    owner: SecretOwner,
    owner_binding_revision: SecretOwnerBindingRevision,
    binding_revision: SecretBindingRevision,
    record_revision: SecretRecordRevision,
    store_revision: SecretStoreRevision,
    binding_set_cas: SecretBindingSetCas,
    backend_instance_id: SecretBackendInstanceId,
    backend_generation: SecretBackendGeneration,
    device_binding_generation: DeviceBindingGeneration,
    capability_revision: CapabilityRevision,
    confirmation: PhysicalConfirmation,
    expires_at: UtcTimestamp,
}

struct BackendActivationOperationContext {
    operation_id: SecretOperationId,
    candidate_id: SecretCandidateId,
    candidate_revision: SecretCandidateRevision,
    projection_digest: SecretProjectionDigest,
    comparison_policy: LegacyActivationComparisonPolicy,
    slot: ActivationConfirmationSlot,
    record_revision: SecretRecordRevision,
    store_revision: SecretStoreRevision,
    binding_set_cas: SecretBindingSetCas,
    backend_instance_id: SecretBackendInstanceId,
    backend_generation: SecretBackendGeneration,
    device_binding_generation: DeviceBindingGeneration,
    capability_revision: CapabilityRevision,
    confirmation: PhysicalConfirmation,
    expires_at: UtcTimestamp,
}

struct BackendRecoveryOperationContext {
    operation_id: SecretOperationId,
    recovery_id: SecretRecoveryId,
    recovery_kind: SecretRecoveryKind,
    recovery_cas: SecretRecoveryCas,
    slot: RecoveryConfirmationSlot,
    record_revision: SecretRecordRevision,
    store_revision: SecretStoreRevision,
    binding_set_cas: SecretBindingSetCas,
    backend_instance_id: SecretBackendInstanceId,
    backend_generation: SecretBackendGeneration,
    device_binding_generation: DeviceBindingGeneration,
    capability_revision: CapabilityRevision,
    confirmation: PhysicalConfirmation,
    expires_at: UtcTimestamp,
}

struct BackendMigrationOperationContext {
    operation_id: SecretOperationId,
    report_id: SecretMigrationReportId,
    owner: SecretOwner,
    comparison_policy: LegacyActivationComparisonPolicy,
    record_revision: SecretRecordRevision,
    store_revision: SecretStoreRevision,
    binding_set_cas: SecretBindingSetCas,
    backend_instance_id: SecretBackendInstanceId,
    backend_generation: SecretBackendGeneration,
    device_binding_generation: DeviceBindingGeneration,
    capability_revision: CapabilityRevision,
    confirmation: PhysicalConfirmation,
    expires_at: UtcTimestamp,
}

struct BackendStagedImportOperationContext {
    operation_id: SecretOperationId,
    authority: StagedImportAuthorityMatchReceipt,
    candidate_id: SecretCandidateId,
    projection_digest: SecretProjectionDigest,
    comparison_policy: LegacyActivationComparisonPolicy,
    slot: StagedImportConfirmationSlot,
    record_revision: SecretRecordRevision,
    store_revision: SecretStoreRevision,
    binding_set_cas: SecretBindingSetCas,
    backend_instance_id: SecretBackendInstanceId,
    backend_generation: SecretBackendGeneration,
    device_binding_generation: DeviceBindingGeneration,
    capability_revision: CapabilityRevision,
    confirmation: PhysicalConfirmation,
    expires_at: UtcTimestamp,
}

enum SecretNonApplyBackendOperation {
    CaptureVerify,
    Validate,
    CandidateDiscard {
        terminal_state: CandidateTerminalState,
        slot: CandidateDiscardConfirmationSlot,
    },
    DirectDelete,
    Revoke,
}

struct BackendNonApplyOperationContext {
    operation_id: SecretOperationId,
    operation: SecretNonApplyBackendOperation,
    terminal_error_context: SecretTerminalOperationContext,
    owner: SecretOwner,
    record_revision: SecretRecordRevision,
    store_revision: SecretStoreRevision,
    binding_set_cas: SecretBindingSetCas,
    device_binding_generation: DeviceBindingGeneration,
    capability_revision: CapabilityRevision,
    confirmation: PhysicalConfirmation,
    expires_at: UtcTimestamp,
}

enum BackendOperationContext {
    Apply(BackendApplyOperationContext),
    Runtime(BackendRuntimeOperationContext),
    Activation(BackendActivationOperationContext),
    Recovery(BackendRecoveryOperationContext),
    Migration(BackendMigrationOperationContext),
    StagedImport(BackendStagedImportOperationContext),
    NonApply(BackendNonApplyOperationContext),
}

pub(in crate::secret) struct OpaqueApplyAdmissionClaim {
    context: BackendApplyOperationContext,
}
pub(in crate::secret) struct OpaqueOperationReadinessClaim {
    operation_id: SecretOperationId,
    _private: (),
}
pub(in crate::secret) struct OpaqueDurableJournalClaim {
    operation_id: SecretOperationId,
    _private: (),
}
pub(in crate::secret) struct OpaqueRuntimeAuthorityClaim {
    context: BackendRuntimeOperationContext,
}
pub(in crate::secret) struct OpaqueActivationAdmissionClaim {
    context: BackendActivationOperationContext,
}
pub(in crate::secret) struct OpaqueRecoveryReadinessClaim {
    context: BackendRecoveryOperationContext,
}
pub(in crate::secret) struct OpaqueMigrationReadinessClaim {
    context: BackendMigrationOperationContext,
}
pub(in crate::secret) struct OpaqueStagedAuthorityClaim {
    context: BackendStagedImportOperationContext,
}
pub(in crate::secret) struct OpaqueNonApplyReadinessClaim {
    context: BackendNonApplyOperationContext,
}

pub(crate) struct BrokeredBackendOperationContext(BackendOperationContext);

pub(crate) struct BackendOperationBroker {
    capture_intents: std::sync::Arc<dyn SecretCaptureIntentRegistry>,
    capabilities: std::sync::Arc<dyn SecretCapabilityRegistry>,
    pending: std::sync::Arc<dyn PendingSecretConfirmationRegistry>,
}

impl BackendOperationBroker {
    // Scanner-allowlisted only from the production and fixed test dependency
    // factories. No caller supplies a registry trait/object/parameter.
    pub(in crate::secret) fn from_production_store(
        opened_store: &OpenedDeviceLocalSecretStore,
    ) -> Result<std::sync::Arc<Self>, SecretInternalError> {
        let _ = opened_store;
        let (capture_intents, capabilities, pending) =
            todo!("construct fixed production registry implementations internally");
        Ok(std::sync::Arc::new(Self {
            capture_intents,
            capabilities,
            pending,
        }))
    }

    #[cfg(any(test, feature = "test-hooks"))]
    pub(in crate::secret) fn from_fixture_mode(
        mode: crate::test_support::SecretTestFixtureMode,
    ) -> std::sync::Arc<Self> {
        let _ = mode;
        let (capture_intents, capabilities, pending) =
            todo!("construct fixed fixture registry implementations internally");
        std::sync::Arc::new(Self {
            capture_intents,
            capabilities,
            pending,
        })
    }

    pub(in crate::secret) fn mint_capture_intent_from_atomic_snapshot(
        &self,
        registration: SecretCaptureIntentRegistration,
    ) -> Result<ListSecretBackendOptionsResult, SecretInternalError> {
        self.capture_intents.mint_from_atomic_snapshot(registration)
    }

    pub(in crate::secret) fn claim_capture_intent_and_fresh_revalidate(
        &self,
        capture_intent_id: SecretCaptureIntentId,
        backend_instance_id: &SecretBackendInstanceId,
        now: &UtcTimestamp,
        legacy_sources: &mut CodexLegacySourceInventoryBridge<'_>,
        authority: &dyn DeviceLocalSecretAuthority,
        backends: &dyn SecretBackendRegistry,
    ) -> Result<ClaimedSecretCaptureIntent, SecretInternalError> {
        let claim = self.capture_intents.claim_once(
            capture_intent_id,
            backend_instance_id,
            now,
        )?;
        let revalidated = (|| {
            let current_legacy_source_coverage = legacy_sources
                .fresh_capture_coverage(&claim.registration.owner)?;
            claim.registration.legacy.coverage
                .assert_same_complete_coverage_as(
                    &current_legacy_source_coverage,
                )?;
            authority.revalidate_claimed_capture_intent(
                &claim,
                current_legacy_source_coverage,
                backends,
                now,
            )
        })();
        if let Err(error) = revalidated {
            self.capture_intents.terminalize(
                claim,
                PendingConfirmationTermination::Failed,
            )?;
            return Err(error);
        }
        Ok(claim)
    }

    pub(in crate::secret) fn consume_capture_intent(
        &self,
        claim: ClaimedSecretCaptureIntent,
    ) -> Result<(), SecretInternalError> {
        self.capture_intents.consume(claim)
    }

    pub(in crate::secret) fn terminalize_capture_intent(
        &self,
        claim: ClaimedSecretCaptureIntent,
        reason: PendingConfirmationTermination,
    ) -> Result<(), SecretInternalError> {
        self.capture_intents.terminalize(claim, reason)
    }

    pub(in crate::secret) fn register_prepared_capability(
        &self,
        registration: PreparedCapabilityRegistration,
    ) -> Result<PreparedSecretCapability, SecretInternalError> {
        self.capabilities.register_prepared(registration)
    }

    fn claim_prepared_capability(
        &self,
        capability: &PreparedSecretCapability,
        now: &UtcTimestamp,
    ) -> Result<SecretCapabilityClaim, SecretInternalError> {
        self.capabilities.claim_prepared(capability, now)
    }

    pub(in crate::secret) fn mark_capability_consumed(
        &self,
        claim: SecretCapabilityClaim,
    ) -> Result<(), SecretInternalError> {
        self.capabilities.mark_consumed(claim)
    }

    pub(in crate::secret) fn invalidate_capability(
        &self,
        claim: SecretCapabilityClaim,
        code: SecretErrorCode,
    ) {
        self.capabilities.invalidate(claim, code)
    }

    fn terminalize_prepared_capability(
        &self,
        capability: &PreparedSecretCapability,
        code: SecretErrorCode,
    ) -> Result<(), SecretInternalError> {
        self.capabilities.terminalize_prepared(capability, code)
    }

    pub(in crate::secret) fn register_pending_confirmation(
        &self,
        registration: PendingConfirmationRegistration,
    ) -> Result<RegisteredPendingConfirmation, SecretInternalError> {
        self.pending.register_pending(registration)
    }

    pub(in crate::secret) fn claim_pending_confirmation(
        &self,
        id: &PendingSecretConfirmationId,
        now: &UtcTimestamp,
    ) -> Result<(), SecretInternalError> {
        self.pending.claim_confirm(id, now)
    }

    pub(in crate::secret) fn mark_pending_confirmation_confirmed(
        &self,
        id: PendingSecretConfirmationId,
    ) -> Result<(), SecretInternalError> {
        self.pending.mark_confirmed(id)
    }

    pub(in crate::secret) fn terminalize_pending_confirmation(
        &self,
        id: PendingSecretConfirmationId,
        reason: PendingConfirmationTermination,
    ) -> Result<(), SecretInternalError> {
        self.pending.terminate(id, reason)
    }

    pub(in crate::secret) fn for_apply(
        &self,
        admission: OpaqueApplyAdmissionClaim,
        readiness: OpaqueOperationReadinessClaim,
        journal: OpaqueDurableJournalClaim,
    ) -> Result<BrokeredBackendOperationContext, SecretInternalError> {
        todo!("consume and equality-check opaque apply admission + readiness + durable-journal claims")
    }

    pub(in crate::secret) fn for_runtime(
        &self,
        authority: OpaqueRuntimeAuthorityClaim,
        readiness: OpaqueOperationReadinessClaim,
        journal: OpaqueDurableJournalClaim,
    ) -> Result<BrokeredBackendOperationContext, SecretInternalError> {
        todo!("consume exact runtime authority + readiness + durable-journal claims")
    }

    pub(in crate::secret) fn for_activation(
        &self,
        admission: OpaqueActivationAdmissionClaim,
        readiness: OpaqueOperationReadinessClaim,
        journal: OpaqueDurableJournalClaim,
    ) -> Result<BrokeredBackendOperationContext, SecretInternalError> {
        todo!("consume exact activation admission + readiness + durable-journal claims")
    }

    pub(in crate::secret) fn for_recovery(
        &self,
        readiness: OpaqueRecoveryReadinessClaim,
        journal: OpaqueDurableJournalClaim,
    ) -> Result<BrokeredBackendOperationContext, SecretInternalError> {
        todo!("consume exact recovery readiness + durable-journal claims")
    }

    pub(in crate::secret) fn for_migration(
        &self,
        readiness: OpaqueMigrationReadinessClaim,
        journal: OpaqueDurableJournalClaim,
    ) -> Result<BrokeredBackendOperationContext, SecretInternalError> {
        todo!("consume exact migration readiness + durable-journal claims")
    }

    pub(in crate::secret) fn for_staged_import(
        &self,
        authority: OpaqueStagedAuthorityClaim,
        readiness: OpaqueOperationReadinessClaim,
        journal: OpaqueDurableJournalClaim,
    ) -> Result<BrokeredBackendOperationContext, SecretInternalError> {
        todo!("consume the authority-match receipt inside staged authority plus exact readiness and durable-journal claims")
    }

    pub(in crate::secret) fn for_non_apply(
        &self,
        readiness: OpaqueNonApplyReadinessClaim,
        journal: OpaqueDurableJournalClaim,
    ) -> Result<BrokeredBackendOperationContext, SecretInternalError> {
        todo!("consume exact operation readiness + durable-journal claims")
    }
}

// Every context, claim and broker field is private and every type is
// non-Clone/non-serde. The scanner allows context literals only inside the
// operation broker, forbids re-export, From/Default and any literal in service
// or command modules, and rejects a direct BackendOperationContext parameter.
// The broker also derives and seals the exact terminal-error context; for a
// non-apply capture it must preserve new/replace/legacy intent or rotation,
// while validate/candidate-discard-record-delete/candidate-discard-record-
// missing/direct-delete/revoke have their fixed validation/discard/delete
// contexts. Candidate discard adds exactly those two operation-specific slots;
// a backend edge cannot choose one later.
// BackendOperationBroker is also the sole owner/caller of the capture-intent,
// capability and pending-confirmation registries. SecretServiceDeps and
// SecretService each retain exactly the same Arc<BackendOperationBroker>; they
// have no parallel registry Arc. Production and test factories construct one
// broker, then move that exact Arc into deps. list -> broker mint; begin ->
// broker atomic claim plus fresh authority/registered-handle revalidation; and
// every cancellation, expiry or later error -> broker terminalization. No
// claimed row can return to Ready and no private registry id crosses the broker.
// SecretService is the long-lived owner of the sole Arc. Private production/
// test assembly may move that same Arc through non-public SecretServiceDeps,
// but there is no caller/test setter, trait-injection parameter, registry
// parameter, broker extractor or AppStateBuilder override.

pub(crate) enum BackendPrepareResult {
    Ready(BackendAuthorizationHandle),
    ConfirmationRequired {
        requirement: BackendConfirmationRequirement,
        pending: BackendPendingConfirmation,
    },
}

pub(crate) struct BackendConfirmationRequirement {
    pub backend_instance_id: SecretBackendInstanceId,
    pub backend_generation: SecretBackendGeneration,
    pub operation: SecretBackendOperation,
    pub confirmation: PhysicalConfirmation,
    pub device: SecretDeviceDisplay,
    pub timeout_seconds: ConfirmationTimeoutSeconds,
    pub prompt_key: HardwarePromptKey,
    pub expires_at: UtcTimestamp,
}

pub(in crate::secret) struct PlatformOperationRequirement<'a> {
    scope: &'a BackendAuthorizationScope,
    operation: SecretBackendOperation,
    confirmation: PhysicalConfirmation,
}

pub(crate) enum PendingConfirmationTermination {
    UserCancelled,
    Expired,
    Discarded,
    Failed,
}

pub(crate) enum BackendProbeResult {
    Present {
        backend_generation: SecretBackendGeneration,
        device_binding_generation: DeviceBindingGeneration,
    },
    Missing {
        backend_generation: SecretBackendGeneration,
        device_binding_generation: DeviceBindingGeneration,
    },
    Revoked {
        hint: BackendRevocationHint,
        backend_generation: SecretBackendGeneration,
        device_binding_generation: DeviceBindingGeneration,
    },
}

pub(in crate::secret) enum PlatformProbeResult {
    Present {
        backend_generation: SecretBackendGeneration,
        device_binding_generation: DeviceBindingGeneration,
    },
    Missing {
        backend_generation: SecretBackendGeneration,
        device_binding_generation: DeviceBindingGeneration,
    },
    Revoked {
        hint: PlatformBackendRevocationHint,
        backend_generation: SecretBackendGeneration,
        device_binding_generation: DeviceBindingGeneration,
    },
}

pub(in crate::secret) struct BackendVerifyReceipt {
    registered_backend: RegisteredBackendHandleBinding,
    device_store_instance_id: std::sync::Arc<DeviceSecretStoreInstanceId>,
    secret_ref: SecretRef,
    record_revision: SecretRecordRevision,
    store_revision: SecretStoreRevision,
    binding_set_cas: SecretBindingSetCas,
    backend_instance_id: SecretBackendInstanceId,
    receipt_id: BackendVerifyReceiptId,
    backend_generation: SecretBackendGeneration,
    device_binding_generation: DeviceBindingGeneration,
    capability_revision: CapabilityRevision,
}

impl BackendVerifyReceipt {
    pub(in crate::secret) fn receipt_id(&self) -> &BackendVerifyReceiptId {
        &self.receipt_id
    }
    pub(in crate::secret) fn backend_generation(&self) -> SecretBackendGeneration {
        self.backend_generation
    }
    pub(in crate::secret) fn device_binding_generation(&self) -> DeviceBindingGeneration {
        self.device_binding_generation
    }
}

pub(crate) struct BackendDeleteReceipt {
    registered_backend: RegisteredBackendHandleBinding,
    device_store_instance_id: std::sync::Arc<DeviceSecretStoreInstanceId>,
    secret_ref: SecretRef,
    record_revision: SecretRecordRevision,
    store_revision: SecretStoreRevision,
    binding_set_cas: SecretBindingSetCas,
    backend_instance_id: SecretBackendInstanceId,
    backend_generation: SecretBackendGeneration,
    device_binding_generation: DeviceBindingGeneration,
    capability_revision: CapabilityRevision,
    disposition: BackendDeleteDisposition,
    completed_at: UtcTimestamp,
}

impl BackendDeleteReceipt {
    pub(in crate::secret) fn into_durable_outcome(
        self,
    ) -> (BackendDeleteDisposition, UtcTimestamp) {
        (self.disposition, self.completed_at)
    }
}

pub(in crate::secret) struct PlatformDeleteResult {
    disposition: BackendDeleteDisposition,
    completed_at: UtcTimestamp,
    backend_generation: SecretBackendGeneration,
    device_binding_generation: DeviceBindingGeneration,
}

wire_enum!(BackendDeleteDisposition { Deleted, AlreadyMissing });

mod platform_backend_sealed {
    pub(super) trait Sealed {}
    // The seal and PlatformBackendPort impls are written in backend.rs, not in
    // platform modules, for the three exact concrete store types.
    #[cfg(target_os = "macos")]
    impl Sealed for crate::secret::platform::macos::MacOsSecretStore {}
    #[cfg(target_os = "windows")]
    impl Sealed for crate::secret::platform::windows::WindowsSecretStore {}
    #[cfg(any(test, feature = "test-hooks"))]
    impl Sealed for crate::secret::testing::InMemorySecretStore {}
}

// This seam is visible only inside crate::secret. Platform modules never
// implement/re-export the public backend contract and never expose raw bytes.
pub(in crate::secret) trait PlatformBackendPort:
    platform_backend_sealed::Sealed + Send + Sync + 'static
{
    fn revocation_observation_capability(
        &self,
    ) -> BackendRevocationObservationCapability;

    fn capabilities_for_record(
        &self,
        record: BackendRecordView<'_>,
        purpose: SecretPurpose,
    ) -> Result<SecretRecordCapabilities, SecretInternalError>;

    fn capabilities_for_new_record(
        &self,
        owner: &SecretOwner,
        purpose: SecretPurpose,
    ) -> Result<SecretRecordCapabilities, SecretInternalError>;

    fn prepare(
        &self,
        record: BackendRecordView<'_>,
        requirement: PlatformOperationRequirement<'_>,
    ) -> Result<PlatformPrepareResult, SecretInternalError>;

    fn confirm(
        &self,
        pending_id: u128,
    ) -> Result<u128, SecretInternalError>;

    fn cancel(
        &self,
        pending_id: u128,
        reason: PendingConfirmationTermination,
    ) -> Result<(), SecretInternalError>;

    // Raw borrow exists only at this private platform ABI and is invoked only
    // by the backend-owned sealed callback below; it is not a getter.
    fn write_and_readback_bytes(
        &self,
        record: BackendRecordView<'_>,
        authorization_id: u128,
        material: &[u8],
    ) -> Result<PlatformWriteReadbackResult, SecretInternalError>;

    fn read_authorized_material_once(
        &self,
        record: BackendRecordView<'_>,
        authorization_id: u128,
    ) -> Result<PlatformAuthorizedReadOutcome, SecretInternalError>;

    fn probe(
        &self,
        record: BackendRecordView<'_>,
    ) -> Result<PlatformProbeResult, SecretInternalError>;

    // The sole raw source/time observation entry. It is reachable only with
    // an authorization prepared for exact General::Revoke scope.
    fn observe_revocation_once(
        &self,
        record: BackendRecordView<'_>,
        authorization_id: u128,
    ) -> Result<PlatformRevocationObservationResult, SecretInternalError>;

    fn delete_or_revoke(
        &self,
        record: BackendRecordView<'_>,
        authorization_id: u128,
        mode: BackendDeleteMode,
    ) -> Result<PlatformDeleteResult, SecretInternalError>;
}

pub(in crate::secret) enum PlatformPrepareResult {
    Ready { authorization_id: u128 },
    ConfirmationRequired {
        pending_id: u128,
        requirement: PlatformConfirmationRequirement,
    },
}

pub(in crate::secret) enum PlatformAuthorizedReadOutcome {
    Material {
        material: SecretMaterial,
        backend_generation: SecretBackendGeneration,
        device_binding_generation: DeviceBindingGeneration,
    },
    Revoked {
        hint: PlatformBackendRevocationHint,
        backend_generation: SecretBackendGeneration,
        device_binding_generation: DeviceBindingGeneration,
    },
}

pub(in crate::secret) struct PlatformRevocationObservationResult {
    observation: PlatformRevocationObservation,
    backend_generation: SecretBackendGeneration,
    device_binding_generation: DeviceBindingGeneration,
}

pub(in crate::secret) struct PlatformConfirmationRequirement {
    device: SecretDeviceDisplay,
    timeout_seconds: ConfirmationTimeoutSeconds,
    prompt_key: HardwarePromptKey,
}

// Private to crate::secret::backend. The platform implementation returns this
// only to PlatformWriteAndReadbackCallback in the same synchronous call stack.
// Its SecretMaterial is never returned by that callback or stored in a field.
pub(in crate::secret) struct PlatformWriteReadbackResult {
    readback: SecretMaterial,
    verify_receipt_id: BackendVerifyReceiptId,
    backend_generation: SecretBackendGeneration,
    device_binding_generation: DeviceBindingGeneration,
}

pub(crate) struct BackendInstanceHandle {
    registered: std::sync::Arc<RegisteredSecretBackend>,
}

struct RegisteredSecretBackend {
    device_instance_id: DeviceInstanceId,
    device_store_instance_id: std::sync::Arc<DeviceSecretStoreInstanceId>,
    instance: SecretBackendInstanceView,
    platform: std::sync::Arc<dyn PlatformBackendPort>,
}

struct PlatformWriteAndReadbackCallback<'a> {
    platform: &'a dyn PlatformBackendPort,
    record: BackendRecordView<'a>,
    registered_backend: RegisteredBackendHandleBinding,
    device_store_instance_id: std::sync::Arc<DeviceSecretStoreInstanceId>,
    secret_ref: SecretRef,
    record_revision: SecretRecordRevision,
    store_revision: SecretStoreRevision,
    binding_set_cas: SecretBindingSetCas,
    backend_instance_id: SecretBackendInstanceId,
    expected_backend_generation: SecretBackendGeneration,
    expected_device_binding_generation: DeviceBindingGeneration,
    capability_revision: CapabilityRevision,
    authorization_id: u128,
    terminal_error_context: SecretTerminalOperationContext,
}

impl backend_material_callback_sealed::Sealed
    for PlatformWriteAndReadbackCallback<'_>
{}

impl BackendMaterialWriteCallback for PlatformWriteAndReadbackCallback<'_> {
    type Receipt = Result<BackendVerifyReceipt, SecretInternalError>;

    fn write_once(self, material: &[u8]) -> Self::Receipt {
        let result = self.platform.write_and_readback_bytes(
            self.record,
            self.authorization_id,
            material,
        )?;
        // The original material borrow is still alive here. ConstantTimeEq is
        // executed before either material can be dropped; `result.readback`
        // zeroizes on every return path and cannot cross this callback.
        let failure = if !result.readback.ct_eq_slice(material) {
            Some(SecretSourceFreeErrorCode::VerifyFailed)
        } else if result.backend_generation != self.expected_backend_generation
            || result.device_binding_generation
                != self.expected_device_binding_generation
        {
            Some(SecretSourceFreeErrorCode::DependencyChanged)
        } else {
            None
        };
        if let Some(code) = failure {
            return Err(SecretInternalError::terminal_operation_failure(
                code,
                self.terminal_error_context,
            ));
        }
        Ok(BackendVerifyReceipt {
            registered_backend: self.registered_backend,
            device_store_instance_id: self.device_store_instance_id,
            secret_ref: self.secret_ref,
            record_revision: self.record_revision,
            store_revision: self.store_revision,
            binding_set_cas: self.binding_set_cas,
            backend_instance_id: self.backend_instance_id,
            receipt_id: result.verify_receipt_id,
            backend_generation: result.backend_generation,
            device_binding_generation: result.device_binding_generation,
            capability_revision: self.capability_revision,
        })
    }
}

// Concrete callback impls are intentionally absent from backend.rs. Each lane
// owner adds its impl in the adapter module that already owns the concrete
// callback and external receipt. The static scanner permits exactly the §7.1.1
// type/route/receipt triples and rejects any second marker or core-side path to
// `crate::services::configuration_apply` / `crate::commands::import_export`.

struct ScopedAuthorizedBackendRead {
    material: SecretMaterial,
    scope: BackendAuthorizationScope,
}

pub(in crate::secret) enum BackendAuthorizedReadOutcome<T> {
    Ready(T),
    Revoked(BackendRevocationHint),
}

pub(crate) struct AuthorizedApplyRead(ScopedAuthorizedBackendRead);
pub(crate) struct AuthorizedActivationRead(ScopedAuthorizedBackendRead);
pub(crate) struct AuthorizedRecoveryRead(ScopedAuthorizedBackendRead);
pub(crate) struct AuthorizedMigrationRead(ScopedAuthorizedBackendRead);
pub(crate) struct AuthorizedStagedImportRead(ScopedAuthorizedBackendRead);
pub(crate) struct AuthorizedProxyRead(ScopedAuthorizedBackendRead);
pub(crate) struct AuthorizedUsageRead(ScopedAuthorizedBackendRead);
pub(crate) struct AuthorizedCodingPlanRead(ScopedAuthorizedBackendRead);
pub(crate) struct AuthorizedModelFetchRead(ScopedAuthorizedBackendRead);
pub(crate) struct AuthorizedValidationRead(ScopedAuthorizedBackendRead);

pub(crate) enum AuthorizedRuntimeRead {
    Proxy(AuthorizedProxyRead),
    Usage(AuthorizedUsageRead),
    CodingPlan(AuthorizedCodingPlanRead),
    ModelFetch(AuthorizedModelFetchRead),
}

pub(crate) struct CandidateReadVerifiedReceipt {
    _private: (),
}

impl AuthorizedApplyRead {
    pub(crate) fn write_apply_once<C>(
        self,
        callback: C,
    ) -> C::Receipt
    where
        C: ApplyMaterialAdapter,
    {
        self.0.material.write_to_sealed_callback(callback)
    }
}

impl AuthorizedActivationRead {
    pub(crate) fn compare_candidate_equality_once<C>(
        self,
        callback: C,
    ) -> C::Receipt
    where
        C: ActivationEqualityMaterialAdapter,
    {
        todo!("require Activation scope + CandidateEquality before exposure");
        self.0.material.write_to_sealed_callback(callback)
    }

    pub(crate) fn verify_explicit_replacement_once(
        self,
    ) -> Result<CandidateReadVerifiedReceipt, SecretInternalError> {
        todo!("require Activation scope + ExplicitReplacement, then consume/drop material")
    }
}

impl AuthorizedRecoveryRead {
    pub(crate) fn compare_recovery_source_once<C>(
        self,
        callback: C,
    ) -> C::Receipt
    where
        C: RecoveryEqualityMaterialAdapter,
    {
        todo!("require Recovery active-record equality slot and exact recovery kind/CAS");
        self.0.material.write_to_sealed_callback(callback)
    }
}

impl AuthorizedMigrationRead {
    pub(crate) fn compare_inventory_once<C>(
        self,
        callback: C,
    ) -> C::Receipt
    where
        C: MigrationEqualityMaterialAdapter,
    {
        self.0.material.write_to_sealed_callback(callback)
    }
}

impl AuthorizedStagedImportRead {
    pub(crate) fn compare_candidate_equality_once<C>(
        self,
        callback: C,
    ) -> C::Receipt
    where
        C: StagedImportEqualityMaterialAdapter,
    {
        todo!("require StagedImport scope + CandidateEquality before exposure");
        self.0.material.write_to_sealed_callback(callback)
    }

    pub(crate) fn verify_explicit_replacement_once(
        self,
    ) -> Result<CandidateReadVerifiedReceipt, SecretInternalError> {
        todo!("require StagedImport scope + ExplicitReplacement, consume/drop material")
    }
}

impl AuthorizedProxyRead {
    pub(crate) fn prepare_request_once<C>(
        self,
        callback: C,
    ) -> C::Receipt
    where
        C: ProxyMaterialAdapter,
    {
        self.0.material.write_to_sealed_callback(callback)
    }
}

impl AuthorizedUsageRead {
    pub(crate) fn prepare_request_once<C>(
        self,
        callback: C,
    ) -> C::Receipt
    where
        C: UsageMaterialAdapter,
    {
        self.0.material.write_to_sealed_callback(callback)
    }
}

impl AuthorizedCodingPlanRead {
    pub(crate) fn prepare_request_once<C>(
        self,
        callback: C,
    ) -> C::Receipt
    where
        C: CodingPlanMaterialAdapter,
    {
        self.0.material.write_to_sealed_callback(callback)
    }
}

impl AuthorizedModelFetchRead {
    pub(crate) fn prepare_request_once<C>(
        self,
        callback: C,
    ) -> C::Receipt
    where
        C: ModelFetchMaterialAdapter,
    {
        self.0.material.write_to_sealed_callback(callback)
    }
}

impl AuthorizedValidationRead {
    pub(crate) fn validate_present_once(
        self,
    ) -> Result<CandidateReadVerifiedReceipt, SecretInternalError> {
        todo!("require exact General::Validate scope, consume/drop material")
    }
}

wire_enum!(BackendDeleteMode { Delete, Revoke });

pub(crate) struct AuthorizedBackendDelete {
    backend: BackendInstanceHandle,
    record: BackendRecordHandle,
    authorization: ConsumedBackendAuthorization,
    mode: BackendDeleteMode,
}

pub(crate) struct BackendMissingReadbackReceipt {
    registered_backend: RegisteredBackendHandleBinding,
    device_store_instance_id: std::sync::Arc<DeviceSecretStoreInstanceId>,
    secret_ref: SecretRef,
    record_revision: SecretRecordRevision,
    store_revision: SecretStoreRevision,
    binding_set_cas: SecretBindingSetCas,
    backend_instance_id: SecretBackendInstanceId,
    backend_generation: SecretBackendGeneration,
    device_binding_generation: DeviceBindingGeneration,
    capability_revision: CapabilityRevision,
    delete_applied_cas: BackendDeleteAppliedCas,
    checked_at: UtcTimestamp,
}

pub(crate) struct AuthorizedBackendMissingReadback {
    backend: BackendInstanceHandle,
    record: BackendRecordHandle,
    authorization: ConsumedBackendAuthorization,
    expected_delete_applied_cas: BackendDeleteAppliedCas,
}

impl AuthorizedBackendMissingReadback {
    pub(crate) fn readback_missing_once(
        self,
        delete_applied_cas: &BackendDeleteAppliedCas,
        now: UtcTimestamp,
    ) -> Result<BackendMissingReadbackReceipt, SecretInternalError> {
        self.authorization
            .scope
            .assert_registered_handle(&self.backend)?;
        self.backend.assert_record_identity(&self.record)?;
        if delete_applied_cas != &self.expected_delete_applied_cas {
            return Err(SecretInternalError::terminal_operation_failure(
                SecretSourceFreeErrorCode::DependencyChanged,
                self.authorization.scope.into_terminal_error_context(),
            ));
        }
        match self.backend.registered.platform.probe(self.record.view())? {
            PlatformProbeResult::Missing {
                backend_generation,
                device_binding_generation,
            } if backend_generation == self.record.backend_generation
                && device_binding_generation
                    == self.record.device_binding_generation => Ok(BackendMissingReadbackReceipt {
                registered_backend:
                    RegisteredBackendHandleBinding::from_handle(&self.backend),
                device_store_instance_id:
                    self.record.device_store_instance_id.clone(),
                secret_ref: self.record.secret_ref.clone(),
                record_revision: self.record.record_revision,
                store_revision: self.record.store_revision,
                binding_set_cas: self.record.binding_set_cas.clone(),
                backend_instance_id: self.record.instance_id.clone(),
                backend_generation,
                device_binding_generation,
                capability_revision: self.record.capability_revision,
                delete_applied_cas: delete_applied_cas.clone(),
                checked_at: now,
            }),
            PlatformProbeResult::Present { .. }
            | PlatformProbeResult::Revoked { .. } => Err(
                SecretInternalError::terminal_operation_failure(
                    SecretSourceFreeErrorCode::DependencyChanged,
                    self.authorization.scope.into_terminal_error_context(),
                ),
            ),
        }
    }
}

impl AuthorizedBackendDelete {
    pub(crate) fn delete_once(self) -> Result<BackendDeleteReceipt, SecretInternalError> {
        self.authorization
            .scope
            .assert_registered_handle(&self.backend)?;
        self.backend.assert_record_identity(&self.record)?;
        let raw = self.backend.registered.platform.delete_or_revoke(
            self.record.view(),
            self.authorization.authorization_id,
            self.mode,
        )?;
        if raw.backend_generation != self.record.backend_generation
            || raw.device_binding_generation
                != self.record.device_binding_generation
        {
            return Err(SecretInternalError::terminal_operation_failure(
                SecretSourceFreeErrorCode::DependencyChanged,
                self.authorization.scope.into_terminal_error_context(),
            ));
        }
        Ok(BackendDeleteReceipt {
            registered_backend:
                RegisteredBackendHandleBinding::from_handle(&self.backend),
            device_store_instance_id:
                self.record.device_store_instance_id.clone(),
            secret_ref: self.record.secret_ref.clone(),
            record_revision: self.record.record_revision,
            store_revision: self.record.store_revision,
            binding_set_cas: self.record.binding_set_cas.clone(),
            backend_instance_id: self.record.instance_id.clone(),
            backend_generation: raw.backend_generation,
            device_binding_generation: raw.device_binding_generation,
            capability_revision: self.record.capability_revision,
            disposition: raw.disposition,
            completed_at: raw.completed_at,
        })
    }
}

impl BackendInstanceHandle {
    pub(crate) fn instance(&self) -> &SecretBackendInstanceView {
        &self.registered.instance
    }

    fn assert_record_identity(
        &self,
        record: &BackendRecordHandle,
    ) -> Result<(), SecretInternalError> {
        (&record.instance_id == self.registered.instance.instance_id()
            && record.backend_generation
                == self.registered.instance.generation()
            && record.device_instance_id
                == self.registered.device_instance_id
            && std::sync::Arc::ptr_eq(
                &record.device_store_instance_id,
                &self.registered.device_store_instance_id,
            )
            && record.registered_backend.assert_same(self).is_ok())
            .then_some(())
            .ok_or_else(SecretInternalError::dependency_changed)
    }

    // Backend wrapper is the only producer of the validated capability type.
    pub(crate) fn capabilities_for_record(
        &self,
        record: &BackendRecordHandle,
        purpose: SecretPurpose,
    ) -> Result<SecretRecordCapabilities, SecretInternalError> {
        self.assert_record_identity(record)?;
        let capabilities = self
            .registered
            .platform
            .capabilities_for_record(record.view(), purpose)?;
        let (instance_id, generation) = capabilities.backend_identity();
        if instance_id != self.registered.instance.instance_id()
            || generation != self.registered.instance.generation()
        {
            return Err(SecretInternalError::dependency_changed());
        }
        Ok(capabilities)
    }

    pub(crate) fn capabilities_for_new_record(
        &self,
        owner: &SecretOwner,
        purpose: SecretPurpose,
    ) -> Result<SecretRecordCapabilities, SecretInternalError> {
        let capabilities = self
            .registered
            .platform
            .capabilities_for_new_record(owner, purpose)?;
        let (instance_id, generation) = capabilities.backend_identity();
        if instance_id != self.registered.instance.instance_id()
            || generation != self.registered.instance.generation()
        {
            return Err(SecretInternalError::dependency_changed());
        }
        Ok(capabilities)
    }

    pub(crate) fn prepare_brokered_operation(
        &self,
        record: &BackendRecordHandle,
        context: BrokeredBackendOperationContext,
    ) -> Result<BackendPrepareResult, SecretInternalError> {
        self.assert_record_identity(record)?;
        let scope = BackendAuthorizationScope::mint_from_context(self, record, context)?;
        scope.assert_registered_handle(self)?;
        let platform_requirement = scope.platform_requirement()?;
        match self
            .registered
            .platform
            .prepare(record.view(), platform_requirement)?
        {
            PlatformPrepareResult::Ready { authorization_id } => {
                Ok(BackendPrepareResult::Ready(
                    BackendAuthorizationHandle::mint(authorization_id, scope),
                ))
            }
            PlatformPrepareResult::ConfirmationRequired {
                pending_id,
                requirement,
            } => {
                let operation = platform_requirement.operation;
                let confirmation = platform_requirement.confirmation;
                scope.validate_confirmation_requirement(
                    self,
                    operation,
                    confirmation,
                    &requirement,
                )?;
                let public_requirement = BackendConfirmationRequirement {
                    backend_instance_id: scope.0.backend_instance_id.clone(),
                    backend_generation: scope.0.backend_generation,
                    operation,
                    confirmation,
                    device: requirement.device.clone(),
                    timeout_seconds: requirement.timeout_seconds,
                    prompt_key: requirement.prompt_key,
                    expires_at: scope.0.expires_at.clone(),
                };
                let pending_requirement = BackendPendingRequirementIdentity {
                    backend_instance_id: scope.0.backend_instance_id.clone(),
                    backend_generation: scope.0.backend_generation,
                    operation,
                    confirmation,
                    device: requirement.device,
                    timeout_seconds: requirement.timeout_seconds,
                    prompt_key: requirement.prompt_key,
                    expires_at: scope.0.expires_at.clone(),
                };
                Ok(BackendPrepareResult::ConfirmationRequired {
                    requirement: public_requirement,
                    pending: BackendPendingConfirmation::mint(
                        pending_id,
                        scope,
                        pending_requirement,
                    ),
                })
            }
        }
    }

    pub(crate) fn confirm_operation(
        &self,
        pending: BackendPendingConfirmation,
        now: &UtcTimestamp,
    ) -> Result<BackendAuthorizationHandle, SecretInternalError> {
        pending.scope.assert_registered_handle(self)?;
        pending.scope.validate_pending_requirement(
            self,
            &pending.requirement,
            now,
            None,
        )?;
        let authorization_id = self.registered.platform.confirm(pending.pending_id)?;
        Ok(BackendAuthorizationHandle::mint(
            authorization_id,
            pending.scope,
        ))
    }

    pub(crate) fn cancel_operation(
        &self,
        pending: BackendPendingConfirmation,
        reason: PendingConfirmationTermination,
        now: &UtcTimestamp,
    ) -> Result<(), SecretInternalError> {
        pending.scope.assert_registered_handle(self)?;
        pending.scope.validate_pending_requirement(
            self,
            &pending.requirement,
            now,
            Some(&reason),
        )?;
        self.registered.platform.cancel(pending.pending_id, reason)
    }

    // Capture-only exact operation: consumes authorization, writes and reads
    // back inside the backend wrapper, ConstantTimeEq compares, zeroizes both
    // materials and returns only a fixed receipt.
    pub(in crate::secret) fn write_and_verify_once(
        &self,
        record: &BackendRecordHandle,
        material: SecretMaterial,
        authorization: BackendAuthorizationHandle,
        operation_id: &SecretOperationId,
        now: &UtcTimestamp,
    ) -> Result<BackendVerifyReceipt, SecretInternalError> {
        self.assert_record_identity(record)?;
        let consumed = authorization.consume(self, record, operation_id, now)?;
        let authorization_id = consumed.authorization_id;
        let terminal_error_context = consumed.scope.into_terminal_error_context();
        material.write_to_sealed_callback(PlatformWriteAndReadbackCallback {
            platform: self.registered.platform.as_ref(),
            record: record.view(),
            registered_backend: RegisteredBackendHandleBinding::from_handle(self),
            device_store_instance_id: record.device_store_instance_id.clone(),
            secret_ref: record.secret_ref.clone(),
            record_revision: record.record_revision,
            store_revision: record.store_revision,
            binding_set_cas: record.binding_set_cas.clone(),
            backend_instance_id: record.instance_id.clone(),
            expected_backend_generation: record.backend_generation,
            expected_device_binding_generation:
                record.device_binding_generation,
            capability_revision: record.capability_revision,
            authorization_id,
            terminal_error_context,
        })
    }

    fn read_scoped_once(
        &self,
        record: &BackendRecordHandle,
        authorization: BackendAuthorizationHandle,
        operation_id: &SecretOperationId,
        now: &UtcTimestamp,
    ) -> Result<BackendAuthorizedReadOutcome<ScopedAuthorizedBackendRead>, SecretInternalError> {
        self.assert_record_identity(record)?;
        let consumed = authorization.consume(self, record, operation_id, now)?;
        match self.registered.platform.read_authorized_material_once(
            record.view(),
            consumed.authorization_id,
        )? {
            PlatformAuthorizedReadOutcome::Material {
                material,
                backend_generation,
                device_binding_generation,
            } if backend_generation == record.backend_generation
                && device_binding_generation
                    == record.device_binding_generation => {
                Ok(BackendAuthorizedReadOutcome::Ready(
                    ScopedAuthorizedBackendRead {
                        material,
                        scope: consumed.scope,
                    },
                ))
            }
            PlatformAuthorizedReadOutcome::Revoked {
                hint: _,
                backend_generation,
                device_binding_generation,
            } if backend_generation == record.backend_generation
                && device_binding_generation
                    == record.device_binding_generation =>
                Ok(BackendAuthorizedReadOutcome::Revoked(
                    BackendRevocationHint {
                        registered_backend:
                            RegisteredBackendHandleBinding::from_handle(self),
                        device_store_instance_id:
                            record.device_store_instance_id.clone(),
                        _private: (),
                    },
                )),
            PlatformAuthorizedReadOutcome::Material { .. }
            | PlatformAuthorizedReadOutcome::Revoked { .. } => {
                Err(SecretInternalError::terminal_operation_failure(
                    SecretSourceFreeErrorCode::DependencyChanged,
                    consumed.scope.into_terminal_error_context(),
                ))
            }
        }
    }

    pub(in crate::secret) fn authorize_apply_read_once(
        &self,
        record: &BackendRecordHandle,
        authorization: BackendAuthorizationHandle,
        operation_id: &SecretOperationId,
        now: &UtcTimestamp,
    ) -> Result<BackendAuthorizedReadOutcome<AuthorizedApplyRead>, SecretInternalError> {
        authorization.scope.require_route(AuthorizedReadRoute::Apply)?;
        todo!("read_scoped_once then require complete Apply scope")
    }

    pub(in crate::secret) fn authorize_activation_read_once(
        &self,
        record: &BackendRecordHandle,
        authorization: BackendAuthorizationHandle,
        operation_id: &SecretOperationId,
        now: &UtcTimestamp,
    ) -> Result<BackendAuthorizedReadOutcome<AuthorizedActivationRead>, SecretInternalError> {
        authorization.scope.require_route(AuthorizedReadRoute::Activation)?;
        todo!("read_scoped_once then require complete Activation candidate-read scope")
    }

    pub(in crate::secret) fn authorize_recovery_read_once(
        &self,
        record: &BackendRecordHandle,
        authorization: BackendAuthorizationHandle,
        operation_id: &SecretOperationId,
        now: &UtcTimestamp,
    ) -> Result<BackendAuthorizedReadOutcome<AuthorizedRecoveryRead>, SecretInternalError> {
        authorization.scope.require_route(AuthorizedReadRoute::Recovery)?;
        todo!("read_scoped_once then require exact Recovery kind/CAS/read slot")
    }

    pub(in crate::secret) fn authorize_migration_read_once(
        &self,
        record: &BackendRecordHandle,
        authorization: BackendAuthorizationHandle,
        operation_id: &SecretOperationId,
        now: &UtcTimestamp,
    ) -> Result<BackendAuthorizedReadOutcome<AuthorizedMigrationRead>, SecretInternalError> {
        authorization.scope.require_route(AuthorizedReadRoute::Migration)?;
        todo!("read_scoped_once then require complete Migration scope")
    }

    pub(in crate::secret) fn authorize_staged_import_read_once(
        &self,
        record: &BackendRecordHandle,
        authorization: BackendAuthorizationHandle,
        operation_id: &SecretOperationId,
        now: &UtcTimestamp,
    ) -> Result<BackendAuthorizedReadOutcome<AuthorizedStagedImportRead>, SecretInternalError> {
        authorization.scope.require_route(AuthorizedReadRoute::StagedImport)?;
        todo!("read_scoped_once then require complete StagedImport scope")
    }

    pub(in crate::secret) fn authorize_proxy_read_once(
        &self,
        record: &BackendRecordHandle,
        authorization: BackendAuthorizationHandle,
        operation_id: &SecretOperationId,
        now: &UtcTimestamp,
    ) -> Result<BackendAuthorizedReadOutcome<AuthorizedProxyRead>, SecretInternalError> {
        authorization.scope.require_route(AuthorizedReadRoute::Proxy)?;
        todo!("read_scoped_once then require Runtime ProxyRequest/processMemory")
    }

    pub(in crate::secret) fn authorize_usage_read_once(
        &self,
        record: &BackendRecordHandle,
        authorization: BackendAuthorizationHandle,
        operation_id: &SecretOperationId,
        now: &UtcTimestamp,
    ) -> Result<BackendAuthorizedReadOutcome<AuthorizedUsageRead>, SecretInternalError> {
        authorization.scope.require_route(AuthorizedReadRoute::Usage)?;
        todo!("read_scoped_once then require Runtime UsageProbe/processMemory")
    }

    pub(in crate::secret) fn authorize_coding_plan_read_once(
        &self,
        record: &BackendRecordHandle,
        authorization: BackendAuthorizationHandle,
        operation_id: &SecretOperationId,
        now: &UtcTimestamp,
    ) -> Result<BackendAuthorizedReadOutcome<AuthorizedCodingPlanRead>, SecretInternalError> {
        authorization.scope.require_route(AuthorizedReadRoute::CodingPlan)?;
        todo!("read_scoped_once then require CodingPlanUsageProbe/processMemory")
    }

    pub(in crate::secret) fn authorize_model_fetch_read_once(
        &self,
        record: &BackendRecordHandle,
        authorization: BackendAuthorizationHandle,
        operation_id: &SecretOperationId,
        now: &UtcTimestamp,
    ) -> Result<BackendAuthorizedReadOutcome<AuthorizedModelFetchRead>, SecretInternalError> {
        authorization.scope.require_route(AuthorizedReadRoute::ModelFetch)?;
        todo!("read_scoped_once then require Runtime ModelFetch/processMemory")
    }

    pub(in crate::secret) fn authorize_validation_read_once(
        &self,
        record: &BackendRecordHandle,
        authorization: BackendAuthorizationHandle,
        operation_id: &SecretOperationId,
        now: &UtcTimestamp,
    ) -> Result<BackendAuthorizedReadOutcome<AuthorizedValidationRead>, SecretInternalError> {
        authorization.scope.require_route(AuthorizedReadRoute::Validation)?;
        todo!("read_scoped_once then require General Validate scope")
    }

    pub(in crate::secret) fn authorize_delete_once(
        self,
        record: BackendRecordHandle,
        authorization: BackendAuthorizationHandle,
        operation_id: &SecretOperationId,
        now: &UtcTimestamp,
        mode: BackendDeleteMode,
    ) -> Result<AuthorizedBackendDelete, SecretInternalError> {
        self.assert_record_identity(&record)?;
        let consumed = authorization.consume(&self, &record, operation_id, now)?;
        consumed.scope.require_delete_mode(mode)?;
        Ok(AuthorizedBackendDelete {
            backend: self,
            record,
            authorization: consumed,
            mode,
        })
    }

    pub(in crate::secret) fn authorize_missing_readback_once(
        self,
        record: BackendRecordHandle,
        authorization: BackendAuthorizationHandle,
        expected_delete_applied_cas: BackendDeleteAppliedCas,
        operation_id: &SecretOperationId,
        now: &UtcTimestamp,
    ) -> Result<AuthorizedBackendMissingReadback, SecretInternalError> {
        authorization.scope.require_missing_readback()?;
        self.assert_record_identity(&record)?;
        let consumed = authorization.consume(&self, &record, operation_id, now)?;
        Ok(AuthorizedBackendMissingReadback {
            backend: self,
            record,
            authorization: consumed,
            expected_delete_applied_cas,
        })
    }

    pub(crate) fn probe(
        &self,
        record: &BackendRecordHandle,
    ) -> Result<BackendProbeResult, SecretInternalError> {
        self.assert_record_identity(record)?;
        match self.registered.platform.probe(record.view())? {
            PlatformProbeResult::Present {
                backend_generation,
                device_binding_generation,
            } if backend_generation == record.backend_generation
                && device_binding_generation
                    == record.device_binding_generation => Ok(BackendProbeResult::Present {
                backend_generation,
                device_binding_generation,
            }),
            PlatformProbeResult::Missing {
                backend_generation,
                device_binding_generation,
            } if backend_generation == record.backend_generation
                && device_binding_generation
                    == record.device_binding_generation => Ok(BackendProbeResult::Missing {
                backend_generation,
                device_binding_generation,
            }),
            PlatformProbeResult::Revoked {
                hint: _,
                backend_generation,
                device_binding_generation,
            } if backend_generation == record.backend_generation
                && device_binding_generation
                    == record.device_binding_generation => {
                Ok(BackendProbeResult::Revoked {
                    hint: BackendRevocationHint {
                        registered_backend:
                            RegisteredBackendHandleBinding::from_handle(self),
                        device_store_instance_id:
                            record.device_store_instance_id.clone(),
                        _private: (),
                    },
                    backend_generation,
                    device_binding_generation,
                })
            }
            PlatformProbeResult::Present { .. }
            | PlatformProbeResult::Missing { .. }
            | PlatformProbeResult::Revoked { .. } => {
                Err(SecretInternalError::dependency_changed())
            }
        }
    }

    pub(in crate::secret) fn observe_revocation_once(
        &self,
        record: &BackendRecordHandle,
        authorization: BackendAuthorizationHandle,
        operation_id: &SecretOperationId,
        now: &UtcTimestamp,
    ) -> Result<BackendRevocationObservation, SecretInternalError> {
        self.assert_record_identity(record)?;
        authorization.scope.require_revoke_observation()?;
        let consumed = authorization.consume(self, record, operation_id, now)?;
        let capabilities = self.capabilities_for_record(record, record.purpose)?;
        let raw = self.registered.platform.observe_revocation_once(
            record.view(),
            consumed.authorization_id,
        )?;
        BackendRevocationObservation::checked_from_platform(
            self,
            record,
            &capabilities,
            consumed,
            raw,
        )
    }
}

pub(crate) trait SecretBackendRegistry: Send + Sync {
    // Exact tuple lookup only. There is no iterator/fallback API.
    fn get_exact(
        &self,
        instance_id: &SecretBackendInstanceId,
        generation: SecretBackendGeneration,
    ) -> Result<BackendInstanceHandle, SecretInternalError>;

    fn selectable_instances(
        &self,
        owner: &SecretOwner,
        purpose: SecretPurpose,
    ) -> Result<Vec<SecretBackendOption>, SecretInternalError>;
}

// These two types live in crate::secret::migration. The constructor is private
// there; backend.rs alone implements the sealed callback trait for it.
pub(in crate::secret) struct LegacyInventoryCompareCallback {
    expected: SecretMaterial,
}

pub(in crate::secret) struct LegacyInventoryComparisonReceipt {
    equal: bool,
}

impl LegacyInventoryCompareCallback {
    fn new(expected: SecretMaterial) -> Self {
        Self { expected }
    }

    pub(in crate::secret) fn write_material_once(
        self,
        actual: &[u8],
    ) -> LegacyInventoryComparisonReceipt {
        LegacyInventoryComparisonReceipt {
            equal: self.expected.ct_eq_slice(actual),
        }
    }
}
```

#### 7.1.1 Exact material callback/factory allowlist

`crate::secret::material` owns `SecretMaterial`; `crate::secret::backend` owns the callback/route traits, record/auth/pending wrappers, registered platform wrapper, all three exact `PlatformBackendPort` impls and the separately authorized scope-specific read, delete and missing-readback wrappers. `crate::secret::backend` is a crate-private module so an allowlisted sibling adapter can name only its crate-private seal/base/route traits; the core file implements only its platform callback. Each external lane implements that exact trio in its owner adapter module after the lane type exists; backend.rs never names or constructs the external type. `crate::secret` (`src-tauri/src/secret/mod.rs`) re-exports only `SecretBackendRegistry`, `BackendInstanceHandle`, validated public views and the exact operation-specific consuming wrappers as `pub(crate)`; it does not re-export `SecretMaterial`, locator/view, platform port, auth ids or pending ids. Platform files expose only their concrete native store methods to the backend-owned impl; locator construction and raw-string field access remain private in `backend.rs`.

The complete `BackendMaterialWriteCallback` implementer/receipt allowlist is:

| Callback owner/type | Fixed receipt |
| --- | --- |
| `crate::secret::backend::PlatformWriteAndReadbackCallback` | internal `Result<BackendVerifyReceipt, SecretInternalError>`; original material borrow remains live through write/readback/ConstantTimeEq, readback material zeroizes inside callback and never leaves it |
| `crate::services::configuration_apply::provider::SecretApplyWriterInvocation` | `SecretWriterReceiptDto` |
| `crate::services::configuration_apply::provider::ActivationCandidateEqualityCompareCallback` | `Result<ProviderLegacySourceMatchReceipt, SecretInternalError>` |
| `crate::services::configuration_apply::provider::RecoveryCandidateEqualityScrubCallback` | `Result<ProviderScrubReadbackReceipt, SecretInternalError>` |
| `crate::commands::import_export::StagedImportCandidateEqualityCompareCallback` (owner adapter only; never imported by backend.rs) | `Result<StagedImportSourceValidationReceipt, SecretInternalError>` |
| `crate::secret::migration::LegacyInventoryCompareCallback` | private `LegacyInventoryComparisonReceipt { equal: bool }` |
| `crate::proxy::ProxyRequestSecretExecution` (narrow re-export of owner-private `forwarder`) | `Result<PreparedProxyRequest, SecretInternalError>` |
| `crate::services::provider::UsageProbeSecretExecution` (narrow re-export of owner-private `usage`) | `Result<PreparedUsageProbeRequest, SecretInternalError>` |
| `crate::services::coding_plan::CodingPlanSecretExecution` | `Result<PreparedCodingPlanRequest, SecretInternalError>` |
| `crate::services::model_fetch::ModelFetchSecretExecution` | `Result<PreparedModelFetchRequest, SecretInternalError>` |

The backend module itself writes every `PlatformBackendPort` impl and its one internal platform callback impl. Lane-owned adapter modules write only the allowlisted seal/base/route triple beside their existing concrete type; they cannot create a backend scope. The scanner rejects any other implementer, route, receipt, callback constructor, `Fn*`, material getter, any locator/raw-address accessor outside `backend.rs`, auth/pending mint/consume outside backend, or direct platform call outside `RegisteredSecretBackend`. It also rejects `crate::services::configuration_apply`, `crate::commands::import_export` or any other external concrete callback path in backend.rs. Every scope-specific read method (`write_apply_once`, activation/staged `compare_candidate_equality_once` or `verify_explicit_replacement_once`, `compare_recovery_source_once`, `compare_inventory_once`, the four exact `prepare_request_once` implementations, and `validate_present_once`) and `AuthorizedBackendDelete::delete_once` consumes `self`; none implements `Clone/Serialize/Deserialize/Debug`. There is no generic `consume_once<C>` or callback-taking public constructor.

Every backend preparation first resolves one exact registered `BackendInstanceHandle` and creates one sealed `BackendAuthorizationScope`. Durable state uses the strict persisted `DeviceInstanceId`; each `SecretBootstrap::open` additionally mints one non-`Clone`, non-serde `DeviceSecretStoreInstanceId`, shared only as `Arc<DeviceSecretStoreInstanceId>` for that opened authority. `RegisteredBackendHandleBinding` retains the same `Arc<RegisteredSecretBackend>`, durable device id and process-store Arc and requires both `Arc::ptr_eq` checks plus the exact durable device/backend instance/generation before any platform `prepare`, `confirm`, `cancel`, authorized read, write, delete, missing readback or probe call. `JournalBackendIdentity` carries only the durable device id; `BackendRecordHandle` and `BackendAuthorizationScope` carry both identities; pending/auth/read/delete wrappers retain the process identity through the owned scope, and receipts retain it through the registered binding plus their process Arc. Neither identity can substitute for the other. The scope's common row binds operation id, ref, record/store revision, complete binding-set CAS, backend instance/generation, device-binding generation, capability revision, exact confirmation policy and expiry; its closed variant additionally binds either apply role/owner binding/consumer/sink/live-sink id, fixed runtime consumer/owner binding, activation candidate/revision/projection/policy/slot, recovery kind/id/CAS/slot, migration identity, staged-import durable object plus process-live authority/admission/stage/owner/row/projection/slot, or the exact general operation/owner. `BackendAuthorizationHandle`, `BackendPendingConfirmation`, pending-registry rows, `ConsumedBackendAuthorization`, every scope-specific read wrapper and `AuthorizedBackendDelete` retain that same scope by ownership; confirmation may move it from pending to authorized but cannot reconstruct it. The named consuming method compares the entire handle/scope/record/operation/route tuple before material exposure, never only operation/ref. Apply, activation, recovery, migration, staged import, proxy, usage, primary coding-plan, model fetch and validation wrappers are not mutually convertible.

`PlatformBackendPort::prepare` receives a borrow-only `PlatformOperationRequirement` derived from the sealed scope. The platform returns only an internal authorization id or `{pendingId, PlatformConfirmationRequirement}`; it cannot construct a public `HardwareConfirmStep`. Before exposing a step, the wrapper checks the returned device, operation, confirmation policy, timeout and scope expiry against that exact handle/scope and stores the complete requirement identity beside the pending id. `confirm_operation` and `cancel_operation` both consume that pending row only after rechecking the same registered handle, instance/generation, device, operation, policy and deadline. Confirm requires the deadline to be live; `Expired` cancellation requires it to be elapsed, while user/discard cancellation consumes the exact row without turning it into authorization. Platform `confirm` returns only the internal authorization id, which is recombined with the retained scope. A pending/auth id from another registered object, generation, record, operation or confirmation session is never consumable; every mismatch is `SECRET_BACKEND_CHANGED/effect=none` before platform access or material exposure.

`write_and_verify_once` is mandatory and proves a real backend write/readback plus `subtle::ConstantTimeEq` comparison inside `PlatformWriteAndReadbackCallback` while the original borrow is alive. Capture, migration and legacy existing-binding comparison MUST use the registered backend wrapper; `probe` is never equality evidence. A hardware implementation may add device-native verification, but it must still return the authorized readback into this callback for the same constant-time comparison/generation receipt; native verification cannot replace it. A handwritten comparison loop is forbidden. The planned direct dependency is exactly `subtle = 2.6.1`. Post-design-freeze dependency work verifies the resolved lock entry, license/advisory state and Rust 1.85 MSRV; it does not reopen or float the version.

`BackendRecordLocator` is backend-private and non-sensitive: exactly 1–128 ASCII bytes, first alphanumeric, remaining bytes limited to `[A-Za-z0-9._:@=-]`, with the shared credential scanner rejecting a semantic assignment or credential prefix at any token/segment boundary, not only at byte zero. It is derived from backend service/account identity only, never material or a material digest, and has no wire/serde/clone/debug implementation.

### 7.2 Error normalization at the backend edge

`SecretInternalError` has a custom redacted `Debug` and `Display` and is converted exactly once into `SecretErrorView`. Any current/future adapter error carrying bytes is destructured by value, its bytes are zeroized, and only then mapped. No source chain crosses the conversion.

| Backend condition | Stable code | Presence |
| --- | --- | --- |
| exact entry absent | `SECRET_MISSING` | `missing` |
| explicit policy/store lock | `SECRET_LOCKED` with `lockSource=backend` | `unknown` |
| permission/access denied | `SECRET_PERMISSION_DENIED` | `unknown` |
| configured hardware instance not registered | `SECRET_BACKEND_UNAVAILABLE + hardwareUnregistered` | `unknown` |
| registered hardware device disconnected | `SECRET_BACKEND_UNAVAILABLE + hardwareDisconnected` | `unknown` |
| OS protected store unavailable/unsupported | `SECRET_BACKEND_UNAVAILABLE + osStoreUnavailable` | `unknown` |
| registered central backend service unavailable | `SECRET_BACKEND_UNAVAILABLE + centralServiceUnavailable` | `unknown` |
| ambiguous store result | operation-specific `SECRET_READ_FAILED` / `WRITE_FAILED` / `DELETE_FAILED` | `unknown` |
| bad encoding/data containing bytes | zeroize bytes, then operation-specific failure | `unknown` |
| device generation/identity mismatch | `SECRET_DEVICE_MISMATCH` | `unknown` |
| backend reports central/device revocation | `SECRET_REVOKED` with source | last observed presence; availability `revoked` |

### 7.3 Material-free prepared capability

```rust
// Process-local, single-use capture-flow registry. The public id is lookup
// text only; the complete owner/binding/legacy/backend snapshot is private.
struct SecretCaptureLegacyExpectation {
    coverage: LegacySourceCoverageReceipt,
    expected_hidden_binding: Option<OwnerBindingExpectation>,
}

struct SecretCaptureBackendSelectionExpectation {
    instance_id: SecretBackendInstanceId,
    backend_generation: SecretBackendGeneration,
    device_instance_id: DeviceInstanceId,
    device_store_instance_id: std::sync::Arc<DeviceSecretStoreInstanceId>,
    registered_backend: RegisteredBackendHandleBinding,
}

struct SecretCaptureIntentRegistration {
    owner: ExistingSecretOwnerToken,
    purpose: SecretPurpose,
    intent: BeginCaptureIntent,
    owner_binding: OwnerBindingExpectation,
    legacy: SecretCaptureLegacyExpectation,
    selectable_backends: Vec<SecretCaptureBackendSelectionExpectation>,
    expires_at: UtcTimestamp,
}

pub(crate) struct ClaimedSecretCaptureIntent {
    registration: SecretCaptureIntentRegistration,
    selected_backend: SecretCaptureBackendSelectionExpectation,
    claim_id: [u8; 16],
}

pub(crate) trait SecretCaptureIntentRegistry: Send + Sync {
    // Called only through BackendOperationBroker after list_secret_backend_options has
    // atomically read owner identity, purpose, requested intent, current
    // owner-binding, current-scrubbable/adjacent-blocked coverage receipt,
    // hidden binding and the exact
    // registered backend option set. It mints a fresh short-lived id and
    // returns the output-only view plus options derived from that same row.
    fn mint_from_atomic_snapshot(
        &self,
        registration: SecretCaptureIntentRegistration,
    ) -> Result<ListSecretBackendOptionsResult, SecretInternalError>;

    // Atomic Ready -> Claimed and single use. begin_secret_capture supplies
    // only the public id and selected instance id; the registry resolves the
    // exact registered handle, then revalidates the whole snapshot before any
    // native material prompt, candidate mint or backend write.
    fn claim_once(
        &self,
        capture_intent_id: SecretCaptureIntentId,
        backend_instance_id: &SecretBackendInstanceId,
        now: &UtcTimestamp,
    ) -> Result<ClaimedSecretCaptureIntent, SecretInternalError>;

    fn consume(
        &self,
        claim: ClaimedSecretCaptureIntent,
    ) -> Result<(), SecretInternalError>;

    fn terminalize(
        &self,
        claim: ClaimedSecretCaptureIntent,
        reason: PendingConfirmationTermination,
    ) -> Result<(), SecretInternalError>;
}

// The registration/claim/backend-binding fields are private; none implements
// Clone/Serialize/Deserialize/Debug. legacyReconcile requires a non-empty
// current-scrubbable coverage and the exact hidden binding expectation. New and
// replacement intents enforce their matching current binding state. Expiry,
// replay or any snapshot drift is zero-write and cannot reuse a candidate.

struct SecretCapabilityId([u8; 16]);

struct SecretCapabilityClaim {
    capability_id: SecretCapabilityId,
    claim_id: [u8; 16],
}

pub(crate) trait SecretCapabilityRegistry: Send + Sync {
    // Registration is called only by BackendOperationBroker after operation has
    // constructed the private, fully-bound registration row. The registry
    // mints the id and returns the registered consuming capability; callers
    // never submit or reconstruct an id.
    fn register_prepared(
        &self,
        registration: PreparedCapabilityRegistration,
    ) -> Result<PreparedSecretCapability, SecretInternalError>;

    // Atomic prepared -> revalidating. Any other state returns
    // SECRET_CAPABILITY_CONSUMED without exposing registry state.
    fn claim_prepared(
        &self,
        capability: &PreparedSecretCapability,
        now: &UtcTimestamp,
    ) -> Result<SecretCapabilityClaim, SecretInternalError>;

    // Atomic revalidating -> consumed after successful revalidation.
    fn mark_consumed(
        &self,
        claim: SecretCapabilityClaim,
    ) -> Result<(), SecretInternalError>;

    // Any failed revalidation is terminal; it cannot return to prepared.
    fn invalidate(
        &self,
        claim: SecretCapabilityClaim,
        code: SecretErrorCode,
    );

    // Atomic prepared -> discarded; used for unused roles/cancel/expiry.
    fn terminalize_prepared(
        &self,
        capability: &PreparedSecretCapability,
        code: SecretErrorCode,
    ) -> Result<(), SecretInternalError>;
}

pub(in crate::secret) struct SecretReadinessId([u8; 16]);

enum SecretReadinessKindRepr {
    Delete {
        secret_ref: SecretRef,
        record_revision: SecretRecordRevision,
        store_revision: SecretStoreRevision,
        binding_set_cas: SecretBindingSetCas,
        backend_instance_id: SecretBackendInstanceId,
        backend_generation: SecretBackendGeneration,
        device_binding_generation: DeviceBindingGeneration,
        capability_revision: CapabilityRevision,
    },
    Recovery {
        recovery_id: SecretRecoveryId,
        recovery_kind: SecretRecoveryKind,
        recovery_cas: SecretRecoveryCas,
        pending_steps: NonEmptySortedRecoverySteps,
    },
}

pub(in crate::secret) struct SecretReadinessKind(SecretReadinessKindRepr);

impl SecretReadinessKind {
    fn delete(
        secret_ref: SecretRef,
        record_revision: SecretRecordRevision,
        store_revision: SecretStoreRevision,
        binding_set_cas: SecretBindingSetCas,
        backend_instance_id: SecretBackendInstanceId,
        backend_generation: SecretBackendGeneration,
        device_binding_generation: DeviceBindingGeneration,
        capability_revision: CapabilityRevision,
    ) -> Self {
        Self(SecretReadinessKindRepr::Delete {
            secret_ref,
            record_revision,
            store_revision,
            binding_set_cas,
            backend_instance_id,
            backend_generation,
            device_binding_generation,
            capability_revision,
        })
    }

    fn recovery(
        recovery_id: SecretRecoveryId,
        recovery_kind: SecretRecoveryKind,
        recovery_cas: SecretRecoveryCas,
        pending_steps: NonEmptySortedRecoverySteps,
    ) -> Self {
        Self(SecretReadinessKindRepr::Recovery {
            recovery_id,
            recovery_kind,
            recovery_cas,
            pending_steps,
        })
    }
}

pub(in crate::secret) struct SecretReadinessRegistration {
    operation_id: SecretOperationId,
    kind: SecretReadinessKind,
    expires_at: UtcTimestamp,
}

pub(in crate::secret) struct SecretReadinessHandle {
    readiness_id: SecretReadinessId,
    operation_id: SecretOperationId,
}

pub(in crate::secret) struct SecretReadinessClaim {
    readiness_id: SecretReadinessId,
    operation_id: SecretOperationId,
    kind: SecretReadinessKind,
    expires_at: UtcTimestamp,
}

pub(in crate::secret) trait SecretReadinessRegistry: Send + Sync {
    // Process-local only. crate::secret::operation is the sole registration
    // caller and provides a freshly native-minted operation id. The registry
    // returns an opaque handle; only the textual operation id enters a DTO.
    fn mint(
        &self,
        registration: SecretReadinessRegistration,
    ) -> Result<SecretReadinessHandle, SecretInternalError>;

    // Atomic Ready -> Claimed. The lookup id is never authority by itself:
    // the closed expected kind/CAS and expiry are compared before claiming.
    // Missing/claimed/consumed map to SECRET_CONFIRMATION_REPLAYED; an expired
    // ready row maps to SECRET_CONFIRMATION_EXPIRED after becoming terminal.
    // Delete identity drift maps DEPENDENCY_CHANGED and recovery
    // kind/CAS/pending-step drift maps RECOVERY_CHANGED, also terminal. None
    // can re-open authorization.
    fn claim_once(
        &self,
        operation_id: &SecretOperationId,
        expected: &SecretReadinessKind,
        now: &UtcTimestamp,
    ) -> Result<SecretReadinessClaim, SecretInternalError>;

    fn consume(
        &self,
        claim: SecretReadinessClaim,
    ) -> Result<(), SecretInternalError>;

    fn expire(
        &self,
        operation_id: &SecretOperationId,
        now: &UtcTimestamp,
    ) -> Result<(), SecretInternalError>;

    fn terminate(
        &self,
        claim: SecretReadinessClaim,
        reason: PendingConfirmationTermination,
    ) -> Result<(), SecretInternalError>;
}

impl SecretReadinessRegistration {
    // The type is visible to the registry impl, but fields are private and this
    // checked factory is private to crate::secret::operation.
    fn checked(
        operation_id: SecretOperationId,
        kind: SecretReadinessKind,
        expires_at: UtcTimestamp,
    ) -> Result<Self, SecretInternalError> {
        todo!("validate future expiry and exact delete/recovery identity")
    }
}

// SecretReadinessId/Handle/Claim/Registration are non-Serialize,
// non-Deserialize, non-Clone and non-Debug. The registry keeps terminal
// tombstones through the maximum replay window; operationId is lookup only.

pub(crate) struct PreparedSecretCapability {
    capability_id: SecretCapabilityId,
    operation_id: SecretOperationId,
    plan_identity: OwnedAdmittedSecretChangePlanIdentity,
    role: SecretApplyRole,
    owner: SecretOwner,
    secret_ref: SecretRef,
    owner_binding_revision: SecretOwnerBindingRevision,
    binding_revision: SecretBindingRevision,
    record_revision: SecretRecordRevision,
    store_revision: SecretStoreRevision,
    binding_set_cas: SecretBindingSetCas,
    backend_instance_id: SecretBackendInstanceId,
    backend_generation: SecretBackendGeneration,
    device_binding_generation: DeviceBindingGeneration,
    capability_revision: CapabilityRevision,
    consumer: SecretChangePlanApplyConsumer,
    target_sink: SecretChangePlanApplySink,
    live_sink_id: CodexLiveSecretSinkId,
    expires_at: UtcTimestamp,
    authorization: BackendAuthorizationHandle,
}

pub(in crate::secret) struct PreparedCapabilityRegistration {
    // Same complete identity as PreparedSecretCapability, without an id.
    prepared: PreparedSecretCapabilityWithoutId,
}

pub(in crate::secret) struct PreparedSecretCapabilityWithoutId {
    operation_id: SecretOperationId,
    plan_identity: OwnedAdmittedSecretChangePlanIdentity,
    role: SecretApplyRole,
    owner: SecretOwner,
    secret_ref: SecretRef,
    owner_binding_revision: SecretOwnerBindingRevision,
    binding_revision: SecretBindingRevision,
    record_revision: SecretRecordRevision,
    store_revision: SecretStoreRevision,
    binding_set_cas: SecretBindingSetCas,
    backend_instance_id: SecretBackendInstanceId,
    backend_generation: SecretBackendGeneration,
    device_binding_generation: DeviceBindingGeneration,
    capability_revision: CapabilityRevision,
    consumer: SecretChangePlanApplyConsumer,
    target_sink: SecretChangePlanApplySink,
    live_sink_id: CodexLiveSecretSinkId,
    expires_at: UtcTimestamp,
    authorization: BackendAuthorizationHandle,
}

impl PreparedCapabilityRegistration {
    // Private to crate::secret::operation; the complete value is assembled
    // only after projection/admission/backend authorization validation.
    fn from_prepared(prepared: PreparedSecretCapabilityWithoutId) -> Self {
        Self { prepared }
    }
}

// PreparedSecretCapability has private fields and no
// Serialize, Deserialize, Clone or Debug implementation.

pub(crate) struct PreparedSecretCapabilityBundle {
    admitted_plan: AdmittedSecretChangePlan,
    operation_id: SecretOperationId,
    projection: SecretApplyPlanProjection,
    target: PreparedCapabilityRoleSlot,
    rollback: Option<PreparedCapabilityRoleSlot>,
}

enum PreparedCapabilityRoleSlot {
    Prepared(PreparedSecretCapability),
    Consumed,
    Discarded,
}

impl PreparedCapabilityRoleSlot {
    fn prepared_ref(&self) -> Result<&PreparedSecretCapability, SecretInternalError> {
        match self {
            Self::Prepared(capability) => Ok(capability),
            Self::Consumed | Self::Discarded => {
                Err(SecretInternalError::capability_consumed())
            }
        }
    }

    fn take_prepared(
        &mut self,
    ) -> Result<PreparedSecretCapability, SecretInternalError> {
        match std::mem::replace(self, Self::Consumed) {
            Self::Prepared(capability) => Ok(capability),
            Self::Consumed => Err(SecretInternalError::capability_consumed()),
            Self::Discarded => {
                *self = Self::Discarded;
                Err(SecretInternalError::capability_consumed())
            }
        }
    }

    fn discard(
        &mut self,
    ) -> Result<Option<PreparedSecretCapability>, SecretInternalError> {
        match std::mem::replace(self, Self::Discarded) {
            Self::Prepared(capability) => Ok(Some(capability)),
            Self::Consumed | Self::Discarded => Ok(None),
        }
    }
}

pub(crate) struct ClaimedPreparedSecretCapability {
    capability: PreparedSecretCapability,
    claim: SecretCapabilityClaim,
}

impl PreparedSecretCapabilityBundle {
    // The only role extraction path. It changes the role slot before the
    // capability is returned, so a writer error/panic cannot make it prepared
    // again and safe Rust cannot borrow one role while moving the other.
    pub(in crate::secret) fn claim_role_for_revalidation(
        &mut self,
        role: SecretApplyRole,
        broker: &BackendOperationBroker,
        now: &UtcTimestamp,
    ) -> Result<ClaimedPreparedSecretCapability, SecretInternalError> {
        let slot = match role {
            SecretApplyRole::Target => &mut self.target,
            SecretApplyRole::Rollback => self.rollback.as_mut()
                .ok_or_else(SecretInternalError::capability_consumed)?,
        };
        // Atomic registry claim happens while the exact role remains in its
        // Prepared slot. Only a successful claim permits the subsequent move.
        let claim = broker.claim_prepared_capability(slot.prepared_ref()?, now)?;
        let capability = slot.take_prepared()?;
        Ok(ClaimedPreparedSecretCapability { capability, claim })
    }

    pub(in crate::secret) fn terminalize_remaining(
        &mut self,
        broker: &BackendOperationBroker,
        code: SecretErrorCode,
    ) -> Result<(), SecretInternalError> {
        if let Ok(target) = self.target.prepared_ref() {
            broker.terminalize_prepared_capability(target, code)?;
        }
        let _ = self.target.discard()?;
        if let Some(rollback) = self.rollback.as_mut() {
            if let Ok(capability) = rollback.prepared_ref() {
                broker.terminalize_prepared_capability(capability, code)?;
            }
            let _ = rollback.discard()?;
        }
        Ok(())
    }

    pub(in crate::secret) fn projection(&self) -> &SecretApplyPlanProjection {
        &self.projection
    }

    pub(in crate::secret) fn admitted_plan(&self) -> &AdmittedSecretChangePlan {
        &self.admitted_plan
    }

    pub(in crate::secret) fn into_finish_parts(
        self,
    ) -> (AdmittedSecretChangePlan, SecretOperationId) {
        (self.admitted_plan, self.operation_id)
    }
}

// The bundle, role slots and both capabilities are
// non-Serialize/non-Deserialize/non-Clone/non-Debug.

pub(crate) struct PendingSecretConfirmation {
    pending_confirmation_id: PendingSecretConfirmationId,
    operation_id: SecretOperationId,
    plan: AdmittedSecretChangePlan,
    projection: SecretApplyPlanProjection,
    prepared_target: Option<PreparedSecretCapability>,
    prepared_rollback: Option<PreparedSecretCapability>,
    pending_role: SecretApplyRole,
    step: SecretApplyHardwareConfirmStep,
    pending: BackendPendingConfirmation,
}

// PendingSecretConfirmation is also non-Serialize/non-Clone/non-Debug.

pub(crate) struct PendingSecretConfirmationId([u8; 16]);

pub(crate) trait PendingSecretConfirmationRegistry: Send + Sync {
    // BackendOperationBroker is the only caller. The registry mints the id
    // and atomically records the opaque state before a step is returned.
    fn register_pending(
        &self,
        registration: PendingConfirmationRegistration,
    ) -> Result<RegisteredPendingConfirmation, SecretInternalError>;

    // Each operation is atomic and terminal. Missing/terminal ids map to replayed.
    fn claim_confirm(
        &self,
        id: &PendingSecretConfirmationId,
        now: &UtcTimestamp,
    ) -> Result<(), SecretInternalError>;

    fn mark_confirmed(
        &self,
        id: PendingSecretConfirmationId,
    ) -> Result<(), SecretInternalError>;

    fn terminate(
        &self,
        id: PendingSecretConfirmationId,
        reason: PendingConfirmationTermination,
    ) -> Result<(), SecretInternalError>;
}

pub(in crate::secret) struct PendingConfirmationRegistration {
    operation_id: SecretOperationId,
    expires_at: UtcTimestamp,
    backend_pending: BackendPendingConfirmation,
    kind: PendingConfirmationKind,
}

pub(in crate::secret) struct RegisteredPendingConfirmation {
    id: PendingSecretConfirmationId,
    backend_pending: BackendPendingConfirmation,
}

impl RegisteredPendingConfirmation {
    fn into_parts(
        self,
    ) -> (PendingSecretConfirmationId, BackendPendingConfirmation) {
        (self.id, self.backend_pending)
    }
}

pub(in crate::secret) enum PendingConfirmationKind {
    Apply(SecretApplyRole),
    CandidateDiscard(CandidateDiscardConfirmationSlot),
    Activation(ActivationConfirmationSlot),
    Recovery(RecoveryConfirmationSlot),
    StagedImport(StagedImportConfirmationSlot),
}

impl PendingConfirmationRegistration {
    // Private to crate::secret::operation; registry ids are never caller input.
    fn from_backend_pending(
        operation_id: SecretOperationId,
        expires_at: UtcTimestamp,
        backend_pending: BackendPendingConfirmation,
        kind: PendingConfirmationKind,
    ) -> Self {
        Self {
            operation_id,
            expires_at,
            backend_pending,
            kind,
        }
    }
}

pub(crate) enum PrepareForApply {
    Prepared {
        public: SecretApplyPreparationView,
        capabilities: PreparedSecretCapabilityBundle,
    },
    ConfirmationRequired {
        public: SecretApplyPreparationView,
        pending: PendingSecretConfirmation,
    },
}

pub(crate) struct PreparedCandidateDiscardRecordDelete {
    operation_id: SecretOperationId,
    record: BackendRecordHandle,
    expected_candidate_revision: SecretCandidateRevision,
    expected_record_revision: SecretRecordRevision,
    expected_store_revision: SecretStoreRevision,
    backend_generation: SecretBackendGeneration,
    device_binding_generation: DeviceBindingGeneration,
    capability_revision: CapabilityRevision,
    expires_at: UtcTimestamp,
    authorization: BackendAuthorizationHandle,
}

pub(crate) struct PreparedCandidateDiscardRecordMissingReadback {
    operation_id: SecretOperationId,
    record: BackendRecordHandle,
    expected_candidate_revision: SecretCandidateRevision,
    expected_record_revision: SecretRecordRevision,
    expected_store_revision: SecretStoreRevision,
    delete_applied_cas_reservation: BackendDeleteAppliedCasReservation,
    backend_generation: SecretBackendGeneration,
    device_binding_generation: DeviceBindingGeneration,
    capability_revision: CapabilityRevision,
    expires_at: UtcTimestamp,
    authorization: BackendAuthorizationHandle,
}

pub(crate) struct PreparedCandidateDiscardBundle {
    operation_id: SecretOperationId,
    journal: CandidateDeleteJournalRow,
    record_delete: PreparedCandidateDiscardRecordDelete,
    record_missing_readback: PreparedCandidateDiscardRecordMissingReadback,
    expires_at: UtcTimestamp,
}

impl PreparedCandidateDiscardBundle {
    pub(in crate::secret) fn into_parts(
        self,
    ) -> (
        SecretOperationId,
        CandidateDeleteJournalRow,
        PreparedCandidateDiscardRecordDelete,
        PreparedCandidateDiscardRecordMissingReadback,
    ) {
        (
            self.operation_id,
            self.journal,
            self.record_delete,
            self.record_missing_readback,
        )
    }
}

pub(crate) enum PendingCandidateDiscardConfirmation {
    RecordDelete {
        pending_confirmation_id: PendingSecretConfirmationId,
        operation_id: SecretOperationId,
        journal: CandidateDeleteJournalRow,
        step: SecretCandidateDiscardHardwareConfirmStep,
        pending: BackendPendingConfirmation,
    },
    RecordMissingReadback {
        pending_confirmation_id: PendingSecretConfirmationId,
        operation_id: SecretOperationId,
        journal: CandidateDeleteJournalRow,
        prepared_record_delete: PreparedCandidateDiscardRecordDelete,
        step: SecretCandidateDiscardHardwareConfirmStep,
        pending: BackendPendingConfirmation,
    },
}

pub(crate) enum PrepareCandidateDiscard {
    AlreadyTerminal(DiscardSecretCandidateResult),
    Prepared {
        public: SecretCandidateDiscardPreparationView,
        bundle: PreparedCandidateDiscardBundle,
    },
    ConfirmationRequired {
        public: SecretCandidateDiscardPreparationView,
        pending: PendingCandidateDiscardConfirmation,
    },
}

// Both slots are prepared/confirmed before the first backend mutation. The
// missing slot may therefore be pre-confirmed, but its authorization remains
// unusable until its operation-owned reservation is fulfilled by the exact
// durable CandidateDiscardDeleteCheckpoint minted after delete.

pub(crate) struct PreparedActivationCandidateRead {
    operation_id: SecretOperationId,
    candidate_record: BackendRecordHandle,
    expected_candidate_revision: SecretCandidateRevision,
    expected_record_revision: SecretRecordRevision,
    expected_store_revision: SecretStoreRevision,
    backend_generation: SecretBackendGeneration,
    device_binding_generation: DeviceBindingGeneration,
    capability_revision: CapabilityRevision,
    expires_at: UtcTimestamp,
    authorization: BackendAuthorizationHandle,
}

pub(crate) struct PreparedActivationOldRecordDelete {
    operation_id: SecretOperationId,
    old_record: BackendRecordHandle,
    expected_record_revision: SecretRecordRevision,
    expected_store_revision: SecretStoreRevision,
    expected_pre_activation_binding_set: SecretBindingSetCas,
    required_post_activation_binding_state: ActivationOldRecordPostBindingState,
    backend_generation: SecretBackendGeneration,
    device_binding_generation: DeviceBindingGeneration,
    capability_revision: CapabilityRevision,
    expires_at: UtcTimestamp,
    authorization: BackendAuthorizationHandle,
}

pub(crate) enum PreparedActivationOldRecordDeleteSlot {
    NotApplicable,
    Prepared(PreparedActivationOldRecordDelete),
}

pub(crate) struct PreparedActivationOldRecordMissingReadback {
    operation_id: SecretOperationId,
    old_record: BackendRecordHandle,
    expected_record_revision: SecretRecordRevision,
    expected_store_revision: SecretStoreRevision,
    expected_pre_activation_binding_set: SecretBindingSetCas,
    delete_applied_cas_reservation: BackendDeleteAppliedCasReservation,
    backend_generation: SecretBackendGeneration,
    device_binding_generation: DeviceBindingGeneration,
    capability_revision: CapabilityRevision,
    expires_at: UtcTimestamp,
    authorization: BackendAuthorizationHandle,
}

pub(crate) enum PreparedActivationOldRecordMissingReadbackSlot {
    NotApplicable,
    Prepared(PreparedActivationOldRecordMissingReadback),
}

pub(crate) struct PreparedCandidateActivationBundle {
    admitted_plan: AdmittedSecretChangePlan,
    operation_id: SecretOperationId,
    projection: SecretCandidateActivationProjection,
    candidate_read: PreparedActivationCandidateRead,
    old_record_delete: PreparedActivationOldRecordDeleteSlot,
    old_record_missing_readback: PreparedActivationOldRecordMissingReadbackSlot,
}

impl PreparedCandidateActivationBundle {
    pub(in crate::secret) fn into_parts(
        self,
    ) -> (
        AdmittedSecretChangePlan,
        SecretOperationId,
        SecretCandidateActivationProjection,
        PreparedActivationCandidateRead,
        PreparedActivationOldRecordDeleteSlot,
        PreparedActivationOldRecordMissingReadbackSlot,
    ) {
        (
            self.admitted_plan,
            self.operation_id,
            self.projection,
            self.candidate_read,
            self.old_record_delete,
            self.old_record_missing_readback,
        )
    }
}

pub(crate) enum ActivationConfirmationSlot {
    CandidateRead,
    OldRecordDelete,
    OldRecordMissingReadback,
}

pub(crate) struct PendingCandidateActivationConfirmation {
    pending_confirmation_id: PendingSecretConfirmationId,
    operation_id: SecretOperationId,
    plan: AdmittedSecretChangePlan,
    projection: SecretCandidateActivationProjection,
    prepared_candidate_read: Option<PreparedActivationCandidateRead>,
    prepared_old_record_delete: Option<PreparedActivationOldRecordDeleteSlot>,
    prepared_old_record_missing_readback:
        Option<PreparedActivationOldRecordMissingReadbackSlot>,
    pending_slot: ActivationConfirmationSlot,
    step: SecretActivationHardwareConfirmStep,
    pending: BackendPendingConfirmation,
}

pub(crate) enum PrepareCandidateActivation {
    Prepared {
        public: SecretActivationPreparationView,
        bundle: PreparedCandidateActivationBundle,
    },
    ConfirmationRequired {
        public: SecretActivationPreparationView,
        pending: PendingCandidateActivationConfirmation,
    },
}

// These three token types are defined in crate::commands::import_export.
// Only ImportCoordinator::scan_temp_database_structure can mint them, from one open temp
// Database object. The live-object identity is process-opaque, non-Clone and
// has no path/string/serde representation.
struct TempDatabaseProcessNonce([u8; 16]);

struct TempDatabaseAuthorityIdentity {
    durable_object_id: TempDatabaseDurableObjectId,
    process_nonce: TempDatabaseProcessNonce,
}

pub(crate) struct TempDatabaseLiveObjectIdentity {
    authority: std::sync::Arc<TempDatabaseAuthorityIdentity>,
}

// Random opaque id persisted by the import coordinator in its stage registry.
// It is neither a path nor a content/value digest and has no public serde/text
// conversion. A reopened temp DB proves this id from its own durable stage row
// before a fresh live-object identity may be minted.
pub(crate) struct TempDatabaseDurableObjectId([u8; 16]);
pub(crate) struct ImportCutoverReceiptId([u8; 16]);
pub(crate) struct StagedImportAdmissionId([u8; 16]);

pub(crate) struct StagedSecretOwnerToken {
    stage_id: ImportStageId,
    temp_database: TempDatabaseLiveObjectIdentity,
    owner: SecretOwner,
    staged_row_revision: StagedRowRevision,
}

pub(crate) struct StagedSecretOwnerIdentity<'a> {
    stage_id: &'a ImportStageId,
    temp_database: &'a TempDatabaseLiveObjectIdentity,
    owner: &'a SecretOwner,
    staged_row_revision: StagedRowRevision,
}

impl StagedSecretOwnerToken {
    // This implementation resides in crate::commands::import_export; #35 sees
    // only this immutable view and cannot construct/replay a staged token.
    pub(crate) fn identity(&self) -> StagedSecretOwnerIdentity<'_> {
        StagedSecretOwnerIdentity {
            stage_id: &self.stage_id,
            temp_database: &self.temp_database,
            owner: &self.owner,
            staged_row_revision: self.staged_row_revision,
        }
    }
}

impl StagedSecretOwnerIdentity<'_> {
    pub(crate) fn stage_id(&self) -> &ImportStageId { self.stage_id }
    pub(crate) fn temp_database(&self) -> &TempDatabaseLiveObjectIdentity {
        self.temp_database
    }
    pub(crate) fn owner(&self) -> &SecretOwner { self.owner }
    pub(crate) fn staged_row_revision(&self) -> StagedRowRevision {
        self.staged_row_revision
    }
}

// The import owner privately creates this binding from the same process-live
// temp DB authority as StagedSecretOwnerToken. #55 may retain it opaquely but
// cannot construct, inspect bytes or substitute another temp DB.
pub(crate) struct StagedImportAdmissionAuthority {
    temp_database: std::sync::Arc<TempDatabaseAuthorityIdentity>,
    stage_id: ImportStageId,
    owner: SecretOwner,
    staged_row_revision: StagedRowRevision,
}

// Defined and privately minted by crate::change_plan::secret_admission (#55),
// never by the import coordinator or #35.
pub(crate) struct AdmittedStagedSecretImportPlan {
    operation: StagedSecretImportActivationOperation,
    plan_id: ChangePlanId,
    plan_digest: ChangePlanDigest,
    projection_digest: SecretProjectionDigest,
    authority: StagedImportAdmissionAuthority,
    admission_id: StagedImportAdmissionId,
}

pub(crate) struct AdmittedStagedSecretImportIdentity<'a> {
    operation: &'a StagedSecretImportActivationOperation,
    plan_id: &'a ChangePlanId,
    plan_digest: &'a ChangePlanDigest,
    projection_digest: &'a SecretProjectionDigest,
    authority: &'a StagedImportAdmissionAuthority,
    admission_id: &'a StagedImportAdmissionId,
}

// Durable journal identity deliberately omits the process nonce. It records
// the old admission and temp object identity so restart can terminate/reconcile
// it, but a fresh process must mint a new live authority and #55 admission.
pub(in crate::secret) struct OwnedAdmittedStagedSecretImportIdentity {
    operation: StagedSecretImportActivationOperation,
    plan_id: ChangePlanId,
    plan_digest: ChangePlanDigest,
    projection_digest: SecretProjectionDigest,
    durable_object_id: TempDatabaseDurableObjectId,
    stage_id: ImportStageId,
    owner: SecretOwner,
    staged_row_revision: StagedRowRevision,
    admission_id: StagedImportAdmissionId,
}

impl AdmittedStagedSecretImportPlan {
    pub(crate) fn identity(&self) -> AdmittedStagedSecretImportIdentity<'_> {
        AdmittedStagedSecretImportIdentity {
            operation: &self.operation,
            plan_id: &self.plan_id,
            plan_digest: &self.plan_digest,
            projection_digest: &self.projection_digest,
            authority: &self.authority,
            admission_id: &self.admission_id,
        }
    }
}

// This exact scope is built only by crate::commands::import_export after the
// equality port proves staged token + #55 admission share one live object.
pub(crate) struct StagedImportBackendAuthorityScope {
    temp_database: std::sync::Arc<TempDatabaseAuthorityIdentity>,
    stage_id: ImportStageId,
    owner: SecretOwner,
    staged_row_revision: StagedRowRevision,
    admission_id: StagedImportAdmissionId,
}

pub(crate) struct StagedImportAuthorityMatchReceipt {
    backend_scope: StagedImportBackendAuthorityScope,
    _private: (),
}

pub(crate) struct ImportStagedAuthorityComparator {
    _private: (),
}

mod staged_import_authority_equality_sealed {
    pub(super) trait Sealed {}
    impl Sealed for super::ImportStagedAuthorityComparator {}
}

pub(crate) trait StagedImportAuthorityEqualityPort:
    staged_import_authority_equality_sealed::Sealed + Send + Sync
{
    fn assert_same_live_authority(
        &self,
        staged_owner: StagedSecretOwnerIdentity<'_>,
        admission: AdmittedStagedSecretImportIdentity<'_>,
    ) -> Result<StagedImportAuthorityMatchReceipt, SecretInternalError>;
}

pub(crate) struct ReopenedStagedImportAuthority {
    durable_object_id: TempDatabaseDurableObjectId,
    resume_cas: StagedImportResumeCas,
    _private: (),
}

pub(crate) struct PriorStagedAdmissionTerminalReceipt {
    terminal: StagedPriorAdmissionTerminal,
    _private: (),
}

pub(crate) struct FreshStagedLiveAuthority {
    staged_owner: StagedSecretOwnerToken,
    admission_authority: StagedImportAdmissionAuthority,
    _private: (),
}

pub(crate) trait StagedImportResumeAuthorityPort:
    staged_import_authority_equality_sealed::Sealed + Send + Sync
{
    fn reopen_durable_stage(
        &self,
        request: &ResumeStagedImportCutoverRequest,
    ) -> Result<ReopenedStagedImportAuthority, SecretInternalError>;

    fn reconcile_prior_admission(
        &self,
        reopened: &ReopenedStagedImportAuthority,
        prior: &OwnedAdmittedStagedSecretImportIdentity,
    ) -> Result<PriorStagedAdmissionTerminalReceipt, SecretInternalError>;

    fn mint_fresh_live_authority(
        &self,
        reopened: ReopenedStagedImportAuthority,
        prior_terminal: PriorStagedAdmissionTerminalReceipt,
    ) -> Result<FreshStagedLiveAuthority, SecretInternalError>;
}

pub(crate) struct PreparedStagedImportCandidateRead {
    operation_id: SecretOperationId,
    candidate_record: BackendRecordHandle,
    expected_candidate_revision: SecretCandidateRevision,
    expected_record_revision: SecretRecordRevision,
    expected_store_revision: SecretStoreRevision,
    backend_generation: SecretBackendGeneration,
    device_binding_generation: DeviceBindingGeneration,
    capability_revision: CapabilityRevision,
    expires_at: UtcTimestamp,
    authorization: BackendAuthorizationHandle,
}

pub(crate) struct PreparedStagedImportBundle {
    admitted_plan: AdmittedStagedSecretImportPlan,
    staged_owner: StagedSecretOwnerToken,
    projection: StagedSecretImportActivationProjection,
    candidate_read: PreparedStagedImportCandidateRead,
}

impl PreparedStagedImportBundle {
    pub(in crate::secret) fn into_parts(
        self,
    ) -> (
        AdmittedStagedSecretImportPlan,
        StagedSecretOwnerToken,
        StagedSecretImportActivationProjection,
        PreparedStagedImportCandidateRead,
    ) {
        (
            self.admitted_plan,
            self.staged_owner,
            self.projection,
            self.candidate_read,
        )
    }
}

pub(crate) enum StagedImportConfirmationSlot {
    CandidateRead,
}

pub(crate) struct PendingStagedImportConfirmation {
    pending_confirmation_id: PendingSecretConfirmationId,
    operation_id: SecretOperationId,
    admitted_plan: AdmittedStagedSecretImportPlan,
    staged_owner: StagedSecretOwnerToken,
    projection: StagedSecretImportActivationProjection,
    pending_slot: StagedImportConfirmationSlot,
    pending: BackendPendingConfirmation,
}

pub(crate) enum PrepareStagedImport {
    Prepared(PreparedStagedImportBundle),
    ConfirmationRequired(PendingStagedImportConfirmation),
}

pub(crate) struct StagedImportSourceValidationReceipt {
    _private: (),
}

// Exact device-store operation journal. The common envelope owns operationId,
// durable DeviceInstanceId and timestamps; the process-local
// DeviceSecretStoreInstanceId is never encoded. Each payload below contains authority
// fields unique to one of the eight operation kinds and one independent phase
// algebra. No payload/phase uses Option, flatten or a generic checkpoint bag.
#[derive(Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(in crate::secret) enum CandidateTerminalState { Discarded, Expired }

pub(in crate::secret) struct JournalAttempt(u32);
pub(in crate::secret) struct DeviceSecretStoreInstanceId([u8; 16]);
pub(in crate::secret) struct BackendVerifyReceiptId([u8; 16]);
pub(in crate::secret) struct DeleteAdmissionId([u8; 16]);
pub(in crate::secret) struct ProviderDetachTransactionId([u8; 16]);

impl JournalAttempt {
    pub(super) fn checked(value: u32) -> Result<Self, SecretInternalError> {
        (value >= 1)
            .then_some(Self(value))
            .ok_or_else(SecretInternalError::input_invalid)
    }
}

pub(in crate::secret) struct JournalBackendIdentity {
    device_instance_id: DeviceInstanceId,
    secret_ref: SecretRef,
    record_revision: SecretRecordRevision,
    binding_set_cas: SecretBindingSetCas,
    backend_instance_id: SecretBackendInstanceId,
    backend_generation: SecretBackendGeneration,
    device_binding_generation: DeviceBindingGeneration,
    capability_revision: CapabilityRevision,
    confirmation: PhysicalConfirmation,
}

pub(in crate::secret) struct JournalCandidateIdentity {
    candidate_id: SecretCandidateId,
    candidate_revision: SecretCandidateRevision,
    candidate_kind: SecretCandidateKind,
    comparison_policy: LegacyActivationComparisonPolicy,
    comparison_impact: LegacyActivationComparisonImpact,
}

pub(in crate::secret) struct JournalPlanIdentity {
    operation: SecretCandidateActivationOperation,
    admission_id: [u8; 16],
    plan_id: ChangePlanId,
    plan_digest: ChangePlanDigest,
    projection_digest: SecretProjectionDigest,
}

pub(in crate::secret) struct StagedImportJournalPlanIdentity {
    operation: StagedSecretImportActivationOperation,
    admission_id: StagedImportAdmissionId,
    plan_id: ChangePlanId,
    plan_digest: ChangePlanDigest,
    projection_digest: SecretProjectionDigest,
}

pub(in crate::secret) struct DeleteJournalAdmissionIdentity {
    admission_id: DeleteAdmissionId,
    readiness_operation_id: SecretOperationId,
    admitted_at: UtcTimestamp,
}

pub(in crate::secret) struct NonEmptySortedJournalTargetOwners(Vec<SecretOwner>);
pub(in crate::secret) struct NonEmptySortedJournalBindingExpectations(
    Vec<OwnerBindingExpectation>,
);
pub(in crate::secret) struct NonEmptyCurrentLegacySourceExpectations(
    CurrentLegacySourceExpectations,
);
pub(in crate::secret) struct NonEmptySortedOwnerBindingRevisions(
    Vec<SecretOwnerBindingRevision>,
);

pub(in crate::secret) enum CaptureCandidateSourceAuthority {
    None,
    CurrentExplicitReplacement {
        source_expectations: NonEmptyCurrentLegacySourceExpectations,
    },
}

pub(in crate::secret) enum DurableCandidateSourceAuthority {
    NoLegacySources,
    Current { expectations: CurrentLegacySourceExpectations },
}

pub(in crate::secret) struct DurableSecretCandidateRecord {
    candidate: JournalCandidateIdentity,
    state: SecretCandidateState,
    pending_terminal_disposition: Option<CandidateTerminalState>,
    store_revision: SecretStoreRevision,
    target_owners: NonEmptySortedJournalTargetOwners,
    expected_bindings: NonEmptySortedJournalBindingExpectations,
    source_authority: DurableCandidateSourceAuthority,
    backend: JournalBackendIdentity,
    created_at: UtcTimestamp,
    expires_at: UtcTimestamp,
}

impl DurableSecretCandidateRecord {
    pub(super) fn checked(
        record: DurableSecretCandidateRecord,
    ) -> Result<Self, SecretInternalError> {
        todo!("policy/impact/kind/source/owner/backend/store/state/expiry plus pending disposition iff matching nonterminal discard journal invariant")
    }
}

pub(in crate::secret) enum DetachProviderOwnerBindingExpectation {
    Bound {
        secret_ref: SecretRef,
        binding_revision: SecretBindingRevision,
        binding_set_cas: SecretBindingSetCas,
        remaining_owners: SortedSecretOwners,
    },
    Unbound { remaining_owners: [SecretOwner; 0] },
}

wire_enum!(NoBlockingLegacySourcesState { Clear });
wire_enum!(CandidateEqualityOnly { CandidateEquality });
wire_enum!(ExplicitReplacementOnly { ExplicitReplacement });
wire_enum!(JournalCandidateTerminalOutcome { CandidateStaged, Compensated });
wire_enum!(JournalActivationTerminalOutcome { Activated });
wire_enum!(UserDeleteRevocationSource { UserDelete });
wire_enum!(NoBindingsRequired { NoBindings });
wire_enum!(ImportStageKind { SqlImport, BinaryRestore, SyncDownload });

pub(in crate::secret) struct StagedTempDatabaseJournalIdentity {
    stage_id: ImportStageId,
    stage_kind: ImportStageKind,
    durable_object_id: TempDatabaseDurableObjectId,
    process_nonce: TempDatabaseProcessNonce,
    owner: SecretOwner,
    staged_row_revision: StagedRowRevision,
    staged_source_set_cas: StagedSourceSetCas,
}

pub(in crate::secret) struct PromotedLiveOwnerCheckpoint {
    owner: SecretOwner,
    owner_binding_revision: SecretOwnerBindingRevision,
    provider_row_revision: ProviderRowRevision,
}

// This is the complete resume-CAS phase algebra, not a best-effort checkpoint
// bag. Every later arm repeats all receipts from the earlier completed arms so
// omission cannot be confused with an earlier phase. Every
// staged_source_set_cas_after_scrub has source_count=0.
pub(in crate::secret) enum StagedImportResumePhase {
    Intent,
    SourcesScrubbed {
        staged_source_set_cas_after_scrub: StagedSourceSetCas,
    },
    CutoverCommitted {
        staged_source_set_cas_after_scrub: StagedSourceSetCas,
        cutover_receipt_id: ImportCutoverReceiptId,
    },
    LiveOwnerMinted {
        staged_source_set_cas_after_scrub: StagedSourceSetCas,
        cutover_receipt_id: ImportCutoverReceiptId,
        promoted_live_owner: PromotedLiveOwnerCheckpoint,
    },
    LocalBindingFinalized {
        staged_source_set_cas_after_scrub: StagedSourceSetCas,
        cutover_receipt_id: ImportCutoverReceiptId,
        promoted_live_owner: PromotedLiveOwnerCheckpoint,
    },
}

// Credential-free internal preimage for public {revision,digest}. None of
// these fields is part of ResumeStagedImportCutoverRequest or any resume result arm.
pub(in crate::secret) struct StagedImportResumePreimageIdentity {
    operation_id: SecretOperationId,
    expected_store_revision: SecretStoreRevision,
    stage_authority: StagedTempDatabaseJournalIdentity,
    source_expectations: StagedLegacySourceExpectations,
    candidate: JournalCandidateIdentity,
    admission: StagedImportJournalPlanIdentity,
    record: JournalBackendIdentity,
    expected_live_binding: OwnerBindingExpectation,
}

pub(in crate::secret) struct StagedImportResumePreimage {
    identity: StagedImportResumePreimageIdentity,
    phase: StagedImportResumePhase,
}

impl StagedImportResumeCas {
    pub(super) fn checked_from_internal_preimage(
        revision: StagedImportResumeRevision,
        preimage: &StagedImportResumePreimage,
    ) -> Result<Self, SecretInternalError> {
        todo!("hash only the exact canonical rows above, never raw struct/debug serialization: immutable journal operation id plus the closed stage/source/plan/candidate/comparison/record/backend/live-binding/five-arm cumulative phase fields; every after-scrub CAS has count zero; every phase/nonce/admission/source/CAS/receipt/owner change first increments revision, then recomputes digest; output only revision+digest")
    }
}

pub(in crate::secret) enum StagedPriorAdmissionTerminal {
    Consumed,
    Terminated,
    AlreadyTerminal,
}

pub(in crate::secret) struct ActivationCleanupRecoveryLink {
    kind: ActivationCleanupRecoveryKind,
    recovery_id: SecretRecoveryId,
    recovery_cas: SecretRecoveryCas,
}
pub(in crate::secret) struct CaptureCompensationRecoveryLink {
    kind: CaptureCompensationRecoveryKind,
    recovery_id: SecretRecoveryId,
    recovery_cas: SecretRecoveryCas,
}
pub(in crate::secret) struct DeleteFinalizationRecoveryLink {
    kind: DeleteFinalizationRecoveryKind,
    recovery_id: SecretRecoveryId,
    recovery_cas: SecretRecoveryCas,
}
pub(in crate::secret) struct OwnerDetachFinalizationRecoveryLink {
    kind: OwnerDetachFinalizationRecoveryKind,
    recovery_id: SecretRecoveryId,
    recovery_cas: SecretRecoveryCas,
}

wire_enum!(ActivationCleanupRecoveryKind { ActivationCleanup });
wire_enum!(CaptureCompensationRecoveryKind { CaptureCompensation });
wire_enum!(DeleteFinalizationRecoveryKind { DeleteFinalization });
wire_enum!(OwnerDetachFinalizationRecoveryKind { OwnerDetachFinalization });

pub(in crate::secret) enum CaptureCandidateJournalPhase {
    Intent,
    BackendApplied { verify_receipt_id: BackendVerifyReceiptId },
    StateFinalized,
    CompensationIntent,
    RecoveryRequired {
        last_error_code: SecretErrorCode,
        recovery: CaptureCompensationRecoveryLink,
    },
    Terminal { outcome: JournalCandidateTerminalOutcome },
}

pub(in crate::secret) enum MigrateLegacyJournalPhase {
    Intent,
    BackendApplied { verify_receipt_id: BackendVerifyReceiptId },
    StateFinalized,
    CompensationIntent,
    RecoveryRequired {
        last_error_code: SecretErrorCode,
        recovery: CaptureCompensationRecoveryLink,
    },
    Terminal { outcome: JournalCandidateTerminalOutcome },
}

pub(in crate::secret) enum RotateCandidateJournalPhase {
    Intent,
    BackendApplied { verify_receipt_id: BackendVerifyReceiptId },
    StateFinalized,
    CompensationIntent,
    RecoveryRequired {
        last_error_code: SecretErrorCode,
        recovery: CaptureCompensationRecoveryLink,
    },
    Terminal { outcome: JournalCandidateTerminalOutcome },
}

// OldRecordDeleteApplied is the crash boundary between the two independent
// backend authorizations. A successful fresh-missing receipt is the final old
// record step, so the authority persists supersession and Terminal atomically;
// it never exposes an empty-suffix missing-verified journal phase. This
// delete-specific durable projection is exactly None or the complete
// three-field applied record; ordinary activation progress stays in its own
// journal phase and cannot masquerade as an old-record checkpoint.
pub(in crate::secret) enum ActivationOldRecordDurableCheckpoint {
    None,
    OldRecordDeleteApplied {
        delete_disposition: BackendDeleteDisposition,
        backend_completed_at: UtcTimestamp,
        delete_applied_cas: BackendDeleteAppliedCas,
    },
}

pub(in crate::secret) enum ActivateCandidateJournalPhase {
    Intent,
    StateFinalized,
    ProviderFinalized,
    OldRecordDeleteIntent,
    OldRecordDeleteApplied {
        checkpoint: ActivationOldRecordDeleteCheckpoint,
    },
    RecoveryRequired {
        last_error_code: SecretErrorCode,
        checkpoint: ActivationOldRecordDurableCheckpoint,
        recovery: ActivationCleanupRecoveryLink,
    },
    Terminal { outcome: JournalActivationTerminalOutcome },
}

pub(in crate::secret) enum DiscardCandidateRecoveryCheckpoint {
    Intent,
    BackendApplied {
        checkpoint: CandidateDiscardDeleteCheckpoint,
    },
    MissingReadbackVerified {
        checkpoint: CandidateDiscardDeleteCheckpoint,
        missing_checked_at: UtcTimestamp,
    },
}

pub(in crate::secret) enum DiscardCandidateJournalPhase {
    Intent,
    BackendApplied {
        checkpoint: CandidateDiscardDeleteCheckpoint,
    },
    MissingReadbackVerified {
        checkpoint: CandidateDiscardDeleteCheckpoint,
        missing_checked_at: UtcTimestamp,
    },
    RecoveryRequired {
        last_error_code: SecretErrorCode,
        checkpoint: DiscardCandidateRecoveryCheckpoint,
    },
    Terminal { terminal_disposition: CandidateTerminalState },
}

pub(in crate::secret) enum DeleteSecretJournalPhase {
    Intent,
    BackendApplied {
        delete_disposition: BackendDeleteDisposition,
        backend_completed_at: UtcTimestamp,
    },
    MissingReadbackVerified { missing_checked_at: UtcTimestamp },
    StateFinalized {
        revoked_at: UtcTimestamp,
        revocation_source: UserDeleteRevocationSource,
    },
    RecoveryRequired {
        last_error_code: SecretErrorCode,
        recovery: DeleteFinalizationRecoveryLink,
    },
    Terminal {
        revoked_at: UtcTimestamp,
        revocation_source: UserDeleteRevocationSource,
    },
}

pub(in crate::secret) enum DetachProviderOwnerJournalPhase {
    Intent,
    ProviderDetachCommitted { provider_detach_commit_id: ProviderDetachCommitId },
    LocalOwnerCasApplied { provider_detach_commit_id: ProviderDetachCommitId },
    RecoveryRequired {
        last_error_code: SecretErrorCode,
        provider_detach_commit_id: ProviderDetachCommitId,
        recovery: OwnerDetachFinalizationRecoveryLink,
    },
    Terminal { provider_detach_commit_id: ProviderDetachCommitId },
}

pub(in crate::secret) enum StagedImportJournalPhase {
    Intent,
    SourcesScrubbed {
        staged_source_set_cas_after_scrub: StagedSourceSetCas,
    },
    CutoverCommitted {
        staged_source_set_cas_after_scrub: StagedSourceSetCas,
        cutover_receipt_id: ImportCutoverReceiptId,
    },
    LiveOwnerMinted {
        staged_source_set_cas_after_scrub: StagedSourceSetCas,
        cutover_receipt_id: ImportCutoverReceiptId,
        promoted_live_owner: PromotedLiveOwnerCheckpoint,
    },
    LocalBindingFinalized {
        staged_source_set_cas_after_scrub: StagedSourceSetCas,
        cutover_receipt_id: ImportCutoverReceiptId,
        promoted_live_owner: PromotedLiveOwnerCheckpoint,
    },
    RecoveryRequired {
        last_error_code: SecretErrorCode,
        resume_phase: StagedImportResumePhase,
    },
    Terminal {
        staged_source_set_cas_after_scrub: StagedSourceSetCas,
        cutover_receipt_id: ImportCutoverReceiptId,
        promoted_live_owner: PromotedLiveOwnerCheckpoint,
    },
}

pub(in crate::secret) enum JournalOldRecordDeleteExpectation {
    NotApplicable,
    DeleteAfterActivation {
        old_record: JournalBackendIdentity,
        required_binding_state: NoBindingsRequired,
        missing_readback_confirmation: PhysicalConfirmation,
    },
}

pub(in crate::secret) struct CandidateDeleteJournalRow {
    attempt: JournalAttempt,
    expected_store_revision: SecretStoreRevision,
    terminal_disposition: CandidateTerminalState,
    candidate: JournalCandidateIdentity,
    target_owners: NonEmptySortedJournalTargetOwners,
    expected_bindings: NonEmptySortedJournalBindingExpectations,
    record: JournalBackendIdentity,
    delete_slot: CandidateDiscardConfirmationSlot,
    missing_readback_slot: CandidateDiscardConfirmationSlot,
    delete_confirmation: PhysicalConfirmation,
    missing_readback_confirmation: PhysicalConfirmation,
    phase: DiscardCandidateJournalPhase,
}

pub(in crate::secret) struct CandidateDeleteIdentity { _private: () }

impl CandidateDeleteJournalRow {
    fn for_explicit_discard(identity: CandidateDeleteIdentity) -> Self {
        Self::checked(identity, CandidateTerminalState::Discarded)
    }

    fn for_expiry_sweep(identity: CandidateDeleteIdentity) -> Self {
        Self::checked(identity, CandidateTerminalState::Expired)
    }

    fn checked(
        identity: CandidateDeleteIdentity,
        terminal_disposition: CandidateTerminalState,
    ) -> Self {
        todo!("copy exact candidate/owner/store/backend identity plus literal RecordDelete/RecordMissingReadback slots and their independent confirmation policies into discardCandidate intent; strict replay accepts only delete -> durable typed BackendApplied -> fresh Validate missing -> MissingReadbackVerified -> immutable terminal sequence")
    }
}

pub(in crate::secret) enum DurableSecretOperationJournalRepr {
    CaptureCandidate {
        attempt: JournalAttempt,
        expected_store_revision: SecretStoreRevision,
        owner_expectation: OwnerBindingExpectation,
        target_owners: NonEmptySortedJournalTargetOwners,
        expected_bindings: NonEmptySortedJournalBindingExpectations,
        candidate: JournalCandidateIdentity,
        source_authority: CaptureCandidateSourceAuthority,
        backend: JournalBackendIdentity,
        phase: CaptureCandidateJournalPhase,
    },
    MigrateLegacy {
        attempt: JournalAttempt,
        expected_store_revision: SecretStoreRevision,
        migration_report_id: SecretMigrationReportId,
        owner_expectation: OwnerBindingExpectation,
        target_owners: NonEmptySortedJournalTargetOwners,
        expected_bindings: NonEmptySortedJournalBindingExpectations,
        candidate: JournalCandidateIdentity,
        comparison_policy: CandidateEqualityOnly,
        source_expectations: NonEmptyCurrentLegacySourceExpectations,
        backend: JournalBackendIdentity,
        phase: MigrateLegacyJournalPhase,
    },
    RotateCandidate {
        attempt: JournalAttempt,
        expected_store_revision: SecretStoreRevision,
        old_record: JournalBackendIdentity,
        expected_old_binding_set: SecretBindingSetCas,
        affected_owners: NonEmptySortedRecoveryAffectedOwners,
        candidate: JournalCandidateIdentity,
        comparison_policy: ExplicitReplacementOnly,
        new_record: JournalBackendIdentity,
        phase: RotateCandidateJournalPhase,
    },
    ActivateCandidate {
        attempt: JournalAttempt,
        expected_store_revision: SecretStoreRevision,
        admission: JournalPlanIdentity,
        candidate: JournalCandidateIdentity,
        active_record: JournalBackendIdentity,
        affected_owners: NonEmptySortedRecoveryAffectedOwners,
        target_owners: NonEmptySortedJournalTargetOwners,
        expected_bindings: NonEmptySortedJournalBindingExpectations,
        source_expectations: CurrentLegacySourceExpectations,
        old_record_delete: JournalOldRecordDeleteExpectation,
        phase: ActivateCandidateJournalPhase,
    },
    DiscardCandidate { row: CandidateDeleteJournalRow },
    DeleteSecret {
        attempt: JournalAttempt,
        expected_store_revision: SecretStoreRevision,
        delete_admission: DeleteJournalAdmissionIdentity,
        record: JournalBackendIdentity,
        affected_owners: NonEmptySortedRecoveryAffectedOwners,
        expected_owner_binding_revisions: NonEmptySortedOwnerBindingRevisions,
        revocation_source: UserDeleteRevocationSource,
        phase: DeleteSecretJournalPhase,
    },
    DetachProviderOwner {
        attempt: JournalAttempt,
        expected_store_revision: SecretStoreRevision,
        provider_delete_impact_id: ProviderDeleteImpactId,
        provider_row_revision: ProviderRowRevision,
        provider_detach_transaction_id: ProviderDetachTransactionId,
        detached_owner: SecretOwner,
        expected_owner_binding_revision: SecretOwnerBindingRevision,
        legacy_source_coverage_state: NoBlockingLegacySourcesState,
        binding_view: DetachProviderOwnerBindingExpectation,
        phase: DetachProviderOwnerJournalPhase,
    },
    StagedImport {
        attempt: JournalAttempt,
        expected_store_revision: SecretStoreRevision,
        stage_authority: StagedTempDatabaseJournalIdentity,
        admission: StagedImportJournalPlanIdentity,
        candidate: JournalCandidateIdentity,
        source_expectations: StagedLegacySourceExpectations,
        record: JournalBackendIdentity,
        expected_live_binding: OwnerBindingExpectation,
        resume_cas: StagedImportResumeCas,
        phase: StagedImportJournalPhase,
    },
}

pub(in crate::secret) struct DurableSecretOperationJournal {
    schema_version: SchemaVersionV1,
    operation_id: SecretOperationId,
    device_instance_id: DeviceInstanceId,
    created_at: UtcTimestamp,
    updated_at: UtcTimestamp,
    payload: DurableSecretOperationJournalRepr,
}

impl DurableSecretOperationJournal {
    pub(super) fn checked(
        schema_version: SchemaVersionV1,
        operation_id: SecretOperationId,
        device_instance_id: DeviceInstanceId,
        created_at: UtcTimestamp,
        updated_at: UtcTimestamp,
        payload: DurableSecretOperationJournalRepr,
    ) -> Result<Self, SecretInternalError> {
        todo!("validate common envelope plus variant-specific candidate/owner/backend/plan/stage/CAS/phase invariants; discard retains full delete checkpoint and staged resume CAS hashes this operation_id plus the exact cumulative five-arm phase")
    }
}

// Normative codec/replay rules:
// - operationKind is exactly captureCandidate|migrateLegacy|rotateCandidate|
//   activateCandidate|discardCandidate|deleteSecret|detachProviderOwner|
//   stagedImport. There is no ninth generic recovery operation. Each variant
//   uses only its named phase enum and every declared required field is encoded
//   in canonical order; there is no optional property bag.
// - CaptureCandidate preserves the complete sorted target-owner set, a
//   one-to-one sorted OwnerBindingExpectation set, candidate policy+impact,
//   exact source expectations and backend identity. NewBinding may have no
//   sources; ExplicitReplacement requires the exact nonempty admitted source
//   set and replaceExistingCredential impact. Drift is not reconstructible.
// - MigrateLegacy has one owner/binding expectation/current source set;
//   RotateCandidate has the original binding-set CAS, complete affected rows
//   and both backend identities; DeleteSecret aligns
//   every state-finalization revision with the sorted affected owners;
//   ActivateCandidate repeats the opaque #55 admission identity, candidate
//   policy+impact, affected rows, current sources, old-delete expectation and
//   active backend. Its OldRecordDeleteApplied arm embeds the complete
//   ActivationOldRecordDeleteCheckpoint; RecoveryRequired embeds the exact
//   None|OldRecordDeleteApplied durable projection without side state.
// - DiscardCandidate is exactly CandidateDeleteJournalRow. Its generated
//   operation id, candidate/ref/revisions, zero-binding-set CAS and complete
//   backend/device/capability tuple are required. Its terminal state and
//   Intent -> BackendApplied{deleteDisposition,backendCompletedAt,
//   deleteAppliedCas} -> MissingReadbackVerified{the same three fields,
//   missingCheckedAt} -> Terminal sequence cannot be relabelled on replay;
//   there is no candidate-discard StateFinalized arm. RecoveryRequired retains
//   exactly the last complete checkpoint. The RecordMissingReadback authorization is independently
//   prepared/confirmed with Validate policy and remains unusable until the
//   durable BackendApplied CAS reservation is fulfilled.
// - DetachProviderOwner.legacy_source_coverage_state is the required literal
//   Clear; any current-scrubbable or adjacent-blocked occurrence invalidates preview before journal
//   creation. binding is mandatory and only Bound|Unbound. Bound carries
//   ref/per-owner binding revision/binding-set CAS and canonical
//   sorted-unique remaining owners. Unbound carries none and requires the
//   empty array. A current legacy source prevents journal creation entirely.
//   Every arm carries Provider-row + owner-binding revisions and the exact
//   Provider detach transaction id; committed phases add the commit id.
// - StagedImport.admission is the sole staged #55 plan/admission identity;
//   no ordinary activation-plan identity is also present. stage_authority
//   binds stage kind, opaque durable object id, fresh process nonce, owner,
//   staged-row revision and staged-source-set CAS, while the resume preimage
//   additionally binds the fresh operation id. Phase ordering is Intent ->
//   SourcesScrubbed(source-set CAS) -> CutoverCommitted(source-set CAS,
//   receipt) -> LiveOwnerMinted(source-set CAS, receipt, promoted
//   owner/Provider-row/owner-binding checkpoint) -> LocalBindingFinalized(the
//   same three cumulative fields) -> Terminal. RecoveryRequired contains one
//   exact StagedImportResumePhase arm with no optional field bag. A phase,
//   process nonce, admission, receipt or promoted-owner change increments the
//   resume revision before digest recomputation. Terminal currentResumeCas is
//   the exact LocalBindingFinalized projection with all three cumulative
//   fields. ImportCoordinator may reopen only by proving
//   that opaque id from the stage row, then minting a new process live-object
//   identity and rechecking CAS/receipt; no path/snapshot/digest is authority.
// - RecoveryRequired phases contain exactly one typed link to the separately
//   stored activationCleanup|captureCompensation|deleteFinalization|
//   ownerDetachFinalization row. StagedImport instead carries its exact
//   five-arm resume phase. A recovery row is never itself a journal operation
//   variant.
// - Unknown tags/fields, illegal phase payloads, unsorted/duplicate/disjoint
//   sets or candidate/backend/plan/stage/CAS mismatch reject before replay.
//   Only typed structural digests are permitted; material/value digests are
//   forbidden. Startup reconciliation and explicit retry share this decoder.

// Activation bundles/pending state are material-free, non-Serialize,
// non-Deserialize, non-Clone and non-Debug. Preparation authorizes the
// candidate read/compare, planned old delete and fresh old-missing readback as
// three independent slots; confirm may return the next slot. All are ready before #41 may
// acquire its lease. The old-delete authorization is bound to the exact
// expectation already hashed into the activation projection.

wire_enum!(CleanupActiveRecordReadOperation { ResolveForApply });
wire_enum!(CleanupActiveRecordReadScope { CleanupActiveRecordCompare });
wire_enum!(CleanupOldRecordDeleteOperation { Delete });
wire_enum!(CleanupOldRecordDeleteScope { CleanupOldRecordDelete });
wire_enum!(CleanupOldRecordMissingReadbackOperation { Validate });
wire_enum!(CleanupOldRecordMissingReadbackScope {
    CleanupOldRecordMissingReadback
});
wire_enum!(CaptureCompensationDeleteOperation { Delete });
wire_enum!(CaptureCompensationDeleteScope { CaptureCompensationDelete });
wire_enum!(CaptureCompensationMissingReadbackOperation { Validate });
wire_enum!(CaptureCompensationMissingReadbackScope {
    CaptureCompensationMissingReadback
});
wire_enum!(DeleteFinalizationDeleteOperation { Delete });
wire_enum!(DeleteFinalizationDeleteScope { DeleteFinalizationDelete });
wire_enum!(DeleteFinalizationMissingReadbackOperation { Validate });
wire_enum!(DeleteFinalizationMissingReadbackScope {
    DeleteFinalizationMissingReadback
});

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretRecoveryReadHardwareConfirmStep {
    pub schema_version: SchemaVersionV1,
    pub step_id: SecretConfirmationStepId,
    pub operation_id: SecretOperationId,
    pub operation: CleanupActiveRecordReadOperation,
    pub scope: CleanupActiveRecordReadScope,
    pub backend_instance_id: SecretBackendInstanceId,
    pub device: SecretDeviceDisplay,
    pub prompt_key: HardwarePromptKey,
    pub expires_at: UtcTimestamp,
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretRecoveryDeleteHardwareConfirmStep {
    pub schema_version: SchemaVersionV1,
    pub step_id: SecretConfirmationStepId,
    pub operation_id: SecretOperationId,
    pub operation: CleanupOldRecordDeleteOperation,
    pub scope: CleanupOldRecordDeleteScope,
    pub backend_instance_id: SecretBackendInstanceId,
    pub device: SecretDeviceDisplay,
    pub prompt_key: HardwarePromptKey,
    pub expires_at: UtcTimestamp,
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretRecoveryOldRecordMissingHardwareConfirmStep {
    pub schema_version: SchemaVersionV1,
    pub step_id: SecretConfirmationStepId,
    pub operation_id: SecretOperationId,
    pub operation: CleanupOldRecordMissingReadbackOperation,
    pub scope: CleanupOldRecordMissingReadbackScope,
    pub backend_instance_id: SecretBackendInstanceId,
    pub device: SecretDeviceDisplay,
    pub prompt_key: HardwarePromptKey,
    pub expires_at: UtcTimestamp,
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretCaptureCompensationDeleteHardwareConfirmStep {
    pub schema_version: SchemaVersionV1,
    pub step_id: SecretConfirmationStepId,
    pub operation_id: SecretOperationId,
    pub operation: CaptureCompensationDeleteOperation,
    pub scope: CaptureCompensationDeleteScope,
    pub backend_instance_id: SecretBackendInstanceId,
    pub device: SecretDeviceDisplay,
    pub prompt_key: HardwarePromptKey,
    pub expires_at: UtcTimestamp,
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretCaptureCompensationMissingHardwareConfirmStep {
    pub schema_version: SchemaVersionV1,
    pub step_id: SecretConfirmationStepId,
    pub operation_id: SecretOperationId,
    pub operation: CaptureCompensationMissingReadbackOperation,
    pub scope: CaptureCompensationMissingReadbackScope,
    pub backend_instance_id: SecretBackendInstanceId,
    pub device: SecretDeviceDisplay,
    pub prompt_key: HardwarePromptKey,
    pub expires_at: UtcTimestamp,
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "slot", content = "confirmation", rename_all = "camelCase")]
pub enum SecretCaptureCompensationHardwareConfirmStep {
    UncommittedRecordDelete(SecretCaptureCompensationDeleteHardwareConfirmStep),
    UncommittedRecordMissingReadback(SecretCaptureCompensationMissingHardwareConfirmStep),
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretDeleteFinalizationDeleteHardwareConfirmStep {
    pub schema_version: SchemaVersionV1,
    pub step_id: SecretConfirmationStepId,
    pub operation_id: SecretOperationId,
    pub operation: DeleteFinalizationDeleteOperation,
    pub scope: DeleteFinalizationDeleteScope,
    pub backend_instance_id: SecretBackendInstanceId,
    pub device: SecretDeviceDisplay,
    pub prompt_key: HardwarePromptKey,
    pub expires_at: UtcTimestamp,
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretDeleteFinalizationMissingHardwareConfirmStep {
    pub schema_version: SchemaVersionV1,
    pub step_id: SecretConfirmationStepId,
    pub operation_id: SecretOperationId,
    pub operation: DeleteFinalizationMissingReadbackOperation,
    pub scope: DeleteFinalizationMissingReadbackScope,
    pub backend_instance_id: SecretBackendInstanceId,
    pub device: SecretDeviceDisplay,
    pub prompt_key: HardwarePromptKey,
    pub expires_at: UtcTimestamp,
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "slot", content = "confirmation", rename_all = "camelCase")]
pub enum SecretDeleteFinalizationHardwareConfirmStep {
    AdmittedRecordDelete(SecretDeleteFinalizationDeleteHardwareConfirmStep),
    AdmittedRecordMissingReadback(
        SecretDeleteFinalizationMissingHardwareConfirmStep,
    ),
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum SecretActivationCleanupHardwareConfirmStep {
    ActiveRecordRead(SecretRecoveryReadHardwareConfirmStep),
    OldRecordDelete(SecretRecoveryDeleteHardwareConfirmStep),
    OldRecordMissingReadback(SecretRecoveryOldRecordMissingHardwareConfirmStep),
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum SecretRecoveryHardwareConfirmStep {
    ActivationCleanup(SecretActivationCleanupHardwareConfirmStep),
    CaptureCompensation(SecretCaptureCompensationHardwareConfirmStep),
    DeleteFinalization(SecretDeleteFinalizationHardwareConfirmStep),
}

pub(crate) struct PreparedCleanupActiveRecordRead {
    operation_id: SecretOperationId,
    active_record: BackendRecordHandle,
    expected_record_revision: SecretRecordRevision,
    expected_store_revision: SecretStoreRevision,
    expected_binding_set: SecretBindingSetCas,
    backend_generation: SecretBackendGeneration,
    device_binding_generation: DeviceBindingGeneration,
    capability_revision: CapabilityRevision,
    expires_at: UtcTimestamp,
    authorization: BackendAuthorizationHandle,
}

pub(crate) enum PreparedCleanupActiveRecordReadSlot {
    NotApplicable,
    Prepared(PreparedCleanupActiveRecordRead),
}

pub(crate) struct PreparedCleanupOldRecordDelete {
    operation_id: SecretOperationId,
    old_record: BackendRecordHandle,
    expected_record_revision: SecretRecordRevision,
    expected_store_revision: SecretStoreRevision,
    backend_generation: SecretBackendGeneration,
    device_binding_generation: DeviceBindingGeneration,
    capability_revision: CapabilityRevision,
    expires_at: UtcTimestamp,
    authorization: BackendAuthorizationHandle,
}

pub(crate) enum PreparedCleanupOldRecordDeleteSlot {
    NotApplicable,
    Prepared(PreparedCleanupOldRecordDelete),
}

pub(crate) struct PreparedCleanupOldRecordMissingReadback {
    operation_id: SecretOperationId,
    old_record: BackendRecordHandle,
    expected_record_revision: SecretRecordRevision,
    expected_store_revision: SecretStoreRevision,
    delete_applied_cas_reservation: BackendDeleteAppliedCasReservation,
    backend_generation: SecretBackendGeneration,
    device_binding_generation: DeviceBindingGeneration,
    capability_revision: CapabilityRevision,
    expires_at: UtcTimestamp,
    authorization: BackendAuthorizationHandle,
}

pub(crate) enum PreparedCleanupOldRecordMissingReadbackSlot {
    NotApplicable,
    Prepared(PreparedCleanupOldRecordMissingReadback),
}

pub(crate) struct PreparedRecoveryUncommittedRecordDelete {
    operation_id: SecretOperationId,
    record: BackendRecordHandle,
    expected_record_revision: SecretRecordRevision,
    expected_store_revision: SecretStoreRevision,
    backend_generation: SecretBackendGeneration,
    device_binding_generation: DeviceBindingGeneration,
    capability_revision: CapabilityRevision,
    expires_at: UtcTimestamp,
    authorization: BackendAuthorizationHandle,
}

pub(crate) enum PreparedRecoveryUncommittedRecordDeleteSlot {
    NotApplicable,
    Prepared(PreparedRecoveryUncommittedRecordDelete),
}

pub(crate) struct PreparedRecoveryUncommittedRecordMissingReadback {
    operation_id: SecretOperationId,
    record: BackendRecordHandle,
    expected_record_revision: SecretRecordRevision,
    expected_store_revision: SecretStoreRevision,
    delete_applied_cas_reservation: BackendDeleteAppliedCasReservation,
    backend_generation: SecretBackendGeneration,
    device_binding_generation: DeviceBindingGeneration,
    capability_revision: CapabilityRevision,
    expires_at: UtcTimestamp,
    authorization: BackendAuthorizationHandle,
}

pub(crate) enum PreparedRecoveryUncommittedRecordMissingReadbackSlot {
    NotApplicable,
    Prepared(PreparedRecoveryUncommittedRecordMissingReadback),
}

pub(crate) struct PreparedRecoveryAdmittedRecordDelete {
    operation_id: SecretOperationId,
    record: BackendRecordHandle,
    expected_record_revision: SecretRecordRevision,
    expected_store_revision: SecretStoreRevision,
    backend_generation: SecretBackendGeneration,
    device_binding_generation: DeviceBindingGeneration,
    capability_revision: CapabilityRevision,
    expires_at: UtcTimestamp,
    authorization: BackendAuthorizationHandle,
}

pub(crate) enum PreparedRecoveryAdmittedRecordDeleteSlot {
    NotApplicable,
    Prepared(PreparedRecoveryAdmittedRecordDelete),
}

pub(crate) struct PreparedRecoveryAdmittedRecordMissingReadback {
    operation_id: SecretOperationId,
    record: BackendRecordHandle,
    expected_record_revision: SecretRecordRevision,
    expected_store_revision: SecretStoreRevision,
    delete_applied_cas_reservation: BackendDeleteAppliedCasReservation,
    backend_generation: SecretBackendGeneration,
    device_binding_generation: DeviceBindingGeneration,
    capability_revision: CapabilityRevision,
    expires_at: UtcTimestamp,
    authorization: BackendAuthorizationHandle,
}

pub(crate) enum PreparedRecoveryAdmittedRecordMissingReadbackSlot {
    NotApplicable,
    Prepared(PreparedRecoveryAdmittedRecordMissingReadback),
}

enum PreparedSecretRecoveryBundleRepr {
    ActivationCleanup {
        operation_id: SecretOperationId,
        recovery_id: SecretRecoveryId,
        expected_recovery_cas: SecretRecoveryCas,
        snapshot: SecretRecoveryAuthoritySnapshot,
        active_record_read: PreparedCleanupActiveRecordReadSlot,
        old_record_delete: PreparedCleanupOldRecordDeleteSlot,
        old_record_missing_readback: PreparedCleanupOldRecordMissingReadbackSlot,
    },
    CaptureCompensation {
        operation_id: SecretOperationId,
        recovery_id: SecretRecoveryId,
        expected_recovery_cas: SecretRecoveryCas,
        snapshot: SecretRecoveryAuthoritySnapshot,
        uncommitted_record_delete: PreparedRecoveryUncommittedRecordDeleteSlot,
        uncommitted_record_missing_readback:
            PreparedRecoveryUncommittedRecordMissingReadbackSlot,
    },
    DeleteFinalization {
        operation_id: SecretOperationId,
        recovery_id: SecretRecoveryId,
        expected_recovery_cas: SecretRecoveryCas,
        snapshot: SecretRecoveryAuthoritySnapshot,
        admitted_record_delete: PreparedRecoveryAdmittedRecordDeleteSlot,
        admitted_record_missing_readback:
            PreparedRecoveryAdmittedRecordMissingReadbackSlot,
    },
    OwnerDetachFinalization {
        operation_id: SecretOperationId,
        recovery_id: SecretRecoveryId,
        expected_recovery_cas: SecretRecoveryCas,
        snapshot: SecretRecoveryAuthoritySnapshot,
    },
}

pub(crate) struct PreparedSecretRecoveryBundle(
    PreparedSecretRecoveryBundleRepr,
);

pub(in crate::secret) enum PreparedSecretRecoveryParts {
    ActivationCleanup(
        SecretOperationId,
        SecretRecoveryId,
        SecretRecoveryCas,
        SecretRecoveryAuthoritySnapshot,
        PreparedCleanupActiveRecordReadSlot,
        PreparedCleanupOldRecordDeleteSlot,
        PreparedCleanupOldRecordMissingReadbackSlot,
    ),
    CaptureCompensation(
        SecretOperationId,
        SecretRecoveryId,
        SecretRecoveryCas,
        SecretRecoveryAuthoritySnapshot,
        PreparedRecoveryUncommittedRecordDeleteSlot,
        PreparedRecoveryUncommittedRecordMissingReadbackSlot,
    ),
    DeleteFinalization(
        SecretOperationId,
        SecretRecoveryId,
        SecretRecoveryCas,
        SecretRecoveryAuthoritySnapshot,
        PreparedRecoveryAdmittedRecordDeleteSlot,
        PreparedRecoveryAdmittedRecordMissingReadbackSlot,
    ),
    OwnerDetachFinalization(
        SecretOperationId,
        SecretRecoveryId,
        SecretRecoveryCas,
        SecretRecoveryAuthoritySnapshot,
    ),
}

impl PreparedSecretRecoveryBundle {
    fn checked(
        repr: PreparedSecretRecoveryBundleRepr,
    ) -> Result<Self, SecretInternalError> {
        todo!("validate recovery kind/CAS and phase-derived independent slots: activation read/delete/old-missing, capture delete/uncommitted-missing, delete admitted-delete/admitted-missing, detach none")
    }

    pub(in crate::secret) fn recovery_kind(&self) -> SecretRecoveryKind {
        match &self.0 {
            PreparedSecretRecoveryBundleRepr::ActivationCleanup { .. } => {
                SecretRecoveryKind::ActivationCleanup
            }
            PreparedSecretRecoveryBundleRepr::CaptureCompensation { .. } => {
                SecretRecoveryKind::CaptureCompensation
            }
            PreparedSecretRecoveryBundleRepr::DeleteFinalization { .. } => {
                SecretRecoveryKind::DeleteFinalization
            }
            PreparedSecretRecoveryBundleRepr::OwnerDetachFinalization { .. } => {
                SecretRecoveryKind::OwnerDetachFinalization
            }
        }
    }

    pub(in crate::secret) fn into_parts(self) -> PreparedSecretRecoveryParts {
        match self.0 {
            PreparedSecretRecoveryBundleRepr::ActivationCleanup {
                operation_id,
                recovery_id,
                expected_recovery_cas,
                snapshot,
                active_record_read,
                old_record_delete,
                old_record_missing_readback,
            } => PreparedSecretRecoveryParts::ActivationCleanup(
                operation_id,
                recovery_id,
                expected_recovery_cas,
                snapshot,
                active_record_read,
                old_record_delete,
                old_record_missing_readback,
            ),
            PreparedSecretRecoveryBundleRepr::CaptureCompensation {
                operation_id,
                recovery_id,
                expected_recovery_cas,
                snapshot,
                uncommitted_record_delete,
                uncommitted_record_missing_readback,
            } => PreparedSecretRecoveryParts::CaptureCompensation(
                operation_id,
                recovery_id,
                expected_recovery_cas,
                snapshot,
                uncommitted_record_delete,
                uncommitted_record_missing_readback,
            ),
            PreparedSecretRecoveryBundleRepr::DeleteFinalization {
                operation_id,
                recovery_id,
                expected_recovery_cas,
                snapshot,
                admitted_record_delete,
                admitted_record_missing_readback,
            } => PreparedSecretRecoveryParts::DeleteFinalization(
                operation_id,
                recovery_id,
                expected_recovery_cas,
                snapshot,
                admitted_record_delete,
                admitted_record_missing_readback,
            ),
            PreparedSecretRecoveryBundleRepr::OwnerDetachFinalization {
                operation_id,
                recovery_id,
                expected_recovery_cas,
                snapshot,
            } => PreparedSecretRecoveryParts::OwnerDetachFinalization(
                operation_id,
                recovery_id,
                expected_recovery_cas,
                snapshot,
            ),
        }
    }
}

pub(crate) enum RecoveryConfirmationSlot {
    ActiveRecordRead,
    OldRecordDelete,
    OldRecordMissingReadback,
    UncommittedRecordDelete,
    UncommittedRecordMissingReadback,
    AdmittedRecordDelete,
    AdmittedRecordMissingReadback,
}

pub(crate) enum ActivationCleanupConfirmationSlot {
    ActiveRecordRead,
    OldRecordDelete,
    OldRecordMissingReadback,
}

pub(crate) enum CaptureCompensationConfirmationSlot {
    UncommittedRecordDelete,
    UncommittedRecordMissingReadback,
}

pub(crate) enum DeleteFinalizationConfirmationSlot {
    AdmittedRecordDelete,
    AdmittedRecordMissingReadback,
}

pub(crate) enum PendingSecretRecoveryConfirmation {
    ActivationCleanup {
        pending_confirmation_id: PendingSecretConfirmationId,
        operation_id: SecretOperationId,
        recovery_id: SecretRecoveryId,
        expected_recovery_cas: SecretRecoveryCas,
        snapshot: SecretRecoveryAuthoritySnapshot,
        prepared_active_record_read: Option<PreparedCleanupActiveRecordReadSlot>,
        prepared_old_record_delete: Option<PreparedCleanupOldRecordDeleteSlot>,
        prepared_old_record_missing_readback:
            Option<PreparedCleanupOldRecordMissingReadbackSlot>,
        pending_slot: ActivationCleanupConfirmationSlot,
        step: SecretActivationCleanupHardwareConfirmStep,
        pending: BackendPendingConfirmation,
    },
    CaptureCompensation {
        pending_confirmation_id: PendingSecretConfirmationId,
        operation_id: SecretOperationId,
        recovery_id: SecretRecoveryId,
        expected_recovery_cas: SecretRecoveryCas,
        snapshot: SecretRecoveryAuthoritySnapshot,
        prepared_uncommitted_record_delete: Option<PreparedRecoveryUncommittedRecordDeleteSlot>,
        prepared_uncommitted_record_missing_readback:
            Option<PreparedRecoveryUncommittedRecordMissingReadbackSlot>,
        pending_slot: CaptureCompensationConfirmationSlot,
        step: SecretCaptureCompensationHardwareConfirmStep,
        pending: BackendPendingConfirmation,
    },
    DeleteFinalization {
        pending_confirmation_id: PendingSecretConfirmationId,
        operation_id: SecretOperationId,
        recovery_id: SecretRecoveryId,
        expected_recovery_cas: SecretRecoveryCas,
        snapshot: SecretRecoveryAuthoritySnapshot,
        prepared_admitted_record_delete:
            Option<PreparedRecoveryAdmittedRecordDeleteSlot>,
        prepared_admitted_record_missing_readback:
            Option<PreparedRecoveryAdmittedRecordMissingReadbackSlot>,
        pending_slot: DeleteFinalizationConfirmationSlot,
        step: SecretDeleteFinalizationHardwareConfirmStep,
        pending: BackendPendingConfirmation,
    },
}

pub(crate) enum PrepareSecretRecovery {
    Prepared(PreparedSecretRecoveryBundle),
    ConfirmationRequired {
        step: SecretRecoveryHardwareConfirmStep,
        pending: PendingSecretRecoveryConfirmation,
    },
}

// Recovery preparation is consuming and material-free. Every pending/read/delete
// platform session is registered before its hardware step can be shown. Cancel,
// expiry and discard terminate the backend session and registry row; Drop is
// not relied on for recovery. Bundle/pending/slot types implement no Clone,
// Serialize, Deserialize or Debug. Only activationCleanup later takes #41's
// lease, and no hardware prompt is legal after that lease is held.

// Actual definition/factory live in
// crate::change_plan::secret_admission. #35 imports this opaque type;
// it cannot construct it or read fields directly.
pub(crate) struct AdmittedSecretChangePlan {
    plan_id: ChangePlanId,
    plan_digest: ChangePlanDigest,
    projection_digest: SecretProjectionDigest,
    admission_id: [u8; 16],
}

pub(crate) struct AdmittedSecretChangePlanIdentity<'a> {
    plan_id: &'a ChangePlanId,
    plan_digest: &'a ChangePlanDigest,
    projection_digest: &'a SecretProjectionDigest,
    admission_id: &'a [u8; 16],
}

pub(crate) struct OwnedAdmittedSecretChangePlanIdentity {
    plan_id: ChangePlanId,
    plan_digest: ChangePlanDigest,
    projection_digest: SecretProjectionDigest,
    admission_id: [u8; 16],
}

impl AdmittedSecretChangePlan {
    // This impl and the sole constructor live in
    // crate::change_plan::secret_admission. The view is immutable and has no
    // constructor/serde; #35 can inspect identity but cannot mint admission.
    pub(crate) fn identity(&self) -> AdmittedSecretChangePlanIdentity<'_> {
        AdmittedSecretChangePlanIdentity {
            plan_id: &self.plan_id,
            plan_digest: &self.plan_digest,
            projection_digest: &self.projection_digest,
            admission_id: &self.admission_id,
        }
    }
}

impl AdmittedSecretChangePlanIdentity<'_> {
    pub(crate) fn plan_id(&self) -> &ChangePlanId {
        self.plan_id
    }

    pub(crate) fn plan_digest(&self) -> &ChangePlanDigest {
        self.plan_digest
    }

    pub(crate) fn projection_digest(&self) -> &SecretProjectionDigest {
        self.projection_digest
    }

    pub(crate) fn into_owned(
        self,
    ) -> OwnedAdmittedSecretChangePlanIdentity {
        OwnedAdmittedSecretChangePlanIdentity {
            plan_id: self.plan_id.clone(),
            plan_digest: self.plan_digest.clone(),
            projection_digest: self.projection_digest.clone(),
            admission_id: *self.admission_id,
        }
    }
}

impl OwnedAdmittedSecretChangePlanIdentity {
    pub(crate) fn matches(&self, admitted: &AdmittedSecretChangePlan) -> bool {
        let current = admitted.identity();
        &self.plan_id == current.plan_id
            && &self.plan_digest == current.plan_digest
            && &self.projection_digest == current.projection_digest
            && &self.admission_id == current.admission_id
    }
}

pub(crate) trait SecretChangePlanAuthority: Send + Sync {
    // #35 receives an already-minted admission. Creation is not exposed on
    // this port; only #55's private owner-module factory may mint one.
    fn assert_still_admitted(
        &self,
        admitted: &AdmittedSecretChangePlan,
    ) -> Result<(), SecretInternalError>;

    fn consume(
        &self,
        admitted: AdmittedSecretChangePlan,
    ) -> Result<(), SecretInternalError>;

    // Consumes a still-admitted plan without applying it.
    fn terminate(
        &self,
        admitted: AdmittedSecretChangePlan,
        reason: SecretDiscardReason,
    ) -> Result<(), SecretInternalError>;

    fn assert_staged_still_admitted(
        &self,
        admitted: &AdmittedStagedSecretImportPlan,
        projection: &StagedSecretImportActivationProjection,
        authority_match: &StagedImportAuthorityMatchReceipt,
    ) -> Result<(), SecretInternalError>;

    fn staged_durable_identity(
        &self,
        admitted: &AdmittedStagedSecretImportPlan,
    ) -> Result<OwnedAdmittedStagedSecretImportIdentity, SecretInternalError>;

    fn consume_staged(
        &self,
        admitted: AdmittedStagedSecretImportPlan,
    ) -> Result<(), SecretInternalError>;

    fn terminate_staged(
        &self,
        admitted: AdmittedStagedSecretImportPlan,
        reason: SecretDiscardReason,
    ) -> Result<(), SecretInternalError>;
}

mod secret_apply_writer_sealed {
    pub(super) trait Sealed {}
    impl Sealed for
        crate::services::configuration_apply::provider::CodexTargetLiveConfigWriterAdapter
    {}
    impl Sealed for
        crate::services::configuration_apply::provider::CodexRollbackLiveConfigWriterAdapter
    {}
}

pub(crate) trait SecretApplyWriter:
    secret_apply_writer_sealed::Sealed
{
    fn live_sink_id(&self) -> CodexLiveSecretSinkId;

    // Synchronous and fixed-result: no await while material is borrowed and no
    // generic return type through which material can escape.
    fn write_and_readback(
        &mut self,
        material: &[u8],
    ) -> SecretWriterReceiptDto;
}

impl SecretApplyWriter for
    crate::services::configuration_apply::provider::CodexTargetLiveConfigWriterAdapter
{
    fn live_sink_id(&self) -> CodexLiveSecretSinkId {
        self.bound_live_sink_id()
    }

    fn write_and_readback(&mut self, material: &[u8]) -> SecretWriterReceiptDto {
        self.write_and_readback_once(material)
    }
}

impl SecretApplyWriter for
    crate::services::configuration_apply::provider::CodexRollbackLiveConfigWriterAdapter
{
    fn live_sink_id(&self) -> CodexLiveSecretSinkId {
        self.bound_live_sink_id()
    }

    fn write_and_readback(&mut self, material: &[u8]) -> SecretWriterReceiptDto {
        self.write_and_readback_once(material)
    }
}

pub(crate) enum SecretApplyWriterInvocation<'a> {
    Target(
        &'a mut crate::services::configuration_apply::provider::CodexTargetLiveConfigWriterAdapter,
    ),
    Rollback(
        &'a mut crate::services::configuration_apply::provider::CodexRollbackLiveConfigWriterAdapter,
    ),
}

impl SecretApplyWriterInvocation<'_> {
    pub(crate) fn live_sink_id(&self) -> CodexLiveSecretSinkId {
        match self {
            Self::Target(writer) => writer.bound_live_sink_id(),
            Self::Rollback(writer) => writer.bound_live_sink_id(),
        }
    }

    // Called only by crate::secret::backend's sealed callback impl.
    pub(crate) fn write_material_once(
        self,
        material: &[u8],
    ) -> SecretWriterReceiptDto {
        match self {
            Self::Target(writer) => writer.write_and_readback_once(material),
            Self::Rollback(writer) => writer.write_and_readback_once(material),
        }
    }
}

// Both concrete adapter types and their private constructors live in
// crate::services::configuration_apply::provider. Only that module's
// target/rollback job
// factories can construct them. SecretApplyWriterInvocation is the closed
// role-to-writer pairing accepted by #35. Each private adapter constructor
// requires one CodexLiveSecretSinkId and binds its exact #41 final-baseline
// projection/readback target; it never accepts or exposes a filesystem path.
// This is the complete implementer
// allowlist; there is no closure/function-pointer adapter constructor.

pub(crate) struct ExistingSecretOwnerToken {
    owner: SecretOwner,
}

impl ExistingSecretOwnerToken {
    // Credential-free inspection only. Construction/existence authority stays
    // private to crate::database::dao::providers.
    pub(crate) fn owner(&self) -> &SecretOwner {
        &self.owner
    }
}
pub(crate) struct SecretApplyAuthoritySnapshot {
    _private: (),
}
pub(crate) struct SecretCandidateAuthoritySnapshot {
    _private: (),
}

impl SecretCandidateAuthoritySnapshot {
    fn validate_activation_result_identity(
        &self,
        result: &SecretActivationResultDtoRepr,
    ) -> Result<(), SecretInternalError> {
        todo!("match candidate/plan/ref and exact affected owner set")
    }
}

// Durable, tagged device-local recovery schema. It is never a public command
// DTO and has no material/material-derived field. Custom device-store encoding
// is owned by crate::secret::device_store::recovery; the private fields prevent
// unchecked construction even inside the wider crate.
pub(in crate::secret) struct RecoveryAffectedOwner {
    owner: SecretOwner,
    owner_binding_revision: SecretOwnerBindingRevision,
    secret_ref: SecretRef,
    binding_revision: SecretBindingRevision,
}

pub(in crate::secret) struct NonEmptySortedRecoveryAffectedOwners(
    Vec<RecoveryAffectedOwner>,
);

impl NonEmptySortedRecoveryAffectedOwners {
    pub(super) fn checked(
        owners: Vec<RecoveryAffectedOwner>,
    ) -> Result<Self, SecretInternalError> {
        todo!("non-empty, strict owner sort, unique owner and active-ref match")
    }
}

pub(in crate::secret) struct NonEmptyRecoverySourceExpectations(
    CurrentLegacySourceExpectations,
);

impl NonEmptyRecoverySourceExpectations {
    pub(super) fn checked(
        values: CurrentLegacySourceExpectations,
    ) -> Result<Self, SecretInternalError> {
        if values.as_slice().is_empty() {
            Err(SecretInternalError::input_invalid())
        } else {
            Ok(Self(values))
        }
    }

    fn as_slice(&self) -> &[LegacySourceExpectation] {
        self.0.as_slice()
    }
}

pub(in crate::secret) struct FinalizeLegacyScrubRecoveryStep {
    expected_store_revision: SecretStoreRevision,
    active_secret_ref: SecretRef,
    active_record_revision: SecretRecordRevision,
    active_binding_set_cas: SecretBindingSetCas,
    backend_instance_id: SecretBackendInstanceId,
    backend_generation: SecretBackendGeneration,
    device_binding_generation: DeviceBindingGeneration,
    capability_revision: CapabilityRevision,
    source_expectations: NonEmptyRecoverySourceExpectations,
    read_confirmation: PhysicalConfirmation,
    structure_digest: RecoveryStructureDigest,
}

pub(in crate::secret) struct DeleteOldRecordRecoveryStep {
    expected_store_revision: SecretStoreRevision,
    old_secret_ref: SecretRef,
    old_record_revision: SecretRecordRevision,
    expected_old_binding_set_cas: SecretBindingSetCas,
    backend_instance_id: SecretBackendInstanceId,
    backend_generation: SecretBackendGeneration,
    device_binding_generation: DeviceBindingGeneration,
    capability_revision: CapabilityRevision,
    delete_confirmation: PhysicalConfirmation,
    required_binding_state: NoBindingsRequired,
}

pub(in crate::secret) struct VerifyOldRecordMissingRecoveryStep {
    read_confirmation: PhysicalConfirmation,
}

pub(in crate::secret) enum ActivationCleanupRecoveryStep {
    FinalizeLegacyScrub(FinalizeLegacyScrubRecoveryStep),
    DeleteOldRecord(DeleteOldRecordRecoveryStep),
    VerifyOldRecordMissing(VerifyOldRecordMissingRecoveryStep),
}

pub(in crate::secret) struct NonEmptyActivationRecoverySteps(
    Vec<ActivationCleanupRecoveryStep>,
);

impl NonEmptyActivationRecoverySteps {
    pub(super) fn checked(
        values: Vec<ActivationCleanupRecoveryStep>,
    ) -> Result<Self, SecretInternalError> {
        todo!("nonempty exact suffix in rank finalizeLegacyScrub < deleteOldRecord < verifyOldRecordMissing")
    }
}

pub(crate) enum ActivationCleanupRecoveryPhase {
    StateFinalized,
    ProviderFinalized,
    OldRecordDeleteIntent,
    OldRecordDeleteApplied {
        checkpoint: RecoveryOldRecordDeleteCheckpoint,
    },
    RecoveryRequired {
        checkpoint: ActivationOldRecordDurableCheckpoint,
    },
}

// Old-record missing readback is independently authorized and consumes the
// durable delete-applied CAS. Because it is the final recovery step, its
// receipt and the supersession + Terminal transition are committed in one
// device-authority transaction. There is no standalone nonterminal
// old-record-missing-verified phase with an empty remaining-step suffix.
pub(in crate::secret) enum ActivationCleanupOldRecordTerminal {
    NotApplicable,
    Superseded {
        disposition: BackendDeleteDisposition,
        source: RotationSupersessionSource,
        revoked_at: UtcTimestamp,
    },
}

pub(in crate::secret) enum ActivationCleanupRecoveryState {
    Nonterminal {
        phase: ActivationCleanupRecoveryPhase,
        remaining_steps: NonEmptyActivationRecoverySteps,
    },
    Terminal {
        old_record: ActivationCleanupOldRecordTerminal,
        remaining_steps: [ActivationCleanupRecoveryStep; 0],
    },
}

pub(in crate::secret) struct CaptureDeleteUncommittedRecordStep {
    delete_confirmation: PhysicalConfirmation,
}
pub(in crate::secret) struct CaptureVerifyMissingStep {
    read_confirmation: PhysicalConfirmation,
}
pub(in crate::secret) struct CaptureFinalizeCompensationStep {
    required_binding_state: NoBindingsRequired,
    terminal_candidate_state: DiscardedCandidateTerminalState,
    required_record_state: AbsentRecordState,
}
pub(in crate::secret) enum CaptureCompensationRecoveryStep {
    DeleteUncommittedRecord(CaptureDeleteUncommittedRecordStep),
    VerifyUncommittedRecordMissing(CaptureVerifyMissingStep),
    FinalizeCaptureCompensation(CaptureFinalizeCompensationStep),
}
pub(in crate::secret) struct CaptureDeleteIntentSteps(
    CaptureDeleteUncommittedRecordStep,
    CaptureVerifyMissingStep,
    CaptureFinalizeCompensationStep,
);
pub(in crate::secret) struct CaptureDeleteAppliedSteps(
    CaptureVerifyMissingStep,
    CaptureFinalizeCompensationStep,
);
pub(in crate::secret) struct CaptureMissingVerifiedSteps(
    CaptureFinalizeCompensationStep,
);
pub(in crate::secret) enum CaptureCompensationRecoveryCheckpointAndSuffix {
    None { remaining_steps: CaptureDeleteIntentSteps },
    DeleteApplied {
        delete_disposition: BackendDeleteDisposition,
        backend_completed_at: UtcTimestamp,
        delete_applied_cas: BackendDeleteAppliedCas,
        remaining_steps: CaptureDeleteAppliedSteps,
    },
    MissingReadbackVerified {
        delete_disposition: BackendDeleteDisposition,
        backend_completed_at: UtcTimestamp,
        delete_applied_cas: BackendDeleteAppliedCas,
        missing_checked_at: UtcTimestamp,
        remaining_steps: CaptureMissingVerifiedSteps,
    },
}
pub(in crate::secret) enum CaptureCompensationRecoveryState {
    DeleteIntent { remaining_steps: CaptureDeleteIntentSteps },
    DeleteApplied {
        delete_disposition: BackendDeleteDisposition,
        backend_completed_at: UtcTimestamp,
        delete_applied_cas: BackendDeleteAppliedCas,
        remaining_steps: CaptureDeleteAppliedSteps,
    },
    MissingReadbackVerified {
        delete_disposition: BackendDeleteDisposition,
        backend_completed_at: UtcTimestamp,
        delete_applied_cas: BackendDeleteAppliedCas,
        missing_checked_at: UtcTimestamp,
        remaining_steps: CaptureMissingVerifiedSteps,
    },
    RecoveryRequired {
        checkpoint_and_suffix: CaptureCompensationRecoveryCheckpointAndSuffix,
    },
    StateFinalized {
        terminal_candidate_state: DiscardedCandidateTerminalState,
        remaining_steps: [CaptureCompensationRecoveryStep; 0],
    },
    Terminal {
        terminal_candidate_state: DiscardedCandidateTerminalState,
        remaining_steps: [CaptureCompensationRecoveryStep; 0],
    },
}

pub(in crate::secret) struct DeleteAdmittedRecordRecoveryStep {
    delete_confirmation: PhysicalConfirmation,
}
pub(in crate::secret) struct DeleteVerifyMissingRecoveryStep {
    read_confirmation: PhysicalConfirmation,
}
pub(in crate::secret) struct DeleteFinalizeStateRecoveryStep {
    required_binding_state: RetainedTombstonesBindingState,
    revocation_source: UserDeleteRevocationSource,
}
pub(in crate::secret) enum DeleteFinalizationRecoveryStep {
    DeleteAdmittedRecord(DeleteAdmittedRecordRecoveryStep),
    VerifyDeletedRecordMissing(DeleteVerifyMissingRecoveryStep),
    FinalizeDeletedRecord(DeleteFinalizeStateRecoveryStep),
}
pub(in crate::secret) struct DeleteIntentSteps(
    DeleteAdmittedRecordRecoveryStep,
    DeleteVerifyMissingRecoveryStep,
    DeleteFinalizeStateRecoveryStep,
);
pub(in crate::secret) struct DeleteAppliedSteps(
    DeleteVerifyMissingRecoveryStep,
    DeleteFinalizeStateRecoveryStep,
);
pub(in crate::secret) struct DeleteMissingVerifiedSteps(
    DeleteFinalizeStateRecoveryStep,
);
pub(in crate::secret) enum DeleteFinalizationRecoveryCheckpointAndSuffix {
    None { remaining_steps: DeleteIntentSteps },
    DeleteApplied {
        delete_disposition: BackendDeleteDisposition,
        backend_completed_at: UtcTimestamp,
        delete_applied_cas: BackendDeleteAppliedCas,
        remaining_steps: DeleteAppliedSteps,
    },
    MissingReadbackVerified {
        delete_disposition: BackendDeleteDisposition,
        backend_completed_at: UtcTimestamp,
        delete_applied_cas: BackendDeleteAppliedCas,
        missing_checked_at: UtcTimestamp,
        remaining_steps: DeleteMissingVerifiedSteps,
    },
}
pub(in crate::secret) enum DeleteFinalizationRecoveryState {
    DeleteIntent { remaining_steps: DeleteIntentSteps },
    DeleteApplied {
        delete_disposition: BackendDeleteDisposition,
        backend_completed_at: UtcTimestamp,
        delete_applied_cas: BackendDeleteAppliedCas,
        remaining_steps: DeleteAppliedSteps,
    },
    MissingReadbackVerified {
        delete_disposition: BackendDeleteDisposition,
        backend_completed_at: UtcTimestamp,
        delete_applied_cas: BackendDeleteAppliedCas,
        missing_checked_at: UtcTimestamp,
        remaining_steps: DeleteMissingVerifiedSteps,
    },
    RecoveryRequired {
        checkpoint_and_suffix: DeleteFinalizationRecoveryCheckpointAndSuffix,
    },
    StateFinalized {
        revoked_at: UtcTimestamp,
        revocation_source: UserDeleteRevocationSource,
        remaining_steps: [DeleteFinalizationRecoveryStep; 0],
    },
    Terminal {
        revoked_at: UtcTimestamp,
        revocation_source: UserDeleteRevocationSource,
        remaining_steps: [DeleteFinalizationRecoveryStep; 0],
    },
}

wire_enum!(AbsentRecordState { Absent });
wire_enum!(RetainedTombstonesBindingState { RetainedTombstones });
wire_enum!(ForbiddenBackendMutation { Forbidden });

pub(in crate::secret) struct OwnerDetachFinalizeLocalStateStep {
    confirmation: NeverPhysicalConfirmation,
    backend_mutation: ForbiddenBackendMutation,
}
pub(in crate::secret) enum OwnerDetachFinalizationNonterminalPhase {
    ProviderDetachCommitted,
    LocalOwnerCasIntent,
    RecoveryRequired,
}
pub(in crate::secret) enum OwnerDetachFinalizationCompletedPhase {
    LocalOwnerCasApplied,
    Terminal,
}
pub(in crate::secret) enum OwnerDetachFinalizationRecoveryState {
    Nonterminal {
        phase: OwnerDetachFinalizationNonterminalPhase,
        remaining_steps: OwnerDetachFinalizeLocalStateStep,
    },
    Completed {
        phase: OwnerDetachFinalizationCompletedPhase,
        remaining_steps: [OwnerDetachFinalizeLocalStateStep; 0],
    },
}

pub(in crate::secret) enum DurableSecretRecoveryRecord {
    ActivationCleanup {
        recovery_id: SecretRecoveryId,
        recovery_cas: SecretRecoveryCas,
        candidate_id: SecretCandidateId,
        candidate_revision: SecretCandidateRevision,
        active_secret_ref: SecretRef,
        active_record_revision: SecretRecordRevision,
        affected_owners: NonEmptySortedRecoveryAffectedOwners,
        state: ActivationCleanupRecoveryState,
        created_at: UtcTimestamp,
        updated_at: UtcTimestamp,
    },
    CaptureCompensation {
        recovery_id: SecretRecoveryId,
        recovery_cas: SecretRecoveryCas,
        candidate_id: SecretCandidateId,
        candidate_revision: SecretCandidateRevision,
        secret_ref: SecretRef,
        record_revision: SecretRecordRevision,
        expected_store_revision: SecretStoreRevision,
        expected_binding_set_cas: SecretBindingSetCas,
        backend_instance_id: SecretBackendInstanceId,
        backend_generation: SecretBackendGeneration,
        device_binding_generation: DeviceBindingGeneration,
        capability_revision: CapabilityRevision,
        state: CaptureCompensationRecoveryState,
        created_at: UtcTimestamp,
        updated_at: UtcTimestamp,
    },
    DeleteFinalization {
        recovery_id: SecretRecoveryId,
        recovery_cas: SecretRecoveryCas,
        delete_admission: DeleteJournalAdmissionIdentity,
        secret_ref: SecretRef,
        record_revision: SecretRecordRevision,
        expected_store_revision: SecretStoreRevision,
        expected_binding_set_cas: SecretBindingSetCas,
        affected_owners: NonEmptySortedRecoveryAffectedOwners,
        backend_instance_id: SecretBackendInstanceId,
        backend_generation: SecretBackendGeneration,
        device_binding_generation: DeviceBindingGeneration,
        capability_revision: CapabilityRevision,
        revocation_source: UserDeleteRevocationSource,
        state: DeleteFinalizationRecoveryState,
        created_at: UtcTimestamp,
        updated_at: UtcTimestamp,
    },
    OwnerDetachFinalization {
        recovery_id: SecretRecoveryId,
        recovery_cas: SecretRecoveryCas,
        provider_delete_impact_id: ProviderDeleteImpactId,
        provider_row_revision: ProviderRowRevision,
        provider_detach_transaction_id: ProviderDetachTransactionId,
        provider_detach_commit_id: ProviderDetachCommitId,
        detached_owner: SecretOwner,
        expected_owner_binding_revision: SecretOwnerBindingRevision,
        expected_store_revision: SecretStoreRevision,
        legacy_source_coverage_state: NoBlockingLegacySourcesState,
        binding_view: DetachProviderOwnerBindingExpectation,
        state: OwnerDetachFinalizationRecoveryState,
        created_at: UtcTimestamp,
        updated_at: UtcTimestamp,
    },
}

impl DurableSecretRecoveryRecord {
    // The private custom codec emits the device-store wire algebra, not these
    // Rust implementation field names. RecoveryRequired encodes
    // phase=recoveryRequired, flattens checkpoint_and_suffix into the exact
    // checkpoint object plus sibling remainingSteps, and never exposes the
    // internal pairing key. StateFinalized/Terminal omit
    // intermediate receipts. Activation Terminal and owner-detach Completed
    // encode their explicit phase plus an empty array. Checked construction
    // rejects every phase/suffix pair not listed by the device-store schema.
    // Activation OldRecordDeleteApplied and its RecoveryRequired checkpoint
    // always encode the indivisible deleteDisposition/backendCompletedAt/
    // deleteAppliedCas triple. The subsequent missing receipt is a commit gate:
    // it is consumed in the same transaction that writes supersession and
    // Terminal, whose revokedAt is exactly backendCompletedAt.
    pub(super) fn checked(
        record: DurableSecretRecoveryRecord,
    ) -> Result<Self, SecretInternalError> {
        todo!("custom strict codec: exact four-arm fields, zero-count/no-legacy literals, phase receipt suffix, sorted owners/steps, full activation delete checkpoint in normal/recovery-required arms, CAS and timestamps; supersession revokedAt equals backendCompletedAt")
    }
}

pub(crate) struct RecoveryProviderProjection {
    recovery_id: SecretRecoveryId,
    recovery_cas: SecretRecoveryCas,
    candidate_id: SecretCandidateId,
    phase: ActivationCleanupRecoveryPhase,
    active_secret_ref: SecretRef,
    active_record_revision: SecretRecordRevision,
    expected_store_revision: SecretStoreRevision,
    active_binding_set_cas: SecretBindingSetCas,
    backend_instance_id: SecretBackendInstanceId,
    backend_generation: SecretBackendGeneration,
    device_binding_generation: DeviceBindingGeneration,
    capability_revision: CapabilityRevision,
    read_confirmation: PhysicalConfirmation,
    structure_digest: RecoveryStructureDigest,
    source_expectations: NonEmptyRecoverySourceExpectations,
}

impl RecoveryProviderProjection {
    // Private checked factory in crate::secret::device_store::recovery. It can
    // be created only from a FinalizeLegacyScrub expectation whose full row and
    // RecoveryCas were re-read under the recovery mutation permit.
    fn checked_from_recovery(
        recovery: &DurableSecretRecoveryRecord,
        step: &ActivationCleanupRecoveryStep,
    ) -> Result<Self, SecretInternalError> {
        todo!("accept only FinalizeLegacyScrub from the current nonterminal remaining suffix; copy exact fields and reject changed CAS")
    }

    pub(crate) fn recovery_id(&self) -> &SecretRecoveryId { &self.recovery_id }
    pub(crate) fn recovery_cas(&self) -> &SecretRecoveryCas { &self.recovery_cas }
    pub(crate) fn candidate_id(&self) -> &SecretCandidateId { &self.candidate_id }
    pub(crate) fn phase(&self) -> &ActivationCleanupRecoveryPhase {
        &self.phase
    }
    pub(crate) fn active_ref(&self) -> &SecretRef { &self.active_secret_ref }
    pub(crate) fn record_revision(&self) -> SecretRecordRevision {
        self.active_record_revision
    }
    pub(crate) fn store_revision(&self) -> SecretStoreRevision {
        self.expected_store_revision
    }
    pub(crate) fn binding_set_cas(&self) -> &SecretBindingSetCas {
        &self.active_binding_set_cas
    }
    pub(crate) fn backend_instance_id(&self) -> &SecretBackendInstanceId {
        &self.backend_instance_id
    }
    pub(crate) fn backend_generation(&self) -> SecretBackendGeneration {
        self.backend_generation
    }
    pub(crate) fn device_binding_generation(&self) -> DeviceBindingGeneration {
        self.device_binding_generation
    }
    pub(crate) fn capability_revision(&self) -> CapabilityRevision {
        self.capability_revision
    }
    pub(crate) fn confirmation(&self) -> PhysicalConfirmation {
        self.read_confirmation
    }
    pub(crate) fn structure_digest(&self) -> &RecoveryStructureDigest {
        &self.structure_digest
    }
    pub(crate) fn source_expectations(&self) -> &[LegacySourceExpectation] {
        self.source_expectations.as_slice()
    }
}

pub(crate) struct SecretRecoveryAuthoritySnapshot {
    _private: (),
}

#[derive(Clone)]
pub(crate) struct CandidateDiscardDeleteCheckpoint {
    delete_disposition: BackendDeleteDisposition,
    backend_completed_at: UtcTimestamp,
    delete_applied_cas: BackendDeleteAppliedCas,
}

pub(crate) struct CandidateDiscardDeleteApplied {
    journal: CandidateDeleteJournalRow,
    checkpoint: CandidateDiscardDeleteCheckpoint,
}

pub(crate) struct AuthorizedCandidateDiscardRecordDelete {
    backend: AuthorizedBackendDelete,
    journal: CandidateDeleteJournalRow,
}

impl AuthorizedCandidateDiscardRecordDelete {
    pub(crate) fn delete_once(
        self,
    ) -> Result<CandidateDiscardDeleteApplied, SecretInternalError> {
        let delete = self.backend.delete_once()?;
        let (delete_disposition, backend_completed_at) =
            delete.into_durable_outcome();
        let _ = (delete_disposition, backend_completed_at);
        todo!("atomically persist DiscardCandidate BackendApplied with the exact three-field checkpoint and mint its operation-bound deleteAppliedCas")
    }
}

pub(crate) struct CandidateDiscardMissingReadbackCheckpoint {
    journal: CandidateDeleteJournalRow,
    delete: CandidateDiscardDeleteCheckpoint,
    missing: BackendMissingReadbackReceipt,
}

pub(crate) struct AuthorizedCandidateDiscardRecordMissingReadback {
    backend: AuthorizedBackendMissingReadback,
    applied: CandidateDiscardDeleteApplied,
}

impl AuthorizedCandidateDiscardRecordMissingReadback {
    pub(crate) fn verify_missing_once(
        self,
        now: UtcTimestamp,
    ) -> Result<CandidateDiscardMissingReadbackCheckpoint, SecretInternalError> {
        let missing = self.backend.readback_missing_once(
            &self.applied.checkpoint.delete_applied_cas,
            now,
        )?;
        let checkpoint = CandidateDiscardMissingReadbackCheckpoint {
            journal: self.applied.journal,
            delete: self.applied.checkpoint,
            missing,
        };
        let _ = checkpoint;
        todo!("durably persist the independent MissingReadbackVerified phase before returning the checkpoint; terminal state is still forbidden")
    }
}

pub(crate) struct CaptureCompensationDeleteCheckpoint {
    snapshot: SecretRecoveryAuthoritySnapshot,
    delete: BackendDeleteReceipt,
    delete_applied_cas: BackendDeleteAppliedCas,
}
pub(crate) struct AuthorizedCaptureCompensationDelete {
    backend: AuthorizedBackendDelete,
    snapshot: SecretRecoveryAuthoritySnapshot,
}

impl AuthorizedCaptureCompensationDelete {
    pub(crate) fn delete_once(
        self,
    ) -> Result<CaptureCompensationDeleteCheckpoint, SecretInternalError> {
        let delete = self.backend.delete_once()?;
        todo!("atomically persist durable backendApplied before returning snapshot + delete receipt + new delete-applied CAS")
    }
}

pub(crate) struct CaptureCompensationMissingCheckpoint {
    snapshot: SecretRecoveryAuthoritySnapshot,
    delete: BackendDeleteReceipt,
    missing: BackendMissingReadbackReceipt,
}

pub(crate) struct AuthorizedCaptureCompensationMissingReadback {
    backend: AuthorizedBackendMissingReadback,
    checkpoint: CaptureCompensationDeleteCheckpoint,
}

impl AuthorizedCaptureCompensationMissingReadback {
    pub(crate) fn verify_missing_once(
        self,
        now: UtcTimestamp,
    ) -> Result<CaptureCompensationMissingCheckpoint, SecretInternalError> {
        let missing = self.backend.readback_missing_once(
            &self.checkpoint.delete_applied_cas,
            now,
        )?;
        todo!("persist MissingReadbackVerified separately; delete and probe are never one call")
    }
}

pub(crate) struct DeleteFinalizationDeleteCheckpoint {
    snapshot: SecretRecoveryAuthoritySnapshot,
    delete: BackendDeleteReceipt,
    delete_applied_cas: BackendDeleteAppliedCas,
}

pub(crate) struct AuthorizedDeleteFinalizationDelete {
    backend: AuthorizedBackendDelete,
    snapshot: SecretRecoveryAuthoritySnapshot,
}

impl AuthorizedDeleteFinalizationDelete {
    pub(crate) fn delete_once(
        self,
    ) -> Result<DeleteFinalizationDeleteCheckpoint, SecretInternalError> {
        let delete = self.backend.delete_once()?;
        todo!("persist deleteFinalization backendApplied and a new delete-applied CAS before any missing readback")
    }
}

pub(crate) struct DeleteFinalizationMissingCheckpoint {
    snapshot: SecretRecoveryAuthoritySnapshot,
    delete: BackendDeleteReceipt,
    missing: BackendMissingReadbackReceipt,
}

pub(crate) struct AuthorizedDeleteFinalizationMissingReadback {
    backend: AuthorizedBackendMissingReadback,
    checkpoint: DeleteFinalizationDeleteCheckpoint,
}

impl AuthorizedDeleteFinalizationMissingReadback {
    pub(crate) fn verify_missing_once(
        self,
        now: UtcTimestamp,
    ) -> Result<DeleteFinalizationMissingCheckpoint, SecretInternalError> {
        let missing = self.backend.readback_missing_once(
            &self.checkpoint.delete_applied_cas,
            now,
        )?;
        todo!("persist deleteFinalization MissingReadbackVerified independently")
    }
}

impl SecretRecoveryAuthoritySnapshot {
    fn validate_recovery_impact_identity(
        &self,
        impact: &SecretRecoveryImpactRepr,
    ) -> Result<(), SecretInternalError> {
        todo!("match recovery/candidate/ref, pending steps and affected owners")
    }

    fn validate_recovery_result_identity(
        &self,
        result: &SecretRecoveryResultRepr,
    ) -> Result<(), SecretInternalError> {
        todo!("match recovery/candidate/ref and the exact affected owner set")
    }
}
pub(crate) struct ActivationBindingCheckpoint {
    _private: (),
}
pub(crate) struct ProviderFinalizedActivationCheckpoint {
    _private: (),
}
pub(crate) struct ActivationOldRecordDeletePostconditionReceipt {
    _private: (),
}
#[derive(Clone)]
pub(crate) struct ActivationOldRecordDeleteCheckpoint {
    delete_disposition: BackendDeleteDisposition,
    backend_completed_at: UtcTimestamp,
    delete_applied_cas: BackendDeleteAppliedCas,
}

impl ActivationOldRecordDeleteCheckpoint {
    fn into_durable_failure_checkpoint(
        self,
    ) -> ActivationOldRecordDurableCheckpoint {
        ActivationOldRecordDurableCheckpoint::OldRecordDeleteApplied {
            delete_disposition: self.delete_disposition,
            backend_completed_at: self.backend_completed_at,
            delete_applied_cas: self.delete_applied_cas,
        }
    }
}

pub(crate) struct ActivationOldRecordDeleteApplied {
    postcondition: ActivationOldRecordDeletePostconditionReceipt,
    checkpoint: ActivationOldRecordDeleteCheckpoint,
}
pub(crate) struct AuthorizedActivationOldRecordDelete {
    backend: AuthorizedBackendDelete,
    postcondition: ActivationOldRecordDeletePostconditionReceipt,
}

impl AuthorizedActivationOldRecordDelete {
    pub(crate) fn delete_once(
        self,
    ) -> Result<ActivationOldRecordDeleteApplied, SecretInternalError> {
        let delete = self.backend.delete_once()?;
        let (delete_disposition, backend_completed_at) =
            delete.into_durable_outcome();
        let _ = (delete_disposition, backend_completed_at, self.postcondition);
        todo!("persist activation OldRecordDeleteApplied with exact disposition/completion/CAS and return postcondition + checkpoint; no supersession yet")
    }
}

pub(crate) struct AuthorizedActivationOldRecordMissingReadback {
    backend: AuthorizedBackendMissingReadback,
    applied: ActivationOldRecordDeleteApplied,
}

impl AuthorizedActivationOldRecordMissingReadback {
    pub(crate) fn verify_missing_once(
        self,
        now: UtcTimestamp,
    ) -> Result<ActivationOldRecordDeleteCompletion, SecretInternalError> {
        let missing = self.backend.readback_missing_once(
            &self.applied.checkpoint.delete_applied_cas,
            now,
        )?;
        let revoked_at =
            self.applied.checkpoint.backend_completed_at.clone();
        let supersession = RotationSupersessionReceipt {
            source: RotationSupersessionSource::SupersededByRotation,
            revoked_at,
        };
        Ok(ActivationOldRecordDeleteCompletion::Completed {
            postcondition: self.applied.postcondition,
            delete: self.applied.checkpoint,
            missing,
            supersession,
        })
    }
}
wire_enum!(RotationSupersessionSource { SupersededByRotation });
pub(crate) struct RotationSupersessionReceipt {
    source: RotationSupersessionSource,
    revoked_at: UtcTimestamp,
}
pub(crate) enum ActivationOldRecordDeleteCompletion {
    NotApplicable,
    Completed {
        postcondition: ActivationOldRecordDeletePostconditionReceipt,
        delete: ActivationOldRecordDeleteCheckpoint,
        missing: BackendMissingReadbackReceipt,
        supersession: RotationSupersessionReceipt,
    },
}
pub(crate) enum ActivationRecoveryCheckpoint {
    ProviderScrubPending(ActivationBindingCheckpoint),
    OldRecordDeletePending(ProviderFinalizedActivationCheckpoint),
    OldRecordMissingReadbackPending(ActivationOldRecordDeleteApplied),
}
// The two Provider receipts are actually defined in
// crate::services::configuration_apply::provider; only its lease-bound port
// implementation
// can construct them. #35 can consume but never inspect or mint them.
pub(crate) struct ProviderScrubReadbackReceipt {
    _private: (),
}
pub(crate) struct RecoveryProviderFinalizedCheckpoint {
    _private: (),
}
pub(crate) struct ProviderLegacySourceMatchReceipt {
    _private: (),
}
pub(crate) struct ProviderReplacementSourceValidationReceipt {
    _private: (),
}
pub(crate) enum ProviderActivationSourceValidationReceipt {
    CandidateEquality(ProviderLegacySourceMatchReceipt),
    ExplicitReplacement(ProviderReplacementSourceValidationReceipt),
}
pub(crate) enum RecoveryStepCheckpoint {
    Initial(SecretRecoveryAuthoritySnapshot),
    ProviderFinalized(RecoveryProviderFinalizedCheckpoint),
}
pub(crate) struct AuthorizedRecoveryOldRecordDelete {
    backend: AuthorizedBackendDelete,
}

#[derive(Clone)]
pub(crate) struct RecoveryOldRecordDeleteCheckpoint {
    delete_disposition: BackendDeleteDisposition,
    backend_completed_at: UtcTimestamp,
    delete_applied_cas: BackendDeleteAppliedCas,
}

impl RecoveryOldRecordDeleteCheckpoint {
    fn checked_from_durable_failure_checkpoint(
        checkpoint: ActivationOldRecordDurableCheckpoint,
    ) -> Result<Self, SecretInternalError> {
        match checkpoint {
            ActivationOldRecordDurableCheckpoint::OldRecordDeleteApplied {
                delete_disposition,
                backend_completed_at,
                delete_applied_cas,
            } => Ok(Self {
                delete_disposition,
                backend_completed_at,
                delete_applied_cas,
            }),
            ActivationOldRecordDurableCheckpoint::None => {
                Err(SecretInternalError::dependency_changed())
            }
        }
    }

    fn into_recovery_required_checkpoint(
        self,
    ) -> ActivationOldRecordDurableCheckpoint {
        ActivationOldRecordDurableCheckpoint::OldRecordDeleteApplied {
            delete_disposition: self.delete_disposition,
            backend_completed_at: self.backend_completed_at,
            delete_applied_cas: self.delete_applied_cas,
        }
    }
}

impl AuthorizedRecoveryOldRecordDelete {
    pub(crate) fn delete_once(
        self,
    ) -> Result<RecoveryOldRecordDeleteCheckpoint, SecretInternalError> {
        let delete = self.backend.delete_once()?;
        let (delete_disposition, backend_completed_at) =
            delete.into_durable_outcome();
        let _ = (delete_disposition, backend_completed_at);
        todo!("persist recovery old-record exact disposition/completion/CAS checkpoint before any probe")
    }
}

pub(crate) struct AuthorizedRecoveryOldRecordMissingReadback {
    backend: AuthorizedBackendMissingReadback,
    checkpoint: RecoveryOldRecordDeleteCheckpoint,
}

impl AuthorizedRecoveryOldRecordMissingReadback {
    pub(crate) fn verify_missing_once(
        self,
        now: UtcTimestamp,
    ) -> Result<RecoveryOldRecordDeleteCompletion, SecretInternalError> {
        let missing = self.backend.readback_missing_once(
            &self.checkpoint.delete_applied_cas,
            now,
        )?;
        let revoked_at = self.checkpoint.backend_completed_at.clone();
        let supersession = RotationSupersessionReceipt {
            source: RotationSupersessionSource::SupersededByRotation,
            revoked_at,
        };
        Ok(RecoveryOldRecordDeleteCompletion::Completed {
            delete: self.checkpoint,
            missing,
            supersession,
        })
    }
}
pub(crate) enum RecoveryOldRecordDeleteCompletion {
    NotPending,
    Completed {
        delete: RecoveryOldRecordDeleteCheckpoint,
        missing: BackendMissingReadbackReceipt,
        supersession: RotationSupersessionReceipt,
    },
}

pub(crate) enum SecretMutationScope<'a> {
    ApplyOwner(&'a ExistingSecretOwnerToken),
    Candidate(&'a SecretCandidateId),
    Recovery(&'a SecretRecoveryId),
    RuntimeOwner(&'a ExistingSecretOwnerToken),
}

pub(crate) struct SecretMutationPermit<'a> {
    // A real keyed std::sync::Mutex guard; never a marker/boolean lease.
    _held_guard: std::sync::MutexGuard<'a, ()>,
}

pub(crate) trait SecretMutationGate: Send + Sync {
    fn acquire<'a>(
        &'a self,
        scope: SecretMutationScope<'_>,
    ) -> Result<SecretMutationPermit<'a>, SecretInternalError>;
}

struct SecretOwnerSummaryAuthorityRow {
    owner: ExistingSecretOwnerToken,
    summary: SecretOwnerCredentialSummary,
}

struct SecretSummaryAuthoritySnapshot {
    owners: Vec<SecretOwnerSummaryAuthorityRow>,
    refs: Vec<SecretRefAggregate>,
    next_cursor: Option<SecretSummaryCursor>,
}

pub(crate) trait DeviceLocalSecretAuthority: Send + Sync {
    fn read_secret_summary_snapshot(
        &self,
        request: &ListSecretSummariesRequest,
    ) -> Result<SecretSummaryAuthoritySnapshot, SecretInternalError>;

    fn revalidate_claimed_capture_intent(
        &self,
        claim: &ClaimedSecretCaptureIntent,
        current_legacy_source_coverage: LegacySourceCoverageReceipt,
        backends: &dyn SecretBackendRegistry,
        now: &UtcTimestamp,
    ) -> Result<(), SecretInternalError>;
    // Freshly compares owner/purpose/intent/binding/coverage/hidden-binding,
    // expiry and the exact selected registered Arc/device/backend tuple.

    fn capture_intent_registration_from_atomic_snapshot(
        &self,
        owner: ExistingSecretOwnerToken,
        request: ListSecretBackendOptionsRequest,
        legacy_source_coverage: LegacySourceCoverageReceipt,
        backends: &dyn SecretBackendRegistry,
        now: &UtcTimestamp,
    ) -> Result<SecretCaptureIntentRegistration, SecretInternalError>;
    // Both receipts above are newly minted by
    // CodexLegacySourceInventoryBridge::fresh_capture_coverage. The authority
    // can consume and compare them but has no legacy-inventory method and
    // cannot construct a coverage receipt.

    fn read_apply_snapshot(
        &self,
        owner: &ExistingSecretOwnerToken,
    ) -> Result<SecretApplyAuthoritySnapshot, SecretInternalError>;

    fn read_candidate_snapshot(
        &self,
        candidate_id: &SecretCandidateId,
    ) -> Result<SecretCandidateAuthoritySnapshot, SecretInternalError>;

    fn authorize_candidate_discard_record_delete(
        &self,
        permit: &mut SecretMutationPermit<'_>,
        journal: CandidateDeleteJournalRow,
        backend: BackendInstanceHandle,
        prepared: PreparedCandidateDiscardRecordDelete,
        now: &UtcTimestamp,
    ) -> Result<AuthorizedCandidateDiscardRecordDelete, SecretInternalError>;

    fn authorize_candidate_discard_record_missing_readback(
        &self,
        permit: &mut SecretMutationPermit<'_>,
        applied: CandidateDiscardDeleteApplied,
        backend: BackendInstanceHandle,
        prepared: PreparedCandidateDiscardRecordMissingReadback,
        now: &UtcTimestamp,
    ) -> Result<AuthorizedCandidateDiscardRecordMissingReadback, SecretInternalError>;
    // This method must consume delete_applied_cas_reservation with the exact
    // operation id + CandidateDiscardDeleteCheckpoint.delete_applied_cas
    // before BackendInstanceHandle::authorize_missing_readback_once is legal.

    fn finalize_candidate_discard(
        &self,
        permit: &mut SecretMutationPermit<'_>,
        checkpoint: CandidateDiscardMissingReadbackCheckpoint,
    ) -> Result<DiscardSecretCandidateResult, SecretInternalError>;
    // Atomically removes the unbound record, writes the candidate/audit state
    // and Terminal with the journal's immutable discarded|expired target; no
    // intermediate StateFinalized or general recovery row is created.

    fn read_recovery_snapshot(
        &self,
        recovery_id: &SecretRecoveryId,
        expected: &SecretRecoveryCas,
    ) -> Result<SecretRecoveryAuthoritySnapshot, SecretInternalError>;

    fn recovery_provider_projection(
        &self,
        permit: &mut SecretMutationPermit<'_>,
        snapshot: &SecretRecoveryAuthoritySnapshot,
    ) -> Result<RecoveryProviderProjection, SecretInternalError>;

    fn mint_runtime_binding(
        &self,
        owner: ExistingSecretOwnerToken,
        consumer: FixedRuntimeConsumer,
    ) -> Result<AuthorityMintedRuntimeBinding, SecretInternalError>;
    // Implementation must fresh-check that the bound record's validated
    // allowedConsumers contains consumer.required_record_consumer(); a named
    // fixed consumer can never borrow another consumer's capability bit.

    fn authorize_apply_read(
        &self,
        permit: &mut SecretMutationPermit<'_>,
        snapshot: &SecretApplyAuthoritySnapshot,
        backend: BackendInstanceHandle,
        prepared: ClaimedPreparedSecretCapability,
        now: &UtcTimestamp,
    ) -> Result<AuthorizedApplyRead, SecretInternalError>;

    fn authorize_runtime_read(
        &self,
        permit: &mut SecretMutationPermit<'_>,
        binding: &AuthorityMintedRuntimeBinding,
        backend: BackendInstanceHandle,
        now: &UtcTimestamp,
    ) -> Result<AuthorizedRuntimeRead, SecretInternalError>;

    fn authorize_migration_read(
        &self,
        permit: &mut SecretMutationPermit<'_>,
        backend: BackendInstanceHandle,
        record: BackendRecordHandle,
        authorization: BackendAuthorizationHandle,
        operation_id: &SecretOperationId,
        now: &UtcTimestamp,
    ) -> Result<AuthorizedMigrationRead, SecretInternalError>;

    fn authorize_staged_import_read(
        &self,
        permit: &mut SecretMutationPermit<'_>,
        staged_owner: &StagedSecretOwnerToken,
        backend: BackendInstanceHandle,
        record: BackendRecordHandle,
        authorization: BackendAuthorizationHandle,
        operation_id: &SecretOperationId,
        now: &UtcTimestamp,
    ) -> Result<AuthorizedStagedImportRead, SecretInternalError>;

    fn persist_backend_revocation_observation(
        &self,
        permit: &mut SecretMutationPermit<'_>,
        observation: BackendRevocationObservation,
    ) -> Result<SecretRevocationView, SecretInternalError>;
    // The implementation destructures the consuming receipt only inside the
    // mutation permit, fresh-revalidates its ref/store/record/binding-set/
    // registered-backend/device/capability tuple, then persists source/time.
    // There is no caller-supplied ref or transplantable observation payload.

    fn commit_activation_binding(
        &self,
        permit: &mut SecretMutationPermit<'_>,
        snapshot: SecretCandidateAuthoritySnapshot,
        projection: &SecretCandidateActivationProjection,
        provider_sources: ProviderActivationSourceValidationReceipt,
    ) -> Result<ActivationBindingCheckpoint, SecretInternalError>;

    fn authorize_activation_candidate_read(
        &self,
        permit: &mut SecretMutationPermit<'_>,
        snapshot: &SecretCandidateAuthoritySnapshot,
        backend: BackendInstanceHandle,
        prepared: PreparedActivationCandidateRead,
        now: &UtcTimestamp,
    ) -> Result<AuthorizedActivationRead, SecretInternalError>;

    fn record_activation_provider_finalized(
        &self,
        permit: &mut SecretMutationPermit<'_>,
        checkpoint: ActivationBindingCheckpoint,
        provider: ProviderScrubReadbackReceipt,
    ) -> Result<ProviderFinalizedActivationCheckpoint, SecretInternalError>;

    fn authorize_activation_old_record_delete(
        &self,
        permit: &mut SecretMutationPermit<'_>,
        checkpoint: &ProviderFinalizedActivationCheckpoint,
        backend: BackendInstanceHandle,
        prepared: PreparedActivationOldRecordDelete,
        now: &UtcTimestamp,
    ) -> Result<AuthorizedActivationOldRecordDelete, SecretInternalError>;

    fn authorize_activation_old_record_missing_readback(
        &self,
        permit: &mut SecretMutationPermit<'_>,
        applied: ActivationOldRecordDeleteApplied,
        backend: BackendInstanceHandle,
        prepared: PreparedActivationOldRecordMissingReadback,
        now: &UtcTimestamp,
    ) -> Result<AuthorizedActivationOldRecordMissingReadback, SecretInternalError>;
    // Consumes the prepared reservation against
    // applied.checkpoint.delete_applied_cas; pre-confirmation alone never
    // authorizes the missing readback.

    fn finalize_activation(
        &self,
        permit: &mut SecretMutationPermit<'_>,
        checkpoint: ProviderFinalizedActivationCheckpoint,
        old_record: ActivationOldRecordDeleteCompletion,
    ) -> Result<SecretActivationResultDto, SecretInternalError>;

    fn record_activation_recovery(
        &self,
        permit: &mut SecretMutationPermit<'_>,
        checkpoint: ActivationRecoveryCheckpoint,
        failure: SecretInternalError,
    ) -> Result<SecretActivationResultDto, SecretInternalError>;

    fn record_recovery_provider_finalized(
        &self,
        permit: &mut SecretMutationPermit<'_>,
        snapshot: SecretRecoveryAuthoritySnapshot,
        provider: ProviderScrubReadbackReceipt,
    ) -> Result<RecoveryProviderFinalizedCheckpoint, SecretInternalError>;

    fn authorize_recovery_active_record_read(
        &self,
        permit: &mut SecretMutationPermit<'_>,
        snapshot: &SecretRecoveryAuthoritySnapshot,
        backend: BackendInstanceHandle,
        prepared: PreparedCleanupActiveRecordRead,
        now: &UtcTimestamp,
    ) -> Result<AuthorizedRecoveryRead, SecretInternalError>;

    fn authorize_recovery_old_record_delete(
        &self,
        permit: &mut SecretMutationPermit<'_>,
        checkpoint: &RecoveryStepCheckpoint,
        backend: BackendInstanceHandle,
        prepared: PreparedCleanupOldRecordDelete,
        now: &UtcTimestamp,
    ) -> Result<AuthorizedRecoveryOldRecordDelete, SecretInternalError>;

    fn authorize_recovery_old_record_missing_readback(
        &self,
        permit: &mut SecretMutationPermit<'_>,
        checkpoint: RecoveryOldRecordDeleteCheckpoint,
        backend: BackendInstanceHandle,
        prepared: PreparedCleanupOldRecordMissingReadback,
        now: &UtcTimestamp,
    ) -> Result<AuthorizedRecoveryOldRecordMissingReadback, SecretInternalError>;
    // RecoveryRequired must reconstruct this exact three-field checkpoint;
    // the missing authorization consumes its reservation against the retained
    // CAS and terminal supersession uses retained backend_completed_at.

    fn finalize_recovery(
        &self,
        permit: &mut SecretMutationPermit<'_>,
        checkpoint: RecoveryStepCheckpoint,
        old_record: RecoveryOldRecordDeleteCompletion,
    ) -> Result<SecretRecoveryResult, SecretInternalError>;

    fn record_recovery_failure(
        &self,
        permit: &mut SecretMutationPermit<'_>,
        checkpoint: RecoveryProviderFinalizedCheckpoint,
        failure: SecretInternalError,
    ) -> Result<SecretRecoveryResult, SecretInternalError>;

    fn authorize_capture_compensation_delete(
        &self,
        permit: &mut SecretMutationPermit<'_>,
        snapshot: SecretRecoveryAuthoritySnapshot,
        backend: BackendInstanceHandle,
        prepared: PreparedRecoveryUncommittedRecordDelete,
        now: &UtcTimestamp,
    ) -> Result<AuthorizedCaptureCompensationDelete, SecretInternalError>;

    fn authorize_capture_compensation_missing_readback(
        &self,
        permit: &mut SecretMutationPermit<'_>,
        checkpoint: CaptureCompensationDeleteCheckpoint,
        backend: BackendInstanceHandle,
        prepared: PreparedRecoveryUncommittedRecordMissingReadback,
        now: &UtcTimestamp,
    ) -> Result<AuthorizedCaptureCompensationMissingReadback, SecretInternalError>;

    fn finalize_capture_compensation(
        &self,
        permit: &mut SecretMutationPermit<'_>,
        checkpoint: CaptureCompensationMissingCheckpoint,
    ) -> Result<SecretRecoveryResult, SecretInternalError>;

    fn authorize_delete_finalization_delete(
        &self,
        permit: &mut SecretMutationPermit<'_>,
        snapshot: SecretRecoveryAuthoritySnapshot,
        backend: BackendInstanceHandle,
        prepared: PreparedRecoveryAdmittedRecordDelete,
        now: &UtcTimestamp,
    ) -> Result<AuthorizedDeleteFinalizationDelete, SecretInternalError>;

    fn authorize_delete_finalization_missing_readback(
        &self,
        permit: &mut SecretMutationPermit<'_>,
        checkpoint: DeleteFinalizationDeleteCheckpoint,
        backend: BackendInstanceHandle,
        prepared: PreparedRecoveryAdmittedRecordMissingReadback,
        now: &UtcTimestamp,
    ) -> Result<AuthorizedDeleteFinalizationMissingReadback, SecretInternalError>;

    fn finalize_deleted_record(
        &self,
        permit: &mut SecretMutationPermit<'_>,
        checkpoint: DeleteFinalizationMissingCheckpoint,
    ) -> Result<SecretRecoveryResult, SecretInternalError>;

    fn finalize_owner_detach(
        &self,
        permit: &mut SecretMutationPermit<'_>,
        snapshot: SecretRecoveryAuthoritySnapshot,
        provider: ProviderDetachCommitReceipt,
    ) -> Result<SecretRecoveryResult, SecretInternalError>;
}

// This object-safe trait and its concrete implementation live in
// crate::secret::device_store. It has no generic method and no Provider/DB
// accessor. Every authorize_* method revalidates the complete authority scope,
// invokes only BackendInstanceHandle's matching consuming read/delete wrapper,
// and returns an unforgeable route-specific Authorized*Read/Delete object.

trait ProviderLeaseBoundPort {
    fn assert_apply_final_baseline(
        &mut self,
        plan: &AdmittedSecretChangePlan,
        projection: &SecretApplyPlanProjection,
    ) -> Result<(), SecretInternalError>;

    fn assert_activation_final_baseline(
        &mut self,
        plan: &AdmittedSecretChangePlan,
        projection: &SecretCandidateActivationProjection,
    ) -> Result<(), SecretInternalError>;

    fn assert_cleanup_final_baseline(
        &mut self,
        projection: &RecoveryProviderProjection,
    ) -> Result<(), SecretInternalError>;

    // CandidateEquality only: resolve the complete exact Provider occurrence
    // set under the held lease, validate every structural revision, compare
    // every value with `expected` through ConstantTimeEq, return no material.
    fn compare_candidate_equality_activation_sources(
        &mut self,
        projection: &SecretCandidateActivationProjection,
        expected: &[u8],
    ) -> Result<ProviderLegacySourceMatchReceipt, SecretInternalError>;

    // ExplicitReplacement only: resolve the same complete exact occurrence
    // set/revisions and validate the admitted replacement impact. It receives
    // only a candidate-read receipt and MUST NOT require or compare old values.
    fn validate_explicit_replacement_sources(
        &mut self,
        projection: &SecretCandidateActivationProjection,
        candidate: CandidateReadVerifiedReceipt,
    ) -> Result<ProviderReplacementSourceValidationReceipt, SecretInternalError>;

    fn scrub_activation_and_readback(
        &mut self,
        projection: &SecretCandidateActivationProjection,
        binding: &ActivationBindingCheckpoint,
    ) -> Result<ProviderScrubReadbackReceipt, SecretInternalError>;

    fn compare_and_scrub_recovery_equality_sources(
        &mut self,
        projection: &RecoveryProviderProjection,
        expected: &[u8],
    ) -> Result<ProviderScrubReadbackReceipt, SecretInternalError>;
}

pub(crate) struct ActivationCandidateEqualityCompareCallback<'a> {
    port: &'a mut dyn ProviderLeaseBoundPort,
    projection: &'a SecretCandidateActivationProjection,
}

impl ActivationCandidateEqualityCompareCallback<'_> {
    fn new<'a>(
        port: &'a mut dyn ProviderLeaseBoundPort,
        projection: &'a SecretCandidateActivationProjection,
    ) -> ActivationCandidateEqualityCompareCallback<'a> {
        ActivationCandidateEqualityCompareCallback { port, projection }
    }

    // Visible crate-wide only so crate::secret::backend can host the sealed
    // trait impl. The type's sole constructor remains owner-private.
    pub(crate) fn write_material_once(
        self,
        material: &[u8],
    ) -> Result<ProviderLegacySourceMatchReceipt, SecretInternalError> {
        self.port
            .compare_candidate_equality_activation_sources(self.projection, material)
    }
}

pub(crate) struct RecoveryCandidateEqualityScrubCallback<'a> {
    port: &'a mut dyn ProviderLeaseBoundPort,
    projection: &'a RecoveryProviderProjection,
}

impl RecoveryCandidateEqualityScrubCallback<'_> {
    fn new<'a>(
        port: &'a mut dyn ProviderLeaseBoundPort,
        projection: &'a RecoveryProviderProjection,
    ) -> RecoveryCandidateEqualityScrubCallback<'a> {
        RecoveryCandidateEqualityScrubCallback { port, projection }
    }

    pub(crate) fn write_material_once(
        self,
        material: &[u8],
    ) -> Result<ProviderScrubReadbackReceipt, SecretInternalError> {
        self.port
            .compare_and_scrub_recovery_equality_sources(self.projection, material)
    }
}

// Actual definition/private constructor live in
// crate::commands::import_export; backend.rs owns only its sealed callback impl.
pub(crate) struct StagedImportCandidateEqualityCompareCallback<'a> {
    port: &'a mut dyn ImportCutoverPort,
    projection: &'a StagedSecretImportActivationProjection,
}

impl StagedImportCandidateEqualityCompareCallback<'_> {
    pub(crate) fn write_material_once(
        self,
        material: &[u8],
    ) -> Result<StagedImportSourceValidationReceipt, SecretInternalError> {
        self.port
            .compare_candidate_equality_staged_sources(self.projection, material)
    }
}

// These opaque contexts live in crate::services::configuration_apply::provider.
// Their
// constructors are private to that owner module and require its live Provider
// lease plus a #55 final-baseline receipt.
pub(crate) struct SecretApplyCoordinatorContext<'a> {
    port: &'a mut dyn ProviderLeaseBoundPort,
}
pub(crate) struct SecretActivationCoordinatorContext<'a> {
    port: &'a mut dyn ProviderLeaseBoundPort,
}
pub(crate) struct ActivationCleanupCoordinatorContext<'a> {
    port: &'a mut dyn ProviderLeaseBoundPort,
    expected_recovery_cas: SecretRecoveryCas,
}

// Local contexts are minted only by crate::secret::operation after readiness
// claim; they carry no Provider/DB capability.
pub(crate) struct CaptureCompensationCoordinatorContext {
    _private: (),
}
pub(crate) struct DeleteFinalizationCoordinatorContext {
    _private: (),
}

// Defined in crate::commands::provider. Constructor requires the already-held
// Provider delete/detach transaction plus the consumed preview registry row.
pub(crate) struct ProviderDetachCommitId([u8; 16]);

pub(crate) struct OwnerDetachCoordinatorContext<'a> {
    port: &'a mut dyn OwnerDetachCoordinatorPort,
    expected_provider_detach_commit_id: ProviderDetachCommitId,
}

pub(crate) trait OwnerDetachCoordinatorPort {
    fn assert_provider_detach_committed(
        &mut self,
        expected_commit_id: &ProviderDetachCommitId,
        recovery_id: &SecretRecoveryId,
        recovery_cas: &SecretRecoveryCas,
    ) -> Result<ProviderDetachCommitReceipt, SecretInternalError>;

    fn finalize_detach_transaction(
        &mut self,
        receipt: ProviderDetachCommitReceipt,
    ) -> Result<(), SecretInternalError>;
}

impl OwnerDetachCoordinatorContext<'_> {
    pub(crate) fn assert_provider_detach_committed(
        &mut self,
        recovery_id: &SecretRecoveryId,
        recovery_cas: &SecretRecoveryCas,
    ) -> Result<ProviderDetachCommitReceipt, SecretInternalError> {
        self.port.assert_provider_detach_committed(
            &self.expected_provider_detach_commit_id,
            recovery_id,
            recovery_cas,
        )
    }

    pub(crate) fn finalize_detach_transaction(
        &mut self,
        receipt: ProviderDetachCommitReceipt,
    ) -> Result<(), SecretInternalError> {
        self.port.finalize_detach_transaction(receipt)
    }
}

pub(crate) enum SecretRecoveryCoordinatorContext<'a> {
    ActivationCleanup(ActivationCleanupCoordinatorContext<'a>),
    CaptureCompensation(CaptureCompensationCoordinatorContext),
    DeleteFinalization(DeleteFinalizationCoordinatorContext),
    OwnerDetachFinalization(OwnerDetachCoordinatorContext<'a>),
}

// Defined in crate::commands::import_export; this is the sole main-integration
// cutover capability. Its constructor requires the same temp Database live
// object as StagedSecretOwnerToken and a still-admitted #55 staged plan.
pub(crate) struct ImportCutoverCoordinatorContext<'a> {
    port: &'a mut dyn ImportCutoverPort,
}

pub(crate) struct ImportCutoverReceipt {
    receipt_id: ImportCutoverReceiptId,
    durable_temp_database: TempDatabaseDurableObjectId,
    stage_id: ImportStageId,
    provider_row_revision: ProviderRowRevision,
}
pub(crate) struct StagedSourcesScrubReadbackReceipt {
    staged_source_set_cas_after_scrub: StagedSourceSetCas,
    _private: (),
}
trait ImportCutoverPort {
    fn assert_staged_final_baseline(
        &mut self,
        plan: &AdmittedStagedSecretImportPlan,
        staged_owner: &StagedSecretOwnerToken,
        projection: &StagedSecretImportActivationProjection,
    ) -> Result<(), SecretInternalError>;

    fn compare_candidate_equality_staged_sources(
        &mut self,
        projection: &StagedSecretImportActivationProjection,
        expected: &[u8],
    ) -> Result<StagedImportSourceValidationReceipt, SecretInternalError>;

    fn validate_staged_explicit_replacement(
        &mut self,
        projection: &StagedSecretImportActivationProjection,
        candidate: CandidateReadVerifiedReceipt,
    ) -> Result<StagedImportSourceValidationReceipt, SecretInternalError>;

    fn scrub_staged_sources_and_readback(
        &mut self,
        projection: &StagedSecretImportActivationProjection,
        validated: StagedImportSourceValidationReceipt,
    ) -> Result<StagedSourcesScrubReadbackReceipt, SecretInternalError>;

    fn cutover_sanitized_temp_database(
        &mut self,
        projection: &StagedSecretImportActivationProjection,
        scrubbed: StagedSourcesScrubReadbackReceipt,
    ) -> Result<ImportCutoverReceipt, SecretInternalError>;

    fn mint_live_owner_after_cutover(
        &mut self,
        receipt: &ImportCutoverReceipt,
        owner: &SecretOwner,
    ) -> Result<ExistingSecretOwnerToken, SecretInternalError>;
}

// Scanner allowlist: every ImportCutoverPort value-bearing method call occurs
// only inside the ImportCutoverCoordinatorContext impl below. The pre-context
// structural scanner cannot name the port/callback and has no staged-value API.

impl ImportCutoverCoordinatorContext<'_> {
    pub(crate) fn assert_staged_final_baseline(
        &mut self,
        plan: &AdmittedStagedSecretImportPlan,
        staged_owner: &StagedSecretOwnerToken,
        projection: &StagedSecretImportActivationProjection,
    ) -> Result<(), SecretInternalError> {
        self.port
            .assert_staged_final_baseline(plan, staged_owner, projection)
    }

    pub(crate) fn validate_staged_sources(
        &mut self,
        read: AuthorizedStagedImportRead,
        projection: &StagedSecretImportActivationProjection,
    ) -> Result<StagedImportSourceValidationReceipt, SecretInternalError> {
        match projection.comparison_policy() {
            LegacyActivationComparisonPolicy::CandidateEquality => read
                .compare_candidate_equality_once(StagedImportCandidateEqualityCompareCallback {
                    port: self.port,
                    projection,
                }),
            LegacyActivationComparisonPolicy::ExplicitReplacement => {
                let candidate = read.verify_explicit_replacement_once()?;
                self.port
                    .validate_staged_explicit_replacement(projection, candidate)
            }
        }
    }

    pub(crate) fn scrub_staged_sources_and_readback(
        &mut self,
        projection: &StagedSecretImportActivationProjection,
        validated: StagedImportSourceValidationReceipt,
    ) -> Result<StagedSourcesScrubReadbackReceipt, SecretInternalError> {
        self.port
            .scrub_staged_sources_and_readback(projection, validated)
    }

    pub(crate) fn cutover_sanitized_temp_database(
        &mut self,
        projection: &StagedSecretImportActivationProjection,
        scrubbed: StagedSourcesScrubReadbackReceipt,
    ) -> Result<ImportCutoverReceipt, SecretInternalError> {
        self.port
            .cutover_sanitized_temp_database(projection, scrubbed)
    }

    pub(crate) fn mint_live_owner_after_cutover(
        &mut self,
        receipt: &ImportCutoverReceipt,
        owner: &SecretOwner,
    ) -> Result<ExistingSecretOwnerToken, SecretInternalError> {
        self.port.mint_live_owner_after_cutover(receipt, owner)
    }
}

impl SecretApplyCoordinatorContext<'_> {
    fn new_with_held_provider_lease<'a>(
        port: &'a mut dyn ProviderLeaseBoundPort,
    ) -> SecretApplyCoordinatorContext<'a> {
        SecretApplyCoordinatorContext { port }
    }

    pub(crate) fn assert_apply_final_baseline(
        &mut self,
        plan: &AdmittedSecretChangePlan,
        projection: &SecretApplyPlanProjection,
    ) -> Result<(), SecretInternalError> {
        self.port
            .assert_apply_final_baseline(plan, projection)
    }
}

impl SecretActivationCoordinatorContext<'_> {
    fn new_with_held_provider_lease<'a>(
        port: &'a mut dyn ProviderLeaseBoundPort,
    ) -> SecretActivationCoordinatorContext<'a> {
        SecretActivationCoordinatorContext { port }
    }

    pub(crate) fn assert_activation_final_baseline(
        &mut self,
        plan: &AdmittedSecretChangePlan,
        projection: &SecretCandidateActivationProjection,
    ) -> Result<(), SecretInternalError> {
        self.port
            .assert_activation_final_baseline(plan, projection)
    }

    pub(crate) fn validate_activation_sources(
        &mut self,
        read: AuthorizedActivationRead,
        projection: &SecretCandidateActivationProjection,
    ) -> Result<ProviderActivationSourceValidationReceipt, SecretInternalError> {
        match projection.comparison_policy() {
            LegacyActivationComparisonPolicy::CandidateEquality => read
                .compare_candidate_equality_once(ActivationCandidateEqualityCompareCallback::new(
                    self.port,
                    projection,
                ))
                .map(ProviderActivationSourceValidationReceipt::CandidateEquality),
            LegacyActivationComparisonPolicy::ExplicitReplacement => {
                let candidate = read.verify_explicit_replacement_once()?;
                self.port
                    .validate_explicit_replacement_sources(projection, candidate)
                    .map(ProviderActivationSourceValidationReceipt::ExplicitReplacement)
            }
        }
    }

    pub(crate) fn scrub_activation_and_readback(
        &mut self,
        projection: &SecretCandidateActivationProjection,
        binding: &ActivationBindingCheckpoint,
    ) -> Result<ProviderScrubReadbackReceipt, SecretInternalError> {
        self.port.scrub_activation_and_readback(projection, binding)
    }
}

impl ActivationCleanupCoordinatorContext<'_> {
    fn new_with_held_provider_lease<'a>(
        port: &'a mut dyn ProviderLeaseBoundPort,
        expected_recovery_cas: SecretRecoveryCas,
    ) -> ActivationCleanupCoordinatorContext<'a> {
        ActivationCleanupCoordinatorContext {
            port,
            expected_recovery_cas,
        }
    }

    pub(crate) fn assert_cleanup_final_baseline(
        &mut self,
        projection: &RecoveryProviderProjection,
    ) -> Result<(), SecretInternalError> {
        if &self.expected_recovery_cas != projection.recovery_cas() {
            return Err(SecretInternalError::recovery_changed());
        }
        self.port.assert_cleanup_final_baseline(projection)
    }

    pub(crate) fn scrub_recovery_with_active_record(
        &mut self,
        read: AuthorizedRecoveryRead,
        projection: &RecoveryProviderProjection,
    ) -> Result<ProviderScrubReadbackReceipt, SecretInternalError> {
        read.compare_recovery_source_once(RecoveryCandidateEqualityScrubCallback::new(
            self.port,
            projection,
        ))
    }
}

pub(crate) trait NativeSecretCapture: Send + Sync {
    fn capture_once(
        &self,
        purpose: SecretPurpose,
    ) -> Result<SecretMaterial, SecretInternalError>;
}

pub(crate) trait SecretClock: Send + Sync {
    fn now(&self) -> UtcTimestamp;
}

pub(crate) trait SecretIdSource: Send + Sync {
    fn operation_id(&self) -> SecretOperationId;
    fn candidate_id(&self) -> SecretCandidateId;
    fn secret_ref(&self) -> SecretRef;
    fn audit_event_id(&self) -> SecretAuditEventId;
    fn confirmation_step_id(&self) -> SecretConfirmationStepId;
    fn recovery_id(&self) -> SecretRecoveryId;
}

pub(crate) struct SecretServiceDeps {
    pub(in crate::secret) store_lifetime: SecretStoreLifetime,
    pub(in crate::secret) authority: std::sync::Arc<dyn DeviceLocalSecretAuthority>,
    pub(in crate::secret) backends: std::sync::Arc<dyn SecretBackendRegistry>,
    pub(in crate::secret) broker: std::sync::Arc<BackendOperationBroker>,
    pub(in crate::secret) readiness: std::sync::Arc<dyn SecretReadinessRegistry>,
    pub(in crate::secret) startup_gate: std::sync::Arc<dyn SecretStartupGateRegistry>,
    pub(in crate::secret) change_plans: std::sync::Arc<dyn SecretChangePlanAuthority>,
    pub(in crate::secret) gate: std::sync::Arc<dyn SecretMutationGate>,
    pub(in crate::secret) capture: std::sync::Arc<dyn NativeSecretCapture>,
    pub(in crate::secret) clock: std::sync::Arc<dyn SecretClock>,
    pub(in crate::secret) id: std::sync::Arc<dyn SecretIdSource>,
}
// SecretServiceDeps is an internal move-only assembly row, not a dependency-
// injection API. Its sole literals are scanner-bound to the production and
// fixture factories; neither AppStateBuilder nor any caller accepts/replaces/
// extracts a broker or one of its registry traits.

// Defined in crate::store. Fields and mint functions are private to that
// module; passing the value is possible, constructing one elsewhere is not.
pub(crate) struct SecretServiceConstructionToken {
    _private: (),
}

// All types in this block live in crate::secret::device_store. The opened
// handle is non-Clone/non-serde, owns the exclusive lifetime lock and embeds
// the one bootstrap token. Only SecretBootstrap::open may derive the private
// root from AppHandle; no API accepts PathBuf/String or reopens by root.
struct DeviceLocalSecretRoot(std::path::PathBuf);
pub(crate) struct SecretBootstrapToken {
    _private: (),
}
struct DeviceLocalStoreLifetimeLock {
    _private: (),
}
pub(crate) struct OpenedDeviceLocalSecretStore {
    root: DeviceLocalSecretRoot,
    device_instance_id: DeviceInstanceId,
    device_store_instance_id: std::sync::Arc<DeviceSecretStoreInstanceId>,
    bootstrap: SecretBootstrapToken,
    lifetime_lock: DeviceLocalStoreLifetimeLock,
}
pub(crate) struct SecretBootstrap;

impl SecretBootstrap {
    pub(crate) fn open(
        app_handle: &tauri::AppHandle,
    ) -> Result<OpenedDeviceLocalSecretStore, SecretInternalError> {
        todo!("derive exact device-local root, acquire one exclusive lifetime lock")
    }
}

impl OpenedDeviceLocalSecretStore {
    pub(crate) fn database_preflight_token(&self) -> &SecretBootstrapToken {
        &self.bootstrap
    }
}

// crate::store owns this non-secret DB path/config authority. It is produced
// by the existing application path resolver; callers cannot pass a raw path.
pub(crate) struct DatabaseOpenAuthority {
    _private: (),
}

impl crate::database::Database {
    pub(crate) fn open_preflight_without_backup(
        authority: &DatabaseOpenAuthority,
        bootstrap: &SecretBootstrapToken,
    ) -> Result<std::sync::Arc<Self>, crate::error::AppError> {
        let _ = (authority, bootstrap);
        todo!("open DB/WAL with automatic/raw backup path disabled")
    }
}

pub(crate) struct SecretBootstrapCleanReceipt {
    legacy_source_coverage: LegacySourceCoverageReceipt,
    _private: (),
}

pub(crate) struct SecretStartupBlockedState {
    issue: SecretIssueView,
    legacy_source_coverage: LegacySourceCoverageReceipt,
    checked_at: UtcTimestamp,
    _private: (),
}

impl SecretBootstrapCleanReceipt {
    pub(crate) fn checked_from_clear_coverage(
        legacy_source_coverage: LegacySourceCoverageReceipt,
    ) -> Result<Self, SecretInternalError> {
        legacy_source_coverage.assert_complete_clear()?;
        Ok(Self {
            legacy_source_coverage,
            _private: (),
        })
    }
}

impl SecretStartupBlockedState {
    pub(crate) fn checked_from_coverage_and_issue(
        issue: SecretIssueView,
        legacy_source_coverage: LegacySourceCoverageReceipt,
        checked_at: UtcTimestamp,
    ) -> Result<Self, SecretInternalError> {
        legacy_source_coverage.assert_complete()?;
        let _ = &issue;
        // A legacy-source blocker additionally requires
        // assert_complete_blocking(); lock/permission/recovery blockers may
        // retain a complete clear receipt but still cannot yield Clean.
        Ok(Self {
            issue,
            legacy_source_coverage,
            checked_at,
            _private: (),
        })
    }
}

pub(crate) enum SecretStartupGateOutcome {
    Clean(SecretBootstrapCleanReceipt),
    Blocked(SecretStartupBlockedState),
}

// Defined in crate::store. It borrows the already-open preflight Database and
// exposes only exact legacy structural inventory/scrub transaction methods to
// the same SecretService; it cannot construct a second secret authority.
pub(crate) struct StartupSecretReconcileContext<'a> {
    port: Box<dyn StartupSecretReconcilePort + 'a>,
    legacy_sources: CodexLegacySourceInventoryBridge<'a>,
}

pub(crate) trait StartupSecretReconcilePort {
    fn reconcile_exact_journaled_provider_step(
        &mut self,
        projection: &RecoveryProviderProjection,
        read: AuthorizedRecoveryRead,
    ) -> Result<ProviderScrubReadbackReceipt, SecretInternalError>;
}

impl<'a> StartupSecretReconcileContext<'a> {
    // The constructor is private to crate::store. #35 receives only the
    // already-open Database-backed port and cannot open/clone a Database or
    // acquire a Provider lease itself.
    fn from_open_database_port(
        port: Box<dyn StartupSecretReconcilePort + 'a>,
        legacy_sources: CodexLegacySourceInventoryBridge<'a>,
    ) -> Self {
        StartupSecretReconcileContext {
            port,
            legacy_sources,
        }
    }

    pub(crate) fn inventory_legacy_source_coverage(
        &mut self,
    ) -> Result<LegacySourceCoverageReceipt, SecretInternalError> {
        self.legacy_sources.fresh_startup_coverage()
    }

    pub(crate) fn reconcile_exact_journaled_provider_step(
        &mut self,
        projection: &RecoveryProviderProjection,
        read: AuthorizedRecoveryRead,
    ) -> Result<ProviderScrubReadbackReceipt, SecretInternalError> {
        self.port
            .reconcile_exact_journaled_provider_step(projection, read)
    }
}

pub(crate) trait SecretStartupGateRegistry: Send + Sync {
    fn arm_managed_runtime(
        &self,
        receipt: crate::SecretCommandRegistrationReceipt,
    ) -> Result<(), SecretInternalError>;

    fn assert_managed_runtime_armed(&self) -> Result<(), SecretInternalError>;

    fn publish_clean(
        &self,
        receipt: &SecretBootstrapCleanReceipt,
    ) -> Result<(), SecretInternalError>;

    fn publish_blocked(
        &self,
        blocked: &SecretStartupBlockedState,
    ) -> Result<(), SecretInternalError>;

    fn assert_consumer_allowed(&self) -> Result<(), SecretInternalError>;
}

// crate::store is the sole owner of the port factory. It creates one
// transaction/lease adapter over the exact Arc<Database> already stored in
// AppState. It never opens a Database, resolves a path or creates secret deps.
pub(crate) fn startup_secret_reconcile_context(
    state: &AppState,
) -> Result<StartupSecretReconcileContext<'_>, crate::error::AppError> {
    let port = crate::store::database_startup_secret_port(&state.db)?;
    let legacy_sources = CodexLegacySourceInventoryBridge::from_app_state(state)?;
    Ok(StartupSecretReconcileContext::from_open_database_port(
        port,
        legacy_sources,
    ))
}

// The preparation function lives in crate::store. Its order is exact: one
// device-store open/lock, backup-suppressed DB preflight, construct
// AppState/SecretService from that same handle, then ask that same service and
// authority to reconcile through an external DB context. It does not publish a
// gate state, create a backup or start a worker. The private envelope forces
// crate-root setup to retain the exact outcome while it first manages AppState
// and completes the static command-handler registration.
pub(crate) struct PreparedProductionAppState {
    state: AppState,
    startup: SecretStartupGateOutcome,
}

impl PreparedProductionAppState {
    // Scanner-allowlisted only at the sole src-tauri/src/lib.rs setup callsite.
    pub(in crate) fn into_managed_parts(
        self,
    ) -> (AppState, SecretStartupGateOutcome) {
        (self.state, self.startup)
    }
}

pub(crate) fn open_production_app_state(
    app_handle: tauri::AppHandle,
    database_authority: DatabaseOpenAuthority,
) -> Result<PreparedProductionAppState, crate::error::AppError> {
    let opened_store = SecretBootstrap::open(&app_handle)?;
    let db = crate::database::Database::open_preflight_without_backup(
        &database_authority,
        opened_store.database_preflight_token(),
    )?;
    let state = AppState::new_production(db, app_handle, opened_store)?;
    let outcome = {
        let mut context = crate::store::startup_secret_reconcile_context(&state)?;
        state.secret_service().reconcile_startup(&mut context)?
    };
    Ok(PreparedProductionAppState {
        state,
        startup: outcome,
    })
}

// This declaration lives at crate root in src-tauri/src/lib.rs. Its only
// constructor follows app.manage(AppState) at the setup callsite after the
// statically declared invoke_handler list is installed. The two private rows
// prove exactly the 15 #35 handlers and the independent main-integration
// resume handler; resume is deliberately not a SecretCommandName variant.
pub(crate) struct ResumeStagedImportCutoverHandlerRegistration {
    command: SecretMainIntegrationCommandName,
    _private: (),
}

impl ResumeStagedImportCutoverHandlerRegistration {
    fn checked_after_handler_registration(
        command: SecretMainIntegrationCommandName,
    ) -> Result<Self, SecretInternalError> {
        if command != SecretMainIntegrationCommandName::ResumeStagedImportCutover {
            return Err(SecretInternalError::input_invalid());
        }
        Ok(Self { command, _private: () })
    }
}

pub(crate) struct SecretCommandRegistrationReceipt {
    secret_commands: [SecretCommandName; 15],
    resume_staged_import_cutover: ResumeStagedImportCutoverHandlerRegistration,
}

impl SecretCommandRegistrationReceipt {
    fn checked_after_static_registration(
        secret_commands: [SecretCommandName; 15],
        resume_staged_import_cutover: ResumeStagedImportCutoverHandlerRegistration,
    ) -> Result<Self, SecretInternalError> {
        let expected = [
            SecretCommandName::ListSecretSummaries,
            SecretCommandName::ListSecretBackendOptions,
            SecretCommandName::BeginSecretCapture,
            SecretCommandName::RotateSecret,
            SecretCommandName::ListSecretCandidates,
            SecretCommandName::DiscardSecretCandidate,
            SecretCommandName::SetSecretLocked,
            SecretCommandName::GetSecretDeleteImpact,
            SecretCommandName::DeleteSecret,
            SecretCommandName::GetSecretCleanupImpact,
            SecretCommandName::RetrySecretCleanup,
            SecretCommandName::ValidateSecret,
            SecretCommandName::CheckSecretApplyReadiness,
            SecretCommandName::MigrateLegacyCodexSecrets,
            SecretCommandName::ListSecretAudit,
        ];
        if secret_commands != expected
            || resume_staged_import_cutover.command
                != SecretMainIntegrationCommandName::ResumeStagedImportCutover
        {
            return Err(SecretInternalError::input_invalid());
        }
        Ok(Self {
            secret_commands,
            resume_staged_import_cutover,
        })
    }
}

// Called from that same setup callsite only after AppState is retrievable via
// app.state::<AppState>(). Clean authorizes one sanitized backup, then gate
// publication, then worker start. Blocked publishes the scrubbed issue but
// starts no backup/worker/consumer. The managed AppState survives both arms.
pub(crate) fn finish_managed_production_secret_startup(
    state: &AppState,
    app_handle: &tauri::AppHandle,
    outcome: SecretStartupGateOutcome,
    commands_registered: crate::SecretCommandRegistrationReceipt,
) -> Result<(), crate::error::AppError> {
    state
        .secret_service()
        .arm_managed_runtime(commands_registered)?;
    advance_managed_production_secret_startup(state, app_handle, outcome)
}

// Existing repair command handlers call this only after their durable state
// is terminal and a fresh same-service reconcile returns an outcome. The gate
// registry proves the initial manage/registration receipt was already consumed;
// no second receipt or setup call is possible.
pub(crate) fn resume_managed_production_secret_startup(
    state: &AppState,
    app_handle: &tauri::AppHandle,
    outcome: SecretStartupGateOutcome,
) -> Result<(), crate::error::AppError> {
    state
        .secret_service()
        .assert_managed_runtime_armed()?;
    advance_managed_production_secret_startup(state, app_handle, outcome)
}

fn advance_managed_production_secret_startup(
    state: &AppState,
    app_handle: &tauri::AppHandle,
    outcome: SecretStartupGateOutcome,
) -> Result<(), crate::error::AppError> {
    match outcome {
        SecretStartupGateOutcome::Clean(clean) => {
            crate::database::backup::create_sanitized_backup_after_secret_gate(
                &state.db,
                &clean,
            )?;
            state.secret_service().publish_startup_clean(&clean)?;
            crate::store::start_workers_after_secret_gate(state, app_handle, clean)?;
        }
        SecretStartupGateOutcome::Blocked(blocked) => {
            state.secret_service().publish_startup_blocked(&blocked)?;
        }
    }
    Ok(())
}

// Exact sole src-tauri/src/lib.rs setup shape (the receipt constructor is
// private in that module and scanner-bound to the line after app.manage):
// let prepared = crate::store::open_production_app_state(...)?;
// let (state, startup) = prepared.into_managed_parts();
// app.manage(state);
// let resume_handler = ResumeStagedImportCutoverHandlerRegistration::
//     checked_after_handler_registration(
//         SecretMainIntegrationCommandName::ResumeStagedImportCutover,
//     )?;
// let commands = SecretCommandRegistrationReceipt::
//     checked_after_static_registration(REGISTERED_SECRET_COMMANDS_15, resume_handler)?;
// let managed = app.state::<crate::store::AppState>();
// crate::store::finish_managed_production_secret_startup(
//     &managed, app.handle(), startup, commands,
// )?;
// REGISTERED_SECRET_COMMANDS_15 is the literal §9 array in the same order as
// the 15 #35 invoke handlers. The independently registered resume handler is
// adjacent in the Tauri handler list but can never enter that array/type.

// A lock/permission/backend-unavailable observation, any unresolved current
// legacy source state, any adjacent-blocked supplemental observation, or any
// non-terminal durable recovery is a reachable
// security outcome, never a construction error: reconcile_startup returns
// Ok(Blocked(...)), the store publishes that blocker, and AppState reaches the
// scrubbed summary plus the existing capture/migrate/discard/recovery repair
// routes. Those routes do not call assert_consumer_allowed. After a successful
// repair the store invokes resume_managed_production_secret_startup with
// a fresh outcome from this same service and the same
// AppState, SecretService, authority and lifetime lock. A new Clean receipt is
// the sole authority for the first sanitized backup and worker start. Only
// unrecoverable device-store/journal corruption or loss of the already-held
// lifetime lock may leave reconcile_startup as Err and abort construction.
// Clean consumes and retains the exact fresh clear LegacySourceCoverageReceipt;
// Blocked retains the exact blocking receipt beside its checked issue. Neither
// result can be minted from a count/category projection or divergent scan.
// Runtime reads and live apply call assert_consumer_allowed both when minting
// authority and immediately before consume; a Blocked state therefore cannot
// race into material exposure, writer mutation or network construction.

pub(in crate::secret) enum SecretStoreLifetime {
    Production(OpenedDeviceLocalSecretStore),
    #[cfg(any(test, feature = "test-hooks"))]
    Test,
}

pub(crate) struct SecretService {
    store_lifetime: SecretStoreLifetime,
    authority: std::sync::Arc<dyn DeviceLocalSecretAuthority>,
    backends: std::sync::Arc<dyn SecretBackendRegistry>,
    broker: std::sync::Arc<BackendOperationBroker>,
    readiness: std::sync::Arc<dyn SecretReadinessRegistry>,
    startup_gate: std::sync::Arc<dyn SecretStartupGateRegistry>,
    change_plans: std::sync::Arc<dyn SecretChangePlanAuthority>,
    gate: std::sync::Arc<dyn SecretMutationGate>,
    capture: std::sync::Arc<dyn NativeSecretCapture>,
    clock: std::sync::Arc<dyn SecretClock>,
    id: std::sync::Arc<dyn SecretIdSource>,
}

// Existing crate::store::AppState retains every existing field. The only
// additive field is the independently owned secret_service Arc; SecretService has no
// Database, Provider DAO, Provider lease or transaction field.
pub struct AppState {
    pub db: std::sync::Arc<crate::database::Database>,
    pub proxy_service: crate::services::ProxyService,
    pub usage_cache: std::sync::Arc<crate::services::UsageCache>,
    pub codex_desktop_service: std::sync::Arc<crate::services::CodexDesktopService>,
    secret_service: std::sync::Arc<SecretService>,
}

impl AppState {
    pub(crate) fn new_production(
        db: std::sync::Arc<crate::database::Database>,
        app_handle: tauri::AppHandle,
        opened_store: OpenedDeviceLocalSecretStore,
    ) -> Result<Self, SecretInternalError> {
        let construction = SecretServiceConstructionToken { _private: () };
        let secret_service = crate::secret::device_store::new_production_service(
            construction,
            app_handle,
            opened_store,
        )?;
        todo!("construct existing AppState fields unchanged")
    }

    #[cfg(any(test, feature = "test-hooks"))]
    fn new_with_secret_test_mode(
        db: std::sync::Arc<crate::database::Database>,
        mode: crate::test_support::SecretTestFixtureMode,
    ) -> Self {
        let construction = SecretServiceConstructionToken { _private: () };
        let secret_service = crate::secret::device_store::new_test_service(
            construction,
            mode,
        );
        todo!("construct existing AppState test fields unchanged")
    }

    pub(crate) fn secret_service(&self) -> &std::sync::Arc<SecretService> {
        &self.secret_service
    }

}

// These two narrow functions live in crate::secret::device_store. Their deps
// builder is private to that module; no caller can submit production authority
// or backend implementations.
fn production_service_deps(
    _app_handle: tauri::AppHandle,
    opened_store: OpenedDeviceLocalSecretStore,
) -> Result<SecretServiceDeps, SecretInternalError> {
    let broker = BackendOperationBroker::from_production_store(&opened_store)?;
    let _ = (opened_store, broker);
    todo!("construct fixed authority/backends/readiness/startup gate and move this exact broker Arc into SecretServiceDeps")
}

pub(crate) fn new_production_service(
    construction: SecretServiceConstructionToken,
    app_handle: tauri::AppHandle,
    opened_store: OpenedDeviceLocalSecretStore,
) -> Result<std::sync::Arc<SecretService>, SecretInternalError> {
    let deps = production_service_deps(app_handle, opened_store)?;
    Ok(std::sync::Arc::new(SecretService::from_deps(
        construction,
        deps,
    )))
}

#[cfg(any(test, feature = "test-hooks"))]
pub(crate) fn new_test_service(
    construction: SecretServiceConstructionToken,
    mode: crate::test_support::SecretTestFixtureMode,
) -> std::sync::Arc<SecretService> {
    let deps = secret_test_support::for_mode(mode);
    std::sync::Arc::new(SecretService::from_deps(
        construction,
        deps,
    ))
}

#[cfg(any(test, feature = "test-hooks"))]
mod secret_test_support {
    // Private fixed support factory. No raw dependency/service factory is
    // exported through the test-hooks feature.
    pub(super) fn for_mode(
        mode: crate::test_support::SecretTestFixtureMode,
    ) -> super::SecretServiceDeps {
        let broker = BackendOperationBroker::from_fixture_mode(mode);
        let _ = (mode, broker);
        todo!("construct fixed test deps and move this exact broker Arc into SecretServiceDeps")
    }
}

#[cfg(any(test, feature = "test-hooks"))]
fn test_database(
) -> Result<std::sync::Arc<crate::database::Database>, crate::error::AppError> {
    todo!("fixed in-memory database fixture")
}

#[cfg(any(test, feature = "test-hooks"))]
pub(crate) fn build_test_app_state(
    mode: crate::test_support::SecretTestFixtureMode,
    database: Option<std::sync::Arc<crate::database::Database>>,
) -> Result<AppState, crate::error::AppError> {
    let db = match database {
        Some(database) => database,
        None => test_database()?,
    };
    Ok(AppState::new_with_secret_test_mode(db, mode))
}

#[cfg(any(test, feature = "test-hooks"))]
pub mod test_support {
    // Re-exported as fyagent_lib::test_support::AppStateBuilder. Fields and
    // support/dependency types stay private; integration crates can only choose
    // a closed fault mode and optionally preserve one caller-owned non-secret
    // Arc<Database> through named methods.
    #[derive(Clone, Copy, PartialEq, Eq)]
    pub enum SecretTestFixtureMode {
        InMemory,
        LockedRead,
        DeniedRead,
        BackendUnavailable,
        VerifyMismatchOnce,
        OldDeleteFailOnce,
    }

    pub struct AppStateBuilder {
        mode: SecretTestFixtureMode,
        database: Option<std::sync::Arc<crate::database::Database>>,
    }

    impl AppStateBuilder {
        pub fn new() -> Self {
            Self {
                mode: SecretTestFixtureMode::InMemory,
                database: None,
            }
        }

        pub fn fixture_mode(mut self, mode: SecretTestFixtureMode) -> Self {
            self.mode = mode;
            self
        }

        pub fn with_database(
            mut self,
            database: std::sync::Arc<crate::database::Database>,
        ) -> Self {
            self.database = Some(database);
            self
        }

        pub fn build(self) -> Result<super::AppState, crate::error::AppError> {
            crate::store::build_test_app_state(self.mode, self.database)
        }
    }
}

impl SecretService {
    // Unique SecretService constructor. Static ownership permits calls only
    // from device_store::new_production_service/new_test_service; those two
    // narrow functions are themselves called only by the two AppState
    // constructors above. Struct literals are forbidden outside this impl.
    pub(in crate::secret) fn from_deps(
        _construction: SecretServiceConstructionToken,
        deps: SecretServiceDeps,
    ) -> Self {
        Self {
            store_lifetime: deps.store_lifetime,
            authority: deps.authority,
            backends: deps.backends,
            broker: deps.broker,
            readiness: deps.readiness,
            startup_gate: deps.startup_gate,
            change_plans: deps.change_plans,
            gate: deps.gate,
            capture: deps.capture,
            clock: deps.clock,
            id: deps.id,
        }
    }

    pub(crate) fn list_secret_summaries(
        &self,
        request: ListSecretSummariesRequest,
        legacy_sources: &mut CodexLegacySourceInventoryBridge<'_>,
    ) -> Result<ListSecretSummariesResult, SecretInternalError> {
        let snapshot = self.authority.read_secret_summary_snapshot(&request)?;
        let mut owners = Vec::with_capacity(snapshot.owners.len());
        for row in snapshot.owners {
            let coverage = legacy_sources
                .fresh_owner_summary_coverage(&row.owner)?;
            owners.push(SecretOwnerCredentialSummary::checked_from_authority(
                row.summary,
                &coverage,
            )?);
        }
        ListSecretSummariesResult::checked_from_authority(
            ListSecretSummariesResult {
                owners,
                refs: snapshot.refs,
                next_cursor: snapshot.next_cursor,
            },
        )
    }

    pub(crate) fn list_secret_backend_options(
        &self,
        owner: ExistingSecretOwnerToken,
        request: ListSecretBackendOptionsRequest,
        legacy_sources: &mut CodexLegacySourceInventoryBridge<'_>,
    ) -> Result<ListSecretBackendOptionsResult, SecretInternalError> {
        let now = self.clock.now();
        let legacy_source_coverage = legacy_sources
            .fresh_capture_coverage(&owner)?;
        let registration = self.authority
            .capture_intent_registration_from_atomic_snapshot(
                owner,
                request,
                legacy_source_coverage,
                self.backends.as_ref(),
                &now,
            )?;
        self.broker
            .mint_capture_intent_from_atomic_snapshot(registration)
    }

    pub(crate) fn begin_secret_capture(
        &self,
        request: BeginSecretCaptureRequest,
        legacy_sources: &mut CodexLegacySourceInventoryBridge<'_>,
    ) -> Result<StageSecretCandidateResult, SecretInternalError> {
        let now = self.clock.now();
        let claim = self.broker.claim_capture_intent_and_fresh_revalidate(
            request.capture_intent_id,
            &request.backend_instance_id,
            &now,
            legacy_sources,
            self.authority.as_ref(),
            self.backends.as_ref(),
        )?;
        match self.stage_claimed_capture(&claim, &now) {
            Ok(result) => {
                self.broker.consume_capture_intent(claim)?;
                Ok(result)
            }
            Err(error) => {
                self.broker.terminalize_capture_intent(
                    claim,
                    PendingConfirmationTermination::Failed,
                )?;
                Err(error)
            }
        }
    }

    fn stage_claimed_capture(
        &self,
        claim: &ClaimedSecretCaptureIntent,
        now: &UtcTimestamp,
    ) -> Result<StageSecretCandidateResult, SecretInternalError> {
        let _ = (claim, now);
        todo!("single native capture/write/verify/journal flow; native input cancellation is an Err consumed by begin_secret_capture terminalization")
    }

    pub(crate) fn check_apply_readiness(
        &self,
        owner: ExistingSecretOwnerToken,
        request: CheckSecretApplyReadinessRequest,
    ) -> Result<SecretApplyReadiness, SecretInternalError> {
        self.startup_gate.assert_consumer_allowed()?;
        todo!("closed contract implementation")
    }

    // Called before the Provider lease. It never reads material.
    pub(crate) fn prepare_for_apply(
        &self,
        plan: AdmittedSecretChangePlan,
        projection: SecretApplyPlanProjection,
    ) -> Result<PrepareForApply, SecretInternalError> {
        self.startup_gate.assert_consumer_allowed()?;
        todo!("closed contract implementation")
    }

    // Called before the Provider lease. Consumes pending native state.
    pub(crate) fn confirm_for_apply(
        &self,
        pending: PendingSecretConfirmation,
    ) -> Result<PrepareForApply, SecretInternalError> {
        todo!("closed contract implementation")
    }

    // Consumes pending native state, terminates its backend session, invalidates
    // every already-prepared role, terminates the admission and registry row.
    pub(crate) fn cancel_for_apply(
        &self,
        pending: PendingSecretConfirmation,
        reason: PendingConfirmationTermination,
    ) -> Result<(), SecretInternalError> {
        todo!("closed contract implementation")
    }

    // Expiry calls cancel_for_apply(..., Expired); renderer cancellation calls
    // UserCancelled; job/baseline discard calls Discarded. No Drop-only cleanup.
    pub(crate) fn discard_prepared(
        &self,
        capabilities: PreparedSecretCapabilityBundle,
        reason: SecretDiscardReason,
    ) -> Result<(), SecretInternalError> {
        todo!("closed contract implementation")
    }

    // The registered discard_secret_candidate handler calls this native-only
    // preparation entry. It creates/loads the immutable disposition journal,
    // generates a fresh operation id and prepares exactly RecordDelete then
    // reservation-bound RecordMissingReadback before any backend mutation.
    pub(crate) fn prepare_candidate_discard(
        &self,
        request: DiscardSecretCandidateRequest,
    ) -> Result<PrepareCandidateDiscard, SecretInternalError> {
        todo!("closed two-slot candidate-discard preparation; terminal replay returns AlreadyTerminal without a slot")
    }

    pub(crate) fn confirm_candidate_discard(
        &self,
        pending: PendingCandidateDiscardConfirmation,
    ) -> Result<PrepareCandidateDiscard, SecretInternalError> {
        todo!("consume only the pending variant's fixed slot; after RecordDelete confirmation prepare/confirm RecordMissingReadback before returning a bundle")
    }

    pub(crate) fn cancel_candidate_discard(
        &self,
        pending: PendingCandidateDiscardConfirmation,
        reason: PendingConfirmationTermination,
    ) -> Result<(), SecretInternalError> {
        todo!("terminalize the fresh operation/pending backend session while preserving the durable journal target and candidate reachability")
    }

    pub(crate) fn discard_prepared_candidate_discard(
        &self,
        bundle: PreparedCandidateDiscardBundle,
        reason: SecretDiscardReason,
    ) -> Result<(), SecretInternalError> {
        todo!("invalidate both one-shot authorizations; preserve immutable nonterminal candidate journal")
    }

    pub(crate) fn execute_candidate_discard(
        &self,
        bundle: PreparedCandidateDiscardBundle,
    ) -> Result<DiscardSecretCandidateResult, SecretInternalError> {
        todo!("under one candidate mutation permit consume delete, persist three-field checkpoint, unlock/consume Validate missing, persist MissingReadbackVerified, then finalize exact disposition")
    }

    // Activation is prepared separately from live apply. Before #41 takes a
    // lease it prepares the mandatory candidate-read/compare authorization
    // and, when projected, the old-record delete authorization. It never reads
    // material during prepare/confirm.
    pub(crate) fn prepare_candidate_activation(
        &self,
        plan: AdmittedSecretChangePlan,
        projection: SecretCandidateActivationProjection,
    ) -> Result<PrepareCandidateActivation, SecretInternalError> {
        todo!("closed contract implementation")
    }

    pub(crate) fn confirm_candidate_activation(
        &self,
        pending: PendingCandidateActivationConfirmation,
    ) -> Result<PrepareCandidateActivation, SecretInternalError> {
        todo!("closed contract implementation")
    }

    pub(crate) fn cancel_candidate_activation(
        &self,
        pending: PendingCandidateActivationConfirmation,
        reason: PendingConfirmationTermination,
    ) -> Result<(), SecretInternalError> {
        todo!("closed contract implementation")
    }

    pub(crate) fn discard_prepared_activation(
        &self,
        bundle: PreparedCandidateActivationBundle,
        reason: SecretDiscardReason,
    ) -> Result<(), SecretInternalError> {
        todo!("closed contract implementation")
    }

    // Called after Provider lease + #55 baseline recheck + backup.
    pub(crate) fn resolve_for_apply(
        &self,
        coordinator: &mut SecretApplyCoordinatorContext<'_>,
        capabilities: &mut PreparedSecretCapabilityBundle,
        invocation: SecretApplyWriterInvocation<'_>,
    ) -> Result<SecretApplyResultDto, SecretInternalError> {
        self.startup_gate.assert_consumer_allowed()?;
        todo!("closed contract implementation")
    }

    // Invalidates every still-prepared role and consumes the plan admission.
    pub(crate) fn finish_apply(
        &self,
        capabilities: PreparedSecretCapabilityBundle,
    ) -> Result<(), SecretInternalError> {
        todo!("closed contract implementation")
    }

    // #35 never acquires a Provider lease. #41 passes the already-held lease +
    // final baseline token, and this call performs fresh compare, local CAS,
    // Provider scrub and journal finalization before that lease is released.
    pub(crate) fn activate_candidate_from_change_plan(
        &self,
        coordinator: &mut SecretActivationCoordinatorContext<'_>,
        bundle: PreparedCandidateActivationBundle,
    ) -> Result<SecretActivationResultDto, SecretInternalError> {
        todo!("closed contract implementation")
    }

    // Main integration, not #41, prepares/optionally confirms this bundle
    // before it acquires its opaque temp-DB cutover context.
    pub(crate) fn prepare_staged_import(
        &self,
        plan: AdmittedStagedSecretImportPlan,
        staged_owner: StagedSecretOwnerToken,
        authority_match: StagedImportAuthorityMatchReceipt,
        projection: StagedSecretImportActivationProjection,
    ) -> Result<PrepareStagedImport, SecretInternalError> {
        todo!("closed staged-only preparation")
    }

    pub(crate) fn confirm_staged_import(
        &self,
        pending: PendingStagedImportConfirmation,
    ) -> Result<PrepareStagedImport, SecretInternalError> {
        todo!("consume exact staged pending confirmation")
    }

    pub(crate) fn cancel_staged_import(
        &self,
        pending: PendingStagedImportConfirmation,
        reason: PendingConfirmationTermination,
    ) -> Result<(), SecretInternalError> {
        todo!("terminate staged pending/admission state")
    }

    pub(crate) fn discard_prepared_staged_import(
        &self,
        bundle: PreparedStagedImportBundle,
        reason: SecretDiscardReason,
    ) -> Result<(), SecretInternalError> {
        todo!("consume candidate authorization, terminate staged admission and registry state")
    }

    pub(crate) fn activate_staged_import(
        &self,
        coordinator: &mut ImportCutoverCoordinatorContext<'_>,
        bundle: PreparedStagedImportBundle,
    ) -> Result<StagedSecretImportActivationResultDto, SecretInternalError> {
        todo!("validate/scrub/cutover/mint live owner/finalize local binding")
    }

    pub(crate) fn get_recovery_impact(
        &self,
        request: GetSecretCleanupImpactRequest,
    ) -> Result<SecretRecoveryImpact, SecretInternalError> {
        todo!("closed contract implementation")
    }

    // Called by retry_secret_cleanup before selecting the kind-specific
    // coordinator. It prepares exactly that row's hardware/backend slots;
    // only a completed activationCleanup bundle later asks #41 for a lease.
    pub(crate) fn prepare_recovery(
        &self,
        request: RetrySecretCleanupRequest,
    ) -> Result<PrepareSecretRecovery, SecretInternalError> {
        todo!("closed contract implementation")
    }

    pub(crate) fn confirm_recovery(
        &self,
        pending: PendingSecretRecoveryConfirmation,
    ) -> Result<PrepareSecretRecovery, SecretInternalError> {
        todo!("closed contract implementation")
    }

    pub(crate) fn cancel_recovery(
        &self,
        pending: PendingSecretRecoveryConfirmation,
        reason: PendingConfirmationTermination,
    ) -> Result<(), SecretInternalError> {
        todo!("terminate backend pending session and pending registry row")
    }

    pub(crate) fn discard_prepared_recovery(
        &self,
        bundle: PreparedSecretRecoveryBundle,
        reason: SecretDiscardReason,
    ) -> Result<(), SecretInternalError> {
        todo!("invalidate every prepared cleanup authorization")
    }

    pub(crate) fn retry_recovery(
        &self,
        coordinator: &mut SecretRecoveryCoordinatorContext<'_>,
        bundle: PreparedSecretRecoveryBundle,
    ) -> Result<SecretRecoveryResult, SecretInternalError> {
        todo!("closed contract implementation")
    }

    // This is the only startup reconciliation entry. It uses the production
    // authority already retained by this service and the port over AppState's
    // already-open Database. It never constructs a temporary authority,
    // reopens the device store or starts a consumer.
    pub(crate) fn reconcile_startup(
        &self,
        context: &mut StartupSecretReconcileContext<'_>,
    ) -> Result<SecretStartupGateOutcome, SecretInternalError> {
        let legacy_source_coverage =
            context.inventory_legacy_source_coverage()?;
        let _ = legacy_source_coverage;
        todo!(
            "consume this fresh complete eleven-domain receipt into Clean only when both retained sets are empty, otherwise retain it in Blocked; map lock/recovery blockers to Ok(Blocked), reserve Err for fatal store corruption or inability to retain the lifetime lock"
        )
    }

    pub(crate) fn publish_startup_clean(
        &self,
        clean: &SecretBootstrapCleanReceipt,
    ) -> Result<(), SecretInternalError> {
        self.startup_gate.publish_clean(clean)
    }

    pub(crate) fn arm_managed_runtime(
        &self,
        receipt: crate::SecretCommandRegistrationReceipt,
    ) -> Result<(), SecretInternalError> {
        self.startup_gate.arm_managed_runtime(receipt)
    }

    pub(crate) fn assert_managed_runtime_armed(
        &self,
    ) -> Result<(), SecretInternalError> {
        self.startup_gate.assert_managed_runtime_armed()
    }

    pub(crate) fn publish_startup_blocked(
        &self,
        blocked: &SecretStartupBlockedState,
    ) -> Result<(), SecretInternalError> {
        self.startup_gate.publish_blocked(blocked)
    }
}

// Owned only by crate::secret::device_store. The authority mints it from one
// fresh device-local binding snapshot; no DAO/runtime module can assemble it.
struct RuntimeSecretBindingIdentityOwned {
    owner: ExistingSecretOwnerToken,
    owner_binding_revision: SecretOwnerBindingRevision,
    secret_ref: SecretRef,
    binding_revision: SecretBindingRevision,
    record_revision: SecretRecordRevision,
    store_revision: SecretStoreRevision,
    binding_set_cas: SecretBindingSetCas,
}

pub(crate) struct AuthorityMintedRuntimeBinding {
    consumer: FixedRuntimeConsumer,
    identity: RuntimeSecretBindingIdentityOwned,
    authority_nonce: [u8; 16],
}

// Borrow-only, non-authorizing identity view. Only
// AuthorityMintedRuntimeBinding::identity can construct it.
pub(crate) struct RuntimeSecretBindingIdentity<'a> {
    owner: &'a ExistingSecretOwnerToken,
    owner_binding_revision: &'a SecretOwnerBindingRevision,
    secret_ref: &'a SecretRef,
    binding_revision: &'a SecretBindingRevision,
    record_revision: &'a SecretRecordRevision,
    store_revision: SecretStoreRevision,
    binding_set_cas: &'a SecretBindingSetCas,
}

impl AuthorityMintedRuntimeBinding {
    // This factory is private in crate::secret::device_store and is called only
    // by its DeviceLocalSecretAuthority implementation after a fresh read.
    fn mint(
        consumer: FixedRuntimeConsumer,
        identity: RuntimeSecretBindingIdentityOwned,
        authority_nonce: [u8; 16],
    ) -> Self {
        Self { consumer, identity, authority_nonce }
    }

    pub(crate) fn identity(&self) -> RuntimeSecretBindingIdentity<'_> {
        RuntimeSecretBindingIdentity {
            owner: &self.identity.owner,
            owner_binding_revision: &self.identity.owner_binding_revision,
            secret_ref: &self.identity.secret_ref,
            binding_revision: &self.identity.binding_revision,
            record_revision: &self.identity.record_revision,
            store_revision: self.identity.store_revision,
            binding_set_cas: &self.identity.binding_set_cas,
        }
    }

    pub(crate) fn require_consumer(
        &self,
        expected: FixedRuntimeConsumer,
    ) -> Result<(), SecretInternalError> {
        if std::mem::discriminant(&self.consumer) == std::mem::discriminant(&expected) {
            Ok(())
        } else {
            Err(SecretInternalError::terminal_operation_failure(
                SecretSourceFreeErrorCode::DependencyChanged,
                SecretTerminalOperationContext::Runtime(expected),
            ))
        }
    }
}

// These wrappers live in their exact runtime owner modules. Their private
// constructors accept only an authority-minted token; no constructor accepts
// owner/ref/revision scalar fields.
pub(crate) struct ProxyRuntimeSecretBinding {
    authority: AuthorityMintedRuntimeBinding,
}
pub(crate) struct UsageRuntimeSecretBinding {
    authority: AuthorityMintedRuntimeBinding,
}
pub(crate) struct CodingPlanRuntimeSecretBinding {
    authority: AuthorityMintedRuntimeBinding,
}
pub(crate) struct ModelFetchRuntimeSecretBinding {
    authority: AuthorityMintedRuntimeBinding,
}

impl ProxyRuntimeSecretBinding {
    fn from_authority(
        authority: AuthorityMintedRuntimeBinding,
    ) -> Result<Self, SecretInternalError> {
        authority.require_consumer(FixedRuntimeConsumer::ProxyRequest)?;
        Ok(Self { authority })
    }

    fn identity(&self) -> RuntimeSecretBindingIdentity<'_> {
        self.authority.identity()
    }
}

impl UsageRuntimeSecretBinding {
    fn from_authority(
        authority: AuthorityMintedRuntimeBinding,
    ) -> Result<Self, SecretInternalError> {
        authority.require_consumer(FixedRuntimeConsumer::UsageProbe)?;
        Ok(Self { authority })
    }

    fn identity(&self) -> RuntimeSecretBindingIdentity<'_> {
        self.authority.identity()
    }
}

impl CodingPlanRuntimeSecretBinding {
    fn from_authority(
        authority: AuthorityMintedRuntimeBinding,
    ) -> Result<Self, SecretInternalError> {
        authority.require_consumer(FixedRuntimeConsumer::CodingPlanUsageProbe)?;
        Ok(Self { authority })
    }

    fn identity(&self) -> RuntimeSecretBindingIdentity<'_> {
        self.authority.identity()
    }
}

impl ModelFetchRuntimeSecretBinding {
    fn from_authority(
        authority: AuthorityMintedRuntimeBinding,
    ) -> Result<Self, SecretInternalError> {
        authority.require_consumer(FixedRuntimeConsumer::ModelFetch)?;
        Ok(Self { authority })
    }

    fn identity(&self) -> RuntimeSecretBindingIdentity<'_> {
        self.authority.identity()
    }
}

impl RuntimeSecretBindingIdentity<'_> {
    pub(crate) fn owner(&self) -> &ExistingSecretOwnerToken {
        self.owner
    }

    pub(crate) fn owner_binding_revision(&self) -> &SecretOwnerBindingRevision {
        self.owner_binding_revision
    }

    pub(crate) fn secret_ref(&self) -> &SecretRef {
        self.secret_ref
    }

    pub(crate) fn binding_revision(&self) -> &SecretBindingRevision {
        self.binding_revision
    }

    pub(crate) fn record_revision(&self) -> &SecretRecordRevision {
        self.record_revision
    }

    pub(crate) fn store_revision(&self) -> SecretStoreRevision {
        self.store_revision
    }

    pub(crate) fn binding_set_cas(&self) -> &SecretBindingSetCas {
        self.binding_set_cas
    }
}

pub(crate) struct ProxyRequestSecretExecution {
    binding: ProxyRuntimeSecretBinding,
    metadata: ProxyRequestMetadata,
    request: ProxySingleSendRequestHandle,
}

wire_enum!(ProxyHttpMethod { Get, Post });
wire_enum!(CodexProxyRoute { Responses, ChatCompletions });

pub(crate) struct NoRedirectPolicy;

impl NoRedirectPolicy {
    // Owner-private HTTP client factories are required to call this exact
    // function; no caller-supplied redirect policy or default client is legal.
    fn reqwest_policy(&self) -> reqwest::redirect::Policy {
        reqwest::redirect::Policy::none()
    }
}

pub(crate) struct ProxyRequestMetadata {
    operation_id: SecretOperationId,
    method: ProxyHttpMethod,
    route: CodexProxyRoute,
    upstream: ValidatedUrl,
    content_length: u64,
    timeout_millis: u32,
    redirect_policy: NoRedirectPolicy,
}

pub(crate) struct ProxySingleSendRequestHandle {
    _private: (),
}

pub(crate) struct ProxyRequestExecutionReceipt {
    _private: (),
}
pub(crate) struct PreparedProxyRequest {
    metadata: ProxyRequestMetadata,
    request: ProxySingleSendRequestHandle,
    authorized_single_send: Zeroizing<Vec<u8>>,
}

impl ProxyRequestSecretExecution {
    fn new(
        binding: ProxyRuntimeSecretBinding,
        metadata: ProxyRequestMetadata,
        request: ProxySingleSendRequestHandle,
    ) -> Self {
        Self { binding, metadata, request }
    }

    pub(crate) fn binding_identity(&self) -> RuntimeSecretBindingIdentity<'_> {
        self.binding.identity()
    }

    pub(crate) fn authority_binding(&self) -> &AuthorityMintedRuntimeBinding {
        &self.binding.authority
    }

    // Called only by crate::secret::backend's sealed callback impl; the sole
    // constructor remains private to crate::proxy::forwarder.
    pub(crate) fn write_material_once(
        self,
        material: &[u8],
    ) -> Result<PreparedProxyRequest, SecretInternalError> {
        Ok(PreparedProxyRequest {
            metadata: self.metadata,
            request: self.request,
            authorized_single_send: Zeroizing::new(material.to_vec()),
        })
    }
}

impl PreparedProxyRequest {
    // Consumes metadata, request body/route and authorization in one transport
    // await. There is no retry/clone/get-header API.
    pub(crate) async fn send_once(
        self,
    ) -> Result<ProxyRequestExecutionReceipt, SecretInternalError> {
        todo!("owner-module single transport await")
    }
}

pub(crate) enum UsageProbeKind {
    Usage,
    Balance,
}

pub(crate) struct UsageProbeMetadata {
    operation_id: SecretOperationId,
    probe: UsageProbeKind,
    upstream: ValidatedUrl,
    timeout_millis: u32,
    redirect_policy: NoRedirectPolicy,
}

pub(crate) struct UsageProbeSingleSendRequestHandle {
    _private: (),
}

pub(crate) struct UsageProbeSecretExecution {
    binding: UsageRuntimeSecretBinding,
    metadata: UsageProbeMetadata,
    request: UsageProbeSingleSendRequestHandle,
}
pub(crate) struct UsageProbeExecutionReceipt {
    _private: (),
}
pub(crate) struct PreparedUsageProbeRequest {
    metadata: UsageProbeMetadata,
    request: UsageProbeSingleSendRequestHandle,
    authorized_single_send: Zeroizing<Vec<u8>>,
}

impl UsageProbeSecretExecution {
    fn new(
        binding: UsageRuntimeSecretBinding,
        metadata: UsageProbeMetadata,
        request: UsageProbeSingleSendRequestHandle,
    ) -> Self {
        Self { binding, metadata, request }
    }

    pub(crate) fn binding_identity(&self) -> RuntimeSecretBindingIdentity<'_> {
        self.binding.identity()
    }

    pub(crate) fn authority_binding(&self) -> &AuthorityMintedRuntimeBinding {
        &self.binding.authority
    }

    pub(crate) fn write_material_once(
        self,
        material: &[u8],
    ) -> Result<PreparedUsageProbeRequest, SecretInternalError> {
        Ok(PreparedUsageProbeRequest {
            metadata: self.metadata,
            request: self.request,
            authorized_single_send: Zeroizing::new(material.to_vec()),
        })
    }
}

impl PreparedUsageProbeRequest {
    pub(crate) async fn send_once(
        self,
    ) -> Result<UsageProbeExecutionReceipt, SecretInternalError> {
        todo!("owner-module single transport await")
    }
}

pub(crate) enum CodingPlanPrimaryAdapter {
    Kimi,
    Zhipu,
    MiniMax,
}

pub(crate) struct CodingPlanMetadata {
    operation_id: SecretOperationId,
    adapter: CodingPlanPrimaryAdapter,
    upstream: ValidatedUrl,
    timeout_millis: u32,
    redirect_policy: NoRedirectPolicy,
}

pub(crate) struct CodingPlanSingleSendRequestHandle {
    _private: (),
}

pub(crate) struct CodingPlanSecretExecution {
    binding: CodingPlanRuntimeSecretBinding,
    metadata: CodingPlanMetadata,
    request: CodingPlanSingleSendRequestHandle,
}
pub(crate) struct CodingPlanExecutionReceipt {
    _private: (),
}
pub(crate) struct PreparedCodingPlanRequest {
    metadata: CodingPlanMetadata,
    request: CodingPlanSingleSendRequestHandle,
    authorized_single_send: Zeroizing<Vec<u8>>,
}

impl CodingPlanSecretExecution {
    fn new(
        binding: CodingPlanRuntimeSecretBinding,
        metadata: CodingPlanMetadata,
        request: CodingPlanSingleSendRequestHandle,
    ) -> Self {
        Self { binding, metadata, request }
    }

    pub(crate) fn binding_identity(&self) -> RuntimeSecretBindingIdentity<'_> {
        self.binding.identity()
    }

    pub(crate) fn authority_binding(&self) -> &AuthorityMintedRuntimeBinding {
        &self.binding.authority
    }

    pub(crate) fn write_material_once(
        self,
        material: &[u8],
    ) -> Result<PreparedCodingPlanRequest, SecretInternalError> {
        Ok(PreparedCodingPlanRequest {
            metadata: self.metadata,
            request: self.request,
            authorized_single_send: Zeroizing::new(material.to_vec()),
        })
    }
}

impl PreparedCodingPlanRequest {
    pub(crate) async fn send_once(
        self,
    ) -> Result<CodingPlanExecutionReceipt, SecretInternalError> {
        todo!("fixed primary-key adapter; one await; redirect policy none")
    }
}

pub(crate) struct ModelFetchMetadata {
    operation_id: SecretOperationId,
    upstream: ValidatedUrl,
    model_provider_id: CodexModelProviderId,
    timeout_millis: u32,
    redirect_policy: NoRedirectPolicy,
}

pub(crate) struct ModelFetchSingleSendRequestHandle {
    _private: (),
}

pub(crate) struct ModelFetchSecretExecution {
    binding: ModelFetchRuntimeSecretBinding,
    metadata: ModelFetchMetadata,
    request: ModelFetchSingleSendRequestHandle,
}
pub(crate) struct ModelFetchExecutionReceipt {
    _private: (),
}
pub(crate) struct PreparedModelFetchRequest {
    metadata: ModelFetchMetadata,
    request: ModelFetchSingleSendRequestHandle,
    authorized_single_send: Zeroizing<Vec<u8>>,
}

impl ModelFetchSecretExecution {
    fn new(
        binding: ModelFetchRuntimeSecretBinding,
        metadata: ModelFetchMetadata,
        request: ModelFetchSingleSendRequestHandle,
    ) -> Self {
        Self { binding, metadata, request }
    }

    pub(crate) fn binding_identity(&self) -> RuntimeSecretBindingIdentity<'_> {
        self.binding.identity()
    }

    pub(crate) fn authority_binding(&self) -> &AuthorityMintedRuntimeBinding {
        &self.binding.authority
    }

    pub(crate) fn write_material_once(
        self,
        material: &[u8],
    ) -> Result<PreparedModelFetchRequest, SecretInternalError> {
        Ok(PreparedModelFetchRequest {
            metadata: self.metadata,
            request: self.request,
            authorized_single_send: Zeroizing::new(material.to_vec()),
        })
    }
}

impl PreparedModelFetchRequest {
    pub(crate) async fn send_once(
        self,
    ) -> Result<ModelFetchExecutionReceipt, SecretInternalError> {
        todo!("owner-module single transport await")
    }
}

// The type blocks above live in their exact owner modules, not crate::secret:
// proxy types: crate::proxy::forwarder; the Codex adapter supplies only closed
// route metadata and cannot construct or retain the execution token
// usage types: crate::services::provider::usage
// primary-key coding-plan types: crate::services::coding_plan
// model-fetch types: crate::services::model_fetch
// Each execution token has a private owner-module constructor and exactly one
// scanner-allowlisted factory callsite. #35 receives it opaquely and can only
// ask for binding_identity, then pass the token only to the backend-owned
// sealed callback. No crate-wide adapter constructor exists.

impl SecretService {
    pub(crate) fn mint_proxy_runtime_binding(
        &self,
        owner: ExistingSecretOwnerToken,
    ) -> Result<AuthorityMintedRuntimeBinding, SecretInternalError> {
        self.startup_gate.assert_consumer_allowed()?;
        self.authority
            .mint_runtime_binding(owner, FixedRuntimeConsumer::ProxyRequest)
    }

    pub(crate) fn mint_usage_runtime_binding(
        &self,
        owner: ExistingSecretOwnerToken,
    ) -> Result<AuthorityMintedRuntimeBinding, SecretInternalError> {
        self.startup_gate.assert_consumer_allowed()?;
        self.authority
            .mint_runtime_binding(owner, FixedRuntimeConsumer::UsageProbe)
    }

    pub(crate) fn mint_model_fetch_runtime_binding(
        &self,
        owner: ExistingSecretOwnerToken,
    ) -> Result<AuthorityMintedRuntimeBinding, SecretInternalError> {
        self.startup_gate.assert_consumer_allowed()?;
        self.authority
            .mint_runtime_binding(owner, FixedRuntimeConsumer::ModelFetch)
    }

    pub(crate) fn mint_coding_plan_runtime_binding(
        &self,
        owner: ExistingSecretOwnerToken,
    ) -> Result<AuthorityMintedRuntimeBinding, SecretInternalError> {
        self.startup_gate.assert_consumer_allowed()?;
        self.authority.mint_runtime_binding(
            owner,
            FixedRuntimeConsumer::CodingPlanUsageProbe,
        )
    }

    pub(crate) async fn execute_proxy_request(
        self: &std::sync::Arc<Self>,
        request: ProxyRequestSecretExecution,
    ) -> Result<ProxyRequestExecutionReceipt, SecretInternalError> {
        self.startup_gate.assert_consumer_allowed()?;
        todo!("closed contract implementation")
    }

    pub(crate) async fn execute_usage_probe(
        self: &std::sync::Arc<Self>,
        request: UsageProbeSecretExecution,
    ) -> Result<UsageProbeExecutionReceipt, SecretInternalError> {
        self.startup_gate.assert_consumer_allowed()?;
        todo!("closed contract implementation")
    }

    pub(crate) async fn execute_coding_plan_usage_probe(
        self: &std::sync::Arc<Self>,
        request: CodingPlanSecretExecution,
    ) -> Result<CodingPlanExecutionReceipt, SecretInternalError> {
        self.startup_gate.assert_consumer_allowed()?;
        todo!("spawn_blocking resolve, then one redirect-none send_once await")
    }

    pub(crate) async fn execute_model_fetch(
        self: &std::sync::Arc<Self>,
        request: ModelFetchSecretExecution,
    ) -> Result<ModelFetchExecutionReceipt, SecretInternalError> {
        self.startup_gate.assert_consumer_allowed()?;
        todo!("closed contract implementation")
    }
}

wire_enum!(SecretDiscardReason {
    PlanStale, BaselineChanged, BackupFailed, JobCancelled, Shutdown
});
```

#### 7.3.1 Opaque-token ownership and factory allowlist

Public/crate visibility of a type is not construction authority. Every row below has private fields, no `Default/Clone/Deserialize`, and a constructor private to the named owner module. #35 may receive/call the opaque value but cannot mint or reconstruct it.

| Opaque type | True owner module | Sole factory/callsite |
| --- | --- | --- |
| `BackendRecordLocator` / `BackendRecordHandle` / `BackendVerifyReceiptId` | `crate::secret::backend` subtree | backend adapter lookup/record creation/write-readback receipt minting inside that subtree; locator parse/read and receipt construction never leave it |
| `BackendAuthorizationHandle` / `BackendPendingConfirmation` / scope-specific `Authorized*Read` / exact independent delete and missing-readback wrappers | `crate::secret::backend` subtree | registered-backend prepare/confirm/read/delete wrappers only; consuming objects cannot be forged, cloned, deserialized, combined or cross-routed |
| `BackendAuthorizationScope` / `RegisteredBackendHandleBinding` / `PlatformOperationRequirement` | `crate::secret::backend` subtree | private `mint_from_context` plus registered backend prepare only; the exact registered `Arc` and complete scope move into auth/pending/authorized handles and are rechecked before every platform action |
| `BackendRevocationHint` / `BackendRevocationObservation` | `crate::secret::backend` subtree | ordinary probe/read may produce only a non-persistable hint; the observation wrapper is minted solely by `observe_revocation_once` after consuming exact Revoke authorization and validating source/time capability plus the full registered-handle/store/record/backend/device tuple |
| `ExistingSecretOwnerToken` | `crate::database::dao::providers` | successful exact Provider-row lookup; renderer `OwnerId` text alone is never authority |
| `CodexLegacySourceInventoryBridge` / `LegacySourceInventoryRevision` / `CompleteLegacySourceCoverageIdentity` / `CompleteLegacySourceInventoryAuthority` / `LegacySourceCoverageReceipt` | `crate::legacy_source_inventory` main-integration bridge plus `crate::secret::legacy_source_coverage` | only the bridge's fixed eleven-adapter fresh inventory privately constructs the positive non-value revision, exact named-domain identity and unforgeable authority; the receipt child module's `LegacySourceCoverageReceipt::checked_from_complete_inventory_authority` consumes that authority and retains the exact revision/identity/current expectations/adjacent observations. Store/Provider/command/other-secret siblings receive only the `pub(crate)` re-export and may name, move, validate or consume it; none can access fields or construct the authority, identity, revision or receipt |
| generated `SecretRef/CandidateId/OperationId/...` / `DurableSecretOperationJournal` / `DurableSecretRecoveryRecord` / concrete `DeviceLocalSecretAuthority` | `crate::secret::device_store` | native RNG/device-authority and strict journal/recovery checked factories; wire deserialization is forbidden on server-generated outputs and durable input is accepted only through the private exact eight-arm/four-arm codec |
| cleanup/activation root result wrappers and private reprs | `crate::secret::device_store::result` | authority snapshot factories only; custom `Serialize`, no `Deserialize`/struct-literal construction |
| `SecretCapabilityId` / `PendingSecretConfirmationId` / `SecretReadinessId` and registrations | `crate::secret::operation` | private checked complete registration rows; registries alone mint ids and atomically record/claim/consume/expire state |
| `AdmittedSecretChangePlan` / `AdmittedStagedSecretImportPlan` / immutable identity views | `crate::change_plan::secret_admission` (#55) | private factories after operation-specific plan id/digest/projection admission; #35's authority port exposes only identity/assert/consume/terminate, never admit/mint |
| `SecretApplyCoordinatorContext` / `SecretActivationCoordinatorContext` / `ActivationCleanupCoordinatorContext` / Provider ports and receipts | `crate::services::configuration_apply::provider` (#41) | private factories in `src-tauri/src/services/configuration_apply/provider.rs` while that module holds the Provider lease and exact final baseline/recovery CAS; none carries `Database` |
| `CaptureCompensationCoordinatorContext` / `DeleteFinalizationCoordinatorContext` | `crate::secret::operation` | minted only after the exact readiness claim; local-only and no Provider/DB authority |
| `OwnerDetachCoordinatorContext` / `ProviderDeleteImpactRegistration` | `crate::commands::provider` (main integration) | private factory while the exact single-use Provider-delete preview and detach transaction are held; never constructible from wire ids/revisions |
| `StagedSecretOwnerToken` / `StagedImportAdmissionAuthority` / temp durable+process-live identities / equality+resume authority ports / `ImportCutoverCoordinatorContext` / staged source receipts | `crate::commands::import_export` (main integration) | private factories bound to one temp `Database` live object, durable object id, fresh process nonce, stage/owner/row revision and admitted staged plan; only the owner equality API emits a consuming backend-scope receipt, and restart must reopen/reconcile/mint fresh authority before #55 readmission |
| `CodexTargetLiveConfigWriterAdapter` / `CodexRollbackLiveConfigWriterAdapter` | `crate::services::configuration_apply::provider` | private target/rollback job factories; only two `SecretApplyWriter` sealed impls |
| `ProxyRuntimeSecretBinding` / `ProxyRequestSecretExecution` / `PreparedProxyRequest` | `crate::proxy::forwarder` | one forwarder factory and consuming `send_once`; `crate::proxy::providers::codex` supplies only closed route metadata |
| `UsageRuntimeSecretBinding` / `UsageProbeSecretExecution` / `PreparedUsageProbeRequest` | `crate::services::provider::usage` | one usage/balance factory and consuming `send_once` |
| `CodingPlanRuntimeSecretBinding` / `CodingPlanSecretExecution` / `PreparedCodingPlanRequest` | `crate::services::coding_plan` | one primary Provider coding-plan factory and consuming redirect-disabled `send_once` |
| `ModelFetchRuntimeSecretBinding` / `ModelFetchSecretExecution` / `PreparedModelFetchRequest` | `crate::services::model_fetch` | one model-fetch factory and consuming `send_once` |
| `AuthorityMintedRuntimeBinding` / `RuntimeSecretBindingIdentity` | `crate::secret::device_store` | four fixed `SecretService::mint_*_runtime_binding` call paths; each runtime owner wraps one token and cannot submit identity scalars |
| `OpenedDeviceLocalSecretStore` / `SecretBootstrapToken` / lifetime lock | `crate::secret::device_store` | only `SecretBootstrap::open(&AppHandle)` before DB preflight; the non-cloneable opened handle supplies the borrow-only DB token and is moved unchanged into `AppState` |
| `PreparedProductionAppState` / `StartupSecretReconcileContext` / `SecretBootstrapCleanReceipt` / `SecretStartupBlockedState` / `SecretStartupGateRegistry` | `crate::store` plus the already-constructed `SecretService` | store-private preparation wraps only the existing DB and returns `AppState + Clean|Blocked`; the sole `src-tauri/src/lib.rs` setup consumes the envelope, manages AppState, proves static command registration, and only then completes backup/gate/workers. No temporary authority, second store open or raw path exists |
| `SecretCommandRegistrationReceipt` / `ResumeStagedImportCutoverHandlerRegistration` | crate root `src-tauri/src/lib.rs` setup | checked private receipt immediately after the sole `app.manage(state)` and installed static handler list; stores the exact 15-element `SecretCommandName` array plus the independently typed main-integration resume handler, which cannot enter that enum; consumed by `finish_managed_production_secret_startup`, never exported or minted by store/#35 |
| `Arc<SecretService>` / `Arc<BackendOperationBroker>` | `crate::store::AppState` plus `crate::secret::device_store` private assembly | store-only production `AppState::new_production(db, app_handle, opened_store)` consumes the already-opened handle before reconciliation and before management; private non-public deps move the exact internally built broker Arc into the sole service field. Integration tests have only feature-gated `fyagent_lib::test_support::AppStateBuilder`, never raw deps/support/service/broker/registry factories, setters or extractors; both reach the token-gated single `SecretService::from_deps` constructor |
| `AppStateBuilder` / `SecretTestFixtureMode` | crate-root feature-gated `test_support` | exact integration shape is `AppStateBuilder::new().fixture_mode(mode).with_database(Arc<Database>).build()`; both setters are optional, default is a fresh memory DB plus `inMemory`, and `with_database` preserves that exact non-secret Arc for caller readback. The closed no-value mode is `inMemory/lockedRead/deniedRead/backendUnavailable/verifyMismatchOnce/oldDeleteFailOnce`; raw secret traits/deps/material/support/service factories remain private |

The normative module/file map is one-to-one: `crate::store` startup composition/gate/context bridge → existing `src-tauri/src/store.rs`; `crate::legacy_source_inventory` main-integration bridge → `src-tauri/src/legacy_source_inventory.rs`; `crate::secret::{material,backend,migration,legacy_source_coverage}` → `src-tauri/src/secret/{material,backend,migration,legacy_source_coverage}.rs`; `crate::secret::device_store` → `src-tauri/src/secret/device_store/mod.rs` plus its platform-neutral device-store children; `crate::change_plan::secret_admission` → `src-tauri/src/change_plan/secret_admission.rs` declared only by `src-tauri/src/change_plan.rs`; `crate::services::configuration_apply::provider` → `src-tauri/src/services/configuration_apply/provider.rs`; `crate::commands::provider` → `src-tauri/src/commands/provider.rs`; `crate::commands::import_export` → `src-tauri/src/commands/import_export.rs`; `crate::proxy::forwarder` → `src-tauri/src/proxy/forwarder.rs`; `crate::services::provider::usage` → `src-tauri/src/services/provider/usage.rs`; `crate::services::coding_plan` → `src-tauri/src/services/coding_plan.rs`; and `crate::services::model_fetch` → `src-tauri/src/services/model_fetch.rs`. No alias module or duplicate adapter file may own these types.

Implementation visibility is frozen to these narrow declarations: crate root declares `pub(crate) mod legacy_source_inventory` and that module exports only `CodexLegacySourceInventoryBridge`, the three opaque identity/authority type names needed by the secret checked factory, and no raw source port; `src-tauri/src/proxy/mod.rs` adds `pub(crate) use forwarder::{ProxyRequestSecretExecution, PreparedProxyRequest, ProxyRequestExecutionReceipt}`; `src-tauri/src/services/provider/mod.rs` adds `pub(crate) use usage::{UsageProbeSecretExecution, PreparedUsageProbeRequest, UsageProbeExecutionReceipt}` and only the exact live writer types needed by #41; `src-tauri/src/database/dao/mod.rs` adds `pub(crate) use providers::ExistingSecretOwnerToken`; `src-tauri/src/services/mod.rs` keeps its existing public `coding_plan`/`model_fetch` modules and adds `pub(crate) mod configuration_apply`, whose `mod.rs` declares `pub(crate) mod provider`; `src-tauri/src/commands/mod.rs` keeps the existing private `provider` and `import_export` children and exports only their exact handler-level opaque context constructors; and `src-tauri/src/secret/mod.rs` re-exports `pub(crate) LegacySourceCoverageReceipt` plus only the service/public views and coordinator-consumed opaque wrappers listed here. `src-tauri/src/lib.rs` adds only feature-gated public `test_support::{AppStateBuilder,SecretTestFixtureMode}`, backed by a crate-private `store.rs` bridge; it does not export secret deps/support/service factories. #35 core contains only route traits and never imports an external concrete callback. Each existing lane-owner or main-integration adapter module implements its exact allowlisted seal/base/route triple after its own types land; no lane type is created or narrow-re-exported merely to make backend.rs compile. No new glob re-export is permitted.

Checked-factory visibility/callsites are also closed: `LegacySourceCoverageReceipt::checked_from_complete_inventory_authority` is `pub(crate)` solely because the sibling main-integration bridge must call it, but its argument is constructible only as a private literal inside `CodexLegacySourceInventoryBridge::collect_complete_inventory_authority`; the bridge immediately consumes that authority and never returns it. The scanner rejects any authority/identity/revision/receipt literal, second authority constructor, receipt factory call outside `CodexLegacySourceInventoryBridge::fresh_complete_coverage`, dynamic domain collection, or receipt `Clone/Serde/Debug/Default` impl. `SecretBackendInstanceView::try_registered` and `SecretRecordCapabilities::try_new` are `pub(in crate::secret)` only because the three backend-owned platform impls need them, and scanner-call only from `crate::secret::backend::RegisteredSecretBackend`; `BackendRecordLocator::parse`, broker-only context literals, `BackendAuthorizationScope::mint_from_context`, `observe_revocation_once`, revocation-receipt construction and auth/pending mint/consume are module-private with only the registered wrapper callsites; `BackendOperationBroker` privately owns the capture-intent/capability/pending registries and is the only registry caller; `SecretReadinessRegistration::checked` and prepared role slots are private to `crate::secret::operation`; the general sorted recovery-step checked constructors are `pub(in crate::secret)` solely for operation registration plus result validation; `CandidateDeleteJournalRow::{for_explicit_discard,for_expiry_sweep}` are private to device-store candidate operation/sweeper callsites and retry has no constructor; `NonEmptySortedRecoveryAffectedOwners::checked`, `NonEmptyRecoverySourceExpectations::checked`, each kind-specific durable recovery expectation factory and `DurableSecretRecoveryRecord::checked` are `pub(super)` inside `crate::secret::device_store::recovery`; root recovery/activation private repr/result factories and affected-owner/summary wrappers are `pub(super)` inside `crate::secret::device_store::result`; `RecoveryProviderProjection::checked_from_recovery` is private to the activation-recovery owner; staged token/projection/durable-object/process-live authority factories, equality comparison and restart reopen/reconcile/fresh-authority factories are private to `crate::commands::import_export`, with only the named handler/#55 readmission callsites; and `PreparedProductionAppState::into_managed_parts`, the one crate-root checked 15+1 registration receipt plus `finish_managed_production_secret_startup` form one scanner-ordered setup sequence after `app.manage`, while later repair may call only the armed `resume_managed_production_secret_startup`. The scanner enumerates those exact calls and rejects any second callsite, struct literal, visibility widening or unchecked `From`/`Default` path.

Candidate discard adds exactly two and only two general-operation confirmation slots: `CandidateDiscardConfirmationSlot::{RecordDelete,RecordMissingReadback}`. The scanner requires their literal operation/scope pairs to be `Delete/candidateDiscardRecordDelete` and `Validate/candidateDiscardRecordMissingReadback`; the durable row must repeat those two fixed slot values and their independent confirmation policies. It requires `CandidateDiscardDeleteCheckpoint` and both durable `BackendApplied`/`RecoveryRequired` encodings to carry exactly `deleteDisposition + backendCompletedAt + deleteAppliedCas`, and requires the missing phase to retain that same typed checkpoint plus `missingCheckedAt`. It rejects a former single-slot `CandidateDelete` variant under `SecretNonApplyBackendOperation`, a combined delete-plus-probe method/type, a missing-readback authorization before the operation-owned `BackendDeleteAppliedCasReservation` is fulfilled by the durable checkpoint, a candidate-discard `StateFinalized` arm, any third candidate slot, or any sixth hardware operation.

Normal activation, durable failure and cleanup recovery keep their three same-field checkpoint types non-interchangeable; scanner allows only `into_durable_failure_checkpoint`, `checked_from_durable_failure_checkpoint` and `into_recovery_required_checkpoint`, never `From/Into`, literals outside the owner or CAS-only reconstruction. The five `StagedImportResumePhase` arms, their cumulative receipt rows, zero-count after-scrub CAS and the immutable common journal `operationId` are likewise scanner-enumerated; an `Option`, flattened bag, former three-arm staged checkpoint type, missing cumulative field or extra phase is rejected.

The contract scanner rejects a struct literal, `new/from_*` call, `Default/Clone/Deserialize` impl or second factory callsite for any row outside its owner. Narrow `pub(crate)` re-exports are fixed: `crate::secret` exposes `SecretService`, coordinator-consumed scope-specific backend objects and public views; backend locators/material/callback seals/platform ports stay inside `crate::secret::{backend,material}`; #55 exports only admitted tokens/immutable identity views; #41 exports only its three apply/activation/activation-recovery contexts, fixed receipts and two live writers; main integration exports only its owner-detach and staged-cutover opaque contexts. It also rejects an `admit/mint` method on #35's `SecretChangePlanAuthority`, an owner-text resolver on `DeviceLocalSecretAuthority`, and `Arc<Database>`, `Database`, `Provider` or a Provider DAO/service field inside `SecretService`. `AppState` itself remains `pub`; existing `db/proxy_service/usage_cache/codex_desktop_service` fields and their visibility remain unchanged. Only the additive `secret_service: Arc<SecretService>` field and its construction token are private, so outside modules cannot use a struct literal and reach it only through the crate-private `secret_service()` accessor. Command/coordinator owners must resolve `OwnerId` to `ExistingSecretOwnerToken` before calling #35; `check_apply_readiness` consumes that opaque token alongside the wire request. All Provider reads/writes used by apply/activation/activation-recovery flow only through the already-held #41 lease-bound context; Provider delete/detach and staged cutover use only their already-held main-integration contexts.

The four runtime methods are the only Codex non-apply material consumers. Their method name fixes `proxyRequest/usageProbe/codingPlanUsageProbe/modelFetch` and `processMemory`; each accepts one opaque execution token constructed only by its real owner module around an `AuthorityMintedRuntimeBinding` returned by #35. `codingPlanUsageProbe` (`FixedRuntimeConsumer::CodingPlanUsageProbe`) is the fixed primary-Provider coding-plan adapter and belongs to `usageProbe/codex_feature_runtime`; it is not a caller-selectable generic consumer. Runtime owners cannot construct owner/ref/revision fields. The authority token contains the exact existing-owner/binding expectation; the enclosing execution token adds complete closed request metadata and its owner-private single-send request/body handle. It has no generic consumer, sink, ref resolver, fallback or bytes-only adapter field. v1 runtime execution requires the record's operation confirmation for resolve to be `never`. `optional` or `required` returns `SECRET_CONFIRMATION_REQUIRED`, `effect=none` before network construction; background proxy/usage/balance/primary-coding-plan/model-fetch never opens an implicit hardware prompt.

Each async runtime method first borrows only the authority token's `binding_identity()` view, then moves the owned execution token into exactly one `spawn_blocking` closure. Inside it, #35 acquires `SecretMutationGate`, revalidates owner/store/record/binding/backend/device/capability revisions, exact-looks up the backend and calls object-safe `DeviceLocalSecretAuthority::authorize_runtime_read` with the same authority-minted binding. That returns the exact route wrapper—`AuthorizedProxyRead`, `AuthorizedUsageRead`, `AuthorizedCodingPlanRead` or `AuthorizedModelFetchRead`—which can call only its matching backend-sealed execution callback. The callback alone invokes the token's fixed `write_material_once`; no caller receives `&[u8]`. `SecretMaterial` is dropped/zeroized before the closure returns. The only join result is a dedicated private `Prepared*Request { metadata, requestHandle, authorizedSingleSend }`. The outer async method immediately calls `send_once(self).await` exactly once; that consuming owner-module API performs one transport await and drops request/authorization on success, error or cancellation. Prepared/execution types are non-cloneable/non-serializable/non-debuggable and expose no retry/header/material accessor; no fixed receipt/error can return bytes, `String`, headers or material-derived data.

All four credential-bearing HTTP metadata types contain the unforgeable `NoRedirectPolicy` literal. Their owner-private client factories must set `reqwest::ClientBuilder::redirect(reqwest::redirect::Policy::none())`; using a shared/default client, following `Location`, rebuilding the request, or copying an authorization header to another origin is forbidden. `UsageProbeKind` remains exactly `Usage|Balance`; the required closed coding-plan equivalent is the separate `CodingPlanPrimaryAdapter` behind `FixedRuntimeConsumer::CodingPlanUsageProbe`, so coding-plan traffic cannot take the generic usage binding. The regression fixture enumerates `proxyRequest`, both `UsageProbeKind` values, every fixed `CodingPlanPrimaryAdapter`, and `modelFetch` against a first endpoint returning each of `301/302/303/307/308` to a distinct second endpoint; every named case must record `result=pass`, the first response is terminal, the second endpoint receives zero requests, and no authorization bytes are retained for a follow-up. A case count or aggregate label is not evidence. There is no authenticated redirect, including same-origin redirect.

`SecretApplyPreparationView` is a serializable view owned by #41's job UI; the capability/pending object remains on the native worker stack. Implement it exactly as:

```rust
#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "status",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum SecretApplyPreparationView {
    Prepared {
        schema_version: SchemaVersionV1,
        operation_id: SecretOperationId,
        expires_at: UtcTimestamp,
    },
    ConfirmationRequired {
        schema_version: SchemaVersionV1,
        operation_id: SecretOperationId,
        step: SecretApplyHardwareConfirmStep,
    },
}
```

### 7.4 Atomic consume/revalidation invariant

`resolve_for_apply` performs this exact sequence:

1. require the owner-module-minted `SecretApplyCoordinatorContext`, call its lease-bound `assert_apply_final_baseline(plan, projection)`, then acquire the per-owner/ref `SecretMutationGate`; #35 holds no DB/Provider handle, never acquires a Provider lease and has no inverse lock path;
2. atomically CAS the capability registry from `prepared` to `revalidating`; missing/other state is `SECRET_CAPABILITY_CONSUMED`;
3. check expiry and #55 admission;
4. match the closed `SecretApplyWriterInvocation::Target|Rollback` variant and call `PreparedSecretCapabilityBundle::claim_role_for_revalidation`. The bundle first asks the registry to atomically claim the complete opaque capability while it remains in the exact role slot; only a successful claim moves that role `Prepared → Consumed` and returns `ClaimedPreparedSecretCapability { capability, claim }`. No caller borrows or submits `SecretCapabilityId`. Re-read its owner binding, binding revision, native `SecretStoreRevision`, record revision/lifecycle, complete binding-set CAS, backend instance/generation, device-store instance, registered-handle binding, device-binding generation and capability revision;
5. require lifecycle active, no policy/backend lock, no revocation, matching consumer/sink, exact `CodexLiveSecretSinkId == invocation.live_sink_id()`, and sink in the current capability set/final-baseline projection;
6. exact-lookup the backend and perform a fresh probe/authorization validation;
7. while still holding the mutation lease, CAS `revalidating` to terminal `consumed`;
8. pass the consumed capability through object-safe `DeviceLocalSecretAuthority::authorize_apply_read`, which consumes its sealed complete `BackendAuthorizationScope` and returns one `AuthorizedApplyRead`; if authorization/read fails the writer is never invoked and no material/getter crosses the backend boundary;
9. synchronously consume that object through `AuthorizedApplyRead::write_apply_once` with the role-matched backend-sealed `SecretApplyWriterInvocation` exactly once; only the two live-config writer adapters are legal, their closed receipt cannot contain material, and `SecretMaterial` is dropped/zeroized before returning and releasing the mutation gate.

Every mismatch in steps 3–6 terminally invalidates the capability and returns `effect=none`. Lock/rotate/delete/rebind therefore cannot race a prepared capability into a target. A backend/device loss after step 6 can still make step 8 fail, but it also occurs before target mutation.

Target and rollback have different capability ids and independent single-consume state. Resolving target never consumes rollback. A successful job calls `finish_apply` to invalidate unused rollback; a failed target writer may resolve rollback once after a fresh role-specific revalidation. If rollback is no longer valid, #41 records a typed recovery requirement and persists no material/capability.

Every confirmation terminal path is consuming. Apply confirmation atomically claims the pending registry row, consumes the backend pending session and either produces the next role or marks the row confirmed. Activation confirmation does the same in fixed order for mandatory candidate-read authorization, then optional projection-hashed old-record delete and old-record-missing authorizations. Recovery confirmation follows the kind-closed slot sequence: activation cleanup may require active-record equality, old-delete and old-missing; capture compensation may require uncommitted-delete then uncommitted-missing; delete finalization may require admitted-delete then admitted-missing; owner detach has no hardware slot. Staged import confirms only a future candidate-read authorization before the temp-DB cutover context exists; confirmation itself performs no read or validation. User cancel, expiry, job discard and shutdown call the matching `cancel_for_apply`, `cancel_candidate_activation`, `cancel_recovery` or `cancel_staged_import`; each consumes/cancels backend state, invalidates every already-prepared authorization, terminates any admission, and marks the pending registry row terminal. `discard_prepared`, `discard_prepared_activation`, `discard_prepared_recovery` and the staged equivalent perform the same terminal work after preparation. After successful terminal cleanup, `UserCancelled` reports `SECRET_CONFIRMATION_CANCELLED` and `Expired` reports `SECRET_CONFIRMATION_EXPIRED`; `Discarded` reports the original plan/job error. A `Drop` implementation may only emit a diagnostic assertion; it is not lifecycle cleanup. Replay after any terminal path is `SECRET_CONFIRMATION_REPLAYED`.

## 8. Exact #55 / #41 sequence

`8.1–8.3` describe a live apply for an already-bound owner. When a candidate must first become bound, `8.4` runs to terminal completion and releases its lease before a new apply preview starts; the two admissions/bundles are never combined.

### 8.1 Preview and approval (#55)

1. #55 asks `check_secret_apply_readiness(role=target, owner, consumer=changePlanApply, targetSink=externalConfigFile, liveSinkId=<closed id>)` and, when rollback/current credentials may be required, the corresponding `role=rollback` request for that owner and its exact sink id. The generic wire values are validating-decoded into role-specific strict projections; reserved consumer/sink values are typed rejects, and no path is accepted.
2. #35 returns operation-scoped `SecretApplyCredentialProjection` values. `ready` and `confirmationRequired` are both previewable; `confirmationRequired` contains only requirement/device/timeout, not a live step.
3. #55 constructs one `SecretApplyPlanProjection { target, rollback? }`, structurally redacts Provider definition/live projections, computes its exact projection digest, and hashes the full bundle into the immutable plan. It MUST NOT hash raw Provider/live material or any material-derived digest.
4. The user approves the #55 plan. The renderer keeps only plan id/digest and public readiness.

### 8.2 Prepare and optional confirm (#41, no Provider lease)

5. #41 asks #55 authority to admit `planId + planDigest + projection` once.
6. #41 passes the native admission and projection to `prepare_for_apply` before acquiring the Provider lease.
7. #35 prepares both target and optional rollback roles without material. If neither needs confirmation, it returns `PreparedSecretCapabilityBundle` plus public `status=prepared`.
8. If either role requires confirmation, #35 returns one native `PendingSecretConfirmation` plus a public `HardwareConfirmStep.role`. #41 renders/emits the step, then consumes it through `confirm_for_apply`. The result is either the next role's pending object or the completed bundle. Cancel/expiry/job discard MUST consume the object through `cancel_for_apply` so backend session, prepared roles, admission and pending registry all become terminal before reporting `SECRET_CONFIRMATION_CANCELLED` / `SECRET_CONFIRMATION_EXPIRED` / original job error. Replay maps to `SECRET_CONFIRMATION_REPLAYED`. All are `effect=none` and do not alter a binding or target.
9. No confirmation receipt/token or capability role is persisted in renderer/job/event/backup/diagnostic data; only the public step/status is visible.

### 8.3 Lease, baseline, backup, resolve (#41)

10. #41 acquires the Provider lease. Lease acquisition failure consumes the prepared bundle through `discard_prepared` before returning; it never leaves an admission/capability pending.
11. #55 rechecks its complete baseline and the exact target/rollback `SecretApplyPlanProjection`. A mismatch discards the bundle and returns plan-stale with `effect=none`.
12. #41 creates the structural recovery backup with typed credential placeholders only. Backup failure discards the bundle and returns `effect=none`.
13. #41 constructs the opaque lease/final-baseline `SecretApplyCoordinatorContext` and the target writer through its private factory bound to `projection.target.liveSinkId`, then calls `resolve_for_apply(&mut context, &mut bundle, SecretApplyWriterInvocation::Target(&mut targetWriter))` immediately before the writer's first target mutation.
14. #35 runs the target-role atomic sequence in `7.4`, reads material only after successful revalidation, and invokes the writer once.
15. On target write/readback failure, #41 may construct the rollback writer bound to `projection.rollback.liveSinkId` and call `resolve_for_apply(&mut context, &mut bundle, SecretApplyWriterInvocation::Rollback(&mut rollbackWriter))`. Rollback performs a fresh independent final-baseline and secret revalidation under the same held lease; it never obtains target material or reads material from the backup.
16. #41 owns target/rollback atomic write/readback/recovery classification. #35 returns role-tagged `SecretApplyResultDto` with stable writer status only; no target path, bytes, header, material fingerprint or raw error is stored.
17. #41 calls `finish_apply(bundle)`, releases the Provider lease and maps every pre-writer secret failure into its job as `effect=none`. No step retries with another backend or inline Provider settings.

Lock order is therefore: `#55 plan admission → optional target/rollback hardware confirmations → Provider lease → #55 baseline → structural backup → role-specific secret mutation lease → exact backend read → writer → release secret lease → optional rollback role → finish bundle → release Provider lease`. No inverse acquisition is allowed.

### 8.4 Candidate activation and legacy scrub (#41 coordinator)

Candidate activation does not write a live target, but it still mutates device-local binding authority and Provider legacy structure as one Provider-coordinated operation:

1. #41 obtains the single-use #55 candidate-activation admission and exact `SecretCandidateActivationProjection`. Candidate durable state, projection, projection digest, admission identity, journal intent and final baseline all carry the same `comparisonPolicy` and `comparisonImpact`; changing either is `SECRET_CHANGE_PLAN_STALE/effect=none`. Automatic `migrateLegacy` and `legacyScrubExistingBinding` plans require `candidateEquality`. A user-approved native capture from `sourcesConflict|bindingConflict`, or explicit `replace|reconcile|rotate`, requires `explicitReplacement` plus an approved replacement impact. Neither policy may be inferred at execution time.
2. Before a Provider lease, #41 calls `prepare_candidate_activation`. It always prepares a material-free candidate-record read authorization bound to the candidate/record/store/backend/device/capability revisions and exact policy. `deleteAfterActivation` independently prepares both the exact old-record delete and the fresh old-record-missing readback expectations; `notApplicable` fixes both slots to `NotApplicable`. Candidate validation, old deletion and old missing readback each have their own typed hardware slot and authorization. `PendingCandidateActivationConfirmation → confirm_candidate_activation` advances only that fixed order before the completed bundle. Cancel/expiry/job discard consumes through `cancel_candidate_activation`; lease-acquisition or final-baseline failure consumes through `discard_prepared_activation`. No platform prompt, pending mint or authorization reconstruction is legal after lease acquisition.
3. #41 acquires the Provider lease and privately constructs `SecretActivationCoordinatorContext { port }`; its one object-safe `ProviderLeaseBoundPort` is bound to that live transaction and exposes only the #55 final-baseline plus typed source-validation/scrub/readback operations. #35 has no constructor, DB field or lease-acquisition API.
4. #41 calls `activate_candidate_from_change_plan(context, preparedActivationBundle)`. Before `SecretMutationGate`, #35 consumes `context.assert_activation_final_baseline(admission, projection)`. Under the gate it fresh-revalidates candidate/store/record/binding/backend/device/capability revisions and obtains only `AuthorizedActivationRead` from the exact prepared scope.
5. `SecretActivationCoordinatorContext::validate_activation_sources` is policy-discriminated. For `candidateEquality`, `AuthorizedActivationRead::compare_candidate_equality_once` keeps the candidate material alive only inside the sealed callback while the lease-bound port resolves the complete admitted Provider/live occurrence set, validates exact locator/origin/category/structural revision, and constant-time-compares every current value to the candidate. For `explicitReplacement`, `AuthorizedActivationRead::verify_explicit_replacement_once` proves the candidate backend identity/readback, then the port validates the complete current occurrence set/revisions and the admitted replacement impact without requiring any old value to equal the candidate. Both policies reject missing, extra, retyped, relocated or revision-drifted occurrences with `SECRET_DEPENDENCY_CHANGED/effect=none`; only equality mode also treats value inequality as drift. The resulting closed `ProviderActivationSourceValidationReceipt::{CandidateEquality,ExplicitReplacement}` is consumed by binding commit and cannot be synthesized from text/projection.
6. #35 writes the exact tagged journal intent, performs the device-local binding CAS and candidate state transition, then—under the same Provider lease—calls `scrub_activation_and_readback(projection, &bindingCheckpoint)` for exactly the admitted current Provider/live occurrences. The checkpoint lineage and journal repeat the policy and source expectations, so the port cannot be invoked before validated commit. Only an opaque `ProviderScrubReadbackReceipt` creates `ProviderFinalizedActivationCheckpoint`; a scrub/readback failure records `activationCleanup` with `finalizeLegacyScrub` pending and returns the typed partial result.
7. Planned old-record deletion runs only from `ProviderFinalizedActivationCheckpoint`. `notApplicable` finalizes as such. `deleteAfterActivation` consumes the independently prepared pre-lease authorization, proves the exact old binding set is now `noBindings`, then calls one `AuthorizedBackendDelete::delete_once`. Delete/already-missing plus fresh missing readback yields `RotationSupersessionReceipt { source=supersededByRotation, revokedAt=backendCompletedAt }`; terminal old-record state persists that exact source/time. Any later failure records `activationCleanup/deleteOldRecord` and never rolls back the new binding.
8. `finalize_activation` or `record_activation_recovery` is the sole tagged result transition. #35 completes/consumes the admission and journal, releases `SecretMutationGate`, returns `SecretActivationResultDto`, then #41 releases its lease.
9. Only after activation is terminal and that lease is released may #55 preview a separate `codexProviderApply` plan. The live apply admission, projection, target/rollback bundle and lease are distinct and cannot accept an unbound candidate or reuse activation capability state.

A crash after binding CAS but before `providerFinalized` derives the unique `cleanupRequired` public mapping in `10`. Startup recovery and `retry_secret_cleanup` reacquire the Provider lease before finishing scrub. #35 never acquires that lease from inside `SecretMutationGate`, so the only lock order is `#41 Provider lease → #35 SecretMutationGate`.

### 8.5 General operation recovery (same two public recovery commands)

`get_secret_cleanup_impact` and `retry_secret_cleanup` are historical command names but carry the closed `SecretRecoveryKind`; they are the only public recovery entry points. There is no shadow command. Each durable row, impact, readiness registration, prepared bundle, result, journal variant and CAS preimage is exactly one of:

- `activationCleanup`: pending `finalizeLegacyScrub|deleteOldRecord|verifyOldRecordMissing`, #41 Provider lease required;
- `captureCompensation`: pending delete/readback/finalize of an uncommitted candidate record, local-only;
- `deleteFinalization`: admitted-record delete may still be prepared/pending or already applied; its independent missing readback and state/tombstone finalization remain local-only;
- `ownerDetachFinalization`: Provider detach committed and device-local owner-binding CAS pending, main-integration detach transaction context required.

`get_secret_cleanup_impact` always returns the kind-tagged impact plus `readiness=ready|confirmationRequired|blocked`; it never returns an issue with `SECRET_OPERATION_RECOVERY_REQUIRED/action=completeRecovery` inside that impact. A blocked readiness is limited to the fresh backend/lock/permission/device/dependency/record/capability conditions needed to prepare the immutable remaining step and carries their exact action from §11. `ready` and `confirmationRequired` are executable typed states, not error self-links.

The registered retry handler first claims the operation-scoped `SecretReadinessRegistry` row and calls `prepare_recovery(request)` before any Provider lease. It mints only slots permitted by the kind/current remaining suffix: activation cleanup independently prepares active-record equality, old-record delete and old-record-missing slots; capture compensation independently prepares uncommitted-record delete and uncommitted-record-missing slots; delete finalization independently prepares admitted-record delete and admitted-record-missing slots; owner detach has no backend material/hardware slot. `confirm_recovery` advances only the kind-specific fixed slot order. Each delete consumes its authorization and durably writes `backendApplied` plus a new delete-applied CAS before the distinct missing-readback authorization can be consumed; no callback combines delete and probe. For activation/activation-cleanup the missing readback is already the last step: its typed receipt and the supersession + terminal transition commit in one device-authority transaction, so no fourth public step and no nonterminal empty `remainingSteps` state exists. Cancel/expiry/replay call `cancel_recovery`; inability to obtain a later context calls `discard_prepared_recovery`. A completed `PreparedSecretRecoveryBundle` exposes only `kind()` and consuming `into_parts()` as `pub(in crate::secret)`; `service.rs` cannot read its private fields.

Dispatch is exact. `activationCleanup` asks #41 to acquire the Provider lease and privately mint `ActivationCleanupCoordinatorContext`; only then does `retry_recovery` acquire `SecretMutationGate`, recheck CAS and obtain a material-free `RecoveryProviderProjection`. A pending scrub step is admitted equality recovery: `AuthorizedRecoveryRead::compare_recovery_source_once` re-resolves exact current Provider/live sources/revisions under the lease, constant-time-compares against the active record, scrubs/readbacks and yields a typed receipt. This recovery never upgrades to `explicitReplacement`. `captureCompensation` and `deleteFinalization` consume only local contexts minted by `crate::secret::operation` after readiness claim and never touch Provider state. `ownerDetachFinalization` consumes an opaque `OwnerDetachCoordinatorContext` constructed only by `crate::commands::provider` while the already-started detach transaction/receipt is held; #35 neither obtains that transaction nor accepts a Provider snapshot/DB handle.

Every branch rechecks the exact kind-specific durable CAS before mutation, journals each completed step, and returns `SecretRecoveryResult { kind, result }`. A pre-step failure preserves CAS/effect; a progressed partial increments CAS and returns only the exact remaining nonempty set. `complete|alreadyComplete` requires all steps terminal. Startup reconciliation may automatically attempt confirmation-free work, but every nonterminal row remains discoverable through `get_secret_cleanup_impact` with the executable `completeRecovery` action; background retry is never the only exit. Hardware-required/optional recovery is never prompted in background. A user can re-enter the same public command, receive/confirm its typed step and finish it.

### 8.6 Staged import activation (main integration, not #41)

Staged SQL import, DB restore and sync download retain their exact staging origins. The only legal order is: (1) `crate::commands::import_export` opens the temp authority and atomically mints `StagedSecretOwnerToken` plus a material-free `StagedSecretImportActivationProjection`; (2) #55 consumes that projection into `AdmittedStagedSecretImportPlan`; (3) main integration obtains one consuming authority-match receipt; (4) #35 prepares and, if needed, confirms candidate-read authorization without invoking a read; (5) and only then main integration constructs `ImportCutoverCoordinatorContext` and calls activation. No later step may be moved ahead of an earlier step. Before the context exists, code may inspect only structural locators, typed source categories/revisions and source-set CAS; it MUST NOT read, parse, compare, validate, scrub or read back any staged source value, and it MUST NOT cut over. The projection references an independently secure-captured, backend-verified candidate; staging inspection never creates or migrates a candidate from a staged value. The token is non-cloneable/non-serde and cannot satisfy live readiness/apply/runtime APIs. The projection digest binds candidate/backend identities, exact structural staged source expectations/CAS, `comparisonPolicy`, replacement impact and cutover structural projection. `AdmittedStagedSecretImportPlan::identity()` is an opaque immutable view over plan/digest plus an import-owner-issued authority binding containing the durable object identity, fresh process nonce, stage, owner and row revision; neither #35 nor #55 can access or compare nonce bytes.

Before constructing an `ImportCutoverCoordinatorContext`, main integration calls its sealed `StagedImportAuthorityEqualityPort::assert_same_live_authority(stagedOwner.identity(), admitted.identity())`. That owner-only API uses process-object identity plus durable-id/stage/owner/row equality and returns one consuming `StagedImportAuthorityMatchReceipt`; no `[u8;16]`, string, path or caller equality is exposed. The operation broker consumes that receipt, readiness and durable-journal claim before `prepare_staged_import` can reach the registered backend. `prepare_staged_import` and `confirm_staged_import` prepare/confirm only a future candidate-read authorization; neither is allowed to read candidate material or any staged value. Cancel, expiry, replay and explicit discard atomically terminate both the prepared/pending backend state and the exact #55 staged admission; neither side may remain reusable. A baseline/cutover failure after preparation calls the same terminal discard path. No prompt is opened while temp cutover authority is held. Main integration then creates the opaque context from the same temp `Database` live object and admitted plan, and calls `activate_staged_import`; this context is the prerequisite for the first staged source value read/parse/compare/validate/scrub/readback and for cutover.

The context final-baseline checks stage/temp-live-object/owner/staged-row/source-set revisions and candidate/backend identity. `candidateEquality` consumes `AuthorizedStagedImportRead::compare_candidate_equality_once` and constant-time-compares every exact temp occurrence after re-resolve. `explicitReplacement` consumes `verify_explicit_replacement_once`, proves the candidate read, and validates the exact approved temp occurrence set/revisions/replacement impact without old-value equality. Missing/extra/retyped/relocated/revision drift is always `effect=none`; value inequality additionally fails equality mode. Only the typed validation receipt allows scrub/readback of those exact temp occurrences, followed by sanitized temp-DB cutover. After the opaque cutover receipt, Provider DAO mints a live `ExistingSecretOwnerToken`; #35 then finalizes the device-local binding CAS and tagged staged-import journal. Ordinary activation can scrub only current Provider/live sources; this staged variant can scrub only its exact temp sources. Neither may cross the boundary.

`StagedSecretImportActivationResultDto` is only the initial staged-activation result. Its `activated|alreadyActivated` arm requires a terminal journal and may return the original candidate/owner summary; its recovery arm returns `currentResumeCas` and `action=resumeStagedImportCutover`. The independent resume handler returns only `ResumeStagedImportCutoverResultDto`. Every resume result data arm has exactly five fields: `{stageId,currentResumeCas,status,action,issue}`. Terminal `activated|alreadyActivated` requires `action=none, issue=null`; `cutoverRecoveryRequired` requires the checked typed `issue` and `action=resumeStagedImportCutover`. Version/command id stay in the common envelope and audit is recorded separately, so schema version, audit event id, candidate id, owner, ref and every summary type are structurally absent from all resume data arms. `SECRET_ACTION_DESTINATIONS_V1` maps the recovery action to the non-#35 main-integration handler `resume_staged_import_cutover`.

Public resume accepts only `{stageId,expectedResumeCas:{revision,digest}}`; it has no request `schemaVersion` field. The handler atomically compares that pair to the current journal CAS before reopening authority. Its digest preimage is internal-only and includes the immutable common journal operation id plus stage/source/candidate/durable-object/process/admission/record/backend identity and one exact cumulative `intent|sourcesScrubbed|cutoverCommitted|liveOwnerMinted|localBindingFinalized` phase arm. `intent` forbids scrub/cutover/promoted-owner receipts; each later arm requires every receipt of its completed predecessors, and only the last two require the promoted live owner. None of those fields is accepted from the renderer or emitted in the resume result. Every phase/nonce/admission/source-CAS/receipt/promoted-owner change increments the revision before recomputing the digest; an operation-id mismatch is a different journal and rejects. On a match, the immutable order repeats: reopen the durable temp object, terminally reconcile the prior admission, mint a fresh process-live token and projection, obtain a new #55 `AdmittedStagedSecretImportPlan`, obtain a new authority-match receipt, prepare/confirm #35, then construct a fresh `ImportCutoverCoordinatorContext`. On a stale/replayed CAS, the handler performs zero backend, Provider, admission or journal writes and returns the recovery arm with the freshly read `currentResumeCas` and checked stale issue. On terminal replay it returns `alreadyActivated` with the freshly read terminal CAS. Neither an old process nonce nor old pending/auth id is reusable. Startup may attempt confirmation-free reconciliation, but the exact handler remains the UAT-visible destination until terminal. This staged recovery is not one of the two #35 cleanup-named commands and does not add a 16th secret command.

## 9. Public command envelope and signatures

The registered renderer-safe commands are exactly these 15:

```rust
async fn list_secret_summaries(
    request: ListSecretSummariesRequest,
) -> SecretCommandResult<ListSecretSummariesResult>;

async fn list_secret_backend_options(
    request: ListSecretBackendOptionsRequest,
) -> SecretCommandResult<ListSecretBackendOptionsResult>;

// Stages; never binds.
async fn begin_secret_capture(
    request: BeginSecretCaptureRequest,
) -> SecretCommandResult<StageSecretCandidateResult>;

// Stages one candidate for the complete confirmed binding set; never switches.
async fn rotate_secret(
    request: RotateSecretRequest,
) -> SecretCommandResult<StageSecretCandidateResult>;

async fn list_secret_candidates(
    request: ListSecretCandidatesRequest,
) -> SecretCommandResult<ListSecretCandidatesResult>;

async fn discard_secret_candidate(
    request: DiscardSecretCandidateRequest,
) -> SecretCommandResult<DiscardSecretCandidateResult>;

// Logical FyAgent policy lock only.
async fn set_secret_locked(
    request: SetSecretLockedRequest,
) -> SecretCommandResult<SecretMutationResult>;

async fn get_secret_delete_impact(
    request: GetSecretDeleteImpactRequest,
) -> SecretCommandResult<SecretDeleteImpact>;

async fn delete_secret(
    request: DeleteSecretRequest,
) -> SecretCommandResult<SecretDeleteResult>;

// Returns impact plus operation-scoped ready/confirmationRequired/blocked.
async fn get_secret_cleanup_impact(
    request: GetSecretCleanupImpactRequest,
) -> SecretCommandResult<SecretRecoveryImpact>;

// Retries only the immutable recovery row's remaining exact steps.
async fn retry_secret_cleanup(
    request: RetrySecretCleanupRequest,
) -> SecretCommandResult<SecretRecoveryResult>;

async fn validate_secret(
    request: ValidateSecretRequest,
) -> SecretCommandResult<SecretValidationResult>;

async fn check_secret_apply_readiness(
    request: CheckSecretApplyReadinessRequest,
) -> SecretCommandResult<SecretApplyReadiness>;

async fn migrate_legacy_codex_secrets(
    request: MigrateLegacyCodexSecretsRequest,
) -> SecretCommandResult<SecretMigrationReport>;

async fn list_secret_audit(
    request: ListSecretAuditRequest,
) -> SecretCommandResult<SecretAuditPage>;
```

The renderer body signatures above intentionally omit Tauri's injected
`State<AppState>`. The private handler adapters for
`list_secret_summaries`, `list_secret_backend_options` and
`begin_secret_capture` each construct one
`CodexLegacySourceInventoryBridge::from_app_state(state)` and pass a mutable
borrow to the matching `SecretService` method. Summary obtains a distinct
fresh receipt for every emitted owner; list-options obtains the receipt stored
in its new capture intent; begin claims that intent first and then obtains a
second fresh receipt for exact revalidation. The handler cannot accept a
receipt, identity, revision, source set or bridge from the renderer.

The following separately registered main-integration handler is the exact
destination of `resumeStagedImportCutover`; it is not a #35 command and is not
included in the 15-name `SecretCommandName` union:

```rust
async fn resume_staged_import_cutover(
    request: ResumeStagedImportCutoverRequest,
) -> MainIntegrationCommandResult<ResumeStagedImportCutoverResultDto>;
```

`activate_candidate_from_change_plan`, staged import activation, all apply/activation/staged `prepare|confirm|cancel|discard` calls, candidate-discard `prepare_candidate_discard|confirm_candidate_discard|cancel_candidate_discard|discard_prepared_candidate_discard|execute_candidate_discard`, and `prepare_recovery|confirm_recovery|cancel_recovery|discard_prepared_recovery|SecretService::retry_recovery(bundle)` are native-only calls and MUST NOT be registered as additional Tauri commands. The existing `discard_secret_candidate` handler alone drives the closed candidate-discard calls and returns only `DiscardSecretCandidateResult`. `resume_staged_import_cutover` is owned by `crate::commands::import_export`, accepts only its exact closed resume CAS and cannot call a generic secret resolver. The sole #35 recovery pair remains `get_secret_cleanup_impact` / `retry_secret_cleanup`; their historical command names do not narrow the kind-discriminated contract. Renderer activation occurs through #55's plan/apply commands, and staged activation through main integration's existing import flow. There is no `get/read/reveal/copy/export secret` command and no `set_secret(value)` command.

The Tauri registration wrapper generates `commandId` before decoding the inner request. Its closed-shape/type/schema stage maps failures to `SECRET_REQUEST_INVALID` without echoing input; the subsequent typed scalar stage maps only a string-valued known `secretRef` grammar failure to `SECRET_REF_INVALID`. This is an explicit decoder error variant, never serde error-text inspection. Therefore malformed transport still receives the same envelope while `REF_INVALID` remains uniquely reachable.

### 9.1 Allowed command errors

Codes not listed for a command are implementation bugs and MUST be converted to `SECRET_INTERNAL`, never leaked as a new string.

| Command | Success DTO | Allowed non-internal errors |
| --- | --- | --- |
| `list_secret_summaries` | `ListSecretSummariesResult` | `REQUEST_INVALID, REF_INVALID, OWNER_KIND_UNSUPPORTED, OWNER_NAMESPACE_UNSUPPORTED` |
| `list_secret_backend_options` | `ListSecretBackendOptionsResult` | `REQUEST_INVALID, OWNER_KIND_UNSUPPORTED, OWNER_NAMESPACE_UNSUPPORTED, OWNER_NOT_FOUND, OWNER_CONFLICT, UNSUPPORTED_PURPOSE, LEGACY_SOURCE_INVALID, LEGACY_CONFLICT, LEGACY_COMPARISON_PENDING` |
| `begin_secret_capture` | `StageSecretCandidateResult` | `REQUEST_INVALID, OWNER_NOT_FOUND, OWNER_CONFLICT, OPERATION_BUSY, INPUT_CANCELLED, INPUT_INVALID, CONFIRMATION_CANCELLED, CONFIRMATION_EXPIRED, CONFIRMATION_REPLAYED, LOCKED, PERMISSION_DENIED, BACKEND_UNAVAILABLE, DEVICE_MISMATCH, WRITE_FAILED, READ_FAILED, VERIFY_FAILED, DEPENDENCY_CHANGED, BACKEND_CHANGED, OPERATION_RECOVERY_REQUIRED` |
| `rotate_secret` | `StageSecretCandidateResult` | `REQUEST_INVALID, REF_INVALID, OWNER_KIND_UNSUPPORTED, OWNER_NAMESPACE_UNSUPPORTED, OWNER_NOT_FOUND, OWNER_CONFLICT, OPERATION_BUSY, UNSUPPORTED_PURPOSE, INPUT_CANCELLED, INPUT_INVALID, MISSING, STALE, REVOKED, CONFIRMATION_CANCELLED, CONFIRMATION_EXPIRED, CONFIRMATION_REPLAYED, LOCKED, PERMISSION_DENIED, BACKEND_UNAVAILABLE, DEVICE_MISMATCH, WRITE_FAILED, READ_FAILED, VERIFY_FAILED, DEPENDENCY_CHANGED, RECORD_CHANGED, BACKEND_CHANGED, OPERATION_RECOVERY_REQUIRED` |
| `list_secret_candidates` | `ListSecretCandidatesResult` | `REQUEST_INVALID, OWNER_KIND_UNSUPPORTED, OWNER_NAMESPACE_UNSUPPORTED` |
| `discard_secret_candidate` | `DiscardSecretCandidateResult` | `REQUEST_INVALID, CANDIDATE_NOT_FOUND, CANDIDATE_CONSUMED, DEPENDENCY_CHANGED, RECORD_CHANGED, BACKEND_CHANGED, CONFIRMATION_CANCELLED, CONFIRMATION_EXPIRED, CONFIRMATION_REPLAYED, DEVICE_MISMATCH, LOCKED, PERMISSION_DENIED, BACKEND_UNAVAILABLE, DELETE_FAILED, READ_FAILED, OPERATION_RECOVERY_REQUIRED` |
| `set_secret_locked` | `SecretMutationResult` | `REQUEST_INVALID, REF_INVALID, MISSING, REVOKED, DEPENDENCY_CHANGED, RECORD_CHANGED` |
| `get_secret_delete_impact` | `SecretDeleteImpact` | `REQUEST_INVALID, REF_INVALID, MISSING` |
| `delete_secret` | `SecretDeleteResult` | `REQUEST_INVALID, REF_INVALID, DEPENDENCY_CHANGED, RECORD_CHANGED, CONFIRMATION_CANCELLED, CONFIRMATION_EXPIRED, CONFIRMATION_REPLAYED, DEVICE_MISMATCH, LOCKED, PERMISSION_DENIED, BACKEND_UNAVAILABLE, DELETE_FAILED, OPERATION_RECOVERY_REQUIRED` |
| `get_secret_cleanup_impact` | `SecretRecoveryImpact` | command-level only `REQUEST_INVALID, RECOVERY_NOT_FOUND, OPERATION_BUSY`; backend/lock/confirmation conditions are typed `impact.readiness`, never `OPERATION_RECOVERY_REQUIRED` recursion |
| `retry_secret_cleanup` | `SecretRecoveryResult` | `REQUEST_INVALID, RECOVERY_NOT_FOUND, RECOVERY_CHANGED, OPERATION_BUSY, DEPENDENCY_CHANGED, RECORD_CHANGED, BACKEND_CHANGED, CONFIRMATION_CANCELLED, CONFIRMATION_EXPIRED, CONFIRMATION_REPLAYED, DEVICE_MISMATCH, LOCKED, PERMISSION_DENIED, BACKEND_UNAVAILABLE, READ_FAILED, DELETE_FAILED, OPERATION_RECOVERY_REQUIRED` |
| `validate_secret` | `SecretValidationResult` | `REQUEST_INVALID, REF_INVALID, RECORD_CHANGED, LOCKED, PERMISSION_DENIED, BACKEND_UNAVAILABLE, CONFIRMATION_CANCELLED, CONFIRMATION_EXPIRED, CONFIRMATION_REPLAYED, DEVICE_MISMATCH, READ_FAILED`; exact `MISSING/REVOKED` are successful `blocked/missing` result states |
| `check_secret_apply_readiness` | `SecretApplyReadiness` | command-level only `REQUEST_INVALID, OWNER_KIND_UNSUPPORTED, OWNER_NAMESPACE_UNSUPPORTED, OWNER_NOT_FOUND, UNSUPPORTED_PURPOSE`; all secret conditions are `status=blocked` inside success |
| `migrate_legacy_codex_secrets` | `SecretMigrationReport` | command-level only `REQUEST_INVALID, OWNER_KIND_UNSUPPORTED, OWNER_NAMESPACE_UNSUPPORTED`; per-owner errors are report rows |
| `list_secret_audit` | `SecretAuditPage` | `REQUEST_INVALID, REF_INVALID, OWNER_KIND_UNSUPPORTED, OWNER_NAMESPACE_UNSUPPORTED` |

The `SECRET_` prefix is shown once in the table for readability; every wire value uses the complete literal from `SecretErrorCode`.

For `discard_secret_candidate`, `delete_secret` and `retry_secret_cleanup`, `SECRET_LOCKED` is exclusively a fresh backend/store lock and carries `lockSource=backend`, `effect=none`. A FyAgent policy lock is not substituted for it, and the branch records no discard/delete/cleanup progress.

`get_secret_delete_impact` creates a fresh operation id and calls `SecretReadinessRegistry::mint(Delete{...})` with the exact ref/record/store/binding-set/backend/device/capability identity and expiry; only the operation id enters `SecretDeleteReadinessContext`. `delete_secret` uses that text solely for `claim_once`, passing the complete request/current identity, then consumes or terminally terminates the opaque claim. `get_secret_cleanup_impact` likewise mints `Recovery{kind,recoveryId,recoveryCas,pendingSteps}`; `retry_secret_cleanup` must atomically claim the exact row before `prepare_recovery`, may perform only those steps, and returns a kind-tagged remaining set if still partial. A second/missing/expired/claimed/consumed operation id is replay-safe and cannot reach backend preparation. The registered handler completes `prepare_recovery/confirm_recovery` before selecting a context. Only `activationCleanup` asks #41 to acquire a Provider lease and construct `ActivationCleanupCoordinatorContext`; `captureCompensation` and `deleteFinalization` use local operation contexts; `ownerDetachFinalization` receives main integration's already-held `OwnerDetachCoordinatorContext`. It then consumes the bundle through `SecretService::retry_recovery`. Every completion consumes readiness; cancel/expiry/context/baseline failure terminates readiness and discards the bundle. `SecretService` never acquires a Provider lease or DB itself. For hardware `confirmationRequired`, delete/recovery opens and consumes native device confirmation before any Provider lease/backend mutation; cancel/expiry is structured and the current binding remains unchanged. Rotate gets its impact/CAS from the selected `SecretRefAggregate` and does not switch anything until the later Change Plan. Lock is a FyAgent policy mutation and never asks a backend to claim that the OS/device was unlocked.

Candidate discard deliberately remains one command, so the public command count stays 15. Each invocation generates a new native operation id, re-reads the exact candidate revision plus candidate-record store/backend/device/capability generations, and constructs exactly two operation-specific preparation slots: `RecordDelete` with a one-shot `Delete` authorization and `RecordMissingReadback` with a distinct one-shot `Validate` authorization. Hardware confirmation for both may complete before mutation, but the missing slot carries a private `BackendDeleteAppliedCasReservation` and cannot execute until the exact operation's durable `backendApplied{deleteDisposition,backendCompletedAt,deleteAppliedCas}` checkpoint fulfills it. Cancel/expiry/replay terminally consumes the operation/pending state and returns the corresponding typed error. Permission/device/backend/generation drift returns the exact allowed row error with `effect=none`. For a new explicit intent, `terminalDisposition=discarded`; for an existing expiry-sweep intent it remains `expired`. Delete/already-missing advances the three-field `backendApplied`; only the subsequent fresh missing readback advances `missingReadbackVerified` with that same triple plus `missingCheckedAt`, after which one atomic transition removes the unbound record, writes the immutable candidate/audit state and persists `terminal`. Delete failure maps to `SECRET_DELETE_FAILED`; fresh-readback failure maps to the newly reachable existing `SECRET_READ_FAILED` row, without adding an error literal. Every failure leaves `verifiedPendingPlan` with its checked immutable pending disposition and reachable issue; `action=discardCandidate` starts a fresh invocation without rewriting that disposition and never replays the terminated operation id. No candidate-discard `stateFinalized`, discard readiness, partial success or activation-cleanup recovery row is fabricated.

## 10. Stable state derivation

The service derives owner state and ref state independently:

| Authority facts | Owner binding state | Ref aggregate |
| --- | --- | --- |
| no binding, no known inline value | `unbound` | none |
| no binding, one/equal inline value | `legacy/singleValuePending` | none until candidate verified |
| no binding, differing inline sources | `legacy/sourcesConflict` | none |
| any malformed/non-string/duplicate/incompletely staged source | `legacy/sourceInvalid` | no new ref/candidate; existing bound aggregate unchanged |
| binding exists, inline comparison blocked by lock/denial/unavailable | `legacy/bindingComparisonPending` | normal aggregate for bound ref |
| binding exists, inline differs after constant-time verify | `legacy/bindingConflict` | normal aggregate for bound ref |
| binding exists, every inline source verified equal | `legacy/approvalRequired` until a scrub plan is approved | normal aggregate |
| active record + probe present | `bound` | `present/ready` |
| policy lock | `bound` | `unknown/locked, lockSource=fyAgentPolicy` without backend access |
| backend lock | `bound` | `unknown/locked, lockSource=backend` |
| explicit permission denial | `bound` | `unknown/denied` |
| exact entry absence with no known revocation | `bound` | `missing/missing` |
| activation binding CAS durable but `providerFinalized` or planned old-record delete not durable | `bound` to the new ref | last observed presence/`stale` with `issue.code=SECRET_OPERATION_RECOVERY_REQUIRED`; involved candidate `cleanupRequired` |
| user delete | `bound` retained for impact | last presence/`revoked, source=userDelete` |
| central/device revocation | `bound` retained | last presence/`revoked` with exact source/time |
| exact backend instance unavailable/device mismatch | `bound` | `unknown/unavailable` |

`cleanupRequired` has exactly one public derivation. It is true iff an `activationCleanup` recovery proves binding CAS committed while `finalizeLegacyScrub|deleteOldRecord|verifyOldRecordMissing` remains non-terminal. Every affected owner remains `bound` to the committed new ref; that ref is `stale` with `issue={code: SECRET_OPERATION_RECOVERY_REQUIRED,retryable:true,action:completeRecovery,recovery:{recoveryId,kind:activationCleanup,recoveryCas}}`; the candidate is `cleanupRequired`; and readiness for `changePlanApply/proxyRequest/usageProbe/codingPlanUsageProbe/modelFetch` is blocked before capability/read/network/writer action. Other recovery kinds expose their own pointer/action but never set candidate `cleanupRequired`. `get_secret_cleanup_impact` and `retry_secret_cleanup` must match kind/CAS. Supersession and cleanup terminal state are minted only after the fresh missing receipt. Only a durable terminal transition restores activation readiness/candidate `activated`.

`lastValidatedAt` changes no security revision. Lock, revoke, backend/capability/device generation, locator, store revision or lifecycle changes increment `recordRevision`. A device-local owner-binding authority row exists even while unbound; every bind/unbind/rebind increments the distinct `SecretOwnerBindingRevision`, preventing unbound ABA. Every bound-row mutation also increments its `SecretBindingRevision` and each affected ref's `bindingSetCas.revision`.

## 11. Complete stable error/action/audit matrix

`Summary projection` describes the state returned on the next summary read. `unchanged` means the failed operation cannot alter stable state. `Audit=none` is allowed only when wire parsing failed before a valid operation identity existed; all other rows append one material-free event.

The closed TypeScript union, Rust enum and matrix below contain exactly the same 47 **distinct** error-code literals and 24 `SecretUserAction` literals; discriminator-specific rows repeat a code/action without adding a literal. `SECRET_LOCKED` requires exactly one lock source, `SECRET_BACKEND_UNAVAILABLE` exactly one unavailable reason, `SECRET_REVOKED` exactly one revocation source whose persisted `SecretRevocationView` supplies the observation time, and `SECRET_OPERATION_RECOVERY_REQUIRED` exactly one recovery kind/pointer except the candidate-terminal-cleanup exception. Where one stable code has context-specific remediation, the private checked factory consumes a closed context that already identifies capture intent, exact runtime entry, delete/recovery readiness, admitted plan, staged resume, validation, rotation or discard; it is never serialized. Every valid `(code, closedContext, source/reason/kind)` row has exactly one `SecretUserAction`, and `SECRET_ACTION_DESTINATIONS_V1` / Rust `secret_action_destination` maps that action to one fresh command flow, native continuation, main-integration handler, or explicit external guidance. Any omitted/extra/incompatible discriminator is rejected by the sole private checked factory; external `SecretInternalError` literals are scanner-forbidden.

The TypeScript error/issue decoder accepts only the closed `SecretUserAction` literal union and immediately indexes `SECRET_ACTION_DESTINATIONS_V1`; an unknown action or missing destination is a response-contract failure. There is no generic `retry` action or generic fresh-invocation destination. UI reducers switch exhaustively on `destination.kind` and, for `secretCommand|freshSecretCommand`, on the exact 15-name `SecretCommandName`. Rust's match has no wildcard. The static parity gate extracts the matrix action column and requires it to be a subset of both unions, then requires every union member to have exactly one destination row. `retryCapture|captureReplacement|chooseBackend|resolveLegacyConflict` enter the closed capture flow with a newly minted capture intent; each runtime retry enters only its named fixed executor; each `freshSecretCommand`, command flow and main-integration resume declares `serverGeneratedNew`. After `unlockBackend`, `requestPermission`, `reconnectDevice`, `openBackendSettings` or `contactAdministrator`, the mapped `refreshSummary` step is mandatory, and any continued delete/recovery/apply/capture/rotate/discard/staged flow must then obtain its own new impact/readiness/admission before preparation. External guidance never revives pending state.

The checked factory's context route is exact and is also used for `SECRET_OPERATION_BUSY`, `SECRET_INTERNAL` and other terminal failures:

| Closed context | Exact action |
| --- | --- |
| `summary` | `refreshSummary` |
| `capture(newBinding)` | `retryCapture` |
| `capture(replaceBinding)` | `captureReplacement` |
| `capture(legacyReconcile)` | `resolveLegacyConflict` |
| `rotation` | `retryRotation` |
| `candidateDiscard` | `discardCandidate` |
| `candidateTerminalCleanupPending` | `discardCandidate` |
| `delete` | `refreshDeleteImpact` |
| `recovery` | `refreshRecoveryImpact` |
| `applyOrActivation` | `reopenChangePlan` |
| `stagedImport` | `resumeStagedImportCutover` |
| `validation` | `refreshSummary` |
| `runtime(proxyRequest)` | `retryProxyRequest` |
| `runtime(usageProbe)` | `retryUsageProbe` |
| `runtime(codingPlanUsageProbe)` | `retryCodingPlanUsageProbe` |
| `runtime(modelFetch)` | `retryModelFetch` |

| Error code | Summary projection | Retryable | User action | Audit outcome | Effect |
| --- | --- | ---: | --- | --- | --- |
| `SECRET_REQUEST_INVALID` | unchanged / no subject | no | `none` | none | `none` |
| `SECRET_REF_INVALID` | unchanged / no valid ref | no | `refreshSummary` | none | `none` |
| `SECRET_OWNER_KIND_UNSUPPORTED` | owner unchanged | no | `none` | `blocked` | `none` |
| `SECRET_OWNER_NAMESPACE_UNSUPPORTED` | owner unchanged | no | `none` | `blocked` | `none` |
| `SECRET_OWNER_NOT_FOUND` | no owner summary | no | `refreshSummary` | `blocked` | `none` |
| `SECRET_OWNER_CONFLICT` | current owner/binding unchanged | yes | `refreshSummary` | `blocked` | `none` |
| `SECRET_OPERATION_BUSY` + exact closed context | unchanged | yes | exact action in the closed-context table | `blocked` | `none` |
| `SECRET_UNSUPPORTED_PURPOSE` | unchanged | no | `none` | `blocked` | `none` |
| `SECRET_CONSUMER_UNSUPPORTED` | unchanged; readiness `blocked` | no | `none` | `blocked` | `none` |
| `SECRET_INPUT_CANCELLED` + exact capture intent | no candidate/record/binding | yes | exact capture action in the closed-context table | `blocked` | `none` |
| `SECRET_INPUT_INVALID` + exact capture intent | no candidate/record/binding | yes | exact capture action in the closed-context table | `blocked` | `none` |
| `SECRET_INPUT_INVALID` + `condition=general` | rejected native value/checked construction; no subject mutation | yes | `refreshSummary` | `blocked` | `none` |
| `SECRET_CANDIDATE_NOT_FOUND` | bindings unchanged | no | `refreshSummary` | `blocked` | `none` |
| `SECRET_CANDIDATE_EXPIRED` | candidate is durably terminal `expired`; no backend cleanup remains | no | `refreshSummary` | `blocked` | `none` |
| `SECRET_CANDIDATE_CONSUMED` | terminal candidate/binding readback | no | `refreshSummary` | `blocked` | `none` |
| `SECRET_CHANGE_PLAN_REQUIRED` | candidate remains pending | yes | `reopenChangePlan` | `blocked` | `none` |
| `SECRET_CHANGE_PLAN_INVALID` | candidate remains pending | no | `reopenChangePlan` | `blocked` | `none` |
| `SECRET_CHANGE_PLAN_STALE` | current summary; candidate remains/discardable | yes | `reopenChangePlan` | `blocked` | `none` |
| `SECRET_MIGRATION_REQUIRED` | owner `legacy`; public Provider remains scrubbed | yes | `resolveLegacyConflict` | `blocked` | `none` |
| `SECRET_LEGACY_SOURCE_INVALID` | owner `legacy/sourceInvalid`; every source retained | no | `resolveLegacyConflict` | `blocked` | `none` |
| `SECRET_LEGACY_CONFLICT` | owner `legacy/sourcesConflict` or `bindingConflict` | no | `resolveLegacyConflict` | `blocked` | `none` |
| `SECRET_LEGACY_COMPARISON_PENDING` | owner `legacy/bindingComparisonPending` | yes | `refreshSummary` | `blocked` | `none` |
| `SECRET_MIGRATION_FAILED` | owner remains legacy; internal plaintext retained | yes | `resolveLegacyConflict` | `failed` | `none` |
| `SECRET_MISSING` | `presence=missing, availability=missing` | no | `captureReplacement` | `blocked` | `none` |
| `SECRET_LOCKED` + `lockSource=fyAgentPolicy` | `presence=unknown, availability=locked` | yes | `unlockFyAgent` | `blocked` | `none` |
| `SECRET_LOCKED` + `lockSource=backend` | `presence=unknown, availability=locked` | yes | `unlockBackend` | `blocked` | `none` |
| `SECRET_PERMISSION_DENIED` | `unknown/denied` | yes | `requestPermission` | `blocked` | `none` |
| `SECRET_BACKEND_UNAVAILABLE` + `hardwareUnregistered` + `capture(newBinding)` | capture has not written; selected instance unavailable | yes | `chooseBackend` | `blocked` | `none` |
| same code/reason + `capture(replaceBinding)` | replacement has not written; selected instance unavailable | yes | `captureReplacement` | `blocked` | `none` |
| same code/reason + `capture(legacyReconcile)` | reconcile has not written; selected instance unavailable | yes | `resolveLegacyConflict` | `blocked` | `none` |
| same code/reason + `rotation` | rotation has not written; selected instance unavailable | yes | `retryRotation` | `blocked` | `none` |
| `SECRET_BACKEND_UNAVAILABLE` + `hardwareUnregistered` + `condition=general` | bound instance remains `unknown/unavailable` | no | `openBackendSettings` | `blocked` | `none` |
| `SECRET_BACKEND_UNAVAILABLE` + `hardwareDisconnected` | `unknown/unavailable` | yes | `reconnectDevice` | `blocked` | `none` |
| `SECRET_BACKEND_UNAVAILABLE` + `osStoreUnavailable` | `unknown/unavailable` | yes | `openBackendSettings` | `blocked` | `none` |
| `SECRET_BACKEND_UNAVAILABLE` + `centralServiceUnavailable` | `unknown/unavailable` | no | `contactAdministrator` | `blocked` | `none` |
| `SECRET_STALE` | observed presence / `stale`; no recovery pointer | yes | `refreshSummary` | `blocked` | `none` |
| `SECRET_REVOKED` + `userDelete` | last presence / `revoked` | no | `captureReplacement` | `blocked` | `none` |
| `SECRET_REVOKED` + `supersededByRotation` | terminal old record; never active again | no | `none` | `blocked` | `none` |
| `SECRET_REVOKED` + `centralBackend` | last presence / `revoked` with observation time | no | `contactAdministrator` | `blocked` | `none` |
| `SECRET_REVOKED` + `deviceAdministration` | last presence / `revoked` with observation time | no | `openBackendSettings` | `blocked` | `none` |
| `SECRET_CONFIRMATION_REQUIRED` | stable summary unchanged; operation readiness only | yes | `confirmDevice` | `blocked` | `none` |
| `SECRET_CONFIRMATION_CANCELLED` / `SECRET_CONFIRMATION_EXPIRED` / `SECRET_CONFIRMATION_REPLAYED` + exact capture intent | capture pending terminal; no candidate write | yes | exact capture action in the closed-context table | `blocked` | `none` |
| same three confirmation codes + `condition=rotationFreshOperation` | rotation pending terminal; old binding unchanged | yes | `retryRotation` | `blocked` | `none` |
| same three confirmation codes + `condition=candidateDiscardFreshOperation` | discard pending terminal; journal disposition unchanged | yes | `discardCandidate` | `blocked` | `none` |
| same three confirmation codes + `condition=deleteReadiness` | delete operation id terminal; binding unchanged | yes | `refreshDeleteImpact` | `blocked` | `none` |
| same three confirmation codes + `condition=recoveryReadiness` | recovery operation id terminal; durable recovery unchanged | yes | `refreshRecoveryImpact` | `blocked` | `none` |
| same three confirmation codes + `condition=applyOrActivationPlan` | prepared apply/activation admission terminal | yes | `reopenChangePlan` | `blocked` | `none` |
| same three confirmation codes + `condition=stagedImportResume` | staged pending/admission terminal; exact five-arm phase/CAS unchanged | yes | `resumeStagedImportCutover` | `blocked` | `none` |
| same three confirmation codes + `condition=validationFreshOperation` | validation operation terminal | yes | `refreshSummary` | `blocked` | `none` |
| `SECRET_DEVICE_MISMATCH` | `unknown/unavailable` | no | `reconnectDevice` | `blocked` | `none` |
| `SECRET_WRITE_FAILED` / `SECRET_VERIFY_FAILED` + exact capture intent | no candidate/binding; failed record compensated or recovery journaled | yes | exact capture action in the closed-context table | `failed` | `none` |
| same write/verify codes + `condition=rotationFreshOperation` | old binding unchanged; failed new record compensated or recovery journaled | yes | `retryRotation` | `failed` | `none` |
| `SECRET_READ_FAILED` + exact capture intent | no candidate/binding; capture readback operation terminal | yes | exact capture action in the closed-context table | `failed` | `none` |
| `SECRET_READ_FAILED` + `condition=rotationFreshOperation` | old binding unchanged; rotation readback operation terminal | yes | `retryRotation` | `failed` | `none` |
| `SECRET_READ_FAILED` + `condition=candidateDiscardFreshOperation` | candidate journal remains nonterminal with the same immutable disposition | yes | `discardCandidate` | `failed` | `none` |
| `SECRET_READ_FAILED` + `condition=deleteReadiness` | delete/readback operation id terminal; binding state not guessed | yes | `refreshDeleteImpact` | `failed` | `none` |
| `SECRET_READ_FAILED` + `runtime(proxyRequest)` | no network construction or target mutation | yes | `retryProxyRequest` | `failed` | `none` |
| `SECRET_READ_FAILED` + `runtime(usageProbe)` | no network construction or target mutation | yes | `retryUsageProbe` | `failed` | `none` |
| `SECRET_READ_FAILED` + `runtime(codingPlanUsageProbe)` | no network construction or target mutation | yes | `retryCodingPlanUsageProbe` | `failed` | `none` |
| `SECRET_READ_FAILED` + `runtime(modelFetch)` | no network construction or target mutation | yes | `retryModelFetch` | `failed` | `none` |
| `SECRET_READ_FAILED` + `condition=validationFreshOperation` | probe-derived summary or `unknown/unavailable` | yes | `refreshSummary` | `failed` | `none` |
| `SECRET_READ_FAILED` + `condition=applyOrActivationPlan` | no writer/binding mutation | yes | `reopenChangePlan` | `failed` | `none` |
| `SECRET_READ_FAILED` + `condition=recoveryReadiness` | durable recovery unchanged | yes | `refreshRecoveryImpact` | `failed` | `none` |
| `SECRET_READ_FAILED` + `condition=stagedImportResume` | staged five-arm phase/CAS unchanged | yes | `resumeStagedImportCutover` | `failed` | `none` |
| `SECRET_DELETE_FAILED` + `condition=candidateDiscardFreshOperation` | candidate journal remains nonterminal with same disposition | yes | `discardCandidate` | `failed` | `none` |
| `SECRET_DELETE_FAILED` + `condition=deleteReadiness` | no durable delete step progressed | yes | `refreshDeleteImpact` | `failed` | `none` |
| `SECRET_DELETE_FAILED` + `condition=recoveryReadiness` | recovery row and remaining step unchanged | yes | `refreshRecoveryImpact` | `failed` | `none` |
| `SECRET_DELETE_FAILED` + `condition=applyOrActivationPlan` | activation pre-commit unchanged; post-commit uses typed recovery row | yes | `reopenChangePlan` | `failed` | `none` |
| `SECRET_PROJECTION_FORBIDDEN` | stable summary unchanged; readiness blocked | no | `reopenChangePlan` | `blocked` | `none` |
| `SECRET_DEPENDENCY_CHANGED` + `condition=deleteReadiness` | delete preview/readiness identity stale | yes | `refreshDeleteImpact` | `blocked` | `none` |
| `SECRET_DEPENDENCY_CHANGED` + `condition=general` | fresh owner/ref summary required | yes | `refreshSummary` | `blocked` | `none` |
| `SECRET_RECORD_CHANGED` + `condition=deleteReadiness` | delete record revision stale | yes | `refreshDeleteImpact` | `blocked` | `none` |
| `SECRET_RECORD_CHANGED` + `condition=general` | fresh ref summary required | yes | `refreshSummary` | `blocked` | `none` |
| `SECRET_BACKEND_CHANGED` + exact capture/rotation context | selected capture/rotation backend identity or capability changed | yes | exact action in the closed-context table, except new binding uses `chooseBackend` | `blocked` | `none` |
| `SECRET_BACKEND_CHANGED` + `condition=general` | fresh backend/capability summary required | yes | `refreshSummary` | `blocked` | `none` |
| `SECRET_CAPABILITY_EXPIRED` / `SECRET_CAPABILITY_CONSUMED` + `runtime(proxyRequest)` | stable summary unchanged; capability terminal | yes | `retryProxyRequest` | `blocked` | `none` |
| same capability codes + `runtime(usageProbe)` | stable summary unchanged; capability terminal | yes | `retryUsageProbe` | `blocked` | `none` |
| same capability codes + `runtime(codingPlanUsageProbe)` | stable summary unchanged; capability terminal | yes | `retryCodingPlanUsageProbe` | `blocked` | `none` |
| same capability codes + `runtime(modelFetch)` | stable summary unchanged; capability terminal | yes | `retryModelFetch` | `blocked` | `none` |
| same capability codes + `condition=applyOrActivationPlan` | admission/capability terminal | yes | `reopenChangePlan` | `blocked` | `none` |
| same capability codes + `condition=recoveryReadiness` | readiness/capability terminal | yes | `refreshRecoveryImpact` | `blocked` | `none` |
| same capability codes + `condition=stagedImportResume` | staged capability/admission terminal | yes | `resumeStagedImportCutover` | `blocked` | `none` |
| `SECRET_RECOVERY_NOT_FOUND` | no matching recovery row; current recovery impact required | no | `refreshRecoveryImpact` | `blocked` | `none` |
| `SECRET_RECOVERY_CHANGED` | current recovery impact/readiness required | yes | `refreshRecoveryImpact` | `blocked` | `none` |
| `SECRET_OPERATION_RECOVERY_REQUIRED` + `activationCleanup` | affected owners bound; active ref stale; candidate `cleanupRequired` | yes | `completeRecovery` | `partial` | `cleanupPending` |
| `SECRET_OPERATION_RECOVERY_REQUIRED` + `captureCompensation` | candidate/uncommitted record remains recoverable | yes | `completeRecovery` | `failed` | `cleanupPending` |
| `SECRET_OPERATION_RECOVERY_REQUIRED` + `deleteFinalization` | backend delete applied; durable state finalization pending | yes | `completeRecovery` | `partial` | `cleanupPending` |
| `SECRET_OPERATION_RECOVERY_REQUIRED` + `ownerDetachFinalization` | Provider detach committed; local binding CAS pending | yes | `completeRecovery` | `partial` | `cleanupPending` |
| `SECRET_OPERATION_RECOVERY_REQUIRED` + `condition=candidateTerminalCleanupPending` (no pointer) | candidate remains `verifiedPendingPlan`; immutable `discarded|expired` disposition and discard journal reachable | yes | `discardCandidate` | `failed` | `none` |
| `SECRET_INTERNAL` + exact closed context | never claim success; stable state re-read required | yes | exact action in the closed-context table | `failed` | `none`; a separately proven recovery item uses `SECRET_OPERATION_RECOVERY_REQUIRED` instead |

### 11.1 Successful/partial audit rows

| Operation result | Action | Outcome | Effect |
| --- | --- | --- | --- |
| verified candidate staged | `captureCandidate` / `rotateCandidate` / `reconcileLegacy` | `success` | `candidateStaged` |
| capture/rotate/reconcile rejected before candidate write | same attempted action | `blocked` or `failed` per error matrix | `none` |
| candidate discarded and backend removed/already absent | `discardCandidate` | `success` | `none` |
| discard attempted with no state change | `discardCandidate` | `blocked` or `failed` per error matrix | `none` |
| candidate expiry journal deletes/readbacks missing then marks `expired` | `discardCandidate` (startup/list sweeper records same scope) | `success` | `none` |
| candidate expiry delete/readback remains journaled and candidate pending | `discardCandidate` | `failed` | `none` |
| candidate activation complete | `activateCandidate` | `success` | `bindingChanged` |
| rotation old-record delete/already-missing plus independent fresh missing readback terminal with `supersededByRotation` source/time | `activateCandidate` / `retryCleanup` | `success` / `recovered` | `recordRevoked` |
| activation complete, legacy/old-record cleanup pending | `activateCandidate` | `partial` | `cleanupPending` |
| activation rejected before binding CAS | `activateCandidate` | `blocked` or `failed` per error matrix | `none` |
| activation candidate-read/old-delete/old-missing preparation or confirmation completes without binding CAS | `activateCandidate` | `success` | `none` |
| logical lock/unlock | `lock` / `unlock` | `success` | `policyChanged` |
| delete/revoke complete | `delete` / `revoke` | `success` | `recordRevoked` |
| readiness ready/confirmation-required/blocked | `checkReadiness` | `success` / `success` / `blocked` | `none` |
| prepare or hardware confirm | `prepareApply` / `confirmHardware` | `success` | `none` |
| apply hardware confirmation cancelled/expired and native pending terminated | `cancelConfirmation` | `success` | `none` |
| activation, staged-import or recovery hardware confirmation cancelled/expired and native pending terminated | `activateCandidate` / `migrateLegacy` / `retryCleanup` | `blocked` | `none` |
| writer receipt `succeeded` | `resolveApply` | `success` | `targetWriterInvoked` |
| writer receipt `failedBeforeMutation` | `resolveApply` | `failed` | `targetWriterInvoked` |
| writer receipt `failedAfterMutation/readbackMismatch/readbackUnavailable` | `resolveApply` | `partial` | `targetWriterInvoked` |
| apply failed before backend read/writer invocation | `prepareApply` / `resolveApply` | `blocked` or `failed` per error matrix | `none` |
| validation reports valid | `validate` | `success` | `none` |
| validation reports missing/revoked/locked/denied | `validate` | `blocked` | `none` |
| validation backend read fails | `validate` | `failed` | `none` |
| migration staged one or more plans | `migrateLegacy` | `success` | `candidateStaged` |
| staged import scrub/cutover/live-owner/binding journal reaches terminal | `migrateLegacy` | `success` | `bindingChanged` |
| staged import stops at a durable cutover recovery checkpoint | `migrateLegacy` | `partial` | `cleanupPending` |
| explicit staged-import resume reaches terminal | `reconcileRecovery` | `recovered` | `bindingChanged` |
| startup recovery reconciled | `reconcileRecovery` | `recovered` | `none` or `cleanupPending` |
| recovery read/delete preparation or confirmation completes before any required Provider lease | `retryCleanup` | `success` | `none` |
| `activationCleanup` completes all remaining scrub/old-delete/old-missing steps | `retryCleanup` | `recovered` | `none` |
| `captureCompensation` deletes/readbacks/finalizes the uncommitted candidate record | `retryCleanup` | `recovered` | `none` |
| `deleteFinalization` independently deletes the admitted record when needed, verifies fresh missing, then persists tombstone/owner summaries | `retryCleanup` | `recovered` | `recordRevoked` |
| `ownerDetachFinalization` completes exact owner tombstone/binding-set CAS after Provider detach | `retryCleanup` | `recovered` | `bindingChanged` |
| explicit recovery completes a proper subset and leaves an exact nonempty step set | `retryCleanup` | `partial` | `cleanupPending` |
| explicit recovery changes nothing | `retryCleanup` | `blocked` or `failed` per error matrix | `none` or `cleanupPending` only if the pre-existing row remains |

## 12. Codex feature-scope trust boundary

### 12.1 Public Provider type

Internal `Provider` remains a persistence/domain type and MUST NOT be a renderer return type on any Codex branch. v1 exposes only the following closed, token-free configuration summary. It has no raw settings/config/TOML/JSON member and no generic value container:

```rust
wire_enum!(CodexWireApi { Responses, ChatCompletions });

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CodexProviderConfigurationSummary {
    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "deserialize_absent_only")]
    pub base_url: Option<ValidatedUrl>,
    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "deserialize_absent_only")]
    pub model: Option<CodexModelId>,
    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "deserialize_absent_only")]
    pub model_provider_id: Option<CodexModelProviderId>,
    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "deserialize_absent_only")]
    pub wire_api: Option<CodexWireApi>,
    pub enabled: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexProviderPublicDto {
    id: OwnerId,
    name: SafeDisplayText,
    configuration: CodexProviderConfigurationSummary,
    owner_binding_summary: SecretOwnerCredentialSummary,
}

impl CodexProviderPublicDto {
    fn checked_from_provider_and_secret_authority(
        dto: CodexProviderPublicDto,
    ) -> Result<Self, SecretInternalError> {
        todo!("provider id/owner binding join and token-free configuration")
    }
}

wire_enum!(ProviderDeleteSeparateAction {
    GetSecretDeleteImpact, GetSecretCleanupImpact, NotApplicable
});
wire_enum!(ProviderDeleteBindingState { Bound, Unbound });

#[derive(Serialize)]
#[serde(
    tag = "state",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
enum ProviderDeleteExistingBindingView {
    Bound {
        secret_ref_display: SecretRefDisplay,
        binding_revision: SecretBindingRevision,
        binding_set_cas: SecretBindingSetCas,
        remaining_owners: SortedSecretOwners,
        becomes_orphan: bool,
    },
    Unbound {
        remaining_owners: [SecretOwner; 0],
        becomes_orphan: AlwaysFalse,
    },
}

#[derive(Serialize)]
#[serde(
    tag = "bindingState",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
enum ProviderDeleteReadyImpactRepr {
    Bound {
        provider_delete_impact_id: ProviderDeleteImpactId,
        provider_row_revision: ProviderRowRevision,
        owner_binding_revision: SecretOwnerBindingRevision,
        previewed_at: UtcTimestamp,
        expires_at: UtcTimestamp,
        owner: SecretOwner,
        existing_binding: ProviderDeleteExistingBindingView,
        legacy_source_coverage: LegacySourceCoverageView,
        delete_allowed: AlwaysTrue,
        effect: SecretEffect,
        secret_retained: AlwaysTrue,
        separate_secret_delete_action: ProviderDeleteSeparateAction,
    },
    Unbound {
        provider_delete_impact_id: ProviderDeleteImpactId,
        provider_row_revision: ProviderRowRevision,
        owner_binding_revision: SecretOwnerBindingRevision,
        previewed_at: UtcTimestamp,
        expires_at: UtcTimestamp,
        owner: SecretOwner,
        existing_binding: ProviderDeleteExistingBindingView,
        legacy_source_coverage: LegacySourceCoverageView,
        delete_allowed: AlwaysTrue,
        effect: SecretEffect,
        separate_secret_delete_action: ProviderDeleteSeparateAction,
    },
}

#[derive(Serialize)]
#[serde(
    tag = "bindingState",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
enum ProviderDeleteBlockedLegacyImpactRepr {
    Bound {
        provider_row_revision: ProviderRowRevision,
        owner_binding_revision: SecretOwnerBindingRevision,
        checked_at: UtcTimestamp,
        owner: SecretOwner,
        existing_binding: ProviderDeleteExistingBindingView,
        legacy_source_coverage: LegacySourceCoverageView,
        delete_allowed: AlwaysFalse,
        effect: SecretEffect,
        action: SecretUserAction,
    },
    Unbound {
        provider_row_revision: ProviderRowRevision,
        owner_binding_revision: SecretOwnerBindingRevision,
        checked_at: UtcTimestamp,
        owner: SecretOwner,
        existing_binding: ProviderDeleteExistingBindingView,
        legacy_source_coverage: LegacySourceCoverageView,
        delete_allowed: AlwaysFalse,
        effect: SecretEffect,
        action: SecretUserAction,
    },
}

#[derive(Serialize)]
#[serde(
    tag = "status",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
enum CodexProviderDeleteImpactRepr {
    Ready {
        schema_version: SchemaVersionV1,
        impact: ProviderDeleteReadyImpactRepr,
    },
    BlockedLegacyResolutionRequired {
        schema_version: SchemaVersionV1,
        blocked: ProviderDeleteBlockedLegacyImpactRepr,
    },
}

pub struct CodexProviderDeleteImpactDto(CodexProviderDeleteImpactRepr);

impl CodexProviderDeleteImpactDto {
    fn checked_ready_from_registry(
        repr: CodexProviderDeleteImpactRepr,
        registry: &ProviderDeleteImpactRegistration,
        coverage: &ProviderLegacySourceCoverageReceipt,
    ) -> Result<Self, SecretInternalError> {
        coverage.coverage.assert_complete_clear()?;
        let checked_view =
            LegacySourceCoverageView::checked_from_coverage_receipt(
                &coverage.coverage,
            )?;
        let _ = checked_view;
        todo!("Ready only; exact clear LegacySourceCoverageReceipt plus bound/unbound registry match")
    }

    fn checked_blocked_from_legacy_inventory(
        repr: CodexProviderDeleteImpactRepr,
        coverage: &ProviderLegacySourceCoverageReceipt,
    ) -> Result<Self, SecretInternalError> {
        coverage.coverage.assert_complete_blocking()?;
        let checked_view =
            LegacySourceCoverageView::checked_from_coverage_receipt(
                &coverage.coverage,
            )?;
        let _ = checked_view;
        todo!("Blocked only; current-scrubbable and/or adjacent-blocked coverage is positive and exact existing binding view matches; no impact id or detach journal is minted")
    }
}

impl Serialize for CodexProviderDeleteImpactDto {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CodexProviderDeleteConfirmRequestDto {
    pub schema_version: SchemaVersionV1,
    pub provider_delete_impact_id: ProviderDeleteImpactId,
}

pub(crate) enum ProviderDeleteBindingRegistration {
    Bound {
        secret_ref: SecretRef,
        binding_revision: SecretBindingRevision,
        binding_set_cas: SecretBindingSetCas,
    },
    Unbound,
}

pub(crate) struct ProviderLegacySourceCoverageReceipt {
    provider_row_revision: ProviderRowRevision,
    owner: SecretOwner,
    coverage: LegacySourceCoverageReceipt,
}

impl ProviderLegacySourceCoverageReceipt {
    fn checked_from_codex_inventory_bridge(
        provider_row_revision: ProviderRowRevision,
        owner: SecretOwner,
        coverage: LegacySourceCoverageReceipt,
    ) -> Result<Self, SecretInternalError> {
        todo!("bind the fresh complete eleven-domain receipt to the exact Provider row/owner snapshot; no raw path/locator/value/value-derived digest fields exist")
    }
}

// The Provider preview calls this exact entry before minting an impact id.
fn fresh_provider_delete_preview_coverage(
    legacy_sources: &mut CodexLegacySourceInventoryBridge<'_>,
    owner: &ExistingSecretOwnerToken,
    provider_row_revision: ProviderRowRevision,
) -> Result<ProviderLegacySourceCoverageReceipt, SecretInternalError> {
    let coverage = legacy_sources.fresh_provider_delete_coverage(
        owner,
        &provider_row_revision,
    )?;
    ProviderLegacySourceCoverageReceipt::checked_from_codex_inventory_bridge(
        provider_row_revision,
        owner.owner().clone(),
        coverage,
    )
}

pub(crate) struct ProviderDeleteImpactRegistration {
    impact_id: ProviderDeleteImpactId,
    owner: ExistingSecretOwnerToken,
    provider_row_revision: ProviderRowRevision,
    owner_binding_revision: SecretOwnerBindingRevision,
    binding: ProviderDeleteBindingRegistration,
    legacy_source_coverage: ProviderLegacySourceCoverageReceipt,
    remaining_owners: SortedSecretOwners,
    becomes_orphan: bool,
    previewed_at: UtcTimestamp,
    expires_at: UtcTimestamp,
}

pub(crate) struct ProviderDeleteImpactRegistrationInput {
    owner: ExistingSecretOwnerToken,
    provider_row_revision: ProviderRowRevision,
    owner_binding_revision: SecretOwnerBindingRevision,
    binding: ProviderDeleteBindingRegistration,
    clear_legacy_source_coverage: ProviderLegacySourceCoverageReceipt,
    remaining_owners: SortedSecretOwners,
    becomes_orphan: bool,
    previewed_at: UtcTimestamp,
    expires_at: UtcTimestamp,
}

pub(crate) struct ClaimedProviderDeleteImpact {
    registration: ProviderDeleteImpactRegistration,
    claim_nonce: [u8; 16],
}

impl ClaimedProviderDeleteImpact {
    fn fresh_revalidate_legacy_coverage(
        &self,
        legacy_sources: &mut CodexLegacySourceInventoryBridge<'_>,
    ) -> Result<ProviderLegacySourceCoverageReceipt, SecretInternalError> {
        let provider_row_revision =
            self.registration.provider_row_revision.clone();
        let coverage = legacy_sources.fresh_provider_delete_coverage(
            &self.registration.owner,
            &provider_row_revision,
        )?;
        let current =
            ProviderLegacySourceCoverageReceipt::checked_from_codex_inventory_bridge(
                provider_row_revision,
                self.registration.owner.owner().clone(),
                coverage,
            )?;
        current.coverage.assert_complete_clear()?;
        current.coverage.assert_same_complete_coverage_as(
            &self.registration.legacy_source_coverage.coverage,
        )?;
        Ok(current)
    }
}

pub(crate) trait ProviderDeleteImpactRegistry: Send + Sync {
    // Implemented/owned only in crate::commands::provider. mint validates the
    // full row above; it alone creates providerDeleteImpactId.
    fn mint(
        &self,
        input: ProviderDeleteImpactRegistrationInput,
    ) -> Result<ProviderDeleteImpactRegistration, SecretInternalError>;

    // Atomic Ready -> Claimed after exact expiry check. Missing/expired/
    // claimed/consumed ids are stale and cannot return the binding.
    fn claim_once(
        &self,
        impact_id: &ProviderDeleteImpactId,
        now: &UtcTimestamp,
    ) -> Result<ClaimedProviderDeleteImpact, SecretInternalError>;

    fn consume(
        &self,
        claim: ClaimedProviderDeleteImpact,
    ) -> Result<ProviderDeleteImpactRegistration, SecretInternalError>;

    fn terminate(&self, claim: ClaimedProviderDeleteImpact);
}

#[derive(Serialize)]
#[serde(
    tag = "status",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
enum CodexProviderDeleteResultRepr {
    ProviderDeletedBoundSecretRetained {
        schema_version: SchemaVersionV1,
        consumed_impact_id: ProviderDeleteImpactId,
        owner: SecretOwner,
        binding_state: ProviderDeleteBindingState,
        remaining_owners: SortedSecretOwners,
        becomes_orphan: bool,
        secret_retained: AlwaysTrue,
        separate_secret_delete_action: ProviderDeleteSeparateAction,
    },
    ProviderDeletedUnbound {
        schema_version: SchemaVersionV1,
        consumed_impact_id: ProviderDeleteImpactId,
        owner: SecretOwner,
        binding_state: ProviderDeleteBindingState,
        remaining_owners: [SecretOwner; 0],
        becomes_orphan: AlwaysFalse,
        separate_secret_delete_action: ProviderDeleteSeparateAction,
    },
    ProviderDeletedBoundDetachRecoveryRequired {
        schema_version: SchemaVersionV1,
        consumed_impact_id: ProviderDeleteImpactId,
        owner: SecretOwner,
        binding_state: ProviderDeleteBindingState,
        remaining_owners: SortedSecretOwners,
        becomes_orphan: bool,
        secret_retained: AlwaysTrue,
        separate_secret_delete_action: ProviderDeleteSeparateAction,
        recovery: SecretRecoveryPointer,
    },
    ProviderDeletedUnboundDetachRecoveryRequired {
        schema_version: SchemaVersionV1,
        consumed_impact_id: ProviderDeleteImpactId,
        owner: SecretOwner,
        binding_state: ProviderDeleteBindingState,
        remaining_owners: [SecretOwner; 0],
        becomes_orphan: AlwaysFalse,
        separate_secret_delete_action: ProviderDeleteSeparateAction,
        recovery: SecretRecoveryPointer,
    },
}

pub struct CodexProviderDeleteResultDto(CodexProviderDeleteResultRepr);

pub(crate) struct ProviderDetachCommitReceipt {
    _private: (),
}

impl CodexProviderDeleteResultDto {
    fn checked_from_detach(
        repr: CodexProviderDeleteResultRepr,
        registration: &ProviderDeleteImpactRegistration,
        commit: &ProviderDetachCommitReceipt,
    ) -> Result<Self, SecretInternalError> {
        todo!("consumed ready/no-legacy preview, exact bound/unbound and recovery arm")
    }
}

impl Serialize for CodexProviderDeleteResultDto {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

#[derive(Deserialize)]
#[serde(
    tag = "operation",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum CodexProviderMutationDto {
    Create {
        name: SafeDisplayText,
        configuration: CodexProviderConfigurationSummary,
    },
    Update {
        id: OwnerId,
        name: SafeDisplayText,
        configuration: CodexProviderConfigurationSummary,
    },
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexLiveStructuralProjection {
    contract_version: SecretContractVersionV1,
    schema_version: SchemaVersionV1,
    owner: SecretOwner,
    structural_revision: CodexLiveStructuralRevision,
    configuration: CodexProviderConfigurationSummary,
    binding_required: AlwaysTrue,
}

impl CodexLiveStructuralProjection {
    // Sole factory in crate::services::configuration_apply::provider after a
    // fresh live structural read. This projection is output/admission input by
    // ownership, never a wire-deserializable caller object.
    fn checked_from_live_authority(
        owner: ExistingSecretOwnerToken,
        structural_revision: CodexLiveStructuralRevision,
        configuration: CodexProviderConfigurationSummary,
    ) -> Result<Self, SecretInternalError> {
        todo!("provider/codex owner, token-free configuration, bindingRequired=true")
    }
}
```

`ValidatedUrl` rejects surrounding-trim drift, every control character, userinfo, query, fragment, percent-encoded/non-ASCII/over-512-byte paths, unsafe path characters and credential-shaped host/path content; its parsed canonical string must equal the input. `CodexModelId` and `CodexModelProviderId` are closed credential-free validated ASCII identifiers. Unknown internal non-secret Provider fields remain byte-for-byte in internal persistence but are neither emitted nor editable through v1. A future editable field requires an explicit schema/Change Plan extension; no public query/cache/event/job/IPC stores `Provider.settings` or raw config.

`CodexProviderPublicDto`, `CodexProviderMutationDto` and `CodexLiveStructuralProjection` are the only Provider configuration/read/mutation DTO names in this contract; the separately named delete-preview/result DTOs below expose only deletion impact and never Provider settings. `CodexProviderMutationDto.operation=create` has no `id`; the Provider owner module generates it. `operation=update` requires the DAO to resolve `id` to the existing exact Provider row before mutation, so syntax-valid caller text cannot mint authority. Every resulting owner is validated as `provider/codex`. `CodexLiveStructuralProjection.structuralRevision` is a non-value-derived layout revision and `bindingRequired=true` is a literal; it never carries a source value, source digest or device-local binding/ref.

Provider deletion is a main-integration operation outside the 15 #35 commands and has an exact preview/confirm boundary:

1. `crate::commands::provider` resolves an existing Provider row, reads its device-local binding independently, constructs `CodexLegacySourceInventoryBridge` from the existing `AppState`, and calls `fresh_provider_delete_preview_coverage` before minting any preview id. The returned receipt proves the exact fixed eleven-domain inventory, not only current Provider/live rows. Binding and legacy state are orthogonal: both bound+legacy and unbound+legacy are representable.
2. If that inventory is nonempty, the only result is `status=blockedLegacyResolutionRequired` with no `providerDeleteImpactId`, `deleteAllowed=false`, `effect=none`, canonical positive `sourceCount`, sorted-unique no-value categories, exact bound/unbound existing-binding view, and canonical `action=resolveLegacyConflict`. The Provider row and every current plaintext occurrence remain untouched. Because no deletion occurred, this branch has no `secretRetained` field and makes no retention claim.
3. Only a fresh `ProviderLegacySourceCoverageReceipt` whose nested `LegacySourceCoverageReceipt` has a complete eleven-domain identity and is exactly `clear` can mint `ProviderDeleteImpactRegistration`; that registration retains the preview receipt. The ready registry arm is exactly bound or unbound; bound carries ref/per-owner binding revision/binding-set CAS/remaining owners/orphan state and `secretRetained=true`, while unbound has no secret-retention field. Either a current-scrubbable source or an adjacent-blocked observation prevents impact-id/journal creation; there is no legacy registry arm.
4. Confirm accepts only `CodexProviderDeleteConfirmRequestDto { schemaVersion:1, providerDeleteImpactId }`, so a blocked preview is structurally unconfirmable. The registry atomically claims an unexpired ready id once and returns its private binding. Missing, expired, replayed or already-consumed previews are stale and return the Provider operation's typed refresh-impact response with `effect=none`.
5. After claim and before any Provider/device write, `ClaimedProviderDeleteImpact::fresh_revalidate_legacy_coverage` calls the bridge again and obtains a new receipt. While holding the Provider delete transaction/lease, main integration compares its inventory revision, all eleven domain revision/presence/count entries, exact current expectations and canonical adjacent observations to the retained preview receipt, then fresh-checks exact Provider-row, no-legacy, owner-binding, bound/unbound and remaining-owner identities. Drift terminates the preview and returns `{code: PROVIDER_DELETE_IMPACT_STALE, action: refreshProviderDeleteImpact, effect: none}` with zero Provider/device mutation; this non-#35 failure does not enter the 47-code secret error set.
6. After confirmation it journals `detachProviderOwner`, deletes the Provider row, then performs the exact owner detach CAS. Bound success is `providerDeletedBoundSecretRetained`; unbound success is `providerDeletedUnbound` and makes no retention claim. A post-Provider/pre-local failure returns the matching bound/unbound detach-recovery result and four-kind `ownerDetachFinalization` pointer. Backend record deletion remains the separate `get_secret_delete_impact → delete_secret` flow.

Both Provider delete DTO decoders reject unknown/value/path/value-digest fields. `remainingOwners` is canonical ascending `secret_owner_sort_key`, duplicate-free, and exactly equals the preview registry set; `becomesOrphan` iff a bound ref has zero remaining owners. Unbound requires empty remaining owners, no ref/binding/binding-set/retention field and `separateSecretDeleteAction=notApplicable`. Bound requires the exact binding view and `get_secret_delete_impact`. `legacySourceCoverage.state=clear` is mandatory for ready; `blockingSourcesPresent` is mandatory for blocked. Current-scrubbable categories are projected from exact `LegacySourceRef` expectations, while adjacent-blocked observations expose only their supplemental category/state and never a locator/origin/path/value/digest. The private Rust factories enforce all variants.

Raw add/update/import/deep-link/UniversalProvider and live JSON/TOML ingress is validated before conversion. At every object/table/inline-table depth, including arrays of objects, the validator:

1. rejects non-ASCII/control/trim-drift keys and duplicate keys after canonicalization;
2. canonicalizes a key by retaining only ASCII alphanumeric characters and lowercasing them, so `apiKey`, `api_key` and `API-KEY` all become `apikey`;
3. rejects every canonical member generated from the sole `FORBIDDEN_SEMANTIC_FIELDS_V1` source-spelling list in `12.3`; no second local list is permitted;
4. recursively validates every child before constructing `CodexProviderMutationDto` or `CodexLiveStructuralProjection`.

Known legacy credential occurrences are inventoried as exact `LegacySourceRef` values before this rejection path. Any forbidden key, malformed/duplicate/incomplete nested config or unrepresentable unknown mutation field fails before DB/live mutation with `SECRET_MIGRATION_REQUIRED` or `SECRET_LEGACY_SOURCE_INVALID` as specified by the migration flow. No empty string or redaction marker is substituted.

### 12.2 Closed call graph

| Codex surface | Required v1 boundary |
| --- | --- |
| Provider SQLite row | non-secret Provider configuration only; no ref/binding/candidate/journal/audit. `CodexProviderPublicDto.ownerBindingSummary` is joined from the device-local owner projection under `app_local_data_dir/device-local/secrets/v1` |
| Provider list/get/failover | `CodexProviderPublicDto` only; never internal `Provider` |
| add/update/import/deep-link/UniversalProvider conversion | closed `CodexProviderMutationDto` only; create omits/server-generates id, update resolves an existing id, and inline values/unknown mutation fields are rejected before writes |
| startup/live auth/config inventory | enumerate `LegacySourceRef` with `origin=liveAuth/liveConfig`, including every top-level, active, inactive and inline TOML table; invalid/non-string/duplicate source is `SECRET_LEGACY_SOURCE_INVALID` |
| staged SQL import / DB restore / sync download | pre-context inventory is structural-only with `origin=sqlImportStaging/dbRestoreStaging/syncDownloadStaging`; `crate::commands::import_export` mints the stage/temp-live-object token, #55 admits the closed staged projection, #35 prepares/confirms authorization without reads, main integration constructs `ImportCutoverCoordinatorContext`, and only then exact staged value validation/scrub/readback/cutover may run. Unresolved values, foreign authority or drift reject cutover with `effect=none` |
| historical/user-owned/managed historical artifacts | permanent v1 scan/report-only `SecretArtifactScanReport`; never rewrite/delete, never enter `legacySourcesToScrub`, candidate projections or recovery steps |
| raw live settings read | closed `CodexLiveStructuralProjection` or command unsupported; never raw JSON/TOML/settings |
| live backfill | non-secret settings only; no token recovery into Provider storage |
| #55 Provider switch / #41 apply | `SecretApplyPlanProjection → prepare/confirm/resolve` |
| Provider delete/detach | current legacy sources always yield no-id `blockedLegacyResolutionRequired`; only no-legacy bound/unbound `CodexProviderDeleteImpactDto →` single-use preview claim/CAS → durable `detachProviderOwner`. Retention is claimed only for bound; orphan secret deletion remains a separate #35 command flow |
| proxy request | dedicated `execute_proxy_request` + owner-private `ProxyRequestSecretExecution/PreparedProxyRequest.send_once`; `consumer=proxyRequest, sink=processMemory` cannot be caller-selected |
| proxy failover | secret readiness/resolve failure is terminal and circuit-neutral: no network and no next Provider; only a real post-send transport/upstream failure may advance and must resolve the next Provider independently |
| usage/balance | dedicated `execute_usage_probe` + closed `UsageProbeKind` + owner-private single-send request; no `String` returned to Provider or IPC |
| primary Provider coding-plan | dedicated `execute_coding_plan_usage_probe` + `consumer=codingPlanUsageProbe` + owner-private `CodingPlanSecretExecution/PreparedCodingPlanRequest.send_once`; belongs to `usageProbe/codex_feature_runtime`, is redirect-disabled and never caller-selectable |
| generic/test UsageScript | Codex primary-secret substitution/fallback is rejected before resolve/eval/network; credential-free scripts may run outside this contract |
| model fetch | dedicated `execute_model_fetch` + owner-private `ModelFetchSecretExecution/PreparedModelFetchRequest.send_once`; request accepts an existing-owner/binding expectation, never an API key |
| Provider terminal | v1 command boundary always returns `SECRET_CONSUMER_UNSUPPORTED`, `effect=none`; enum/sink remain wire-reserved and never enter record capabilities |
| official Codex OAuth | preserve Codex-owned OAuth state in place where safe; never classify/copy it as `codexApiKey` or place it in Provider/sync/backup |
| #41 takeover/rollback backup | local structural bundle uses typed owner/ref/location/revision placeholders only; rollback acquires a fresh prepared rollback capability |
| manual export / remote sync / Workspace Pack | sanitized Provider structure plus `bindingRequired` only; omit material, backend locator and device-local ref/binding identity |
| diagnostics | status/capability/counts/stable codes only; no internal Provider serialization |
| logs/events/errors | operation/event ids, stable enum and backend instance only; no ref by default in logs and no raw source |
| Provider query/cache/event/job/IPC | only `CodexProviderPublicDto` / `CodexProviderMutationDto` / `CodexLiveStructuralProjection` as applicable; no internal settings/raw config |
| startup bootstrap / DB preflight | `SecretBootstrap::open(&AppHandle) → Database::open_preflight_without_backup(authority, openedStore.database_preflight_token()) → AppState::new_production(db, appHandle, openedStore) → same SecretService.reconcile_startup(existing-DB context) → PreparedProductionAppState`; sole crate-root setup then `app.manage(state) → static-command-registration receipt → Clean: sanitized backup, gate release, workers / Blocked: publish scrubbed issue, workers off`. One non-cloneable lifetime lock, no raw pre-gate backup, temporary authority or reopen |

Every row receives an independent runtime canary assertion. `codex_feature_runtime` passes only when every row is covered. `repository_static_inventory` remains a report and cannot be converted into a pass with a broad allowlist.

The existing fixed primary-Provider coding-plan adapters use the same `codingPlanUsageProbe` owner binding and are in v1 scope. ZenMux credentials, independent access-key/secret-key pairs, login/session credentials and any adapter-specific secondary credential are separate credential classes and remain explicit follow-up debt; they may not borrow the primary key route, fall back to Provider inline settings or be marked covered by the primary coding-plan canary.

### 12.3 Forbidden wire keys

The schema scanner applies the same separator-insensitive ASCII canonicalization as raw ingress (retain alphanumeric, lowercase) at every nested object and rejects canonical duplicates. The following block is the sole normative source-spelling list named `FORBIDDEN_SEMANTIC_FIELDS_V1`; its 25 rows generate exactly 24 canonical members because `apiKey` and `api_key` deliberately collapse to one. Rust and TypeScript contain that same 24-member canonical set. Thus every spelling/case/separator variant of these semantic keys is forbidden in #35/#55/#41 public DTOs, commands, plans, queries, caches, jobs, events, receipts and fixtures:

```text
secret
secretValue
value
apiKey
api_key
openaiApiKey
experimentalBearerToken
token
accessToken
refreshToken
accessKey
secretKey
password
authorization
credential
privateKey
credentialBlob
backendLocator
rawError
rawMessage
rawConfig
providerSettings
liveSettings
absolutePath
materialDigest
```

The exact allowlist is:

```text
secretRef
secretRefDisplay
secretState
secretSummary
secretCount
secretBackend
secretPurpose
secretOperation
secretCandidate
secretProjection
lastValidatedAt
```

Inside `LegacySourceRef` only, `locationId/category/origin` are also allowed and their enums/newtype are closed above. An allowlisted name does not waive scalar validation. The canonical key `credential` has no waiver: public structural fields use `bindingState`, `ownerBindingSummary` and `bindingRequired`, and no public DTO field may restore a `credential*` spelling. `ownerId`, display text, writer code and cursors still use strict constructors so material-shaped arbitrary strings cannot be hidden in a nominally safe field.

`contract_schema` runs one shared Rust/TypeScript self-test table. For every source spelling it generates camelCase, snake_case, kebab-case, SCREAMING_SNAKE_CASE, dotted and repeated/mixed-separator forms at root, nested-object and array-object depth; each must reject, including canonical duplicates. Semantic-key and ASCII-only scalar validation first rejects every non-ASCII scalar, then applies ASCII-only lowercase/canonicalization; neither side invokes Unicode lowercase/case-folding. Scalar fixtures reject token-boundary occurrences anywhere, not only at string start: `provider=sk-live`, `note/Bearer%20abc`, `id:AKIAEXAMPLE`, `nested.api_key=value` and `prefix/github_pat_example`. The exact separator table is asserted code-point-for-code-point on both sides; `note\u00a0api_key=value` (NBSP) and `note\u2003API-KEY=value` (EM SPACE) reject in Rust and TypeScript, while an unlisted code point is never silently treated as a separator by only one language. They accept non-boundary substrings such as `basket-case` and `task-skeleton`. `OwnerId("api\u212Aey")` (Kelvin sign) and key `AP\u0130_KEY` (Latin capital I with dot) reject as non-ASCII before canonicalization on both sides. `SafeDisplayText` remains Unicode-capable: its shared scanner treats every non-ASCII scalar as a hard boundary and never folds it, so benign `"Kelvin \u212A"` and `"\u0130stanbul"` have the same accept result in both implementations. No TypeScript `String.toLowerCase`, `\s`/Unicode-regex shorthand, Rust Unicode lowercase or whitespace predicate may replace these exact rules. The same fixtures are applied to `SafeDisplayText`, `OwnerId`, `CodexModelId`, `CodexModelProviderId`, `ValidatedUrl` host/path segments and backend-private locators; any Rust/TypeScript result mismatch fails the gate.

## 13. Legacy reconcile and migration result invariants

The migration scanner works per `provider/codex` owner and enumerates every known source category. It compares material only inside native memory:

1. Build the complete `LegacySourceRef` list first. `locationId` is derived only from canonical structural locator bytes (`origin + category + internal table/index identity`), never from the source value. Live auth/config and each SQL-import/restore/sync staging source retain their exact origin. Ordinary current-owner candidates store their current-source expectations; staged source expectations live only in the staged projection/journal and are paired with an independently captured candidate. Locator, origin and category are never collapsed to a count/category-only row.
2. Malformed TOML, duplicate key, non-string credential field or incomplete staging parse makes the owner `sourceInvalid` and returns `SECRET_LEGACY_SOURCE_INVALID`. No subset may be compared, staged or scrubbed.
3. Official Codex OAuth fields are not `LegacySourceRef` values and are never coerced into `codexApiKey`.
4. `providerNonCanonicalProxyAlias` never seeds a migration candidate; it requires explicit replacement/reconcile and is scrubbed only after activation. `providerUsageScriptApiKey` may join a scrub-only candidate only after constant-time equality with the bound primary key; a distinct value is a conflict.
5. No binding + zero canonical values → `noCredential`.
6. No binding + one distinct canonical value (including multiple equal copies) → stage a new verified candidate; return `candidateStaged`.
7. No binding + more than one distinct canonical value → `conflict`; retain all internal values; public projection remains scrubbed.
8. Binding + all inline values successfully verified equal to a fresh read of the exact bound backend record → stage `legacyScrubExistingBinding` with exact expectations; return `cleanupCandidateStaged`.
9. Binding + any verified difference → `conflict`; retain inline values.
10. Binding + backend cannot be read/confirmed → `comparisonPending`; retain inline values. `probe=present` is never enough to scrub.
11. Automatic migration and `legacyScrubExistingBinding` candidates persist `comparisonPolicy=candidateEquality`: approved activation fresh-reads the candidate, re-resolves the complete exact current occurrence set/revisions under the #41 lease and constant-time-compares every value before binding CAS. User recovery from `sourcesConflict|bindingConflict`, or an explicit native replace/reconcile/rotate capture, persists `comparisonPolicy=explicitReplacement` plus the approved replacement impact: activation still fresh-validates the complete exact occurrence set/revisions and candidate backend identity, but intentionally does not require the conflicting old values to equal the new candidate. Either policy rejects missing/extra/retyped/relocated/revision drift with `SECRET_DEPENDENCY_CHANGED/effect=none`; value mismatch additionally rejects equality mode. The policy-discriminated opaque receipt is required by binding CAS, journal and scrub.
12. Ordinary retry never chooses a source. Cancel produces no candidate or scrub. Re-running after activation is `alreadyMigrated` and returns the existing bound owner summary.

`LegacyMigrationOwnerResult.action` is likewise safe when cached: `noCredential|alreadyMigrated → none`, `candidateStaged|cleanupCandidateStaged → reopenChangePlan`, `comparisonPending → refreshSummary`, `sourceInvalid|failed → resolveLegacyConflict`, and `conflict` follows the exact owner state (`sourcesConflict → resolveLegacyConflict`, `bindingConflict → captureReplacement`). `blocked` uses the one source-specific external action from §11 and then fresh summary/readiness. No migration result row may emit invocation-dependent `retry`.

`SecretMigrationReport.status` is derived:

- `noChanges`: every row is `noCredential/alreadyMigrated`; report-only artifact findings do not become actions;
- `staged`: at least one candidate/projection exists, no #55 plan has yet been created, and there is no conflict/block/failure;
- `approvalRequired`: at least one row has `planId` and that #55 plan awaits approval;
- `partial`: a mix of staged/success and source-invalid/conflict/block/failure, or an artifact scan is partial/unreadable while at least one owner progressed;
- `blocked`: no owner progressed and at least one row is source-invalid/conflict/comparison-pending/blocked/failed, or the requested artifact scan itself is blocked.

Every historical/user-owned/managed historical finding contributes only to `findingCount/reportOnlyCount`; unreadable inputs contribute to `unreadableCount`. v1 never rewrites, deletes, imports or approves one of these artifacts and reports only category/counts, never paths. Ordinary `legacySourcesToScrub` is restricted to the complete current Provider/live occurrence set (`origin=providerRow|liveAuth|liveConfig`) with exact locator/category/structural revision. `sqlImportStaging|dbRestoreStaging|syncDownloadStaging` remain exact structural inventory origins and never seed or migrate a candidate. The staged projection references an independently secure-captured, backend-verified candidate; only after `ImportCutoverCoordinatorContext` exists may staged values be read and compared against it. Structural expectations enter scrub expectations only in the dedicated `StagedSecretImportActivationProjection`, bound to its temp-DB live-object identity and cutover CAS; they are never normalized into or scrubbed by ordinary activation/recovery.

## 14. Hardware instance and operation policy

| Field | `osKeyring` MVP | registered hardware record |
| --- | --- | --- |
| instance identity | device-local `sbi_*` OS adapter | one `sbi_*` per configured plugin/device instance |
| generation | increments on store reconfiguration | increments on plugin/device/attestation change |
| device display | absent or sanitized local OS account label (`osAccount/platform`), no user name/path | required sanitized device name/class/transport; no serial/locator |
| record capabilities | snapshot per record | queried per record; never backend-wide static fiction |
| confirmation | all v1 operations `never` | exact per `captureVerify/validate/resolveForApply/delete/revoke` |
| residency | `osProtectedStore` | `hardwareOnly` |
| allowed consumers | `SecretRuntimeConsumer = changePlanApply, proxyRequest, usageProbe, codingPlanUsageProbe, modelFetch`; never `providerTerminal` | explicit subset of the same strict v1 type |
| allowed sinks | `SecretRuntimeSink = processMemory, externalConfigFile`; external requires approved plan plus one exact non-path `CodexLiveSecretSinkId`; child-process sink is wire-reserved/rejected | explicit strict subset; default only `processMemory`; any external sink also requires exact closed sink id |
| persistent projection | true with approved plan | false by default and absent from allowed sinks |
| central revocation | `centralRevocation=false`, `revocationObservation=unsupported` | `centralRevocation=true` iff validated capability says `revocationObservation=sourceAndTime` and the adapter can return `{source=centralBackend|deviceAdministration, revokedAt}` |
| fallback | false | false |

Backend write, read/compare, delete and fresh missing-readback confirmation are therefore explicitly represented by exactly five hardware policy operations: `captureVerify`, `validate`, `resolveForApply`, `delete`, `revoke`. Every missing-readback authorization reuses the `validate` policy; it never adds a sixth operation, combines with delete, or reuses delete authorization. Its named confirmation slot and durable delete-applied checkpoint remain independent. The closed operation-specific slot inventory is activation `3` plus recovery `7` plus candidate discard `2`, exactly `12`; exactly `10` of those are delete/missing-specific. A hardware `captureVerify` confirmation is completed before secure input opens. Candidate activation and activation recovery prepare active-record, old-delete and old-missing confirmations independently before #41 takes the Provider lease; no prompt starts under that lease. Dedicated proxy/usage/balance/primary-coding-plan/model-fetch execution is v1-eligible only when confirmation is `never`; it never prompts implicitly. Rotation may activate the new binding before old-record deletion; cancel/failure of either delete or missing readback is `activatedCleanupPending` and creates a callable `activationCleanup` row, never rollback to the old binding.

Ordinary `probe` and authorized read can return only a non-`Clone`, non-serde, non-persistable `BackendRevocationHint`; authority has no method that accepts it. `PlatformRevocationObservation` is obtainable only through `PlatformBackendPort::observe_revocation_once`. The registered wrapper first consumes an authorization whose complete scope is exact `General::Revoke`, then verifies `BackendRevocationObservationCapability::SourceAndTime`, the record's validated `centralRevocation=true` capability, both durable `DeviceInstanceId` equality and process-store `Arc::ptr_eq`, registered binding, ref, store/record revisions, binding-set CAS, backend instance/generation, returned backend/device generations, device-binding generation and capability revision. Only then does it produce one non-`Clone`/non-serde consuming `BackendRevocationObservation` receipt. `DeviceLocalSecretAuthority::persist_backend_revocation_observation` accepts only that receipt (never a caller-supplied ref), fresh-revalidates the same tuple under its mutation permit, consumes it and persists source/time. `missing`, a hint, lock, permission denial, device mismatch and backend unavailable never synthesize revocation. OS keyring capability is always `centralRevocation=false`; a hardware adapter that cannot supply source/time must also publish false. The persisted observation becomes the sole central/device revocation source in summaries, errors and audit rows.

If Windows backend verification is referenced by implementation evidence, its case table must enumerate every named OS-keyring write/readback/delete/locked/denied/missing/encoding case and every applicable hardware confirmation/revocation case with `result=pass`. A total count, aggregate “passed” label or omitted case cannot satisfy this contract.

## 15. Review-finding closure map

This contract supplies a concrete disposition for the contract-layer findings below; reviewers still decide whether the surrounding authority documents adopted it correctly.

| Findings | Contract disposition |
| --- | --- |
| PR-001, DD-001, DD-006, DD-015, AR-007 | truthful `codex_feature_runtime` boundary, exact closed call graph and closed `CodexProviderPublicDto/CodexProviderMutationDto/CodexLiveStructuralProjection` with no arbitrary settings; Agent/non-Codex debt does not become a global claim |
| PR-002, DD-003, AR-006 | apply uses preview → prepare/confirm → Provider lease/final baseline/backup → role resolve; activation and `activationCleanup` prepare every hardware read/delete/missing-readback slot pre-lease, then require #41-held Provider lease/final baseline before #35 mutation gate. Local recovery stays local, owner detach uses main integration's already-held context, and staged import prepares authorization before its main-integration cutover context without reading staged values |
| PR-003, AR-006 | material-free capability plus sealed backend authorization scope bound to plan/role-or-slot/owner/store/record/binding/backend/device/capability/recovery-kind-CAS/sink/expiry; independent private target/rollback role slots and process-local delete/kind-tagged-recovery readiness are single-consume/replay-safe |
| PR-004, DD-002 | `agent` remains wire-reserved but every runtime request is typed-rejected |
| PR-005, PR-011, DD-002, AR-008 | owner-level legacy union, verify-not-probe equality, staged reconcile and exact per-owner/report/result DTOs; historical/user-owned/managed historical artifacts are permanently scan/report-only |
| PR-007 | `revoked` availability, source/time/action and distinct missing/user-delete/central-device mapping; rotation old-record terminal is exactly `supersededByRotation` only after a fresh typed missing-readback receipt |
| PR-008 | stable aggregate has no operation confirmation; readiness/prepare own requirement/step |
| PR-009, DD-011 | required `lockSource` and distinct user actions |
| PR-010, DD-009, AR-009 | revision + digest + exact-row binding-set CAS on rotate/lock/delete plus byte-exact recovery CAS over full affected-owner and durable tagged remaining-step rows |
| PR-012, AR-012 | backend instance, generation, per-record capabilities and safe device display; unregistered hardware hidden |
| DD-002 | exact mirrored request/result/error envelope for all 15 registered commands, including kind-tagged recovery impact/readiness/retry under the two existing cleanup-named commands |
| DD-003 | explicit scope-specific sealed backend `read + verify/delete/missing-readback/observe-revocation`, callback-contained write/readback compare, concrete token-gated `Arc<SecretService>`/`SecretServiceDeps` + opaque test builder shape, one lifetime-locked bootstrap whose same service reconciles before a managed/registered `AppState` gate starts backup/workers, exact #41/main-integration contexts/live sink ids, authority-minted proxy/usage/coding-plan/model-fetch bindings, no-redirect fixed HTTP consumers and consuming fixed-output `send_once` APIs |
| DD-011 | strict native-generated identifier/revision types and exhaustive error/action/audit matrix |
| AR-004 | #55 projection contains only ref/revisions/sink/capability identity; no raw Provider/live or material-derived digest |
| AR-005 | normative #35 boundary adds no SQLite schema/`user_version` transition and does not occupy v17 |
| PV7-001 | public staged resume request is exactly `stageId + expectedResumeCas{revision,digest}`; its independent no-value result returns the typed current CAS in every terminal/recovery arm while every authority/checkpoint/candidate/owner/ref/summary field remains internal |
| PV7-002 | durable terminal expiry carries no pending issue, always routes to `refreshSummary`, never reuses the candidate, and requires a newly minted capture intent or rotation flow after refresh |
| PV7-003 | `resolveLegacyConflict` is the executable `secretCaptureFlow(legacyReconcile)`; the server-held intent binds exact owner/current legacy/hidden-binding authority and begin accepts only intent id + registered backend id |
| CAV7-001 | immutable staged order is structural temp authority/projection → #55 admission → authority match → #35 authorization prepare/confirm → `ImportCutoverCoordinatorContext`; no staged value read/parse/compare/validate/scrub/readback/cutover is legal before the context |
| CAV7-002 | `deleteFinalization` has independent admitted-record delete and admitted-record-missing prepared/pending/confirmation/authorization slots with durable `backendApplied` between them |
| CAV7-003 | capture compensation has independent uncommitted-record delete and missing-readback consumes; the combined delete+probe type/API is absent |
| CAV7-004 | activation and activation cleanup each have independent old-delete and old-missing slots/CAS; the missing receipt atomically mints supersession + terminal state, with no fourth step or empty-suffix nonterminal phase |
| CAV7-005 | ordinary read/probe returns only non-persistable `BackendRevocationHint`; only exact Revoke-authorized `observe_revocation_once` can mint a persistent observation |
| CAV7-006 | durable journal/backend identity binds strict `DeviceInstanceId`; each opened store mints a separate non-cloneable/non-serde process `DeviceSecretStoreInstanceId`; records/scopes bind both and receipts/pending retain the process Arc plus registered binding, with `Arc::ptr_eq`, durable-id and returned-generation checks before data/receipts cross it |
| CAV7-007 | operation-owned capability bundle atomically claims the opaque capability before moving the exact role and terminalizes remaining roles without exposing/borrowing a private id |
| CAV7-008 | all backend operation-context fields and constructors are operation-broker private; brokered preparation consumes typed opaque admission/readiness/journal/runtime/staged claims and scanner rejects literals/re-exports |
| CAV7-009 | `SecretBootstrapToken` is a non-cloneable `pub(crate)` opaque type with private fields, so the opened-store borrow is legal across sibling modules without widening construction |
| CAV7-010 | generic `retry` is absent; all 24 actions have one closed destination, capture/runtime flows are typed and every retry-like entry declares a fresh server-generated operation id |
| CAV7-011 | every `SecretInternalError` field is private; the sole exhaustive checked factory accepts a closed source-free code enum or a dedicated required-source/recovery wrapper, derives retry/action/effect, and external literals are scanner-forbidden |
| CAV7-012 | TS/Rust request/result/action mirrors retain strict null/omit/deny-unknown/output-only rules, exact per-command error allowlists, 15 #35 commands, 47 errors, 24 actions, 8 journals and 4 recovery kinds |
| V9-001 | initial staged activation keeps its terminal summary DTO, while independent `ResumeStagedImportCutoverResultDto` has exact five-field data `{stageId,currentResumeCas,status,action,issue}` in every arm (`issue=null` terminal), returns fresh CAS, and forbids schema/audit/candidate/owner/ref/summary; request is exactly `stageId + expectedResumeCas` |
| V9-002 / product P1 | one opaque `pub(crate)` non-Clone/non-Serde/non-Debug `LegacySourceCoverageReceipt` retains a non-value `LegacySourceInventoryRevision`, exact fixed-field `CompleteLegacySourceCoverageIdentity`, current-scrubbable expectations and category-only adjacent observations. `CodexLegacySourceInventoryBridge` alone privately constructs `CompleteLegacySourceInventoryAuthority`; the receipt factory consumes it, while store/Provider siblings can only name/move/validate/consume the receipt. Startup/each summary/capture mint/capture revalidation/Provider preview/Provider confirm each obtain a new receipt, and empty sets yield Clean only with all eleven domain revision/presence/count proofs complete |
| V9-003 | strict durable `DeviceInstanceId(dev_*)` and process-only `DeviceSecretStoreInstanceId` have disjoint lifecycles; journal rows never serialize the process identity and live backend objects never substitute the durable id for Arc identity |
| V9-004 | `BackendOperationBroker` privately owns capture-intent/capability/pending registries; private assembly moves the same Arc through non-public deps into the sole `SecretService` field, with no caller/test injection or extractor, and executable list-mint, begin-claim/revalidate and consume/terminalize flow |
| V9-005 | hardware confirmation algebra remains exactly five operations; missing-readback uses `Validate` policy while retaining its independent slot, authorization and durable checkpoint |
| V9-006 | `SecretLegacyCleanupPending::validate` matches only its two legal arms and every arm returns `Result`; the nonexistent `Self::Summary` branch is absent |
| V9-007 | backend core declares only generic sealed route traits and its platform callback; #41/main/runtime concrete callback impls live in lane adapter modules, so #35 core has no compile dependency on not-yet-landed external callback types |
| V9-008 | the checked registration receipt stores the exact 15-element `SecretCommandName` array plus a separately typed `resume_staged_import_cutover` handler proof; resume cannot enter `SecretCommandName` and returns its independent DTO |
| ARR-001 | candidate discard/expiry has a closed two-slot preparation algebra: independent `RecordDelete/Delete` and `RecordMissingReadback/Validate` one-shot authorizations, an operation-owned CAS reservation, exact three-field durable `BackendApplied`, and a separate fresh-missing phase before one atomic terminal transition; there is no candidate `stateFinalized`, and public counts/recovery kinds do not change |
| ARR-002 | normal activation and `activationCleanup` retain exact `{deleteDisposition,backendCompletedAt,deleteAppliedCas}` old-record checkpoints, including `RecoveryRequired`; resume/recovery-CAS codecs consume all three, and fresh missing plus supersession/Terminal commit atomically with `revokedAt=backendCompletedAt` |
| ARR-003 | staged resume CAS binds the common journal `operationId` and exactly five cumulative phase arms `intent|sourcesScrubbed|cutoverCommitted|liveOwnerMinted|localBindingFinalized`; revision/digest advances on phase/nonce/admission/receipt/owner changes, with five named canonical digest fixture plans |

This file does not by itself close non-contract findings about crash-journal implementation, sync/import/restore execution, exact Win32/AppKit APIs, post-freeze resolved-lock/license/advisory/MSRV verification for the planned direct `subtle = 2.6.1` dependency, file-owner adjudication, actual AppState wiring, V2 adapter files, native host provisioning or executable evidence commands. Those remain in their respective authority/design owners and MUST stay open until separately revised and re-reviewed.
