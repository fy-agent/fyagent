import { useQueryClient } from "@tanstack/react-query";
import {
  useCallback,
  useMemo,
  useRef,
  useState,
  type ChangeEvent,
  type FormEvent,
} from "react";
import { type BlockerFunction } from "react-router-dom";

import { getPromptAppBrand } from "../../shared/assets/apps";
import { errorMessage, isNativeOnlyError } from "../../shared/features/helpers";
import { useFeatures } from "../../shared/features/provider";
import {
  featureKeys,
  usePromptLibraries,
  usePromptLiveFile,
  usePrompts,
} from "../../shared/features/queries";
import {
  PRODUCT_DIRECTORY,
  PROMPT_ONLY_DIRECTORY,
} from "../../shared/features/directory";
import {
  PROMPT_APP_IDS,
  type ManagedPrompt,
  type PromptAppId,
} from "../../shared/features/types";
import {
  CatalogDetail,
  CatalogList,
  CatalogListItem,
  CatalogMasterDetail,
  CatalogRail,
} from "../../shared/ui/catalog";
import {
  Badge,
  Button,
  ConfirmDialog,
  EmptyState,
  InlineNotice,
  Input,
  Spinner,
  Switch,
} from "../../shared/ui/primitives";
import { usePrimaryBlocker } from "../../shared/ui/PrimaryBlocker";
import { FeatureList, FeatureListItem } from "../../shared/ui/FeatureList";
import { FeatureSearch } from "../../shared/ui/FeatureSearch";
import { SplitPanes } from "../../shared/ui/split";

import "./page.css";

const APP_LABELS = {
  ...Object.fromEntries(
    PRODUCT_DIRECTORY.flatMap((entry) =>
      entry.promptAppId ? [[entry.promptAppId, entry.displayName]] : [],
    ),
  ),
  ...Object.fromEntries(
    PROMPT_ONLY_DIRECTORY.map((entry) => [
      entry.promptAppId,
      entry.displayName,
    ]),
  ),
} as Record<PromptAppId, string>;

const REFRESH_WARNING = "写入可能已完成，但状态刷新失败";
const PROMPT_SPLIT_LABELS = ["调整列表与编辑的宽度"];

interface PromptDraft {
  name: string;
  description: string;
  content: string;
}

interface EditorState {
  mode: "new" | "edit";
  prompt: ManagedPrompt | null;
  draft: PromptDraft;
  baseline: PromptDraft | null;
}

type DiscardIntent =
  | { kind: "close-editor" }
  | { kind: "switch-app"; app: PromptAppId }
  | { kind: "select"; id: string }
  | { kind: "new" }
  | { kind: "route" }
  | null;

function toDraft(prompt: ManagedPrompt): PromptDraft {
  return {
    name: prompt.name,
    description: prompt.description ?? "",
    content: prompt.content,
  };
}

function isSameDraft(first: PromptDraft, second: PromptDraft): boolean {
  return (
    first.name === second.name &&
    first.description === second.description &&
    first.content === second.content
  );
}

function createNewEditor(): EditorState {
  return {
    mode: "new",
    prompt: null,
    draft: { name: "", description: "", content: "" },
    baseline: null,
  };
}

function createEditEditor(prompt: ManagedPrompt): EditorState {
  const draft = toDraft(prompt);
  return {
    mode: "edit",
    prompt,
    draft,
    baseline: { ...draft },
  };
}

function resolveEditor(
  editor: EditorState | null,
  selected: ManagedPrompt | null,
): EditorState | null {
  if (editor?.mode === "new") return editor;
  const dirty =
    editor !== null &&
    (editor.baseline === null || !isSameDraft(editor.draft, editor.baseline));
  if (dirty) return editor;
  if (!selected) return null;
  if (
    editor?.prompt?.id === selected.id &&
    editor.baseline !== null &&
    isSameDraft(editor.baseline, toDraft(selected))
  ) {
    return editor;
  }
  return createEditEditor(selected);
}

function formatTimestamp(timestamp: number | undefined): string {
  if (timestamp === undefined) return "—";
  const milliseconds =
    timestamp > 10_000_000_000 ? timestamp : timestamp * 1000;
  const date = new Date(milliseconds);
  return Number.isNaN(date.getTime()) ? "—" : date.toLocaleString();
}

