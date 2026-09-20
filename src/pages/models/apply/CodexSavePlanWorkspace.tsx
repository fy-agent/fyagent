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
      description="确认后将保存并启用此配置。FyAgent 将 API Key 保存在本机系统凭据库；Codex 仍需在 config.toml 中读取明文 Key，备份也可能含有旧 Key。请勿分享这些文件。删除 FyAgent 中的配置不会撤销服务商处的 Key。"
    />
  );
}
