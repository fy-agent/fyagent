/* eslint-disable react-refresh/only-export-components */
import { CaretDownIcon } from "@phosphor-icons/react/dist/csr/CaretDown";
import { QuestionIcon } from "@phosphor-icons/react/dist/csr/Question";
import { useCallback, useRef, useState, type ReactNode } from "react";

import { classNames } from "../../shared/design-system/classNames";
import type { ModelWriteTarget } from "../../shared/features/types";
import { CatalogDetail } from "../../shared/ui/catalog";
import { CopyablePath } from "../../shared/features/controls/CopyablePath";
import {
  Badge,
  Button,
  Checkbox,
  Dialog,
  Tooltip,
} from "../../shared/ui/primitives";
import { FieldFeedback, type Notice } from "./feedback";
import type {
  ModelProbeResult,
  ReachabilityResult,
} from "../../shared/features/types";

export function NoticeView({ notice }: { notice: Notice | null }) {
  return <FieldFeedback notice={notice} />;
}

export function useModelsDraftCommit() {
  const draftRevisionRef = useRef(0);
  const [draftRevision, setDraftRevision] = useState(0);
  const [committedRevision, setCommittedRevision] = useState(0);

  const markDirty = useCallback(() => {
    draftRevisionRef.current += 1;
    setDraftRevision(draftRevisionRef.current);
  }, []);
  const captureRevision = useCallback(() => draftRevisionRef.current, []);
  const isCurrentRevision = useCallback(
    (revision: number) => draftRevisionRef.current === revision,
    [],
  );
  const commitRevision = useCallback((revision: number) => {
    setCommittedRevision(revision);
  }, []);

  return {
    pending: draftRevision !== committedRevision,
    resetVersion: `${draftRevision}:${committedRevision}`,
    markDirty,
    captureRevision,
    isCurrentRevision,
    commitRevision,
  };
}

export function useModelsWriteConfirm<T>() {
  const [pending, setPending] = useState<T | null>(null);
  const pendingRef = useRef<T | null>(null);

  const requestConfirm = useCallback((value: T) => {
    pendingRef.current = value;
    setPending(value);
  }, []);

  const takePending = useCallback((): T | null => {
    const value = pendingRef.current;
    pendingRef.current = null;
    setPending(null);
    return value;
  }, []);

  return {
    open: pending !== null,
    pending,
    requestConfirm,
    takePending,
  };
}

export function ModelsWriteDisclosure({
  targets,
}: {
  targets: readonly ModelWriteTarget[];
}) {
  if (targets.length === 0) return null;
  return (
    <div className="fy-models-write-disclosure">
      <div className="fy-models-write-targets">
        {targets.map((target) => (
          <div className="fy-models-write-target" key={target.path}>
            <div className="fy-models-write-path-row">
              <span className="fy-models-write-path-label">将修改</span>
              <CopyablePath label="配置文件路径" value={target.path} />
            </div>
            <div className="fy-models-write-path-row">
              <span className="fy-models-write-path-label">备份位置</span>
              <CopyablePath label="备份文件路径" value={target.backupPath} />
            </div>
            {!target.exists ? (
              <span className="fy-models-muted">
                当前文件尚不存在，首次创建时没有前像可备份。
              </span>
            ) : null}
          </div>
        ))}
      </div>
      <p className="fy-models-muted">
        每个文件只保留这一份备份；再次保存会用修改前的最新内容更新它。
      </p>
    </div>
  );
}

export function ModelsWriteConfirmDialog({
  open,
  targets,
  onConfirm,
  onCancel,
}: {
  open: boolean;
  targets: readonly ModelWriteTarget[];
  onConfirm: () => void;
  onCancel: () => void;
}) {
  return (
    <Dialog
      open={open}
      onOpenChange={(next) => {
        if (!next) onCancel();
      }}
      title="保存前确认"
      description="本次只修改下列配置文件中的相关模型字段，并在写入前保留一份滚动备份。"
      actions={
        <>
          <Button autoFocus onClick={onCancel}>
            取消
          </Button>
          <Button className="fy-control-button-primary" onClick={onConfirm}>
            确认保存
          </Button>
        </>
      }
    >
      <ModelsWriteDisclosure targets={targets} />
    </Dialog>
  );
}

