import { useQueryClient } from "@tanstack/react-query";
import { useEffect, useId, useRef, useState } from "react";
import { useLocation, useNavigate } from "react-router-dom";

import {
  appendAgentReturnToPath,
  agentReturnDescriptorFromManagementSearch,
} from "../../shared/features/agent-navigation";
import type {
  BindXaiManagedRequest,
  BindXaiManagedResult,
  ModelWriteTarget,
} from "../../shared/features/models";
import { useFeatures } from "../../shared/features/provider";
import {
  featureKeys,
  useManagedAuthOverview,
} from "../../shared/features/queries";
import {
  isXaiSubscriptionModelId,
  xaiBindErrorCode,
} from "../../shared/features/xai-subscription";
import { FileWriteDisclosure } from "../../shared/features/controls/FileWriteDisclosure";
import { Button } from "../../shared/ui/Button";
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "../../shared/ui/Collapsible";
import { Dialog } from "../../shared/ui/Dialog";
import { InlineNotice, Input, Spinner } from "../../shared/ui/primitives";
import { FieldFeedback, ModelsSection, type Notice } from "./feedback";
import { GroupedModelChips } from "./modelChips";

type Props = {
  active: boolean;
  disabled: boolean;
  app: "claude" | "codex";
  writeTargets: readonly ModelWriteTarget[];
  onBeginWrite: () => boolean;
  onEndWrite: () => void;
  onUnconfirmed: () => void;
};

const TARGET_LABELS = {
  claude: "Claude Code",
  codex: "Codex",
};

type CliBindRequest = BindXaiManagedRequest & {
  app: "claude" | "codex";
};

