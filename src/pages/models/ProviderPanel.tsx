import type { AppMode } from "../../domain/configuration/types/proxy";
import type {
  ClaudeQuickSetupPreview,
  ClaudeQuickSetupOutcome,
} from "../../shared/features/claude-quick-setup";
import {
  lazy,
  Suspense,
  useCallback,
  useEffect,
  useRef,
  useState,
} from "react";
import { useLocation, useNavigate } from "react-router-dom";
import { useQueryClient } from "@tanstack/react-query";
import {
  defaultApiProtocol,
  isToolOnlyApi,
  providerApiCredentialError,
  type ApiProtocol,
} from "../../domain/configuration/providerApi";
import {
  ProviderApiFields,
  type ProviderApiFormFill,
} from "../../shared/features/controls/ProviderApiFields";
import {
  appendAgentReturnToPath,
  agentReturnDescriptorFromManagementSearch,
} from "../../shared/features/agent-navigation";
import type {
  ChangeJobSnapshot,
  ChangePlan,
  ChangePlanErrorCode,
} from "../../shared/features/change-plans";
import { useFeatures } from "../../shared/features/provider";
import { featureKeys, useProviderSummary } from "../../shared/features/queries";
import type {
  CodexProviderMutationWarning,
  ModelWriteTarget,
  ProviderAppId,
  ProviderQuickSetupRequest,
} from "../../shared/features/types";
import { Button } from "../../shared/ui/Button";
import {
  Checkbox,
  InlineNotice,
  Input,
  SecretInput,
  Spinner,
} from "../../shared/ui/primitives";
import { CatalogDetail } from "../../shared/ui/catalog";
import { FieldFeedback, type Notice } from "./feedback";
import {
  buildQuickSetupRequest,
  CLAUDE_EXPLICIT_V1_WARNING,
  claudeBaseUrlHasExplicitV1Path,
  isHttpUrl,
  QUICK_SETUP_PROVIDER_IDS,
  validateQuickSetup,
  type QuickSetupErrors,
} from "./quickSetup";
import { GroupedModelChips, ModelVendorIcon } from "./modelChips";
import { ModelConnectivityTest } from "./ModelConnectivityTest";
import {
  ModelsPanelHeader,
  ModelsWriteConfirmDialog,
  NoticeView,
  useModelsDraftCommit,
  useModelsWriteConfirm,
} from "./modelsShared";
import { XaiSubscriptionSection } from "./XaiSubscriptionSection";
import { CodexSavePlanWorkspace, hasUnconfirmedAuthority } from "./apply";
import { changePlanErrorCode } from "../../shared/features/change-plans-ui/changePlanErrors";
import { addUniqueModelIds } from "./workBuddyModels";

const ProviderModesPanel = lazy(() =>
  import("./ProviderModesPanel").then((module) => ({
    default: module.ProviderModesPanel,
  })),
);

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

