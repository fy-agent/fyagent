import { useEffect, useRef, useState } from "react";

import type {
  ManagedAuthAccountSummary,
  ManagedAuthConsumer,
  ManagedAuthLoginMethod,
  ManagedAuthProvider,
  ManagedAuthProviderSummary,
} from "../../shared/features/managed-auth";
import { useFeatures, useOpenExternal } from "../../shared/features/provider";
import {
  Button,
  Dialog,
  InlineNotice,
  Spinner,
} from "../../shared/ui/primitives";
import { ProviderMark, StatusBadge } from "./common";
import {
  loginStagePresentation,
  managedAuthConsumerLabel,
  managedAuthProviderLabel,
  managedAuthReasonCopy,
} from "./presentation";
import type { ManagedAuthLoginController } from "./useManagedAuthLoginSession";

const providerDescriptions: Record<ManagedAuthProvider, string> = {
  openai: "用于 Codex、FyAgent Local Proxy 或 OpenCode Desktop",
  xai: "用于 Grok Build、FyAgent Local Proxy 或 OpenCode Desktop",
  github_copilot: "用于支持 GitHub Copilot 的 Provider",
};

function preferredMethod(
  provider: ManagedAuthProvider,
  summary: ManagedAuthProviderSummary | undefined,
): ManagedAuthLoginMethod {
  return provider === "openai" &&
    summary?.loginMethods.includes("browser_loopback")
    ? "browser_loopback"
    : "device_code";
}

interface LoginDialogProps {
  open: boolean;
  providers: ManagedAuthProviderSummary[];
  initialConsumer: ManagedAuthConsumer | null;
  reauthenticateAccount: ManagedAuthAccountSummary | null;
  controller: ManagedAuthLoginController;
  onOpenChange: (open: boolean) => void;
  onFinished: () => void;
}

export function LoginDialog(props: LoginDialogProps) {
  const key = `${props.reauthenticateAccount?.accountId ?? "new"}:${props.initialConsumer ?? "none"}`;
  return <LoginDialogContent key={key} {...props} />;
}

