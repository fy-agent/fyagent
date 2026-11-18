import { useEffect, useRef, useState } from "react";

import { CopyablePath } from "../../shared/features/controls/CopyablePath";
import {
  OMP_PROVIDER_APIS,
  isValidOmpModelId,
  ompErrorCode,
  type OmpChangedFile,
  type OmpFileStatus,
  type OmpProviderApi,
  type OmpProviderSummary,
  type OmpSavePreview,
  type OmpSaveRequest,
} from "../../shared/features/omp";
import { useFeatures } from "../../shared/features/provider";
import {
  useOmpCliDiscovery,
  useOmpConfig,
} from "../../shared/features/queries";
import { Button } from "../../shared/ui/Button";
import { CatalogDetail } from "../../shared/ui/catalog";
import { Dialog } from "../../shared/ui/Dialog";
import {
  Checkbox,
  InlineNotice,
  Input,
  Spinner,
} from "../../shared/ui/primitives";
import { FieldFeedback, ModelsSection, type Notice } from "./feedback";
import { ModelsPanelHeader, NoticeView, useModelsDraftCommit } from "./modelsShared";
import {
  OMP_API_LABELS,
  OMP_NEW_TARGET,
  buildOmpSaveRequest,
  defaultOmpSelection,
  draftFromProvider,
  isEditableOmpProvider,
  planModels,
  providerById,
  validateOmpDraft,
  type OmpDraft,
  type OmpDraftErrors,
  type OmpKeySource,
  type OmpTargetSelection,
} from "./ompModels";

const FILE_STATUS_COPY: Readonly<Record<OmpFileStatus, string>> = {
  ok: "已读取",
  notFound: "尚未创建（保存时新建）",
  legacyJsonOnly: "仅有旧版 models.json",
  invalidYaml: "YAML 无法解析",
};

const CHANGED_FILE_COPY: Readonly<Record<OmpChangedFile, string>> = {
  models: "更新 models.yml 中该供应商的地址、协议、鉴权引用与所列模型（保留其余注释与字段）",
  config: "更新 config.yml 中的 modelRoles.default（保留其余角色配置与注释）",
};

const KEY_SOURCE_LABELS: Readonly<Record<OmpKeySource, string>> = {
  keep: "保留现有引用",
  env: "环境变量名",
  command: "!命令引用",
};

interface PendingSave {
  request: OmpSaveRequest;
  preview: OmpSavePreview;
  revision: number;
}

function keySummary(provider: OmpProviderSummary): string {
  if (provider.auth === "none") return "无需鉴权";
  if (provider.auth === "oauth") return "由 omp 登录管理";
  if (provider.hasCommandKey) return "已配置命令引用";
  if (provider.hasApiKey) return "已配置环境变量/密钥引用";
  return "未配置引用";
}

