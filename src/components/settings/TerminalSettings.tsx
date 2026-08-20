import { useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { isMac, isWindows } from "@/lib/platform";

// Terminal options per platform
const MACOS_TERMINALS = [
  { value: "terminal", labelKey: "settings.terminal.options.macos.terminal" },
  { value: "iterm2", labelKey: "settings.terminal.options.macos.iterm2" },
  { value: "alacritty", labelKey: "settings.terminal.options.macos.alacritty" },
  { value: "kitty", labelKey: "settings.terminal.options.macos.kitty" },
  { value: "ghostty", labelKey: "settings.terminal.options.macos.ghostty" },
  { value: "wezterm", labelKey: "settings.terminal.options.macos.wezterm" },
  { value: "kaku", labelKey: "settings.terminal.options.macos.kaku" },
  { value: "warp", labelKey: "settings.terminal.options.macos.warp" },
] as const;

const WINDOWS_TERMINALS = [
  { value: "cmd", labelKey: "settings.terminal.options.windows.cmd" },
  {
    value: "powershell",
    labelKey: "settings.terminal.options.windows.powershell",
  },
  { value: "wt", labelKey: "settings.terminal.options.windows.wt" },
] as const;

function getTerminalConfiguration() {
  if (isMac()) {
    return { options: MACOS_TERMINALS, defaultTerminal: "terminal" } as const;
  }
  if (isWindows()) {
    return { options: WINDOWS_TERMINALS, defaultTerminal: "cmd" } as const;
  }
  return null;
}

export interface TerminalSettingsProps {
  value?: string;
  onChange: (value: string) => void;
}

export function TerminalSettings({ value, onChange }: TerminalSettingsProps) {
  const { t } = useTranslation();
  const configuration = getTerminalConfiguration();
  const onChangeRef = useRef(onChange);
  onChangeRef.current = onChange;

  const isSupportedValue =
    configuration?.options.some((terminal) => terminal.value === value) ??
    false;
  const currentValue = isSupportedValue
    ? value
    : configuration?.defaultTerminal;
  const defaultTerminal = configuration?.defaultTerminal;

  useEffect(() => {
    if (
      value !== undefined &&
      defaultTerminal !== undefined &&
      !isSupportedValue
    ) {
      onChangeRef.current(defaultTerminal);
    }
  }, [defaultTerminal, isSupportedValue, value]);

  if (!configuration) {
    return (
      <section className="space-y-2">
        <header className="space-y-1">
          <h3 className="text-sm font-medium">
            {t("settings.terminal.title")}
          </h3>
          <p className="text-xs text-muted-foreground">
            {t("settings.terminal.description")}
          </p>
        </header>
        <p className="text-xs text-muted-foreground">
          {t("settings.terminal.unsupportedPlatform")}
        </p>
      </section>
    );
  }

  return (
    <section className="space-y-2">
      <header className="space-y-1">
        <h3 className="text-sm font-medium">{t("settings.terminal.title")}</h3>
        <p className="text-xs text-muted-foreground">
          {t("settings.terminal.description")}
        </p>
      </header>
      <Select value={currentValue} onValueChange={onChange}>
        <SelectTrigger className="w-[200px]">
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          {configuration.options.map((terminal) => (
            <SelectItem key={terminal.value} value={terminal.value}>
              {t(terminal.labelKey)}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
      <p className="text-xs text-muted-foreground">
        {t("settings.terminal.fallbackHint")}
      </p>
    </section>
  );
}
