import { useTranslation } from "react-i18next";
import type { ToolInstallation } from "@/lib/api/settings";

/**
 * 单处工具安装的信息行：来源徽章 + 版本（或「无法运行」）+「默认」标记。
 * 冲突诊断列表与升级确认对话框共用。不展示绝对路径。
 */
export function ToolInstallRow({ inst }: { inst: ToolInstallation }) {
  const { t } = useTranslation();
  return (
    <div className="flex items-center gap-1.5 text-[10px]">
      <span className="shrink-0 rounded bg-background/80 px-1 py-0.5 font-mono text-muted-foreground">
        {inst.source}
      </span>
      <span
        className={
          inst.runnable
            ? "min-w-0 flex-1 truncate font-mono text-foreground"
            : "min-w-0 flex-1 truncate text-yellow-600 dark:text-yellow-400"
        }
      >
        {inst.runnable ? inst.version : t("settings.toolConflictNotRunnable")}
      </span>
      {inst.is_path_default && (
        <span className="shrink-0 rounded-full border border-primary/30 bg-primary/10 px-1 py-0.5 text-[9px] text-primary">
          {t("settings.toolConflictDefault")}
        </span>
      )}
    </div>
  );
}
