import {
  CancelledError,
  useQuery,
  useQueryClient,
} from "@tanstack/react-query";
import { useCallback, useEffect, useState } from "react";

import type { ChangeJobSnapshot, ChangePlansPort } from "../change-plans";
import { featureKeys } from "../queries";
import { usePersistentVisibility } from "../../ui/PersistentSurface";
import {
  changePlanErrorCode,
  isActiveJobStatus,
  JOB_REFRESH_INTERVAL_MS,
} from "./changePlanErrors";

/** Only redacted native snapshots enter Query; write requests and API keys never do. */
export function useChangeJob(port: ChangePlansPort, active: boolean) {
  const client = useQueryClient();
  const visible = usePersistentVisibility();
  const enabled = active && visible;
  const [jobId, setJobId] = useState<string | null>(null);
  const query = useQuery({
    queryKey: featureKeys.changeJob(jobId),
    queryFn: async ({ signal }) => {
      if (!jobId) throw { code: "job_not_found" };
      try {
        const snapshot = await port.getChangeJob(jobId);
        // IPC cannot be aborted, but an obsolete read must not overwrite newer authority.
        if (signal.aborted) throw new CancelledError();
        const current = client.getQueryData<ChangeJobSnapshot>(
          featureKeys.changeJob(jobId),
        );
        return current && current.revision > snapshot.revision
          ? current
          : snapshot;
      } catch (cause) {
        if (signal.aborted) throw new CancelledError();
        throw { code: changePlanErrorCode(cause) };
      }
    },
    enabled: enabled && jobId !== null,
    staleTime: Infinity,
    gcTime: 0,
    retry: false,
    refetchOnWindowFocus: false,
    refetchOnReconnect: false,
    refetchInterval: (current) =>
      current.state.error ||
      !current.state.data ||
      !isActiveJobStatus(current.state.data.status)
        ? false
        : JOB_REFRESH_INTERVAL_MS,
  });

  useEffect(() => {
    if (enabled || !jobId) return;
    // Do not cancel a read still owned by another visible observer of this job.
    void client.cancelQueries({
      queryKey: featureKeys.changeJob(jobId),
      exact: true,
      type: "inactive",
    });
  }, [client, enabled, jobId]);

  const setJob = useCallback(
    (snapshot: ChangeJobSnapshot | null) => {
      if (snapshot) {
        void client.cancelQueries(
          { queryKey: featureKeys.changeJob(snapshot.jobId), exact: true },
          { revert: false },
        );
        // setQueryData creates a query before its observer; GC keeps the longest
        // lifetime ever configured. Set one family default before that first seed.
        client.setQueryDefaults(featureKeys.changeJobs, { gcTime: 0 });
        client.setQueryData<ChangeJobSnapshot>(
          featureKeys.changeJob(snapshot.jobId),
          (current) =>
            current && current.revision > snapshot.revision
              ? current
              : snapshot,
        );
      }
      // Removing this observer lets Query cancel only when no other consumer remains.
      setJobId(snapshot?.jobId ?? null);
    },
    [client],
  );

  return {
    refetch: query.refetch,
    job: jobId ? (query.data ?? null) : null,
    error:
      jobId && query.error ? { code: changePlanErrorCode(query.error) } : null,
    setJob,
  };
}
