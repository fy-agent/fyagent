import { useEffect, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import {
  useMutation,
  useQuery,
  useQueryClient,
  type Query,
} from "@tanstack/react-query";
import {
  changeJobUpdatedEventSchema,
  changePlanApi,
  type ChangeJobSnapshot,
  type ChangeJobUpdatedEvent,
} from "@/lib/api/change-plan";

export const CHANGE_JOB_POLL_INTERVAL_MS = 1000;

export const changePlanKeys = {
  all: ["changePlan"] as const,
  job: (jobId: string) => [...changePlanKeys.all, "job", jobId] as const,
  recoverable: () => [...changePlanKeys.all, "recoverable"] as const,
};

export function isTerminalChangeJob(job?: ChangeJobSnapshot): boolean {
  return !!job && !["planned", "running"].includes(job.status);
}

export function changeJobRefetchInterval(
  query: Query<ChangeJobSnapshot>,
): number | false {
  return isTerminalChangeJob(query.state.data)
    ? false
    : CHANGE_JOB_POLL_INTERVAL_MS;
}

export function shouldInvalidateChangeJobEvent(
  event: ChangeJobUpdatedEvent,
  jobId: string,
  acceptedEventSeq: number,
): boolean {
  return event.jobId === jobId && event.eventSeq > acceptedEventSeq;
}

export function useCreateCodexProviderSwitchPlan() {
  return useMutation({
    mutationFn: (targetProviderId: string) =>
      changePlanApi.createCodexProviderSwitchPlan(targetProviderId),
  });
}

export function useApplyChangePlan() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({
      planId,
      planDigest,
    }: {
      planId: string;
      planDigest: string;
    }) => changePlanApi.apply(planId, planDigest),
    onSuccess: (outcome) => {
      if (outcome.job) {
        queryClient.setQueryData(
          changePlanKeys.job(outcome.job.jobId),
          outcome.job,
        );
      }
    },
  });
}

export function useChangeJob(jobId?: string) {
  const queryClient = useQueryClient();
  const acceptedEventSeq = useRef(0);
  const acceptedJobId = useRef<string>();
  const query = useQuery({
    queryKey: changePlanKeys.job(jobId ?? "none"),
    queryFn: () => changePlanApi.getJob(jobId!),
    enabled: !!jobId,
    refetchInterval: changeJobRefetchInterval,
  });

  useEffect(() => {
    if (!jobId) return;
    if (acceptedJobId.current !== jobId) {
      acceptedJobId.current = jobId;
      acceptedEventSeq.current = 0;
    }
    acceptedEventSeq.current = Math.max(
      acceptedEventSeq.current,
      query.data?.eventSeq ?? 0,
    );
  }, [jobId, query.data?.eventSeq]);

  useEffect(() => {
    if (!jobId) return;
    let disposed = false;
    let unlisten: (() => void) | undefined;
    void listen<unknown>("change-job://updated", (raw) => {
      const parsed = changeJobUpdatedEventSchema.safeParse(raw.payload);
      if (
        parsed.success &&
        shouldInvalidateChangeJobEvent(
          parsed.data,
          jobId,
          acceptedEventSeq.current,
        )
      ) {
        acceptedEventSeq.current = parsed.data.eventSeq;
        void queryClient.invalidateQueries({
          queryKey: changePlanKeys.job(jobId),
        });
      }
    })
      .then((release) => {
        if (disposed) release();
        else unlisten = release;
      })
      .catch(() => undefined);
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [jobId, queryClient]);

  return query;
}
