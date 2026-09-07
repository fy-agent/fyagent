import { InlineNotice } from "../../shared/ui/primitives";
import { ModelsGuidancePanel } from "./modelsShared";

export function QoderModelsPanel() {
  return (
    <ModelsGuidancePanel
      ariaLabel="QoderWork CN 模型设置"
      title="QoderWork CN"
      summary="官方不支持第三方模型配置"
    >
      <InlineNotice>官方不支持第三方模型配置</InlineNotice>
    </ModelsGuidancePanel>
  );
}
