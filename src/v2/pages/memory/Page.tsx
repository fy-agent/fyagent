import { MagnifyingGlassIcon } from "@phosphor-icons/react/dist/csr/MagnifyingGlass";
import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type FormEvent,
} from "react";
import { useBlocker, type BlockerFunction } from "react-router-dom";

import {
  agentTargetById,
  agentTargets,
  memoryWritableTargetIds,
  type AgentTargetId,
} from "@/v2/shared/config/agentTargets";

import {
  dailyMemoryItems,
  longTermMemoryItems,
  sessionSourceItems,
  type MemoryCategory,
  type MemoryLocalState,
  type MemoryPrototypeItem,
  type MemoryResourceState,
} from "./prototype";
import "./page.css";

const categoryLabels: Record<MemoryCategory, string> = {
  longTerm: "长期记忆",
  daily: "每日记录",
  sessions: "会话记录",
};

const categoryHeadings: Record<MemoryCategory, string> = {
  longTerm: "本机长期记忆",
  daily: "本机每日记录",
  sessions: "本机会话来源",
};

const categorySearchLabels: Record<MemoryCategory, string> = {
  longTerm: "搜索长期记忆",
  daily: "搜索每日记录",
  sessions: "搜索会话来源",
};

const localStateLabels: Record<MemoryLocalState, string> = {
  source: "来源记录",
  "saved-preview": "前端预览已保存",
  "changes-pending": "修改待保存",
  "managed-by-prompts": "由提示词管理",
};

const resourceStateLabels: Record<MemoryResourceState, string> = {
  exists: "已存在",
  missing: "未发现",
  "frontend-draft": "前端草稿 · 未创建文件",
};

const promotableSourceItems: readonly MemoryPrototypeItem[] = [
  ...dailyMemoryItems,
  ...sessionSourceItems,
];

interface MemoryListItem {
  id: string;
  name: string;
  meta: string;
  status: string;
}

function cloneMemoryItem(item: MemoryPrototypeItem): MemoryPrototypeItem {
  return {
    ...item,
    provenance: item.provenance ? { ...item.provenance } : null,
    syncTargetIds: [...item.syncTargetIds],
    previewTasks: item.previewTasks.map((task) => ({ ...task })),
  };
}

function sameTargetSet(
  left: readonly AgentTargetId[],
  right: readonly AgentTargetId[],
): boolean {
  return (
    left.length === right.length &&
    left.every((targetId) => right.includes(targetId))
  );
}

function matchesQuery(values: string[], query: string): boolean {
  const normalizedQuery = query.trim().toLocaleLowerCase();
  return (
    !normalizedQuery ||
    values.some((value) => value.toLocaleLowerCase().includes(normalizedQuery))
  );
}

function listStatus(
  item: MemoryPrototypeItem,
  isCurrentDirty: boolean,
): string {
  if (item.category === "longTerm") {
    return localStateLabels[
      isCurrentDirty ? "changes-pending" : item.localState
    ];
  }
  if (item.category === "sessions") {
    return `${item.itemCount ?? 0} 条`;
  }
  return `${item.itemCount ?? 0} 个文件`;
}

