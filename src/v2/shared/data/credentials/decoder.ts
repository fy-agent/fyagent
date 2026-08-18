import {
  BEGIN_SECRET_CAPTURE_REQUEST_KEYS,
  CREDENTIAL_PREFIX_MARKERS_V1,
  FORBIDDEN_SEMANTIC_FIELDS_V1,
  SECRET_LOCK_SOURCES,
  SECRET_REVOCATION_SOURCES,
  SECRET_STABLE_AVAILABILITIES,
  SECRET_USER_ACTIONS,
  type BeginSecretCaptureRequest,
  type CredentialsSnapshot,
  type OwnerBindingState,
  type SecretBackendInstanceId,
  type SecretCaptureIntentId,
  type SecretCandidateSummary,
  type SecretConfirmationRequirementView,
  type SecretDeleteImpact,
  type SecretOwnerCredentialSummary,
  type SecretRef,
  type SecretRefAggregate,
  type SecretRefDisplay,
  type SecretStableAvailability,
  type SecretUserAction,
} from "./types";

export class SecretContractDecodeError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "SecretContractDecodeError";
  }
}

const SECRET_REF_RE =
  /^sec_[0-9a-f]{12}4[0-9a-f]{3}[89ab][0-9a-f]{15}$/;
const CAPTURE_INTENT_RE =
  /^sci_[0-9a-f]{12}4[0-9a-f]{3}[89ab][0-9a-f]{15}$/;
const BACKEND_INSTANCE_RE =
  /^sbi_[0-9a-f]{12}4[0-9a-f]{3}[89ab][0-9a-f]{15}$/;
const CANDIDATE_ID_RE =
  /^scd_[0-9a-f]{12}4[0-9a-f]{3}[89ab][0-9a-f]{15}$/;
const OWNER_ID_RE = /^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$/;
const SECRET_REF_DISPLAY_RE = /^sec_…[0-9a-f]{4}$/;

const CREDENTIAL_SEPARATOR_CODE_POINTS_V1: readonly number[] = [
  0x0009, 0x000a, 0x000b, 0x000c, 0x000d, 0x0020, 0x0023, 0x0026, 0x002c,
  0x002e, 0x002f, 0x003a, 0x003b, 0x003d, 0x003f, 0x0040, 0x005c, 0x00a0,
  0x2003,
];
const CREDENTIAL_SEPARATOR_SET_V1 = new Set(CREDENTIAL_SEPARATOR_CODE_POINTS_V1);

const isCredentialSeparatorV1 = (value: string): boolean => {
  const codePoint = value.codePointAt(0);
  return codePoint !== undefined && CREDENTIAL_SEPARATOR_SET_V1.has(codePoint);
};

const asciiLowerV1 = (value: string): string =>
  [...value]
    .map((ch) => {
      const cp = ch.codePointAt(0)!;
      return cp >= 0x41 && cp <= 0x5a ? String.fromCodePoint(cp + 0x20) : ch;
    })
    .join("");

const isAsciiV1 = (value: string): boolean =>
  [...value].every((ch) => ch.codePointAt(0)! <= 0x7f);

const credentialSemanticPartsV1 = (
  value: string,
  unicodeBoundary: boolean,
): readonly string[] => {
  const parts: string[] = [];
  let current = "";
  for (const ch of value) {
    if (
      isCredentialSeparatorV1(ch) ||
      (unicodeBoundary && ch.codePointAt(0)! > 0x7f)
    ) {
      if (current.length > 0) {
        parts.push(current);
      }
      current = "";
    } else {
      current += ch;
    }
  }
  if (current.length > 0) {
    parts.push(current);
  }
  return parts;
};

export const canonicalSemanticKeyV1 = (value: string): string => {
  if (!isAsciiV1(value)) {
    throw new SecretContractDecodeError("non-ASCII semantic key");
  }
  return asciiLowerV1([...value].filter((ch) => /[A-Za-z0-9]/.test(ch)).join(""));
};

