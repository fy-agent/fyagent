import type { ProviderQuickSetupRequest } from "../../../shared/features/models";
import { useFeatures } from "../../../shared/features/provider";
import {
  SavePlanWorkspace,
  type SavePlanWorkspaceProps,
} from "./SavePlanWorkspace";

export function CodexSavePlanWorkspace(
  props: SavePlanWorkspaceProps<ProviderQuickSetupRequest>,
) {
  const { ports } = useFeatures();
  return (
    <SavePlanWorkspace
      {...props}
      create={(request) =>
        ports.changePlans.createCodexProviderUpsertPlan(request)
      }
      label="保存 Codex Provider"
      title="保存并设为当前配置"
      description="请先检查更改内容。只有确认后才会保存并启用此 Provider。"
    />
  );
}
