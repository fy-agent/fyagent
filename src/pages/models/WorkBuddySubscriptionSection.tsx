import { useEffect, useId, useRef, useState } from "react";
import type { ChangeJobSnapshot, ChangePlan } from "../../shared/features/change-plans";
import type { ModelWriteTarget, WorkBuddySubscriptionPlanRequest } from "../../shared/features/models";
import { useFeatures } from "../../shared/features/provider";
import { useManagedAuthOverview } from "../../shared/features/queries";
import { isXaiSubscriptionModelId } from "../../shared/features/xai-subscription";
import { hasUnconfirmedAuthority } from "../../shared/features/change-plans-ui/view-model";
import { Button } from "../../shared/ui/Button";
import { Input } from "../../shared/ui/primitives";
import { FieldFeedback, ModelsSection, type Notice } from "./feedback";
import { GroupedModelChips } from "./modelChips";
import { WorkBuddySubscriptionPlanWorkspace } from "./apply/WorkBuddySubscriptionPlanWorkspace";

type Props = {
  active: boolean;
  disabled: boolean;
  writeTargets: readonly ModelWriteTarget[];
  onBeginWrite: () => boolean;
  onEndWrite: () => void;
  onUnconfirmed: () => void;
  onSaved: () => void;
};

export function WorkBuddySubscriptionSection(props: Props) {
  const { ports } = useFeatures();
  const overview = useManagedAuthOverview(props.active);
  const id = useId();
  const [accountId, setAccountId] = useState("");
  const [modelId, setModelId] = useState("");
  const [models, setModels] = useState<string[]>([]);
  const [pending, setPending] = useState<{
    request: WorkBuddySubscriptionPlanRequest;
    plan: ChangePlan;
    provider: "openai" | "xai";
    accountName: string;
  } | null>(null);
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState<Notice | null>(null);
  const [blocked, setBlocked] = useState(false);
  const generation = useRef(0);
  const catalogGeneration = useRef(0);
  const localLock = useRef(false);
  const attempted = useRef(false);
  const uncertain = useRef(false);
  useEffect(() => () => {
    generation.current++;
    catalogGeneration.current++;
  }, []);
  useEffect(() => {
    if (!props.active) catalogGeneration.current++;
  }, [props.active]);
  const accounts = overview.data?.accounts.filter(a => a.provider === "openai" || a.provider === "xai") ?? [];
  const selected = accounts.find(a => a.accountId === accountId);
  const ready = !overview.isError && selected?.health === "ready";
  const locked = !props.active || props.disabled || busy || pending !== null || blocked;
  const markUnconfirmed = () => {
    uncertain.current = true;
    setBlocked(true);
    props.onUnconfirmed();
    setNotice({ tone: "warning", title: "当前配置待确认", description: "已暂停修改。请重新打开页面并检查当前配置后再设置。" });
  };
  const selectAccount = async (nextId: string) => {
    if (locked) return;
    const revision = ++catalogGeneration.current;
    setAccountId(nextId);
    setModelId("");
    setModels([]);
    setNotice(null);
    if (accounts.find(a => a.accountId === nextId)?.provider !== "xai") return;
    try {
      const result = await ports.providers.fetchXaiManagedModels(nextId);
      if (revision === catalogGeneration.current) setModels(result.models);
    } catch {
      if (revision === catalogGeneration.current)
        setNotice({ tone: "warning", title: "无法加载模型选项", description: "可以手动填写订阅支持的模型 ID。" });
    }
  };
  const preview = async () => {
    if (locked || localLock.current || !ready || !isXaiSubscriptionModelId(modelId.trim()) || !props.writeTargets.length || !props.onBeginWrite()) return;
    localLock.current = true;
    attempted.current = false;
    uncertain.current = false;
    setBusy(true);
    setNotice(null);
    const revision = ++generation.current;
    const provider = selected.provider;
    if (provider !== "openai" && provider !== "xai") return;
    try {
      const source = await ports.workbuddy.getSubscriptionSource();
      if (revision !== generation.current) return;
      const request = { accountId, modelId: modelId.trim(), expectedRevision: source.revision, expectedBindingVersion: source.bindingVersion };
      const plan = await ports.changePlans.createWorkBuddySubscriptionPlan(request);
      if (revision !== generation.current) return;
      setPending({ request, plan, provider, accountName: selected.displayName ?? selected.login });
    } catch {
      if (revision !== generation.current) return;
      localLock.current = false;
      props.onEndWrite();
      setNotice({ tone: "error", title: "无法预览订阅设置", description: "请检查账号和当前配置后重试。" });
    } finally {
      if (revision === generation.current) setBusy(false);
    }
  };
  const terminal = async (job: ChangeJobSnapshot) => {
    if (!pending) return;
    const revision = generation.current;
    if (hasUnconfirmedAuthority(job) || !["succeeded", "warning"].includes(job.status)) {
      markUnconfirmed();
      return;
    }
    try {
      // Bypass cached query data: only this post-job native read can confirm saving.
      const result = await ports.workbuddy.getSubscriptionSource();
      if (revision !== generation.current) return;
      const source = result.source;
      if (!source || !result.revision || source.accountId !== pending.request.accountId ||
          source.provider !== pending.provider || source.modelId !== pending.request.modelId ||
          source.localBaseUrl !== pending.plan.targetProviderName ||
          result.bindingVersion !== source.bindingVersion ||
          source.bindingVersion <= (pending.request.expectedBindingVersion ?? 0)) {
        markUnconfirmed();
        return;
      }
      attempted.current = false;
      setNotice({ tone: "info", title: "WorkBuddy 订阅配置已保存", description: "请保持 FyAgent 运行，并在 WorkBuddy 中检查模型。保存配置不代表对话调用成功。" });
      props.onSaved();
    } catch {
      if (revision === generation.current) markUnconfirmed();
    }
  };
  const dismiss = () => {
    generation.current++;
    setPending(null);
    localLock.current = false;
    if (attempted.current || uncertain.current) markUnconfirmed();
    else props.onEndWrite();
  };
  return <ModelsSection title="使用账号订阅（实验性）" ariaLabel="账号订阅设置">
    <p className="fy-models-muted">使用订阅时需要 FyAgent 保持运行；退出后停止转发。OpenAI 请填写账号可用的模型 ID，xAI 选项不代表账号可用性。</p>
    {overview.isError ? <p>无法读取订阅账号，请刷新后重试。</p> : null}
    {!overview.isPending && accounts.length === 0 ? <p>请先到账号与认证登录 ChatGPT 或 xAI 账号。</p> : null}
    <fieldset disabled={locked || overview.isError}>
      <legend>订阅账号</legend>
      {accounts.map(a => <label key={a.accountId} className="fy-models-checkbox-row">
        <input type="radio" name={`${id}-account`} checked={a.accountId === accountId}
          disabled={a.health !== "ready"} onChange={() => void selectAccount(a.accountId)} />
        {a.provider === "openai" ? "ChatGPT" : "xAI"} · {a.displayName ?? a.login}{a.health !== "ready" ? "（暂不可用）" : ""}
      </label>)}
    </fieldset>
    <GroupedModelChips ids={models} selectedId={modelId} onSelect={locked || !ready ? undefined : setModelId} emptyLabel="可手动填写模型 ID" />
    <label className="fy-control-field" htmlFor={`${id}-model`}>订阅模型 ID</label>
    <Input id={`${id}-model`} value={modelId} maxLength={128} disabled={locked || !ready} onChange={e => { setModelId(e.target.value); setNotice(null); }} />
    <Button disabled={locked || !ready || !isXaiSubscriptionModelId(modelId.trim()) || !props.writeTargets.length} onClick={() => void preview()}>预览 WorkBuddy 订阅设置</Button>
    <FieldFeedback notice={notice} />
    {pending ? <>
      <p>账号：{pending.accountName} · 模型：{pending.request.modelId}</p>
      <WorkBuddySubscriptionPlanWorkspace active={props.active} request={pending.request} plan={pending.plan}
        writeTargets={props.writeTargets} previewError={null}
        canApply={!blocked && !overview.isError && accounts.some(a => a.accountId === pending.request.accountId && a.health === "ready")}
        onApplyStart={() => { attempted.current = true; }} onApplyError={markUnconfirmed}
        onPlanChange={plan => { if (plan) setPending(current => current ? { ...current, plan } : null); }}
        onTerminal={job => void terminal(job)} onDismiss={dismiss} />
    </> : null}
  </ModelsSection>;
}
