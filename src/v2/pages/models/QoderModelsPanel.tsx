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
      <InlineNotice>
        不支持第三方模型配置。可在应用目录中管理 Hooks 和 MCP。
      </InlineNotice>
      <ModelsActionRow
        title="管理 Hooks 和 MCP"
        onClick={() => navigate("/agents?target=qoderwork")}
      />
    </ModelsGuidancePanel>
  );
}
