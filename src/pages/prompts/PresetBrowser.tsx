import { useMemo, useState, type MouseEvent } from "react";

import { Button } from "../../shared/ui/Button";
import type { DialogOriginRef } from "../../shared/ui/dialogOrigin";
import { FeatureList, FeatureListItem } from "../../shared/ui/FeatureList";
import { FeatureSearch } from "../../shared/ui/FeatureSearch";
import { EmptyState } from "../../shared/ui/primitives";
import { SplitPanes } from "../../shared/ui/split";
import {
  PROMPT_PRESET_CATEGORIES,
  searchPromptPresets,
  type PromptPreset,
  type PromptPresetCategory,
} from "./presets";

const SPLIT_LABELS = ["调整预设列表与预览的宽度"];

export function PresetBrowser({
  appLabel,
  canUse,
  originRef,
  onUse,
}: {
  appLabel: string;
  canUse: boolean;
  originRef: DialogOriginRef;
  onUse: (preset: PromptPreset, event: MouseEvent<HTMLButtonElement>) => void;
}) {
  const [search, setSearch] = useState("");
  const [category, setCategory] = useState<PromptPresetCategory | "all">("all");
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const items = useMemo(
    () => searchPromptPresets(search, category),
    [search, category],
  );
  const selected = items.find((item) => item.id === selectedId) ?? items[0];

  return (
    <div className="fy-prompts-preset-browser" aria-label="FDE 预设库">
      <div className="fy-feature-toolbar fy-prompts-search-toolbar">
        <FeatureSearch
          ariaLabel="搜索 FDE 预设"
          placeholder="搜索领域、系统或交付任务"
          value={search}
          onValueChange={setSearch}
        />
        <select
          className="fy-control-select"
          aria-label="预设领域"
          value={category}
          onChange={(event) => {
            const value = event.target.value;
            const match = PROMPT_PRESET_CATEGORIES.find(
              (item) => item.id === value,
            );
            if (value === "all" || match) setCategory(match?.id ?? "all");
          }}
        >
          <option value="all">全部领域</option>
          {PROMPT_PRESET_CATEGORIES.map((item) => (
            <option key={item.id} value={item.id}>
              {item.label}
            </option>
          ))}
        </select>
      </div>
      {selected ? (
        <div className="fy-prompts-split-wrapper">
          <SplitPanes
            className="fy-prompts-split"
            maxWidths={[320]}
            minWidths={[200, 380]}
            separatorLabels={SPLIT_LABELS}
          >
            <section
              className="fy-feature-panel fy-prompts-library-pane"
              aria-label="FDE 预设列表"
            >
              <div className="fy-prompts-library-head">
                <h2>领域预设 · {items.length}</h2>
              </div>
              <FeatureList
                id="prompt-presets-list"
                className="fy-prompts-library-list"
              >
                {items.map((item) => (
                  <FeatureListItem
                    key={item.id}
                    title={item.name}
                    ariaLabel={item.name}
                    selected={item.id === selected.id}
                    onSelect={() => setSelectedId(item.id)}
                  >
                    <span className="fy-prompts-card-desc">
                      {item.description}
                    </span>
                  </FeatureListItem>
                ))}
              </FeatureList>
            </section>
            <section
              className="fy-feature-panel fy-prompts-editor-pane"
              aria-label="FDE 预设预览"
            >
              <header className="fy-prompts-editor-head">
                <div className="fy-prompts-editor-header-info">
                  <div className="fy-prompts-editor-title-row">
                    <h2>{selected.name}</h2>
                  </div>
                  <p className="fy-prompts-editor-meta">
                    用于 {appLabel}，保存后可单独启用。
                  </p>
                </div>
                <Button
                  className="fy-control-button-primary"
                  disabled={!canUse}
                  dialogOriginRef={originRef}
                  onClick={(event) => onUse(selected, event)}
                >
                  使用此预设
                </Button>
              </header>
              <textarea
                className="fy-control-textarea fy-prompts-editor-content"
                aria-label="预设内容"
                readOnly
                spellCheck={false}
                value={selected.content}
              />
            </section>
          </SplitPanes>
        </div>
      ) : (
        <EmptyState
          title="没有匹配的 FDE 预设"
          actions={
            <Button
              onClick={() => {
                setSearch("");
                setCategory("all");
              }}
            >
              清空筛选
            </Button>
          }
        />
      )}
    </div>
  );
}
