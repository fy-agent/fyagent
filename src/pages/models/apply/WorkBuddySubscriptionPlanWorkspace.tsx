import type { WorkBuddySubscriptionPlanRequest } from "../../../shared/features/models";
import { useFeatures } from "../../../shared/features/provider";
import {
  SavePlanWorkspace,
  type SavePlanWorkspaceProps,
} from "./SavePlanWorkspace";

export function WorkBuddySubscriptionPlanWorkspace(
  props: SavePlanWorkspaceProps<WorkBuddySubscriptionPlanRequest>,
) {
  const { ports } = useFeatures();
  return (
    <SavePlanWorkspace
      {...props}
      create={ports.changePlans.createWorkBuddySubscriptionPlan}
      label="保存 WorkBuddy 订阅设置"
      title="确认订阅设置"
      description="确认后将更新 WorkBuddy 的模型来源。同名模型的服务地址和凭据会被替换；其他模型保留。使用时需要 FyAgent 保持运行。"
    />
  );
}
