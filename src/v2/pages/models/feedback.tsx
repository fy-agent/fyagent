/* eslint-disable react-refresh/only-export-components */
import { CheckCircleIcon } from "@phosphor-icons/react/dist/csr/CheckCircle";
import { WarningCircleIcon } from "@phosphor-icons/react/dist/csr/WarningCircle";
import { useState, type ReactNode } from "react";

import { classNames } from "../../shared/design-system/classNames";
import { InlineNotice } from "../../shared/ui/primitives";

export type Notice = {
  tone: "info" | "warning" | "error";
  title: string;
  description?: string;
};

export type FieldNotices<K extends string> = Partial<Record<K, Notice>>;

export function useFieldNotices<K extends string>() {
  const [notices, setNotices] = useState<FieldNotices<K>>({});

  const show = (field: K, notice: Notice) => {
    setNotices({ [field]: notice } as FieldNotices<K>);
  };

  const patch = (field: K, notice: Notice) => {
    setNotices((current) => ({ ...current, [field]: notice }));
  };

  const clear = () => setNotices({});

  const dismiss = (field: K) => {
    setNotices((current) => {
      if (current[field] === undefined) return current;
      const next = { ...current };
      delete next[field];
      return next;
    });
  };

  return { notices, setNotices, show, patch, clear, dismiss };
}

export function focusControl(node: HTMLElement | null) {
  node?.focus();
  node?.scrollIntoView?.({ block: "nearest", inline: "nearest" });
}

export function isErrorNotice(notice: Notice | undefined): boolean {
  return notice?.tone === "error";
}

export function FieldFeedback({
  id,
  notice,
}: {
  id?: string;
  notice: Notice | null | undefined;
}) {
  if (!notice) return null;
  const Icon = notice.tone === "info" ? CheckCircleIcon : WarningCircleIcon;
  return (
    <div className={`fy-models-feedback fy-models-feedback-${notice.tone}`}>
      <InlineNotice tone={notice.tone}>
        <span className="fy-models-feedback-icon" aria-hidden>
          <Icon size={16} weight="fill" />
        </span>
        <div className="fy-models-feedback-copy" id={id}>
          <strong>{notice.title}</strong>
          {notice.description ? <p>{notice.description}</p> : null}
        </div>
      </InlineNotice>
    </div>
  );
}

export function ModelsSection({
  title,
  titleId,
  invalid = false,
  children,
  className,
  testId,
  ariaLabel,
}: {
  title?: string;
  titleId?: string;
  invalid?: boolean;
  children: ReactNode;
  className?: string;
  testId?: string;
  ariaLabel?: string;
}) {
  return (
    <section
      className={classNames("fy-models-section", className)}
      data-invalid={invalid || undefined}
      data-testid={testId}
      aria-label={ariaLabel}
      aria-labelledby={titleId}
    >
      {title ? (
        <h3 id={titleId} className="fy-models-section-title">
          {title}
        </h3>
      ) : null}
      {children}
    </section>
  );
}
