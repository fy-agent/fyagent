import {
  fireEvent,
  render,
  screen,
  waitFor,
  act,
  within,
} from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useEffect, useState } from "react";
import { createMemoryRouter, RouterProvider } from "react-router-dom";
import { describe, expect, it, vi } from "vitest";
import { ProjectsPage, type ProjectsPageProps } from "@/pages/projects/Page";
import type { Project, ProjectDependencySnapshot } from "@/domain/projects";
import { FeatureProvider } from "@/shared/features/provider";
import { PrimaryBlockerProvider } from "@/shared/ui/PrimaryBlocker";
import { createBrowserFeaturePorts } from "@/shared/platform/browser/features";
import { usePersistentVisibility } from "@/shared/ui/PersistentSurface";

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
function fixture(pageProps: ProjectsPageProps = {}, initialProject = a) {
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
              <ProjectsPage {...pageProps} />
            </PrimaryBlockerProvider>
          </FeatureProvider>
        ),
      },
    ],
    { initialEntries: [`/projects?project=${initialProject}`] },
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
    fireEvent.click(screen.getByText("项目设置"));
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
    fireEvent.click(screen.getByText("项目设置"));
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
    fireEvent.click(screen.getByText("项目设置"));
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
      screen.queryByRole("button", { name: /启动项目 Agent/ }),
    ).not.toBeInTheDocument();
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

  it("creates a project under its selected customer from the visible entry", async () => {
    const { ports, router, records } = fixture({}, "");
    ports.projects.create = vi.fn(async (customerId, name) => {
      const next = { ...project(a, name), customerId };
      records.splice(0, records.length, next);
      return next;
    });
    render(<RouterProvider router={router} />);
    await screen.findByRole("button", { name: /项目 A 客户/ });
    fireEvent.click(screen.getByRole("button", { name: "新建项目" }));
    const form = screen.getByRole("form", { name: "新建项目" });
    fireEvent.change(within(form).getByLabelText("所属客户"), {
      target: { value: customer },
    });
    fireEvent.change(within(form).getByLabelText("项目名称"), {
      target: { value: "客户周报" },
    });
    fireEvent.click(within(form).getByRole("button", { name: "创建项目" }));
    await screen.findByRole("heading", { name: "客户周报" });
    expect(ports.projects.create).toHaveBeenCalledWith(customer, "客户周报");
    expect(
      screen.queryByRole("form", { name: "新建项目" }),
    ).not.toBeInTheDocument();
  });

  it("starts from an empty customer list without requiring a separate customer screen", async () => {
    const { ports, router, records } = fixture({}, "");
    records.length = 0;
    const customers: {
      customerId: string;
      name: string;
      revision: number;
      archived: boolean;
    }[] = [];
    ports.projects.listCustomers = vi.fn(async () => customers);
    ports.projects.createCustomer = vi.fn(async (name) => {
      const record = {
        customerId: customer,
        name,
        revision: 0,
        archived: false,
      };
      customers.push(record);
      return record;
    });
    ports.projects.create = vi.fn(async (customerId, name) => {
      const next = { ...project(a, name), customerId };
      records.push(next);
      return next;
    });
    render(<RouterProvider router={router} />);
    await screen.findByText("还没有项目。从一个具体的客户需求开始。");
    fireEvent.click(screen.getByRole("button", { name: "创建第一个项目" }));
    const form = screen.getByRole("form", { name: "新建项目" });
    fireEvent.change(within(form).getByLabelText("客户名称"), {
      target: { value: "星河商贸" },
    });
    fireEvent.change(within(form).getByLabelText("项目名称"), {
      target: { value: "经营周报" },
    });
    fireEvent.click(within(form).getByRole("button", { name: "创建项目" }));
    await screen.findByRole("heading", { name: "经营周报" });
    expect(ports.projects.createCustomer).toHaveBeenCalledTimes(1);
    expect(ports.projects.create).toHaveBeenCalledWith(customer, "经营周报");
  });

  it("keeps context and handoff drafts across workspaces and pauses hidden panel work", async () => {
    const user = userEvent.setup();
    const readEvidence = vi.fn();
    function HandoffDraft() {
      const visible = usePersistentVisibility();
      const [draft, setDraft] = useState("");
      useEffect(() => {
        if (visible) readEvidence();
      }, [visible]);
      return (
        <label>
          交接草稿
          <input value={draft} onChange={(e) => setDraft(e.target.value)} />
        </label>
      );
    }
    const { ports, router } = fixture({ VerificationPanel: HandoffDraft });
    render(<RouterProvider router={router} />);
    const context = await screen.findByLabelText("工作说明");
    fireEvent.change(context, { target: { value: "待保存的项目范围" } });
    expect(readEvidence).not.toHaveBeenCalled();
    await user.click(screen.getByRole("tab", { name: "验证与交接" }));
    expect(context).not.toBeVisible();
    expect(readEvidence).toHaveBeenCalledTimes(1);
    const handoff = screen.getByLabelText("交接草稿");
    fireEvent.change(handoff, { target: { value: "下周交给小周" } });
    await user.click(screen.getByRole("tab", { name: "交付方案" }));
    expect(handoff).not.toBeVisible();
    expect(readEvidence).toHaveBeenCalledTimes(1);
    await user.click(screen.getByRole("tab", { name: "项目准备" }));
    expect(screen.getByLabelText("工作说明")).toBe(context);
    expect(context).toHaveValue("待保存的项目范围");
    await user.click(screen.getByRole("tab", { name: "验证与交接" }));
    expect(screen.getByLabelText("交接草稿")).toBe(handoff);
    expect(handoff).toHaveValue("下周交给小周");
    expect(ports.projects.writeContext).not.toHaveBeenCalled();
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });

  it("shows individual changed resource states and usable Codex directory guidance", async () => {
    const { ports, router, records } = fixture();
    const resource = {
      kind: "skill" as const,
      agentId: "codex",
      rawId: "weekly",
      model: null,
      pinnedVersion: "saved",
    };
    records[0].resources = [resource];
    records[0].codexPrepared = true;
    ports.projects.resourceOptions = vi.fn(async () => [
      { ...resource, label: "周报整理", version: "current" },
    ]);
    ports.projects.getContext = vi.fn(async (projectId) => ({
      projectId,
      projectRevision: 0,
      content: "A 的说明",
      state: "materialized" as const,
      directory: "/tmp/project-a",
      codexInstructions: "请打开本目录。",
    }));
    const snapshot = await ports.projects.dependencySnapshot(a);
    ports.projects.dependencySnapshot = vi.fn(async () => ({
      ...snapshot,
      contextState: "materialized" as const,
      resources: [
        { resource, observedVersion: "current", state: "drifted" as const },
      ],
    }));
    render(<RouterProvider router={router} />);
    await screen.findByDisplayValue("A 的说明");
    fireEvent.click(screen.getByText("资源与账号"));
    expect(await screen.findByText("来源已变化")).toBeVisible();
    expect(screen.getByText(/在 Codex 中选择下方目录继续工作/)).toBeVisible();
    expect(screen.getByRole("button", { name: "复制工作目录" })).toBeEnabled();
    expect(
      screen.queryByText(/资源版本暂无法完整确认/),
    ).not.toBeInTheDocument();
  });
});
