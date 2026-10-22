import { useEffect, useRef, useState } from "react";

import { CopyablePath } from "../../shared/features/controls/CopyablePath";
import { ExternalLinkButton } from "../../shared/features/controls/ExternalLinkButton";
import {
  OMP_PROVIDER_APIS,
  OMP_SUGGESTED_PROVIDER_PREFIX,
  isValidOmpModelId,
  ompErrorCode,
  type OmpChangedFile,
  type OmpConfigRead,
  type OmpFileStatus,
  type OmpProviderSummary,
  type OmpSavePreview,
  type OmpSaveRequest,
  type OmpSaveResult,
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
import { GroupedModelChips } from "./modelChips";
import {
  ModelsPanelHeader,
  NoticeView,
  useModelsDraftCommit,
} from "./modelsShared";
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
  type OmpDraftField,
  type OmpKeySource,
  type OmpTargetSelection,
} from "./ompModels";

const OMP_INSTALL_COMMAND = "irm https://omp.sh/install.ps1 | iex";
const OMP_OFFICIAL_URL = "https://omp.sh/";

const FILE_STATUS_COPY: Readonly<Record<OmpFileStatus, string>> = {
  ok: "已读取",
  notFound: "尚未创建，保存时新建",
  legacyJsonOnly: "仅有旧版 models.json",
  invalidYaml: "无法解析",
};

const CHANGED_FILE_COPY: Readonly<Record<OmpChangedFile, string>> = {
  models: "只更新此供应商的地址、协议、鉴权引用和所列模型",
  config: "只更新 modelRoles.default，保留其他角色",
};

const LEGACY_JSON_NOTICE: Notice = {
  tone: "error",
  title: "请先让 omp 迁移旧版配置",
  description:
    "检测到仅有 models.json。运行一次 omp 后它会自动迁移为 models.yml，然后点“重新读取”。",
};

const INVALID_YAML_NOTICE: Notice = {
  tone: "error",
  title: "omp 配置文件无法解析",
  description:
    "请先在 omp 配置目录中修复 YAML 语法，再点“重新读取”。FyAgent 不会覆盖无法解析的文件。",
};

const KEY_SOURCE_LABELS: Readonly<Record<OmpKeySource, string>> = {
  keep: "保留现有引用",
  env: "环境变量",
  command: "命令引用",
};

const FIELD_ORDER: readonly OmpDraftField[] = [
  "providerId",
  "baseUrl",
  "keyRef",
  "models",
  "defaultModel",
];

const RESTART_GUIDANCE =
  "omp 在启动时读取配置。请退出正在运行的 omp 会话后重新启动，新会话会使用这次保存的设置。";

/** Component-local only: the request may carry a key reference. */
interface PendingSave {
  request: OmpSaveRequest;
  preview: OmpSavePreview;
  revision: number;
}

function keyIntentSummary(
  request: OmpSaveRequest,
  preview: OmpSavePreview,
): string {
  const apiKey = request.provider.apiKey ?? null;
  if (apiKey === null) return keySummary(preview.provider);
  return apiKey.startsWith("!") ? "命令引用" : `环境变量 ${apiKey}`;
}

function keySummary(provider: OmpProviderSummary): string {
  if (provider.auth === "none") return "无需鉴权";
  if (provider.auth === "oauth") return "由 omp 登录管理";
  if (provider.hasCommandKey) return "命令引用";
  if (provider.hasApiKey) return "环境变量或密钥引用";
  return "未设置";
}

function errorNotice(code: ReturnType<typeof ompErrorCode>): Notice {
  switch (code) {
    case "stale":
      return {
        tone: "warning",
        title: "配置文件已被其他程序修改，未写入",
        description: "已重新读取当前配置，请检查后再次预览并保存。",
      };
    case "config":
      return {
        tone: "error",
        title: "omp 配置文件当前无法安全修改",
        description:
          "请检查 YAML 语法或旧版 models.json 迁移状态，修复后点“重新读取”再预览。",
      };
    case "invalid_input":
      return {
        tone: "error",
        title: "omp 不接受当前填写的内容",
        description: "请检查供应商 ID、服务地址、密钥引用和模型 ID 后重试。",
      };
    default:
      return {
        tone: "error",
        title: "未能生成保存预览",
        description: "草稿已保留，请稍后重试。",
      };
  }
}