export function noticeFromReachability(result: ReachabilityResult): Notice {
  if (!result.success) {
    return {
      tone: "error",
      title: "服务不可达",
      description: "请检查地址、网络或网关状态后重试。",
    };
  }
  if (result.status === "degraded") {
    return {
      tone: "warning",
      title: "服务可达，但响应较慢",
      description:
        result.responseTimeMs !== null
          ? `耗时 ${result.responseTimeMs} ms。`
          : undefined,
    };
  }
  return {
    tone: "info",
    title: "服务可达",
    description:
      result.httpStatus !== null ? `HTTP ${result.httpStatus}` : undefined,
  };
}

export function noticeFromModelProbe(result: ModelProbeResult): Notice {
  if (!result.success) {
    return {
      tone: "error",
      title: "连通测试失败",
      description:
        result.message.trim() || "请检查地址、凭据、模型和服务状态后重试。",
    };
  }
  if (result.status === "degraded") {
    return {
      tone: "warning",
      title: "连通测试成功，但响应较慢",
      description: result.message,
    };
  }
  return {
    tone: "info",
    title: "连通测试成功",
    description: result.message,
  };
}

export function ModelsPanelHeader({
  title,
  summary,
  pending,
  children,
}: {
  title: string;
  summary?: string;
  pending?: boolean;
  children?: ReactNode;
}) {
  return (
    <header
      className="fy-models-config-heading fy-models-commit-heading"
      data-pending={pending || undefined}
    >
      <div>
        <h2>{title}</h2>
        {summary && <p>{summary}</p>}
      </div>
      {children ? (
        <div className="fy-models-commit" data-testid="models-commit">
          {pending ? <Badge tone="warning">待保存</Badge> : null}
          {children}
        </div>
      ) : null}
    </header>
  );
}

export function ModelsGuidancePanel({
  ariaLabel,
  title,
  summary,
  children,
}: {
  ariaLabel: string;
  title: string;
  summary: string;
  children?: ReactNode;
}) {
  return (
    <CatalogDetail className="fy-models-config-panel" ariaLabel={ariaLabel}>
      <ModelsPanelHeader title={title} summary={summary} />
      {children}
    </CatalogDetail>
  );
}

function ModelsSurfaceToggle({
  title,
  onClick,
  expanded,
  testId,
  trailing,
}: {
  title: string;
  onClick: () => void;
  expanded?: boolean;
  testId?: string;
  trailing?: ReactNode;
}) {
  return (
    <button
      type="button"
      className="fy-models-existing-toggle"
      data-testid={testId}
      aria-expanded={expanded}
      onClick={onClick}
    >
      <h3>{title}</h3>
      {trailing ? (
        <span className="fy-models-existing-meta">{trailing}</span>
      ) : null}
    </button>
  );
}

export function ModelsExistingSection({
  title,
  countLabel,
  count,
  open,
  onOpenChange,
  testId,
  toggleTestId,
  ariaLabel,
  invalid = false,
  children,
}: {
  title: string;
  countLabel: string;
  count: number;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  testId?: string;
  toggleTestId?: string;
  ariaLabel?: string;
  invalid?: boolean;
  children?: ReactNode;
}) {
  return (
    <section
      className="fy-models-existing"
      data-testid={testId}
      data-invalid={invalid || undefined}
      aria-label={ariaLabel}
    >
      <ModelsSurfaceToggle
        title={title}
        expanded={open}
        testId={toggleTestId}
        onClick={() => onOpenChange(!open)}
        trailing={
          <>
            <span>{countLabel}</span>
            <strong className="fy-models-existing-count">{count}</strong>
            <CaretDownIcon
              className={classNames(
                "fy-models-caret",
                open && "fy-models-caret-open",
              )}
              size={18}
              aria-hidden
            />
          </>
        }
      />
      {open ? children : null}
    </section>
  );
}

export function NoApiKeyOption({
  checked,
  onCheckedChange,
  disabled = false,
}: {
  checked: boolean;
  onCheckedChange: (checked: boolean) => void;
  disabled?: boolean;
}) {
  return (
    <div className="fy-models-checkbox-row fy-models-checkbox-row-inline fy-models-form-wide">
      <Checkbox
        checked={checked}
        onCheckedChange={onCheckedChange}
        label="允许无 API Key"
        disabled={disabled}
      />
      <span>不使用 API Key</span>
      <Tooltip
        label={
          <span className="fy-models-help-copy">
            给不需要鉴权的本地模型使用，例如本机的 Ollama、LM
            Studio。勾选后请求不会携带 API Key。
          </span>
        }
      >
        <button
          type="button"
          className="fy-models-help"
          aria-label="不使用 API Key 说明"
        >
          <QuestionIcon size={16} weight="regular" aria-hidden />
        </button>
      </Tooltip>
    </div>
  );
}
