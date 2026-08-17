import { useEffect, useMemo, useRef, useState, type RefObject } from "react";

import {
  AVAILABILITY_LABELS_ZH,
  BACKEND_OPERATION_LABELS_ZH,
  BINDING_STATE_LABELS_ZH,
  CANDIDATE_KIND_LABELS_ZH,
  CANDIDATE_PLAN_BANNER,
  CAPTURE_WAITING_COPY,
  COMPARISON_MEANING_ZH,
  EMPTY_CREDENTIALS_COPY,
  LEGACY_WARNING_COPY,
  MISSING_STATUS_COPY,
  NO_FALLBACK_COPY,
  PENDING_BACKEND_REACHABLE_COPY,
  PROVIDER_RETAINED_COPY,
  REVOCATION_SOURCE_LABELS_ZH,
  SECRET_USER_ACTION_LABELS_ZH,
  SEPARATE_SECRET_DELETE_COPY,
  UNBOUND_STATUS_COPY,
  credentialBrowserFixtures,
  type CredentialsSnapshot,
  type SecretStableAvailability,
  type SecretUserAction,
} from "@/v2/shared/data/credentials";

import {
  buildCredentialRows,
  nextActionLabel,
  type CredentialListRow,
} from "./prototype";
import "./credentials.css";

type OverlayKind = "capture-options" | "capture-waiting" | "hardware" | null;
type ImpactKind = "secret" | "provider" | "provider-blocked" | null;

export interface CredentialsPanelProps {
  snapshot?: CredentialsSnapshot;
  empty?: boolean;
  initialOwnerId?: string;
  initialOverlay?: OverlayKind;
  initialImpact?: ImpactKind;
}


function isStagedCandidateRow(row: CredentialListRow): boolean {
  const state = row.candidate?.state;
  return state === "verifiedPendingPlan" || state === "expired";
}

function lockSourceOf(row: CredentialListRow) {
  return row.aggregate?.lock?.source ?? row.aggregate?.issue?.lockSource;
}

function availabilityGlyph(availability: SecretStableAvailability): string {
  switch (availability) {
    case "ready":
      return "●";
    case "missing":
      return "○";
    case "locked":
      return "▣";
    case "denied":
      return "⊘";
    case "stale":
      return "◐";
    case "revoked":
      return "✕";
    case "unavailable":
      return "–";
  }
}

function AvailabilityTriple({
  availability,
}: {
  availability: SecretStableAvailability;
}) {
  return (
    <span
      className="fy-credentials-availability"
      data-availability={availability}
    >
      <i className="fy-credentials-availability-icon" aria-hidden>
        {availabilityGlyph(availability)}
      </i>
      <span>{AVAILABILITY_LABELS_ZH[availability]}</span>
    </span>
  );
}

