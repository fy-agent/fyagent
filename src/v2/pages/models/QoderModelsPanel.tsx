import { useNavigate } from "react-router-dom";

import { InlineNotice } from "../../shared/ui/primitives";
import { ModelsActionRow, ModelsGuidancePanel } from "./modelsShared";

export function QoderModelsPanel() {
  const navigate = useNavigate();

  return (
    <ModelsGuidancePanel
      ariaLabel="QoderWork CN 模型设置"
      title="QoderWork CN"
      summary="不支持第三方模型配置"
    >
      <InlineNotice>不支持第三方模型配置。可在 MCP 页管理 MCP。</InlineNotice>
      <ModelsActionRow title="管理 MCP" onClick={() => navigate("/mcp")} />
    </ModelsGuidancePanel>
  );
}
