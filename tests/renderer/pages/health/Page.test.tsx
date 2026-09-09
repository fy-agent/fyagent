import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { createMemoryRouter, RouterProvider } from "react-router-dom";
import { describe, expect, it, vi } from "vitest";
import { HealthPage } from "@/pages/health/Page";
import { FeatureProvider } from "@/shared/features/provider";
import { createBrowserFeaturePorts } from "@/shared/platform/browser/features";
import { healthSnapshotFixture } from "../../fixtures/health";

function setup(initialEntry = "/health?agent=codex") {
  const ports = createBrowserFeaturePorts();
  ports.health.get = vi.fn(async (id) =>
    healthSnapshotFixture(
      id,
      id === "claude-code"
        ? {
            auth: {
              state: "attention",
              severity: "warning",
              reasonCode: "auth_logged_out",
              action: "authentication",
            },
          }
        : {},
    ),
  );
  const router = createMemoryRouter(
    [
      {
        path: "/health",
        element: (
          <FeatureProvider ports={ports}>
            <HealthPage />
          </FeatureProvider>
        ),
      },
      { path: "*", element: <p>处理页面</p> },
    ],
    { initialEntries: [initialEntry] },
  );
  render(<RouterProvider router={router} />);
  return { ports, router, user: userEvent.setup() };
}

describe("HealthPage", () => {
  it("renders all twelve local checks, times and a separate request action", async () => {
    const { ports, router, user } = setup();
    const detail = screen.getByRole("region", { name: "Codex 运行状态" });
    await within(detail).findByText("本机检查正常");
    expect(detail.querySelectorAll("[data-check-id]")).toHaveLength(12);
    expect(detail.querySelectorAll("time")).toHaveLength(13);
    expect(
      within(detail).getByText(/尚无此软件的本机代理请求记录/),
    ).toBeVisible();
    await user.click(
      within(detail).getByRole("button", {
        name: "前往模型测试：最近一次请求",
      }),
    );
    await waitFor(() => expect(router.state.location.pathname).toBe("/models"));
    expect(router.state.location.search).toBe("?target=codex");
    expect(ports.health.get).toHaveBeenCalledTimes(1);
  });

  it("filters by name and status, keeps failures explicit and preserves a closed selected target", async () => {
    const { ports, user } = setup("/health?agent=claude-code");
    const detail = screen.getByRole("region", { name: "Claude Code 运行状态" });
    await within(detail).findByRole("button", { name: "查看登录：登录状态" });
    expect(ports.health.get).toHaveBeenCalledWith("claude-code");
    await user.type(
      screen.getByRole("searchbox", { name: "搜索软件" }),
      "does-not-exist",
    );
    await screen.findByText("没有匹配的软件");
    await user.click(screen.getByRole("button", { name: "清除筛选" }));
    await user.selectOptions(
      screen.getByRole("combobox", { name: "筛选状态" }),
      "needs_attention",
    );
    expect(
      screen
        .getByRole("region", { name: "软件运行状态" })
        .querySelectorAll("[role=listitem]"),
    ).toHaveLength(1);
    ports.health.get = vi.fn(async () => {
      throw new Error("raw secret");
    });
    await user.click(
      within(detail).getByRole("button", { name: "重新检查此软件" }),
    );
    await within(detail).findByText(
      "本次读取失败，以下保留上次结果。请重新检查。",
    );
    expect(
      within(detail).getAllByText("本机记录显示未登录，请先登录。"),
    ).toHaveLength(2);
    expect(screen.queryByText(/raw secret/)).not.toBeInTheDocument();
  });

  it("never dispatches an untrusted URL value", async () => {
    const { ports } = setup(
      "/health?agent=https%3A%2F%2Fexample.test&returnUrl=javascript%3Avoid(0)",
    );
    await within(
      screen.getByRole("region", { name: "Codex 运行状态" }),
    ).findByText("本机检查正常");
    expect(ports.health.get).toHaveBeenCalledWith("codex");
    expect(ports.health.get).toHaveBeenCalledTimes(1);
  });
});