export function CredentialsPanel({
  snapshot = credentialBrowserFixtures,
  empty = false,
  initialOwnerId,
  initialOverlay = null,
  initialImpact = null,
}: CredentialsPanelProps) {
  const workspace = empty
    ? { ...snapshot, owners: [], refs: [], candidates: [] }
    : snapshot;
  const rows = useMemo(() => buildCredentialRows(workspace), [workspace]);
  const [selectedOwnerId, setSelectedOwnerId] = useState(
    initialOwnerId ?? rows[0]?.ownerId ?? "",
  );
  const [overlay, setOverlay] = useState<OverlayKind>(initialOverlay);
  const [impactKind, setImpactKind] = useState<ImpactKind>(initialImpact);
  const captureButtonRef = useRef<HTMLButtonElement>(null);
  const cancelButtonRef = useRef<HTMLButtonElement>(null);

  const selected = rows.find((row) => row.ownerId === selectedOwnerId) ?? rows[0];
  const hasLegacy = rows.some((row) => row.summary.bindingState.state === "legacy");
  const showCandidate = Boolean(selected?.candidate);

  useEffect(() => {
    if (impactKind && cancelButtonRef.current) {
      cancelButtonRef.current.focus();
    }
  }, [impactKind]);

  const openCapture = () => {
    setOverlay("capture-options");
  };

  const cancelOverlay = () => {
    setOverlay(null);
    captureButtonRef.current?.focus();
  };

  return (
    <section
      className="fy-credentials-page"
      aria-labelledby="fy-credentials-title"
      data-testid="credentials-panel"
      data-data-source="prototype"
    >
      <header className="fy-credentials-header">
        <div className="fy-credentials-title-group">
          <h1 id="fy-credentials-title">凭据</h1>
          <p>本机引用与无值状态 · 不显示密钥</p>
        </div>
        <button
          ref={captureButtonRef}
          className="fy-credentials-primary-action"
          type="button"
          onClick={openCapture}
        >
          采集凭据
        </button>
      </header>

      {rows.length === 0 ? (
        <div className="fy-credentials-empty" data-testid="credentials-empty">
          <p>{EMPTY_CREDENTIALS_COPY}</p>
          <button
            className="fy-credentials-primary-action"
            type="button"
            onClick={openCapture}
          >
            采集凭据
          </button>
        </div>
      ) : (
        <div className="fy-credentials-grid">
          <section
            className="fy-credentials-pane fy-credentials-list-pane"
            data-testid="credentials-list"
          >
            <h2>Provider</h2>
            {hasLegacy ? (
              <p className="fy-credentials-warning">{LEGACY_WARNING_COPY}</p>
            ) : null}
            <ul className="fy-credentials-list" role="listbox" aria-label="凭据列表">
              {rows.map((row) => (
                <li key={row.ownerId}>
                  <button
                    className="fy-credentials-row"
                    type="button"
                    role="option"
                    aria-selected={row.ownerId === selected?.ownerId}
                    data-selected={row.ownerId === selected?.ownerId ? "true" : "false"}
                    data-owner-id={row.ownerId}
                    data-binding-state={row.summary.bindingState.state}
                    data-availability={row.aggregate?.availability ?? "unavailable"}
                    data-staged-plan={isStagedCandidateRow(row) ? "true" : "false"}
                    data-next-action={row.nextAction}
                    onClick={() => setSelectedOwnerId(row.ownerId)}
                  >
                    <strong className="fy-credentials-row-name">{row.displayName}</strong>
                    <span className="fy-credentials-chip">
                      {isStagedCandidateRow(row)
                        ? "等待变更计划"
                        : BINDING_STATE_LABELS_ZH[row.summary.bindingState.state]}
                    </span>
                    {isStagedCandidateRow(row) ? null : row.aggregate ? (
                      <AvailabilityTriple availability={row.aggregate.availability} />
                    ) : (
                      <span className="fy-credentials-next">
                        {row.summary.bindingState.state === "legacy"
                          ? "明文"
                          : "—"}
                      </span>
                    )}
                    <span className="fy-credentials-next">
                      {nextActionLabel(row.nextAction)}
                    </span>
                  </button>
                </li>
              ))}
            </ul>
          </section>

          {selected && showCandidate && selected.candidate ? (
            <CandidatePlan
              row={selected}
              snapshot={workspace}
            />
          ) : selected ? (
            <StatusCard
              row={selected}
              snapshot={workspace}
              onCapture={openCapture}
              onSecretDelete={() => setImpactKind("secret")}
              onProviderDelete={() =>
                setImpactKind(
                  selected.summary.bindingState.state === "legacy"
                    ? "provider-blocked"
                    : "provider",
                )
              }
            />
          ) : null}
        </div>
      )}

      {impactKind ? (
        <ImpactDialog
          kind={impactKind}
          snapshot={workspace}
          cancelRef={cancelButtonRef}
          onClose={() => setImpactKind(null)}
          onSecretDelete={() => setImpactKind("secret")}
        />
      ) : null}

      {overlay ? (
        <CaptureOrHardwareOverlay
          kind={overlay}
          snapshot={workspace}
          onSelectBackend={() => setOverlay("capture-waiting")}
          onCancel={cancelOverlay}
        />
      ) : null}
    </section>
  );
}

