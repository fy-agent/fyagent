import { CaretDownIcon } from "@phosphor-icons/react/dist/csr/CaretDown";
import { useEffect, useMemo, useRef, useState } from "react";
import { useLocation, useSearchParams } from "react-router-dom";

import { getAgentBrand, type AgentIconId } from "../../shared/assets/agents";
import { classNames } from "../../shared/design-system/classNames";
import type { FeaturePorts } from "../../shared/features/ports";
import { useFeatures } from "../../shared/features/provider";
import {
  useProviderSummary,
  useWorkBuddyModelIds,
  useWorkBuddyStatus,
} from "../../shared/features/queries";
import type {
  CodexProviderMutationWarning,
  ProviderAppId,
} from "../../shared/features/types";
import { PersistentSurface } from "../../shared/ui/PersistentSurface";
import {
  Button,
  Checkbox,
  Dialog,
  InlineNotice,
  Input,
  SecretInput,
  Spinner,
} from "../../shared/ui/primitives";
import {
  CatalogDetail,
  CatalogList,
  CatalogListItem,
  CatalogMasterDetail,
  CatalogRail,
} from "../../shared/ui/catalog";
import {
  FieldFeedback,
  focusControl,
  isErrorNotice,
  ModelsSection,
  useFieldNotices,
  type Notice,
} from "./feedback";
import {
  buildQuickSetupRequest,
  CLAUDE_EXPLICIT_V1_WARNING,
  claudeBaseUrlHasExplicitV1Path,
  isHttpUrl,
  MODEL_TARGETS,
  parseManualModelIds,
  parseModelTarget,
  QUICK_SETUP_PROVIDER_IDS,
  validateQuickSetup,
  type ModelTarget,
  type QuickSetupErrors,
} from "./quickSetup";
import {
  GroupedModelChips,
  ModelSearchField,
  ModelVendorIcon,
} from "./modelChips";
import { ModelConnectivityTest } from "./ModelConnectivityTest";
import { ModelsPanelHeader, NoApiKeyOption, NoticeView } from "./modelsShared";
import { OpenCodeModelsPanel } from "./OpenCodeModelsPanel";
import { QoderModelsPanel } from "./QoderModelsPanel";
import { TraeModelsPanel } from "./TraeModelsPanel";
import {
  addUniqueModelIds,
  filterModelIds,
  nativeErrorCode,
  splitWorkBuddyDraft,
} from "./workBuddyModels";
import "./Page.css";

type WorkBuddySaveRequest = Parameters<
  FeaturePorts["workbuddy"]["saveModels"]
>[0];

const EMPTY_MODEL_IDS: readonly string[] = [];

const TARGET_LABELS: Record<ModelTarget, string> = {
  qoderwork: "QoderWork CN",
  trae: "TRAE Work CN",
  workbuddy: "WorkBuddy",
  grokbuild: "Grok Build",
  codex: "Codex",
  claude: "Claude Code",
  opencode: "OpenCode",
};

const TARGET_ICON_IDS: Readonly<Record<ModelTarget, AgentIconId>> = {
  qoderwork: "qoderwork",
  trae: "trae-work",
  workbuddy: "workbuddy",
  grokbuild: "grokbuild",
  codex: "codex",
  claude: "claude-code",
  opencode: "opencode",
};

type WorkBuddyNoticeField =
  | "baseUrl"
  | "apiKey"
  | "fetch"
  | "draft"
  | "save"
  | "existing";

