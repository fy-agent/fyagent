import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useMemo, useState } from "react";

import {
  BATCH_CONFIGURABLE_TARGETS,
  DESKTOP_PRESET_TEMPLATES,
  buildBatchTargetSpecs,
  customGatewayToDesktopPreset,
  type CustomGatewayTemplate,
  type DesktopPresetTemplate,
  type SaveCustomGatewayTemplateRequest,
} from "../../domain/configuration/presets/desktopPresetTemplates";
import type { ApiProtocol } from "../../domain/configuration/providerApi";
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

interface TemplateFormState {
  id?: string;
  name: string;
  summary: string;
  baseUrl: string;
  anthropicBaseUrl: string;
  defaultModelId: string;
  codexProtocol: ApiProtocol;
  apiKeyUrl: string;
}

function emptyTemplateForm(
  seed?: Partial<SaveCustomGatewayTemplateRequest> | null,
): TemplateFormState {
  return {
    id: seed?.id,
    name: seed?.name ?? "",
    summary: seed?.summary ?? "",
    baseUrl: seed?.baseUrl ?? "",
    anthropicBaseUrl: seed?.anthropicBaseUrl ?? "",
    defaultModelId: seed?.defaultModelId ?? "",
    codexProtocol: seed?.codexProtocol === "chat" ? "chat" : "responses",
    apiKeyUrl: seed?.apiKeyUrl ?? "",
  };
}