const hasTokenBoundaryMarkerV1 = (value: string, marker: string): boolean => {
  for (let from = 0; from <= value.length - marker.length; from += 1) {
    const index = value.indexOf(marker, from);
    if (index < 0) {
      return false;
    }
    if (index === 0 || !/[A-Za-z0-9]/.test(value.charAt(index - 1))) {
      return true;
    }
    from = index;
  }
  return false;
};

const credentialShapedTokenStreamV1 = (
  value: string,
  unicodeBoundary: boolean,
): boolean => {
  const lower = asciiLowerV1(value);
  const semantic = credentialSemanticPartsV1(lower, unicodeBoundary).some(
    (part) => {
      const canonical = canonicalSemanticKeyV1(part);
      return (
        FORBIDDEN_SEMANTIC_FIELDS_V1.has(canonical) || canonical === "bearer"
      );
    },
  );
  return (
    semantic ||
    CREDENTIAL_PREFIX_MARKERS_V1.some((marker) =>
      hasTokenBoundaryMarkerV1(lower, marker),
    )
  );
};

export const credentialShapedAsciiV1 = (value: string): boolean =>
  !isAsciiV1(value) || credentialShapedTokenStreamV1(value, false);

export const credentialShapedDisplayV1 = (value: string): boolean =>
  credentialShapedTokenStreamV1(value, true);

export function isForbiddenSemanticField(name: string): boolean {
  try {
    return FORBIDDEN_SEMANTIC_FIELDS_V1.has(canonicalSemanticKeyV1(name));
  } catch {
    return false;
  }
}

function fail(path: string, message: string): never {
  throw new SecretContractDecodeError(`${path}: ${message}`);
}

function assertObject(value: unknown, path: string): Record<string, unknown> {
  if (value === null || typeof value !== "object" || Array.isArray(value)) {
    fail(path, "expected object");
  }
  return value as Record<string, unknown>;
}

function assertAllowedKeys(
  record: Record<string, unknown>,
  allowed: readonly string[],
  path: string,
): void {
  const allowedSet = new Set(allowed);
  for (const key of Object.keys(record)) {
    if (isForbiddenSemanticField(key)) {
      fail(path, `forbidden semantic field "${key}"`);
    }
    if (!allowedSet.has(key)) {
      fail(path, `unknown field "${key}"`);
    }
  }
}

const ALLOWED_CONTRACT_LITERALS = new Set([
  "secret.hardware.confirmTouch",
]);

function assertString(value: unknown, path: string): string {
  if (typeof value !== "string") {
    fail(path, "expected string");
  }
  if (value.includes("\0") || /[\u0000-\u001f\u007f]/.test(value)) {
    fail(path, "control characters are forbidden");
  }
  if (
    !ALLOWED_CONTRACT_LITERALS.has(value) &&
    credentialShapedDisplayV1(value)
  ) {
    fail(path, "credential-shaped string is forbidden");
  }
  return value;
}

export function assertNotSecretRefDisplayIdentity(
  value: string,
  path: string,
): void {
  if (value.includes("…") || value.includes("...")) {
    fail(path, "secretRefDisplay must not be used as identity");
  }
}

export function decodeSecretRef(value: unknown, path = "secretRef"): SecretRef {
  const text = assertString(value, path);
  assertNotSecretRefDisplayIdentity(text, path);
  if (!SECRET_REF_RE.test(text)) {
    fail(path, "invalid SecretRef");
  }
  return text as SecretRef;
}

export function decodeSecretRefDisplay(
  value: unknown,
  path = "secretRefDisplay",
): SecretRefDisplay {
  const text = assertString(value, path);
  if (!SECRET_REF_DISPLAY_RE.test(text)) {
    fail(path, "invalid secretRefDisplay");
  }
  return text as SecretRefDisplay;
}

function decodeCaptureIntentId(
  value: unknown,
  path: string,
): SecretCaptureIntentId {
  const text = assertString(value, path);
  assertNotSecretRefDisplayIdentity(text, path);
  if (!CAPTURE_INTENT_RE.test(text)) {
    fail(path, "invalid SecretCaptureIntentId");
  }
  return text as SecretCaptureIntentId;
}

