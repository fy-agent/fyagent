import { useEffect, useRef, useState } from "react";

import { Button } from "../../ui/Button";
import { useFeatures } from "../provider";
import type {
  BatchSetupTargetId,
  SavedModelProbeCategory,
  SavedModelProbeResult,
} from "../types";
import "./inline-model-probe.css";

const CATEGORY_SHORT_LABELS: Readonly<Record<SavedModelProbeCategory, string>> =
  {
    ok: "可用",
    invalid_key: "Key 无效",
    quota_exhausted: "额度不足",
    endpoint_or_model_mismatch: "模型/路径不匹配",
    network_unreachable: "网络异常",
  };

export interface InlineModelProbeButtonProps {
  target: BatchSetupTargetId;
  providerId?: string | null;
  modelId: string;
  disabled?: boolean;
}

interface ProbeViewState {
  scopeKey: string;
  phase: "idle" | "probing" | "done";
  result: SavedModelProbeResult | null;
  errorText: string | null;
}

export function InlineModelProbeButton({
  target,
  providerId,
  modelId,
  disabled = false,
}: InlineModelProbeButtonProps) {
  const { ports } = useFeatures();
  const scopeKey = `${target}::${providerId ?? ""}::${modelId}`;
  const [state, setState] = useState<ProbeViewState>({
    scopeKey,
    phase: "idle",
    result: null,
    errorText: null,
  });
  const mountedRef = useRef(true);
  const requestSeqRef = useRef(0);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
    };
  }, []);

  const current: ProbeViewState =
    state.scopeKey === scopeKey
      ? state
      : { scopeKey, phase: "idle", result: null, errorText: null };

  const handleProbe = async () => {
    if (disabled || current.phase === "probing") return;
    const seq = requestSeqRef.current + 1;
    requestSeqRef.current = seq;
    setState({
      scopeKey,
      phase: "probing",
      result: null,
      errorText: null,
    });
    try {
      const result = await ports.providers.probeSavedModel({
        target,
        ...(providerId ? { providerId } : {}),
        modelId,
      });
      if (!mountedRef.current || requestSeqRef.current !== seq) return;
      setState({
        scopeKey,
        phase: "done",
        result,
        errorText: null,
      });
    } catch (error) {
      if (!mountedRef.current || requestSeqRef.current !== seq) return;
      const message =
        error instanceof Error && error.message.trim()
          ? error.message.trim()
          : "无法读取已保存凭据或发起测试";
      setState({
        scopeKey,
        phase: "done",
        result: null,
        errorText: message,
      });
    }
  };

  return (
    <span className="fy-inline-probe" data-testid="inline-model-probe">
      <Button
        className="fy-inline-probe-trigger"
        disabled={disabled || current.phase === "probing"}
        aria-label={`测试模型 ${modelId} 连通性`}
        onClick={(event) => {
          event.stopPropagation();
          void handleProbe();
        }}
      >
        {current.phase === "probing" ? "测速中…" : "测速"}
      </Button>
      {current.phase === "done" && current.result ? (
        current.result.ok ? (
          <span
            className="fy-inline-probe-pill fy-inline-probe-ok"
            role="status"
            data-probe-category={current.result.category}
            title={current.result.message}
          >
            ✓{" "}
            {current.result.latencyMs !== null
              ? `${current.result.latencyMs}ms`
              : CATEGORY_SHORT_LABELS.ok}
          </span>
        ) : (
          <span
            className="fy-inline-probe-pill fy-inline-probe-error"
            role="status"
            data-probe-category={current.result.category}
            title={current.result.message}
          >
            {CATEGORY_SHORT_LABELS[current.result.category]}
            {current.result.httpStatus
              ? ` (${current.result.httpStatus})`
              : ""}{" "}
            · {current.result.message}
          </span>
        )
      ) : null}
      {current.phase === "done" && current.errorText ? (
        <span
          className="fy-inline-probe-pill fy-inline-probe-error"
          role="status"
          title={current.errorText}
        >
          测试失败 · {current.errorText}
        </span>
      ) : null}
    </span>
  );
}
