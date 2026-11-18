import { useEffect, useMemo, useRef, useState } from "react";
import type {
  ChangeJobSnapshot,
  ChangePlan,
  ChangePlanErrorCode,
} from "../../shared/features/change-plans";
import type { FeaturePorts } from "../../shared/features/ports";
import { useFeatures } from "../../shared/features/provider";
import {
  useWorkBuddyModelIds,
  useWorkBuddyStatus,
} from "../../shared/features/queries";
import type { ModelWriteTarget } from "../../shared/features/types";
import { Button } from "../../shared/ui/Button";
import { Dialog } from "../../shared/ui/Dialog";
import {
  InlineNotice,
  Input,
  SecretInput,
  Spinner,
} from "../../shared/ui/primitives";
import { CatalogDetail } from "../../shared/ui/catalog";
import {
  FieldFeedback,
  focusControl,
  isErrorNotice,
  ModelsSection,
  useFieldNotices,
  type Notice,
} from "./feedback";
import { isHttpUrl, parseManualModelIds } from "./quickSetup";
import { GroupedModelChips, ModelSearchField } from "./modelChips";
import { ModelConnectivityTest } from "./ModelConnectivityTest";
import {
  ModelsPanelHeader,
  ModelsExistingSection,
  NoApiKeyOption,
  useModelsDraftCommit,
} from "./modelsShared";
import { WorkBuddySubscriptionSection } from "./WorkBuddySubscriptionSection";
import { WorkBuddySavePlanWorkspace, hasUnconfirmedAuthority } from "./apply";
import { changePlanErrorCode } from "../../shared/features/change-plans-ui/changePlanErrors";
import {
  addUniqueModelIds,
  filterModelIds,
  splitWorkBuddyDraft,
} from "./workBuddyModels";

type WorkBuddySaveRequest = Parameters<
  FeaturePorts["workbuddy"]["saveModels"]
>[0];

const EMPTY_MODEL_IDS: readonly string[] = [];

type WorkBuddyNoticeField =
  | "baseUrl"
  | "apiKey"
  | "fetch"
  | "draft"
  | "save"
  | "existing";

