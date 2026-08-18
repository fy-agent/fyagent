export type LayerState = "ok" | "warn" | "fail" | "unknown";
export type PreflightItemState = "pass" | "warn" | "fail" | "unknown";
export type InstallMode = "official_guide" | "package_manager" | "native_verified";

export interface SourceLayerState {
  agentId: string;
  sourceState: LayerState;
  officialLandingUrl: string | null;
  legalEntity: string | null;
  licenseUrl: string | null;
  licenseScope: string;
  packageSourceKind: string;
  cacheAllowed: boolean;
  redistributionAllowed: boolean | null;
  installMode: InstallMode;
  evidenceUrl: string | null;
  checkedAt: string;
  writtenPermissionNeeded: boolean;
}

export interface FactorState {
  state: LayerState;
  value: string | null;
}

export interface IntegrityLayerState {
  integrityState: LayerState;
  hash: FactorState;
  signature: FactorState;
  revocation: FactorState;
  verificationSource: string[];
  integritySummary: string;
  checkedAt: string;
}

export interface PreflightItem {
  code: string;
  state: PreflightItemState;
  message: string;
  hint: string;
  checkedAt: string;
  source: string;
}

export interface PreflightLayerState {
  preflightState: LayerState;
  checks: PreflightItem[];
  checkedAt: string;
}

export interface PlanLayerState {
  planSnapshotId: string | null;
  planHash: string | null;
  snapshotStale: boolean;
  driftReasons: string[];
  refreshedAt: string;
}

export interface InstallContract {
  schema: string;
  agentId: string;
  catalog: SourceLayerState;
  package: IntegrityLayerState;
  environment: PreflightLayerState;
  plan: PlanLayerState;
  updatedAt: string;
  installAllowed: boolean;
  guideAllowed: boolean;
}
