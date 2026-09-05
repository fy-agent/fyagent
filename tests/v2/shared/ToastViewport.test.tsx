import { render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { ToastViewport } from "@/v2/shared/ui/ToastViewport";

describe("toast presentation ownership", () => {
  it("announces current notifications without inventing actions or a second expiry timer", async () => {
    const message = { id: 1, title: "配置已保存", tone: "success" as const };
    const { rerender } = render(<ToastViewport messages={[message]} />);
    expect(screen.getByRole("status")).toHaveTextContent("配置已保存");
    expect(screen.queryByRole("button")).toBeNull();
    rerender(<ToastViewport messages={[]} />);
    await waitFor(() => expect(screen.queryByRole("status")).toBeNull());
    await waitFor(() => expect(document.querySelector(".fy-toast")).toBeNull());
  });
});
