import { useCallback, useEffect, useRef, useState } from "react";
import { useQueryClient } from "@tanstack/react-query";

import { getAgentBrand } from "../../shared/assets/agents";
import { useFeatures } from "../../shared/features/provider";
import { featureKeys } from "../../shared/features/queries";
import type { FeaturePorts } from "../../shared/features/ports";
import type {
  AgentCatalogEntry,
  AgentCatalogId,
} from "../../shared/features/types";
import { useFrontendReady } from "../../shared/platform/useFrontendReady";
import { Button, PressableButton } from "../../shared/ui/Button";
import { BrandIconFrame } from "../../shared/ui/catalog";
import { Dialog } from "../../shared/ui/Dialog";
import { InlineNotice } from "../../shared/ui/primitives";
import { usePersistentVisibility } from "../../shared/ui/PersistentSurface";
import { usePersistentSearchParams } from "../../shared/ui/usePersistentSearchParams";
import {
  firstUseRecommendations,
  GUIDE_PURPOSES,
  type GuidePurpose,
  startingPresetForAgent,
} from "./firstUseRecommendations";
import "./FirstUseGuide.css";

export type AgentConfigIntent = "keep" | "replace" | "inspect";

export interface AgentConfigureRequest {
  agentId: AgentCatalogId;
  intent: AgentConfigIntent;
  existingSummary?: string;
}

export type FirstUseGuideConfigureHandler = (
  request: AgentConfigureRequest,
) => void;

export interface FirstUseGuideProps {
  entries: readonly AgentCatalogEntry[];
  onConfigure?: FirstUseGuideConfigureHandler;
}

type DismissalState = "idle" | "pending" | "error";

export type AgentConfigInspection =
  | { status: "configured"; summary: string }
  | { status: "empty" }
  | { status: "unknown"; message?: string };

interface ExistingConfigState {
  entry: AgentCatalogEntry;
  status: "configured" | "unknown";
  summary?: string;
  message?: string;
}

async function checkExistingAgentConfiguration(
  ports: FeaturePorts,
  agentId: AgentCatalogId,
): Promise<AgentConfigInspection> {
  try {
    if (
      agentId === "codex" ||
      agentId === "claude-code" ||
      agentId === "grokbuild"
    ) {
      const app = agentId === "claude-code" ? "claude" : agentId;
      const summary = await ports.providers.getSummary(app);
      const live = summary.live;
      if (!live || live.target !== app || live.state === "unreadable") {
        return {
          status: "unknown",
          message: "无法读取实际配置文件，请重试或进入软件详情查看。",
        };
      }
      if (live.state === "configured") {
        const details = [
          live.connection.modelId ? `模型：${live.connection.modelId}` : null,
          live.connection.baseUrl
            ? `服务地址：${live.connection.baseUrl}`
            : null,
        ].filter(Boolean);
        return {
          status: "configured",
          summary: details.join("；") || "实际配置文件中已有连接设置",
        };
      }
    } else if (agentId === "workbuddy") {
      const modelIds = await ports.workbuddy.getModelIds();
      if (modelIds?.ids && modelIds.ids.length > 0) {
        return {
          status: "configured",
          summary: `实际配置文件：已配置 ${modelIds.ids.length} 个模型`,
        };
      }
    } else if (agentId === "opencode") {
      const snapshot = await ports.opencodeModels.getSnapshot();
      if (snapshot?.exists && snapshot.providers?.length > 0) {
        return {
          status: "configured",
          summary: `实际配置文件：已配置 ${snapshot.providers.length} 个模型提供商`,
        };
      }
    }
    return { status: "empty" };
  } catch {
    return {
      status: "unknown",
      message: "读取当前配置失败，请重试或进入软件详情查看。",
    };
  }
}

