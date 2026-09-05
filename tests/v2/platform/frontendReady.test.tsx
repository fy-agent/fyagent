import { act, render, screen, waitFor } from "@testing-library/react";
import { lazy, Suspense } from "react";
import { describe, expect, it, vi } from "vitest";

import { useFrontendReady } from "@/v2/shared/platform/useFrontendReady";
import { PersistentSurface } from "@/v2/shared/ui/PersistentSurface";
import { RootError } from "@/v2/app/RootError";
import { initialPrimaryPageId } from "@/v2/app/primaryPages";

const { signal } = vi.hoisted(() => ({ signal: vi.fn(async () => undefined) }));
vi.mock("@/v2/shared/platform/lifecycle", () => ({
  signalFrontendReady: signal,
}));

function Surface({ ready = true }: { ready?: boolean }) {
  useFrontendReady(ready);
  return <main>{ready ? "usable content" : "local snapshot pending"}</main>;
}

describe("frontend presentation readiness", () => {
  it("awaits marked local artwork, ignores remote artwork and rejects stale completion", async () => {
    let release!: () => void;
    const decoded = new Promise<void>((resolve) => {
      release = resolve;
    });
    const remoteDecode = vi.fn(() => new Promise<void>(() => undefined));
    function Illustrated({ ready }: { ready: boolean }) {
      useFrontendReady(ready);
      return (
        <>
          <img
            alt="local icon"
            data-fy-startup-image=""
            src="/local.png"
            ref={(image) => {
              if (image) image.decode = vi.fn(() => decoded);
            }}
          />
          <img
            alt="remote icon"
            data-fy-startup-image=""
            src="https://external.example.test/icon.png"
            ref={(image) => {
              if (image) image.decode = remoteDecode;
            }}
          />
        </>
      );
    }
    const view = render(<Illustrated ready />);
    expect(signal).not.toHaveBeenCalled();
    expect(remoteDecode).not.toHaveBeenCalled();
    view.rerender(<Illustrated ready={false} />);
    await act(async () => {
      release();
      await decoded;
    });
    expect(signal).not.toHaveBeenCalled();
    view.rerender(<Illustrated ready />);
    await waitFor(() => expect(signal).toHaveBeenCalledTimes(1));
  });

  it("does not block usable content on failed decorative image decoding", async () => {
    render(
      <>
        <img
          alt="broken"
          data-fy-startup-image=""
          src="/broken.png"
          ref={(image) => {
            if (image)
              image.decode = vi
                .fn()
                .mockRejectedValue(new Error("decode failed"));
          }}
        />
        <Surface />
      </>,
    );
    await waitFor(() => expect(signal).toHaveBeenCalledTimes(1));
  });
  it("does not signal from a suspended or hidden page", async () => {
    let release!: (value: { default: typeof Surface }) => void;
    const module = new Promise<{ default: typeof Surface }>((resolve) => {
      release = resolve;
    });
    const Page = lazy(() => module);
    const tree = (active: boolean) => (
      <PersistentSurface active={active}>
        <Suspense fallback={<span>chunk pending</span>}>
          <Page />
        </Suspense>
      </PersistentSurface>
    );
    const view = render(tree(false));
    expect(signal).not.toHaveBeenCalled();
    await act(async () => {
      release({ default: Surface });
      await module;
    });
    expect(signal).not.toHaveBeenCalled();
    view.rerender(tree(true));
    await waitFor(() => expect(signal).toHaveBeenCalledTimes(1));
    expect(screen.getByText("usable content")).toBeVisible();
  });

  it("waits for local snapshot settlement, not animation frames or native visibility", async () => {
    const frame = vi
      .spyOn(window, "requestAnimationFrame")
      .mockImplementation(() => 1);
    const view = render(<Surface ready={false} />);
    expect(signal).not.toHaveBeenCalled();
    view.rerender(<Surface />);
    await waitFor(() => expect(signal).toHaveBeenCalledTimes(1));
    expect(frame).not.toHaveBeenCalled();
  });

  it("makes a module failure presentable with an explicit reload action", async () => {
    render(<RootError />);
    expect(screen.getByRole("alert")).toHaveTextContent("页面暂时无法打开");
    expect(screen.getByRole("button", { name: "重新加载界面" })).toBeEnabled();
    await waitFor(() => expect(signal).toHaveBeenCalledTimes(1));
  });

  it.each([
    ["", "agents"],
    ["#/auth?consumer=codex", "auth"],
    ["#/models?target=codex", "models"],
    ["#/skills", "skills"],
    ["https://outside.test", "agents"],
    ["#/../auth", "agents"],
  ])("selects only a closed initial route from %s", (hash, expected) => {
    expect(initialPrimaryPageId(hash)).toBe(expected);
  });
});