export function PresetBatchDrawer({
  open,
  originRef,
  onOpenChange,
  initialTemplateDraft,
}: {
  open: boolean;
  originRef?: DialogOriginRef;
  onOpenChange: (open: boolean) => void;
  initialTemplateDraft?: Partial<SaveCustomGatewayTemplateRequest> | null;
}) {
  const { ports } = useFeatures();
  const queryClient = useQueryClient();

  const customTemplatesQuery = useQuery({
    queryKey: featureKeys.customGatewayTemplates,
    queryFn: () => ports.providers.listCustomGatewayTemplates(),
    enabled: open,
  });

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

  // Custom template editor state
  const [templateForm, setTemplateForm] = useState<TemplateFormState | null>(
    () =>
      initialTemplateDraft ? emptyTemplateForm(initialTemplateDraft) : null,
  );
  const [templateSaving, setTemplateSaving] = useState(false);
  const [templateError, setTemplateError] = useState<string | null>(null);

  const customTemplates = useMemo(
    () => customTemplatesQuery.data ?? [],
    [customTemplatesQuery.data],
  );

  const customPresets = useMemo(
    () => customTemplates.map(customGatewayToDesktopPreset),
    [customTemplates],
  );

  const allPresets = useMemo(
    () => [...DESKTOP_PRESET_TEMPLATES, ...customPresets],
    [customPresets],
  );

  const selectedPreset: DesktopPresetTemplate = useMemo(
    () =>
      allPresets.find((p) => p.id === selectedPresetId) ??
      DESKTOP_PRESET_TEMPLATES[0],
    [allPresets, selectedPresetId],
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

  const handleSaveTemplate = async () => {
    if (!templateForm) return;
    setTemplateSaving(true);
    setTemplateError(null);
    try {
      const saved = await ports.providers.saveCustomGatewayTemplate({
        ...(templateForm.id ? { id: templateForm.id } : {}),
        name: templateForm.name,
        summary: templateForm.summary,
        baseUrl: templateForm.baseUrl,
        ...(templateForm.anthropicBaseUrl.trim()
          ? { anthropicBaseUrl: templateForm.anthropicBaseUrl.trim() }
          : {}),
        defaultModelId: templateForm.defaultModelId,
        codexProtocol: templateForm.codexProtocol,
        ...(templateForm.apiKeyUrl.trim()
          ? { apiKeyUrl: templateForm.apiKeyUrl.trim() }
          : {}),
      });
      await queryClient.invalidateQueries({
        queryKey: featureKeys.customGatewayTemplates,
      });
      setTemplateForm(null);
      handleSelectPreset(saved.id);
    } catch (err) {
      setTemplateError(
        err instanceof Error ? err.message : "保存自定义模板失败",
      );
    } finally {
      setTemplateSaving(false);
    }
  };

  const handleDeleteTemplate = async (template: CustomGatewayTemplate) => {
    setTemplateSaving(true);
    setTemplateError(null);
    try {
      await ports.providers.deleteCustomGatewayTemplate(template.id);
      await queryClient.invalidateQueries({
        queryKey: featureKeys.customGatewayTemplates,
      });
      if (selectedPresetId === template.id) {
        handleSelectPreset(DESKTOP_PRESET_TEMPLATES[0].id);
      }
    } catch (err) {
      setTemplateError(
        err instanceof Error ? err.message : "删除自定义模板失败",
      );
    } finally {
      setTemplateSaving(false);
    }
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
      setError("无法生成批量配置预览，请检查配置文件权限后重试。");
    } finally {
      setBusy(false);
    }
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

  const handleApply = async (overrideTargets?: BatchSetupTargetId[]) => {
    const trimmedKey = apiKey.trim();
    if (!trimmedKey) {
      setError("请先粘贴 API Key");
      return;
    }
    const targetIds = overrideTargets ?? effectiveTargets;
    const readyTargets = previewItems
      ? targetIds.filter(
          (id) =>
            previewItems.find((item) => item.target === id)?.ready ?? true,
        )
      : targetIds;
    if (readyTargets.length === 0) {
      setError("当前所选目标均存在前置阻塞项，请先处理预览中的提示。");
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
      setError("未能完成批量配置，原文件未改动。");
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
    setTemplateForm(null);
    setTemplateError(null);
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
          setTemplateForm(null);
          setTemplateError(null);
        }
        onOpenChange(next);
      }}
      title="一键配置主流大模型"
      description="选择服务商并粘贴 API Key，一次配好多个 AI 软件。"
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
              disabled={busy || templateForm !== null}
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
            <strong>1. 选择服务商或我的模板</strong>
            <div className="fy-preset-batch-header-tools">
              {selectedPreset.apiKeyUrl ? (
                <ExternalLinkButton url={selectedPreset.apiKeyUrl}>
                  前往获取 {selectedPreset.name} API Key
                </ExternalLinkButton>
              ) : null}
              <Button
                disabled={templateSaving}
                onClick={() => {
                  setTemplateError(null);
                  setTemplateForm((current) =>
                    current ? null : emptyTemplateForm(),
                  );
                }}
              >
                {templateForm ? "收起模板编辑" : "+ 新建自定义模板"}
              </Button>
            </div>
          </div>

          {templateForm ? (
            <div
              className="fy-custom-template-editor"
              aria-label="自定义网关模板表单"
            >
              <strong>
                {templateForm.id ? "编辑自定义网关模板" : "新建自定义网关模板"}
              </strong>
              <div className="fy-custom-template-fields">
                <label className="fy-models-field">
                  <span>模板名称</span>
                  <Input
                    value={templateForm.name}
                    onChange={(e) =>
                      setTemplateForm({ ...templateForm, name: e.target.value })
                    }
                    placeholder="例如 公司内部网关 / 我的中转站"
                  />
                </label>
                <label className="fy-models-field">
                  <span>默认模型 ID</span>
                  <Input
                    value={templateForm.defaultModelId}
                    onChange={(e) =>
                      setTemplateForm({
                        ...templateForm,
                        defaultModelId: e.target.value,
                      })
                    }
                    placeholder="例如 gpt-5.4 或 claude-sonnet-4-6"
                  />
                </label>
                <label className="fy-models-field">
                  <span>服务地址（OpenAI 兼容 Base URL）</span>
                  <Input
                    value={templateForm.baseUrl}
                    onChange={(e) =>
                      setTemplateForm({
                        ...templateForm,
                        baseUrl: e.target.value,
                      })
                    }
                    placeholder="https://gateway.example/v1"
                  />
                </label>
                <label className="fy-models-field">
                  <span>Claude 专用地址（选填，留空沿用上方地址）</span>
                  <Input
                    value={templateForm.anthropicBaseUrl}
                    onChange={(e) =>
                      setTemplateForm({
                        ...templateForm,
                        anthropicBaseUrl: e.target.value,
                      })
                    }
                    placeholder="https://gateway.example/anthropic"
                  />
                </label>
                <div className="fy-models-field">
                  <span>Codex 协议</span>
                  <div className="fy-models-chip-row">
                    {(["responses", "chat"] as const).map((proto) => (
                      <Button
                        key={proto}
                        className={
                          templateForm.codexProtocol === proto
                            ? "fy-control-button-primary"
                            : undefined
                        }
                        onClick={() =>
                          setTemplateForm({
                            ...templateForm,
                            codexProtocol: proto,
                          })
                        }
                      >
                        {proto === "responses"
                          ? "Responses API"
                          : "Chat Completions"}
                      </Button>
                    ))}
                  </div>
                </div>
                <label className="fy-models-field">
                  <span>备注说明（选填）</span>
                  <Input
                    value={templateForm.summary}
                    onChange={(e) =>
                      setTemplateForm({
                        ...templateForm,
                        summary: e.target.value,
                      })
                    }
                    placeholder="例如 团队共享中转网关"
                  />
                </label>
              </div>
              {templateError ? (
                <InlineNotice tone="error">{templateError}</InlineNotice>
              ) : null}
              <div className="fy-custom-template-actions">
                <Button
                  disabled={templateSaving}
                  onClick={() => {
                    setTemplateForm(null);
                    setTemplateError(null);
                  }}
                >
                  取消
                </Button>
                <Button
                  className="fy-control-button-primary"
                  disabled={templateSaving}
                  onClick={() => void handleSaveTemplate()}
                >
                  {templateSaving ? "正在保存…" : "保存模板"}
                </Button>
              </div>
            </div>
          ) : null}

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

          {customTemplates.length > 0 ? (
            <div className="fy-custom-template-section">
              <strong className="fy-custom-template-heading">
                我的自定义模板 ({customTemplates.length})
              </strong>
              <div
                className="fy-preset-batch-grid"
                role="radiogroup"
                aria-label="我的自定义网关模板"
              >
                {customTemplates.map((tpl) => {
                  const active = tpl.id === selectedPreset.id;
                  return (
                    <div
                      key={tpl.id}
                      className="fy-custom-template-card-wrap"
                      data-selected={active ? "true" : undefined}
                    >
                      <button
                        type="button"
                        role="radio"
                        aria-checked={active}
                        className="fy-custom-template-select"
                        onClick={() => handleSelectPreset(tpl.id)}
                      >
                        <strong>{tpl.name}</strong>
                        <span>
                          {tpl.defaultModelId} · {tpl.summary || tpl.baseUrl}
                        </span>
                      </button>
                      <div className="fy-custom-template-card-tools">
                        <Button
                          disabled={templateSaving}
                          aria-label={`编辑模板 ${tpl.name}`}
                          onClick={() => {
                            setTemplateError(null);
                            setTemplateForm(emptyTemplateForm(tpl));
                          }}
                        >
                          编辑
                        </Button>
                        <Button
                          disabled={templateSaving}
                          aria-label={`删除模板 ${tpl.name}`}
                          onClick={() => void handleDeleteTemplate(tpl)}
                        >
                          删除
                        </Button>
                      </div>
                    </div>
                  );
                })}
              </div>
            </div>
          ) : null}
        </section>

        <section aria-label="填写 API Key 与目标软件">
          <label className="fy-models-field">
            <strong>2. 粘贴 API Key</strong>
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
            <strong>即将改动的配置文件</strong>
            <p className="fy-preset-batch-paths">
              仅更新所选软件的模型地址、密钥与模型
              ID，写入前自动备份；绝不改动聊天记录、MCP、Skills、官方订阅登录及其他手改字段。
            </p>
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
    </Dialog>
  );
}
