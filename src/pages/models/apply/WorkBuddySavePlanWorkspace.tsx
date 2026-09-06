import type { WorkBuddySaveModelsRequest } from "../../../shared/features/models";
import { useFeatures } from "../../../shared/features/provider";
import {
  SavePlanWorkspace,
  type SavePlanWorkspaceProps,
} from "./SavePlanWorkspace";

export function WorkBuddySavePlanWorkspace(
  props: SavePlanWorkspaceProps<WorkBuddySaveModelsRequest>,
) {
  const { ports } = useFeatures();
  return (
    <SavePlanWorkspace
      {...props}
      create={(request) => ports.changePlans.createWorkBuddySavePlan(request)}
      label="保存 WorkBuddy 模型设置"
      title="保存并应用"
      description="请先检查更改内容。只有确认后才会更新 WorkBuddy 模型设置。"
    />
  );
}