export function ProviderPanel({
  app,
  active,
  writesBlocked,
  onBlockWrites,
  onRecoverWrites,
  initialForm,
}: {
  app: ProviderAppId;
  active: boolean;
  writesBlocked: boolean;
  onBlockWrites: (app: ProviderAppId) => void;
  onRecoverWrites: (app: ProviderAppId) => void;
  initialForm?: ProviderApiFormFill;
}) {
  const navigate = useNavigate();
  const { search } = useLocation();
  const { ports } = useFeatures();
  const summaryQuery = useProviderSummary(app, active);
  const queryClient = useQueryClient();
  const refreshProviderModes = useCallback(async () => {
    if (app === "grokbuild") return;
    await Promise.all(
      [
        featureKeys.providerMode(app),
        featureKeys.providerStack(app),
        featureKeys.providerList(app),
      ].map((queryKey) => queryClient.invalidateQueries({ queryKey })),
    );
  }, [app, queryClient]);
  const [modeView, setModeView] = useState<AppMode>("direct");
  const [name, setName] = useState(
    initialForm?.name ?? PROVIDER_DEFAULT_NAMES[app],
  );
  const [baseUrl, setBaseUrl] = useState(initialForm?.connection.baseUrl ?? "");
  const [apiKey, setApiKeyState] = useState("");
  const apiKeyRef = useRef("");
  const [modelId, setModelId] = useState(initialForm?.connection.modelId ?? "");
  const [protocol, setProtocol] = useState<ApiProtocol>(
    initialForm?.connection.protocol ?? defaultApiProtocol(app),
  );
  const [requiresFreshKey, setRequiresFreshKey] = useState(
    Boolean(initialForm),
  );
  const [fetchedModelIds, setFetchedModelIds] = useState<string[]>([]);
  const [ownedByById, setOwnedByById] = useState<Record<string, string>>({});
  const [fetchBusy, setFetchBusy] = useState(false);
  const [probeBusy, setProbeBusy] = useState(false);
  const [imageExtension, setImageExtension] = useState(false);
  const [websockets, setWebsockets] = useState(false);
  const [errors, setErrors] = useState<QuickSetupErrors>({});
  const [busy, setBusy] = useState(false);
  const [subscriptionBusy, setSubscriptionBusy] = useState(false);
  const [notice, setNotice] = useState<Notice | null>(null);
  const [warningCodes, setWarningCodes] = useState<
    CodexProviderMutationWarning[]
  >([]);
  const [codexSaveRequest, setCodexSaveRequest] =
    useState<ProviderQuickSetupRequest | null>(null);
  const [codexWriteTargets, setCodexWriteTargets] = useState<
    readonly ModelWriteTarget[]
  >([]);
  const [codexSavePlan, setCodexSavePlan] = useState<ChangePlan | null>(null);
  const [codexSavePreviewError, setCodexSavePreviewError] = useState<{
    code: ChangePlanErrorCode;
    message?: string;
  } | null>(null);
  const writeLock = useRef(false);
  const writeConfirm = useModelsWriteConfirm<
    | { kind: "claude"; preview: ClaudeQuickSetupPreview; revision: number }
    | {
        kind: "grokbuild";
        request: ProviderQuickSetupRequest;
        revision: number;
        targets: readonly ModelWriteTarget[];
      }
  >();
  const [claudeOutcome, setClaudeOutcome] =
    useState<ClaudeQuickSetupOutcome | null>(null);
  const previewGeneration = useRef(0);
  const activeRef = useRef(active);
  const submittedRevisionRef = useRef(0);
  const mountedRef = useRef(true);
  const nameInputRef = useRef<HTMLInputElement>(null);
  const baseUrlInputRef = useRef<HTMLInputElement>(null);
  const apiKeyInputRef = useRef<HTMLInputElement>(null);
  const modelIdInputRef = useRef<HTMLInputElement>(null);
  const draftCommit = useModelsDraftCommit(Boolean(initialForm));

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

  const { pending: pendingWrite, takePending: discardPendingWrite } =
    writeConfirm;
  const { resetVersion: draftVersion, isCurrentRevision } = draftCommit;
  useEffect(() => {
    activeRef.current = active;
    previewGeneration.current += 1;
  }, [active, draftVersion]);
  useEffect(() => {
    if (
      !active ||
      (pendingWrite?.kind === "claude" &&
        !isCurrentRevision(pendingWrite.revision))
    ) {
      discardPendingWrite();
    }
  }, [
    active,
    pendingWrite,
    discardPendingWrite,
    isCurrentRevision,
    draftVersion,
  ]);

  const providerId = QUICK_SETUP_PROVIDER_IDS[app];
  const providerExists = Boolean(summaryQuery.data?.providers[providerId]);
  const currentId = summaryQuery.data?.currentId ?? "";
  const savedConnection = summaryQuery.data?.providers[providerId]?.connection;
  const canRetainCodexCredential =
    app === "codex" &&
    !requiresFreshKey &&
    savedConnection !== undefined &&
    savedConnection.baseUrl === baseUrl.trim() &&
    savedConnection.protocol === protocol;

  const restrictedPlan = isToolOnlyApi(baseUrl, apiKey);
  const fillApiForm = (value: ProviderApiFormFill) => {
    setRequiresFreshKey(true);
    setName(value.name);
    setBaseUrl(value.connection.baseUrl);
    setModelId(value.connection.modelId);
    setProtocol(value.connection.protocol);
    clearApiKey();
    setFetchedModelIds([]);
    setOwnedByById({});
    setImageExtension(false);
    setWebsockets(false);
    setErrors({});
    setNotice(null);
    draftCommit.markDirty();
  };

  const fetchProviderModels = async () => {
    if (fetchBusy || busy || writesBlocked || restrictedPlan) return;
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
    const credentialError = providerApiCredentialError(
      baseUrl,
      apiKeyRef.current,
      protocol,
    );
    if (credentialError) {
      setErrors((current) => ({ ...current, apiKey: credentialError }));
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
    if (restrictedPlan) return false;
    const credentialError = providerApiCredentialError(
      baseUrl,
      apiKeyRef.current,
      protocol,
    );
    if (credentialError) {
      setErrors((current) => ({ ...current, apiKey: credentialError }));
      return false;
    }
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

  const requestSave = () => {
    if (
      writeLock.current ||
      subscriptionBusy ||
      writesBlocked ||
      writeConfirm.open
    )
      return;
    const validated = validateQuickSetup(
      {
        name,
        baseUrl,
        apiKey: apiKeyRef.current,
        modelId,
        protocol,
      },
      app,
      canRetainCodexCredential,
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
    const targets = summaryQuery.data?.writeTargets ?? [];
    if (app !== "claude" && targets.length === 0) return;
    const request = buildQuickSetupRequest(
      app,
      validated.value,
      app === "codex" ? { imageExtension, websockets } : undefined,
    );
    const revision = draftCommit.captureRevision();
    if (app === "codex") {
      // Plan creation is read-only. Its shared preview owns the one save
      // confirmation, including the native-owned file impact disclosure.
      setCodexWriteTargets(targets);
      void submit(request, revision);
    } else if (app === "claude") {
      void previewClaude(request, revision);
    } else {
      writeConfirm.requestConfirm({
        kind: "grokbuild",
        request,
        revision,
        targets,
      });
    }
  };

  const previewClaude = async (
    request: ProviderQuickSetupRequest,
    revision: number,
  ) => {
    if (writeLock.current) return;
    writeLock.current = true;
    setBusy(true);
    setNotice(null);
    setClaudeOutcome(null);
    const generation = previewGeneration.current;
    try {
      const preview = await ports.providers.previewClaudeQuickSetup(request);
      if (
        !mountedRef.current ||
        !activeRef.current ||
        generation !== previewGeneration.current ||
        !draftCommit.isCurrentRevision(revision)
      )
        return;
      writeConfirm.requestConfirm({ kind: "claude", preview, revision });
    } catch {
      if (
        mountedRef.current &&
        activeRef.current &&
        generation === previewGeneration.current &&
        draftCommit.isCurrentRevision(revision)
      ) {
        setNotice({
          tone: "error",
          title: "无法读取保存范围",
          description: "草稿和凭据已保留，请重新预览。",
        });
      }
    } finally {
      writeLock.current = false;
      if (mountedRef.current) setBusy(false);
    }
  };

  const applyClaude = async (pending: {
    preview: ClaudeQuickSetupPreview;
    revision: number;
  }) => {
    if (
      writeLock.current ||
      writesBlocked ||
      !activeRef.current ||
      !draftCommit.isCurrentRevision(pending.revision)
    )
      return;
    writeLock.current = true;
    setBusy(true);
    setNotice(null);
    try {
      const outcome = await ports.providers.applyClaudeQuickSetupPreview({
        previewId: pending.preview.previewId,
      });
      // The keyed panel may unmount on target switch while ModelsPage lives.
      // Preserve uncertain write authority in that parent before UI admission.
      if (outcome.overall === "unknown" || outcome.overall === "partial")
        onBlockWrites(app);
      if (!mountedRef.current) return;
      setClaudeOutcome(outcome);
      if (outcome.providerState === "applied") {
        draftCommit.commitRevision(pending.revision);
        if (draftCommit.isCurrentRevision(pending.revision)) clearApiKey();
      }
      if (
        outcome.overall === "unknown" &&
        draftCommit.isCurrentRevision(pending.revision)
      )
        clearApiKey();
      const notices: Record<ClaudeQuickSetupOutcome["overall"], Notice> = {
        applied: {
          tone: "info",
          title: "模型设置已保存并设为当前配置",
          description:
            "两个文件的结果已分别确认。请在 Claude 中刷新或新建会话后查看更改。",
        },
        partial: {
          tone: "warning",
          title: "模型条目已保存，部分文件未完成",
          description: "请分别检查文件结果，模型条目会保留。",
        },
        stale: {
          tone: "warning",
          title: "保存范围已变化，未写入",
          description: "草稿和凭据已保留，请重新预览后确认。",
        },
        rolledBack: {
          tone: "error",
          title: "未能保存设置，已还原之前的状态",
          description: "草稿和凭据已保留，请重新预览后重试。",
        },
        unknown: {
          tone: "error",
          title: "无法确认当前设置",
          description: "已暂停继续保存。请分别检查文件结果。",
        },
      };
      setNotice(notices[outcome.overall]);
    } catch {
      onBlockWrites(app);
      if (mountedRef.current) {
        if (draftCommit.isCurrentRevision(pending.revision)) clearApiKey();
        setClaudeOutcome({
          contractVersion: 1,
          overall: "unknown",
          providerState: "unknown",
          files: [
            { target: "claude_settings", state: "unknown" },
            { target: "claude_mcp", state: "unknown" },
          ],
        });
        setNotice({
          tone: "error",
          title: "无法确认当前设置",
          description: "已暂停继续保存。请分别检查文件结果。",
        });
      }
    } finally {
      try {
        await summaryQuery.refetch();
        void refreshProviderModes();
      } catch {
        /* The native outcome remains authoritative. */
      } finally {
        writeLock.current = false;
        if (mountedRef.current) setBusy(false);
      }
    }
  };

  const confirmWrite = () => {
    if (writeLock.current) return;
    const pending = writeConfirm.takePending();
    if (!pending) return;
    if (pending.kind === "claude") void applyClaude(pending);
    else void submit(pending.request, pending.revision);
  };

  const submit = async (
    request: ProviderQuickSetupRequest,
    submittedRevision: number,
  ) => {
    if (writeLock.current || writesBlocked) return;
    writeLock.current = true;
    setBusy(true);
    setErrors({});
    setNotice(null);
    setWarningCodes([]);
    if (app === "codex") {
      submittedRevisionRef.current = submittedRevision;
      setCodexSavePreviewError(null);
      try {
        const nextPlan =
          await ports.changePlans.createCodexProviderUpsertPlan(request);
        if (!mountedRef.current) return;
        setCodexSaveRequest(request);
        setCodexSavePlan(nextPlan);
        setWarningCodes(
          sanitizeWarningCodes(nextPlan.risks.map((risk) => risk.code)),
        );
      } catch (cause) {
        if (!mountedRef.current) return;
        setCodexSaveRequest(request);
        setCodexSavePlan(null);
        setCodexSavePreviewError({ code: changePlanErrorCode(cause) });
      }
      if (mountedRef.current) setBusy(false);
      return;
    }

    let authorityRereadAttempted = false;
    let keepWriteLock = false;
    try {
      const applyResult = await ports.providers.applyQuickSetupWithResult(
        request,
        app,
      );
      if (!mountedRef.current) return;
      draftCommit.commitRevision(submittedRevision);
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
      if (!activeIdConfirmed) {
        setNotice({
          tone: "warning",
          title: "模型设置已保存，待确认",
          description: "请刷新状态后确认当前配置。",
        });
      } else {
        setNotice({
          tone: warnings.length || hasPartialWarning ? "warning" : "info",
          title: "模型设置已保存并设为当前配置",
          description: hasPartialWarning
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
      if (draftCommit.isCurrentRevision(submittedRevision)) clearApiKey();
      if (!authorityRereadAttempted) await summaryQuery.refetch();
      if (mountedRef.current) setBusy(false);
      if (!keepWriteLock) writeLock.current = false;
    }
  };

  const handleCodexSaveDismiss = useCallback(() => {
    setCodexSaveRequest(null);
    setCodexWriteTargets([]);
    setCodexSavePlan(null);
    setCodexSavePreviewError(null);
    writeLock.current = false;
    if (mountedRef.current) setBusy(false);
  }, []);

  const handleCodexSavePlanChange = useCallback((plan: ChangePlan | null) => {
    setCodexSavePlan(plan);
    if (plan) {
      setCodexSavePreviewError(null);
      setWarningCodes(
        sanitizeWarningCodes(plan.risks.map((risk) => risk.code)),
      );
    }
  }, []);

  const handleCodexSaveTerminal = useCallback(
    (job: ChangeJobSnapshot) => {
      const submittedRevision = submittedRevisionRef.current;
      const unconfirmed = hasUnconfirmedAuthority(job);
      const applied =
        !unconfirmed &&
        (job.status === "succeeded" || job.status === "warning");
      if (applied) draftCommit.commitRevision(submittedRevision);
      if (draftCommit.isCurrentRevision(submittedRevision)) clearApiKey();
      void (async () => {
        let refreshed: Awaited<ReturnType<typeof summaryQuery.refetch>> | null;
        try {
          refreshed = await summaryQuery.refetch();
          void refreshProviderModes();
        } catch {
          refreshed = null;
        }
        if (!mountedRef.current) return;
        const activeIdConfirmed =
          refreshed !== null &&
          !refreshed.isError &&
          refreshed.data?.currentId === providerId;
        const liveDescription = job.liveConfigChanged
          ? "重启或新建会话后即可使用新的设置。"
          : "请在应用中刷新或新建会话后查看更改。";
        if (unconfirmed) {
          writeLock.current = true;
          onBlockWrites(app);
          setNotice({
            tone: "error",
            title: "无法确认当前设置",
            description:
              "为避免覆盖现有设置，已暂停继续保存。请重新打开页面并检查当前配置。",
          });
          return;
        }
        writeLock.current = false;
        if (job.status === "failed" || job.status === "cancelled") {
          setNotice({
            tone: "error",
            title:
              job.resultCode === "writer_failed_baseline_restored"
                ? "未能保存设置，已还原之前的状态"
                : "未能保存设置",
            description: "请检查输入后重试。",
          });
          return;
        }
        if (!applied) return;
        if (!activeIdConfirmed) {
          setNotice({
            tone: "warning",
            title: "模型设置已保存，待确认",
            description: `${liveDescription} 请刷新状态后确认当前配置。`,
          });
          return;
        }
        setNotice({
          tone: job.status === "warning" ? "warning" : "info",
          title: "模型设置已保存并设为当前配置",
          description:
            job.status === "warning"
              ? `${liveDescription} 部分设置仍需确认。`
              : liveDescription,
        });
      })();
    },
    [
      app,
      draftCommit,
      onBlockWrites,
      providerId,
      summaryQuery,
      refreshProviderModes,
    ],
  );

  const label = PROVIDER_LABELS[app];
  const queryUnavailable = summaryQuery.isError;
  const queryPending = summaryQuery.isLoading;

  return (
    <CatalogDetail
      className="fy-models-config-panel"
      ariaLabel={`${label} 模型配置`}
    >
      <ModelsPanelHeader title={label} pending={draftCommit.pending}>
        {modeView === "direct" && (
          <Button
            className="fy-control-button-primary fy-models-commit-button"
            disabled={
              busy ||
              subscriptionBusy ||
              probeBusy ||
              writesBlocked ||
              queryPending ||
              queryUnavailable ||
              Boolean(codexSaveRequest || codexSavePlan) ||
              (app !== "claude" &&
                (summaryQuery.data?.writeTargets.length ?? 0) === 0)
            }
            onClick={requestSave}
            dialogOriginRef={writeConfirm.originRef}
          >
            {busy
              ? "配置中…"
              : writesBlocked
                ? "暂时无法确认当前设置"
                : "保存并设为当前配置"}
          </Button>
        )}
      </ModelsPanelHeader>
      {(app === "claude" || app === "codex") && (
        <Suspense fallback={<Spinner label="正在读取连接模式" />}>
          <ProviderModesPanel
            app={app}
            active={active}
            disabled={
              busy ||
              subscriptionBusy ||
              writesBlocked ||
              writeConfirm.open ||
              Boolean(codexSaveRequest || codexSavePlan)
            }
            view={modeView}
            onViewChange={setModeView}
          />
        </Suspense>
      )}
      <div className="fy-models-direct-content" hidden={modeView !== "direct"}>
        {app === "claude" || app === "codex" || app === "grokbuild" ? (
          <XaiSubscriptionSection
            key={app}
            app={app}
            active={active}
            disabled={
              busy ||
              fetchBusy ||
              probeBusy ||
              subscriptionBusy ||
              writesBlocked ||
              writeConfirm.open ||
              Boolean(codexSaveRequest || codexSavePlan)
            }
            writeTargets={summaryQuery.data?.writeTargets ?? []}
            recoveryDisabled={
              busy ||
              fetchBusy ||
              probeBusy ||
              subscriptionBusy ||
              writeConfirm.open ||
              Boolean(codexSaveRequest || codexSavePlan)
            }
            onBeginRecovery={() => {
              if (
                writeLock.current ||
                busy ||
                fetchBusy ||
                probeBusy ||
                writeConfirm.open ||
                codexSaveRequest ||
                codexSavePlan
              )
                return false;
              writeLock.current = true;
              setSubscriptionBusy(true);
              return true;
            }}
            onRecoveryConfirmed={() => onRecoverWrites(app)}
            onBeginWrite={() => {
              if (writeLock.current || writesBlocked) return false;
              writeLock.current = true;
              setSubscriptionBusy(true);
              return true;
            }}
            onEndWrite={() => {
              writeLock.current = false;
              void refreshProviderModes();
              if (mountedRef.current) setSubscriptionBusy(false);
            }}
            onUnconfirmed={() => onBlockWrites(app)}
          />
        ) : null}

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
              <strong>
                {providerExists ? "已有设置，将更新" : "尚未设置"}
              </strong>
            </div>
            <div className="fy-models-status-item">
              <span>当前配置</span>
              <strong>{currentId ? "已设置" : "尚未设置"}</strong>
            </div>
          </div>
        )}

        {app === "codex" && summaryQuery.data ? (
          <>
            <div className="fy-models-source-entry">
              <p>保存后将启用本次配置。</p>
              <Button
                onClick={() => {
                  const descriptor =
                    agentReturnDescriptorFromManagementSearch(search);
                  const path = "/auth?consumer=codex&view=connections";
                  navigate(
                    descriptor
                      ? appendAgentReturnToPath(path, descriptor)
                      : path,
                  );
                }}
              >
                管理 Codex 账号与来源
              </Button>
            </div>
            <CodexSavePlanWorkspace
              key={codexSavePlan?.planId ?? "codex-save-preview"}
              active={active}
              writeTargets={codexWriteTargets}
              request={codexSaveRequest}
              plan={codexSavePlan}
              previewError={codexSavePreviewError}
              onPlanChange={handleCodexSavePlanChange}
              onTerminal={handleCodexSaveTerminal}
              onDismiss={handleCodexSaveDismiss}
            />
          </>
        ) : null}

        <div className="fy-models-form">
          <ProviderApiFields
            app={app}
            baseUrl={baseUrl}
            protocol={protocol}
            saved={Object.values(summaryQuery.data?.providers ?? {})}
            disabled={
              busy ||
              fetchBusy ||
              probeBusy ||
              writesBlocked ||
              writeConfirm.open ||
              Boolean(codexSaveRequest || codexSavePlan)
            }
            error={errors.protocol}
            onFill={fillApiForm}
            onProtocolChange={(next) => {
              setProtocol(next);
              if (next !== "responses") {
                setImageExtension(false);
                setWebsockets(false);
              }
              draftCommit.markDirty();
            }}
          />
          <div className="fy-control-field">
            <label htmlFor={`${app}-quick-setup-name`}>配置名称</label>
            <Input
              ref={nameInputRef}
              disabled={writesBlocked}
              id={`${app}-quick-setup-name`}
              name={`${app}-quick-setup-name`}
              value={name}
              onChange={(event) => {
                setName(event.target.value);
                draftCommit.markDirty();
              }}
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
              disabled={writesBlocked}
              id={`${app}-quick-setup-base-url`}
              name={`${app}-quick-setup-base-url`}
              type="url"
              value={baseUrl}
              onChange={(event) => {
                setBaseUrl(event.target.value);
                draftCommit.markDirty();
              }}
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
            {canRetainCodexCredential && (
              <p className="fy-models-muted">
                留空保留已保存的 API Key，填写新值会替换它。
              </p>
            )}
            <SecretInput
              ref={apiKeyInputRef}
              disabled={writesBlocked}
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
                disabled={writesBlocked}
                id={`${app}-quick-setup-model-id`}
                name={`${app}-quick-setup-model-id`}
                value={modelId}
                onChange={(event) => {
                  setModelId(event.target.value);
                  draftCommit.markDirty();
                }}
                autoComplete="off"
                spellCheck={false}
                aria-invalid={Boolean(errors.modelId)}
                aria-describedby={
                  errors.modelId
                    ? `${app}-quick-setup-model-id-error`
                    : undefined
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
                disabled={busy || fetchBusy || writesBlocked || restrictedPlan}
                resetVersion={draftCommit.resetVersion}
                onPrepare={prepareModelProbe}
                onBusyChange={setProbeBusy}
                onStatus={(requestId) =>
                  ports.providers.getModelProbeStatus(requestId)
                }
                onCancel={(requestId) =>
                  ports.providers.cancelModelProbe(requestId)
                }
                onProbe={(selectedModelId, requestId) =>
                  ports.providers.checkModel({
                    requestId,
                    app,
                    baseUrl: baseUrl.trim(),
                    apiKey: apiKeyRef.current.trim(),
                    modelId: selectedModelId,
                    protocol,
                    ...(app === "codex"
                      ? { codexImageExtension: imageExtension }
                      : {}),
                  })
                }
              />
              <Button
                disabled={
                  busy ||
                  fetchBusy ||
                  probeBusy ||
                  writesBlocked ||
                  restrictedPlan
                }
                onClick={() => void fetchProviderModels()}
              >
                {fetchBusy ? "读取中…" : "拉取模型"}
              </Button>
            </div>
          </div>
          {restrictedPlan && (
            <div className="fy-models-form-wide">
              <InlineNotice tone="info">
                此套餐请在允许的目标编程工具中验证；本页不发送模型拉取或测试请求。
              </InlineNotice>
            </div>
          )}
          <div className="fy-models-form-wide">
            <GroupedModelChips
              ids={selectableModelIds}
              selectedId={modelId}
              onSelect={(id) => {
                setModelId(id);
                draftCommit.markDirty();
              }}
              removable
              removeDisabled={busy || fetchBusy || probeBusy}
              ownedByById={ownedByById}
              onRemove={(id) => {
                setFetchedModelIds((current) =>
                  current.filter((item) => item !== id),
                );
                if (modelId === id) {
                  setModelId("");
                  draftCommit.markDirty();
                }
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
                  disabled={protocol !== "responses"}
                  checked={imageExtension}
                  onCheckedChange={(checked) => {
                    setImageExtension(checked);
                    draftCommit.markDirty();
                  }}
                  label="启用内置生图扩展"
                />
                <span>启用内置生图扩展</span>
              </div>
              <div className="fy-models-checkbox-row">
                <Checkbox
                  disabled={protocol !== "responses"}
                  checked={websockets}
                  onCheckedChange={(checked) => {
                    setWebsockets(checked);
                    draftCommit.markDirty();
                  }}
                  label="启用 WebSocket 传输"
                />
                <span>启用 WebSocket 传输</span>
              </div>
            </div>
          )}
        </div>

        <NoticeView notice={notice} />
        {claudeOutcome && (
          <ul aria-label="Claude 文件保存结果">
            {claudeOutcome.files.map((file) => (
              <li key={file.target}>
                {file.target === "claude_settings"
                  ? "settings.json"
                  : ".claude.json"}
                ：
                {
                  {
                    applied: "已写入",
                    unchanged: "保持不变",
                    rolledBack: "未完成，已保留或还原此前内容",
                    conflict: "存在外部改动，未覆盖",
                    notAttempted: "未写入",
                    unknown: "待确认",
                  }[file.state]
                }
              </li>
            ))}
          </ul>
        )}
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
        <ModelsWriteConfirmDialog
          originRef={writeConfirm.originRef}
          open={writeConfirm.open}
          targets={
            writeConfirm.pending?.kind === "claude"
              ? writeConfirm.pending.preview.writeTargets
              : (writeConfirm.pending?.targets ?? [])
          }
          preservedPaths={
            writeConfirm.pending?.kind === "claude"
              ? writeConfirm.pending.preview.preservedPaths
              : undefined
          }
          onConfirm={confirmWrite}
          onCancel={() => {
            writeConfirm.takePending();
          }}
        />
      </div>
    </CatalogDetail>
  );
}
