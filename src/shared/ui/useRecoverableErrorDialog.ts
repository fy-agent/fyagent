import { useCallback, useEffect, useState } from "react";

export interface RecoverableErrorDialogContent {
  key: string;
  title: string;
  message: string;
}

export function useRecoverableErrorDialog(
  error: RecoverableErrorDialogContent | null,
  options: { rearm: boolean; suppress: boolean },
): {
  content: RecoverableErrorDialogContent | null;
  open: boolean;
  dismiss: () => void;
} {
  const [dismissedKey, setDismissedKey] = useState<string | null>(null);
  const errorKey = error?.key ?? null;

  useEffect(() => {
    if (options.rearm) {
      setDismissedKey(null);
    } else if (options.suppress && errorKey) {
      setDismissedKey(errorKey);
    }
  }, [errorKey, options.rearm, options.suppress]);

  const dismiss = useCallback(() => {
    if (errorKey) setDismissedKey(errorKey);
  }, [errorKey]);

  return {
    content: error,
    open: error !== null && !options.suppress && dismissedKey !== errorKey,
    dismiss,
  };
}