export function XaiSubscriptionSection(props: Props) {
  const { ports } = useFeatures();
  const queryClient = useQueryClient();
  const overview = useManagedAuthOverview(props.active);
  const navigate = useNavigate();
  const { search } = useLocation();
  const id = useId();
  const [accountId, setAccountId] = useState("");
  const [modelId, setModelId] = useState("");
  const [modelIds, setModelIds] = useState<string[]>([]);
  const [manualModel, setManualModel] = useState(false);
  const [pending, setPending] = useState<CliBindRequest | null>(null);
  const [busy, setBusy] = useState<"fetch" | "bind" | null>(null);
  const [notice, setNotice] = useState<Notice | null>(null);
  const [saved, setSaved] = useState<BindXaiManagedResult | null>(null);
  const lock = useRef(false);
  const mounted = useRef(true);
  const originRef = useRef<HTMLElement | null>(null);
  const cancelRef = useRef<HTMLButtonElement>(null);
  useEffect(() => {
    mounted.current = true;
    return () => {
      mounted.current = false;
    };
  }, []);

  const accounts =
    overview.data?.accounts.filter((item) => item.provider === "xai") ?? [];
  const selected = accounts.find((item) => item.accountId === accountId);
  const accountReady = !overview.isError && selected?.health === "ready";
  const locked = !props.active || props.disabled || busy !== null;
  const canBind =
    accountReady && isXaiSubscriptionModelId(modelId.trim()) && !locked;
  const pendingAccountReady =
    !overview.isError &&
    accounts.some(
      (item) =>
        item.accountId === pending?.accountId && item.health === "ready",
    );

  const openAuth = (connections = false) => {
    const descriptor = agentReturnDescriptorFromManagementSearch(search);
    const path = connections
      ? "/auth?consumer=codex&view=connections"
      : "/auth?view=accounts";
    navigate(descriptor ? appendAgentReturnToPath(path, descriptor) : path);
  };

  const fetchModels = async (selectedAccountId = accountId) => {
    if (
      lock.current ||
      locked ||
      overview.isError ||
      !accounts.some(
        (account) =>
          account.accountId === selectedAccountId && account.health === "ready",
      )
    )
      return;
    lock.current = true;
    setBusy("fetch");
    setNotice(null);
    try {
      const result =
        await ports.providers.fetchXaiManagedModels(selectedAccountId);
      if (!mounted.current) return;
      setModelIds(result.models);
      setManualModel(result.models.length === 0);
      setNotice({
        tone: result.models.length ? "info" : "warning",
        title: result.models.length
          ? `已加载 ${result.models.length} 个模型选项`
          : "暂无模型选项",
        description:
          "这些选项来自 Grok CLI 官方示例，并非你的账号模型名单。请选择一个，或手动填写订阅支持的模型 ID；实际可用性以服务返回为准。",
      });
    } catch {
      if (mounted.current) {
        setManualModel(true);
        setNotice({
          tone: "error",
          title: "无法加载模型选项",
          description: "请检查账号登录状态。也可以手动填写订阅支持的模型 ID。",
        });
      }
    } finally {
      lock.current = false;
      if (mounted.current) setBusy(null);
    }
  };

  const confirmBind = async () => {
    if (!pending || !pendingAccountReady || locked || lock.current) return;
    if (!props.onBeginWrite()) return;
    lock.current = true;
    setBusy("bind");
    setNotice(null);
    setSaved(null);
    const request = pending;
    setPending(null);
    let bindingReturned = false;
    try {
      const result = await ports.providers.bindXaiManaged(request);
      bindingReturned = true;
      await Promise.all([
        queryClient.invalidateQueries({
          queryKey: featureKeys.providerSummary(props.app),
          refetchType: "none",
        }),
        queryClient.invalidateQueries({
          queryKey: featureKeys.managedAuthOverview,
          refetchType: "none",
        }),
      ]);
      const [summary] = await Promise.all([
        queryClient.fetchQuery({
          queryKey: featureKeys.providerSummary(props.app),
          queryFn: () =>
            ports.providers.getSummary(
              props.app === "codex" ? "codex" : "claude",
            ),
        }),
        queryClient.fetchQuery({
          queryKey: featureKeys.managedAuthOverview,
          queryFn: ports.managedAuth.getOverview,
        }),
      ]);
      if (
        !summary.providers[result.providerId] ||
        (request.app === "claude" &&
          result.activated &&
          summary.currentId !== result.providerId)
      ) {
        props.onUnconfirmed();
        if (mounted.current)
          setNotice({
            tone: "warning",
            title: "设置已保存，当前状态待确认",
            description: "请重新读取配置后再继续设置。",
          });
        return;
      }
      if (!mounted.current) return;
      setSaved(result);
      setNotice(
        result.activated
          ? {
              tone: "info",
              title: `已将 SuperGrok 应用到 ${TARGET_LABELS[request.app]}`,
              description:
                "本机配置已确认。请重新打开目标软件或新建会话；实际调用与额度使用以服务返回为准。",
            }
          : {
              tone: "info",
              title: "已保存 Codex 订阅配置",
              description: `请前往账号与认证，选择「${result.providerName}」并预览、确认应用。当前请求来源尚未切换。`,
            },
      );
    } catch (error) {
      const code = xaiBindErrorCode(error);
      const unconfirmed =
        bindingReturned ||
        code === null ||
        code === "rollback_partial_state_unknown";
      if (unconfirmed) props.onUnconfirmed();
      if (mounted.current)
        setNotice({
          tone: "error",
          title: unconfirmed ? "无法确认当前设置" : "未能应用订阅配置",
          description: unconfirmed
            ? "已暂停继续修改此目标。请重新打开模型页面，检查当前配置。"
            : code === "account_unavailable"
              ? "所选账号暂时不能用于本机转发。请到账号与认证检查登录状态，必要时重新登录。"
              : code === "provider_conflict"
                ? "已有配置发生变化，请重新读取后再设置。"
                : code === "apply_failed_rolled_back"
                  ? "之前的配置已恢复。请检查本机连接服务后重试。"
                  : "请重新选择账号和模型后再试。",
        });
      await queryClient.invalidateQueries({
        queryKey: featureKeys.managedAuthOverview,
      });
    } finally {
      lock.current = false;
      props.onEndWrite();
      if (mounted.current) setBusy(null);
    }
  };

  return (
    <ModelsSection
      title="使用 Grok 订阅（实验性）"
      ariaLabel="SuperGrok 订阅设置"
    >
      <p className="fy-models-muted">
        选择在 FyAgent 中登录的账号和模型。使用订阅时，请保持 FyAgent
        在后台运行；完全退出后会停止转发。账号是否支持调用及额度使用，以 Grok
        服务返回为准。
      </p>
      {overview.isPending ? <Spinner label="正在读取订阅账号" /> : null}
      {overview.isError ? (
        <InlineNotice tone="warning">
          无法读取已保存的账号，请刷新后重试。
        </InlineNotice>
      ) : null}
      {!overview.isPending && !overview.isError && accounts.length === 0 ? (
        <InlineNotice tone="info">
          还没有保存的 Grok 账号，请先到账号与认证登录。
        </InlineNotice>
      ) : null}
      {accounts.length > 0 ? (
        <fieldset
          className="fy-models-section"
          disabled={locked || overview.isError}
        >
          <legend>订阅账号</legend>
          {accounts.map((account) => (
            <label key={account.accountId} className="fy-models-checkbox-row">
              <input
                type="radio"
                name={`${id}-account`}
                value={account.accountId}
                checked={accountId === account.accountId}
                disabled={account.health !== "ready"}
                onChange={() => {
                  if (locked) return;
                  setAccountId(account.accountId);
                  setModelId("");
                  setModelIds([]);
                  setSaved(null);
                  setNotice(null);
                  setManualModel(false);
                  void fetchModels(account.accountId);
                }}
              />
              <span>
                {account.displayName ?? account.login}
                {account.health === "requires_reauth"
                  ? "（需要重新登录）"
                  : account.health !== "ready"
                    ? "（暂不可用）"
                    : ""}
              </span>
            </label>
          ))}
        </fieldset>
      ) : null}
      <div className="fy-models-inline-fields">
        <Button disabled={locked} onClick={() => openAuth()}>
          管理 Grok 账号
        </Button>
        <Button
          disabled={locked || overview.isFetching}
          onClick={() => void overview.refetch()}
        >
          刷新订阅账号
        </Button>
        <Button
          disabled={locked || !accountReady}
          onClick={() => void fetchModels()}
        >
          {busy === "fetch" ? "加载中…" : "查看模型选项"}
        </Button>
      </div>
      {modelIds.length > 0 ? (
        <GroupedModelChips
          ids={modelIds}
          selectedId={modelId}
          onSelect={
            locked
              ? undefined
              : (value) => {
                  setModelId(value);
                  setSaved(null);
                }
          }
          emptyLabel="尚未加载模型选项"
        />
      ) : null}
      {modelId ? <p className="fy-models-muted">已选模型：{modelId}</p> : null}
      <Collapsible open={manualModel} onOpenChange={setManualModel}>
        <CollapsibleTrigger asChild>
          <Button
            disabled={locked || !accountReady}
            aria-expanded={manualModel}
          >
            {manualModel ? "收起手动输入" : "手动填写模型 ID"}
          </Button>
        </CollapsibleTrigger>
        <CollapsibleContent open={manualModel}>
          <div className="fy-control-field">
            <label htmlFor={`${id}-model`}>订阅模型 ID</label>
            <Input
              id={`${id}-model`}
              value={modelId}
              disabled={locked || !accountReady}
              maxLength={128}
              onChange={(event) => {
                setModelId(event.target.value);
                setSaved(null);
              }}
              aria-invalid={
                modelId.length > 0 && !isXaiSubscriptionModelId(modelId.trim())
              }
              aria-describedby={`${id}-model-help`}
            />
            <p id={`${id}-model-help`} className="fy-models-muted">
              从选项中选择，或输入订阅支持的模型 ID。
            </p>
          </div>
        </CollapsibleContent>
      </Collapsible>
      <div className="fy-models-inline-fields">
        <Button
          disabled={!canBind || props.writeTargets.length === 0}
          dialogOriginRef={originRef}
          onClick={() =>
            setPending({
              app: props.app === "codex" ? "codex" : "claude",
              accountId,
              modelId: modelId.trim(),
            })
          }
        >
          {props.app === "codex" ? "保存 Codex 订阅配置" : "应用到 Claude Code"}
        </Button>
      </div>
      <FieldFeedback notice={notice} />
      {saved?.app === "codex" ? (
        <Button disabled={locked} onClick={() => openAuth(true)}>
          继续预览 Codex 配置
        </Button>
      ) : null}
      <Dialog
        open={pending !== null}
        originRef={originRef}
        initialFocusRef={cancelRef}
        onOpenChange={(open) => {
          if (!open && !lock.current) setPending(null);
        }}
        title={
          pending
            ? `确认${pending.app === "claude" ? "应用" : "保存"} ${TARGET_LABELS[pending.app]} 订阅配置`
            : "确认订阅配置"
        }
        description={
          pending?.app === "codex"
            ? "先保存账号和模型选择，再到账号与认证预览并确认切换。"
            : "将目标软件连接到 FyAgent 的本机转发服务，并保留原配置备份。"
        }
        actions={
          <>
            <Button
              ref={cancelRef}
              disabled={busy === "bind"}
              onClick={() => setPending(null)}
            >
              取消
            </Button>
            <Button
              className="fy-control-button-primary"
              disabled={locked || !pendingAccountReady}
              onClick={() => void confirmBind()}
            >
              确认{pending?.app === "claude" ? "应用" : "保存"}
            </Button>
          </>
        }
      >
        {pending ? (
          <>
            <p>
              账号：
              {accounts.find((item) => item.accountId === pending.accountId)
                ?.displayName ??
                accounts.find((item) => item.accountId === pending.accountId)
                  ?.login}
            </p>
            <p>模型：{pending.modelId}</p>
          </>
        ) : null}
        {pending?.app === "claude" ? (
          <FileWriteDisclosure targets={props.writeTargets} />
        ) : null}
      </Dialog>
    </ModelsSection>
  );
}