function StatusCard({
  row,
  snapshot,
  onCapture,
  onSecretDelete,
  onProviderDelete,
}: {
  row: CredentialListRow;
  snapshot: CredentialsSnapshot;
  onCapture: () => void;
  onSecretDelete: () => void;
  onProviderDelete: () => void;
}) {
  const binding = row.summary.bindingState;
  const availability = row.aggregate?.availability;
  const isUnbound = binding.state === "unbound";
  const isLocked = availability === "locked";
  const isMissing = availability === "missing";
  const isRevoked = availability === "revoked";
  const isUnavailable = availability === "unavailable";
  const isReady = availability === "ready";

  return (
    <section
      className="fy-credentials-pane fy-credentials-status"
      data-testid="credentials-status"
      data-binding-state={binding.state}
      data-availability={availability ?? "none"}
      data-lock-source={lockSourceOf(row) ?? ""}
    >
      <h2>{row.displayName}</h2>
      <div className="fy-credentials-chip-row">
        <span className="fy-credentials-chip">
          {BINDING_STATE_LABELS_ZH[binding.state]}
        </span>
        {availability ? <AvailabilityTriple availability={availability} /> : null}
        {row.aggregate ? (
          <span className="fy-credentials-chip">
            {row.aggregate.presence === "present"
              ? "在场"
              : row.aggregate.presence === "missing"
                ? "未在场"
                : "未知"}
          </span>
        ) : null}
      </div>

      {binding.state === "bound" ? (
        <code className="fy-credentials-ref" data-testid="secret-ref-display">
          {binding.secretRefDisplay}
        </code>
      ) : null}

      {row.aggregate?.backend ? (
        <p className="fy-credentials-meta">
          {row.aggregate.backend.kind === "osKeyring" ? "本机钥匙串" : "硬件"}
          {row.aggregate.backend.device
            ? ` · ${row.aggregate.backend.device.displayName}`
            : ""}
        </p>
      ) : null}

      {isUnbound ? <p className="fy-credentials-issue">{UNBOUND_STATUS_COPY}</p> : null}
      {isMissing ? <p className="fy-credentials-issue">{MISSING_STATUS_COPY}</p> : null}
      {isRevoked && row.aggregate?.revocation ? (
        <p className="fy-credentials-issue" data-revocation-source={row.aggregate.revocation.source}>
          撤销来源：{REVOCATION_SOURCE_LABELS_ZH[row.aggregate.revocation.source]}
        </p>
      ) : null}
      {isUnavailable &&
      row.aggregate?.issue?.backendUnavailableReason === "hardwareUnregistered" ? (
        <p className="fy-credentials-issue">重新连接设备 / 打开后端设置 · 不回退本机钥匙串</p>
      ) : null}
      {row.aggregate?.issue && !isMissing && !isRevoked ? (
        <p className="fy-credentials-issue">{row.aggregate.issue.code}</p>
      ) : null}

      <div className="fy-credentials-actions">
        {isUnbound || isMissing ? (
          <button className="fy-credentials-primary-action" type="button" onClick={onCapture}>
            采集凭据
          </button>
        ) : null}
        {isLocked && lockSourceOf(row) === "fyAgentPolicy" ? (
          <button
            className="fy-credentials-primary-action"
            type="button"
            data-lock-action="unlockFyAgent"
            data-lock-source="fyAgentPolicy"
          >
            {SECRET_USER_ACTION_LABELS_ZH.unlockFyAgent}
          </button>
        ) : null}
        {isLocked && lockSourceOf(row) === "backend" ? (
          <button
            className="fy-credentials-primary-action"
            type="button"
            data-lock-action="unlockBackend"
            data-lock-source="backend"
          >
            {SECRET_USER_ACTION_LABELS_ZH.unlockBackend}
          </button>
        ) : null}
        {isUnavailable ? (
          <>
            <button className="fy-credentials-primary-action" type="button">
              {SECRET_USER_ACTION_LABELS_ZH.reconnectDevice}
            </button>
            <button className="fy-credentials-secondary-action" type="button">
              {SECRET_USER_ACTION_LABELS_ZH.openBackendSettings}
            </button>
          </>
        ) : null}
        {availability === "stale" || availability === "denied" ? (
          <button className="fy-credentials-primary-action" type="button">
            {nextActionLabel(row.nextAction)}
          </button>
        ) : null}
      </div>

      {isReady ? (
        <div className="fy-credentials-secondary-actions">
          <button className="fy-credentials-secondary-action" type="button">
            轮换
          </button>
          <button className="fy-credentials-secondary-action" type="button">
            锁定
          </button>
          <button className="fy-credentials-secondary-action" type="button" onClick={onSecretDelete}>
            删除本机凭据
          </button>
          <button className="fy-credentials-secondary-action" type="button" onClick={onProviderDelete}>
            删除 Provider
          </button>
        </div>
      ) : null}

      {binding.state === "legacy" ? (
        <div className="fy-credentials-secondary-actions">
          <button className="fy-credentials-primary-action" type="button" onClick={onCapture}>
            {SECRET_USER_ACTION_LABELS_ZH.resolveLegacyConflict}
          </button>
          <button className="fy-credentials-secondary-action" type="button" onClick={onProviderDelete}>
            删除 Provider
          </button>
        </div>
      ) : null}

      <p className="fy-visually-hidden">
        稳定摘要不含 confirmation step、operation id 或 capability。硬件确认见叠层。
        引用 {snapshot.schemaVersion}
      </p>
    </section>
  );
}

