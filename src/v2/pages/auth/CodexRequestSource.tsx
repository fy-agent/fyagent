import { useCallback } from "react";
import { useLocation, useNavigate } from "react-router-dom";

import { ChangePlanWorkspace } from "../../shared/features/change-plans-ui/ChangePlanWorkspace";
import { useProviderSummary } from "../../shared/features/queries";
import {
  appendAgentReturnToPath,
  agentReturnDescriptorFromManagementSearch,
} from "../../shared/features/agent-navigation";
import { Button } from "../../shared/ui/Button";
import { InlineNotice, Spinner } from "../../shared/ui/primitives";

/** Account management owns source selection; Models still owns configuration editing. */
export function CodexRequestSource({
  active,
  disabled,
  onBusyChange,
  onRefreshOverview,
}: {
  active: boolean;
  disabled: boolean;
  onBusyChange: (busy: boolean) => void;
  onRefreshOverview: () => Promise<unknown>;
}) {
  const summary = useProviderSummary("codex", active);
  const refreshSummary = summary.refetch;
  const navigate = useNavigate();
  const { search } = useLocation();
  const reconcile = useCallback(async () => {
    await Promise.all([
      refreshSummary({ throwOnError: true }),
      onRefreshOverview(),
    ]);
  }, [refreshSummary, onRefreshOverview]);
  const openModels = () => {
    const descriptor = agentReturnDescriptorFromManagementSearch(search);
    const path = "/models?target=codex";
    navigate(descriptor ? appendAgentReturnToPath(path, descriptor) : path);
  };
  return (
    <>
      {summary.isPending ? <Spinner label="正在读取已保存配置" /> : null}
      {summary.isError ? (
        <InlineNotice tone="warning">
          无法读取 Codex 已保存配置。
          <Button onClick={() => void summary.refetch()}>重试读取配置</Button>
        </InlineNotice>
      ) : null}
      {summary.data ? (
        <ChangePlanWorkspace
          active={active}
          providers={summary.data.providers}
          currentId={summary.data.currentId}
          disabled={disabled || summary.isError || summary.isFetching}
          onBusyChange={onBusyChange}
          onTerminal={reconcile}
        />
      ) : null}
      <p className="fy-auth-source-help">
        需要添加服务地址、API Key 或修改模型参数？
      </p>
      <Button disabled={disabled} onClick={openModels}>
        编辑 Codex 模型配置
      </Button>
    </>
  );
}