export function OmpModelsPanel({ active }: { active: boolean }) {
  const { ports } = useFeatures();
  const configQuery = useOmpConfig(active);
  const cliQuery = useOmpCliDiscovery(active);
  const commit = useModelsDraftCommit();
  const saveOriginRef = useRef<HTMLElement | null>(null);

  const [selection, setSelection] = useState<OmpTargetSelection | null>(null);
  const [draft, setDraft] = useState<OmpDraft>(() => draftFromProvider(null, null));
  const [newModelId, setNewModelId] = useState("");
  const [errors, setErrors] = useState<OmpDraftErrors>({});
  const [notice, setNotice] = useState<Notice | null>(null);
  const [busy, setBusy] = useState(false);
  const [pendingSave, setPendingSave] = useState<PendingSave | null>(null);

  const read = configQuery.data;
  const effectiveSelection = selection ?? defaultOmpSelection(read);
  const selectedProvider = providerById(read, effectiveSelection);
  const editable = !selectedProvider || isEditableOmpProvider(selectedProvider);

  useEffect(() => {
    if (!read || commit.pending) return;
    const nextSel = selection ?? defaultOmpSelection(read);
    const prov = providerById(read, nextSel);
    setDraft(draftFromProvider(prov, read.defaultModel));
  }, [read, selection, commit.pending]);

  const updateDraft = (patch: Partial<OmpDraft>) => {
    commit.markDirty();
    setDraft((prev) => ({ ...prev, ...patch }));
    setErrors({});
  };

  const handleSelectTarget = (next: OmpTargetSelection) => {
    setSelection(next);
    const prov = providerById(read, next);
    setDraft(draftFromProvider(prov, read?.defaultModel ?? null));
    setErrors({});
    setNotice(null);
  };

  const addModelId = () => {
    const trimmed = newModelId.trim();
    if (!trimmed) return;
    if (!isValidOmpModelId(trimmed)) {
      setErrors((prev) => ({
        ...prev,
        models: "模型 ID 不能为空，也不能以 / 开头或结尾。",
      }));
      return;
    }
    const merged = planModels(draft, selectedProvider);
    if (merged.some((m) => m.id === trimmed)) {
      setNewModelId("");
      return;
    }
    updateDraft({
      models: [...draft.models, { id: trimmed }],
      defaultModelId: draft.defaultModelId || trimmed,
    });
    setNewModelId("");
  };

  const removeDraftModel = (id: string) => {
    updateDraft({
      models: draft.models.filter((m) => m.id !== id),
      defaultModelId: draft.defaultModelId === id ? "" : draft.defaultModelId,
    });
  };

  const handlePreviewSave = async () => {
    if (!read || busy || !editable) return;
    const nextErrors = validateOmpDraft(draft, selectedProvider, read);
    if (Object.keys(nextErrors).length > 0) {
      setErrors(nextErrors);
      return;
    }
    setBusy(true);
    setNotice(null);
    const rev = commit.captureRevision();
    try {
      const request = buildOmpSaveRequest(draft, selectedProvider, read);
      const preview = await ports.ompModels.previewSave(request);
      setPendingSave({ request, preview, revision: rev });
    } catch (err) {
      const code = ompErrorCode(err);
      setNotice({
        tone: "error",
        title:
          code === "stale"
            ? "配置文件已在外部修改，请先点击“重新读取”"
            : code === "invalid_input"
              ? "输入参数不符合 omp YAML 规范，请检查字段"
              : "生成保存预览失败",
      });
    } finally {
      setBusy(false);
    }
  };

  const handleConfirmApply = async () => {
    if (!pendingSave || busy) return;
    const currentPending = pendingSave;
    setPendingSave(null);
    setBusy(true);
    try {
      const result = await ports.ompModels.applySave({
        planId: currentPending.preview.planId,
        planDigest: currentPending.preview.planDigest,
      });
      await configQuery.refetch();
      commit.commitRevision(currentPending.revision);
      setSelection(`existing:${result.provider.id}`);
      setNotice({
        tone: "info",
        title: "已写入 omp 配置文件",
        description:
          "请退出正在运行的 omp 会话后重新启动，新会话将读取最新模型设置。",
      });
    } catch (err) {
      const code = ompErrorCode(err);
      setNotice({
        tone: "error",
        title:
          code === "stale"
            ? "保存前检测到外部并发修改，未覆盖文件"
            : "写入 omp 配置失败，原文件已保持不变",
      });
    } finally {
      setBusy(false);
    }
  };

  const mergedModels = planModels(draft, selectedProvider);
  const blockedReason =
    read?.modelsFileStatus === "legacyJsonOnly"
      ? {
          tone: "error" as const,
          title: "检测到旧版 models.json",
          description:
            "请先在终端运行一次 omp 自动迁移生成 models.yml，再点击“重新读取”。",
        }
      : read?.modelsFileStatus === "invalidYaml" ||
          read?.configFileStatus === "invalidYaml"
        ? {
            tone: "error" as const,
            title: "omp YAML 语法无法解析",
            description:
              "FyAgent 不会覆盖无法解析的文件。请先修复 YAML 语法后再重新读取。",
          }
        : null;

  return (
    <CatalogDetail
      className="fy-models-config-panel"
      ariaLabel="Oh My Pi (omp) 模型配置"
    >
      <ModelsPanelHeader
        title="Oh My Pi (omp)"
        summary="无损局部编辑 ~/.omp/agent/models.yml 与 config.yml，保留注释、未知字段与环境变量引用。"
        pending={commit.pending}
      >
        <Button
          disabled={configQuery.isFetching || busy}
          onClick={() => {
            void configQuery.refetch();
            void cliQuery.refetch();
          }}
        >
          重新读取
        </Button>
        <Button
          className="fy-control-button-primary"
          disabled={
            busy ||
            configQuery.isPending ||
            !read ||
            Boolean(blockedReason) ||
            !editable
          }
          onClick={(e) => {
            saveOriginRef.current = e.currentTarget;
            void handlePreviewSave();
          }}
        >
          {busy ? "处理中…" : "预览并保存"}
        </Button>
      </ModelsPanelHeader>

      <NoticeView notice={blockedReason ?? notice} />

      {configQuery.isPending ? (
        <div className="fy-models-loading">
          <Spinner label="正在读取 omp 配置" />
          <span>正在读取 omp 配置文件…</span>
        </div>
      ) : configQuery.isError || !read ? (
        <InlineNotice tone="error">
          无法读取 omp 配置目录，请检查文件权限后点击“重新读取”。
        </InlineNotice>
      ) : (
        <div className="fy-models-sections">
          <ModelsSection title="运行环境与目标文件">
            <p className="fy-models-hint">
              Profile：<strong>{read.profile}</strong> · CLI 状态：
              {cliQuery.data?.detected
                ? `已检测${cliQuery.data.version ? ` (v${cliQuery.data.version})` : ""}`
                : "未检测到 omp 可执行文件（仍可预写配置）"}
            </p>
            <div className="fy-models-paths">
              <div>
                <span>models.yml（{FILE_STATUS_COPY[read.modelsFileStatus]}）</span>
                <CopyablePath value={read.modelsPath} label="models.yml 路径" />
              </div>
              <div>
                <span>config.yml（{FILE_STATUS_COPY[read.configFileStatus]}）</span>
                <CopyablePath value={read.configPath} label="config.yml 路径" />
              </div>
            </div>
          </ModelsSection>

          <ModelsSection title="选择或新建供应商" invalid={Boolean(errors.providerId)}>
            <div className="fy-models-chip-row" role="group" aria-label="omp 供应商选择">
              {read.providers.map((prov) => {
                const targetKey: OmpTargetSelection = `existing:${prov.id}`;
                return (
                  <Button
                    key={prov.id}
                    className={
                      effectiveSelection === targetKey
                        ? "fy-control-button-primary"
                        : undefined
                    }
                    onClick={() => handleSelectTarget(targetKey)}
                  >
                    {prov.id} ({prov.models.length})
                  </Button>
                );
              })}
              <Button
                className={
                  effectiveSelection === OMP_NEW_TARGET
                    ? "fy-control-button-primary"
                    : undefined
                }
                onClick={() => handleSelectTarget(OMP_NEW_TARGET)}
              >
                + 新建供应商
              </Button>
            </div>

            {!selectedProvider ? (
              <label className="fy-models-field">
                <span>供应商 ID</span>
                <Input
                  value={draft.providerId}
                  onChange={(e) => updateDraft({ providerId: e.target.value })}
                  placeholder="例如 fyagent-deepseek"
                  disabled={Boolean(blockedReason)}
                />
                {errors.providerId ? (
                  <FieldFeedback
                    notice={{ tone: "error", title: errors.providerId }}
                  />
                ) : null}
              </label>
            ) : (
              <p className="fy-models-hint">
                当前编辑：<strong>{selectedProvider.id}</strong> ·{" "}
                {keySummary(selectedProvider)}
              </p>
            )}

            {!editable ? (
              <InlineNotice tone="info">
                该供应商使用非 OpenAI 兼容协议（{selectedProvider?.api}），为保护私有字段仅展示只读状态，请在 omp 内直接编辑。
              </InlineNotice>
            ) : null}
          </ModelsSection>

          {editable ? (
            <>
              <ModelsSection title="接口地址与鉴权引用" invalid={Boolean(errors.baseUrl || errors.keyRef)}>
                <label className="fy-models-field">
                  <span>Base URL</span>
                  <Input
                    value={draft.baseUrl}
                    onChange={(e) => updateDraft({ baseUrl: e.target.value })}
                    placeholder="https://api.deepseek.com/v1"
                    disabled={Boolean(blockedReason)}
                  />
                  {errors.baseUrl ? (
                    <FieldFeedback
                      notice={{ tone: "error", title: errors.baseUrl }}
                    />
                  ) : null}
                </label>

                <div className="fy-models-field">
                  <span>协议类型</span>
                  <div className="fy-models-chip-row">
                    {OMP_PROVIDER_APIS.map((api: OmpProviderApi) => (
                      <Button
                        key={api}
                        className={
                          draft.api === api
                            ? "fy-control-button-primary"
                            : undefined
                        }
                        onClick={() => updateDraft({ api })}
                      >
                        {OMP_API_LABELS[api]}
                      </Button>
                    ))}
                  </div>
                </div>

                <div className="fy-models-checkbox-row fy-models-checkbox-row-inline">
                  <Checkbox
                    checked={draft.auth === "none"}
                    onCheckedChange={(noAuth) =>
                      updateDraft({ auth: noAuth ? "none" : "apiKey" })
                    }
                    label="本地无鉴权模型（auth: none）"
                  />
                  <span>本地无鉴权模型（如 Ollama）</span>
                </div>

                {draft.auth === "apiKey" ? (
                  <div className="fy-models-field">
                    <span>密钥引用方式（omp 推荐填写环境变量名或 !命令引用）</span>
                    <div className="fy-models-chip-row">
                      {(["keep", "env", "command"] as const)
                        .filter((mode) => mode !== "keep" || selectedProvider?.hasApiKey)
                        .map((mode) => (
                          <Button
                            key={mode}
                            className={
                              draft.keySource === mode
                                ? "fy-control-button-primary"
                                : undefined
                            }
                            onClick={() => updateDraft({ keySource: mode })}
                          >
                            {KEY_SOURCE_LABELS[mode]}
                          </Button>
                        ))}
                    </div>
                    {draft.keySource === "env" ? (
                      <Input
                        value={draft.envName}
                        onChange={(e) => updateDraft({ envName: e.target.value })}
                        placeholder="例如 DEEPSEEK_API_KEY"
                      />
                    ) : draft.keySource === "command" ? (
                      <Input
                        value={draft.commandRef}
                        onChange={(e) =>
                          updateDraft({ commandRef: e.target.value })
                        }
                        placeholder="例如 !op read op://Private/DeepSeek/credential"
                      />
                    ) : null}
                    {errors.keyRef ? (
                      <FieldFeedback
                        notice={{ tone: "error", title: errors.keyRef }}
                      />
                    ) : null}
                  </div>
                ) : null}
              </ModelsSection>

              <ModelsSection title="模型列表与默认角色" invalid={Boolean(errors.models || errors.defaultModel)}>
                <div className="fy-models-inline-input">
                  <Input
                    value={newModelId}
                    onChange={(e) => setNewModelId(e.target.value)}
                    onKeyDown={(e) => {
                      if (e.key === "Enter") {
                        e.preventDefault();
                        addModelId();
                      }
                    }}
                    placeholder="输入模型 ID（如 deepseek-chat）后回车或点添加"
                  />
                  <Button onClick={addModelId}>添加模型</Button>
                </div>

                {mergedModels.length > 0 ? (
                  <div className="fy-models-chip-row">
                    {mergedModels.map((m) => {
                      const fromDraft = draft.models.some((d) => d.id === m.id);
                      return (
                        <span key={m.id} className="fy-models-chip">
                          <code>{m.id}</code>
                          {fromDraft ? (
                            <Button
                              onClick={() => removeDraftModel(m.id)}
                              aria-label={`移除 ${m.id}`}
                            >
                              ×
                            </Button>
                          ) : null}
                        </span>
                      );
                    })}
                  </div>
                ) : null}
                {errors.models ? (
                  <FieldFeedback
                    notice={{ tone: "error", title: errors.models }}
                  />
                ) : null}

                <div className="fy-models-checkbox-row fy-models-checkbox-row-inline">
                  <Checkbox
                    checked={draft.setDefault}
                    onCheckedChange={(checked) =>
                      updateDraft({
                        setDefault: checked,
                        defaultModelId:
                          draft.defaultModelId || mergedModels[0]?.id || "",
                      })
                    }
                    label="设为 omp 默认模型（modelRoles.default）"
                  />
                  <span>同步设为 omp 默认模型（更新 config.yml）</span>
                </div>

                {draft.setDefault && mergedModels.length > 0 ? (
                  <div className="fy-models-chip-row">
                    {mergedModels.map((m) => (
                      <Button
                        key={m.id}
                        className={
                          draft.defaultModelId === m.id
                            ? "fy-control-button-primary"
                            : undefined
                        }
                        onClick={() => updateDraft({ defaultModelId: m.id })}
                      >
                        {m.id}
                      </Button>
                    ))}
                  </div>
                ) : null}
                {errors.defaultModel ? (
                  <FieldFeedback
                    notice={{ tone: "error", title: errors.defaultModel }}
                  />
                ) : null}
              </ModelsSection>
            </>
          ) : null}
        </div>
      )}

      <Dialog
        open={pendingSave !== null}
        originRef={saveOriginRef}
        onOpenChange={(open) => {
          if (!open) setPendingSave(null);
        }}
        title="确认写入 omp 配置"
        description="写入前会自动生成滚动备份，并校验文件未被外部进程并发修改。"
        actions={
          pendingSave ? (
            <>
              <Button onClick={() => setPendingSave(null)}>取消</Button>
              <Button
                className="fy-control-button-primary"
                disabled={busy}
                onClick={() => void handleConfirmApply()}
              >
                确认写入
              </Button>
            </>
          ) : undefined
        }
      >
        {pendingSave ? (
          <div className="fy-models-preview-summary">
            <p>
              目标供应商：<strong>{pendingSave.preview.provider.id}</strong>（共{" "}
              {pendingSave.preview.provider.models.length} 个模型）
            </p>
            {pendingSave.preview.defaultModel ? (
              <p>
                默认模型角色：<code>{pendingSave.preview.defaultModel}</code>
              </p>
            ) : null}
            {pendingSave.preview.changedFiles.length === 0 ? (
              <InlineNotice tone="info">
                当前配置与文件内容一致，确认后无需改动磁盘文件。
              </InlineNotice>
            ) : (
              <ul>
                {pendingSave.preview.changedFiles.map((file) => (
                  <li key={file}>{CHANGED_FILE_COPY[file]}</li>
                ))}
              </ul>
            )}
          </div>
        ) : null}
      </Dialog>
    </CatalogDetail>
  );
}
