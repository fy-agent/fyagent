import { describe, expect, it } from "vitest";
import { skillUpdateErrorMessage } from "@/shared/features/skills";

describe("Skills linked path refusal", () => {
  it("explains a native read-only refusal without leaking a source path", () => {
    const message = skillUpdateErrorMessage(
      new Error(
        JSON.stringify({
          code: "SKILL_LINK_READ_ONLY",
          context: {},
          suggestion: "useOrdinaryDirectory",
        }),
      ),
    );
    expect(message).toContain("此操作未执行");
    expect(message).toContain("只读");
    expect(message).toContain("不会写入或删除");
    expect(message).toContain("普通目录");
  });

  it("does not treat arbitrary text or unrelated codes as link authority", () => {
    expect(
      skillUpdateErrorMessage("SKILL_LINK_READ_ONLY C:\\private"),
    ).toBeUndefined();
    expect(
      skillUpdateErrorMessage(JSON.stringify({ code: "OTHER" })),
    ).toBeUndefined();
  });
});