function decodeBackendInstanceId(
  value: unknown,
  path: string,
): SecretBackendInstanceId {
  const text = assertString(value, path);
  assertNotSecretRefDisplayIdentity(text, path);
  if (!BACKEND_INSTANCE_RE.test(text)) {
    fail(path, "invalid SecretBackendInstanceId");
  }
  return text as SecretBackendInstanceId;
}

export function assertPublicNoValue(value: unknown, path = "$"): void {
  if (typeof value === "string") {
    assertString(value, path);
    return;
  }
  if (Array.isArray(value)) {
    value.forEach((item, index) => assertPublicNoValue(item, `${path}[${index}]`));
    return;
  }
  if (value && typeof value === "object") {
    for (const [key, nested] of Object.entries(value)) {
      if (isForbiddenSemanticField(key)) {
        fail(path, `forbidden semantic field "${key}"`);
      }
      assertPublicNoValue(nested, `${path}.${key}`);
    }
  }
}

export function decodeBeginSecretCaptureRequest(
  value: unknown,
): BeginSecretCaptureRequest {
  const record = assertObject(value, "BeginSecretCaptureRequest");
  for (const key of Object.keys(record)) {
    if (isForbiddenSemanticField(key)) {
      fail("BeginSecretCaptureRequest", `forbidden semantic field "${key}"`);
    }
    if (key === "secretRef" || key === "secretRefDisplay") {
      fail(
        "BeginSecretCaptureRequest",
        "panel request may only send captureIntentId and backendInstanceId",
      );
    }
  }
  assertAllowedKeys(
    record,
    BEGIN_SECRET_CAPTURE_REQUEST_KEYS,
    "BeginSecretCaptureRequest",
  );
  return {
    captureIntentId: decodeCaptureIntentId(
      record.captureIntentId,
      "BeginSecretCaptureRequest.captureIntentId",
    ),
    backendInstanceId: decodeBackendInstanceId(
      record.backendInstanceId,
      "BeginSecretCaptureRequest.backendInstanceId",
    ),
  };
}

const OWNER_KEYS = ["kind", "namespace", "ownerId", "slot"] as const;

function decodeOwner(value: unknown, path: string) {
  const record = assertObject(value, path);
  assertAllowedKeys(record, OWNER_KEYS, path);
  const ownerId = assertString(record.ownerId, `${path}.ownerId`);
  if (!OWNER_ID_RE.test(ownerId) || credentialShapedAsciiV1(ownerId)) {
    fail(`${path}.ownerId`, "invalid OwnerId");
  }
  if (record.kind !== "provider") {
    fail(`${path}.kind`, "v1 panel accepts provider only");
  }
  if (record.namespace !== "codex") {
    fail(`${path}.namespace`, "v1 panel accepts codex only");
  }
  if (record.slot !== "primaryApiKey") {
    fail(`${path}.slot`, "v1 panel accepts primaryApiKey only");
  }
  return {
    kind: "provider" as const,
    namespace: "codex" as const,
    ownerId: ownerId as never,
    slot: "primaryApiKey" as const,
  };
}

function decodeAvailability(
  value: unknown,
  path: string,
): SecretStableAvailability {
  const text = assertString(value, path);
  if (
    !(SECRET_STABLE_AVAILABILITIES as readonly string[]).includes(text)
  ) {
    fail(path, "unknown availability");
  }
  return text as SecretStableAvailability;
}

function decodeAction(value: unknown, path: string): SecretUserAction {
  const text = assertString(value, path);
  if (!(SECRET_USER_ACTIONS as readonly string[]).includes(text)) {
    fail(path, "unknown SecretUserAction");
  }
  return text as SecretUserAction;
}

const BINDING_BOUND_KEYS = [
  "state",
  "secretRef",
  "secretRefDisplay",
  "bindingRevision",
] as const;
const BINDING_LEGACY_KEYS = [
  "state",
  "legacyState",
  "sources",
  "sourceCount",
  "action",
] as const;
const BINDING_UNBOUND_KEYS = ["state"] as const;

