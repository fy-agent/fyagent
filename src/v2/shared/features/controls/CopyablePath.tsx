import { useEffect, useState } from "react";

import { useFeatures } from "../provider";
import { Button } from "../../ui/primitives";

export function CopyablePath({
  value,
  label = "安装目录",
  revealValue = true,
}: {
  value: string;
  label?: string;
  revealValue?: boolean;
}) {
  const { notify } = useFeatures();
  const [copied, setCopied] = useState(false);
  const actionLabel = copied ? `已复制${label}` : `复制${label}`;

  useEffect(() => {
    if (!copied) return;
    const timer = window.setTimeout(() => setCopied(false), 1600);
    return () => window.clearTimeout(timer);
  }, [copied]);

  const copyPath = async () => {
    try {
      await navigator.clipboard.writeText(value);
      setCopied(true);
    } catch {
      notify({ tone: "error", title: `无法复制${label}` });
    }
  };

  return (
    <div className="fy-feature-path">
      {revealValue ? (
        <code className="fy-feature-path-value" title={value}>
          {value}
        </code>
      ) : null}
      <Button aria-label={actionLabel} onClick={() => void copyPath()}>
        {copied ? "已复制" : "复制"}
      </Button>
    </div>
  );
}
