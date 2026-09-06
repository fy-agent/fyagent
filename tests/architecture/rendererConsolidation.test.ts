import fs from "node:fs";
import path from "node:path";
import { execFileSync } from "node:child_process";
import { describe, expect, it } from "vitest";
import { QUICK_SETUP_PROVIDER_IDS } from "@/pages/models/quickSetup";
import { CHANGE_PLAN_CONTRACT_VERSION } from "@/shared/features/change-plans";

const root = process.cwd();
const read = (file: string) => fs.readFileSync(path.join(root, file), "utf8");
const tracked = execFileSync("git", ["ls-files", "-z"], {
  cwd: root,
  encoding: "utf8",
  maxBuffer: 8 * 1024 * 1024,
})
  .split("\0")
  .filter(Boolean);

describe("one product renderer", () => {
  it("retains one ordinary module entry and no offline HTML delivery", () => {
    const entry = read("src/index.html");
    expect(entry).toContain('type="module" src="./main.tsx"');
    expect(entry.match(/<script\b/g)).toHaveLength(1);
    expect(entry).not.toMatch(
      /file:|location\.replace|location\.assign|build-v2/,
    );
    expect(tracked).toContain("src/main.tsx");
    expect(tracked).toContain("tests/browser/startup.spec.ts");
    for (const retired of [
      "deplink.html",
      "FyAgent-前端交互预览.html",
      "scripts/build-v2-preview.mjs",
      "scripts/preview-html.mjs",
      "tsconfig.v2.json",
      "vitest.v2.config.ts",
      "eslint.v2.config.mjs",
      "playwright.v2.config.ts",
    ])
      expect(tracked, retired).not.toContain(retired);
    expect(
      tracked.filter((file) =>
        /^(?:src\/v\d+\/|tests\/v\d+(?:-browser)?\/)/.test(file),
      ),
    ).toEqual([]);
  });

  it("runs both adopted unit environments from one canonical task", () => {
    const manifest = JSON.parse(read("package.json")) as {
      scripts: Record<string, string>;
    };
    expect(manifest.scripts["test:unit"]).toBe(
      "node --throw-deprecation ./node_modules/vitest/vitest.mjs run",
    );
    expect(
      Object.keys(manifest.scripts).filter((name) => /:v\d+(?::|$)/.test(name)),
    ).toEqual([]);
    const config = read("vitest.config.ts");
    expect(config).toContain('name: "renderer"');
    expect(config).toContain('name: "contracts"');
    expect(config).toContain("tests/renderer/");
    expect(manifest.scripts["build:renderer"]).toContain(
      "scripts/verify-route-chunks.mjs",
    );
    expect(manifest.scripts["build:renderer"]).not.toMatch(
      /preview|standalone/,
    );
  });

  it("does not confuse persisted identifiers and native protocol versions with source roles", () => {
    expect(QUICK_SETUP_PROVIDER_IDS).toEqual({
      claude: "fyagent-v2-quick-setup-claude",
      codex: "fyagent-v2-quick-setup-codex",
      grokbuild: "fyagent-v2-quick-setup-grokbuild",
    });
    expect(CHANGE_PLAN_CONTRACT_VERSION).toBe("fyagent-change-plan/v2");
    expect(
      read("src/domain/configuration/presets/codexProviderPresets.ts"),
    ).toContain('displayName: "MiMo V2.5 Pro"');
  });

  it("does not send users to retired generator downloads", () => {
    for (const language of ["zh", "en", "ja"]) {
      for (const doc of ["3-providers/3.1-add.md", "6-faq/6.3-deeplink.md"]) {
        expect(read(`docs/user-manual/${language}/${doc}`)).not.toContain(
          "deplink.html",
        );
      }
    }
  });
});