function WorkBuddyPanel({ active }: { active: boolean }) {
  const { ports } = useFeatures();
  const statusQuery = useWorkBuddyStatus(active);
  const modelIdsQuery = useWorkBuddyModelIds(active);
  const [baseUrl, setBaseUrl] = useState("");
  const [apiKey, setApiKeyState] = useState("");
  const apiKeyRef = useRef("");
  const [allowNoApiKey, setAllowNoApiKey] = useState(false);
  const [manualDraft, setManualDraft] = useState("");
  const [draftModelIds, setDraftModelIds] = useState<string[]>([]);
  const [fetchedSourceIds, setFetchedSourceIds] = useState<Set<string>>(
    () => new Set(),
  );
  const [existingSearch, setExistingSearch] = useState("");
  const [draftSearch, setDraftSearch] = useState("");
  const [existingOpen, setExistingOpen] = useState(false);
  const [truncated, setTruncated] = useState(false);
  const [busy, setBusy] = useState<
    "fetch" | "save" | "delete" | "reachability" | null
  >(null);
  const { notices, show, clear, dismiss } =
    useFieldNotices<WorkBuddyNoticeField>();
  const [pendingOverwrite, setPendingOverwrite] = useState<{
    request: WorkBuddySaveRequest;
    token: string;
    existingIds: string[];
  } | null>(null);
  const [pendingDeleteId, setPendingDeleteId] = useState<string | null>(null);
  const writeLock = useRef(false);
  const mountedRef = useRef(true);
  const baseUrlInputRef = useRef<HTMLInputElement>(null);
  const apiKeyInputRef = useRef<HTMLInputElement>(null);
  const manualModelsInputRef = useRef<HTMLInputElement>(null);

  const setApiKey = (value: string) => {
    apiKeyRef.current = value;
    setApiKeyState(value);
  };
  const clearApiKey = () => {
    apiKeyRef.current = "";
    if (mountedRef.current) setApiKeyState("");
  };

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      apiKeyRef.current = "";
    };
  }, []);

  const refreshAuthoritativeState = async (): Promise<boolean> => {
    try {
      const [statusResult, modelIdsResult] = await Promise.all([
        statusQuery.refetch(),
        modelIdsQuery.refetch(),
      ]);
      return (
        !statusResult.isError &&
        statusResult.data !== undefined &&
        !modelIdsResult.isError &&
        modelIdsResult.data !== undefined
      );
    } catch {
      return false;
    }
  };

  const validateConnection = (): boolean => {
    if (!isHttpUrl(baseUrl.trim())) {
      show("baseUrl", {
        tone: "error",
        title: "请输入有效的服务地址",
        description: "只接受不含账号信息的 HTTP(S) 地址。",
      });
      focusControl(baseUrlInputRef.current);
      return false;
    }
    if (!allowNoApiKey && !apiKeyRef.current.trim()) {
      show("apiKey", { tone: "error", title: "请输入 API Key" });
      focusControl(apiKeyInputRef.current);
      return false;
    }
    const credential = apiKeyRef.current.trim();
    if (credential) {
      const parsed = new URL(baseUrl.trim());
      const collision =
        parsed.hostname.includes(credential.toLocaleLowerCase("en-US")) ||
        parsed.pathname.split("/").some((segment) => {
          if (segment.includes(credential)) return true;
          try {
            return decodeURIComponent(segment).includes(credential);
          } catch {
            return false;
          }
        });
      if (collision) {
        show("baseUrl", {
          tone: "error",
          title: "服务地址不能包含 API Key",
        });
        focusControl(baseUrlInputRef.current);
        return false;
      }
    }
    return true;
  };

  const fetchModels = async () => {
    if (writeLock.current || !validateConnection()) return;
    const submittedApiKey = apiKeyRef.current.trim();
    writeLock.current = true;
    setBusy("fetch");
    clear();
    try {
      const result = await ports.workbuddy.fetchModels({
        baseUrl: baseUrl.trim(),
        apiKey: submittedApiKey,
        allowNoApiKey,
      });
      if (!mountedRef.current) return;
      if (
        submittedApiKey &&
        result.models.some((modelId) =>
          modelId.trim().includes(submittedApiKey),
        )
      ) {
        throw new Error("credential-model-id-conflict");
      }
      setDraftModelIds((current) => addUniqueModelIds(current, result.models));
      setFetchedSourceIds(new Set(result.models));
      setTruncated(result.truncated);
      show("fetch", {
        tone: result.truncated ? "warning" : "info",
        title: result.truncated
          ? "已达到可显示的模型数量上限"
          : `已读取 ${result.models.length} 个模型`,
        description: "请确认选择后再保存。",
      });
    } catch {
      if (mountedRef.current)
        show("fetch", {
          tone: "error",
          title: "模型读取失败",
          description: "请检查地址、凭据和服务状态后重试。",
        });
    } finally {
      if (mountedRef.current) setBusy(null);
      writeLock.current = false;
    }
  };

  const collectDraftIds = (): string[] =>
    addUniqueModelIds(draftModelIds, parseManualModelIds(manualDraft));

  const fillManualModels = () => {
    const pending = parseManualModelIds(manualDraft);
    if (pending.length === 0) {
      show("draft", { tone: "error", title: "请输入模型 ID" });
      focusControl(manualModelsInputRef.current);
      return;
    }
    const submittedApiKey = apiKeyRef.current.trim();
    if (
      submittedApiKey &&
      pending.some((modelId) => modelId.includes(submittedApiKey))
    ) {
      show("draft", {
        tone: "error",
        title: "模型 ID 不能包含 API Key",
        description: "请检查模型 ID 后重试。",
      });
      focusControl(manualModelsInputRef.current);
      return;
    }
    setDraftModelIds((current) => addUniqueModelIds(current, pending));
    setManualDraft("");
    dismiss("draft");
  };

  const clearDraftModels = () => {
    setDraftModelIds([]);
    setFetchedSourceIds(new Set());
    setTruncated(false);
  };

  const buildSaveRequest = (draftIds: string[]): WorkBuddySaveRequest => {
    const { selectedModelIds, manualModelIds } = splitWorkBuddyDraft(
      draftIds,
      fetchedSourceIds,
    );
    const request = {
      baseUrl: baseUrl.trim(),
      apiKey: apiKeyRef.current.trim(),
      allowNoApiKey,
      selectedModelIds,
      manualModelIds,
      removedModelIds: [],
      clearExistingApiKeys: false,
      expectedRevision:
        modelIdsQuery.data?.revision ?? statusQuery.data?.revision ?? null,
    } satisfies WorkBuddySaveRequest;

    Object.freeze(request.selectedModelIds);
    Object.freeze(request.manualModelIds);
    Object.freeze(request.removedModelIds);
    return Object.freeze(request);
  };

  const saveRequest = async (request: WorkBuddySaveRequest) => {
    if (writeLock.current) return;
    writeLock.current = true;
    setBusy("save");
    clear();
    let shouldRefresh = true;
    let rereadNotice: {
      confirmed: Notice;
      unconfirmed: Notice;
    } | null = null;
    try {
      const result = await ports.workbuddy.saveModels(request);
      if (!mountedRef.current) return;

      switch (result.state) {
        case "saved":
          setPendingOverwrite(null);
          show("save", {
            tone: "info",
            title: "WorkBuddy 模型配置已保存",
            description: `共 ${result.modelCount} 个模型；新增 ${result.createdEntries}，更新 ${result.updatedEntries}。`,
          });
          break;
        case "concurrent_modification":
          setPendingOverwrite(null);
          rereadNotice = {
            confirmed: {
              tone: "warning",
              title: "配置已被其他操作修改",
              description: "已刷新当前设置，请检查后再次提交。",
            },
            unconfirmed: {
              tone: "warning",
              title: "配置已被其他操作修改",
              description: "暂时无法刷新当前设置，请刷新后再次提交。",
            },
          };
          break;
        case "overwrite_confirmation_required":
          if (request.overwriteToken) {
            setPendingOverwrite(null);
            rereadNotice = {
              confirmed: {
                tone: "error",
                title: "覆盖确认已失效",
                description: "已刷新当前设置，请重新提交。",
              },
              unconfirmed: {
                tone: "error",
                title: "覆盖确认已失效",
                description: "暂时无法刷新当前设置，请刷新后重新提交。",
              },
            };
          } else {
            shouldRefresh = false;
            setPendingOverwrite({
              request,
              token: result.token,
              existingIds: [...result.existingIds],
            });
          }
          break;
      }
    } catch (error) {
      if (mountedRef.current) {
        setPendingOverwrite(null);
        const code = nativeErrorCode(error);
        if (
          request.overwriteToken &&
          (code === "WORKBUDDY_OVERWRITE_TOKEN_EXPIRED" ||
            code === "WORKBUDDY_OVERWRITE_TOKEN_INVALID")
        ) {
          rereadNotice = {
            confirmed: {
              tone: "error",
              title: "覆盖确认已失效",
              description: "已刷新当前设置，请重新提交。",
            },
            unconfirmed: {
              tone: "error",
              title: "覆盖确认已失效",
              description: "暂时无法刷新当前设置，请刷新后重新提交。",
            },
          };
        } else {
          show("save", {
            tone: "error",
            title: "保存失败",
            description: "请刷新当前设置、检查输入后重试。",
          });
        }
      }
    } finally {
      clearApiKey();
      const rereadConfirmed = shouldRefresh
        ? await refreshAuthoritativeState()
        : false;
      if (mountedRef.current && rereadNotice) {
        show(
          "save",
          rereadConfirmed ? rereadNotice.confirmed : rereadNotice.unconfirmed,
        );
      }
      if (mountedRef.current) setBusy(null);
      writeLock.current = false;
    }
  };

  const startSave = () => {
    if (writeLock.current) return;
    const draftIds = collectDraftIds();
    const hasDraft = draftIds.length > 0;
    if (!hasDraft) {
      show("draft", {
        tone: "error",
        title: "请至少添加一个模型 ID",
      });
      focusControl(manualModelsInputRef.current);
      return;
    }
    if (!validateConnection()) return;
    const request = buildSaveRequest(draftIds);
    const submittedApiKey = request.apiKey.trim();
    if (
      submittedApiKey &&
      [...request.selectedModelIds, ...request.manualModelIds].some((modelId) =>
        modelId.trim().includes(submittedApiKey),
      )
    ) {
      clearApiKey();
      show("draft", {
        tone: "error",
        title: "模型 ID 不能包含 API Key",
        description: "请检查模型 ID 后重试。",
      });
      focusControl(manualModelsInputRef.current);
      return;
    }
    if (parseManualModelIds(manualDraft).length > 0) {
      setDraftModelIds(draftIds);
      setManualDraft("");
    }
    void saveRequest(request);
  };

  const confirmOverwrite = () => {
    if (!pendingOverwrite || writeLock.current) return;
    const frozen = pendingOverwrite;
    setPendingOverwrite(null);
    void saveRequest({
      ...frozen.request,
      overwriteToken: frozen.token,
    });
  };

  const deleteExistingModel = async (modelId: string) => {
    if (writeLock.current) return;
    writeLock.current = true;
    setBusy("delete");
    dismiss("existing");
    const selectedModelIds: string[] = [];
    const manualModelIds: string[] = [];
    const removedModelIds = [modelId];
    const request = {
      baseUrl: "",
      apiKey: "",
      allowNoApiKey: false,
      selectedModelIds,
      manualModelIds,
      removedModelIds,
      clearExistingApiKeys: false,
      expectedRevision:
        modelIdsQuery.data?.revision ?? statusQuery.data?.revision ?? null,
    } satisfies WorkBuddySaveRequest;
    Object.freeze(request.selectedModelIds);
    Object.freeze(request.manualModelIds);
    Object.freeze(request.removedModelIds);

    let notice: Notice | null = null;
    try {
      let result = await ports.workbuddy.saveModels(request);
      if (result.state === "overwrite_confirmation_required" && result.token) {
        result = await ports.workbuddy.saveModels({
          ...request,
          overwriteToken: result.token,
        });
      }
      if (!mountedRef.current) return;
      switch (result.state) {
        case "saved":
          setPendingDeleteId(null);
          notice = { tone: "info", title: "已删除该模型配置" };
          break;
        case "concurrent_modification":
          notice = {
            tone: "warning",
            title: "配置已被其他操作修改",
            description: "已刷新当前设置，请检查后再删除。",
          };
          break;
        case "overwrite_confirmation_required":
          notice = {
            tone: "error",
            title: "删除确认已失效",
            description: "请刷新当前设置后重新删除。",
          };
          break;
      }
    } catch {
      if (mountedRef.current) {
        notice = {
          tone: "error",
          title: "删除失败",
          description: "请刷新当前设置后重试。",
        };
      }
    } finally {
      await refreshAuthoritativeState();
      if (mountedRef.current && notice) show("existing", notice);
      if (mountedRef.current) setBusy(null);
      writeLock.current = false;
    }
  };

  const modelIds = modelIdsQuery.data?.ids ?? EMPTY_MODEL_IDS;
  const filteredExistingIds = useMemo(
    () => filterModelIds(modelIds, existingSearch),
    [modelIds, existingSearch],
  );
  const filteredDraftIds = useMemo(
    () => filterModelIds(draftModelIds, draftSearch),
    [draftModelIds, draftSearch],
  );
  const loading = statusQuery.isLoading || modelIdsQuery.isLoading;
  const readFailed = statusQuery.isError || modelIdsQuery.isError;

  return (
    <CatalogDetail
      className="fy-models-config-panel"
      ariaLabel="WorkBuddy 模型配置"
    >
      <ModelsPanelHeader
        title="WorkBuddy"
        summary="查看并管理 WorkBuddy 的模型设置。添加或修改后请保存并应用。"
        pending={
          draftModelIds.length > 0 ||
          Boolean(manualDraft.trim()) ||
          Boolean(baseUrl.trim()) ||
          Boolean(apiKey.trim())
        }
      >
        <Button
          className="fy-control-button-primary fy-models-commit-button"
          disabled={busy !== null || loading || readFailed}
          onClick={startSave}
        >
          {busy === "save" ? "保存中…" : "保存并应用"}
        </Button>
      </ModelsPanelHeader>
      <FieldFeedback id="workbuddy-save-error" notice={notices.save} />

      {loading && <Spinner label="正在读取 WorkBuddy 状态" />}
      {readFailed && (
        <InlineNotice tone="error">
          暂时无法读取 WorkBuddy 配置，请重试。
        </InlineNotice>
      )}
      <section
        className="fy-models-existing"
        data-testid="workbuddy-model-ids"
        data-invalid={isErrorNotice(notices.existing) || undefined}
        aria-label="当前已有的第三方模型 ID"
      >
        <button
          type="button"
          className="fy-models-existing-toggle"
          data-testid="workbuddy-status"
          aria-expanded={existingOpen}
          onClick={() => setExistingOpen((open) => !open)}
        >
          <h3>当前已有的第三方模型 ID</h3>
          <span className="fy-models-existing-meta">
            <span>已有第三方模型数量</span>
            <strong className="fy-models-existing-count">
              {modelIds.length}
            </strong>
            <CaretDownIcon
              className={classNames(
                "fy-models-caret",
                existingOpen && "fy-models-caret-open",
              )}
              size={18}
              aria-hidden
            />
          </span>
        </button>
        {existingOpen ? (
          <>
            {modelIds.length > 0 ? (
              <ModelSearchField
                id="workbuddy-existing-search"
                label="搜索已有模型"
                value={existingSearch}
                onChange={setExistingSearch}
              />
            ) : null}
            <GroupedModelChips
              ids={filteredExistingIds}
              removable
              removeDisabled={busy !== null || loading || readFailed}
              onRemove={(modelId) => {
                if (busy !== null || writeLock.current) return;
                setPendingDeleteId(modelId);
              }}
              emptyLabel={
                existingSearch.trim() ? "没有匹配的模型 ID" : "未观察到模型 ID"
              }
            />
            <FieldFeedback
              id="workbuddy-existing-error"
              notice={notices.existing}
            />
          </>
        ) : null}
      </section>

      <ModelsSection
        title="连接设置"
        titleId="workbuddy-connection-title"
        invalid={
          isErrorNotice(notices.baseUrl) || isErrorNotice(notices.apiKey)
        }
      >
        <div className="fy-models-form">
          <label className="fy-control-field">
            服务地址
            <Input
              ref={baseUrlInputRef}
              id="workbuddy-base-url"
              name="workbuddy-base-url"
              type="url"
              value={baseUrl}
              onChange={(event) => {
                setBaseUrl(event.target.value);
                dismiss("baseUrl");
              }}
              placeholder="https://gateway.example/v1"
              autoComplete="off"
              spellCheck={false}
              aria-invalid={isErrorNotice(notices.baseUrl)}
              aria-describedby={
                notices.baseUrl ? "workbuddy-base-url-error" : undefined
              }
            />
            <FieldFeedback
              id="workbuddy-base-url-error"
              notice={notices.baseUrl}
            />
          </label>
          <div className="fy-control-field">
            <label htmlFor="workbuddy-api-key">API Key</label>
            <SecretInput
              ref={apiKeyInputRef}
              id="workbuddy-api-key"
              name="workbuddy-api-key"
              value={apiKey}
              onChange={(event) => {
                setApiKey(event.target.value);
                dismiss("apiKey");
              }}
              autoComplete="off"
              spellCheck={false}
              aria-invalid={isErrorNotice(notices.apiKey)}
              aria-describedby={
                notices.apiKey ? "workbuddy-api-key-error" : undefined
              }
              revealLabel="显示 API Key"
              hideLabel="隐藏 API Key"
            />
            <FieldFeedback
              id="workbuddy-api-key-error"
              notice={notices.apiKey}
            />
          </div>
          <NoApiKeyOption
            checked={allowNoApiKey}
            onCheckedChange={(checked) => {
              setAllowNoApiKey(checked);
              if (checked) dismiss("apiKey");
            }}
            disabled={busy !== null}
          />
        </div>
      </ModelsSection>

      <section
        className="fy-models-draft"
        data-testid="workbuddy-draft-models"
        data-invalid={isErrorNotice(notices.draft) || undefined}
        aria-label="待保存的模型 ID"
      >
        <h3>待保存的模型 ID</h3>
        {truncated ? (
          <p className="fy-models-muted">已达到可显示的模型数量上限。</p>
        ) : null}
        {draftModelIds.length > 0 ? (
          <ModelSearchField
            id="workbuddy-draft-search"
            label="搜索待保存模型"
            value={draftSearch}
            onChange={setDraftSearch}
          />
        ) : null}
        <GroupedModelChips
          ids={filteredDraftIds}
          removable
          removeDisabled={busy !== null}
          onRemove={(modelId) =>
            setDraftModelIds((current) =>
              current.filter((id) => id !== modelId),
            )
          }
          emptyLabel={
            draftSearch.trim()
              ? "没有匹配的模型 ID"
              : "尚未添加模型。可拉取远程模型，或手动填入模型 ID。"
          }
        />
        <div className="fy-models-action-block">
          <div className="fy-models-actions">
            <ModelConnectivityTest
              searchId="workbuddy-probe-search"
              modelIds={draftModelIds}
              disabled={busy !== null && busy !== "reachability"}
              onPrepare={validateConnection}
              onBusyChange={(probing) =>
                setBusy(probing ? "reachability" : null)
              }
              onProbe={(modelId) =>
                ports.workbuddy.checkModel({
                  app: "workbuddy",
                  baseUrl: baseUrl.trim(),
                  apiKey: apiKeyRef.current.trim(),
                  modelId,
                })
              }
            />
            <Button disabled={busy !== null} onClick={() => void fetchModels()}>
              {busy === "fetch" ? "读取中…" : "拉取模型"}
            </Button>
            <Button
              className="fy-control-button-danger"
              disabled={busy !== null || draftModelIds.length === 0}
              onClick={clearDraftModels}
            >
              清除所有模型
            </Button>
          </div>
          <FieldFeedback id="workbuddy-fetch-error" notice={notices.fetch} />
        </div>
        <div className="fy-models-manual-row">
          <label className="fy-control-field fy-models-manual-field">
            自定义模型 ID
            <Input
              ref={manualModelsInputRef}
              id="workbuddy-manual-model-ids"
              name="workbuddy-manual-model-ids"
              value={manualDraft}
              onChange={(event) => {
                setManualDraft(event.target.value);
                dismiss("draft");
              }}
              onKeyDown={(event) => {
                if (event.key === "Enter") {
                  event.preventDefault();
                  fillManualModels();
                }
              }}
              placeholder="输入模型 ID，多个用逗号分隔"
              autoComplete="off"
              spellCheck={false}
              aria-invalid={isErrorNotice(notices.draft)}
              aria-describedby={
                notices.draft ? "workbuddy-draft-error" : undefined
              }
            />
          </label>
          <Button disabled={busy !== null} onClick={fillManualModels}>
            填入
          </Button>
        </div>
        <FieldFeedback id="workbuddy-draft-error" notice={notices.draft} />
        <p className="fy-models-muted">
          {draftModelIds.length > 0
            ? `已选择 ${draftModelIds.length} 个模型，保存并应用后才会写入配置。`
            : "已选择 0 个模型"}
        </p>
      </section>

      <Dialog
        open={Boolean(pendingOverwrite)}
        onOpenChange={(open) => {
          if (!open && busy === null) setPendingOverwrite(null);
        }}
        title="确认覆盖已有模型"
        description={
          pendingOverwrite
            ? `以下模型已存在：${pendingOverwrite.existingIds.slice(0, 6).join(", ")}${pendingOverwrite.existingIds.length > 6 ? "…" : ""}`
            : undefined
        }
        actions={
          <>
            <Button
              disabled={busy !== null}
              onClick={() => setPendingOverwrite(null)}
            >
              取消
            </Button>
            <Button
              className="fy-control-button-danger"
              disabled={busy !== null}
              onClick={confirmOverwrite}
            >
              {busy === "save" ? "处理中…" : "确认覆盖"}
            </Button>
          </>
        }
      >
        <p>确认后将使用当前选择覆盖已有模型。</p>
      </Dialog>
      <Dialog
        open={pendingDeleteId !== null}
        onOpenChange={(open) => {
          if (!open && busy !== "delete") setPendingDeleteId(null);
        }}
        title="确认删除模型"
        description="此操作将会删除该模型配置，不可恢复，是否确认删除"
        actions={
          <>
            <Button
              disabled={busy === "delete"}
              onClick={() => setPendingDeleteId(null)}
            >
              取消
            </Button>
            <Button
              className="fy-control-button-danger"
              disabled={busy === "delete" || pendingDeleteId === null}
              onClick={() => {
                if (pendingDeleteId) void deleteExistingModel(pendingDeleteId);
              }}
            >
              {busy === "delete" ? "删除中…" : "确认删除"}
            </Button>
          </>
        }
      >
        {pendingDeleteId ? (
          <p>
            将删除 <code>{pendingDeleteId}</code>。
          </p>
        ) : null}
      </Dialog>
    </CatalogDetail>
  );
}