function searchPrompts(
  prompts: readonly ManagedPrompt[],
  query: string,
): ManagedPrompt[] {
  const normalized = query.trim().toLocaleLowerCase();
  if (!normalized) return [...prompts];
  return prompts.filter((prompt) =>
    [prompt.name, prompt.description, prompt.content, prompt.id]
      .filter((value): value is string => typeof value === "string")
      .some((value) => value.toLocaleLowerCase().includes(normalized)),
  );
}

export function PromptsPage() {
  const queryClient = useQueryClient();
  const { ports, notify } = useFeatures();
  const [app, setApp] = useState<PromptAppId>("claude");
  const promptsQuery = usePrompts(app);
  const promptLibraries = usePromptLibraries();
  const liveFileQuery = usePromptLiveFile(app);
  const [search, setSearch] = useState("");
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [editor, setEditor] = useState<EditorState | null>(null);
  const [deleteTarget, setDeleteTarget] = useState<ManagedPrompt | null>(null);
  const [discardIntent, setDiscardIntent] = useState<DiscardIntent>(null);
  const [busy, setBusy] = useState(false);
  const [writeError, setWriteError] = useState<string | null>(null);
  const [refreshWarning, setRefreshWarning] = useState(false);
  const writeLock = useRef(false);

  const prompts = useMemo(() => promptsQuery.data ?? [], [promptsQuery.data]);
  const filtered = useMemo(
    () => searchPrompts(prompts, search),
    [prompts, search],
  );
  const selected =
    editor?.mode === "new"
      ? null
      : selectedId
        ? (prompts.find((prompt) => prompt.id === selectedId) ?? null)
        : (filtered[0] ?? null);
  const enabledByApp = useMemo(() => {
    const counts = {} as Record<PromptAppId, number | null>;
    for (const [index, id] of PROMPT_APP_IDS.entries()) {
      const query = promptLibraries[index];
      counts[id] = query?.data
        ? query.data.filter((prompt) => prompt.enabled).length
        : query?.isPending
          ? null
          : 0;
    }
    return counts;
  }, [promptLibraries]);

  const editorDirty =
    editor !== null &&
    (editor.baseline === null || !isSameDraft(editor.draft, editor.baseline));
  const shouldBlockNavigation = useCallback<BlockerFunction>(
    ({ currentLocation, nextLocation }) =>
      editorDirty && currentLocation.pathname !== nextLocation.pathname,
    [editorDirty],
  );
  const blocker = usePrimaryBlocker(shouldBlockNavigation);
  const activeDiscardIntent: DiscardIntent =
    discardIntent ?? (blocker.state === "blocked" ? { kind: "route" } : null);
  const activeEditor = resolveEditor(editor, selected);

  const refresh = async (targetApp: PromptAppId): Promise<boolean> => {
    await Promise.all([
      queryClient.invalidateQueries({
        queryKey: featureKeys.prompts(targetApp),
        refetchType: "none",
      }),
      queryClient.invalidateQueries({
        queryKey: featureKeys.promptLiveFile(targetApp),
        refetchType: "none",
      }),
    ]);
    const results = await Promise.allSettled([
      queryClient.fetchQuery({
        queryKey: featureKeys.prompts(targetApp),
        queryFn: () => ports.prompts.getAll(targetApp),
      }),
      queryClient.fetchQuery({
        queryKey: featureKeys.promptLiveFile(targetApp),
        queryFn: () => ports.prompts.getCurrentFileContent(targetApp),
      }),
    ]);
    return results.every((result) => result.status === "fulfilled");
  };

  const write = async (
    title: string,
    operation: () => Promise<void>,
  ): Promise<"failed" | "refreshed" | "refresh-failed"> => {
    if (writeLock.current) return "failed";
    writeLock.current = true;
    setBusy(true);
    setWriteError(null);
    setRefreshWarning(false);
    const targetApp = app;
    try {
      await operation();
    } catch (error) {
      const message = errorMessage(error);
      setWriteError(`${title}失败：${message}`);
      notify({ tone: "error", title: `${title}失败`, description: message });
      setBusy(false);
      writeLock.current = false;
      return "failed";
    }

    const refreshed = await refresh(targetApp);
    setBusy(false);
    writeLock.current = false;
    if (!refreshed) {
      setRefreshWarning(true);
      notify({ tone: "error", title: REFRESH_WARNING });
      return "refresh-failed";
    }
    notify({ tone: "success", title });
    return "refreshed";
  };

  const resetWorkspace = (nextApp?: PromptAppId) => {
    setEditor(null);
    setDeleteTarget(null);
    setSelectedId(null);
    setSearch("");
    setWriteError(null);
    setRefreshWarning(false);
    if (nextApp) setApp(nextApp);
  };

  const requestAppChange = (nextApp: PromptAppId) => {
    if (nextApp === app || busy) return;
    if (editorDirty) {
      setDiscardIntent({ kind: "switch-app", app: nextApp });
      return;
    }
    resetWorkspace(nextApp);
  };

  const requestSelect = (id: string) => {
    if (busy || id === selected?.id) return;
    if (editorDirty) {
      setDiscardIntent({ kind: "select", id });
      return;
    }
    setEditor(null);
    setSelectedId(id);
  };

  const requestNew = () => {
    if (busy || nativeUnavailable) return;
    if (editor?.mode === "new" && !editorDirty) return;
    if (editorDirty) {
      setDiscardIntent({ kind: "new" });
      return;
    }
    setSelectedId(null);
    setEditor(createNewEditor());
  };

  const requestEditorClose = () => {
    if (busy) return;
    if (editorDirty) {
      setDiscardIntent({ kind: "close-editor" });
      return;
    }
    setEditor(selected ? createEditEditor(selected) : null);
  };

  const cancelDiscard = () => {
    if (activeDiscardIntent?.kind === "route" && blocker.state === "blocked") {
      blocker.reset();
    }
    setDiscardIntent(null);
  };

  const confirmDiscard = () => {
    const intent = activeDiscardIntent;
    setDiscardIntent(null);
    if (intent?.kind === "switch-app") {
      resetWorkspace(intent.app);
    } else if (intent?.kind === "select") {
      setEditor(null);
      setSelectedId(intent.id);
    } else if (intent?.kind === "new") {
      setSelectedId(null);
      setEditor(createNewEditor());
    } else if (intent?.kind === "close-editor") {
      setEditor(selected ? createEditEditor(selected) : null);
    } else if (intent?.kind === "route" && blocker.state === "blocked") {
      setEditor(null);
      blocker.proceed();
    }
  };

  const updateDraft =
    (field: keyof PromptDraft) =>
    (event: ChangeEvent<HTMLInputElement | HTMLTextAreaElement>) => {
      const value = event.target.value;
      setEditor((current) => {
        const base = current ?? (selected ? createEditEditor(selected) : null);
        return base
          ? { ...base, draft: { ...base.draft, [field]: value } }
          : current;
      });
      setWriteError(null);
    };

  const saveEditor = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (!activeEditor || busy || !activeEditor.draft.name.trim()) return;
    const now = Math.floor(Date.now() / 1000);
    const prompt: ManagedPrompt = {
      id: activeEditor.prompt?.id ?? `prompt-${Date.now()}`,
      name: activeEditor.draft.name.trim(),
      description: activeEditor.draft.description.trim() || undefined,
      content: activeEditor.draft.content.trim(),
      enabled: activeEditor.prompt?.enabled ?? false,
      createdAt: activeEditor.prompt?.createdAt ?? now,
      updatedAt: now,
    };
    const result = await write(
      activeEditor.mode === "new" ? "提示词已创建" : "提示词已保存",
      () => ports.prompts.upsert(app, prompt),
    );
    if (result !== "failed") {
      setSelectedId(prompt.id);
      setEditor(createEditEditor(prompt));
    }
  };

  const importFromFile = async () => {
    let importedId = "";
    const result = await write("提示词已从文件导入", async () => {
      importedId = await ports.prompts.importFromFile(app);
    });
    if (result !== "failed" && importedId) {
      setSelectedId(importedId);
      setEditor(null);
    }
  };

  const togglePrompt = async (prompt: ManagedPrompt, enabled: boolean) => {
    await write(enabled ? "提示词已启用" : "提示词已停用", () =>
      enabled
        ? ports.prompts.enable(app, prompt.id)
        : ports.prompts.upsert(app, { ...prompt, enabled: false }),
    );
  };

  const requestDelete = (prompt: ManagedPrompt) => {
    if (prompt.enabled) {
      setWriteError("已启用提示词不能删除，请先停用后再删除");
      return;
    }
    setWriteError(null);
    setDeleteTarget(prompt);
  };

  const nativeUnavailable =
    promptsQuery.data === undefined && isNativeOnlyError(promptsQuery.error);
  const readFailed = promptsQuery.error && promptsQuery.data === undefined;
  const enabledCount = prompts.filter((prompt) => prompt.enabled).length;

  const workspaceBody = nativeUnavailable ? (
    <EmptyState
      title="桌面能力不可用"
      description="提示词管理仅在 FyAgent 桌面应用中可用。"
    />
  ) : promptsQuery.isPending && promptsQuery.data === undefined ? (
    <EmptyState
      title={`正在加载 ${APP_LABELS[app]} 提示词`}
      description="正在读取该应用的提示词"
    >
      <Spinner />
    </EmptyState>
  ) : readFailed ? (
    <EmptyState
      title={`无法加载 ${APP_LABELS[app]} 提示词`}
      description={errorMessage(promptsQuery.error)}
      actions={
        <Button onClick={() => void promptsQuery.refetch()}>重试</Button>
      }
    />
  ) : prompts.length === 0 && activeEditor?.mode !== "new" ? (
    <EmptyState
      title={`${APP_LABELS[app]} 还没有提示词`}
      description="可以新建提示词，或从当前文件导入。"
      actions={
        <>
          <Button disabled={busy} onClick={() => void importFromFile()}>
            从文件导入
          </Button>{" "}
          <Button
            className="fy-control-button-primary"
            disabled={busy}
            onClick={requestNew}
          >
            新建提示词
          </Button>
        </>
      }
    />
  ) : filtered.length === 0 && activeEditor?.mode !== "new" && !selected ? (
    <EmptyState
      title="没有匹配的提示词"
      description={`已加载 ${prompts.length} 条 ${APP_LABELS[app]} 提示词，可清空或调整搜索条件。`}
      actions={<Button onClick={() => setSearch("")}>清空搜索</Button>}
    />
  ) : (
    <SplitPanes
      maxWidths={[240]}
      minWidths={[200, 380]}
      separatorLabels={PROMPT_SPLIT_LABELS}
    >
      <section
        className="fy-feature-panel fy-prompts-library"
        aria-label="提示词列表"
      >
        <h2>
          提示词库 · {prompts.length}
          <span className="fy-prompts-heading-meta">
            {enabledCount} 条已启用
          </span>
        </h2>
        {selected &&
        search.trim() &&
        !filtered.some((prompt) => prompt.id === selected.id) ? (
          <p className="fy-feature-description">
            当前编辑的提示词不在搜索结果中。
          </p>
        ) : null}
        <FeatureList id="prompts-list">
          {filtered.length === 0 ? (
            <p className="fy-feature-description">没有匹配的提示词</p>
          ) : null}
          {filtered.map((prompt) => (
            <FeatureListItem
              key={prompt.id}
              selected={prompt.id === selected?.id}
              title={prompt.name}
              onSelect={() => requestSelect(prompt.id)}
            >
              <span>
                {prompt.description || "暂无描述"} ·{" "}
                {prompt.enabled ? "已启用" : "未启用"}
              </span>
            </FeatureListItem>
          ))}
        </FeatureList>
      </section>
      {activeEditor ? (
        <PromptEditorPane
          appLabel={APP_LABELS[app]}
          busy={busy}
          editor={activeEditor}
          enabled={selected?.enabled ?? false}
          liveError={liveFileQuery.error}
          livePending={
            liveFileQuery.isPending && liveFileQuery.data === undefined
          }
          liveValue={liveFileQuery.data}
          onCloseNew={requestEditorClose}
          onDelete={selected ? () => requestDelete(selected) : undefined}
          onDraftChange={updateDraft}
          onSave={saveEditor}
          onToggle={
            selected
              ? (enabled) => void togglePrompt(selected, enabled)
              : undefined
          }
        />
      ) : (
        <section
          className="fy-feature-panel fy-prompts-editor-pane"
          aria-label="提示词详情"
        >
          <EmptyState
            title="选择一条提示词"
            description="从左侧打开提示词后即可直接阅读和编辑正文。"
          />
        </section>
      )}
    </SplitPanes>
  );

  return (
    <div
      className="fy-feature-page fy-split-page fy-catalog-page fy-prompts-page"
      data-testid="prompts-page"
      data-data-source="native"
    >
      <header className="fy-feature-header">
        <div className="fy-feature-heading">
          <h1>提示词</h1>
          <p>按应用管理可启用的提示词。点开后直接阅读和编辑正文。</p>
        </div>
        <div className="fy-feature-actions">
          <Button
            disabled={busy || nativeUnavailable}
            onClick={() => void importFromFile()}
          >
            从文件导入
          </Button>
          <Button
            className="fy-control-button-primary"
            disabled={busy || nativeUnavailable}
            onClick={requestNew}
          >
            新建提示词
          </Button>
        </div>
      </header>

      {writeError && <InlineNotice tone="error">{writeError}</InlineNotice>}
      {refreshWarning && (
        <InlineNotice tone="warning">
          {REFRESH_WARNING}
          。已保留上一次成功读取的数据，请重试刷新后再继续操作。
        </InlineNotice>
      )}
      {promptsQuery.error && promptsQuery.data !== undefined && (
        <InlineNotice tone="error">
          提示词刷新失败，正在显示上一次成功数据：
          {errorMessage(promptsQuery.error)}
        </InlineNotice>
      )}

      <div className="fy-feature-workspace">
        <CatalogMasterDetail>
          <CatalogRail ariaLabel="提示词应用" title="应用">
            <CatalogList>
              {PROMPT_APP_IDS.map((id) => (
                <CatalogListItem
                  key={id}
                  asset={getPromptAppBrand(id)}
                  label={APP_LABELS[id]}
                  summary={
                    enabledByApp[id] === null
                      ? "读取中"
                      : `${enabledByApp[id]} 条已启用`
                  }
                  selected={id === app}
                  disabled={busy}
                  testId={`prompt-app-${id}`}
                  onSelect={() => requestAppChange(id)}
                />
              ))}
            </CatalogList>
          </CatalogRail>
          <CatalogDetail ariaLabel={`${APP_LABELS[app]} 提示词工作区`}>
            <div className="fy-feature-toolbar">
              <label className="fy-control-field">
                搜索
                <FeatureSearch
                  ariaLabel="搜索提示词"
                  placeholder="搜索名称、描述、内容或 ID"
                  value={search}
                  disabled={nativeUnavailable}
                  onValueChange={setSearch}
                />
              </label>
            </div>
            {workspaceBody}
          </CatalogDetail>
        </CatalogMasterDetail>
      </div>

      <ConfirmDialog
        open={deleteTarget !== null}
        title={`删除 ${deleteTarget?.name ?? "提示词"}`}
        description="删除后无法从提示词库恢复；只有未启用的提示词可以删除。"
        pending={busy}
        onCancel={() => setDeleteTarget(null)}
        onConfirm={async () => {
          const target = deleteTarget;
          if (!target) return;
          const result = await write("提示词已删除", () =>
            ports.prompts.delete(app, target.id),
          );
          if (result !== "failed") setDeleteTarget(null);
        }}
      />

      <ConfirmDialog
        open={activeDiscardIntent !== null}
        title="放弃未保存的提示词更改"
        description="当前编辑内容尚未保存。确认放弃后再继续切换或离开页面。"
        pending={busy}
        onCancel={cancelDiscard}
        onConfirm={confirmDiscard}
      />
    </div>
  );
}

