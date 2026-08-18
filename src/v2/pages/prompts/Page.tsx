import { MagnifyingGlassIcon } from "@phosphor-icons/react/dist/csr/MagnifyingGlass";
import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ChangeEvent,
  type FormEvent,
} from "react";
import { useBlocker, type BlockerFunction } from "react-router-dom";

import {
  agentTargets,
  countCoveredAgentInstances,
  groupPromptTargetsByCanonicalResource,
  type AgentTargetId,
} from "@/v2/shared/config/agentTargets";

import { promptPrototypeItems, type PromptPrototypeItem } from "./prototype";
import "./page.css";

function copyPrompt(item: PromptPrototypeItem): PromptPrototypeItem {
  return { ...item, targetIds: [...item.targetIds] };
}

function sameTargets(
  first: readonly AgentTargetId[],
  second: readonly AgentTargetId[],
): boolean {
  const firstSet = new Set(first);
  const secondSet = new Set(second);
  return (
    firstSet.size === secondSet.size &&
    [...firstSet].every((targetId) => secondSet.has(targetId))
  );
}

function samePrompt(
  first: PromptPrototypeItem,
  second: PromptPrototypeItem,
): boolean {
  return (
    first.name === second.name &&
    first.description === second.description &&
    first.content === second.content &&
    first.enabled === second.enabled &&
    sameTargets(first.targetIds, second.targetIds)
  );
}

interface PromptDraftState {
  value: PromptPrototypeItem;
  baseline: PromptPrototypeItem | null;
  hasSavedBaseline: boolean;
}

const discardMessage = "当前提示词尚未保存，确定放弃更改吗？";

