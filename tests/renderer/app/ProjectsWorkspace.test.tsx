import { useQueryClient, type QueryClient } from "@tanstack/react-query";
import {
  act,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { ProjectsWorkspace } from "@/app/ProjectsWorkspace";
import type {
  ProjectPanelProps,
  ProjectsPageProps,
} from "@/pages/projects/Page";
import type { KitEvidenceAdapter } from "@/shared/features/delivery-kits";
import { FeatureProvider } from "@/shared/features/provider";
import { featureKeys } from "@/shared/features/queries";
import { createBrowserFeaturePorts } from "@/shared/platform/browser/features";
import type { Project } from "@/domain/projects";
import type { VerificationSnapshot } from "@/domain/verification";
import {
  evidenceFixture,
  verificationFixture,
  PROJECT,
  OTHER_PROJECT,
} from "../fixtures/verification";

const scope = vi.hoisted(() => ({
  projectId: "00000000-0000-4000-8000-000000000001",
  revision: 1,
  adapter: null as KitEvidenceAdapter | null,
}));
vi.mock("@/pages/projects/Page", () => ({
  ProjectsPage: ({
    DeliveryKitPanel,
    VerificationPanel,
  }: ProjectsPageProps) => {
    const props: ProjectPanelProps = {
      projectId: scope.projectId,
      projectRevision: scope.revision,
      disabled: false,
      archived: false,
      onProjectChanged: async () => undefined,
    };
    return (
      <>
        {DeliveryKitPanel && <DeliveryKitPanel {...props} />}
        {VerificationPanel && <VerificationPanel {...props} />}
      </>
    );
  },
}));
vi.mock("@/shared/features/delivery-kits-ui/ProjectDeliveryKitsPanel", () => ({
  ProjectDeliveryKitsPanel: ({
    evidenceAdapter,
  }: {
    evidenceAdapter: KitEvidenceAdapter;
  }) => {
    scope.adapter = evidenceAdapter;
    return null;
  },
}));
const kit = {
  kitId: "weekly-report",
  kitVersion: "1.0.0",
  manifestDigest: "a".repeat(64),
};
function project(projectId: string): Project {
  return {
    projectId,
    projectRevision: scope.revision,
    customerId: OTHER_PROJECT,
    name: "Project",
    archived: false,
    resources: [],
    credentials: [],
    kit,
    contextGeneration: null,
    codexPrepared: false,
    createdAt: "2026-09-19T00:00:00Z",
    updatedAt: "2026-09-19T00:00:00Z",
  };
}
function snapshot(
  projectId: string,
  revision: number,
  stale: boolean,
): VerificationSnapshot {
  return {
    ...verificationFixture(projectId),
    projectRevision: revision,
    evidence: [
      {
        ...evidenceFixture(),
        recordedAt: new Date(Date.now() - 1000).toISOString(),
        validity: stale ? "stale" : "current",
        kit,
      },
    ],
  };
}

describe("project verification session writeback", () => {
  it.each([
    ["panel", "revision", false],
    ["panel", "project", false],
    ["kit", "revision", false],
    ["kit", "project", false],
    ["panel", "revision", true],
  ] as const)(
    "drops old %s responses after %s changes (reject=%s)",
    async (origin, change, reject) => {
      scope.projectId = PROJECT;
      scope.revision = 1;
      const ports = createBrowserFeaturePorts();
      ports.projects.get = vi.fn(async (id) => project(id));
      ports.verification.get = vi.fn(async (id) =>
        snapshot(id, scope.revision, scope.revision > 1),
      );
      let resolve!: (value: VerificationSnapshot) => void;
      let fail!: (error: Error) => void;
      ports.verification.run = vi.fn(
        () =>
          new Promise<VerificationSnapshot>((yes, no) => {
            resolve = yes;
            fail = no;
          }),
      );
      let client!: QueryClient;
      function Cache() {
        client = useQueryClient();
        return null;
      }
      const view = () => (
        <FeatureProvider ports={ports}>
          <Cache />
          <ProjectsWorkspace />
        </FeatureProvider>
      );
      const rendered = render(view());
      await screen.findByText("通过");
      let kitOperation: Promise<unknown> | undefined;
      if (origin === "panel") {
        fireEvent.click(screen.getByText("检查保存的配置"));
      } else {
        kitOperation = scope
          .adapter!.recordLocalFixture({
            ...kit,
            projectId: PROJECT,
            projectRevision: 1,
          })
          .catch(() => undefined);
      }
      await waitFor(() =>
        expect(ports.verification.run).toHaveBeenCalledTimes(1),
      );
      const runId = vi.mocked(ports.verification.run).mock.calls[0][0].runId;
      scope.revision = 2;
      if (change === "project") scope.projectId = OTHER_PROJECT;
      rendered.rerender(view());
      await screen.findByText("待复核");
      await waitFor(() =>
        expect(
          client.getQueryData<VerificationSnapshot>(
            featureKeys.verification(scope.projectId),
          )?.projectRevision,
        ).toBe(2),
      );
      const cacheBefore = client.getQueryData(
        featureKeys.verification(PROJECT),
      );
      await act(async () => {
        if (reject) fail(new Error("late failure"));
        else
          resolve({
            ...snapshot(PROJECT, 1, false),
            evidence: [
              {
                ...evidenceFixture(),
                recordedAt: new Date(Date.now() - 1000).toISOString(),
                id: runId,
                kit,
              },
            ],
          });
        await kitOperation;
      });
      expect(client.getQueryData(featureKeys.verification(PROJECT))).toEqual(
        cacheBefore,
      );
      expect(
        client.getQueryData<VerificationSnapshot>(
          featureKeys.verification(scope.projectId),
        )?.projectRevision,
      ).toBe(2);
      expect(screen.queryByText("通过")).not.toBeInTheDocument();
      expect(screen.getByText("待复核")).toBeVisible();
      expect(screen.queryByRole("alert")).not.toBeInTheDocument();
      expect(
        client.getQueryState(featureKeys.verification(scope.projectId))
          ?.isInvalidated,
      ).toBe(false);
    },
  );
});
