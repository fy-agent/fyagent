import type { ButtonHTMLAttributes, ReactNode } from "react";

import { useOpenExternal } from "../features/provider";
import { Button } from "./primitives";

type ExternalLinkButtonProps = {
  url?: string;
  children: ReactNode;
  errorTitle?: string;
  busyLabel?: string;
} & Omit<
  ButtonHTMLAttributes<HTMLButtonElement>,
  "children" | "onClick" | "type"
>;

export function ExternalLinkButton({
  url,
  children,
  errorTitle = "无法打开链接",
  busyLabel = "正在打开…",
  disabled,
  className,
  ...props
}: ExternalLinkButtonProps) {
  const { openExternal, openingUrl } = useOpenExternal();
  const opening = Boolean(url) && openingUrl === url;

  return (
    <Button
      {...props}
      className={className}
      disabled={disabled || !url || openingUrl !== null}
      aria-busy={opening || undefined}
      onClick={() => {
        if (!url) return;
        void openExternal(url, { errorTitle });
      }}
    >
      {opening ? busyLabel : children}
    </Button>
  );
}