function PromptIdentityFields({
  autoFocusName = false,
  busy,
  description,
  name,
  onDraftChange,
}: {
  autoFocusName?: boolean;
  busy: boolean;
  description: string;
  name: string;
  onDraftChange: (
    field: keyof PromptDraft,
  ) => (event: ChangeEvent<HTMLInputElement | HTMLTextAreaElement>) => void;
}) {
  return (
    <div className="fy-prompts-editor-identity">
      <label className="fy-control-field">
        名称
        <Input
          autoFocus={autoFocusName}
          aria-label="名称"
          value={name}
          disabled={busy}
          onChange={onDraftChange("name")}
        />
      </label>
      <label className="fy-control-field">
        描述
        <Input
          aria-label="描述"
          value={description}
          disabled={busy}
          onChange={onDraftChange("description")}
        />
      </label>
    </div>
  );
}

function PromptEditorPane({
  appLabel,
  busy,
  editor,
  enabled,
  liveError,
  livePending,
  liveValue,
  onCloseNew,
  onDelete,
  onDraftChange,
  onSave,
  onToggle,
}: {
  appLabel: string;
  busy: boolean;
  editor: EditorState;
  enabled: boolean;
  liveError: unknown;
  livePending: boolean;
  liveValue: string | null | undefined;
  onCloseNew: () => void;
  onDelete?: () => void;
  onDraftChange: (
    field: keyof PromptDraft,
  ) => (event: ChangeEvent<HTMLInputElement | HTMLTextAreaElement>) => void;
  onSave: (event: FormEvent<HTMLFormElement>) => void;
  onToggle?: (enabled: boolean) => void;
}) {
  const title =
    editor.mode === "new"
      ? `新建 ${appLabel} 提示词`
      : (editor.prompt?.name ?? "提示词");

  return (
    <section
      className="fy-feature-panel fy-prompts-editor-pane"
      aria-label="提示词详情"
    >
      <form
        id="fy-prompts-editor-form"
        className="fy-prompts-editor-form"
        onSubmit={onSave}
      >
        <header className="fy-prompts-editor-head">
          <div className="fy-feature-detail-title">
            <h2>{title}</h2>
            {editor.mode === "edit" && (
              <Badge tone={enabled ? "accent" : "neutral"}>
                {enabled ? "已启用" : "未启用"}
              </Badge>
            )}
          </div>
          {editor.prompt && onToggle ? (
            <Switch
              checked={enabled}
              disabled={busy}
              label={`${enabled ? "停用" : "启用"}${editor.prompt.name}`}
              onCheckedChange={onToggle}
            />
          ) : null}
          <div className="fy-feature-actions">
            <Button
              className="fy-control-button-primary"
              disabled={busy || !editor.draft.name.trim()}
              type="submit"
            >
              {busy ? "保存中…" : "保存"}
            </Button>
            {editor.mode === "new" ? (
              <Button disabled={busy} onClick={onCloseNew} type="button">
                取消
              </Button>
            ) : (
              <Button
                className="fy-control-button-danger"
                disabled={busy}
                onClick={onDelete}
                type="button"
              >
                删除
              </Button>
            )}
          </div>
        </header>
        {editor.mode === "new" ? (
          <p className="fy-feature-description">
            保存后会写入该应用的提示词库，但不会自动启用。
          </p>
        ) : (
          <p className="fy-prompts-editor-meta">
            {appLabel} · 更新于 {formatTimestamp(editor.prompt?.updatedAt)}
          </p>
        )}
        {editor.mode === "new" ? (
          <PromptIdentityFields
            autoFocusName
            busy={busy}
            description={editor.draft.description}
            name={editor.draft.name}
            onDraftChange={onDraftChange}
          />
        ) : null}
        <label className="fy-control-field fy-prompts-editor-content-field">
          内容
          <textarea
            className="fy-control-textarea fy-prompts-editor-content"
            aria-label="内容"
            value={editor.draft.content}
            disabled={busy}
            spellCheck={false}
            onChange={onDraftChange("content")}
          />
        </label>
        {editor.mode === "edit" ? (
          <PromptIdentityFields
            busy={busy}
            description={editor.draft.description}
            name={editor.draft.name}
            onDraftChange={onDraftChange}
          />
        ) : null}
      </form>
      <details className="fy-prompts-live">
        <summary>当前使用的内容 · {appLabel}</summary>
        {livePending ? (
          <Spinner label="正在读取当前使用的内容" />
        ) : liveError && liveValue === undefined ? (
          <InlineNotice tone="error">
            暂时无法读取当前使用的内容：{errorMessage(liveError)}
          </InlineNotice>
        ) : liveValue === null || liveValue === undefined ? (
          <p className="fy-feature-description">当前没有使用中的内容。</p>
        ) : (
          <textarea
            className="fy-control-textarea fy-prompts-live-content"
            aria-label="当前使用的内容"
            value={liveValue}
            readOnly
            spellCheck={false}
          />
        )}
        {liveError != null && liveValue !== undefined && (
          <InlineNotice tone="error">
            当前内容刷新失败，正在显示已加载内容：
            {errorMessage(liveError)}
          </InlineNotice>
        )}
      </details>
    </section>
  );
}