const WARNING_COPY: Record<CodexProviderMutationWarning, string> = {
  CODEX_WEBSOCKET_NON_GPT_MODEL:
    "当前模型可能与此连接方式不兼容，请确认后使用。",
  CODEX_WEBSOCKET_PROXY_MAY_BE_UNSUPPORTED:
    "当前网络代理可能影响连接，请确认后使用。",
};

function sanitizeWarningCodes(
  ...groups: ReadonlyArray<readonly string[] | undefined>
): CodexProviderMutationWarning[] {
  return [
    ...new Set(
      groups
        .flatMap((group) => group ?? [])
        .filter((code): code is CodexProviderMutationWarning =>
          Object.prototype.hasOwnProperty.call(WARNING_COPY, code),
        ),
    ),
  ];
}

const PROVIDER_LABELS: Readonly<Record<ProviderAppId, string>> = {
  claude: "Claude Code",
  codex: "Codex",
  grokbuild: "Grok Build",
};

const PROVIDER_DEFAULT_NAMES: Readonly<Record<ProviderAppId, string>> = {
  claude: "FyAgent Claude",
  codex: "FyAgent Codex",
  grokbuild: "FyAgent Grok Build",
};

function ProviderPanel({
  app,
  active,
  writesBlocked,
  onBlockWrites,
}: {
  app: ProviderAppId;
  active: boolean;
  writesBlocked: boolean;
  onBlockWrites: (app: ProviderAppId) => void;
}) {
  const { ports } = useFeatures();
  const summaryQuery = useProviderSummary(app, active);
  const [name, setName] = useState(PROVIDER_DEFAULT_NAMES[app]);
  const [baseUrl, setBaseUrl] = useState("");
  const [apiKey, setApiKeyState] = useState("");
  const apiKeyRef = useRef("");
  const [modelId, setModelId] = useState("");
  const [fetchedModelIds, setFetchedModelIds] = useState<string[]>([]);
  const [ownedByById, setOwnedByById] = useState<Record<string, string>>({});
  const [fetchBusy, setFetchBusy] = useState(false);
  const [probeBusy, setProbeBusy] = useState(false);
  const [imageExtension, setImageExtension] = useState(false);
  const [websockets, setWebsockets] = useState(false);
  const [errors, setErrors] = useState<QuickSetupErrors>({});
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState<Notice | null>(null);
  const [warningCodes, setWarningCodes] = useState<
    CodexProviderMutationWarning[]
  >([]);
  const writeLock = useRef(false);
  const mountedRef = useRef(true);
  const nameInputRef = useRef<HTMLInputElement>(null);
  const baseUrlInputRef = useRef<HTMLInputElement>(null);
  const apiKeyInputRef = useRef<HTMLInputElement>(null);
  const modelIdInputRef = useRef<HTMLInputElement>(null);

  const setApiKey = (value: string) => {
    apiKeyRef.current = value;
    setApiKeyState(value);
  };
  const clearApiKey = () => {
    apiKeyRef.current = "";
    if (mountedRef.current) setApiKeyState("");
  };

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      apiKeyRef.current = "";
    };
  }, []);

  const providerId = QUICK_SETUP_PROVIDER_IDS[app];
  const providerExists = Boolean(summaryQuery.data?.providers[providerId]);
  const currentId = summaryQuery.data?.currentId ?? "";

  const fetchProviderModels = async () => {
    if (fetchBusy || busy || writesBlocked) return;
    if (!isHttpUrl(baseUrl.trim())) {
      setErrors((current) => ({
        ...current,
        baseUrl: "请输入有效的服务地址",
      }));
      baseUrlInputRef.current?.focus();
      return;
    }
    if (!apiKeyRef.current.trim()) {
      setErrors((current) => ({ ...current, apiKey: "请输入 API Key" }));
      apiKeyInputRef.current?.focus();
      return;
    }
    setFetchBusy(true);
    setErrors((current) => ({
      ...current,
      baseUrl: undefined,
      apiKey: undefined,
    }));
    try {
      const models = await ports.providers.fetchModels(
        baseUrl.trim(),
        apiKeyRef.current.trim(),
      );
      if (!mountedRef.current) return;
      const ids = models.map((model) => model.id);
      const nextOwned: Record<string, string> = {};
      for (const model of models) {
        if (model.ownedBy) nextOwned[model.id] = model.ownedBy;
      }
      setOwnedByById((current) => ({ ...current, ...nextOwned }));
      setFetchedModelIds((current) => addUniqueModelIds(current, ids));
    } catch {
      if (mountedRef.current) {
        setNotice({
          tone: "error",
          title: "模型读取失败",
          description: "请检查地址、凭据和服务状态后重试。",
        });
      }
    } finally {
      if (mountedRef.current) setFetchBusy(false);
    }
  };

  const selectableModelIds = addUniqueModelIds(
    fetchedModelIds,
    modelId ? [modelId] : [],
  );

  const prepareModelProbe = () => {
    if (!isHttpUrl(baseUrl.trim())) {
      setErrors((current) => ({
        ...current,
        baseUrl: "请输入不含账号信息的 HTTP(S) 地址",
      }));
      baseUrlInputRef.current?.focus();
      return false;
    }
    if (!apiKeyRef.current.trim()) {
      setErrors((current) => ({ ...current, apiKey: "请输入 API Key" }));
      apiKeyInputRef.current?.focus();
      return false;
    }
    setErrors((current) => ({
      ...current,
      baseUrl: undefined,
      apiKey: undefined,
    }));
    return true;
  };

  const submit = async () => {
    if (writeLock.current || writesBlocked) return;
    const validated = validateQuickSetup(
      {
        name,
        baseUrl,
        apiKey: apiKeyRef.current,
        modelId,
      },
      app,
    );
    if (!validated.ok) {
      setErrors(validated.errors);
      const firstInvalidField = (
        ["name", "baseUrl", "apiKey", "modelId"] as const
      ).find((field) => validated.errors[field]);
      const fieldRefs = {
        name: nameInputRef,
        baseUrl: baseUrlInputRef,
        apiKey: apiKeyInputRef,
        modelId: modelIdInputRef,
      };
      if (firstInvalidField) fieldRefs[firstInvalidField].current?.focus();
      return;
    }

    writeLock.current = true;
    setBusy(true);
    setErrors({});
    setNotice(null);
    setWarningCodes([]);
    let authorityRereadAttempted = false;
    let keepWriteLock = false;
    try {
      const request = buildQuickSetupRequest(
        app,
        validated.value,
        app === "codex" ? { imageExtension, websockets } : undefined,
      );
      const applyResult = await ports.providers.applyQuickSetupWithResult(
        request,
        app,
      );
      if (!mountedRef.current) return;
      const warnings = sanitizeWarningCodes(applyResult.warningCodes);
      setWarningCodes(warnings);
      const hasPartialWarning = applyResult.value.warnings.length > 0;
      let refreshed: Awaited<ReturnType<typeof summaryQuery.refetch>> | null =
        null;
      try {
        refreshed = await summaryQuery.refetch();
      } catch {
        refreshed = null;
      } finally {
        authorityRereadAttempted = true;
      }
      if (!mountedRef.current) return;

      const activeIdConfirmed =
        refreshed !== null &&
        !refreshed.isError &&
        refreshed.data?.currentId === providerId;
      const liveDescription = applyResult.liveConfigChanged
        ? "重启或新建会话后即可使用新的设置。"
        : "请在应用中刷新或新建会话后查看更改。";
      if (!activeIdConfirmed) {
        setNotice({
          tone: "warning",
          title: "模型设置已保存，待确认",
          description:
            app === "codex"
              ? `${liveDescription} 请刷新状态后确认当前配置。`
              : "请刷新状态后确认当前配置。",
        });
      } else {
        setNotice({
          tone: warnings.length || hasPartialWarning ? "warning" : "info",
          title: "模型设置已保存并设为当前配置",
          description:
            app === "codex"
              ? hasPartialWarning
                ? `${liveDescription} 部分设置仍需确认。`
                : liveDescription
              : hasPartialWarning
                ? "保存完成，但部分设置仍需确认。"
                : "请在应用中刷新或新建会话后查看更改。",
        });
      }
    } catch (error) {
      if (mountedRef.current) {
        const rollbackConfirmed =
          typeof error === "object" &&
          error !== null &&
          "code" in error &&
          error.code === "APPLY_FAILED_ROLLED_BACK";
        const stateUnknown = !rollbackConfirmed;
        if (stateUnknown) {
          keepWriteLock = true;
          onBlockWrites(app);
        }
        setNotice({
          tone: "error",
          title: stateUnknown
            ? "无法确认当前设置"
            : "未能保存设置，已还原之前的状态",
          description: stateUnknown
            ? "为避免覆盖现有设置，已暂停继续保存。请重新打开页面并检查当前配置。"
            : "请检查输入后重试。",
        });
      }
    } finally {
      clearApiKey();
      if (!authorityRereadAttempted) await summaryQuery.refetch();
      if (mountedRef.current) setBusy(false);
      if (!keepWriteLock) writeLock.current = false;
    }
  };

  const label = PROVIDER_LABELS[app];
  const queryUnavailable = summaryQuery.isError;
  const queryPending = summaryQuery.isLoading;

  return (
    <CatalogDetail
      className="fy-models-config-panel"
      ariaLabel={`${label} 模型配置`}
    >
      <ModelsPanelHeader
        title={label}
        summary="配置服务地址、模型和 API Key，并设为当前配置。"
        pending={Boolean(baseUrl.trim() || apiKey.trim() || modelId.trim())}
      >
        <Button
          className="fy-control-button-primary fy-models-commit-button"
          disabled={
            busy ||
            probeBusy ||
            writesBlocked ||
            queryPending ||
            queryUnavailable
          }
          onClick={() => void submit()}
        >
          {busy
            ? "配置中…"
            : writesBlocked
              ? "暂时无法确认当前设置"
              : "保存并设为当前配置"}
        </Button>
      </ModelsPanelHeader>

      {queryPending && <Spinner label={`正在读取 ${label} 配置`} />}
      {queryUnavailable && (
        <InlineNotice tone="error">
          暂时无法读取当前配置，请稍后重试。
        </InlineNotice>
      )}
      {!queryUnavailable && !queryPending && (
        <div
          className="fy-models-status-grid"
          data-testid={active ? "provider-status" : undefined}
        >
          <div className="fy-models-status-item">
            <span>保存的配置</span>
            <strong>{providerExists ? "已有设置，将更新" : "尚未设置"}</strong>
          </div>
          <div className="fy-models-status-item">
            <span>当前配置</span>
            <strong>{currentId ? "已设置" : "尚未设置"}</strong>
          </div>
        </div>
      )}

      <div className="fy-models-form">
        <div className="fy-control-field">
          <label htmlFor={`${app}-quick-setup-name`}>配置名称</label>
          <Input
            ref={nameInputRef}
            id={`${app}-quick-setup-name`}
            name={`${app}-quick-setup-name`}
            value={name}
            onChange={(event) => setName(event.target.value)}
            aria-invalid={Boolean(errors.name)}
            aria-describedby={
              errors.name ? `${app}-quick-setup-name-error` : undefined
            }
          />
          {errors.name && (
            <span
              id={`${app}-quick-setup-name-error`}
              className="fy-control-field-error"
              role="alert"
            >
              {errors.name}
            </span>
          )}
        </div>
        <div className="fy-control-field">
          <label htmlFor={`${app}-quick-setup-base-url`}>服务地址</label>
          <Input
            ref={baseUrlInputRef}
            id={`${app}-quick-setup-base-url`}
            name={`${app}-quick-setup-base-url`}
            type="url"
            value={baseUrl}
            onChange={(event) => setBaseUrl(event.target.value)}
            placeholder={
              app === "claude"
                ? "https://gateway.example"
                : "https://gateway.example/v1"
            }
            autoComplete="off"
            spellCheck={false}
            aria-invalid={Boolean(errors.baseUrl)}
            aria-describedby={
              [
                errors.baseUrl ? `${app}-quick-setup-base-url-error` : null,
                app === "claude" && claudeBaseUrlHasExplicitV1Path(baseUrl)
                  ? `${app}-quick-setup-base-url-v1-warning`
                  : null,
              ]
                .filter(Boolean)
                .join(" ") || undefined
            }
          />
          {errors.baseUrl && (
            <span
              id={`${app}-quick-setup-base-url-error`}
              className="fy-control-field-error"
              role="alert"
            >
              {errors.baseUrl}
            </span>
          )}
          {app === "claude" && claudeBaseUrlHasExplicitV1Path(baseUrl) ? (
            <FieldFeedback
              id={`${app}-quick-setup-base-url-v1-warning`}
              notice={{
                tone: "warning",
                title: CLAUDE_EXPLICIT_V1_WARNING,
              }}
            />
          ) : null}
        </div>
        <div className="fy-control-field">
          <label htmlFor={`${app}-quick-setup-api-key`}>API Key</label>
          <SecretInput
            ref={apiKeyInputRef}
            id={`${app}-quick-setup-api-key`}
            name={`${app}-quick-setup-api-key`}
            value={apiKey}
            onChange={(event) => setApiKey(event.target.value)}
            autoComplete="off"
            spellCheck={false}
            aria-invalid={Boolean(errors.apiKey)}
            aria-describedby={
              errors.apiKey ? `${app}-quick-setup-api-key-error` : undefined
            }
            revealLabel="显示 API Key"
            hideLabel="隐藏 API Key"
          />
          {errors.apiKey && (
            <span
              id={`${app}-quick-setup-api-key-error`}
              className="fy-control-field-error"
              role="alert"
            >
              {errors.apiKey}
            </span>
          )}
        </div>
        <div className="fy-control-field">
          <label htmlFor={`${app}-quick-setup-model-id`}>模型 ID</label>
          <div className="fy-models-id-with-icon">
            <ModelVendorIcon modelId={modelId} />
            <Input
              ref={modelIdInputRef}
              id={`${app}-quick-setup-model-id`}
              name={`${app}-quick-setup-model-id`}
              value={modelId}
              onChange={(event) => setModelId(event.target.value)}
              autoComplete="off"
              spellCheck={false}
              aria-invalid={Boolean(errors.modelId)}
              aria-describedby={
                errors.modelId ? `${app}-quick-setup-model-id-error` : undefined
              }
            />
          </div>
          {errors.modelId && (
            <span
              id={`${app}-quick-setup-model-id-error`}
              className="fy-control-field-error"
              role="alert"
            >
              {errors.modelId}
            </span>
          )}
        </div>
        <div className="fy-models-form-wide">
          <div className="fy-models-actions">
            <ModelConnectivityTest
              searchId={`${app}-probe-search`}
              modelIds={selectableModelIds}
              ownedByById={ownedByById}
              disabled={busy || fetchBusy || writesBlocked}
              onPrepare={prepareModelProbe}
              onBusyChange={setProbeBusy}
              onProbe={(selectedModelId) =>
                ports.providers.checkModel({
                  app,
                  baseUrl: baseUrl.trim(),
                  apiKey: apiKeyRef.current.trim(),
                  modelId: selectedModelId,
                })
              }
            />
            <Button
              disabled={busy || fetchBusy || probeBusy || writesBlocked}
              onClick={() => void fetchProviderModels()}
            >
              {fetchBusy ? "读取中…" : "拉取模型"}
            </Button>
          </div>
        </div>
        <div className="fy-models-form-wide">
          <GroupedModelChips
            ids={selectableModelIds}
            selectedId={modelId}
            onSelect={setModelId}
            removable
            removeDisabled={busy || fetchBusy || probeBusy}
            ownedByById={ownedByById}
            onRemove={(id) => {
              setFetchedModelIds((current) =>
                current.filter((item) => item !== id),
              );
              if (modelId === id) setModelId("");
            }}
            emptyLabel="尚未拉取模型。可点击拉取，或手动填入模型 ID。"
          />
        </div>
        {app === "codex" && (
          <div
            className="fy-models-codex-features"
            data-testid="codex-features"
          >
            <div className="fy-models-checkbox-row">
              <Checkbox
                checked={imageExtension}
                onCheckedChange={setImageExtension}
                label="启用内置生图扩展"
              />
              <span>启用内置生图扩展</span>
            </div>
            <div className="fy-models-checkbox-row">
              <Checkbox
                checked={websockets}
                onCheckedChange={setWebsockets}
                label="启用 WebSocket 传输"
              />
              <span>启用 WebSocket 传输</span>
            </div>
          </div>
        )}
      </div>

      <NoticeView notice={notice} />
      {warningCodes.length > 0 && (
        <InlineNotice tone="warning">
          <strong>Codex 使用提示</strong>
          <ul className="fy-models-warning-list">
            {warningCodes.map((code) => (
              <li key={code}>{WARNING_COPY[code]}</li>
            ))}
          </ul>
        </InlineNotice>
      )}
    </CatalogDetail>
  );
}

