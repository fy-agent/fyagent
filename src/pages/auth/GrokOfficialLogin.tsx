import { useRef, useState } from "react";

import { useFeatures } from "../../shared/features/provider";
import { useAgentAuthObservation } from "../../shared/features/queries";
import { useAgentAuthSession } from "../../shared/features/useAgentAuthSession";
import { Button } from "../../shared/ui/Button";
import { Dialog } from "../../shared/ui/Dialog";
import { usePersistentVisibility } from "../../shared/ui/PersistentSurface";
import { InlineNotice, Spinner } from "../../shared/ui/primitives";

export function GrokOfficialLogin() {
  const enabled = usePersistentVisibility();
  const { ports } = useFeatures();
  const observation = useAgentAuthObservation("grokbuild", enabled);
  const [confirmLogout, setConfirmLogout] = useState(false);
  const logoutOrigin = useRef<HTMLButtonElement>(null);
  const cancelRef = useRef<HTMLButtonElement>(null);
  const session = useAgentAuthSession({
    agentId: "grokbuild",
    port: ports.agentAuth,
    enabled,
    onTerminal: () => {
      void observation.refetch();
    },
  });
  const allowed = observation.isError
    ? []
    : (observation.data?.allowedIntents ?? []);
  const snapshot = session.snapshot;
  const disabled = !enabled || session.busy || observation.isFetching;
  const refresh = () => {
    void observation.refetch();
    session.retryRecovery();
  };

  return (
    <section className="fy-auth-section" aria-labelledby="grok-official-login">
      <div className="fy-auth-section-heading">
        <div>
          <h3 id="grok-official-login">Grok 官方 CLI 登录</h3>
          <p>
            在官方终端入口完成登录或退出。FyAgent 暂时无法确认 CLI
            的登录、过期或退出结果。
          </p>
        </div>
      </div>
      <p>
        登录过期时，请重新打开官方登录。也可在终端运行 <code>grok login</code>
        ；退出使用 <code>grok logout</code>。
      </p>
      <p>
        xAI 设备码账号保存在 FyAgent，用于支持的软件连接。API Key
        请在模型配置中管理。
      </p>
      {observation.isPending || session.recovering ? (
        <Spinner label="正在检查官方登录入口" />
      ) : null}
      {observation.isError || observation.data?.kind === "unavailable" ? (
        <InlineNotice tone="warning">
          暂时无法打开官方 CLI 入口。请检查 Grok CLI
          后重新检查，或在官方终端中手动继续。
        </InlineNotice>
      ) : null}
      {session.error ? (
        <InlineNotice tone="warning">
          未能确认认证操作状态。请重新检查后再试；已打开的官方终端可能仍在运行。
        </InlineNotice>
      ) : null}
      {snapshot ? (
        <InlineNotice
          tone={snapshot.stage === "handoff_complete" ? "info" : "warning"}
        >
          <span role="status">
            {snapshot.stage === "handoff_complete"
              ? snapshot.intent === "logout"
                ? "已向官方 CLI 发出退出操作，请在官方终端确认结果。"
                : "已打开官方 CLI 登录入口，请在终端完成登录。"
              : snapshot.stage === "failed"
                ? "官方 CLI 操作未完成，请重新检查后重试。"
                : snapshot.stage === "cancelled" ||
                    snapshot.stage === "timed_out"
                  ? "已停止等待。官方终端操作可能仍在继续，请手动确认结果。"
                  : "正在交给官方 CLI 处理。"}
          </span>
        </InlineNotice>
      ) : null}
      <div className="fy-feature-actions">
        {allowed.includes("login") ? (
          <Button
            disabled={disabled}
            onClick={() =>
              void session.start({ agentId: "grokbuild", intent: "login" })
            }
          >
            打开官方 CLI 登录
          </Button>
        ) : null}
        {allowed.includes("logout") ? (
          <Button
            ref={logoutOrigin}
            disabled={disabled}
            onClick={() => setConfirmLogout(true)}
          >
            退出官方 CLI 登录
          </Button>
        ) : null}
        {snapshot?.canStopWaiting ? (
          <Button
            disabled={session.submitting}
            onClick={() => void session.stopWaiting()}
          >
            停止等待
          </Button>
        ) : null}
        <Button
          disabled={
            session.submitting || session.recovering || observation.isFetching
          }
          onClick={refresh}
        >
          重新检查官方入口
        </Button>
      </div>
      <Dialog
        open={confirmLogout && enabled}
        originRef={logoutOrigin}
        initialFocusRef={cancelRef}
        onOpenChange={setConfirmLogout}
        title="退出 Grok 官方 CLI 登录"
        description="将调用官方 CLI 的退出操作。之后使用 CLI 可能需要重新登录；FyAgent 中保存的 xAI 设备码账号和 API Key 配置不会被移除。"
        actions={
          <>
            <Button ref={cancelRef} onClick={() => setConfirmLogout(false)}>
              取消
            </Button>
            <Button
              className="fy-control-button-danger"
              disabled={disabled || !allowed.includes("logout")}
              onClick={() => {
                setConfirmLogout(false);
                void session.start({ agentId: "grokbuild", intent: "logout" });
              }}
            >
              确认退出官方 CLI
            </Button>
          </>
        }
      />
    </section>
  );
}