export function MemoryPage() {
  const nextDraftId = useRef(1);
  const [category, setCategory] = useState<MemoryCategory>("longTerm");
  const [itemsByCategory, setItemsByCategory] = useState<
    Record<MemoryCategory, MemoryPrototypeItem[]>
  >(() => ({
    longTerm: longTermMemoryItems.map(cloneMemoryItem),
    daily: dailyMemoryItems.map(cloneMemoryItem),
    sessions: sessionSourceItems.map(cloneMemoryItem),
  }));
  const [selectedIds, setSelectedIds] = useState<
    Record<MemoryCategory, string>
  >({
    longTerm: longTermMemoryItems[0].id,
    daily: dailyMemoryItems[0].id,
    sessions: sessionSourceItems[0].id,
  });
  const [queries, setQueries] = useState<Record<MemoryCategory, string>>({
    longTerm: "",
    daily: "",
    sessions: "",
  });
  const [draftTitle, setDraftTitle] = useState(longTermMemoryItems[0].title);
  const [draftContent, setDraftContent] = useState(
    longTermMemoryItems[0].content,
  );
  const [draftTargetIds, setDraftTargetIds] = useState<AgentTargetId[]>([
    ...longTermMemoryItems[0].syncTargetIds,
  ]);
  const [baseline, setBaseline] = useState<MemoryPrototypeItem | null>(() =>
    cloneMemoryItem(longTermMemoryItems[0]),
  );
  const [transientPromotedId, setTransientPromotedId] = useState<string | null>(
    null,
  );
  const [feedback, setFeedback] = useState("");

  const categoryItems = itemsByCategory[category];
  const selectedItem =
    categoryItems.find((item) => item.id === selectedIds[category]) ??
    categoryItems[0];
  const editorReadOnly =
    category !== "longTerm" ||
    selectedItem.owner === "prompts" ||
    !selectedItem.editableInPrototype;
  const isDirty =
    !editorReadOnly &&
    (baseline === null ||
      draftTitle.trim() !== baseline.title ||
      draftContent !== baseline.content ||
      !sameTargetSet(draftTargetIds, baseline.syncTargetIds));

  const shouldBlockNavigation = useCallback<BlockerFunction>(
    ({ currentLocation, nextLocation }) =>
      isDirty && currentLocation.pathname !== nextLocation.pathname,
    [isDirty],
  );
  const blocker = useBlocker(shouldBlockNavigation);

  useEffect(() => {
    if (blocker.state !== "blocked") {
      return;
    }

    if (window.confirm("当前内容尚未保存，确定放弃更改吗？")) {
      blocker.proceed();
    } else {
      blocker.reset();
    }
  }, [blocker]);

  const listItems = useMemo<MemoryListItem[]>(() => {
    const query = queries[category];
    return itemsByCategory[category]
      .filter((item) =>
        matchesQuery(
          [
            item.title,
            item.sourceLabel,
            item.purpose,
            item.path,
            item.storageKind,
            item.content,
          ],
          query,
        ),
      )
      .map((item) => ({
        id: item.id,
        name: item.title,
        meta: `${item.sourceLabel} · ${item.storageKind}`,
        status: listStatus(item, item.id === selectedItem.id && isDirty),
      }));
  }, [category, isDirty, itemsByCategory, queries, selectedItem.id]);

  const loadDraft = (
    item: MemoryPrototypeItem,
    nextBaseline: MemoryPrototypeItem | null = item,
  ) => {
    setDraftTitle(item.title);
    setDraftContent(item.content);
    setDraftTargetIds([...item.syncTargetIds]);
    setBaseline(nextBaseline ? cloneMemoryItem(nextBaseline) : null);
  };

  const removeTransientDraft = (): MemoryPrototypeItem | null => {
    if (!transientPromotedId) {
      return null;
    }

    const remainingItems = itemsByCategory.longTerm.filter(
      (item) => item.id !== transientPromotedId,
    );
    const fallbackItem = remainingItems[0] ?? null;
    setItemsByCategory((current) => ({
      ...current,
      longTerm: current.longTerm.filter(
        (item) => item.id !== transientPromotedId,
      ),
    }));
    setSelectedIds((current) =>
      current.longTerm === transientPromotedId && fallbackItem
        ? { ...current, longTerm: fallbackItem.id }
        : current,
    );
    setTransientPromotedId(null);
    return fallbackItem;
  };

  const confirmDiscard = (): boolean => {
    if (!isDirty) {
      return true;
    }
    if (!window.confirm("当前内容尚未保存，确定放弃更改吗？")) {
      return false;
    }
    removeTransientDraft();
    return true;
  };

  const switchCategory = (nextCategory: MemoryCategory) => {
    if (nextCategory === category || !confirmDiscard()) {
      return;
    }

    const nextItems = itemsByCategory[nextCategory];
    const nextItem =
      nextItems.find((item) => item.id === selectedIds[nextCategory]) ??
      nextItems[0];
    setCategory(nextCategory);
    loadDraft(nextItem);
    setFeedback("");
  };

  const selectItem = (id: string) => {
    if (id === selectedIds[category] || !confirmDiscard()) {
      return;
    }

    const nextItem = categoryItems.find((item) => item.id === id);
    if (!nextItem) {
      return;
    }

    setSelectedIds((currentIds) => ({ ...currentIds, [category]: id }));
    loadDraft(nextItem);
    setFeedback("");
  };

  const scanLocalAgents = () => {
    if (!confirmDiscard()) {
      return;
    }

    const fallbackItem =
      selectedItem.id === transientPromotedId
        ? itemsByCategory.longTerm.find(
            (item) => item.id !== transientPromotedId,
          )
        : null;
    const itemToReload = fallbackItem ?? baseline ?? selectedItem;
    if (fallbackItem) {
      setSelectedIds((current) => ({
        ...current,
        longTerm: fallbackItem.id,
      }));
    }
    loadDraft(itemToReload);
    setFeedback("模拟扫描：6 个工具、8 个 Agent 实例；未访问本机文件");
  };

  const saveMemory = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (editorReadOnly || !draftTitle.trim() || !isDirty) {
      return;
    }

    const nextItem: MemoryPrototypeItem = {
      ...selectedItem,
      title: draftTitle.trim(),
      content: draftContent,
      updatedAt: "刚刚",
      syncTargetIds: [...draftTargetIds],
      localState: "saved-preview",
      revision: selectedItem.revision + 1,
      previewTasks: [],
    };
    setItemsByCategory((current) => ({
      ...current,
      longTerm: current.longTerm.map((item) =>
        item.id === selectedItem.id ? nextItem : item,
      ),
    }));
    setDraftTitle(nextItem.title);
    setBaseline(cloneMemoryItem(nextItem));
    if (transientPromotedId === selectedItem.id) {
      setTransientPromotedId(null);
    }
    setFeedback("已保存到前端预览；尚未写入本机文件");
  };

  const toggleDraftTarget = (targetId: AgentTargetId) => {
    if (editorReadOnly) {
      return;
    }
    setDraftTargetIds((currentIds) =>
      currentIds.includes(targetId)
        ? currentIds.filter((id) => id !== targetId)
        : [...currentIds, targetId],
    );
    setFeedback("");
  };

  const previewLongTermSync = () => {
    if (selectedItem.owner === "prompts") {
      setFeedback("此文件由提示词页面管理");
      return;
    }
    if (baseline === null || isDirty) {
      setFeedback("请先保存当前修改，再生成同步预览");
      return;
    }
    if (baseline.syncTargetIds.length === 0) {
      setFeedback("请先选择并保存至少一个同步目标");
      return;
    }

    const previewTasks = baseline.syncTargetIds.map((targetId) => ({
      targetId,
      sourceRevision: baseline.revision,
      previewState: "pending" as const,
      durableState: "not-run" as const,
      createdAt: "刚刚",
      error: null,
    }));
    const nextItem: MemoryPrototypeItem = {
      ...selectedItem,
      previewTasks,
    };
    setItemsByCategory((current) => ({
      ...current,
      longTerm: current.longTerm.map((item) =>
        item.id === selectedItem.id ? nextItem : item,
      ),
    }));
    setBaseline(cloneMemoryItem(nextItem));
    setFeedback(
      `前端预览：已生成 ${previewTasks.length} 个待执行任务；未写入本机文件`,
    );
  };

  const promoteToLongTerm = () => {
    if (category === "longTerm" || !confirmDiscard()) {
      return;
    }

    const id = `prototype-long-term-${nextDraftId.current}`;
    nextDraftId.current += 1;
    const nextItem: MemoryPrototypeItem = {
      ...selectedItem,
      id,
      category: "longTerm",
      title: `${selectedItem.title} · 提炼草稿`,
      sourceLabel: `${selectedItem.sourceLabel} · 手动提炼`,
      purpose: "从原始记录提炼的长期记忆草稿",
      path: "FyAgent 前端草稿",
      storageKind: "Markdown",
      writable: true,
      editableInPrototype: true,
      searchable: true,
      itemCount: 1,
      updatedAt: "刚刚",
      resourceState: "frontend-draft",
      localState: "changes-pending",
      revision: 0,
      provenance: {
        sourceItemId: selectedItem.id,
        sourceTargetId: selectedItem.sourceTargetId,
        sourceToolId: selectedItem.toolId,
        sourcePath: selectedItem.path,
        sourceUpdatedAt: selectedItem.updatedAt,
        capturedAt: "刚刚",
        sourceSummary: selectedItem.purpose,
      },
      syncTargetIds: [],
      previewTasks: [],
      owner: "memory",
    };

    setItemsByCategory((current) => ({
      ...current,
      longTerm: [nextItem, ...current.longTerm],
    }));
    setSelectedIds((current) => ({ ...current, longTerm: id }));
    setCategory("longTerm");
    setTransientPromotedId(id);
    loadDraft(nextItem, null);
    setFeedback("已生成长期记忆草稿；原始记录保持不变");
  };

  const sourceTarget = agentTargetById(selectedItem.sourceTargetId);
  const provenanceSourceItem = selectedItem.provenance
    ? promotableSourceItems.find(
        (item) => item.id === selectedItem.provenance?.sourceItemId,
      )
    : undefined;
  const provenanceSourceTarget = selectedItem.provenance
    ? agentTargetById(selectedItem.provenance.sourceTargetId)
    : undefined;
  const lineCount = Math.max(6, draftContent.split("\n").length);
  const syncTargets = agentTargets.filter((target) =>
    memoryWritableTargetIds.includes(target.id),
  );
  const currentLocalState: MemoryLocalState = isDirty
    ? "changes-pending"
    : selectedItem.localState;
  const currentOperation =
    category !== "longTerm"
      ? "只读提炼"
      : selectedItem.owner === "prompts"
        ? "由提示词管理"
        : selectedItem.editableInPrototype
          ? "可编辑前端预览"
          : "只读来源";

  return (
    <section
      className="fy-memory-page"
      aria-labelledby="fy-memory-title"
      data-testid="memory-page"
      data-data-source="prototype"
    >
      <header className="fy-memory-header">
        <div className="fy-memory-title-group">
          <h1 id="fy-memory-title">记忆</h1>
          <p>用匿名化结构预览 6 个工具、8 个 Agent 实例的记忆来源</p>
          <strong className="fy-memory-prototype-status">
            前端原型 · 未读取或写入本机文件
          </strong>
        </div>
        <button
          className="fy-memory-primary-action"
          type="button"
          onClick={scanLocalAgents}
        >
          重新扫描本机
        </button>
      </header>

      <div className="fy-memory-tabs-row">
        <div className="fy-memory-tabs" role="tablist" aria-label="记忆分类">
          {(Object.keys(categoryLabels) as MemoryCategory[]).map(
            (categoryId) => (
              <button
                key={categoryId}
                id={`fy-memory-tab-${categoryId}`}
                className="fy-memory-tab"
                type="button"
                role="tab"
                aria-selected={category === categoryId}
                aria-controls="fy-memory-workspace"
                onClick={() => switchCategory(categoryId)}
              >
                {categoryLabels[categoryId]}
              </button>
            ),
          )}
        </div>
      </div>

      <div
        id="fy-memory-workspace"
        className="fy-memory-grid"
        role="tabpanel"
        aria-labelledby={`fy-memory-tab-${category}`}
      >
        <section
          className="fy-memory-pane fy-memory-library"
          aria-labelledby="fy-memory-library-title"
          data-testid="memory-library"
        >
          <h2 id="fy-memory-library-title">{categoryHeadings[category]}</h2>

          <label className="fy-memory-search">
            <MagnifyingGlassIcon size={24} weight="regular" aria-hidden />
            <span className="fy-memory-visually-hidden">
              {categorySearchLabels[category]}
            </span>
            <input
              type="search"
              value={queries[category]}
              aria-label={categorySearchLabels[category]}
              placeholder={categorySearchLabels[category]}
              onChange={(event) =>
                setQueries((currentQueries) => ({
                  ...currentQueries,
                  [category]: event.target.value,
                }))
              }
            />
          </label>

          <ul
            className="fy-memory-list"
            aria-label={`${categoryLabels[category]}列表`}
          >
            {listItems.map((item) => (
              <li key={item.id}>
                <button
                  className="fy-memory-list-row"
                  type="button"
                  aria-pressed={item.id === selectedIds[category]}
                  data-selected={
                    item.id === selectedIds[category] ? "true" : "false"
                  }
                  onClick={() => selectItem(item.id)}
                >
                  <span className="fy-memory-row-copy">
                    <strong>{item.name}</strong>
                    <small>{item.meta}</small>
                  </span>
                  <span className="fy-memory-row-state">
                    <em>{item.status}</em>
                    <span className="fy-memory-radio" aria-hidden />
                  </span>
                </button>
              </li>
            ))}
          </ul>

          {listItems.length === 0 ? (
            <p className="fy-memory-empty">没有匹配的内容</p>
          ) : null}
        </section>

        <section
          className="fy-memory-pane fy-memory-editor"
          aria-labelledby="fy-memory-editor-title"
          data-testid="memory-editor"
        >
          <h2 id="fy-memory-editor-title">{draftTitle}</h2>
          <form className="fy-memory-editor-form" onSubmit={saveMemory}>
            {!editorReadOnly ? (
              <label className="fy-memory-title-field">
                <span>标题</span>
                <input
                  type="text"
                  aria-label="记忆标题"
                  value={draftTitle}
                  onChange={(event) => {
                    setDraftTitle(event.target.value);
                    setFeedback("");
                  }}
                />
              </label>
            ) : null}

            <div
              className="fy-memory-code-editor"
              data-readonly={editorReadOnly ? "true" : "false"}
            >
              <ol className="fy-memory-line-numbers" aria-hidden>
                {Array.from({ length: lineCount }, (_, index) => (
                  <li key={index}>{index + 1}</li>
                ))}
              </ol>
              <textarea
                value={draftContent}
                aria-label="记忆内容"
                spellCheck={false}
                readOnly={editorReadOnly}
                onChange={(event) => {
                  setDraftContent(event.target.value);
                  setFeedback("");
                }}
              />
            </div>

            <footer className="fy-memory-editor-footer">
              <span className="fy-memory-feedback" aria-live="polite">
                {feedback}
              </span>
              <button
                className="fy-memory-save-button"
                type="submit"
                disabled={editorReadOnly || !draftTitle.trim() || !isDirty}
              >
                {editorReadOnly ? "只读来源" : "保存"}
              </button>
            </footer>
          </form>
        </section>

        <aside
          className="fy-memory-pane fy-memory-inspector"
          aria-labelledby="fy-memory-inspector-title"
          data-testid="memory-inspector"
        >
          <h2 id="fy-memory-inspector-title">来源与同步</h2>

          <dl className="fy-memory-details">
            <div>
              <dt>工具</dt>
              <dd>{sourceTarget?.name ?? "未知来源"}</dd>
            </div>
            <div>
              <dt>来源</dt>
              <dd>{selectedItem.sourceLabel}</dd>
            </div>
            <div>
              <dt>存储</dt>
              <dd>{selectedItem.storageKind}</dd>
            </div>
            <div>
              <dt>位置</dt>
              <dd title={selectedItem.path}>{selectedItem.path}</dd>
            </div>
            <div>
              <dt>路径状态</dt>
              <dd
                className="fy-memory-path-state"
                data-resource-state={selectedItem.resourceState}
              >
                {resourceStateLabels[selectedItem.resourceState]}
              </dd>
            </div>
            <div>
              <dt>来源能力</dt>
              <dd>
                {selectedItem.writable ? "可读写" : "只读"}
                {selectedItem.searchable ? " · 可搜索" : ""}
              </dd>
            </div>
            <div>
              <dt>本轮操作</dt>
              <dd>{currentOperation}</dd>
            </div>
            <div>
              <dt>数量</dt>
              <dd>{selectedItem.itemCount ?? "—"}</dd>
            </div>
            <div>
              <dt>本地状态</dt>
              <dd className="fy-memory-local-state">
                {localStateLabels[currentLocalState]}
              </dd>
            </div>
            {category === "longTerm" ? (
              <div>
                <dt>前端修订</dt>
                <dd>r{selectedItem.revision}</dd>
              </div>
            ) : null}
            <div>
              <dt>更新</dt>
              <dd>{selectedItem.updatedAt}</dd>
            </div>
          </dl>

          {selectedItem.provenance ? (
            <section
              className="fy-memory-provenance"
              aria-label="提炼来源"
              data-testid="memory-provenance"
            >
              <h3>提炼自</h3>
              <dl>
                <div>
                  <dt>来源条目</dt>
                  <dd className="fy-memory-provenance-reference">
                    <span>{provenanceSourceItem?.title ?? "未知来源条目"}</span>
                    <small>ID: {selectedItem.provenance.sourceItemId}</small>
                  </dd>
                </div>
                <div>
                  <dt>来源工具</dt>
                  <dd className="fy-memory-provenance-reference">
                    <span>{provenanceSourceTarget?.name ?? "未知工具"}</span>
                    <small>
                      toolId: {selectedItem.provenance.sourceToolId}
                    </small>
                  </dd>
                </div>
                <div>
                  <dt>来源目标</dt>
                  <dd className="fy-memory-provenance-reference">
                    <span>
                      {provenanceSourceTarget?.scopeLabel ?? "未知范围"}
                    </span>
                    <small>
                      targetId: {selectedItem.provenance.sourceTargetId}
                    </small>
                  </dd>
                </div>
                <div>
                  <dt>来源路径</dt>
                  <dd title={selectedItem.provenance.sourcePath}>
                    {selectedItem.provenance.sourcePath}
                  </dd>
                </div>
                <div>
                  <dt>来源更新</dt>
                  <dd>{selectedItem.provenance.sourceUpdatedAt}</dd>
                </div>
                <div>
                  <dt>来源摘要</dt>
                  <dd>{selectedItem.provenance.sourceSummary}</dd>
                </div>
                <div>
                  <dt>提炼时间</dt>
                  <dd>{selectedItem.provenance.capturedAt}</dd>
                </div>
              </dl>
            </section>
          ) : null}

          {category === "longTerm" &&
          selectedItem.owner === "memory" &&
          selectedItem.editableInPrototype ? (
            <>
              <div className="fy-memory-target-heading">
                <strong>同步目标</strong>
                <span>{draftTargetIds.length} 个</span>
              </div>
              <ul className="fy-memory-target-list" aria-label="同步目标">
                {syncTargets.map((target) => {
                  const isSelected = draftTargetIds.includes(target.id);
                  return (
                    <li key={target.id}>
                      <button
                        className="fy-memory-target-row"
                        type="button"
                        role="checkbox"
                        aria-checked={isSelected}
                        aria-label={`${isSelected ? "取消同步到" : "同步到"}${target.name}${target.scopeLabel}`}
                        onClick={() => toggleDraftTarget(target.id)}
                      >
                        <span>
                          <strong>{target.name}</strong>
                          <small>{target.memoryDestination}</small>
                        </span>
                        <i aria-hidden>{isSelected ? "✓" : ""}</i>
                      </button>
                    </li>
                  );
                })}
              </ul>
              {isDirty ? (
                <p className="fy-memory-sync-hint">
                  请先保存当前修改，再生成同步预览
                </p>
              ) : null}
              <button
                className="fy-memory-secondary-button fy-memory-sync-button"
                type="button"
                disabled={baseline === null || isDirty}
                onClick={previewLongTermSync}
              >
                生成 {draftTargetIds.length} 个同步预览任务
              </button>

              {selectedItem.previewTasks.length > 0 ? (
                <section
                  className="fy-memory-preview-tasks"
                  aria-label="待执行同步预览"
                  data-testid="memory-preview-tasks"
                >
                  <h3>待执行任务</h3>
                  <ul>
                    {selectedItem.previewTasks.map((task) => {
                      const target = agentTargetById(task.targetId);
                      return (
                        <li
                          key={task.targetId}
                          data-preview-state={task.previewState}
                          data-durable-state={task.durableState}
                        >
                          <span>
                            <strong>{target?.name ?? "未知目标"}</strong>
                            <small>{target?.scopeLabel ?? task.targetId}</small>
                          </span>
                          <span>
                            <em>待执行 · 未写入</em>
                            <small>基于修订 r{task.sourceRevision}</small>
                          </span>
                        </li>
                      );
                    })}
                  </ul>
                </section>
              ) : null}
            </>
          ) : null}

          {category === "longTerm" && selectedItem.owner === "prompts" ? (
            <div className="fy-memory-inspector-note">
              这组文件包含身份、工作规则和工具上下文。为避免重复写入，请在提示词页面管理。
            </div>
          ) : null}

          {category !== "longTerm" ? (
            <>
              <div className="fy-memory-inspector-note">
                {category === "sessions"
                  ? "会话来源保持只读；提炼时只复制可复用结论，不改动原始记录。"
                  : "每日记录本轮保持只读；提炼后再选择需要同步的长期记忆目标。"}
              </div>
              <button
                className="fy-memory-secondary-button"
                type="button"
                onClick={promoteToLongTerm}
              >
                提炼为长期记忆
              </button>
            </>
          ) : null}
        </aside>
      </div>
    </section>
  );
}