function decodeBindingState(value: unknown, path: string): OwnerBindingState {
  const record = assertObject(value, path);
  const state = assertString(record.state, `${path}.state`);
  if (state === "bound") {
    assertAllowedKeys(record, BINDING_BOUND_KEYS, path);
    return {
      state: "bound",
      secretRef: decodeSecretRef(record.secretRef, `${path}.secretRef`),
      secretRefDisplay: decodeSecretRefDisplay(
        record.secretRefDisplay,
        `${path}.secretRefDisplay`,
      ),
      bindingRevision: record.bindingRevision as never,
    };
  }
  if (state === "legacy") {
    assertAllowedKeys(record, BINDING_LEGACY_KEYS, path);
    return {
      state: "legacy",
      legacyState: record.legacyState as never,
      sources: record.sources as never,
      sourceCount: record.sourceCount as never,
      action: decodeAction(record.action, `${path}.action`),
    };
  }
  if (state === "unbound") {
    assertAllowedKeys(record, BINDING_UNBOUND_KEYS, path);
    return { state: "unbound" };
  }
  fail(path, "unknown binding state");
}

const OWNER_SUMMARY_KEYS = [
  "schemaVersion",
  "owner",
  "purpose",
  "ownerBindingRevision",
  "bindingState",
  "legacySourceCoverage",
] as const;

export function decodeSecretOwnerCredentialSummary(
  value: unknown,
  path = "SecretOwnerCredentialSummary",
): SecretOwnerCredentialSummary {
  const record = assertObject(value, path);
  assertAllowedKeys(record, OWNER_SUMMARY_KEYS, path);
  assertPublicNoValue(record, path);
  if (record.schemaVersion !== 1) {
    fail(`${path}.schemaVersion`, "expected 1");
  }
  if (record.purpose !== "codexApiKey") {
    fail(`${path}.purpose`, "expected codexApiKey");
  }
  return {
    schemaVersion: 1,
    owner: decodeOwner(record.owner, `${path}.owner`),
    purpose: "codexApiKey",
    ownerBindingRevision: record.ownerBindingRevision as never,
    bindingState: decodeBindingState(record.bindingState, `${path}.bindingState`),
    legacySourceCoverage: record.legacySourceCoverage as never,
  };
}

const REF_KEYS = [
  "schemaVersion",
  "secretRef",
  "secretRefDisplay",
  "purpose",
  "presence",
  "availability",
  "backend",
  "lock",
  "revocation",
  "issue",
  "createdAt",
] as const;

export function decodeSecretRefAggregate(
  value: unknown,
  path = "SecretRefAggregate",
): SecretRefAggregate {
  const record = assertObject(value, path);
  assertAllowedKeys(record, REF_KEYS, path);
  assertPublicNoValue(record, path);
  const availability = decodeAvailability(
    record.availability,
    `${path}.availability`,
  );
  if (record.lock) {
    const lock = assertObject(record.lock, `${path}.lock`);
    const source = assertString(lock.source, `${path}.lock.source`);
    if (!(SECRET_LOCK_SOURCES as readonly string[]).includes(source)) {
      fail(`${path}.lock.source`, "unknown lockSource");
    }
  }
  if (record.revocation) {
    const revocation = assertObject(record.revocation, `${path}.revocation`);
    const source = assertString(
      revocation.source,
      `${path}.revocation.source`,
    );
    if (!(SECRET_REVOCATION_SOURCES as readonly string[]).includes(source)) {
      fail(`${path}.revocation.source`, "unknown revocationSource");
    }
  }
  return {
    schemaVersion: 1,
    secretRef: decodeSecretRef(record.secretRef, `${path}.secretRef`),
    secretRefDisplay: decodeSecretRefDisplay(
      record.secretRefDisplay,
      `${path}.secretRefDisplay`,
    ),
    purpose: "codexApiKey",
    presence: record.presence as never,
    availability,
    backend: record.backend as never,
    lock: record.lock as never,
    revocation: record.revocation as never,
    issue: record.issue as never,
    createdAt: record.createdAt as never,
  };
}