export function WorkBuddyPanel({ active }: { active: boolean }) {
  const { ports } = useFeatures();
  const statusQuery = useWorkBuddyStatus(active);
  const modelIdsQuery = useWorkBuddyModelIds(active);
  const [sourceMode, setSourceMode] = useState<"api" | "subscription">("api");
  const [subscriptionWriting, setSubscriptionWriting] = useState(false);
  const [subscriptionBlocked, setSubscriptionBlocked] = useState(false);
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
  const [pendingDeleteId, setPendingDeleteId] = useState<string | null>(null);
  const deleteOriginRef = useRef<HTMLElement | null>(null);
  const [workBuddyWriteTargets, setWorkBuddyWriteTargets] = useState<
    readonly ModelWriteTarget[]
  >([]);
  const [workBuddySaveRequest, setWorkBuddySaveRequest] =
    useState<WorkBuddySaveRequest | null>(null);
  const [workBuddySavePlan, setWorkBuddySavePlan] = useState<ChangePlan | null>(
    null,
  );
  const [workBuddySavePreviewError, setWorkBuddySavePreviewError] = useState<{
    code: ChangePlanErrorCode;
    message?: string;
  } | null>(null);
  const writeLock = useRef(false);
  const submittedRevisionRef = useRef(0);
  const mountedRef = useRef(true);
  const baseUrlInputRef = useRef<HTMLInputElement>(null);
  const apiKeyInputRef = useRef<HTMLInputElement>(null);
  const manualModelsInputRef = useRef<HTMLInputElement>(null);
  const draftCommit = useModelsDraftCommit();

  const setApiKey = (value: string) => {
    apiKeyRef.current = value;
    setApiKeyState(value);
    draftCommit.markDirty();
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
      draftCommit.markDirty();
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
    draftCommit.markDirty();
    dismiss("draft");
  };

  const clearDraftModels = () => {
    setDraftModelIds([]);
    setFetchedSourceIds(new Set());
    setTruncated(false);
    draftCommit.markDirty();
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
    const status = statusQuery.data;
    if (!status) return;
    const submittedRevision = draftCommit.captureRevision();
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
    setWorkBuddyWriteTargets([
      {
        path: status.path,
        backupPath: status.backupPath,
        exists: status.exists,
      },
    ]);
    setDraftModelIds(draftIds);
    setManualDraft("");
    void createSavePlan(request, submittedRevision);
  };

  const createSavePlan = async (
    request: WorkBuddySaveRequest,
    submittedRevision: number,
  ) => {
    if (writeLock.current) return;
    writeLock.current = true;
    submittedRevisionRef.current = submittedRevision;
    setBusy("save");
    clear();
    setWorkBuddySavePreviewError(null);
    try {
      const nextPlan = await ports.changePlans.createWorkBuddySavePlan(request);
      if (!mountedRef.current) return;
      setWorkBuddySaveRequest(request);
      setWorkBuddySavePlan(nextPlan);
    } catch (cause) {
      if (!mountedRef.current) return;
      setWorkBuddySaveRequest(request);
      setWorkBuddySavePlan(null);
      setWorkBuddySavePreviewError({ code: changePlanErrorCode(cause) });
    } finally {
      if (mountedRef.current) setBusy(null);
    }
  };

  const handleWorkBuddySaveDismiss = () => {
    setWorkBuddySaveRequest(null);
    setWorkBuddySavePlan(null);
    setWorkBuddySavePreviewError(null);
    setWorkBuddyWriteTargets([]);
    writeLock.current = false;
    if (mountedRef.current) setBusy(null);
  };

  const handleWorkBuddySavePlanChange = (plan: ChangePlan | null) => {
    setWorkBuddySavePlan(plan);
    if (plan) setWorkBuddySavePreviewError(null);
  };

  const handleWorkBuddySaveTerminal = (job: ChangeJobSnapshot) => {
    const submittedRevision = submittedRevisionRef.current;
    const unconfirmed = hasUnconfirmedAuthority(job);
    const applied =
      !unconfirmed && (job.status === "succeeded" || job.status === "warning");
    if (applied) draftCommit.commitRevision(submittedRevision);
    if (draftCommit.isCurrentRevision(submittedRevision)) clearApiKey();
    void (async () => {
      const rereadConfirmed = await refreshAuthoritativeState();
      if (!mountedRef.current) return;
      if (unconfirmed) {
        writeLock.current = true;
        show("save", {
          tone: "warning",
          title: "无法确认当前配置",
          description: rereadConfirmed
            ? "已刷新当前设置，请检查后再保存。"
            : "请重新打开页面并检查当前设置后再保存。",
        });
        return;
      }
      if (applied) {
        show("save", {
          tone: "info",
          title: "WorkBuddy 模型配置已保存",
          description: rereadConfirmed
            ? "当前设置已更新。请在 WorkBuddy 中检查模型列表。"
            : "设置已保存，但暂时无法刷新页面。请重新打开后检查。",
        });
        writeLock.current = false;
        return;
      }
      show("save", {
        tone:
          job.resultCode === "writer_failed_baseline_restored"
            ? "error"
            : "warning",
        title:
          job.resultCode === "writer_failed_baseline_restored"
            ? "保存失败，已恢复原配置"
            : "保存未完成",
        description: rereadConfirmed
          ? "已刷新当前设置，请检查后重试。"
          : "请重新打开页面并检查当前设置后重试。",
      });
      writeLock.current = false;
    })();
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
      <ModelsPanelHeader title="WorkBuddy" pending={draftCommit.pending}>
        <Button
          className="fy-control-button-primary fy-models-commit-button"
          disabled={
            sourceMode !== "api" ||
            subscriptionWriting ||
            subscriptionBlocked ||
            busy !== null ||
            loading ||
            readFailed ||
            Boolean(
              workBuddySaveRequest ||
                workBuddySavePlan ||
                workBuddySavePreviewError,
            )
          }
          onClick={startSave}
        >
          {busy === "save" ? "保存中…" : "保存并应用"}
        </Button>
      </ModelsPanelHeader>
      <div className="fy-models-inline-fields" aria-label="WorkBuddy 配置来源">
        <Button
          aria-pressed={sourceMode === "api"}
          disabled={
            subscriptionWriting ||
            busy !== null ||
            Boolean(workBuddySaveRequest)
          }
          onClick={() => setSourceMode("api")}
        >
          API Key 配置
        </Button>
        <Button
          aria-pressed={sourceMode === "subscription"}
          disabled={
            subscriptionWriting ||
            busy !== null ||
            Boolean(workBuddySaveRequest)
          }
          onClick={() => setSourceMode("subscription")}
        >
          账号订阅
        </Button>
      </div>
      <div hidden={sourceMode !== "subscription"}>
        <WorkBuddySubscriptionSection
          active={active && sourceMode === "subscription"}
          disabled={
            loading ||
            readFailed ||
            subscriptionBlocked ||
            subscriptionWriting ||
            busy !== null ||
            Boolean(workBuddySaveRequest)
          }
          writeTargets={
            statusQuery.data
              ? [
                  {
                    path: statusQuery.data.path,
                    backupPath: statusQuery.data.backupPath,
                    exists: statusQuery.data.exists,
                  },
                ]
              : []
          }
          onBeginWrite={() => {
            if (writeLock.current || subscriptionBlocked || !active)
              return false;
            writeLock.current = true;
            setSubscriptionWriting(true);
            return true;
          }}
          onEndWrite={() => {
            writeLock.current = false;
            setSubscriptionWriting(false);
          }}
          onUnconfirmed={() => {
            writeLock.current = true;
            setSubscriptionBlocked(true);
            setSubscriptionWriting(false);
          }}
          onSaved={() => {
            void refreshAuthoritativeState();
          }}
        />
      </div>
      <div hidden={sourceMode !== "api"}>
        <FieldFeedback id="workbuddy-save-error" notice={notices.save} />
        <WorkBuddySavePlanWorkspace
          key={workBuddySavePlan?.planId ?? "workbuddy-save-preview"}
          active={active}
          writeTargets={workBuddyWriteTargets}
          request={workBuddySaveRequest}
          plan={workBuddySavePlan}
          previewError={workBuddySavePreviewError}
          onPlanChange={handleWorkBuddySavePlanChange}
          onTerminal={handleWorkBuddySaveTerminal}
          onDismiss={handleWorkBuddySaveDismiss}
        />

        {loading && <Spinner label="正在读取 WorkBuddy 状态" />}
        {readFailed && (
          <InlineNotice tone="error">
            暂时无法读取 WorkBuddy 配置，请重试。
          </InlineNotice>
        )}
        <ModelsExistingSection
          title="当前已有的第三方模型 ID"
          countLabel="已有第三方模型数量"
          count={modelIds.length}
          open={existingOpen}
          onOpenChange={setExistingOpen}
          testId="workbuddy-model-ids"
          toggleTestId="workbuddy-status"
          invalid={isErrorNotice(notices.existing)}
          ariaLabel="当前已有的第三方模型 ID"
        >
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
            removeOriginRef={deleteOriginRef}
            onRemove={(modelId) => {
              if (busy !== null || writeLock.current) return;
              setPendingDeleteId(modelId);
            }}
            emptyLabel={
              existingSearch.trim()
                ? "没有匹配的模型 ID"
                : "还没有找到已配置的模型 ID"
            }
          />
          <FieldFeedback
            id="workbuddy-existing-error"
            notice={notices.existing}
          />
        </ModelsExistingSection>

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
                  draftCommit.markDirty();
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
                draftCommit.markDirty();
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
            onRemove={(modelId) => {
              setDraftModelIds((current) =>
                current.filter((id) => id !== modelId),
              );
              draftCommit.markDirty();
            }}
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
                resetVersion={draftCommit.resetVersion}
                onPrepare={validateConnection}
                onBusyChange={(probing) =>
                  setBusy(probing ? "reachability" : null)
                }
                onStatus={(requestId) =>
                  ports.workbuddy.getModelProbeStatus(requestId)
                }
                onCancel={(requestId) =>
                  ports.workbuddy.cancelModelProbe(requestId)
                }
                onProbe={(modelId, requestId) =>
                  ports.workbuddy.checkModel({
                    requestId,
                    app: "workbuddy",
                    baseUrl: baseUrl.trim(),
                    apiKey: apiKeyRef.current.trim(),
                    modelId,
                  })
                }
              />
              <Button
                disabled={busy !== null}
                onClick={() => void fetchModels()}
              >
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
                  draftCommit.markDirty();
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
              ? `已选择 ${draftModelIds.length} 个模型，尚未保存。`
              : "已选择 0 个模型"}
          </p>
        </section>

        <Dialog
          open={pendingDeleteId !== null}
          originRef={deleteOriginRef}
          onOpenChange={(open) => {
            if (!open && busy !== "delete") setPendingDeleteId(null);
          }}
          title="确认删除模型"
          description="模型配置删除后无法恢复。"
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
                  if (pendingDeleteId)
                    void deleteExistingModel(pendingDeleteId);
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
      </div>
    </CatalogDetail>
  );
}
