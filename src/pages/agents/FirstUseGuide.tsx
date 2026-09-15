import { useEffect, useRef, useState } from "react";
import { useQueryClient } from "@tanstack/react-query";

import { getAgentBrand } from "../../shared/assets/agents";
import { useFeatures } from "../../shared/features/provider";
import { featureKeys } from "../../shared/features/queries";
import type { AgentCatalogEntry } from "../../shared/features/types";
import { useFrontendReady } from "../../shared/platform/useFrontendReady";
import { Button, PressableButton } from "../../shared/ui/Button";
import { BrandIconFrame } from "../../shared/ui/catalog";
import { InlineNotice } from "../../shared/ui/primitives";
import { usePersistentVisibility } from "../../shared/ui/PersistentSurface";
import {
  firstUseRecommendations,
  GUIDE_PURPOSES,
  type GuidePurpose,
} from "./firstUseRecommendations";
import "./FirstUseGuide.css";

type DismissalState = "idle" | "pending" | "error";

export function FirstUseGuide({
  entries,
}: {
  entries: readonly AgentCatalogEntry[];
}) {
  const { ports } = useFeatures();
  const queryClient = useQueryClient();
  const visible = usePersistentVisibility();
  useFrontendReady();
  const [purpose, setPurpose] = useState<GuidePurpose | null>(null);
  const headingRef = useRef<HTMLHeadingElement>(null);
  const dismissalRef = useRef<DismissalState>("idle");
  const [dismissal, setDismissal] = useState<DismissalState>("idle");
  const isPending = dismissal === "pending";
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

  const publishDismissal = (state: typeof dismissal) => {
    dismissalRef.current = state;
    if (mounted.current && visibleRef.current) setDismissal(state);
  };

  const dismiss = async () => {
    if (
      !mounted.current ||
      !visibleRef.current ||
      dismissalRef.current === "pending"
    )
      return;
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
          {recommendations.map(({ entry, reason }) => (
            <article
              key={entry.id}
              className="fy-first-use-recommendation"
              data-agent-id={entry.id}
            >
              <BrandIconFrame asset={getAgentBrand(entry.id)} size="detail" />
              <div>
                <h2>{entry.displayName}</h2>
                <p>{reason}</p>
              </div>
            </article>
          ))}
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
        ) : null}
        <Button
          className="fy-first-use-skip"
          disabled={isPending || !visible}
          onClick={() => void dismiss()}
        >
          跳过引导
        </Button>
      </footer>
    </section>
  );
}