function CandidatePlan({
  row,
  snapshot,
}: {
  row: CredentialListRow;
  snapshot: CredentialsSnapshot;
}) {
  const candidate = row.candidate;
  if (!candidate) {
    return null;
  }
  const pending = candidate.pendingTerminalDisposition;
  const expired = candidate.state === "expired";
  const cleanup = candidate.state === "cleanupRequired";
  const primaryAction: SecretUserAction = expired
    ? "refreshSummary"
    : cleanup
      ? "completeRecovery"
      : pending
        ? "discardCandidate"
        : "reopenChangePlan";

  return (
    <section
      className="fy-credentials-pane fy-credentials-candidate"
      data-testid="credentials-candidate"
      data-candidate-state={candidate.state}
      data-pending-disposition={pending ?? ""}
    >
      <p className="fy-credentials-banner">{CANDIDATE_PLAN_BANNER}</p>
      <h2>{row.displayName}</h2>
      <p className="fy-credentials-meta">
        <span>{CANDIDATE_KIND_LABELS_ZH[candidate.kind]}</span>
        <span> · </span>
        <span>{COMPARISON_MEANING_ZH[candidate.comparisonPolicy]}</span>
      </p>
      <ul className="fy-credentials-owner-list" aria-label="受影响 Owner">
        {candidate.targetOwners.map((owner) => (
          <li key={owner.ownerId}>
            {snapshot.ownerDisplayNames[owner.ownerId] ?? owner.ownerId}
          </li>
        ))}
      </ul>
      <div className="fy-credentials-chip-row">
        {candidate.legacySourceCounts.map((item) => (
          <span className="fy-credentials-chip" key={item.category}>
            {item.category} · {item.count}
          </span>
        ))}
      </div>
      <p className="fy-credentials-meta">到期 {candidate.expiresAt}</p>
      {pending ? (
        <p className="fy-credentials-issue">
          {pending === "discarded" ? "丢弃未完成，条目仍在" : "过期清理未完成，条目仍在"}
          · {PENDING_BACKEND_REACHABLE_COPY}
        </p>
      ) : null}
      {cleanup ? (
        <p className="fy-credentials-issue">绑定已切，清理未完，consumer 当前 fail closed</p>
      ) : null}
      {expired ? (
        <p className="fy-credentials-issue">已过期，请刷新后再采集</p>
      ) : null}
      <div className="fy-credentials-actions">
        <button className="fy-credentials-primary-action" type="button">
          {SECRET_USER_ACTION_LABELS_ZH[primaryAction]}
        </button>
        {!pending && !expired && !cleanup ? (
          <button className="fy-credentials-secondary-action" type="button">
            {SECRET_USER_ACTION_LABELS_ZH.discardCandidate}
          </button>
        ) : null}
      </div>
    </section>
  );
}