export function FirstUseGuide({ entries, onConfigure }: FirstUseGuideProps) {
  const { ports } = useFeatures();
  const queryClient = useQueryClient();
  const visible = usePersistentVisibility();
  const { setSearchParams } = usePersistentSearchParams();
  useFrontendReady();
  const [purpose, setPurpose] = useState<GuidePurpose | null>(null);
  const [checkingAgentId, setCheckingAgentId] = useState<AgentCatalogId | null>(
    null,
  );
  const [existingConfigState, setExistingConfigState] =
    useState<ExistingConfigState | null>(null);
  const selectingRef = useRef(false);
  const originRef = useRef<HTMLElement | null>(null);
  const headingRef = useRef<HTMLHeadingElement>(null);
  const dismissalRef = useRef<DismissalState>("idle");
  const [dismissal, setDismissal] = useState<DismissalState>("idle");
  const isPending = dismissal === "pending" || checkingAgentId !== null;
  const mounted = useRef(true);
  const visibleRef = useRef(visible);
  const recommendations = purpose
    ? firstUseRecommendations(entries, purpose)
    : [];

  useEffect(() => {
    if (visible) headingRef.current?.focus({ preventScroll: true });
  }, [purpose, visible]);

  useEffect(() => {
    mounted.current = true;
    return () => {
      mounted.current = false;
    };
  }, []);

  useEffect(() => {
    visibleRef.current = visible;
    if (visible) setDismissal(dismissalRef.current);
  }, [visible]);

  const publishDismissal = (nextState: DismissalState) => {
    dismissalRef.current = nextState;
    if (mounted.current && visibleRef.current) setDismissal(nextState);
  };

  const dismiss = async () => {
    if (
      !mounted.current ||
      !visibleRef.current ||
      dismissalRef.current === "pending"
    )
      return;
    setExistingConfigState(null);
    publishDismissal("pending");
    try {
      await queryClient.cancelQueries({ queryKey: featureKeys.firstUseGuide });
      const state = await ports.settings.dismissFirstUseGuide();
      // Native persistence remains authoritative even if this route is hidden.
      queryClient.setQueryData(featureKeys.firstUseGuide, state);
      publishDismissal("idle");
    } catch {
      // Reconcile a hidden failure on return without updating a hidden page.
      publishDismissal("error");
    }
  };

  const enterAgent = useCallback(
    (
      agentId: AgentCatalogId,
      intent: AgentConfigIntent,
      existingSummary?: string,
    ) => {
      if (
        !mounted.current ||
        !visibleRef.current ||
        (dismissalRef.current as DismissalState) === "pending"
      ) {
        return;
      }
      if (onConfigure) {
        onConfigure({
          agentId,
          intent,
          existingSummary,
        });
      } else {
        setSearchParams(
          intent === "inspect"
            ? { target: agentId }
            : { target: agentId, section: "models", intent },
        );
      }
    },
    [onConfigure, setSearchParams],
  );

  const handleSelectAgent = async (entry: AgentCatalogEntry) => {
    // Synchronous admission check against double-click or invalid lifecycle state
    if (
      selectingRef.current ||
      !mounted.current ||
      !visibleRef.current ||
      (dismissalRef.current as DismissalState) === "pending" ||
      checkingAgentId !== null
    ) {
      return;
    }
    selectingRef.current = true;
    setCheckingAgentId(entry.id);

    try {
      const result = await checkExistingAgentConfiguration(ports, entry.id);

      // Lifecycle check after asynchronous inspection
      if (
        !mounted.current ||
        !visibleRef.current ||
        (dismissalRef.current as DismissalState) === "pending"
      ) {
        return;
      }

      if (result.status === "configured") {
        setExistingConfigState({
          entry,
          status: "configured",
          summary: result.summary,
        });
        return;
      }

      if (result.status === "unknown") {
        setExistingConfigState({
          entry,
          status: "unknown",
          message: result.message,
        });
        return;
      }

      enterAgent(entry.id, "replace");
    } finally {
      selectingRef.current = false;
      if (mounted.current) setCheckingAgentId(null);
    }
  };

  return (
    <section
      className="fy-first-use-guide"
      aria-label="首次使用引导"
      aria-busy={isPending}
    >
      <h1 ref={headingRef} tabIndex={-1}>
        {purpose ? "推荐你从这些软件开始" : "你主要想用 AI 做什么？"}
      </h1>
      {purpose ? (
        <div className="fy-first-use-recommendations">
          {recommendations.map(({ entry, reason }) => {
            const preset = purpose
              ? startingPresetForAgent(entry.id, purpose)
              : undefined;
            return (
              <article
                key={entry.id}
                className="fy-first-use-recommendation"
                data-agent-id={entry.id}
              >
                <BrandIconFrame asset={getAgentBrand(entry.id)} size="detail" />
                <div className="fy-first-use-recommendation-content">
                  <h2>{entry.displayName}</h2>
                  <p className="fy-first-use-recommendation-reason">{reason}</p>
                  {preset ? (
                    <div className="fy-first-use-preset-hint">
                      <span className="fy-first-use-preset-category">
                        {preset.category}
                      </span>
                      <span className="fy-first-use-preset-name">
                        建议起始指令：{preset.name}
                      </span>
                    </div>
                  ) : null}
                </div>
                <div className="fy-first-use-recommendation-action">
                  <Button
                    className="fy-first-use-entry-btn"
                    disabled={isPending || !visible}
                    onClick={(event) => {
                      originRef.current = event.currentTarget;
                      void handleSelectAgent(entry);
                    }}
                  >
                    开始配置
                  </Button>
                </div>
              </article>
            );
          })}
          {recommendations.length === 0 ? (
            <p>可在软件目录中查看其他选择。</p>
          ) : null}
        </div>
      ) : (
        <div
          className="fy-first-use-choices"
          role="group"
          aria-label="使用用途"
        >
          {GUIDE_PURPOSES.map((option) => (
            <PressableButton
              key={option.id}
              className="fy-first-use-choice"
              aria-label={option.label}
              disabled={isPending || !visible}
              onClick={() => setPurpose(option.id)}
            >
              <strong>{option.label}</strong>
              <span>{option.description}</span>
            </PressableButton>
          ))}
        </div>
      )}
      {dismissal === "error" ? (
        <InlineNotice tone="error">暂时无法保存引导状态，请重试。</InlineNotice>
      ) : null}
      <footer className="fy-first-use-actions">
        {purpose ? (
          <div className="fy-first-use-completion">
            <Button
              disabled={isPending || !visible}
              onClick={() => setPurpose(null)}
            >
              重新选择
            </Button>
            <Button
              className="fy-control-button-primary"
              disabled={isPending || !visible}
              onClick={() => void dismiss()}
            >
              {isPending ? "正在保存…" : "查看全部软件"}
            </Button>
          </div>
        ) : (
          <Button
            className="fy-first-use-skip"
            disabled={isPending || !visible}
            onClick={() => void dismiss()}
          >
            跳过引导
          </Button>
        )}
      </footer>

      <Dialog
        open={existingConfigState !== null}
        onOpenChange={(open) => {
          if (!open) setExistingConfigState(null);
        }}
        title={
          existingConfigState?.status === "unknown"
            ? `无法确认 ${existingConfigState?.entry.displayName ?? ""} 配置状态`
            : `确认 ${existingConfigState?.entry.displayName ?? ""} 配置方式`
        }
        originRef={originRef}
        actions={
          existingConfigState ? (
            <div className="fy-first-use-dialog-actions">
              <Button onClick={() => setExistingConfigState(null)}>取消</Button>
              {existingConfigState.status === "unknown" ? (
                <>
                  <Button
                    onClick={() => {
                      const entry = existingConfigState.entry;
                      setExistingConfigState(null);
                      void handleSelectAgent(entry);
                    }}
                  >
                    重试
                  </Button>
                  <Button
                    className="fy-control-button-primary"
                    onClick={() => {
                      const state = existingConfigState;
                      setExistingConfigState(null);
                      enterAgent(state.entry.id, "inspect");
                    }}
                  >
                    进入软件详情查看
                  </Button>
                </>
              ) : (
                <>
                  <Button
                    onClick={() => {
                      const state = existingConfigState;
                      setExistingConfigState(null);
                      enterAgent(state.entry.id, "replace", state.summary);
                    }}
                  >
                    替换现有配置
                  </Button>
                  <Button
                    className="fy-control-button-primary"
                    onClick={() => {
                      const state = existingConfigState;
                      setExistingConfigState(null);
                      enterAgent(state.entry.id, "keep", state.summary);
                    }}
                  >
                    保留现有配置
                  </Button>
                </>
              )}
            </div>
          ) : undefined
        }
      >
        {existingConfigState ? (
          existingConfigState.status === "unknown" ? (
            <div className="fy-first-use-existing-config-content">
              <p>
                无法确认 {existingConfigState.entry.displayName}{" "}
                的现有配置状态。当前不确定是否存在可用配置。
              </p>
              {existingConfigState.message ? (
                <p className="fy-first-use-keep-notice">
                  原因：{existingConfigState.message}
                </p>
              ) : null}
            </div>
          ) : (
            <div className="fy-first-use-existing-config-content">
              <p>
                检测到 {existingConfigState.entry.displayName}{" "}
                的实际配置文件中已有设置
                {existingConfigState.summary
                  ? `（${existingConfigState.summary}）`
                  : ""}
                。你可以保留现有设置，或进入重新配置。尚未测试连接。
              </p>
              <p className="fy-first-use-keep-notice">
                保留现有配置不会发起模型网络请求。
              </p>
            </div>
          )
        ) : null}
      </Dialog>
    </section>
  );
}
