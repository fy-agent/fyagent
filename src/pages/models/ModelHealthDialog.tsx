import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useMemo, useState } from "react";

import {
  BATCH_CONFIGURABLE_TARGETS,
  type SaveCustomGatewayTemplateRequest,
} from "../../domain/configuration/presets/desktopPresetTemplates";
import { useFeatures } from "../../shared/features/provider";
import { featureKeys } from "../../shared/features/queries";
import type {
  BatchProviderPreviewItem,
  BatchSetupTargetId,
  BatchTargetOutcome,
  SavedModelProbeResult,
  TargetModelSnapshot,
} from "../../shared/features/types";
import { Button } from "../../shared/ui/Button";
import { Dialog } from "../../shared/ui/Dialog";
import type { DialogOriginRef } from "../../shared/ui/dialogOrigin";
import {
  Badge,
  Checkbox,
  InlineNotice,
  Input,
  Spinner,
} from "../../shared/ui/primitives";

const TARGET_LABEL_MAP: Readonly<Record<BatchSetupTargetId, string>> =
  Object.fromEntries(
    BATCH_CONFIGURABLE_TARGETS.map((item) => [item.id, item.label]),
  ) as Record<BatchSetupTargetId, string>;

const PROBE_CONCURRENCY = 3;

type DialogView = "overview" | "clone" | "rotate";

interface ProbeEntry {
  status: "running" | "done" | "error";
  result?: SavedModelProbeResult;
  error?: string;
}

function credentialBadge(snapshot: TargetModelSnapshot): {
  label: string;
  tone: "neutral" | "accent" | "warning";
} {
  if (!snapshot.configured) return { label: "未配置", tone: "neutral" };
  if (snapshot.credentialSource === "oauth")
    return { label: "官方订阅", tone: "neutral" };
  if (snapshot.credentialSource === "env_or_command")
    return { label: "环境变量 / 命令", tone: "neutral" };
  if (snapshot.credentialSource === "api_key")
    return { label: "已配置", tone: "accent" };
  return { label: "无密钥", tone: "warning" };
}

function formatProbeSummary(entry: ProbeEntry | undefined): {
  tone: "neutral" | "accent" | "warning";
  text: string;
} | null {
  if (!entry) return null;
  if (entry.status === "running") {
    return { tone: "neutral", text: "测速中…" };
  }
  if (entry.status === "error") {
    return { tone: "warning", text: entry.error || "测速失败" };
  }
  const result = entry.result;
  if (!result) return null;
  if (result.ok) {
    return {
      tone: "accent",
      text:
        result.latencyMs !== null ? `可用 · ${result.latencyMs} ms` : "可用",
    };
  }
  const shortLabel =
    result.category === "invalid_key"
      ? "Key 无效"
      : result.category === "quota_exhausted"
        ? "额度不足"
        : result.category === "endpoint_or_model_mismatch"
          ? "模型或路径不匹配"
          : "网络异常";
  return {
    tone: "warning",
    text:
      result.httpStatus !== null
        ? `${shortLabel} (${result.httpStatus})`
        : shortLabel,
  };
}