export function OmpModelsPanel({
  active,
  writesBlocked,
  onBlockWrites,
}: {
  active: boolean;
  writesBlocked: boolean;
  onBlockWrites: () => void;
}) {
  const { ports } = useFeatures();
  const configQuery = useOmpConfig(active);
  const cliQuery = useOmpCliDiscovery(active);
  const read = configQuery.data;
  const [selection, setSelection] = useState<OmpTargetSelection | null>(null);
  const [pendingSelection, setPendingSelection] =
    useState<OmpTargetSelection | null>(null);
  const [edited, setEdited] = useState<{
    selection: OmpTargetSelection;
    draft: OmpDraft;
  } | null>(null);
  const [errors, setErrors] = useState<OmpDraftErrors>({});
  const [modelIdInput, setModelIdInput] = useState("");
  const [modelNameInput, setModelNameInput] = useState("");
  const [busy, setBusy] = useState<"preview" | "apply" | null>(null);
  const [notice, setNotice] = useState<Notice | null>(null);
  const [pending, setPending] = useState<PendingSave | null>(null);
  const [savedResult, setSavedResult] = useState<OmpSaveResult | null>(null);
  const writeLock = useRef(false);
  const mountedRef = useRef(true);
  const saveButtonRef = useRef<HTMLElement | null>(null);
  const providerSelectRef = useRef<HTMLElement | null>(null);
  const providerIdRef = useRef<HTMLInputElement>(null);
  const baseUrlRef = useRef<HTMLInputElement>(null);
  const keyRefInputRef = useRef<HTMLInputElement>(null);
  const modelIdRef = useRef<HTMLInputElement>(null);
  const defaultModelRef = useRef<HTMLSelectElement>(null);
  const focusField = (field: OmpDraftField) => {
    const target = {
      providerId: providerIdRef,
      baseUrl: baseUrlRef,
      keyRef: keyRefInputRef,
      models: modelIdRef,
      defaultModel: defaultModelRef,
    }[field];
    target.current?.focus();
  };
  const draftCommit = useModelsDraftCommit();

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
    };
  }, []);

  const selectionMissing =
    read !== undefined &&
    selection !== null &&
    selection !== OMP_NEW_TARGET &&
    !providerById(read, selection);
  const effectiveSelection =
    selection !== null && !selectionMissing
      ? selection
      : defaultOmpSelection(read);
  const provider = providerById(read, effectiveSelection);
  const readOnlyProvider =
    provider !== null && !isEditableOmpProvider(provider);
  const draft: OmpDraft | null =
    edited?.selection === effectiveSelection
      ? edited.draft
      : read
        ? draftFromProvider(provider, read.defaultModel)
        : null;

  const loading = configQuery.isLoading;
  const readFailed = configQuery.isError;
  const readErrorCode = readFailed ? ompErrorCode(configQuery.error) : null;
  const legacyOnly = read?.modelsFileStatus === "legacyJsonOnly";
  const invalidYaml =
    read?.modelsFileStatus === "invalidYaml" ||
    read?.configFileStatus === "invalidYaml";
  const unsafeFiles = legacyOnly || invalidYaml;
  const locked =
    busy !== null ||
    writesBlocked ||
    pending !== null ||
    loading ||
    readFailed ||
    unsafeFiles;

  const update = (patch: Partial<OmpDraft>, fields: OmpDraftField[] = []) => {
    if (!draft) return;
    setEdited({
      selection: effectiveSelection,
      draft: { ...draft, ...patch },
    });
    draftCommit.markDirty();
    setSavedResult(null);
    if (fields.length > 0)
      setErrors((current) => {
        const next = { ...current };
        for (const field of fields) delete next[field];
        return next;
      });
  };

  const applySelection = (next: OmpTargetSelection) => {
    if (!read) return;
    setSelection(next);
    setEdited(null);
    setErrors({});
    setNotice(null);
    setSavedResult(null);
    setModelIdInput("");
    setModelNameInput("");
    draftCommit.commitRevision(draftCommit.captureRevision());
  };

  const addModel = () => {
    const id = modelIdInput.trim();
    if (!isValidOmpModelId(id)) {
      setErrors((current) => ({
        ...current,
        models: "模型 ID 不能为空，也不能以 / 开头或结尾。",
      }));
      modelIdRef.current?.focus();
      return;
    }
    const name = modelNameInput.trim() || null;
    update(
      {
        models: [
          ...(draft?.models.filter((model) => model.id !== id) ?? []),
          { id, name },
        ],
      },
      ["models"],
    );
    setModelIdInput("");
    setModelNameInput("");
  };

  const requestPreview = async () => {
    if (
      writeLock.current ||
      writesBlocked ||
      !read ||
      !draft ||
      readOnlyProvider
    )
      return;
    const nextErrors = validateOmpDraft(draft, provider, read);
    setErrors(nextErrors);
    const firstInvalid = FIELD_ORDER.find((field) => nextErrors[field]);
    if (firstInvalid) {
      focusField(firstInvalid);
      return;
    }
    const request = buildOmpSaveRequest(draft, provider, read);
    const revision = draftCommit.captureRevision();
    writeLock.current = true;
    setBusy("preview");
    setNotice(null);
    setSavedResult(null);
    try {
      const preview = await ports.ompModels.previewSave(request);
      if (!mountedRef.current || !draftCommit.isCurrentRevision(revision))
        return;
      setPending({ request, preview, revision });
    } catch (cause) {
      if (!mountedRef.current) return;
      const code = ompErrorCode(cause);
      setNotice(errorNotice(code));
      if (code === "stale") void configQuery.refetch();
    } finally {
      writeLock.current = false;
      if (mountedRef.current) setBusy(null);
    }
  };

  const confirmApply = async () => {
    const current = pending;
    setPending(null);
    if (
      !current ||
      writeLock.current ||
      writesBlocked ||
      !draftCommit.isCurrentRevision(current.revision)
    )
      return;
    writeLock.current = true;
    setBusy("apply");
    setNotice(null);
    let result: OmpSaveResult | null = null;
    let nextNotice: Notice | null = null;
    let keepLocked = false;
    try {
      result = await ports.ompModels.applySave({
        planId: current.preview.planId,
        planDigest: current.preview.planDigest,
      });
      if (result.provider.id !== current.request.provider.id) {
        result = null;
        keepLocked = true;
        onBlockWrites();
        nextNotice = {
          tone: "error",
          title: "无法确认保存结果",
          description:
            "保存结果与预览的供应商不一致，已暂停继续保存。请重新打开模型页面并检查配置文件。",
        };
      }
    } catch (cause) {
      const code = ompErrorCode(cause);
      if (code === "stale" || code === "invalid_input" || code === "config") {
        nextNotice = errorNotice(code);
      } else {
        keepLocked = true;
        onBlockWrites();
        nextNotice = {
          tone: "error",
          title: "无法确认保存结果",
          description:
            "为避免覆盖 omp 配置，已暂停继续保存。请重新打开模型页面并检查配置文件。",
        };
      }
    }

    let reread: OmpConfigRead | undefined;
    try {
      const refreshed = await configQuery.refetch();
      reread = refreshed.isError ? undefined : refreshed.data;
    } catch {
      reread = undefined;
    }
    if (!mountedRef.current) {
      writeLock.current = keepLocked;
      return;
    }

    if (result) {
      const requested = current.request;
      const requestedDefault = requested.defaultModel ?? null;
      const savedProvider = reread?.providers.find(
        (item) => item.id === requested.provider.id,
      );
      const savedModelIds = new Set(
        savedProvider?.models.map((model) => model.id) ?? [],
      );
      const readbackMatches =
        savedProvider !== undefined &&
        savedProvider.baseUrl === requested.provider.baseUrl &&
        savedProvider.api === requested.provider.api &&
        requested.provider.models.every(({ id }) => savedModelIds.has(id)) &&
        (requestedDefault === null ||
          reread?.defaultModel === requestedDefault);
      draftCommit.commitRevision(current.revision);
      if (reread && savedProvider) {
        setSelection(`existing:${savedProvider.id}`);
        setEdited(null);
      }
      setSavedResult(result);
      nextNotice = readbackMatches
        ? {
            tone: "info",
            title: "已保存到 omp 配置",
            description: RESTART_GUIDANCE,
          }
        : {
            tone: "warning",
            title: "已保存，但重新读取的结果与提交内容不一致",
            description: "文件可能刚被其他程序修改，请检查后再预览并保存。",
          };
    }

    setNotice(nextNotice);
    writeLock.current = keepLocked;
    setBusy(null);
  };

  const cliStatus = cliQuery.isLoading
    ? "正在检测"
    : cliQuery.isError
      ? "无法检测"
      : cliQuery.data?.detected
        ? cliQuery.data.version
          ? `已安装 · v${cliQuery.data.version}`
          : "已安装，版本未知"
        : "未检测到";

  const finalModels = draft ? planModels(draft, provider) : [];
  const keySourceOptions: OmpKeySource[] = provider?.hasApiKey
    ? ["keep", "env", "command"]
    : ["env", "command"];

  return (
    <CatalogDetail
      className="fy-models-config-panel"
      ariaLabel="Oh My Pi 模型配置"
    >
      <ModelsPanelHeader
        title="Oh My Pi"
        summary="编辑 omp 的 models.yml 供应商，可选设置默认模型。保存前会先预览，不会改动其他供应商、注释和未修改字段。"
        pending={draftCommit.pending}
      >
        <Button
          className="fy-control-button-primary fy-models-commit-button"
          disabled={locked || readOnlyProvider || !draft}
          dialogOriginRef={saveButtonRef}
          onClick={() => void requestPreview()}
        >
          {busy === "preview"
            ? "正在预览…"
            : busy === "apply"
              ? "保存中…"
              : writesBlocked
                ? "暂时无法确认当前设置"
                : "预览并保存"}
        </Button>
      </ModelsPanelHeader>

      {writesBlocked && (
        <InlineNotice tone="warning">
          上次保存结果尚未确认，已暂停修改。请重新打开模型页面后检查配置。
        </InlineNotice>
      )}
      {selectionMissing ? (
        <InlineNotice tone="warning">
          所选供应商已不在配置中，可能已在 omp 外被删除，已切换到其他供应商。
        </InlineNotice>
      ) : null}

      <div className="fy-models-status-grid" data-testid="omp-status">
        <div className="fy-models-status-item">
          <span>omp CLI</span>
          <strong>{cliStatus}</strong>
        </div>
        <div className="fy-models-status-item">
          <span>当前 Profile</span>
          <strong>{read?.profile ?? "—"}</strong>
        </div>
        <div className="fy-models-status-item">
          <span>默认模型</span>
          <strong>{read?.defaultModel ?? "未设置"}</strong>
        </div>
      </div>

      {cliQuery.data?.executablePath ? (
        <CopyablePath
          value={cliQuery.data.executablePath}
          label="omp 程序位置"
        />
      ) : null}

      {cliQuery.data && !cliQuery.data.detected ? (
        <InlineNotice tone="info">
          未检测到可运行的 omp。可在 PowerShell 中运行官方安装命令{" "}
          <code>{OMP_INSTALL_COMMAND}</code>
          ，安装后重新打开此页。FyAgent
          不会代为执行安装；也可以先保存配置，安装后再启动 omp 使用。
          <ExternalLinkButton url={OMP_OFFICIAL_URL}>
            打开 omp 官方安装说明
          </ExternalLinkButton>
        </InlineNotice>
      ) : null}
      {cliQuery.data?.detected ? (
        <p className="fy-models-muted">
          内置供应商的账号登录请在 omp 中执行 /login；此处只管理自定义 API
          供应商。
        </p>
      ) : null}

      {loading && <Spinner label="正在读取 omp 配置" />}
      {readFailed && (
        <FieldFeedback
          notice={
            readErrorCode === "config"
              ? errorNotice("config")
              : {
                  tone: "error",
                  title: "暂时无法读取 omp 配置",
                  description: "请确认配置目录可访问后点“重新读取”。",
                }
          }
        />
      )}
      {readFailed || unsafeFiles ? (
        <Button
          disabled={configQuery.isFetching}
          onClick={() => void configQuery.refetch()}
        >
          {configQuery.isFetching ? "正在读取…" : "重新读取"}
        </Button>
      ) : null}

      {read ? (
        <ModelsSection title="配置文件" titleId="omp-files-title">
          <dl className="fy-models-omp-files">
            <div>
              <dt>models</dt>
              <dd>
                <code>{read.modelsPath}</code>
                <span>{FILE_STATUS_COPY[read.modelsFileStatus]}</span>
              </dd>
            </div>
            <div>
              <dt>config</dt>
              <dd>
                <code>{read.configPath}</code>
                <span>{FILE_STATUS_COPY[read.configFileStatus]}</span>
              </dd>
            </div>
          </dl>
          <p className="fy-models-muted">
            路径按 FyAgent 启动时的 OMP_PROFILE、PI_PROFILE、PI_CONFIG_DIR 和
            PI_CODING_AGENT_DIR 解析；命令行 --profile 只影响那一次运行。
          </p>
          {legacyOnly ? (
            <FieldFeedback notice={LEGACY_JSON_NOTICE} />
          ) : invalidYaml ? (
            <FieldFeedback notice={INVALID_YAML_NOTICE} />
          ) : null}
        </ModelsSection>
      ) : null}

      {read && draft && !unsafeFiles ? (
        <>
          <label className="fy-control-field" htmlFor="omp-provider-target">
            供应商
            <select
              ref={(node) => {
                providerSelectRef.current = node;
              }}
              id="omp-provider-target"
              className="fy-control-select"
              value={effectiveSelection}
              disabled={locked}
              onChange={(event) => {
                const next = event.target.value as OmpTargetSelection;
                if (draftCommit.pending) setPendingSelection(next);
                else applySelection(next);
              }}
            >
              {read.providers.map((item) => (
                <option key={item.id} value={`existing:${item.id}`}>
                  {item.id}
                  {isEditableOmpProvider(item) ? "" : "（只读）"}
                </option>
              ))}
              <option value={OMP_NEW_TARGET}>新建供应商</option>
            </select>
          </label>

          {readOnlyProvider && provider ? (
            <InlineNotice tone="info">
              该供应商使用 {provider.api} 协议，FyAgent 暂不编辑这种格式，请在
              omp 中修改。下方仅显示其模型。
            </InlineNotice>
          ) : null}

          {provider ? (
            <ModelsSection
              title="文件中的现有设置"
              titleId="omp-existing-title"
            >
              <div className="fy-models-status-grid">
                <div className="fy-models-status-item">
                  <span>服务地址</span>
                  <strong>{provider.baseUrl ?? "未设置"}</strong>
                </div>
                <div className="fy-models-status-item">
                  <span>协议</span>
                  <strong>{provider.api ?? "未设置"}</strong>
                </div>
                <div className="fy-models-status-item">
                  <span>鉴权</span>
                  <strong>{keySummary(provider)}</strong>
                </div>
              </div>
              <GroupedModelChips
                ids={provider.models.map((model) => model.id)}
                emptyLabel="该供应商还没有自定义模型。"
              />
            </ModelsSection>
          ) : null}

          {!readOnlyProvider ? (
            <ModelsSection
              title={provider ? "修改此供应商" : "新供应商"}
              titleId="omp-form-title"
            >
              <div className="fy-models-form">
                {!provider ? (
                  <div className="fy-control-field">
                    <label htmlFor="omp-provider-id">供应商 ID</label>
                    <Input
                      ref={providerIdRef}
                      id="omp-provider-id"
                      value={draft.providerId}
                      disabled={locked}
                      onChange={(event) =>
                        update({ providerId: event.target.value }, [
                          "providerId",
                        ])
                      }
                      autoComplete="off"
                      spellCheck={false}
                      aria-invalid={Boolean(errors.providerId)}
                      aria-describedby="omp-provider-id-hint"
                    />
                    <span id="omp-provider-id-hint" className="fy-models-muted">
                      建议以 {OMP_SUGGESTED_PROVIDER_PREFIX} 开头，避免与 omp
                      内置供应商重名。保存后不可在此改名。
                    </span>
                    {errors.providerId ? (
                      <span className="fy-control-field-error" role="alert">
                        {errors.providerId}
                      </span>
                    ) : null}
                  </div>
                ) : null}

                <div className="fy-control-field">
                  <label htmlFor="omp-base-url">服务地址</label>
                  <Input
                    ref={baseUrlRef}
                    id="omp-base-url"
                    type="url"
                    value={draft.baseUrl}
                    disabled={locked}
                    onChange={(event) =>
                      update({ baseUrl: event.target.value }, ["baseUrl"])
                    }
                    placeholder="https://gateway.example/v1"
                    autoComplete="off"
                    spellCheck={false}
                    aria-invalid={Boolean(errors.baseUrl)}
                  />
                  {errors.baseUrl ? (
                    <span className="fy-control-field-error" role="alert">
                      {errors.baseUrl}
                    </span>
                  ) : null}
                </div>

                <label className="fy-control-field" htmlFor="omp-api">
                  协议
                  <select
                    id="omp-api"
                    className="fy-control-select"
                    value={draft.api}
                    disabled={locked}
                    onChange={(event) =>
                      update({
                        api: event.target.value as OmpDraft["api"],
                      })
                    }
                  >
                    {OMP_PROVIDER_APIS.map((api) => (
                      <option key={api} value={api}>
                        {OMP_API_LABELS[api]}（{api}）
                      </option>
                    ))}
                  </select>
                </label>

                <div className="fy-control-field fy-models-form-wide">
                  <span id="omp-auth-label">鉴权</span>
                  {draft.auth === "oauth" ? (
                    <p className="fy-models-muted">
                      此供应商由 omp 登录管理，保存时保持不变。
                    </p>
                  ) : (
                    <div
                      className="fy-models-inline-fields"
                      role="group"
                      aria-labelledby="omp-auth-label"
                    >
                      <Button
                        aria-pressed={draft.auth === "apiKey"}
                        disabled={locked}
                        onClick={() => update({ auth: "apiKey" }, ["keyRef"])}
                      >
                        密钥引用
                      </Button>
                      <Button
                        aria-pressed={draft.auth === "none"}
                        disabled={locked}
                        onClick={() => update({ auth: "none" }, ["keyRef"])}
                      >
                        无需鉴权
                      </Button>
                    </div>
                  )}
                </div>

                {draft.auth === "apiKey" ? (
                  <div className="fy-control-field fy-models-form-wide">
                    <span id="omp-key-source-label">密钥来源</span>
                    <div
                      className="fy-models-inline-fields"
                      role="group"
                      aria-labelledby="omp-key-source-label"
                    >
                      {keySourceOptions.map((source) => (
                        <Button
                          key={source}
                          aria-pressed={draft.keySource === source}
                          disabled={locked}
                          onClick={() =>
                            update({ keySource: source }, ["keyRef"])
                          }
                        >
                          {KEY_SOURCE_LABELS[source]}
                        </Button>
                      ))}
                    </div>
                    {draft.keySource === "keep" ? (
                      <p className="fy-models-muted">
                        文件中的 apiKey 原样保留，界面不显示原值。
                      </p>
                    ) : draft.keySource === "env" ? (
                      <>
                        <label htmlFor="omp-env-name">环境变量名</label>
                        <Input
                          ref={keyRefInputRef}
                          id="omp-env-name"
                          value={draft.envName}
                          disabled={locked}
                          onChange={(event) =>
                            update({ envName: event.target.value }, ["keyRef"])
                          }
                          placeholder="MY_GATEWAY_API_KEY"
                          autoComplete="off"
                          spellCheck={false}
                          aria-invalid={Boolean(errors.keyRef)}
                        />
                        <p className="fy-models-muted">
                          只写入变量名，不保存密钥本身。若未设置该变量，omp
                          会将变量名作为密钥发送，请先在运行 omp
                          的终端设置变量。
                        </p>
                      </>
                    ) : (
                      <>
                        <label htmlFor="omp-command-ref">命令引用</label>
                        <Input
                          ref={keyRefInputRef}
                          id="omp-command-ref"
                          value={draft.commandRef}
                          disabled={locked}
                          onChange={(event) =>
                            update({ commandRef: event.target.value }, [
                              "keyRef",
                            ])
                          }
                          placeholder="!op read op://vault/gateway/key"
                          autoComplete="off"
                          spellCheck={false}
                          aria-invalid={Boolean(errors.keyRef)}
                        />
                        <p className="fy-models-muted">
                          omp 在需要密钥时执行这条命令。FyAgent
                          只原样写入，不会执行它。
                        </p>
                      </>
                    )}
                    {errors.keyRef ? (
                      <span className="fy-control-field-error" role="alert">
                        {errors.keyRef}
                      </span>
                    ) : null}
                  </div>
                ) : null}
              </div>
            </ModelsSection>
          ) : null}

          {!readOnlyProvider ? (
            <section
              className="fy-models-draft"
              data-testid="omp-draft-models"
              data-invalid={Boolean(errors.models) || undefined}
              aria-label="本次添加或更新的模型"
            >
              <h3>本次添加或更新的模型</h3>
              <p className="fy-models-muted">
                已有模型会保留；填写相同 ID 可更新显示名称。模型 ID 可包含 /。
              </p>
              <GroupedModelChips
                ids={draft.models.map((model) => model.id)}
                removable
                removeDisabled={locked}
                onRemove={(id) =>
                  update({
                    models: draft.models.filter((model) => model.id !== id),
                  })
                }
                emptyLabel={
                  provider
                    ? "未添加新模型，保存时只更新上方连接设置。"
                    : "请至少添加一个模型。"
                }
              />
              <div className="fy-models-manual-row">
                <label className="fy-control-field fy-models-manual-field">
                  模型 ID
                  <Input
                    ref={modelIdRef}
                    id="omp-model-id"
                    value={modelIdInput}
                    disabled={locked}
                    onChange={(event) => setModelIdInput(event.target.value)}
                    onKeyDown={(event) => {
                      if (event.key === "Enter") {
                        event.preventDefault();
                        addModel();
                      }
                    }}
                    placeholder="vendor/model-name"
                    autoComplete="off"
                    spellCheck={false}
                    aria-invalid={Boolean(errors.models)}
                  />
                </label>
                <label className="fy-control-field fy-models-manual-field">
                  显示名称（可选）
                  <Input
                    id="omp-model-name"
                    value={modelNameInput}
                    disabled={locked}
                    onChange={(event) => setModelNameInput(event.target.value)}
                    onKeyDown={(event) => {
                      if (event.key === "Enter") {
                        event.preventDefault();
                        addModel();
                      }
                    }}
                    autoComplete="off"
                  />
                </label>
                <Button disabled={locked} onClick={addModel}>
                  添加
                </Button>
              </div>
              {errors.models ? (
                <span className="fy-control-field-error" role="alert">
                  {errors.models}
                </span>
              ) : null}

              <div className="fy-models-checkbox-row">
                <Checkbox
                  checked={draft.setDefault}
                  disabled={locked || finalModels.length === 0}
                  onCheckedChange={(checked) =>
                    update(
                      {
                        setDefault: checked,
                        defaultModelId:
                          draft.defaultModelId || (finalModels[0]?.id ?? ""),
                      },
                      ["defaultModel"],
                    )
                  }
                  label="同时设为 omp 默认模型"
                />
                <span>
                  同时设为 omp 默认模型（写入 config.yml 的 modelRoles.default）
                </span>
              </div>
              {draft.setDefault ? (
                <label className="fy-control-field" htmlFor="omp-default-model">
                  默认模型
                  <select
                    ref={defaultModelRef}
                    id="omp-default-model"
                    className="fy-control-select"
                    value={draft.defaultModelId}
                    disabled={locked}
                    onChange={(event) =>
                      update({ defaultModelId: event.target.value }, [
                        "defaultModel",
                      ])
                    }
                  >
                    <option value="">请选择</option>
                    {finalModels.map((model) => (
                      <option key={model.id} value={model.id}>
                        {model.id}
                      </option>
                    ))}
                  </select>
                  {errors.defaultModel ? (
                    <span className="fy-control-field-error" role="alert">
                      {errors.defaultModel}
                    </span>
                  ) : null}
                </label>
              ) : null}
            </section>
          ) : null}
        </>
      ) : null}

      <NoticeView notice={notice} />
      {savedResult ? (
        <ul className="fy-models-omp-result" aria-label="omp 保存结果">
          <li>
            models：<code>{savedResult.modelsPath}</code>
            {savedResult.modelsWritten ? " 已写入" : " 保持不变"}
          </li>
          <li>
            config：<code>{savedResult.configPath}</code>
            {savedResult.configWritten ? " 已写入" : " 保持不变"}
          </li>
        </ul>
      ) : null}

      <Dialog
        open={pending !== null}
        originRef={saveButtonRef}
        size="comfortable"
        title="保存前确认"
        description="以下是即将写入的变更。确认后才会写入，写入前会在同目录保留备份。"
        onOpenChange={(open) => {
          if (!open) setPending(null);
        }}
        actions={
          <>
            <Button onClick={() => setPending(null)}>取消</Button>
            <Button
              className="fy-control-button-primary"
              onClick={() => void confirmApply()}
            >
              确认保存
            </Button>
          </>
        }
      >
        {pending ? (
          <div className="fy-models-omp-preview">
            <p>
              Profile：<strong>{pending.preview.profile}</strong>
            </p>
            {pending.preview.changedFiles.length > 0 ? (
              <ul aria-label="将修改的文件">
                {pending.preview.changedFiles.map((file) => (
                  <li key={file}>
                    <code>
                      {file === "models"
                        ? pending.preview.modelsPath
                        : pending.preview.configPath}
                    </code>
                    <span>
                      {CHANGED_FILE_COPY[file]}：
                      <strong>
                        {file === "models"
                          ? pending.preview.provider.id
                          : (pending.preview.defaultModel ?? "未设置")}
                      </strong>
                    </span>
                  </li>
                ))}
              </ul>
            ) : (
              <p>预览未发现需要写入的变化。</p>
            )}
            {pending.preview.warnings.length > 0 ? (
              <InlineNotice tone="warning">
                <ul aria-label="预览提示">
                  {pending.preview.warnings.map((warning) => (
                    <li key={warning}>{warning}</li>
                  ))}
                </ul>
              </InlineNotice>
            ) : null}
            <dl className="fy-models-omp-files">
              <div>
                <dt>服务地址</dt>
                <dd>{pending.preview.provider.baseUrl ?? "未设置"}</dd>
              </div>
              <div>
                <dt>协议</dt>
                <dd>{pending.preview.provider.api ?? "未设置"}</dd>
              </div>
              <div>
                <dt>鉴权</dt>
                <dd>{keyIntentSummary(pending.request, pending.preview)}</dd>
              </div>
              <div>
                <dt>保存后的模型</dt>
                <dd>{pending.preview.provider.models.length} 个</dd>
              </div>
            </dl>
            <p className="fy-models-muted">
              其他供应商、注释、未知字段和未修改的鉴权引用都保持原样。
            </p>
          </div>
        ) : null}
      </Dialog>

      <Dialog
        open={pendingSelection !== null}
        originRef={providerSelectRef}
        title="放弃未保存的修改？"
        description="切换供应商会清除当前草稿。"
        onOpenChange={(open) => {
          if (!open) setPendingSelection(null);
        }}
        actions={
          <>
            <Button onClick={() => setPendingSelection(null)}>继续编辑</Button>
            <Button
              className="fy-control-button-danger"
              onClick={() => {
                const next = pendingSelection;
                setPendingSelection(null);
                if (next) applySelection(next);
              }}
            >
              放弃并切换
            </Button>
          </>
        }
      />
    </CatalogDetail>
  );
}
