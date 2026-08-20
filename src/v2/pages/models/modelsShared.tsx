import { CaretDownIcon } from "@phosphor-icons/react/dist/csr/CaretDown";
import { QuestionIcon } from "@phosphor-icons/react/dist/csr/Question";
import type { ReactNode } from "react";

import { classNames } from "../../shared/design-system/classNames";
import { CatalogDetail } from "../../shared/ui/catalog";
import { Badge, Checkbox, Tooltip } from "../../shared/ui/primitives";
import { FieldFeedback, type Notice } from "./feedback";
import type { ReachabilityResult } from "../../shared/features/types";

export function NoticeView({ notice }: { notice: Notice | null }) {
  return <FieldFeedback notice={notice} />;
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

export function ModelsPanelHeader({
  title,
  summary,
  pending = false,
  children,
}: {
  title: string;
  summary: string;
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
        <p>{summary}</p>
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
