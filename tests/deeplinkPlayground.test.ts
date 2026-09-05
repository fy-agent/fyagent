import fs from "node:fs";
import { JSDOM } from "jsdom";
import { TextDecoder, TextEncoder } from "node:util";
import { describe, expect, it, vi } from "vitest";

const html = fs.readFileSync("deplink.html", "utf8");

function inspect(parameters: Record<string, string>) {
  const alert = vi.fn();
  const dom = new JSDOM(html, {
    url: "https://playground.example.test/",
    runScripts: "dangerously", // Only the reviewed local document executes; inputs are set afterward.
    beforeParse(window) {
      Object.assign(window, { TextDecoder, TextEncoder });
      window.alert = alert;
      window.HTMLElement.prototype.scrollIntoView = vi.fn();
    },
  });
  const input = dom.window.document.getElementById(
    "parseUrl",
  ) as HTMLInputElement;
  input.value = `fyagent://v1/import?${new URLSearchParams(parameters)}`;
  dom.window.parseDeepLink();
  expect(alert).not.toHaveBeenCalled();
  return dom;
}

describe("offline deep-link inspector", () => {
  it("ships explicit placeholders in raw and encoded Context7 examples", () => {
    const dom = new JSDOM(html);
    try {
      expect(html).not.toMatch(/ctx7sk-[a-f0-9-]{36}/i);
      const links = [
        ...dom.window.document.querySelectorAll<HTMLAnchorElement>(
          'a[href^="fyagent:"]',
        ),
      ];
      const configs = links
        .map((link) => new URL(link.href).searchParams)
        .filter(
          (params) => params.get("resource") === "mcp" && params.has("config"),
        )
        .map((params) =>
          JSON.parse(
            Buffer.from(params.get("config")!, "base64").toString("utf8"),
          ),
        );
      const examples = configs.filter((config) => config.mcpServers?.context7);
      expect(examples.length).toBeGreaterThan(0);
      for (const config of examples) {
        const args: string[] = config.mcpServers.context7.args;
        expect(args[args.indexOf("--api-key") + 1]).toBe(
          "REPLACE_WITH_YOUR_CONTEXT7_API_KEY",
        );
      }
    } finally {
      dom.window.close();
    }
  });
  it("treats parameters, decoded JSON and scripts as text, never markup", () => {
    const marker =
      '<img data-untrusted="yes" src="x"><script>window.injected = true;</script>';
    const config = {
      env: {
        NORMAL: `${marker}-configuration`,
        API_KEY: "test-only-placeholder",
      },
    };
    const dom = inspect({
      resource: "provider",
      app: "claude",
      name: marker,
      notes: marker,
      endpoint: `${marker},https://api.example.test`,
      configFormat: marker,
      config: Buffer.from(JSON.stringify(config)).toString("base64"),
      usageEnabled: "true",
      usageScript: Buffer.from(`${marker}-usage-script`).toString("base64url"),
    });
    try {
      const result = dom.window.document.getElementById("parseResult")!;
      expect(result.textContent).toContain(marker);
      expect(result.textContent).toContain(`${marker}-configuration`);
      expect(result.textContent).toContain(`${marker}-usage-script`);
      expect(result.textContent).not.toContain("配置文件解析失败");
      expect(result.querySelector("img, script")).toBeNull();
      expect(dom.window.injected).toBeUndefined();
      expect(result.textContent).toContain("备用 1");
      expect(result.querySelector("details summary")?.textContent).toContain(
        "原始 JSON",
      );
    } finally {
      dom.window.close();
    }
  });

  it("masks short credentials completely and replaces earlier results", () => {
    const dom = inspect({
      resource: "provider",
      apiKey: "short-secret",
      name: "first",
    });
    try {
      const result = dom.window.document.getElementById("parseResult")!;
      expect(result.textContent).not.toContain("short-secret");
      expect(result.textContent).toContain("****");
      (
        dom.window.document.getElementById("parseUrl") as HTMLInputElement
      ).value = "fyagent://v1/import?name=second&config=invalid";
      dom.window.parseDeepLink();
      expect(result.textContent).toContain("second");
      expect(result.textContent).not.toContain("first");
      expect(result.textContent).toContain("配置文件解析失败");
      expect(result.querySelector("img, script")).toBeNull();
    } finally {
      dom.window.close();
    }
  });
});