function LoginDialogContent({
  open,
  providers,
  initialConsumer,
  reauthenticateAccount,
  controller,
  onOpenChange,
  onFinished,
}: LoginDialogProps) {
  const { notify } = useFeatures();
  const { openExternal, openingUrl } = useOpenExternal();
  const initialProvider =
    reauthenticateAccount?.provider ??
    providers.find(
      (item) =>
        item.available &&
        (initialConsumer === null || item.consumers.includes(initialConsumer)),
    )?.provider ??
    null;
  const [step, setStep] = useState<1 | 2 | 3>(
    reauthenticateAccount || initialConsumer ? 2 : 1,
  );
  const [provider, setProvider] = useState<ManagedAuthProvider | null>(
    initialProvider,
  );
  const [consumer, setConsumer] = useState<ManagedAuthConsumer | null>(
    initialConsumer,
  );
  const [saveOnly, setSaveOnly] = useState(
    reauthenticateAccount ? false : initialConsumer === null,
  );
  const [method, setMethod] = useState<ManagedAuthLoginMethod>(() =>
    initialProvider
      ? preferredMethod(
          initialProvider,
          providers.find((item) => item.provider === initialProvider),
        )
      : "device_code",
  );
  const [copied, setCopied] = useState(false);
  const wasOpen = useRef(open);

  useEffect(() => {
    const justOpened = open && !wasOpen.current;
    wasOpen.current = open;
    if (!justOpened || controller.snapshot) return;
    setStep(reauthenticateAccount || initialConsumer ? 2 : 1);
    setCopied(false);
  }, [controller.snapshot, initialConsumer, open, reauthenticateAccount]);

  useEffect(() => {
    if (!copied) return;
    const timer = window.setTimeout(() => setCopied(false), 1_600);
    return () => window.clearTimeout(timer);
  }, [copied]);

  const selectedProvider = providers.find((item) => item.provider === provider);
  const availableConsumers = selectedProvider?.consumers ?? [];
  const purpose = reauthenticateAccount
    ? "reauthenticate"
    : saveOnly
      ? "save_only"
      : "connect_consumer";
  const canContinueFromUse =
    provider !== null &&
    selectedProvider?.available === true &&
    (purpose !== "connect_consumer" || consumer !== null);

  const selectProvider = (next: ManagedAuthProvider) => {
    const summary = providers.find((item) => item.provider === next);
    setProvider(next);
    setMethod(preferredMethod(next, summary));
    if (consumer !== null && !summary?.consumers.includes(consumer)) {
      setConsumer(null);
      setSaveOnly(true);
    }
  };

  const startLogin = async () => {
    if (!provider || !canContinueFromUse) return;
    await controller.start({
      provider,
      purpose,
      consumer: purpose === "connect_consumer" ? consumer : null,
      method,
      accountId: reauthenticateAccount?.accountId ?? null,
    });
  };

  const copyUserCode = async () => {
    const userCode = controller.snapshot?.userCode;
    if (!userCode) return;
    try {
      await navigator.clipboard.writeText(userCode);
      setCopied(true);
    } catch {
      notify({ tone: "error", title: "无法复制设备码" });
    }
  };

  const finish = () => {
    onFinished();
    controller.reset();
  };

  const session = controller.snapshot;
  const sessionPresentation = session
    ? loginStagePresentation(session.stage)
    : null;

  const actions = (() => {
    if (session) {
      if (session.terminal) {
        return (
          <>
            {session.canRetry ? (
              <Button
                disabled={controller.submitting}
                onClick={() => void controller.retry()}
              >
                重新登录
              </Button>
            ) : null}
            <Button autoFocus onClick={finish}>
              完成
            </Button>
          </>
        );
      }
      return (
        <>
          {session.canSwitchToDeviceCode ? (
            <Button
              disabled={controller.submitting}
              onClick={() => void controller.switchMethod("device_code")}
            >
              改用设备码
            </Button>
          ) : null}
          <Button
            disabled={controller.submitting}
            onClick={() => void controller.reopen()}
          >
            重新打开官方页面
          </Button>
          {session.canCancel ? (
            <Button
              className="fy-control-button-danger-subtle"
              disabled={controller.submitting}
              onClick={() => void controller.cancel()}
            >
              取消登录
            </Button>
          ) : null}
        </>
      );
    }
    if (step === 1) {
      return (
        <>
          <Button onClick={() => onOpenChange(false)}>取消</Button>
          <Button disabled={provider === null} onClick={() => setStep(2)}>
            下一步
          </Button>
        </>
      );
    }
    if (step === 2) {
      return (
        <>
          {!reauthenticateAccount ? (
            <Button onClick={() => setStep(1)}>上一步</Button>
          ) : (
            <Button onClick={() => onOpenChange(false)}>取消</Button>
          )}
          <Button disabled={!canContinueFromUse} onClick={() => setStep(3)}>
            下一步
          </Button>
        </>
      );
    }
    return (
      <>
        <Button onClick={() => setStep(2)}>上一步</Button>
        <Button
          disabled={controller.submitting}
          onClick={() => void startLogin()}
        >
          {controller.submitting ? "正在启动…" : "继续"}
        </Button>
      </>
    );
  })();

  return (
    <Dialog
      open={open}
      onOpenChange={onOpenChange}
      title={
        reauthenticateAccount
          ? `重新登录 ${reauthenticateAccount.login}`
          : "添加官方账号"
      }
      description={
        session
          ? "登录由官方服务完成；FyAgent 只保存完成连接所需的账号状态。"
          : "选择账号类型和这次登录的用途。"
      }
      actions={actions}
      large
    >
      {session && sessionPresentation ? (
        <div className="fy-auth-login-session" data-stage={session.stage}>
          <div className="fy-auth-login-stage-heading">
            {!session.terminal ? (
              <Spinner label={sessionPresentation.title} />
            ) : null}
            <div>
              <h3>{sessionPresentation.title}</h3>
              <p>{sessionPresentation.description}</p>
            </div>
          </div>
          <div className="fy-auth-login-summary">
            <StatusBadge
              label={managedAuthProviderLabel(session.provider)}
              tone={session.terminal ? "accent" : "neutral"}
            />
            {session.consumer ? (
              <span>将连接到 {managedAuthConsumerLabel(session.consumer)}</span>
            ) : (
              <span>仅保存账号</span>
            )}
          </div>
          {session.method === "device_code" && session.userCode ? (
            <section className="fy-auth-device-code" aria-label="设备码登录">
              <p>在 {session.officialHost} 的官方页面输入以下设备码：</p>
              <div>
                <code aria-label={`设备码 ${session.userCode}`}>
                  {session.userCode}
                </code>
                <Button
                  aria-label={copied ? "已复制设备码" : "复制设备码"}
                  onClick={() => void copyUserCode()}
                >
                  {copied ? "已复制" : "复制"}
                </Button>
              </div>
              <p>只使用你刚刚在 FyAgent 中请求的设备码。</p>
              {session.verificationUri ? (
                <Button
                  disabled={openingUrl === session.verificationUri}
                  onClick={() =>
                    void openExternal(session.verificationUri as string, {
                      errorTitle: "无法打开官方验证页",
                    })
                  }
                >
                  打开官方页面
                </Button>
              ) : null}
            </section>
          ) : null}
          {session.reasonCode ? (
            <InlineNotice
              tone={session.stage === "completed" ? "info" : "warning"}
            >
              {managedAuthReasonCopy(session.reasonCode)}
            </InlineNotice>
          ) : null}
          {controller.error ? (
            <InlineNotice tone="warning">{controller.error}</InlineNotice>
          ) : null}
        </div>
      ) : step === 1 ? (
        <div
          className="fy-auth-login-options"
          role="group"
          aria-label="账号类型"
        >
          {providers.map((item) => (
            <button
              key={item.provider}
              type="button"
              aria-pressed={provider === item.provider}
              disabled={!item.available}
              onClick={() => selectProvider(item.provider)}
            >
              <ProviderMark provider={item.provider} size="detail" />
              <span>
                <strong>{managedAuthProviderLabel(item.provider)}</strong>
                <small>{providerDescriptions[item.provider]}</small>
                {!item.available ? <small>当前不可用</small> : null}
              </span>
            </button>
          ))}
        </div>
      ) : step === 2 && provider ? (
        <div className="fy-auth-login-use">
          <div className="fy-auth-login-provider-summary">
            <ProviderMark provider={provider} />
            <div>
              <strong>{managedAuthProviderLabel(provider)}</strong>
              <span>{providerDescriptions[provider]}</span>
            </div>
          </div>
          {reauthenticateAccount ? (
            <InlineNotice>
              将更新此账号的登录凭据。使用独立登录的其他软件不受影响。
            </InlineNotice>
          ) : (
            <fieldset>
              <legend>这次登录用于</legend>
              <label>
                <input
                  type="radio"
                  name="managed-auth-purpose"
                  checked={saveOnly}
                  onChange={() => {
                    setSaveOnly(true);
                    setConsumer(null);
                  }}
                />
                仅保存账号
              </label>
              {availableConsumers.map((candidate) => (
                <label key={candidate}>
                  <input
                    type="radio"
                    name="managed-auth-purpose"
                    checked={!saveOnly && consumer === candidate}
                    onChange={() => {
                      setSaveOnly(false);
                      setConsumer(candidate);
                    }}
                  />
                  连接 {managedAuthConsumerLabel(candidate)}
                </label>
              ))}
            </fieldset>
          )}
          {provider === "openai" &&
          selectedProvider?.loginMethods.length === 2 ? (
            <fieldset>
              <legend>登录方式</legend>
              <label>
                <input
                  type="radio"
                  name="managed-auth-method"
                  checked={method === "browser_loopback"}
                  onChange={() => setMethod("browser_loopback")}
                />
                浏览器登录（推荐）
              </label>
              <label>
                <input
                  type="radio"
                  name="managed-auth-method"
                  checked={method === "device_code"}
                  onChange={() => setMethod("device_code")}
                />
                设备码登录
              </label>
            </fieldset>
          ) : null}
        </div>
      ) : provider ? (
        <div className="fy-auth-login-confirmation">
          <ProviderMark provider={provider} size="detail" />
          <div>
            <h3>即将打开 {managedAuthProviderLabel(provider)} 官方登录页</h3>
            <p>
              域名：
              {provider === "openai"
                ? "auth.openai.com / chatgpt.com"
                : provider === "xai"
                  ? "auth.x.ai"
                  : "github.com"}
            </p>
            <p>
              {method === "browser_loopback"
                ? "完成后浏览器会返回此设备。"
                : "浏览器会要求确认由 FyAgent 生成的设备码。"}
            </p>
          </div>
        </div>
      ) : null}
    </Dialog>
  );
}