function ImpactDialog({
  kind,
  snapshot,
  cancelRef,
  onClose,
  onSecretDelete,
}: {
  kind: Exclude<ImpactKind, null>;
  snapshot: CredentialsSnapshot;
  cancelRef: RefObject<HTMLButtonElement | null>;
  onClose: () => void;
  onSecretDelete: () => void;
}) {
  const blocked = kind === "provider-blocked";
  const secret = kind === "secret";
  const title = secret ? "删除本机凭据" : "删除 Provider";
  const confirmLabel = secret ? "删除本机凭据" : "卸下 Provider";

  return (
    <div className="fy-credentials-overlay" data-testid="credentials-impact">
      <div
        className="fy-credentials-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="fy-credentials-impact-title"
        data-impact-kind={kind}
      >
        <h2 id="fy-credentials-impact-title">{title}</h2>
        {secret ? (
          <>
            <ul className="fy-credentials-owner-list" aria-label="受影响 Owner">
              {snapshot.secretDeleteImpact.impact.affectedOwners.map((item) => (
                <li key={item.owner.ownerId}>
                  {snapshot.ownerDisplayNames[item.owner.ownerId] ?? item.owner.ownerId}
                </li>
              ))}
            </ul>
            <p className="fy-credentials-issue">{NO_FALLBACK_COPY}</p>
          </>
        ) : null}
        {kind === "provider" ? (
          <>
            <p className="fy-credentials-issue">{PROVIDER_RETAINED_COPY}</p>
            <ul className="fy-credentials-owner-list" aria-label="剩余 Owner">
              {snapshot.providerDeleteReady.impact.existingBinding.remainingOwners.map(
                (item) => (
                  <li key={item.ownerId}>
                    {snapshot.ownerDisplayNames[item.ownerId] ?? item.ownerId}
                  </li>
                ),
              )}
            </ul>
            <p className="fy-credentials-meta">
              孤儿：{snapshot.providerDeleteReady.impact.existingBinding.becomesOrphan ? "是" : "否"}
            </p>
            <button
              className="fy-credentials-secondary-action"
              type="button"
              onClick={onSecretDelete}
            >
              {SEPARATE_SECRET_DELETE_COPY}
            </button>
          </>
        ) : null}
        {blocked ? (
          <>
            <p className="fy-credentials-warning">{LEGACY_WARNING_COPY}</p>
            <p className="fy-credentials-meta">
              明文来源 {snapshot.providerDeleteBlocked.blocked.legacySourceCoverage.currentScrubbable.sourceCount} 类
            </p>
            <button className="fy-credentials-primary-action" type="button">
              {SECRET_USER_ACTION_LABELS_ZH.resolveLegacyConflict}
            </button>
          </>
        ) : null}
        <div className="fy-credentials-dialog-actions">
          <button
            ref={cancelRef}
            className="fy-credentials-ghost-action"
            type="button"
            onClick={onClose}
          >
            取消
          </button>
          {blocked ? null : (
            <button className="fy-credentials-danger-action" type="button">
              {confirmLabel}
            </button>
          )}
        </div>
      </div>
    </div>
  );
}

function CaptureOrHardwareOverlay({
  kind,
  snapshot,
  onSelectBackend,
  onCancel,
}: {
  kind: Exclude<OverlayKind, null>;
  snapshot: CredentialsSnapshot;
  onSelectBackend: () => void;
  onCancel: () => void;
}) {
  if (kind === "hardware") {
    const confirmation = snapshot.hardwareConfirmation;
    return (
      <div className="fy-credentials-overlay" data-testid="credentials-hardware-overlay">
        <div className="fy-credentials-dialog" role="dialog" aria-modal="true">
          <h2>硬件确认</h2>
          <p className="fy-credentials-meta">{confirmation.device.displayName}</p>
          <p className="fy-credentials-meta">
            {BACKEND_OPERATION_LABELS_ZH[confirmation.operation]}
          </p>
          <p className="fy-credentials-meta">{confirmation.timeoutSeconds} 秒</p>
          <button className="fy-credentials-ghost-action" type="button" onClick={onCancel}>
            取消
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="fy-credentials-overlay" data-testid="credentials-capture-overlay">
      <div className="fy-credentials-dialog" role="dialog" aria-modal="true">
        {kind === "capture-waiting" ? (
          <p>{CAPTURE_WAITING_COPY}</p>
        ) : (
          <>
            <h2>选择已注册后端</h2>
            {snapshot.registeredBackends.map((option) => (
              <button
                key={option.backend.instanceId}
                className="fy-credentials-backend-option"
                type="button"
                onClick={onSelectBackend}
              >
                {option.backend.device?.displayName ?? option.backend.kind}
              </button>
            ))}
          </>
        )}
        <button className="fy-credentials-ghost-action" type="button" onClick={onCancel}>
          取消
        </button>
      </div>
    </div>
  );
}