export function PromptsPage() {
  const nextPrototypeId = useRef(promptPrototypeItems.length + 1);
  const [items, setItems] = useState<PromptPrototypeItem[]>(() =>
    promptPrototypeItems.map(copyPrompt),
  );
  const [selectedId, setSelectedId] = useState(promptPrototypeItems[0].id);
  const [query, setQuery] = useState("");
  const [draft, setDraft] = useState<PromptDraftState>(() => ({
    value: copyPrompt(promptPrototypeItems[0]),
    baseline: copyPrompt(promptPrototypeItems[0]),
    hasSavedBaseline: true,
  }));
  const [transientNewId, setTransientNewId] = useState<string | null>(null);
  const [feedback, setFeedback] = useState("");

  const filteredItems = useMemo(() => {
    const normalizedQuery = query.trim().toLocaleLowerCase();
    if (!normalizedQuery) {
      return items;
    }

    return items.filter((item) =>
      [
        item.name,
        item.description,
        item.content,
        item.category,
        item.origin,
      ].some((value) => value.toLocaleLowerCase().includes(normalizedQuery)),
    );
  }, [items, query]);

  const isDirty =
    !draft.hasSavedBaseline ||
    draft.baseline === null ||
    !samePrompt(draft.value, draft.baseline);
  const enabledCount = items.filter((item) => item.enabled).length;
  const canonicalTargetGroups = useMemo(
    () => groupPromptTargetsByCanonicalResource(draft.value.targetIds),
    [draft.value.targetIds],
  );

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

    if (window.confirm(discardMessage)) {
      blocker.proceed();
    } else {
      blocker.reset();
    }
  }, [blocker]);

  const confirmDiscard = (): boolean =>
    !isDirty || window.confirm(discardMessage);

  const removeTransientItem = () => {
    if (transientNewId === null) {
      return;
    }

    setItems((currentItems) =>
      currentItems.filter((item) => item.id !== transientNewId),
    );
    setTransientNewId(null);
  };

  const loadSavedPrompt = (item: PromptPrototypeItem) => {
    setSelectedId(item.id);
    setDraft({
      value: copyPrompt(item),
      baseline: copyPrompt(item),
      hasSavedBaseline: true,
    });
  };

  const selectPrompt = (id: string) => {
    const item = items.find((candidate) => candidate.id === id);
    if (!item || id === selectedId || !confirmDiscard()) {
      return;
    }

    removeTransientItem();
    loadSavedPrompt(item);
    setFeedback("");
  };

  const togglePrompt = (id: string) => {
    const target = items.find((item) => item.id === id);
    if (!target) {
      return;
    }

    if (id === transientNewId) {
      setFeedback("请先保存当前提示词，再启用");
      return;
    }

    const nextEnabled = !target.enabled;
    if (nextEnabled && target.targetIds.length === 0) {
      setFeedback("请先选择并保存至少一个注入目标");
      return;
    }

    setItems((currentItems) =>
      currentItems.map((item) =>
        item.id === id ? copyPrompt({ ...item, enabled: nextEnabled }) : item,
      ),
    );
    if (id === selectedId) {
      setDraft((currentDraft) => ({
        ...currentDraft,
        value: { ...currentDraft.value, enabled: nextEnabled },
        baseline:
          currentDraft.baseline === null
            ? null
            : { ...currentDraft.baseline, enabled: nextEnabled },
      }));
    }
    setFeedback(nextEnabled ? "已加入前端组合" : "已移出前端组合");
  };

  const createPrompt = () => {
    if (!confirmDiscard()) {
      return;
    }

    const id = `prototype-prompt-${nextPrototypeId.current}`;
    nextPrototypeId.current += 1;

    const nextItem: PromptPrototypeItem = {
      id,
      name: "未命名提示词",
      description: "",
      content: "",
      enabled: false,
      kind: "custom",
      category: "自定义",
      origin: "用户创建",
      targetIds: [],
      updatedAt: "—",
    };

    setItems((currentItems) => [
      nextItem,
      ...currentItems.filter((item) => item.id !== transientNewId),
    ]);
    setSelectedId(id);
    setDraft({
      value: copyPrompt(nextItem),
      baseline: null,
      hasSavedBaseline: false,
    });
    setTransientNewId(id);
    setQuery("");
    setFeedback("先填写内容并选择注入目标");
  };

  const updateDraft =
    (field: "name" | "description" | "content") =>
    (event: ChangeEvent<HTMLInputElement | HTMLTextAreaElement>) => {
      setDraft((currentDraft) => ({
        ...currentDraft,
        value: { ...currentDraft.value, [field]: event.target.value },
      }));
      setFeedback("");
    };

  const toggleDraftTarget = (targetId: AgentTargetId) => {
    const isSelected = draft.value.targetIds.includes(targetId);
    if (
      isSelected &&
      draft.value.enabled &&
      draft.value.targetIds.length === 1
    ) {
      setFeedback("已启用规则至少保留一个目标；请先停用再清空范围");
      return;
    }

    setDraft((currentDraft) => ({
      ...currentDraft,
      value: {
        ...currentDraft.value,
        targetIds: isSelected
          ? currentDraft.value.targetIds.filter((id) => id !== targetId)
          : [...currentDraft.value.targetIds, targetId],
      },
    }));
    setFeedback("");
  };

  const savePrompt = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const normalizedName = draft.value.name.trim();
    if (!normalizedName) {
      return;
    }
    if (draft.value.enabled && draft.value.targetIds.length === 0) {
      setFeedback("已启用规则必须选择至少一个注入目标");
      return;
    }

    const nextDraft: PromptPrototypeItem = {
      ...draft.value,
      name: normalizedName,
      targetIds: [...draft.value.targetIds],
      updatedAt: "刚刚",
    };
    setDraft({
      value: copyPrompt(nextDraft),
      baseline: copyPrompt(nextDraft),
      hasSavedBaseline: true,
    });
    setItems((currentItems) =>
      currentItems.map((item) =>
        item.id === selectedId ? copyPrompt(nextDraft) : item,
      ),
    );
    setTransientNewId(null);
    setFeedback("已保存到前端预览；未写入本机文件");
  };

  return (
    <section
      className="fy-prompts-page"
      aria-labelledby="fy-prompts-title"
      data-testid="prompts-page"
      data-data-source="prototype"
    >
      <header className="fy-prompts-header">
        <div className="fy-prompts-title-group">
          <h1 id="fy-prompts-title">提示词</h1>
          <div className="fy-prompts-context-row">
            <p>在前端预览中组合长期规则</p>
            <span className="fy-prompts-prototype-status">
              前端原型 · 未读取或写入本机文件
            </span>
          </div>
        </div>
        <button
          className="fy-prompts-primary-action"
          type="button"
          onClick={createPrompt}
        >
          新建提示词
        </button>
      </header>

      <div className="fy-prompts-grid">
        <section
          className="fy-prompt-pane fy-prompt-library"
          aria-labelledby="fy-prompt-library-title"
          data-testid="prompt-library"
        >
          <div className="fy-prompt-library-heading">
            <h2 id="fy-prompt-library-title">提示词库</h2>
            <span>{enabledCount} 条已启用</span>
          </div>

          <label className="fy-prompt-search">
            <MagnifyingGlassIcon size={24} weight="regular" aria-hidden />
            <span className="fy-visually-hidden">搜索提示词</span>
            <input
              type="search"
              value={query}
              placeholder="搜索提示词"
              onChange={(event) => setQuery(event.target.value)}
            />
          </label>

          <ul className="fy-prompt-list" aria-label="提示词列表">
            {filteredItems.map((item) => (
              <li
                key={item.id}
                className="fy-prompt-list-row"
                data-selected={item.id === selectedId ? "true" : "false"}
              >
                <button
                  className="fy-prompt-row-main"
                  type="button"
                  aria-pressed={item.id === selectedId}
                  onClick={() => selectPrompt(item.id)}
                >
                  <span className="fy-prompt-radio" aria-hidden />
                  <span className="fy-prompt-row-copy">
                    <span className="fy-prompt-row-title">
                      <strong>{item.name}</strong>
                      <em>{item.category}</em>
                    </span>
                    <span>
                      {item.description || "暂无描述"} · {item.targetIds.length}
                      个目标
                    </span>
                  </span>
                </button>
                <button
                  className="fy-prompt-switch"
                  type="button"
                  role="switch"
                  aria-checked={item.enabled}
                  aria-label={(item.enabled ? "停用" : "启用") + item.name}
                  onClick={() => togglePrompt(item.id)}
                >
                  <span aria-hidden />
                </button>
              </li>
            ))}
          </ul>

          {filteredItems.length === 0 ? (
            <p className="fy-prompt-no-results">没有匹配的提示词</p>
          ) : null}
        </section>

        <section
          className="fy-prompt-pane fy-prompt-editor"
          aria-labelledby="fy-prompt-editor-title"
          data-testid="prompt-editor"
        >
          <h2 id="fy-prompt-editor-title">
            {draft.value.name.trim() || "未命名提示词"}
          </h2>

          <form className="fy-prompt-editor-form" onSubmit={savePrompt}>
            <label className="fy-prompt-field">
              <span>名称</span>
              <input
                type="text"
                value={draft.value.name}
                aria-label="名称"
                onChange={updateDraft("name")}
              />
            </label>

            <label className="fy-prompt-field">
              <span>描述</span>
              <input
                type="text"
                value={draft.value.description}
                aria-label="描述"
                onChange={updateDraft("description")}
              />
            </label>

            <label className="fy-prompt-field fy-prompt-editor-area">
              <span>内容</span>
              <textarea
                value={draft.value.content}
                aria-label="内容"
                spellCheck={false}
                onChange={updateDraft("content")}
              />
            </label>

            <div className="fy-prompt-editor-footer">
              <span className="fy-prompt-save-message" aria-live="polite">
                {feedback}
              </span>
              <button
                className="fy-prompt-save-button"
                type="submit"
                disabled={!draft.value.name.trim()}
              >
                保存
              </button>
            </div>
          </form>
        </section>

        <aside
          className="fy-prompt-pane fy-prompt-inspector"
          aria-labelledby="fy-prompt-inspector-title"
          data-testid="prompt-inspector"
        >
          <h2 id="fy-prompt-inspector-title">注入目标</h2>

          <div className="fy-prompt-scope-summary">
            <span>当前组合范围</span>
            <strong>{canonicalTargetGroups.length} 个目标文件</strong>
            <em data-enabled={draft.value.enabled ? "true" : "false"}>
              {countCoveredAgentInstances(draft.value.targetIds)} 个 Agent
            </em>
          </div>

          <ul className="fy-prompt-target-list" aria-label="注入目标">
            {agentTargets.map((target) => {
              const isSelected = draft.value.targetIds.includes(target.id);
              return (
                <li key={target.id}>
                  <button
                    className="fy-prompt-target-row"
                    type="button"
                    role="checkbox"
                    aria-checked={isSelected}
                    aria-label={`${isSelected ? "取消注入到" : "注入到"}${target.name}${target.scopeLabel}`}
                    onClick={() => toggleDraftTarget(target.id)}
                  >
                    <span>
                      <strong>
                        {target.name}
                        <em>{target.scopeLabel}</em>
                      </strong>
                      <small>{target.promptPath}</small>
                    </span>
                    <span className="fy-prompt-target-state">
                      <small>
                        {target.promptPathState === "exists"
                          ? "已存在"
                          : "启用时创建"}
                      </small>
                      <i aria-hidden>{isSelected ? "✓" : ""}</i>
                    </span>
                  </button>
                </li>
              );
            })}
          </ul>

          <dl className="fy-prompt-details">
            <div>
              <dt>来源</dt>
              <dd>{draft.value.origin}</dd>
            </div>
            <div>
              <dt>类型</dt>
              <dd>{draft.value.category}</dd>
            </div>
            <div>
              <dt>更新时间</dt>
              <dd>{draft.value.updatedAt}</dd>
            </div>
          </dl>

          <div className="fy-prompt-info-note">
            <span className="fy-prompt-info-icon" aria-hidden>
              i
            </span>
            <span>
              接入真实同步后，同一路径只执行一次，并保护托管区块外内容
            </span>
          </div>
        </aside>
      </div>
    </section>
  );
}
