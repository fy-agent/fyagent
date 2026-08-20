import { useMemo, useState } from "react";

import { useTraeWorkModelIds } from "../../shared/features/queries";
import { InlineNotice, Spinner } from "../../shared/ui/primitives";
import { GroupedModelChips, ModelSearchField } from "./modelChips";
import { ModelsExistingSection, ModelsGuidancePanel } from "./modelsShared";
import { filterModelIds } from "./workBuddyModels";

const EMPTY_MODEL_IDS: readonly string[] = [];

export function TraeModelsPanel({ active }: { active: boolean }) {
  const modelIdsQuery = useTraeWorkModelIds(active);
  const [existingSearch, setExistingSearch] = useState("");
  const [existingOpen, setExistingOpen] = useState(false);

  const modelIds = modelIdsQuery.data?.modelIds ?? EMPTY_MODEL_IDS;
  const filteredExistingIds = useMemo(
    () => filterModelIds(modelIds, existingSearch),
    [modelIds, existingSearch],
  );
  const loading = modelIdsQuery.isLoading;
  const readFailed = modelIdsQuery.isError;

  return (
    <ModelsGuidancePanel
      ariaLabel="TRAE Work CN 模型设置"
      title="TRAE Work CN"
      summary="自定义模型需在 TRAE Work CN 中添加。FyAgent 不会写入其本地模型配置。"
    >
      <InlineNotice>
        TRAE Work CN
        以云端模型列表为准。写入本机缓存的自定义模型会在应用启动时被覆盖，因此无法在此保存或应用。
      </InlineNotice>

      {loading && <Spinner label="正在读取 TRAE 当前模型" />}
      {readFailed && (
        <InlineNotice tone="error">
          暂时无法读取 TRAE 当前模型，请重试。
        </InlineNotice>
      )}

      <ModelsExistingSection
        title="TRAE 当前第三方模型 ID"
        countLabel="当前可见数量"
        count={modelIds.length}
        open={existingOpen}
        onOpenChange={setExistingOpen}
        testId="trae-model-ids"
        ariaLabel="TRAE 当前第三方模型 ID"
      >
        {modelIds.length > 0 ? (
          <ModelSearchField
            id="trae-existing-search"
            label="搜索当前模型"
            value={existingSearch}
            onChange={setExistingSearch}
          />
        ) : null}
        <GroupedModelChips
          ids={filteredExistingIds}
          emptyLabel={
            existingSearch.trim()
              ? "没有匹配的模型 ID"
              : "未观察到第三方模型 ID。请在 TRAE Work CN 中添加。"
          }
        />
      </ModelsExistingSection>
    </ModelsGuidancePanel>
  );
}
