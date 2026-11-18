import { useQueryClient } from "@tanstack/react-query";
import { useMemo, useState } from "react";

import {
  BATCH_CONFIGURABLE_TARGETS,
  DESKTOP_PRESET_TEMPLATES,
  buildBatchTargetSpecs,
  type DesktopPresetTemplate,
} from "../../domain/configuration/presets/desktopPresetTemplates";
import { ExternalLinkButton } from "../../shared/features/controls/ExternalLinkButton";
import { useFeatures } from "../../shared/features/provider";
import { featureKeys } from "../../shared/features/queries";
import type {
  BatchProviderPreviewItem,
  BatchSetupTargetId,
  BatchTargetOutcome,
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

export function PresetBatchDrawer({
  open,
  originRef,
  onOpenChange,
}: {
  open: boolean;
  originRef?: DialogOriginRef;
  onOpenChange: (open: boolean) => void;
}) {
  const { ports } = useFeatures();
  const queryClient = useQueryClient();

  const [selectedPresetId, setSelectedPresetId] = useState<string>(
    DESKTOP_PRESET_TEMPLATES[0].id,
  );
  const [apiKey, setApiKey] = useState("");
  const [checkedTargets, setCheckedTargets] = useState<BatchSetupTargetId[]>(
    () =>
      BATCH_CONFIGURABLE_TARGETS.filter((t) => t.defaultChecked).map(
        (t) => t.id,
      ),
  );
  const [previewItems, setPreviewItems] = useState<
    BatchProviderPreviewItem[] | null
  >(null);
  const [outcomes, setOutcomes] = useState<BatchTargetOutcome[] | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const selectedPreset: DesktopPresetTemplate = useMemo(
    () =>
      DESKTOP_PRESET_TEMPLATES.find((p) => p.id === selectedPresetId) ??
      DESKTOP_PRESET_TEMPLATES[0],
    [selectedPresetId],
  );

  const supportedTargetIds = useMemo(
    () =>
      BATCH_CONFIGURABLE_TARGETS.filter(
        (item) => selectedPreset.targets[item.id] !== undefined,
      ),
    [selectedPreset],
  );

  const effectiveTargets = useMemo(
    () =>
      checkedTargets.filter((id) => selectedPreset.targets[id] !== undefined),
    [checkedTargets, selectedPreset],
  );

  const handleSelectPreset = (id: string) => {
    setSelectedPresetId(id);
    setPreviewItems(null);
    setOutcomes(null);
    setError(null);
  };

  const toggleTarget = (id: BatchSetupTargetId, next: boolean) => {
    setPreviewItems(null);
    setOutcomes(null);
    setError(null);
    setCheckedTargets((prev) =>
      next
        ? prev.includes(id)
          ? prev
          : [...prev, id]
        : prev.filter((item) => item !== id),
    );
  };

  const handlePreview = async () => {
    const trimmedKey = apiKey.trim();
    if (!trimmedKey) {
      setError("请先粘贴 API Key");
      return;
    }
    if (effectiveTargets.length === 0) {
      setError("请至少勾选 1 个要配置的应用");
      return;
    }
    setBusy(true);
    setError(null);
    setOutcomes(null);
    try {
      const targets = buildBatchTargetSpecs(selectedPreset, effectiveTargets);
      const result = await ports.providers.previewBatchSetup({ targets });
      setPreviewItems(result.items);
    } catch {
      setError("无法生成写入预览，请检查配置文件权限后重试。");
    } finally {
      setBusy(false);
    }
  };

  const invalidateModelQueries = async () => {
    await Promise.allSettled([
      queryClient.invalidateQueries({
        queryKey: featureKeys.providerSummary("claude"),
      }),
      queryClient.invalidateQueries({
        queryKey: featureKeys.providerSummary("codex"),
      }),
      queryClient.invalidateQueries({
        queryKey: featureKeys.providerSummary("grokbuild"),
      }),
      queryClient.invalidateQueries({
        queryKey: featureKeys.providerList("claude"),
      }),
      queryClient.invalidateQueries({
        queryKey: featureKeys.providerList("codex"),
      }),
      queryClient.invalidateQueries({ queryKey: featureKeys.workbuddyStatus }),
      queryClient.invalidateQueries({
        queryKey: featureKeys.workbuddyModelIds,
      }),
      queryClient.invalidateQueries({
        queryKey: featureKeys.openCodeModelSnapshot,
      }),
      queryClient.invalidateQueries({ queryKey: featureKeys.ompConfig }),
    ]);
  };

  const handleApply = async (retryTargets?: BatchSetupTargetId[]) => {
    const trimmedKey = apiKey.trim();
    if (!trimmedKey || busy) return;
    const targetList = retryTargets ?? effectiveTargets;
    const readyTargets = previewItems
      ? targetList.filter(
          (id) =>
            previewItems.find((item) => item.target === id)?.ready !== false,
        )
      : targetList;
    if (readyTargets.length === 0) {
      setError("当前没有可安全写入的目标应用，请先处理上方提示。");
      return;
    }
    setBusy(true);
    setError(null);
    try {
      const specs = buildBatchTargetSpecs(selectedPreset, readyTargets);
      const res = await ports.providers.applyBatchSetup({
        apiKey: trimmedKey,
        targets: specs,
      });
      setOutcomes(res.outcomes);
      if (res.outcomes.every((o) => o.status === "applied")) {
        setApiKey("");
      }
      await invalidateModelQueries();
    } catch {
      setError("批量装配未能完成，未提交的目标文件保持原状。");
    } finally {
      setBusy(false);
    }
  };

  const failedTargets =
    outcomes?.filter((o) => o.status === "failed").map((o) => o.target) ?? [];

  const handleClose = () => {
    setApiKey("");
    setPreviewItems(null);
    setOutcomes(null);
    setError(null);
    onOpenChange(false);
  };

  return (
    <Dialog
      open={open}
      size="comfortable"
      originRef={originRef}
      onOpenChange={(next) => {
        if (!next) {
          setApiKey("");
          setPreviewItems(null);
          setOutcomes(null);
          setError(null);
        }
        onOpenChange(next);
      }}
      title="一键配置主流大模型"
      description="选定服务商并粘贴一次 API Key，按各客户端协议自动完成备份、无损写入与回读校验（Codex / Claude / Grok Build 将同步更新一键配置方案并设为当前激活）。"
      actions={
        <>
          <Button onClick={handleClose}>关闭</Button>
          {outcomes && failedTargets.length > 0 ? (
            <Button
              className="fy-control-button-primary"
              disabled={busy}
              onClick={() => void handleApply(failedTargets)}
            >
              重试失败项 ({failedTargets.length})
            </Button>
          ) : previewItems ? (
            <>
              <Button
                disabled={busy}
                onClick={() => {
                  setPreviewItems(null);
                  setOutcomes(null);
                }}
              >
                返回修改
              </Button>
              <Button
                className="fy-control-button-primary"
                disabled={busy}
                onClick={() => void handleApply()}
              >
                {busy ? "正在写入…" : "确认一键写入"}
              </Button>
            </>
          ) : (
            <Button
              className="fy-control-button-primary"
              disabled={busy}
              onClick={() => void handlePreview()}
            >
              {busy ? "正在检查…" : "预览改动范围"}
            </Button>
          )}
        </>
      }
    >
      <div className="fy-preset-batch-body">
        <section aria-label="主流模型服务商">
          <div className="fy-preset-batch-step-header">
            <strong>1. 选择主流服务商</strong>
            <ExternalLinkButton url={selectedPreset.apiKeyUrl}>
              前往获取 {selectedPreset.name} API Key
            </ExternalLinkButton>
          </div>
          <div
            className="fy-preset-batch-grid"
            role="radiogroup"
            aria-label="选择模型服务商"
          >
            {DESKTOP_PRESET_TEMPLATES.map((preset) => {
              const active = preset.id === selectedPreset.id;
              return (
                <button
                  key={preset.id}
                  type="button"
                  role="radio"
                  aria-checked={active}
                  className="fy-preset-batch-card"
                  data-selected={active ? "true" : undefined}
                  onClick={() => handleSelectPreset(preset.id)}
                >
                  <strong>{preset.name}</strong>
                  <span>{preset.summary}</span>
                </button>
              );
            })}
          </div>
        </section>

        <section aria-label="填写 API Key 与目标软件">
          <label className="fy-models-field">
            <strong>2. 粘贴 API Key（自动去除首尾空格与换行）</strong>
            <Input
              type="password"
              value={apiKey}
              onChange={(e) => {
                setApiKey(e.target.value);
                setPreviewItems(null);
                setOutcomes(null);
                setError(null);
              }}
              placeholder={`粘贴 ${selectedPreset.name} 的 API Key`}
              autoComplete="off"
            />
          </label>

          <div className="fy-preset-batch-targets">
            <strong>3. 勾选要同步写入的 AI 软件</strong>
            <div className="fy-preset-batch-target-list">
              {supportedTargetIds.map((item) => {
                const mapping = selectedPreset.targets[item.id]!;
                return (
                  <label key={item.id} className="fy-preset-batch-target-row">
                    <Checkbox
                      checked={effectiveTargets.includes(item.id)}
                      onCheckedChange={(next) => toggleTarget(item.id, next)}
                      label={item.label}
                    />
                    <span className="fy-preset-batch-target-name">
                      {item.label}
                    </span>
                    <code className="fy-preset-batch-target-meta">
                      {mapping.modelId} · {mapping.protocol}
                    </code>
                  </label>
                );
              })}
            </div>
          </div>
        </section>

        {busy ? (
          <div className="fy-models-loading">
            <Spinner label="正在处理批量配置" />
            <span>正在处理所选应用的配置…</span>
          </div>
        ) : null}

        {error ? <InlineNotice tone="error">{error}</InlineNotice> : null}

        {previewItems && !outcomes ? (
          <section
            className="fy-preset-batch-preview"
            aria-label="即将改动的配置文件预览"
          >
            <strong>即将改动的配置文件（写前自动生成滚动备份）</strong>
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
          <section
            className="fy-preset-batch-outcomes"
            aria-label="批量写入结果"
          >
            <strong>执行结果（单应用失败仅回滚自身，不影响已成功应用）</strong>
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
    </Dialog>
  );
}
