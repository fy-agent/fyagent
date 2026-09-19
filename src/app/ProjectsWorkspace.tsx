import { useMemo } from "react";
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
  const { ports } = useFeatures();
  const client = useQueryClient();
  const projectAdapter = useMemo<KitProjectAdapter>(
    () => ({
      bindDeliveryKit: async (request) => {
        if (props.disabled || props.archived)
          throw new Error("project_changed");
        const result = await ports.projects.bindDeliveryKit(request);
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
        if (props.disabled || props.archived)
          throw new Error("project_changed");
        const project = await ports.projects.get(request.projectId);
        if (
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
      projectId={props.disabled || props.archived ? null : props.projectId}
      projectRevision={props.projectRevision}
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