function renderTargetPanel(
  target: ModelTarget,
  active: boolean,
  blockedProviderWrites: Partial<Record<ProviderAppId, boolean>>,
  onBlockProviderWrites: (app: ProviderAppId) => void,
) {
  switch (target) {
    case "workbuddy":
      return <WorkBuddyPanel active={active} />;
    case "codex":
    case "claude":
    case "grokbuild":
      return (
        <ProviderPanel
          app={target}
          active={active}
          writesBlocked={Boolean(blockedProviderWrites[target])}
          onBlockWrites={onBlockProviderWrites}
        />
      );
    case "qoderwork":
      return <QoderModelsPanel />;
    case "trae":
      return <TraeModelsPanel active={active} />;
    case "opencode":
      return <OpenCodeModelsPanel active={active} />;
  }
}

export function ModelsPage() {
  const { pathname } = useLocation();
  const [searchParams, setSearchParams] = useSearchParams();
  const pageActive = pathname === "/models";
  const [blockedProviderWrites, setBlockedProviderWrites] = useState<
    Partial<Record<ProviderAppId, boolean>>
  >({});
  const rawTarget = searchParams.get("target");
  const [sessionTarget, setSessionTarget] = useState(() =>
    parseModelTarget(rawTarget),
  );
  if (pageActive && rawTarget !== null) {
    const parsed = parseModelTarget(rawTarget);
    if (parsed !== sessionTarget) setSessionTarget(parsed);
  }
  const target =
    pageActive && rawTarget !== null
      ? parseModelTarget(rawTarget)
      : sessionTarget;
  const [visitedTargets, setVisitedTargets] = useState(
    () => new Set<ModelTarget>([target]),
  );
  const targets = useMemo(() => MODEL_TARGETS, []);

  if (!visitedTargets.has(target)) {
    const next = new Set(visitedTargets);
    next.add(target);
    setVisitedTargets(next);
  }

  useEffect(() => {
    if (!pageActive) return;
    if (searchParams.get("target") !== null) return;
    if (sessionTarget === "qoderwork") return;
    setSearchParams({ target: sessionTarget }, { replace: true });
  }, [pageActive, searchParams, sessionTarget, setSearchParams]);

  const blockProviderWrites = (app: ProviderAppId) => {
    setBlockedProviderWrites((current) => ({
      ...current,
      [app]: true,
    }));
  };

  return (
    <div
      className="fy-feature-page fy-split-page fy-catalog-page fy-models-page"
      data-testid="models-page"
      aria-label="模型"
    >
      <CatalogMasterDetail>
        <CatalogRail as="aside" ariaLabel="模型配置目标" title="选择应用">
          <CatalogList>
            {targets.map((candidate) => (
              <CatalogListItem
                key={candidate}
                asset={getAgentBrand(TARGET_ICON_IDS[candidate])}
                label={TARGET_LABELS[candidate]}
                selected={candidate === target}
                testId={`model-target-${candidate}`}
                onSelect={() =>
                  setSearchParams({ target: candidate }, { replace: true })
                }
              />
            ))}
          </CatalogList>
        </CatalogRail>
        <div className="fy-models-target-stack">
          {MODEL_TARGETS.filter((candidate) =>
            visitedTargets.has(candidate),
          ).map((candidate) => (
            <PersistentSurface
              key={candidate}
              active={pageActive && candidate === target}
            >
              {renderTargetPanel(
                candidate,
                pageActive && candidate === target,
                blockedProviderWrites,
                blockProviderWrites,
              )}
            </PersistentSurface>
          ))}
        </div>
      </CatalogMasterDetail>
    </div>
  );
}
