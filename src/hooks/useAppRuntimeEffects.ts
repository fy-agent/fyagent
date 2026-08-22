import { useCallback, useEffect, useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import type { AppId, ProviderSwitchEvent } from "@/lib/api";
import { providersApi, systemApi } from "@/lib/api";
import { checkAllEnvConflicts, checkEnvConflicts } from "@/lib/api/env";
import { proxyKeys } from "@/lib/query";
import type { EnvConflict } from "@/types/env";
import { useTauriEvent } from "./useTauriEvent";

interface SyncStatusUpdatedPayload {
  source?: string;
  status?: string;
  error?: string;
}

interface UseAppRuntimeEffectsOptions {
  activeApp: AppId;
  enabled: boolean;
}

const ENV_BANNER_DISMISSED_KEY = "env_banner_dismissed";

export function useAppRuntimeEffects({
  activeApp,
  enabled,
}: UseAppRuntimeEffectsOptions) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [envConflicts, setEnvConflicts] = useState<EnvConflict[]>([]);
  const [showEnvBanner, setShowEnvBanner] = useState(false);

  const loadAllEnvConflicts = useCallback(async (): Promise<EnvConflict[]> => {
    const allConflicts = await checkAllEnvConflicts();
    return Object.values(allConflicts).flat();
  }, []);

  const refreshEnvConflicts = useCallback(async () => {
    try {
      const conflicts = await loadAllEnvConflicts();
      setEnvConflicts(conflicts);
      if (conflicts.length === 0) setShowEnvBanner(false);
    } catch (error) {
      console.error(
        "[App] Failed to re-check conflicts after deletion:",
        error,
      );
    }
  }, [loadAllEnvConflicts]);

  const dismissEnvBanner = useCallback(() => {
    setShowEnvBanner(false);
    sessionStorage.setItem(ENV_BANNER_DISMISSED_KEY, "true");
  }, []);

  useEffect(() => {
    if (!enabled) return;

    let unsubscribe: (() => void) | undefined;
    let active = true;

    const setupListener = async () => {
      try {
        const off = await providersApi.onSwitched(
          async (event: ProviderSwitchEvent) => {
            if (event.appType === activeApp) {
              await queryClient.invalidateQueries({
                queryKey: ["providers", activeApp],
              });
            }
          },
        );
        if (!active) {
          off();
          return;
        }
        unsubscribe = off;
      } catch (error) {
        console.error("[App] Failed to subscribe provider switch event", error);
      }
    };

    void setupListener();
    return () => {
      active = false;
      unsubscribe?.();
    };
  }, [activeApp, enabled, queryClient]);

  useTauriEvent(
    "universal-provider-synced",
    async () => {
      await queryClient.invalidateQueries({ queryKey: ["providers"] });
      try {
        await providersApi.updateTrayMenu();
      } catch (error) {
        console.error("[App] Failed to update tray menu", error);
      }
    },
    enabled,
  );

  useTauriEvent(
    "profile-applied",
    async () => {
      await queryClient.invalidateQueries({ queryKey: ["profiles"] });
      await queryClient.invalidateQueries({ queryKey: ["mcp", "all"] });
      await queryClient.invalidateQueries({ queryKey: ["skills"] });
      await queryClient.invalidateQueries({
        queryKey: proxyKeys.takeoverStatus,
      });
      await queryClient.invalidateQueries({ queryKey: proxyKeys.status });
      await queryClient.invalidateQueries({
        queryKey: ["providers", "claude-desktop"],
      });
    },
    enabled,
  );

  useTauriEvent<SyncStatusUpdatedPayload | null | undefined>(
    "webdav-sync-status-updated",
    async (payload) => {
      const statusPayload = payload ?? {};
      await queryClient.invalidateQueries({ queryKey: ["settings"] });
      if (statusPayload.source !== "auto" || statusPayload.status !== "error") {
        return;
      }
      toast.error(
        t("settings.webdavSync.autoSyncFailedToast", {
          error: statusPayload.error || t("common.unknown"),
        }),
      );
    },
  );

  useTauriEvent<SyncStatusUpdatedPayload | null | undefined>(
    "s3-sync-status-updated",
    async (payload) => {
      const statusPayload = payload ?? {};
      await queryClient.invalidateQueries({ queryKey: ["settings"] });
      if (statusPayload.source !== "auto" || statusPayload.status !== "error") {
        return;
      }
      toast.error(
        t("settings.s3Sync.autoSyncFailedToast", {
          error: statusPayload.error || t("common.unknown"),
        }),
      );
    },
  );

  useTauriEvent<{ appType: string; providerName: string }>(
    "proxy-official-warning",
    (payload) => {
      toast.warning(
        t("notifications.proxyOfficialWarning", {
          name: payload.providerName,
          defaultValue: `当前供应商 ${payload.providerName} 是官方供应商，建议切换到第三方供应商后再使用代理接管`,
        }),
        { duration: 8000 },
      );
    },
    enabled,
  );

  useEffect(() => {
    if (!enabled) return;

    const checkEnvOnStartup = async () => {
      try {
        const conflicts = await loadAllEnvConflicts();
        if (conflicts.length > 0) {
          setEnvConflicts(conflicts);
          const dismissed = sessionStorage.getItem(ENV_BANNER_DISMISSED_KEY);
          if (!dismissed) setShowEnvBanner(true);
        }
      } catch (error) {
        console.error(
          "[App] Failed to check environment conflicts on startup:",
          error,
        );
      }
    };

    void checkEnvOnStartup();
  }, [enabled, loadAllEnvConflicts]);

  useEffect(() => {
    if (!enabled) return;

    const checkMigration = async () => {
      try {
        const migrated = await systemApi.getMigrationResult();
        if (migrated) {
          toast.success(
            t("migration.success", { defaultValue: "配置迁移成功" }),
            { closeButton: true },
          );
        }
      } catch (error) {
        console.error("[App] Failed to check migration result:", error);
      }
    };

    void checkMigration();
  }, [enabled, t]);

  useEffect(() => {
    if (!enabled) return;

    const checkSkillsMigration = async () => {
      try {
        const result = await systemApi.getSkillsMigrationResult();
        if (result?.error) {
          toast.error(t("migration.skillsFailed"), {
            description: t("migration.skillsFailedDescription"),
            closeButton: true,
          });
          console.error("[App] Skills SSOT migration failed:", result.error);
          return;
        }
        if (result && result.count > 0) {
          toast.success(t("migration.skillsSuccess", { count: result.count }), {
            closeButton: true,
          });
          await queryClient.invalidateQueries({ queryKey: ["skills"] });
        }
      } catch (error) {
        console.error("[App] Failed to check skills migration result:", error);
      }
    };

    void checkSkillsMigration();
  }, [enabled, queryClient, t]);

  useEffect(() => {
    if (!enabled) return;

    const checkEnvOnSwitch = async () => {
      try {
        const conflicts = await checkEnvConflicts(activeApp);
        if (conflicts.length === 0) return;

        setEnvConflicts((previous) => {
          const existingKeys = new Set(
            previous.map(
              (conflict) => `${conflict.varName}:${conflict.sourcePath}`,
            ),
          );
          const newConflicts = conflicts.filter(
            (conflict) =>
              !existingKeys.has(`${conflict.varName}:${conflict.sourcePath}`),
          );
          return [...previous, ...newConflicts];
        });
        const dismissed = sessionStorage.getItem(ENV_BANNER_DISMISSED_KEY);
        if (!dismissed) setShowEnvBanner(true);
      } catch (error) {
        console.error(
          "[App] Failed to check environment conflicts on app switch:",
          error,
        );
      }
    };

    void checkEnvOnSwitch();
  }, [activeApp, enabled]);

  return {
    dismissEnvBanner,
    envConflicts,
    refreshEnvConflicts,
    showEnvBanner,
  };
}
