import { useEffect, useMemo, useRef } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { ProjectsPage, type ProjectPanelProps } from "../pages/projects/Page";
import { ProjectDeliveryKitsPanel } from "../shared/features/delivery-kits-ui/ProjectDeliveryKitsPanel";
import { VerificationPanel } from "../shared/features/verification/VerificationPanel";
import { useFeatures } from "../shared/features/provider";
import { featureKeys } from "../shared/features/queries";
import type {
  KitEvidenceAdapter,
  KitProjectAdapter,
} from "../shared/features/delivery-kits";

function DeliveryPanel(props: ProjectPanelProps) {
  return (
    <DeliverySession
      key={`${props.projectId}:${props.projectRevision}:${props.disabled || props.archived}`}
      {...props}
    />
  );
}

function DeliverySession(props: ProjectPanelProps) {
  const live = useRef(true);
  useEffect(() => {
    live.current = true;
    return () => {
      live.current = false;
    };
  }, []);
  const { ports } = useFeatures();
  const client = useQueryClient();
  const projectAdapter = useMemo<KitProjectAdapter>(
    () => ({
      bindDeliveryKit: async (request) => {
        if (!live.current || props.disabled || props.archived)
          throw new Error("project_changed");
        const result = await ports.projects.bindDeliveryKit(request);
        if (!live.current) throw new Error("project_changed");
        await client.invalidateQueries({
          queryKey: featureKeys.verification(request.projectId),
        });
        return result;
      },
    }),
    [ports, client, props.disabled, props.archived],
  );
  const evidenceAdapter = useMemo<KitEvidenceAdapter>(
    () => ({
      recordLocalFixture: async (request) => {
        if (!live.current || props.disabled || props.archived)
          throw new Error("project_changed");
        const project = await ports.projects.get(request.projectId);
        if (
          !live.current ||
          project.projectRevision !== request.projectRevision ||
          project.kit?.kitId !== request.kitId ||
          project.kit.kitVersion !== request.kitVersion ||
          project.kit.manifestDigest !== request.manifestDigest
        )
          throw new Error("project_changed");
        const runId = crypto.randomUUID();
        const snapshot = await ports.verification.run({
          projectId: request.projectId,
          expectedRevision: request.projectRevision,
          checker: "kit_validator",
          fixture: "baseline",
          runId,
        });
        if (!live.current) throw new Error("project_changed");
        const record = snapshot.evidence.find((entry) => entry.id === runId);
        if (
          !record ||
          record.validity !== "current" ||
          record.kit?.manifestDigest !== request.manifestDigest
        )
          throw new Error("evidence_unavailable");
        client.setQueryData(
          featureKeys.verification(request.projectId),
          snapshot,
        );
        return { recordId: record.id };
      },
    }),
    [ports, client, props.disabled, props.archived],
  );
  return (
    <ProjectDeliveryKitsPanel
      projectId={props.projectId}
      projectRevision={props.projectRevision}
      currentKit={props.kit}
      mutationBlockedReason={
        props.archived
          ? "项目已归档，可查看方案与样例。"
          : props.disabled
            ? "请先保存项目资料，再更换方案或保存检查记录。"
            : undefined
      }
      port={ports.deliveryKits}
      projectAdapter={projectAdapter}
      evidenceAdapter={evidenceAdapter}
      onProjectChanged={() => void props.onProjectChanged()}
    />
  );
}

function EvidencePanel(props: ProjectPanelProps) {
  return (
    <VerificationPanel
      key={`${props.projectId}:${props.projectRevision}`}
      projectId={props.projectId}
      mutationBlockedReason={
        props.archived
          ? "项目已归档，已有记录可查阅和导出。"
          : props.disabled
            ? "请先保存项目资料，再检查或登记结果。"
            : undefined
      }
    />
  );
}

export function ProjectsWorkspace() {
  return (
    <ProjectsPage
      DeliveryKitPanel={DeliveryPanel}
      VerificationPanel={EvidencePanel}
    />
  );
}