const CANDIDATE_KEYS = [
  "schemaVersion",
  "candidateId",
  "candidateRevision",
  "kind",
  "comparisonPolicy",
  "comparisonImpact",
  "state",
  "secretRefDisplay",
  "purpose",
  "targetOwners",
  "legacySourceCounts",
  "createdAt",
  "expiresAt",
  "pendingTerminalDisposition",
  "issue",
] as const;

export function decodeSecretCandidateSummary(
  value: unknown,
  path = "SecretCandidateSummary",
): SecretCandidateSummary {
  const record = assertObject(value, path);
  assertAllowedKeys(record, CANDIDATE_KEYS, path);
  assertPublicNoValue(record, path);
  const candidateId = assertString(record.candidateId, `${path}.candidateId`);
  if (!CANDIDATE_ID_RE.test(candidateId)) {
    fail(`${path}.candidateId`, "invalid SecretCandidateId");
  }
  return record as unknown as SecretCandidateSummary;
}

export function decodeSecretDeleteImpact(
  value: unknown,
  path = "SecretDeleteImpact",
): SecretDeleteImpact {
  const record = assertObject(value, path);
  assertAllowedKeys(record, ["impact", "readiness"], path);
  assertPublicNoValue(record, path);
  const impact = assertObject(record.impact, `${path}.impact`);
  if (impact.noFallback !== true) {
    fail(`${path}.impact.noFallback`, "must be true");
  }
  return record as unknown as SecretDeleteImpact;
}

export function decodeSecretConfirmationRequirementView(
  value: unknown,
  path = "SecretConfirmationRequirementView",
): SecretConfirmationRequirementView {
  const record = assertObject(value, path);
  assertAllowedKeys(
    record,
    ["operation", "device", "timeoutSeconds", "promptKey"],
    path,
  );
  assertPublicNoValue(record, path);
  return record as unknown as SecretConfirmationRequirementView;
}

const SNAPSHOT_KEYS = [
  "schemaVersion",
  "owners",
  "refs",
  "candidates",
  "captureIntent",
  "registeredBackends",
  "secretDeleteImpact",
  "providerDeleteReady",
  "providerDeleteBlocked",
  "hardwareConfirmation",
  "ownerDisplayNames",
] as const;

export function decodeCredentialsSnapshot(
  value: unknown,
): CredentialsSnapshot {
  const record = assertObject(value, "CredentialsSnapshot");
  assertAllowedKeys(record, SNAPSHOT_KEYS, "CredentialsSnapshot");
  assertPublicNoValue(record, "CredentialsSnapshot");
  if (!Array.isArray(record.owners) || !Array.isArray(record.refs)) {
    fail("CredentialsSnapshot", "owners and refs must be arrays");
  }
  record.owners.forEach((owner, index) =>
    decodeSecretOwnerCredentialSummary(
      owner,
      `CredentialsSnapshot.owners[${index}]`,
    ),
  );
  record.refs.forEach((ref, index) =>
    decodeSecretRefAggregate(ref, `CredentialsSnapshot.refs[${index}]`),
  );
  if (Array.isArray(record.candidates)) {
    record.candidates.forEach((candidate, index) =>
      decodeSecretCandidateSummary(
        candidate,
        `CredentialsSnapshot.candidates[${index}]`,
      ),
    );
  }
  decodeSecretDeleteImpact(record.secretDeleteImpact);
  decodeSecretConfirmationRequirementView(record.hardwareConfirmation);
  const blocked = assertObject(
    record.providerDeleteBlocked,
    "CredentialsSnapshot.providerDeleteBlocked",
  );
  if (blocked.status !== "blockedLegacyResolutionRequired") {
    fail(
      "CredentialsSnapshot.providerDeleteBlocked.status",
      "expected blockedLegacyResolutionRequired",
    );
  }
  const blockedBody = assertObject(
    blocked.blocked,
    "CredentialsSnapshot.providerDeleteBlocked.blocked",
  );
  if ("providerDeleteImpactId" in blockedBody) {
    fail(
      "CredentialsSnapshot.providerDeleteBlocked.blocked",
      "blocked provider delete must not carry an impact id",
    );
  }
  return record as unknown as CredentialsSnapshot;
}
