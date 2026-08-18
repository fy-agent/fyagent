import { describe, expect, it } from "vitest";

import { createBrowserWindowFramePort } from "../../../src/v2/shared/platform/browser/windowFrame";

describe("browser WindowFramePort", () => {
  it("exposes the frozen browser identity and safe no-op methods", async () => {
    const port = createBrowserWindowFramePort();

    expect(port.isNative).toBe(false);
    expect(port.platform).toBe("browser");

    await expect(
      Promise.all([
        port.prepareFrame(),
        port.minimize(),
        port.toggleMaximize(),
        port.close(),
      ]),
    ).resolves.toEqual([undefined, undefined, undefined, undefined]);
  });
});
