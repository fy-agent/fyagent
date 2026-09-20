import { useQuery } from "@tanstack/react-query";

import { usePersistentVisibility } from "../ui/PersistentSurface";
import { useFeatures } from "./provider";

export function useAppVersion(enabled: boolean) {
  const { ports } = useFeatures();
  const visible = usePersistentVisibility();
  return useQuery({
    queryKey: ["fyagent", "app-version"],
    queryFn: ports.settings.getAppVersion,
    enabled: enabled && visible,
    staleTime: Infinity,
    retry: false,
    refetchOnWindowFocus: false,
    refetchOnReconnect: false,
  });
}