export function ModelHealthDialog({
  open,
  originRef,
  onOpenChange,
  onSaveAsTemplate,
}: {
  open: boolean;
  originRef?: DialogOriginRef;
  onOpenChange: (open: boolean) => void;
  onSaveAsTemplate: (draft: Partial<SaveCustomGatewayTemplateRequest>) => void;
}) {
  const { ports } = useFeatures();
  const queryClient = useQueryClient();

  const overviewQuery = useQuery({
    queryKey: featureKeys.modelHealthOverview,
    queryFn: () => ports.providers.getModelHealthOverview(),
    enabled: open,
  });

  const [view, setView] = useState<DialogView>("overview");
  const [probeMap, setProbeMap] = useState<
    Partial<Record<BatchSetupTargetId, ProbeEntry>>
  >({});
  const [probingAll, setProbingAll] = useState(false);

  // Clone state
  const [cloneSource, setCloneSource] = useState<BatchSetupTargetId | null>(
    null,
  );
  const [cloneDestinations, setCloneDestinations] = useState<
    BatchSetupTargetId[]
  >([]);
  const [cloneModelOverride, setCloneModelOverride] = useState("");
  const [cloneClaudeUrlOverride, setCloneClaudeUrlOverride] = useState("");

  // Rotate key state
  const [rotateTargets, setRotateTargets] = useState<BatchSetupTargetId[]>([]);
  const [rotateApiKey, setRotateApiKey] = useState("");

  // Shared preview / apply feedback
  const [previewItems, setPreviewItems] = useState<
    BatchProviderPreviewItem[] | null
  >(null);
  const [outcomes, setOutcomes] = useState<BatchTargetOutcome[] | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const targets = useMemo(
    () => overviewQuery.data?.targets ?? [],
    [overviewQuery.data?.targets],
  );
  const probeableTargets = useMemo(
    () => targets.filter((item) => item.canProbe && Boolean(item.modelId)),
    [targets],
  );
  const rotatableTargets = useMemo(
    () => targets.filter((item) => item.canRotateKey),
    [targets],
  );
  const activeCloneSnapshot = useMemo(
    () => targets.find((item) => item.target === cloneSource) ?? null,
    [targets, cloneSource],
  );

  const resetActionState = () => {
    setPreviewItems(null);
    setOutcomes(null);
    setError(null);
  };

  const invalidateModelQueries = async () => {
    await Promise.all([
      queryClient.invalidateQueries({
        queryKey: featureKeys.modelHealthOverview,
      }),
      queryClient.invalidateQueries({ queryKey: featureKeys.workbuddyStatus }),
      queryClient.invalidateQueries({
        queryKey: featureKeys.workbuddyModelIds,
      }),
      queryClient.invalidateQueries({
        queryKey: featureKeys.openCodeModelSnapshot,
      }),
      queryClient.invalidateQueries({ queryKey: featureKeys.ompConfig }),
      queryClient.invalidateQueries({
        queryKey: featureKeys.providerSummary("codex"),
      }),
      queryClient.invalidateQueries({
        queryKey: featureKeys.providerSummary("claude"),
      }),
      queryClient.invalidateQueries({
        queryKey: featureKeys.providerSummary("grokbuild"),
      }),
      queryClient.invalidateQueries({
        queryKey: featureKeys.codexReloadStatus,
      }),
    ]);
  };

  const probeSingle = async (snapshot: TargetModelSnapshot) => {
    if (!snapshot.canProbe || !snapshot.modelId) return;
    setProbeMap((prev) => ({
      ...prev,
      [snapshot.target]: { status: "running" },
    }));
    try {
      const result = await ports.providers.probeSavedModel({
        target: snapshot.target,
        providerId: snapshot.providerId,
        modelId: snapshot.modelId,
      });
      setProbeMap((prev) => ({
        ...prev,
        [snapshot.target]: { status: "done", result },
      }));
    } catch (err) {
      setProbeMap((prev) => ({
        ...prev,
        [snapshot.target]: {
          status: "error",
          error: err instanceof Error ? err.message : "测速请求失败",
        },
      }));
    }
  };

  const handleProbeAll = async () => {
    if (probingAll || probeableTargets.length === 0) return;
    setProbingAll(true);
    try {
      const queue = [...probeableTargets];
      const workers = Array.from(
        { length: Math.min(PROBE_CONCURRENCY, queue.length) },
        async () => {
          while (queue.length > 0) {
            const next = queue.shift();
            if (next) await probeSingle(next);
          }
        },
      );
      await Promise.all(workers);
    } finally {
      setProbingAll(false);
    }
  };

  const openCloneFlow = (source: TargetModelSnapshot) => {
    resetActionState();
    setCloneSource(source.target);
    const unconfiguredOthers = targets
      .filter(
        (item) =>
          item.target !== source.target &&
          !item.configured &&
          item.target !== "grokbuild",
      )
      .map((item) => item.target);
    const fallbackOthers = BATCH_CONFIGURABLE_TARGETS.filter(
      (item) => item.id !== source.target && item.defaultChecked,
    ).map((item) => item.id);
    setCloneDestinations(
      unconfiguredOthers.length > 0 ? unconfiguredOthers : fallbackOthers,
    );
    setCloneModelOverride("");
    setCloneClaudeUrlOverride("");
    setView("clone");
  };

  const openRotateFlow = (anchor?: TargetModelSnapshot) => {
    resetActionState();
    setRotateApiKey("");
    if (anchor?.endpointHost) {
      const sameHost = rotatableTargets
        .filter((item) => item.endpointHost === anchor.endpointHost)
        .map((item) => item.target);
      setRotateTargets(sameHost.length > 0 ? sameHost : [anchor.target]);
    } else if (anchor) {
      setRotateTargets([anchor.target]);
    } else {
      setRotateTargets(rotatableTargets.map((item) => item.target));
    }
    setView("rotate");
  };

  const buildClonePayload = (overrideTargets?: BatchSetupTargetId[]) => {
    if (!cloneSource) return null;
    const destinationTargets = overrideTargets ?? cloneDestinations;
    const baseUrlOverrides: Partial<Record<BatchSetupTargetId, string>> = {};
    if (
      cloneSource !== "claude" &&
      destinationTargets.includes("claude") &&
      cloneClaudeUrlOverride.trim()
    ) {
      baseUrlOverrides.claude = cloneClaudeUrlOverride.trim();
    }
    return {
      sourceTarget: cloneSource,
      destinationTargets,
      ...(cloneModelOverride.trim()
        ? { modelIdOverride: cloneModelOverride.trim() }
        : {}),
      ...(Object.keys(baseUrlOverrides).length > 0 ? { baseUrlOverrides } : {}),
    };
  };

  const handlePreviewClone = async () => {
    const payload = buildClonePayload();
    if (!payload || payload.destinationTargets.length === 0) {
      setError("请至少勾选 1 个目标应用");
      return;
    }
    setBusy(true);
    setError(null);
    setOutcomes(null);
    try {
      const result = await ports.providers.previewCloneModelConfig(payload);
      setPreviewItems(result.items);
    } catch (err) {
      setError(err instanceof Error ? err.message : "无法生成克隆预览");
    } finally {
      setBusy(false);
    }
  };

  const handleApplyClone = async (retryTargets?: BatchSetupTargetId[]) => {
    const targetsToApply =
      retryTargets ??
      (previewItems
        ? previewItems.filter((item) => item.ready).map((item) => item.target)
        : cloneDestinations);
    const payload = buildClonePayload(targetsToApply);
    if (!payload || payload.destinationTargets.length === 0) {
      setError("没有可写入的目标应用");
      return;
    }
    setBusy(true);
    setError(null);
    try {
      const res = await ports.providers.applyCloneModelConfig(payload);
      setOutcomes(res.outcomes);
      await invalidateModelQueries();
    } catch (err) {
      setError(err instanceof Error ? err.message : "克隆写入失败");
    } finally {
      setBusy(false);
    }
  };

  const handlePreviewRotate = async () => {
    if (!rotateApiKey.trim()) {
      setError("请先粘贴新的 API Key");
      return;
    }
    if (rotateTargets.length === 0) {
      setError("请至少勾选 1 个要轮换 Key 的应用");
      return;
    }
    setBusy(true);
    setError(null);
    setOutcomes(null);
    try {
      const result = await ports.providers.previewApiKeyRotation({
        targets: rotateTargets,
      });
      setPreviewItems(result.items);
    } catch (err) {
      setError(err instanceof Error ? err.message : "无法生成轮换预览");
    } finally {
      setBusy(false);
    }
  };

  const handleApplyRotate = async (retryTargets?: BatchSetupTargetId[]) => {
    const trimmedKey = rotateApiKey.trim();
    if (!trimmedKey) {
      setError("请先粘贴新的 API Key");
      return;
    }
    const targetsToApply =
      retryTargets ??
      (previewItems
        ? previewItems.filter((item) => item.ready).map((item) => item.target)
        : rotateTargets);
    if (targetsToApply.length === 0) {
      setError("没有可轮换的目标应用");
      return;
    }
    setBusy(true);
    setError(null);
    try {
      const res = await ports.providers.applyApiKeyRotation({
        apiKey: trimmedKey,
        targets: targetsToApply,
      });
      setOutcomes(res.outcomes);
      if (res.outcomes.every((o) => o.status === "applied")) {
        setRotateApiKey("");
      }
      await invalidateModelQueries();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Key 轮换未能完成");
    } finally {
      setBusy(false);
    }
  };

  const handleClose = () => {
    setRotateApiKey("");
    setView("overview");
    resetActionState();
    onOpenChange(false);
  };

  const failedTargets =
    outcomes?.filter((o) => o.status === "failed").map((o) => o.target) ?? [];

  const title =
    view === "clone"
      ? `从 ${cloneSource ? TARGET_LABEL_MAP[cloneSource] : ""} 克隆配置`
      : view === "rotate"
        ? "API Key 一键轮换"
        : "全端模型健康体检与克隆";

  const description =
    view === "clone"
      ? "直接复用本地已保存的服务地址、模型与 API Key，同步到其他 AI 软件。"
      : view === "rotate"
        ? "输入新 API Key，批量更新所选软件的密钥并保留各自原有模型与接口地址。"
        : "集中查看 6 款可配软件当前模型状态，支持一键全端测速、跨端克隆与 Key 轮换。";

  return (
    <Dialog
      open={open}
      size="wide"
      originRef={originRef}
      onOpenChange={(next) => {
        if (!next) {
          setRotateApiKey("");
          setView("overview");
          resetActionState();
        }
        onOpenChange(next);
      }}
      title={title}
      description={description}
      actions={
        view === "overview" ? (
          <>
            <Button onClick={handleClose}>关闭</Button>
            {rotatableTargets.length > 0 ? (
              <Button disabled={probingAll} onClick={() => openRotateFlow()}>
                Key 一键轮换
              </Button>
            ) : null}
            <Button
              disabled={overviewQuery.isFetching || probingAll}
              onClick={() => void overviewQuery.refetch()}
            >
              刷新状态
            </Button>
            <Button
              className="fy-control-button-primary"
              disabled={
                overviewQuery.isPending ||
                probingAll ||
                probeableTargets.length === 0
              }
              onClick={() => void handleProbeAll()}
            >
              {probingAll
                ? "正在全端测速…"
                : `一键全端测速 (${probeableTargets.length})`}
            </Button>
          </>
        ) : (
          <>
            <Button
              disabled={busy}
              onClick={() => {
                setRotateApiKey("");
                setView("overview");
                resetActionState();
              }}
            >
              返回总览
            </Button>
            {outcomes && failedTargets.length > 0 ? (
              <Button
                className="fy-control-button-primary"
                disabled={busy}
                onClick={() =>
                  void (view === "clone"
                    ? handleApplyClone(failedTargets)
                    : handleApplyRotate(failedTargets))
                }
              >
                重试失败项 ({failedTargets.length})
              </Button>
            ) : previewItems ? (
              <>
                <Button disabled={busy} onClick={resetActionState}>
                  返回修改
                </Button>
                <Button
                  className="fy-control-button-primary"
                  disabled={busy}
                  onClick={() =>
                    void (view === "clone"
                      ? handleApplyClone()
                      : handleApplyRotate())
                  }
                >
                  {busy
                    ? "正在写入…"
                    : view === "clone"
                      ? "确认克隆写入"
                      : "确认一键轮换"}
                </Button>
              </>
            ) : (
              <Button
                className="fy-control-button-primary"
                disabled={busy}
                onClick={() =>
                  void (view === "clone"
                    ? handlePreviewClone()
                    : handlePreviewRotate())
                }
              >
                {busy
                  ? "正在检查…"
                  : view === "clone"
                    ? "预览克隆范围"
                    : "预览轮换范围"}
              </Button>
            )}
          </>
        )
      }
    >
      {view === "overview" ? (
        <div className="fy-model-health-body">
          {overviewQuery.isPending ? (
            <div className="fy-models-loading">
              <Spinner label="正在读取全端模型状态" />
              <span>正在读取全端配置…</span>
            </div>
          ) : overviewQuery.isError ? (
            <InlineNotice tone="error">
              无法读取全端模型状态，请刷新后重试。
            </InlineNotice>
          ) : (
            <ul
              className="fy-model-health-list"
              aria-label="各软件模型状态总览"
            >
              {targets.map((snapshot) => {
                const badge = credentialBadge(snapshot);
                const probePill = formatProbeSummary(probeMap[snapshot.target]);
                const rowProbing =
                  probeMap[snapshot.target]?.status === "running";
                return (
                  <li
                    key={snapshot.target}
                    className="fy-model-health-row"
                    data-testid={`health-row-${snapshot.target}`}
                  >
                    <div className="fy-model-health-main">
                      <div className="fy-model-health-title">
                        <strong>{TARGET_LABEL_MAP[snapshot.target]}</strong>
                        <Badge tone={badge.tone}>{badge.label}</Badge>
                        {probePill ? (
                          <Badge tone={probePill.tone}>{probePill.text}</Badge>
                        ) : null}
                      </div>
                      <p className="fy-model-health-meta">
                        {snapshot.configured ? (
                          <>
                            {snapshot.modelId ? (
                              <code>{snapshot.modelId}</code>
                            ) : (
                              <span>{snapshot.providerName || "已启用"}</span>
                            )}
                            {snapshot.endpointHost ? (
                              <span> · {snapshot.endpointHost}</span>
                            ) : null}
                            {snapshot.modelCount > 1 ? (
                              <span> · 共 {snapshot.modelCount} 个模型</span>
                            ) : null}
                            {snapshot.statusNote ? (
                              <span> · {snapshot.statusNote}</span>
                            ) : null}
                          </>
                        ) : (
                          <span>
                            {snapshot.statusNote || "尚未配置第三方模型"}
                          </span>
                        )}
                      </p>
                    </div>
                    <div className="fy-model-health-actions">
                      {snapshot.canProbe && snapshot.modelId ? (
                        <Button
                          disabled={rowProbing || probingAll}
                          onClick={() => void probeSingle(snapshot)}
                        >
                          {rowProbing ? "测速中…" : "测速"}
                        </Button>
                      ) : null}
                      {snapshot.canCloneSource ? (
                        <Button
                          disabled={probingAll}
                          onClick={() => openCloneFlow(snapshot)}
                        >
                          克隆到其他端
                        </Button>
                      ) : null}
                      {snapshot.canRotateKey ? (
                        <Button
                          disabled={probingAll}
                          onClick={() => openRotateFlow(snapshot)}
                        >
                          轮换 Key
                        </Button>
                      ) : null}
                      {snapshot.configured &&
                      snapshot.baseUrl &&
                      snapshot.modelId ? (
                        <Button
                          disabled={probingAll}
                          onClick={() => {
                            const draft: Partial<SaveCustomGatewayTemplateRequest> =
                              {
                                name:
                                  snapshot.providerName ||
                                  snapshot.endpointHost ||
                                  "自定义网关",
                                baseUrl: snapshot.baseUrl!,
                                defaultModelId: snapshot.modelId!,
                                codexProtocol:
                                  snapshot.protocol === "chat"
                                    ? "chat"
                                    : "responses",
                              };
                            handleClose();
                            onSaveAsTemplate(draft);
                          }}
                        >
                          存为模板
                        </Button>
                      ) : null}
                    </div>
                  </li>
                );
              })}
            </ul>
          )}
        </div>
      ) : view === "clone" ? (
        <div className="fy-preset-batch-body">
          {activeCloneSnapshot ? (
            <InlineNotice tone="info">
              来源：
              <strong>{TARGET_LABEL_MAP[activeCloneSnapshot.target]}</strong>
              {" · "}
              <code>{activeCloneSnapshot.modelId}</code>
              {activeCloneSnapshot.endpointHost
                ? ` · ${activeCloneSnapshot.endpointHost}`
                : ""}
            </InlineNotice>
          ) : null}

          <div className="fy-preset-batch-targets">
            <strong>选择要克隆写入的目标软件</strong>
            <div className="fy-preset-batch-target-list">
              {BATCH_CONFIGURABLE_TARGETS.filter(
                (item) => item.id !== cloneSource,
              ).map((item) => (
                <label key={item.id} className="fy-preset-batch-target-row">
                  <Checkbox
                    checked={cloneDestinations.includes(item.id)}
                    onCheckedChange={(checked) => {
                      resetActionState();
                      setCloneDestinations((prev) =>
                        checked
                          ? prev.includes(item.id)
                            ? prev
                            : [...prev, item.id]
                          : prev.filter((id) => id !== item.id),
                      );
                    }}
                    label={item.label}
                  />
                  <span className="fy-preset-batch-target-name">
                    {item.label}
                  </span>
                </label>
              ))}
            </div>
          </div>

          <div className="fy-model-health-overrides">
            <label className="fy-models-field">
              <span>目标模型 ID（留空则沿用来源模型）</span>
              <Input
                value={cloneModelOverride}
                onChange={(e) => {
                  setCloneModelOverride(e.target.value);
                  resetActionState();
                }}
                placeholder={activeCloneSnapshot?.modelId ?? "例如 gpt-5.4"}
              />
            </label>
            {cloneSource !== "claude" &&
            cloneDestinations.includes("claude") ? (
              <label className="fy-models-field">
                <span>Claude 专用地址（选填，留空则自动匹配）</span>
                <Input
                  value={cloneClaudeUrlOverride}
                  onChange={(e) => {
                    setCloneClaudeUrlOverride(e.target.value);
                    resetActionState();
                  }}
                  placeholder="https://gateway.example/anthropic"
                />
              </label>
            ) : null}
          </div>

          {busy ? (
            <div className="fy-models-loading">
              <Spinner label="正在处理克隆配置" />
              <span>正在处理所选应用…</span>
            </div>
          ) : null}
          {error ? <InlineNotice tone="error">{error}</InlineNotice> : null}

          {previewItems && !outcomes ? (
            <section className="fy-preset-batch-preview" aria-label="克隆预览">
              <strong>即将改动的配置文件</strong>
              <ul>
                {previewItems.map((item) => (
                  <li key={item.target}>
                    <div>
                      <strong>{TARGET_LABEL_MAP[item.target]}</strong> · 模型{" "}
                      <code>{item.modelId}</code> · 地址{" "}
                      <code>{item.baseUrl}</code>
                    </div>
                    {item.paths.length > 0 ? (
                      <div className="fy-preset-batch-paths">
                        文件：{item.paths.join(" , ")}
                      </div>
                    ) : null}
                    {item.warning ? (
                      <InlineNotice tone="warning">{item.warning}</InlineNotice>
                    ) : null}
                  </li>
                ))}
              </ul>
            </section>
          ) : null}

          {outcomes ? (
            <section className="fy-preset-batch-outcomes" aria-label="克隆结果">
              <strong>执行结果</strong>
              <ul>
                {outcomes.map((outcome) => (
                  <li
                    key={outcome.target}
                    className="fy-preset-batch-outcome-row"
                  >
                    <Badge
                      tone={outcome.status === "applied" ? "accent" : "warning"}
                    >
                      {outcome.status === "applied" ? "已生效" : "失败已回滚"}
                    </Badge>
                    <strong>{TARGET_LABEL_MAP[outcome.target]}</strong>
                    <span>{outcome.message}</span>
                  </li>
                ))}
              </ul>
            </section>
          ) : null}
        </div>
      ) : (
        <div className="fy-preset-batch-body">
          <label className="fy-models-field">
            <strong>1. 粘贴新的 API Key</strong>
            <Input
              type="password"
              value={rotateApiKey}
              onChange={(e) => {
                setRotateApiKey(e.target.value);
                resetActionState();
              }}
              placeholder="粘贴新的 API Key"
              autoComplete="off"
            />
          </label>

          <div className="fy-preset-batch-targets">
            <strong>2. 勾选要同步更换 Key 的软件</strong>
            {rotatableTargets.length === 0 ? (
              <InlineNotice tone="info">
                当前没有可轮换 API Key 的自定义配置。
              </InlineNotice>
            ) : (
              <div className="fy-preset-batch-target-list">
                {rotatableTargets.map((item) => (
                  <label
                    key={item.target}
                    className="fy-preset-batch-target-row"
                  >
                    <Checkbox
                      checked={rotateTargets.includes(item.target)}
                      onCheckedChange={(checked) => {
                        resetActionState();
                        setRotateTargets((prev) =>
                          checked
                            ? prev.includes(item.target)
                              ? prev
                              : [...prev, item.target]
                            : prev.filter((id) => id !== item.target),
                        );
                      }}
                      label={TARGET_LABEL_MAP[item.target]}
                    />
                    <span className="fy-preset-batch-target-name">
                      {TARGET_LABEL_MAP[item.target]}
                    </span>
                    <code className="fy-preset-batch-target-meta">
                      {item.modelId}
                      {item.endpointHost ? ` · ${item.endpointHost}` : ""}
                    </code>
                  </label>
                ))}
              </div>
            )}
          </div>

          {busy ? (
            <div className="fy-models-loading">
              <Spinner label="正在处理 Key 轮换" />
              <span>正在处理所选应用…</span>
            </div>
          ) : null}
          {error ? <InlineNotice tone="error">{error}</InlineNotice> : null}

          {previewItems && !outcomes ? (
            <section className="fy-preset-batch-preview" aria-label="轮换预览">
              <strong>即将更新密钥的配置文件</strong>
              <ul>
                {previewItems.map((item) => (
                  <li key={item.target}>
                    <div>
                      <strong>{TARGET_LABEL_MAP[item.target]}</strong> ·
                      保留模型 <code>{item.modelId}</code> · 地址{" "}
                      <code>{item.baseUrl}</code>
                    </div>
                    {item.paths.length > 0 ? (
                      <div className="fy-preset-batch-paths">
                        文件：{item.paths.join(" , ")}
                      </div>
                    ) : null}
                    {item.warning ? (
                      <InlineNotice tone="warning">{item.warning}</InlineNotice>
                    ) : null}
                  </li>
                ))}
              </ul>
            </section>
          ) : null}

          {outcomes ? (
            <section className="fy-preset-batch-outcomes" aria-label="轮换结果">
              <strong>执行结果</strong>
              <ul>
                {outcomes.map((outcome) => (
                  <li
                    key={outcome.target}
                    className="fy-preset-batch-outcome-row"
                  >
                    <Badge
                      tone={outcome.status === "applied" ? "accent" : "warning"}
                    >
                      {outcome.status === "applied" ? "已生效" : "失败已回滚"}
                    </Badge>
                    <strong>{TARGET_LABEL_MAP[outcome.target]}</strong>
                    <span>{outcome.message}</span>
                  </li>
                ))}
              </ul>
            </section>
          ) : null}
        </div>
      )}
    </Dialog>
  );
}
