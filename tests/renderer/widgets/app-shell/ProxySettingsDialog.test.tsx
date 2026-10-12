import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { FeatureProvider } from "@/shared/features/provider";
import { createBrowserFeaturePorts } from "@/shared/platform/browser/features";
import { TopBar } from "@/widgets/app-shell/TopBar";

async function fixture() {
  const ports = createBrowserFeaturePorts();
  let config = {
    proxyEnabled: false,
    listenAddress: "127.0.0.1",
    listenPort: 15721,
    enableLogging: true,
  };
  let key: string | null = null;
  ports.settings.getAppVersion = vi.fn(async () => "0.4.11");
  ports.settings.getProxyConfig = vi.fn(async () => ({ ...config }));
  ports.settings.getProxyAccessKey = vi.fn(async () => key);
  ports.settings.saveProxyConfig = vi.fn(async (value) => {
    config = value;
    key ??= "first-access-key";
  });
  ports.settings.regenerateProxyAccessKey = vi.fn(async () => {
    key = "new-access-key";
    return key;
  });
  const user = userEvent.setup();
  const copy = vi.spyOn(navigator.clipboard, "writeText").mockResolvedValue();
  render(
    <FeatureProvider ports={ports}>
      <TopBar />
    </FeatureProvider>,
  );
  expect(ports.settings.getProxyAccessKey).not.toHaveBeenCalled();
  await user.click(screen.getByRole("button", { name: "关于 FyAgent" }));
  await act(async () => {
    await vi.dynamicImportSettled();
  });
  await user.click(await screen.findByRole("button", { name: "代理设置" }));
  await act(async () => {
    await vi.dynamicImportSettled();
  });
  await screen.findByRole("dialog", { name: "代理设置" });
  await waitFor(() =>
    expect(screen.getByLabelText("监听地址")).toHaveValue("127.0.0.1"),
  );
  return { ports, user, copy };
}

describe("Proxy settings", () => {
  it("reads back the generated key after saving LAN listening, copies and rotates it, then clears secrets on close", async () => {
    const { ports, user, copy } = await fixture();
    await user.clear(screen.getByLabelText("监听地址"));
    await user.type(screen.getByLabelText("监听地址"), "0.0.0.0");
    await user.click(screen.getByRole("button", { name: "保存并重启服务" }));
    await screen.findByText("代理设置已保存。");
    expect(ports.settings.saveProxyConfig).toHaveBeenCalledWith({
      proxyEnabled: false,
      listenAddress: "0.0.0.0",
      listenPort: 15721,
      enableLogging: true,
    });
    const secret = screen.getByLabelText("局域网访问密钥");
    expect(secret).toHaveValue("first-access-key");
    expect(secret).toHaveAttribute("type", "password");
    await user.click(screen.getByRole("button", { name: "显示访问密钥" }));
    expect(secret).toHaveAttribute("type", "text");
    await user.click(screen.getByRole("button", { name: "复制密钥" }));
    expect(copy).toHaveBeenLastCalledWith("first-access-key");
    await user.click(screen.getByRole("button", { name: "重新生成密钥" }));
    await screen.findByText("新密钥已保存，旧密钥已失效。请更新局域网客户端。");
    expect(secret).toHaveValue("new-access-key");
    await user.click(screen.getByRole("button", { name: "复制密钥" }));
    expect(copy).toHaveBeenLastCalledWith("new-access-key");
    expect(screen.getByText(/Authorization: Bearer/)).toHaveTextContent(
      "x-api-key",
    );
    await user.click(screen.getByRole("button", { name: "关闭" }));
    expect(screen.queryByLabelText("局域网访问密钥")).not.toBeInTheDocument();
  });

  it("keeps the current key when regeneration fails and shows a safe error", async () => {
    const { ports, user } = await fixture();
    ports.settings.regenerateProxyAccessKey = vi.fn(async () => {
      throw new Error("secret vendor error");
    });
    await user.click(screen.getByRole("button", { name: "重新生成密钥" }));
    await screen.findByText("无法重新生成密钥，请重试。");
    expect(screen.getByLabelText("局域网访问密钥")).toHaveValue("");
    expect(screen.queryByText(/secret vendor error/)).not.toBeInTheDocument();
  });
});
