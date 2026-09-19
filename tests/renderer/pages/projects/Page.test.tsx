import {
  fireEvent,
  render,
  screen,
  waitFor,
  act,
} from "@testing-library/react";
import { createMemoryRouter, RouterProvider } from "react-router-dom";
import { describe, expect, it, vi } from "vitest";
import { ProjectsPage } from "@/pages/projects/Page";
import type { Project, ProjectDependencySnapshot } from "@/domain/projects";
import { FeatureProvider } from "@/shared/features/provider";
import { PrimaryBlockerProvider } from "@/shared/ui/PrimaryBlocker";
import { createBrowserFeaturePorts } from "@/shared/platform/browser/features";

const a = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa";
const b = "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb";
const customer = "cccccccc-cccc-4ccc-8ccc-cccccccccccc";
function project(id: string, name: string): Project {
  return {
    projectId: id,
    customerId: customer,
    name,
    projectRevision: 0,
    archived: false,
    resources: [],
    credentials: [],
    kit: null,
    contextGeneration: null,
    codexPrepared: false,
    createdAt: "2026-09-19T00:00:00Z",
    updatedAt: "2026-09-19T00:00:00Z",
  };
}
function fixture() {
  const ports = createBrowserFeaturePorts();
  const records = [project(a, "项目 A"), project(b, "项目 B")];
  const contents = new Map([
    [a, "A 的说明"],
    [b, "B 的说明"],
  ]);
  ports.projects.list = vi.fn(async () => records.map((p) => ({ ...p })));
  ports.projects.listCustomers = vi.fn(async () => [
    { customerId: customer, name: "客户", revision: 0, archived: false },
  ]);
  ports.projects.getContext = vi.fn(async (projectId: string) => ({
    projectId,
    projectRevision: records.find((p) => p.projectId === projectId)!
      .projectRevision,
    content: contents.get(projectId)!,
    directory: null,
    codexInstructions: null,
    state: "not_created" as const,
  }));
  ports.projects.resourceOptions = vi.fn(async () => []);
  ports.projects.credentialOptions = vi.fn(async () => []);
  ports.projects.dependencySnapshot = vi.fn(
    async (projectId: string): Promise<ProjectDependencySnapshot> => ({
      projectId,
      projectRevision: 0,
      archived: false,
      kit: null,
      kitState: "missing",
      resources: [],
      credentials: [],
      contextGeneration: null,
      contextState: "not_created",
      observedAt: "2026-09-19T00:00:00Z",
      runtimeAvailable: false,
    }),
  );
  ports.projects.writeContext = vi.fn();
  const router = createMemoryRouter(
    [
      {
        path: "/projects",
        element: (
          <FeatureProvider ports={ports}>
            <PrimaryBlockerProvider>
              <ProjectsPage />
            </PrimaryBlockerProvider>
          </FeatureProvider>
        ),
      },
    ],
    { initialEntries: [`/projects?project=${a}`] },
  );
  return { ports, router, records, contents };
}
describe("customer projects fixture UI", () => {
  it("saves both edited fields without discarding the other draft", async () => {
    const { ports, router, records, contents } = fixture();
    ports.projects.update = vi.fn(async (request, name, archived) => {
      const p = records.find((p) => p.projectId === request.projectId)!;
      expect(request.expectedRevision).toBe(p.projectRevision);
      Object.assign(p, {
        name,
        archived,
        projectRevision: p.projectRevision + 1,
      });
      return { ...p };
    });
    ports.projects.writeContext = vi.fn(async (request, content) => {
      const p = records.find((p) => p.projectId === request.projectId)!;
      expect(request.expectedRevision).toBe(p.projectRevision);
      p.projectRevision += 1;
      contents.set(p.projectId, content);
      return ports.projects.getContext(p.projectId);
    });
    render(<RouterProvider router={router} />);
    await screen.findByDisplayValue("A 的说明");
    fireEvent.change(screen.getAllByLabelText("项目名称").at(-1)!, {
      target: { value: "新版名称" },
    });
    fireEvent.change(screen.getByLabelText("工作说明"), {
      target: { value: "新版说明" },
    });
    fireEvent.click(
      screen.getAllByRole("button", { name: "保存名称" }).at(-1)!,
    );
    await waitFor(() =>
      expect(
        screen.getByRole("heading", { name: "新版名称" }),
      ).toBeInTheDocument(),
    );
    expect(screen.getByLabelText("工作说明")).toHaveValue("新版说明");
    await waitFor(() =>
      expect(
        screen.getByRole("button", { name: "保存工作说明" }),
      ).toBeEnabled(),
    );
    fireEvent.click(screen.getByRole("button", { name: "保存工作说明" }));
    await waitFor(() =>
      expect(ports.projects.writeContext).toHaveBeenCalledWith(
        { projectId: a, expectedRevision: 1 },
        "新版说明",
      ),
    );
    await waitFor(() => expect(contents.get(a)).toBe("新版说明"));
    expect(records[0].name).toBe("新版名称");
  });
  it("keeps both drafts when a save fails", async () => {
    const { ports, router } = fixture();
    ports.projects.update = vi.fn(async () => {
      throw "projects_revision_conflict";
    });
    render(<RouterProvider router={router} />);
    await screen.findByDisplayValue("A 的说明");
    fireEvent.change(screen.getAllByLabelText("项目名称").at(-1)!, {
      target: { value: "名称草稿" },
    });
    fireEvent.change(screen.getByLabelText("工作说明"), {
      target: { value: "说明草稿" },
    });
    fireEvent.click(
      screen.getAllByRole("button", { name: "保存名称" }).at(-1)!,
    );
    await screen.findByText(/项目已发生变化/);
    expect(screen.getAllByLabelText("项目名称").at(-1)).toHaveValue("名称草稿");
    expect(screen.getByLabelText("工作说明")).toHaveValue("说明草稿");
  });
  it("keeps project actions available with damaged context and recovers only after confirmation", async () => {
    const { ports, router } = fixture();
    ports.projects.getContext = vi.fn(async () => {
      throw "projects_context_changed";
    });
    ports.projects.writeContext = vi.fn(async (request, content) => ({
      projectId: request.projectId,
      projectRevision: request.expectedRevision + 1,
      content,
      directory: null,
      state: "materialized" as const,
      codexInstructions: null,
    }));
    render(<RouterProvider router={router} />);
    await screen.findByText(/工作说明文件无法读取或已在外部修改/);
    expect(screen.getByRole("button", { name: "归档项目" })).toBeEnabled();
    fireEvent.change(screen.getByLabelText("工作说明"), {
      target: { value: "恢复内容" },
    });
    fireEvent.click(screen.getByRole("button", { name: "重新建立工作说明" }));
    await screen.findByRole("dialog");
    expect(ports.projects.writeContext).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: "取消" }));
    await waitFor(() =>
      expect(screen.queryByRole("dialog")).not.toBeInTheDocument(),
    );
    expect(screen.getByLabelText("工作说明")).toHaveValue("恢复内容");
    fireEvent.click(screen.getByRole("button", { name: "重新建立工作说明" }));
    await screen.findByRole("dialog");
    fireEvent.click(screen.getByRole("button", { name: "确认" }));
    await waitFor(() =>
      expect(ports.projects.writeContext).toHaveBeenCalledWith(
        { projectId: a, expectedRevision: 0 },
        "恢复内容",
        true,
      ),
    );
  });

  it("selection only reads the new project and does not write context or global settings", async () => {
    const { ports, router } = fixture();
    render(<RouterProvider router={router} />);
    await screen.findByDisplayValue("A 的说明");
    fireEvent.click(screen.getByRole("button", { name: /项目 B 客户/ }));
    await screen.findByDisplayValue("B 的说明");
    expect(ports.projects.writeContext).not.toHaveBeenCalled();
    expect(
      screen.getByRole("button", { name: /启动项目 Agent/ }),
    ).toBeDisabled();
  });
  it("dirty project switch can be cancelled and never writes to B", async () => {
    const { ports, router } = fixture();
    render(<RouterProvider router={router} />);
    const input = await screen.findByLabelText("工作说明");
    fireEvent.change(input, { target: { value: "A 的未保存内容" } });
    fireEvent.click(screen.getByRole("button", { name: /项目 B 客户/ }));
    await screen.findByRole("dialog");
    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "取消" }));
    });
    await waitFor(() =>
      expect(screen.queryByRole("dialog")).not.toBeInTheDocument(),
    );
    expect(screen.getByLabelText("工作说明")).toHaveValue("A 的未保存内容");
    expect(router.state.location.search).toContain(a);
    expect(ports.projects.writeContext).not.toHaveBeenCalled();
  });
  it("stale native writes preserve draft and carry the original project revision", async () => {
    const { ports, router } = fixture();
    ports.projects.writeContext = vi.fn(async () => {
      throw "projects_revision_conflict";
    });
    render(<RouterProvider router={router} />);
    const input = await screen.findByLabelText("工作说明");
    fireEvent.change(input, { target: { value: "keep this draft" } });
    fireEvent.click(screen.getByRole("button", { name: "保存工作说明" }));
    await waitFor(() =>
      expect(ports.projects.writeContext).toHaveBeenCalledWith(
        { projectId: a, expectedRevision: 0 },
        "keep this draft",
      ),
    );
    await screen.findByText(/项目已发生变化/);
    expect(input).toHaveValue("keep this draft");
  });
});
