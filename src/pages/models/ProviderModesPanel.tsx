// CC Switch v4.0.4 ModeTabs/ModeDialog/ClaudeStackModelsField flows,
// composed with FyAgent's existing controls and typed native port.
import { useRef, useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import type { Provider } from "../../domain/configuration/types";
import type { AppMode } from "../../domain/configuration/types/proxy";
import {
  isOfficialAccount,
  providerNeedsRouting,
} from "../../domain/configuration/serialization/providerCapabilities";
import { isPlainObject } from "../../domain/configuration/serialization/providerConfigStructural";
import { extractCodexBaseUrl } from "../../domain/configuration/serialization/providerConfigUtils";
import {
  modelsDevQueryOptions,
  type ModelsDevResponse,
} from "../../domain/configuration/modelsDev";
import { resolveModelMetadata } from "../../domain/configuration/modelMetadata";
import { codexPresetModelSources } from "../../domain/configuration/presets/presetModelMetadata";
import { useFeatures } from "../../shared/features/provider";
import { featureKeys } from "../../shared/features/queries";
import { Button } from "../../shared/ui/Button";
import { Dialog } from "../../shared/ui/Dialog";
import { FeatureTabs, FeatureTabPanel } from "../../shared/ui/FeatureTabs";
import {
  Checkbox,
  InlineNotice,
  Input,
  SecretInput,
  Spinner,
} from "../../shared/ui/primitives";
import {
  fillCodexCatalogModel,
  providerDefaultModel,
  publishedModels,
  REASONING_LEVELS,
  withPublishedModels,
  type PublishedModel,
} from "./aggregationModels";

type App = "claude" | "codex";
const MODES = [
  { id: "direct", label: "直连" },
  { id: "route", label: "路由" },
  { id: "stack", label: "聚合" },
] as const;
const MODE_NAMES: Record<AppMode, string> = {
  direct: "直连",
  route: "路由",
  stack: "聚合",
};
const NOTICE_COPY = {
  routeOwnsCatalog:
    "默认供应商正在使用自己的模型列表。使用聚合模型列表后，其他供应商的模型才能一起显示。",
  officialModelsBundled:
    "暂未取得账号专属模型，当前使用 Codex 自带的官方模型列表。",
  officialModelsUnavailable: "暂时无法取得官方模型列表，请刷新后重试。",
};

export function ProviderModesPanel({
  app,
  active,
  disabled,
  view,
  onViewChange,
}: {
  app: App;
  active: boolean;
  disabled: boolean;
  view: AppMode;
  onViewChange: (mode: AppMode) => void;
}) {
  const { ports } = useFeatures();
  const queryClient = useQueryClient();
  const mode = useQuery({
    queryKey: featureKeys.providerMode(app),
    queryFn: () => ports.providers.getMode(app),
    enabled: active,
  });
  const stack = useQuery({
    queryKey: featureKeys.providerStack(app),
    queryFn: () => ports.providers.getStack(app),
    enabled: active,
  });
  const providers = useQuery({
    queryKey: featureKeys.providerList(app),
    queryFn: () => ports.providers.getAll(app),
    enabled: active,
    gcTime: 0,
  });
  const [busy, setBusy] = useState(false);
  const lock = useRef(false);
  const [notice, setNotice] = useState<{
    error?: boolean;
    text: string;
  } | null>(null);
  const [enter, setEnter] = useState<"route" | "stack" | null>(null);
  const [pick, setPick] = useState("");
  const [restart, setRestart] = useState(false);
  const [needsRoute, setNeedsRoute] = useState<Provider | null>(null);
  const [editor, setEditor] = useState<Provider | "new" | null>(null);
  const origin = useRef<HTMLElement | null>(null);
  const list = Object.values(providers.data ?? {});
  const eligible = list.filter(
    (provider) => app === "codex" || !isOfficialAccount(app, provider),
  );
  const current = mode.data;
  const members = stack.data?.members ?? [];
  const defaultId = current?.routeProviderId;
  const hasOfficial = list.some((provider) => isOfficialAccount(app, provider));
  const staleClients = stack.data?.staleClients;
  const currentName = list.find(
    (provider) =>
      provider.id ===
      (current?.mode === "direct" ? current.directProviderId : defaultId),
  )?.name;
  const blocked = disabled || busy || !active;
  const refresh = async () => {
    await Promise.all([
      queryClient.invalidateQueries(
        { queryKey: featureKeys.providerMode(app) },
        { throwOnError: true },
      ),
      queryClient.invalidateQueries(
        { queryKey: featureKeys.providerStack(app) },
        { throwOnError: true },
      ),
      queryClient.invalidateQueries(
        { queryKey: featureKeys.providerList(app) },
        { throwOnError: true },
      ),
      queryClient.invalidateQueries({
        queryKey: featureKeys.providerSummary(app),
      }),
    ]);
  };
  const perform = async (
    operation: () => Promise<unknown>,
    success?: string,
  ) => {
    if (lock.current || blocked) return false;
    lock.current = true;
    setBusy(true);
    setNotice(null);
    let applied = false;
    try {
      const warning = await operation();
      applied = true;
      await refresh();
      if (typeof warning === "string" && warning) setNotice({ text: warning });
      else if (success) setNotice({ text: success });
      return true;
    } catch (error) {
      setNotice({
        error: true,
        text: applied
          ? "设置已更新，但无法刷新状态，请重试读取。"
          : isPlainObject(error) && error.partial === true
            ? "部分设置已保存。请刷新状态后重试。"
            : "未能完成操作，请检查供应商配置后重试。",
      });
      if (!applied) await refresh().catch(() => undefined);
      return applied;
    } finally {
      lock.current = false;
      setBusy(false);
    }
  };
  const openEnter = (target: "route" | "stack") => {
    setPick(
      eligible.find((provider) => provider.id === defaultId)?.id ??
        eligible.find((provider) => provider.id === current?.directProviderId)
          ?.id ??
        eligible[0]?.id ??
        "",
    );
    setEnter(target);
  };
  const switchDirect = (id?: string) =>
    perform(
      async () => {
        if (current?.mode !== "direct")
          await ports.providers.setMode(app, false);
        if (id && current?.directProviderId !== id)
          await ports.providers.switch(app, id);
      },
      app === "codex"
        ? "已切换为直连。请重新打开 Codex 使配置生效。"
        : "已切换为直连。",
    );
  const providerRow = (provider: Provider) => {
    const member = members.find((item) => item.providerId === provider.id);
    const official = isOfficialAccount(app, provider);
    const isDefault = provider.id === defaultId;
    return (
      <div className="fy-aggregation-provider" key={provider.id}>
        <div>
          <strong>{provider.name}</strong>
          <p className="fy-models-muted">
            {official
              ? "官方账号"
              : `${publishedModels(provider, app).length} 个模型`}
            {view === "stack" && isDefault ? " · 默认供应商" : ""}
          </p>
        </div>
        <div className="fy-aggregation-actions">
          {!official && (
            <Button
              disabled={blocked}
              dialogOriginRef={origin}
              onClick={() => setEditor(provider)}
            >
              编辑模型
            </Button>
          )}
          {view === "direct" && (
            <Button
              disabled={
                blocked ||
                (current?.mode === "direct" &&
                  current.directProviderId === provider.id)
              }
              dialogOriginRef={origin}
              onClick={() =>
                providerNeedsRouting(app, provider)
                  ? setNeedsRoute(provider)
                  : void switchDirect(provider.id)
              }
            >
              {current?.mode === "direct" &&
              current.directProviderId === provider.id
                ? "当前配置"
                : "直连使用"}
            </Button>
          )}
          {view === "route" && (
            <Button
              disabled={
                blocked ||
                !current ||
                (app === "claude" && official) ||
                (current.mode === "route" && isDefault)
              }
              onClick={() =>
                current?.mode === "route"
                  ? void perform(() =>
                      ports.providers.setRoute(app, provider.id),
                    )
                  : (setPick(provider.id), setEnter("route"))
              }
            >
              {current?.mode === "route" && isDefault
                ? "当前路由"
                : "路由到此供应商"}
            </Button>
          )}
          {view === "stack" && !official && (
            <Button
              disabled={blocked || Boolean(member?.route)}
              onClick={() =>
                void perform(() =>
                  ports.providers.setStackMember(app, provider.id, !member),
                )
              }
            >
              {member ? "移出聚合" : "加入聚合"}
            </Button>
          )}
          {view === "stack" &&
            current?.mode === "stack" &&
            !(app === "claude" && official) && (
              <Button
                disabled={blocked || isDefault}
                onClick={() =>
                  void perform(() => ports.providers.setRoute(app, provider.id))
                }
              >
                {isDefault ? "默认供应商" : "设为默认供应商"}
              </Button>
            )}
        </div>
      </div>
    );
  };

  return (
    <section className="fy-aggregation">
      <FeatureTabs
        id={`provider-mode-${app}`}
        label="连接模式"
        value={view}
        onChange={onViewChange}
        options={MODES}
      />
      <div className="fy-aggregation-heading">
        <p>
          当前：<strong>{current ? MODE_NAMES[current.mode] : "读取中"}</strong>
          {currentName ? ` · ${currentName}` : ""}
          {current && current.mode !== "direct" && !current.attached
            ? " · 尚未连接"
            : ""}
        </p>
        <Button
          disabled={blocked}
          dialogOriginRef={origin}
          onClick={() => setEditor("new")}
        >
          添加供应商
        </Button>
      </div>
      {notice && (
        <InlineNotice tone={notice.error ? "error" : "info"}>
          {notice.text}
        </InlineNotice>
      )}
      {(mode.isError || stack.isError || providers.isError) && (
        <InlineNotice tone="error">
          无法读取连接模式。
          <Button onClick={() => void refresh().catch(() => undefined)}>
            重试
          </Button>
        </InlineNotice>
      )}
      {(mode.isLoading || providers.isLoading) && (
        <Spinner label="正在读取连接模式" />
      )}
      {MODES.map((tab) => (
        <FeatureTabPanel
          key={tab.id}
          tabsId={`provider-mode-${app}`}
          value={tab.id}
          active={view === tab.id}
          layout="flow"
        >
          {view === tab.id && (
            <>
              {tab.id !== "direct" && (
                <div className="fy-aggregation-heading">
                  <p className="fy-models-muted">
                    {tab.id === "stack"
                      ? "在原生模型列表中切换供应商，不自动故障转移。"
                      : "通过本地路由连接供应商，保留已有故障转移设置。"}
                  </p>
                  <Button
                    className="fy-control-button-primary"
                    disabled={
                      blocked ||
                      !current ||
                      eligible.length === 0 ||
                      current.mode === tab.id
                    }
                    dialogOriginRef={origin}
                    onClick={() => openEnter(tab.id)}
                  >
                    {current?.mode === tab.id ? "已启用" : `启用${tab.label}…`}
                  </Button>
                </div>
              )}
              {tab.id === "direct" && current && current.mode !== "direct" && (
                <Button disabled={blocked} onClick={() => void switchDirect()}>
                  回到直连
                </Button>
              )}
              {tab.id === "stack" && (
                <>
                  <p className="fy-models-muted">
                    {members.length} 家供应商 ·{" "}
                    {members.reduce(
                      (sum, member) => sum + member.modelIds.length,
                      0,
                    )}{" "}
                    个模型
                  </p>
                  {hasOfficial && (
                    <p className="fy-models-muted">
                      {app === "claude"
                        ? "Claude 官方订阅请使用直连。"
                        : "ChatGPT 官方订阅只能作为默认供应商。"}
                    </p>
                  )}
                  {stack.data?.notice && (
                    <InlineNotice tone="warning">
                      {NOTICE_COPY[stack.data.notice]}
                      {stack.data.notice === "routeOwnsCatalog" && (
                        <Button
                          disabled={blocked}
                          onClick={() =>
                            void perform(() =>
                              ports.providers.adoptCodexCatalog(),
                            )
                          }
                        >
                          使用聚合模型列表
                        </Button>
                      )}
                    </InlineNotice>
                  )}
                  {app === "codex" &&
                    (staleClients?.daemon || staleClients?.others) && (
                      <InlineNotice tone="warning">
                        Codex 仍在使用旧模型列表。
                        {staleClients.others &&
                          "桌面版请完全退出后重新打开（macOS 使用 ⌘Q），编辑器插件请重启窗口。"}
                        {staleClients.daemon && (
                          <Button
                            disabled={blocked}
                            dialogOriginRef={origin}
                            onClick={() => setRestart(true)}
                          >
                            重启 Codex 命令行服务…
                          </Button>
                        )}
                      </InlineNotice>
                    )}
                  <h3>聚合名单</h3>
                  {members.length === 0 ? (
                    <p className="fy-models-muted">
                      从下方供应商加入模型，启用时选择默认供应商。
                    </p>
                  ) : (
                    list
                      .filter((provider) =>
                        members.some(
                          (member) => member.providerId === provider.id,
                        ),
                      )
                      .map(providerRow)
                  )}
                  <h3>可用供应商</h3>
                  {list
                    .filter(
                      (provider) =>
                        !members.some(
                          (member) => member.providerId === provider.id,
                        ),
                    )
                    .map(providerRow)}
                </>
              )}
              {tab.id !== "stack" && list.map(providerRow)}
            </>
          )}
        </FeatureTabPanel>
      ))}
      <Dialog
        open={active && needsRoute !== null}
        originRef={origin}
        onOpenChange={(open) => !open && !busy && setNeedsRoute(null)}
        title={`${needsRoute?.name ?? "此供应商"} 需要路由`}
        description="这家供应商的协议转换或账号认证需要 FyAgent 本地路由。"
        actions={
          <>
            <Button disabled={busy} onClick={() => setNeedsRoute(null)}>
              取消
            </Button>
            <Button
              disabled={blocked}
              onClick={() =>
                void (async () => {
                  if (needsRoute && (await switchDirect(needsRoute.id)))
                    setNeedsRoute(null);
                })()
              }
            >
              仍然直连使用
            </Button>
            <Button
              className="fy-control-button-primary"
              disabled={blocked}
              onClick={() =>
                void (async () => {
                  if (
                    needsRoute &&
                    (await perform(() =>
                      ports.providers.setMode(app, true, false, needsRoute.id),
                    ))
                  )
                    setNeedsRoute(null);
                })()
              }
            >
              启用路由并使用
            </Button>
          </>
        }
      />
      <Dialog
        open={active && enter !== null}
        originRef={origin}
        onOpenChange={(open) => !open && !busy && setEnter(null)}
        title={`启用${enter ? MODE_NAMES[enter] : ""}`}
        actions={
          <>
            <Button disabled={busy} onClick={() => setEnter(null)}>
              取消
            </Button>
            <Button
              className="fy-control-button-primary"
              disabled={blocked || !pick}
              onClick={() =>
                void (async () => {
                  if (
                    enter &&
                    (await perform(() =>
                      ports.providers.setMode(
                        app,
                        true,
                        enter === "stack",
                        pick,
                      ),
                    ))
                  )
                    setEnter(null);
                })()
              }
            >
              确认启用
            </Button>
          </>
        }
      >
        <label className="fy-aggregation-field">
          {enter === "stack" ? "默认供应商" : "路由供应商"}
          <select
            className="fy-control-input"
            value={pick}
            onChange={(event) => setPick(event.target.value)}
          >
            {eligible.map((provider) => (
              <option key={provider.id} value={provider.id}>
                {provider.name}
              </option>
            ))}
          </select>
        </label>
        <p>
          {enter === "stack"
            ? `除默认供应商外，将发布 ${members.filter((member) => member.providerId !== pick).reduce((sum, member) => sum + member.modelIds.length, 0)} 个模型。普通模型请求使用默认供应商，选中聚合模型后直达对应供应商。`
            : "客户端将通过 FyAgent 本地路由发送请求。"}
        </p>
        <p className="fy-models-muted">
          将更新{" "}
          {app === "claude"
            ? "~/.claude/settings.json"
            : "~/.codex/config.toml"}{" "}
          并自动启动本地路由。使用期间保持 FyAgent
          运行；正常退出时还原直连配置。
          {app === "codex" ? " 完成后请重新打开 Codex。" : ""}
        </p>
        {enter === "stack" &&
          app === "codex" &&
          list.some(
            (provider) =>
              isOfficialAccount(app, provider) && provider.id !== pick,
          ) && (
            <InlineNotice tone="warning">
              未选为默认的官方账号不会出现在聚合列表中。
            </InlineNotice>
          )}
        {notice?.error && (
          <InlineNotice tone="error">{notice.text}</InlineNotice>
        )}
      </Dialog>
      <Dialog
        open={active && restart}
        originRef={origin}
        onOpenChange={(open) => !open && !busy && setRestart(false)}
        title="重启 Codex 命令行服务"
        description="重启会中断服务中正在运行的任务。完成后重新打开命令行会话以加载新模型列表。"
        actions={
          <>
            <Button disabled={busy} onClick={() => setRestart(false)}>
              取消
            </Button>
            <Button
              disabled={blocked}
              onClick={() =>
                void (async () => {
                  if (
                    await perform(async () => {
                      const result = await ports.providers.restartCodexDaemon();
                      return result === "restarted"
                        ? "Codex 命令行服务已重启。"
                        : "Codex 命令行服务未运行，可直接打开新会话。";
                    })
                  )
                    setRestart(false);
                })()
              }
            >
              确认重启
            </Button>
          </>
        }
      />
      {editor && active && (
        <ProviderModelEditor
          key={editor === "new" ? "new" : editor.id}
          app={app}
          provider={editor === "new" ? null : editor}
          originRef={origin}
          onClose={() => setEditor(null)}
          onSaved={async () => {
            try {
              await refresh();
              setNotice({ text: "模型已保存。" });
            } catch {
              setNotice({
                error: true,
                text: "模型已保存，但无法刷新列表，请重试读取。",
              });
            }
          }}
        />
      )}
    </section>
  );
}

function ProviderModelEditor({
  app,
  provider,
  originRef,
  onClose,
  onSaved,
}: {
  app: App;
  provider: Provider | null;
  originRef: { current: HTMLElement | null };
  onClose: () => void;
  onSaved: () => Promise<void>;
}) {
  const { ports } = useFeatures();
  const queryClient = useQueryClient();
  const [name, setName] = useState(provider?.name ?? "");
  const [baseUrl, setBaseUrl] = useState("");
  const [apiKey, setApiKey] = useState("");
  const [format, setFormat] = useState(
    app === "claude" ? "anthropic" : "openai_responses",
  );
  const [rows, setRows] = useState<(PublishedModel & { rowId: string })[]>(() =>
    (provider ? publishedModels(provider, app) : []).map((row) => ({
      ...row,
      rowId: crypto.randomUUID(),
    })),
  );
  const rowsRef = useRef(rows);
  const metadataGeneration = useRef(0);
  const commitRows = (updateRows: (current: typeof rows) => typeof rows) => {
    rowsRef.current = updateRows(rowsRef.current);
    setRows(rowsRef.current);
  };
  const [defaultModel, setDefaultModel] = useState(
    provider ? providerDefaultModel(provider, app) : "",
  );
  const [fetched, setFetched] = useState<string[]>([]);
  const [selected, setSelected] = useState<string[]>([]);
  const [search, setSearch] = useState("");
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState("");
  const lock = useRef(false);
  const env =
    provider && isPlainObject(provider.settingsConfig.env)
      ? provider.settingsConfig.env
      : {};
  const auth =
    provider && isPlainObject(provider.settingsConfig.auth)
      ? provider.settingsConfig.auth
      : {};
  const config =
    typeof provider?.settingsConfig.config === "string"
      ? provider.settingsConfig.config
      : "";
  const endpoint = provider
    ? app === "claude"
      ? typeof env.ANTHROPIC_BASE_URL === "string"
        ? env.ANTHROPIC_BASE_URL
        : ""
      : (extractCodexBaseUrl(config) ?? "")
    : baseUrl;
  const storedKey =
    app === "claude"
      ? (env.ANTHROPIC_AUTH_TOKEN ?? env.ANTHROPIC_API_KEY)
      : auth.OPENAI_API_KEY;
  const credential = provider
    ? typeof storedKey === "string"
      ? storedKey
      : ""
    : apiKey;
  const filtered = fetched.filter(
    (id) =>
      id.toLowerCase().includes(search.trim().toLowerCase()) &&
      !rows.some((row) => row.model === id),
  );
  const clearFetchedModels = () => {
    metadataGeneration.current += 1;
    setFetched([]);
    setSelected([]);
    setMessage("");
  };
  const update = (rowId: string, patch: Partial<PublishedModel>) =>
    commitRows((current) =>
      current.map((row) => (row.rowId === rowId ? { ...row, ...patch } : row)),
    );
  const fetchModels = async () => {
    const generation = metadataGeneration.current;
    setBusy(true);
    setMessage("");
    try {
      const models = await ports.providers.fetchModels(endpoint, credential);
      if (metadataGeneration.current !== generation) return;
      setFetched(models.map((model) => model.id));
      if (models.length === 0) setMessage("供应商未返回模型，可手动添加。");
      if (app === "codex")
        void queryClient.prefetchQuery(modelsDevQueryOptions);
    } catch {
      if (metadataGeneration.current === generation)
        setMessage("无法获取模型，请检查供应商地址和 API Key，或手动添加。");
    } finally {
      setBusy(false);
    }
  };
  const fillModels = (ids: string[]) => {
    setMessage("");
    const generation = metadataGeneration.current;
    const presets = codexPresetModelSources();
    const apply = (data?: ModelsDevResponse) => {
      if (metadataGeneration.current !== generation) return;
      const sources = new Set<string>();
      const next = rowsRef.current.map((row) => {
        if (!ids.includes(row.model)) return row;
        const metadata = resolveModelMetadata(row.model, {
          baseUrl: endpoint,
          presets,
          modelsDev: data,
        });
        if (!metadata) return row;
        const filled = fillCodexCatalogModel(row, metadata);
        if (JSON.stringify(filled) !== JSON.stringify(row))
          metadata.sources.forEach((source) => sources.add(source));
        return filled;
      });
      if (!sources.size) return;
      commitRows(() => next);
      const source = [...sources]
        .map((value) => (value === "preset" ? "同地址预设" : "models.dev"))
        .join("、");
      setMessage(`已补全空白参数（来源：${source}），请核对。`);
    };
    const cached = queryClient.getQueryData(modelsDevQueryOptions.queryKey);
    apply(cached);
    if (!cached)
      void queryClient
        .fetchQuery(modelsDevQueryOptions)
        .then(apply)
        .catch(() => undefined);
  };
  const addSelected = () => {
    commitRows((current) => [
      ...current,
      ...selected
        .filter((id) => !current.some((row) => row.model === id))
        .map((model) => ({ model, rowId: crypto.randomUUID() })),
    ]);
    if (app === "codex") void fillModels(selected);
    setSelected([]);
  };
  const save = async () => {
    if (lock.current) return;
    if (!name.trim() || (!provider && (!baseUrl.trim() || !apiKey.trim()))) {
      setMessage("请填写供应商名称、API 地址和 API Key。");
      return;
    }
    lock.current = true;
    setBusy(true);
    setMessage("");
    const models = rows.map(({ rowId, ...row }) => {
      void rowId;
      return row;
    });
    const firstModel = defaultModel || models[0]?.model || "";
    const wireApi =
      format === "anthropic"
        ? "anthropic"
        : format === "openai_chat"
          ? "chat"
          : "responses";
    const seed: Provider = provider ?? {
      id: crypto.randomUUID(),
      name: name.trim(),
      category: "custom",
      meta: {
        apiFormat:
          format === "anthropic"
            ? "anthropic"
            : format === "openai_chat"
              ? "openai_chat"
              : "openai_responses",
        // A configured marker with no extension header means explicitly off.
        ...(app === "codex" ? { imageExtensionConfigured: true } : {}),
      },
      settingsConfig:
        app === "claude"
          ? {
              env: {
                ANTHROPIC_BASE_URL: baseUrl.trim(),
                ANTHROPIC_AUTH_TOKEN: apiKey.trim(),
                ANTHROPIC_MODEL: firstModel,
              },
            }
          : {
              auth: { OPENAI_API_KEY: apiKey.trim() },
              config: `model_provider = "custom"\nmodel = ${JSON.stringify(firstModel)}\n[model_providers.custom]\nname = ${JSON.stringify(name.trim())}\nbase_url = ${JSON.stringify(baseUrl.trim())}\nwire_api = "${wireApi}"\nrequires_openai_auth = true\n`,
            },
    };
    try {
      const saved = withPublishedModels(
        { ...seed, name: name.trim() },
        app,
        models,
        firstModel,
      );
      if (provider) await ports.providers.update(app, saved);
      else await ports.providers.add(app, saved);
      setApiKey("");
      await onSaved();
      onClose();
    } catch {
      setMessage("未能保存供应商，请检查配置后重试。");
    } finally {
      lock.current = false;
      setBusy(false);
    }
  };
  return (
    <Dialog
      open
      originRef={originRef}
      size="wide"
      title={provider ? `编辑 ${provider.name} 的模型` : "添加供应商"}
      onOpenChange={(open) => !open && !busy && onClose()}
      actions={
        <>
          <Button disabled={busy} onClick={onClose}>
            取消
          </Button>
          <Button
            className="fy-control-button-primary"
            disabled={busy}
            onClick={() => void save()}
          >
            保存供应商
          </Button>
        </>
      }
    >
      <div className="fy-aggregation-editor">
        <label className="fy-aggregation-field">
          供应商名称
          <Input
            value={name}
            onChange={(event) => setName(event.target.value)}
          />
        </label>
        {!provider && (
          <>
            <label className="fy-aggregation-field">
              API 地址
              <Input
                value={baseUrl}
                onChange={(event) => {
                  clearFetchedModels();
                  setBaseUrl(event.target.value);
                }}
                placeholder="https://api.example.com/v1"
              />
            </label>
            <label className="fy-aggregation-field">
              API Key
              <SecretInput
                value={apiKey}
                onChange={(event) => {
                  clearFetchedModels();
                  setApiKey(event.target.value);
                }}
              />
            </label>
            <label className="fy-aggregation-field">
              API 协议
              <select
                className="fy-control-input"
                value={format}
                onChange={(event) => setFormat(event.target.value)}
              >
                <option value="anthropic">Anthropic Messages</option>
                <option value="openai_responses">OpenAI Responses</option>
                <option value="openai_chat">OpenAI Chat Completions</option>
              </select>
            </label>
          </>
        )}
        <div className="fy-aggregation-heading">
          <h3>发布到原生选择器的模型</h3>
          <div className="fy-aggregation-actions">
            <Button
              disabled={busy || !endpoint || !credential}
              onClick={() => void fetchModels()}
            >
              获取模型
            </Button>
            <Button
              disabled={busy}
              onClick={() =>
                commitRows((current) => [
                  ...current,
                  { model: "", rowId: crypto.randomUUID() },
                ])
              }
            >
              手动添加
            </Button>
            {app === "codex" && (
              <Button
                disabled={busy || !rows.length}
                onClick={() => void fillModels(rows.map((row) => row.model))}
              >
                补全空白参数
              </Button>
            )}
          </div>
        </div>
        <p className="fy-models-muted">
          {app === "claude"
            ? "第一行（★）为默认模型，用于启动、后台任务与子代理。"
            : "★ 为默认请求模型。模型参数应与这家供应商实际支持的能力一致。"}
        </p>
        {app === "codex" && (
          <label className="fy-aggregation-field">
            默认请求模型
            <Input
              value={defaultModel}
              onChange={(event) => setDefaultModel(event.target.value)}
              placeholder={rows[0]?.model || "留空时使用模型列表第一行"}
            />
          </label>
        )}
        {message && <InlineNotice>{message}</InlineNotice>}
        {fetched.length > 0 && (
          <div className="fy-models-section">
            <label className="fy-aggregation-field">
              搜索模型
              <Input
                value={search}
                onChange={(event) => setSearch(event.target.value)}
              />
            </label>
            <div className="fy-aggregation-actions">
              <Button onClick={() => setSelected(filtered)}>
                选择全部搜索结果
              </Button>
              <Button disabled={!selected.length} onClick={addSelected}>
                添加选中的 {selected.length} 个模型
              </Button>
            </div>
            <div className="fy-aggregation-fetched">
              {filtered.map((id) => (
                <label key={id}>
                  <Checkbox
                    label={`选择 ${id}`}
                    checked={selected.includes(id)}
                    onCheckedChange={(checked) =>
                      setSelected((current) =>
                        checked
                          ? [...current, id]
                          : current.filter((item) => item !== id),
                      )
                    }
                  />
                  {id}
                </label>
              ))}
            </div>
          </div>
        )}
        {rows.length === 0 && (
          <p className="fy-models-muted">
            还没有模型。获取供应商模型后批量添加，或手动填写。
          </p>
        )}
        {rows.map((row, index) => (
          <div key={row.rowId} className="fy-aggregation-model">
            <div className="fy-aggregation-model-main">
              <Button
                aria-label={`将 ${row.model || index + 1} 设为默认模型`}
                aria-pressed={
                  app === "claude" ? index === 0 : row.model === defaultModel
                }
                onClick={() => {
                  setDefaultModel(row.model);
                  if (app === "claude")
                    commitRows((current) => [
                      row,
                      ...current.filter((other) => other.rowId !== row.rowId),
                    ]);
                }}
              >
                {(app === "claude" ? index === 0 : row.model === defaultModel)
                  ? "★"
                  : "☆"}
              </Button>
              <label className="fy-aggregation-field">
                模型 ID
                <Input
                  value={row.model}
                  onChange={(event) =>
                    update(row.rowId, { model: event.target.value })
                  }
                />
              </label>
              <label className="fy-aggregation-field">
                显示名称
                <Input
                  value={row.displayName ?? ""}
                  onChange={(event) =>
                    update(row.rowId, { displayName: event.target.value })
                  }
                />
              </label>
              <Button
                aria-label={`移除模型 ${row.model || index + 1}`}
                onClick={() =>
                  commitRows((current) =>
                    current.filter((other) => other.rowId !== row.rowId),
                  )
                }
              >
                移除
              </Button>
            </div>
            {app === "claude" ? (
              <label className="fy-aggregation-check">
                <Checkbox
                  label={`${row.model || index + 1} 支持 1M 上下文`}
                  checked={row.oneM === true || /\[1m\]/i.test(row.model)}
                  onCheckedChange={(checked) =>
                    update(row.rowId, {
                      model: row.model.replace(/\[1m\]/gi, ""),
                      oneM: checked,
                    })
                  }
                />
                1M 上下文
              </label>
            ) : (
              <div className="fy-aggregation-model-options">
                <label className="fy-aggregation-field">
                  上下文窗口
                  <Input
                    type="number"
                    min="1"
                    value={row.contextWindow ?? ""}
                    onChange={(event) =>
                      update(row.rowId, { contextWindow: event.target.value })
                    }
                  />
                </label>
                <label className="fy-aggregation-check">
                  <Checkbox
                    label={`${row.model || index + 1} 支持图片`}
                    checked={row.inputModalities?.includes("image") ?? false}
                    onCheckedChange={(checked) =>
                      update(row.rowId, {
                        inputModalities: checked ? ["text", "image"] : ["text"],
                      })
                    }
                  />
                  图片输入
                </label>
                <fieldset className="fy-aggregation-reasoning">
                  <legend>思考档位</legend>
                  {REASONING_LEVELS.map((level) => (
                    <label key={level}>
                      <Checkbox
                        label={`${row.model || index + 1} ${level}`}
                        checked={row.reasoningLevels?.includes(level) ?? false}
                        onCheckedChange={(checked) =>
                          update(row.rowId, {
                            defaultReasoningLevel:
                              !checked && row.defaultReasoningLevel === level
                                ? undefined
                                : row.defaultReasoningLevel,
                            reasoningLevels: REASONING_LEVELS.filter(
                              (candidate) =>
                                candidate === level
                                  ? checked
                                  : row.reasoningLevels?.includes(candidate),
                            ),
                          })
                        }
                      />
                      {level}
                    </label>
                  ))}
                </fieldset>
                {Boolean(row.reasoningLevels?.length) && (
                  <label className="fy-aggregation-field">
                    默认思考档位
                    <select
                      className="fy-control-input"
                      value={row.defaultReasoningLevel ?? ""}
                      onChange={(event) =>
                        update(row.rowId, {
                          defaultReasoningLevel:
                            event.target.value || undefined,
                        })
                      }
                    >
                      <option value="">自动</option>
                      {row.reasoningLevels?.map((level) => (
                        <option key={level} value={level}>
                          {level}
                        </option>
                      ))}
                    </select>
                  </label>
                )}
              </div>
            )}
          </div>
        ))}
      </div>
    </Dialog>
  );
}
