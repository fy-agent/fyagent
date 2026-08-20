import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";

import { SecretInput } from "@/v2/shared/ui/SecretInput";

describe("SecretInput", () => {
  it("keeps the value hidden until the reusable toggle is pressed", async () => {
    const user = userEvent.setup();
    render(
      <label>
        令牌
        <SecretInput defaultValue="reusable-secret" />
      </label>,
    );

    const input = screen.getByLabelText("令牌");
    expect(input).toHaveAttribute("type", "password");
    expect(input).toHaveValue("reusable-secret");

    await user.click(screen.getByRole("button", { name: "显示" }));
    expect(input).toHaveAttribute("type", "text");
    expect(screen.getByRole("button", { name: "隐藏" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );

    await user.click(screen.getByRole("button", { name: "隐藏" }));
    expect(input).toHaveAttribute("type", "password");
  });
});
