import { execFileSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";
import { expectedInstallerNames } from "../scripts/release/release-contract.mjs";

const ROOT = path.resolve(__dirname, "..");
const EXTERNAL_PLAN_MARKER = ["fyagent", "modernization", "plan"].join("-");
const LEGACY_REPOSITORY_SLUG = ["NongHua123", "fyagent"].join("/");
const HISTORICAL_RELEASE_NOTE_PREFIX = "docs/release-notes/v0.3.0-";
const HISTORICAL_TRELLIS_ARCHIVE_PREFIX = ".trellis/tasks/archive/";

const CURRENT_DEVELOPMENT_DOCS = [
  "docs/fyagent/development/README.md",
  "docs/fyagent/development/architecture/ownership.md",
  "docs/fyagent/development/ci-release/ci.md",
  "docs/fyagent/development/ci-release/release.md",
  "docs/fyagent/development/configuration/codex-provider.md",
  "docs/fyagent/development/configuration/workbuddy.md",
  "docs/fyagent/development/mise-tasks.md",
  "docs/fyagent/development/tooling/mise.md",
  "docs/fyagent/development/trellis/update-and-overlay.md",
  "docs/fyagent/development/validation.md",
  "docs/fyagent/development/windows/codex-desktop.md",
  "docs/fyagent/development/windows/installer.md",
] as const;

const CURRENT_BACKEND_OWNERS = [
  ".trellis/spec/backend/fyagent-version-contract.md",
  ".trellis/spec/backend/windows-installer.md",
  ".trellis/spec/backend/windows-runtime-security.md",
  ".trellis/spec/backend/codex-desktop-installer.md",
  ".trellis/spec/backend/codex-provider-configuration.md",
  ".trellis/spec/backend/workbuddy-configuration.md",
  ".trellis/spec/backend/github-ci-workflow.md",
  ".trellis/spec/backend/github-release-workflow.md",
  ".trellis/spec/backend/trellis-tooling.md",
] as const;

const RETIRED_BACKEND_OWNERS = [
  ".trellis/spec/backend/fyagent-v1-0-1-config-domains.md",
  ".trellis/spec/backend/windows-release-boundary.md",
] as const;

const LOCALIZED_INSTALLATION_GUIDES = [
  {
    file: "docs/user-manual/en/1-getting-started/1.2-installation.md",
    trustPatterns: [
      /not signed with an Apple Developer ID/,
      /not\s+notarized by Apple/,
      /Open Anyway/,
      /Do not disable Gatekeeper/,
      /remove the file's quarantine attribute/,
    ],
  },
  {
    file: "docs/user-manual/ja/1-getting-started/1.2-installation.md",
    trustPatterns: [
      /Apple Developer ID では署名されておらず/,
      /公証も受けていません/,
      /このまま開く[\s\S]{0,40}Open Anyway/,
      /Gatekeeper を無効にしたり/,
      /quarantine[\s\S]{0,40}削除したりしないで/,
    ],
  },
  {
    file: "docs/user-manual/zh/1-getting-started/1.2-installation.md",
    trustPatterns: [
      /未使用 Apple\s+Developer ID\s+签名/,
      /未经 Apple 公证/,
      /仍要打开[\s\S]{0,40}Open\s+Anyway/,
      /不要关闭 Gatekeeper/,
      /不要移除[\s\S]{0,40}quarantine/,
    ],
  },
] as const;

const PUBLIC_READMES = ["README.md", "README_JA.md", "README_ZH.md"] as const;

const CURRENT_PUBLIC_REPOSITORY_FILES = [
  ...PUBLIC_READMES,
  "CONTRIBUTING.md",
  "SECURITY.md",
  "SUPPORT.md",
  ".github/ISSUE_TEMPLATE/bug_report.yml",
  ".github/ISSUE_TEMPLATE/config.yml",
  ".github/ISSUE_TEMPLATE/doc_issue.yml",
  ".github/ISSUE_TEMPLATE/feature_request.yml",
  ".github/ISSUE_TEMPLATE/question.yml",
  "flatpak/com.fyagent.desktop.metainfo.xml",
] as const;

const INSTALLER_NAME_TEMPLATES = expectedInstallerNames("1.2.3").map((name) =>
  name.replace("1.2.3", "X.Y.Z"),
);

const MANUAL_LANGUAGES = ["en", "ja", "zh"] as const;
const EXPECTED_MANUAL_CHAPTERS = [
  "1-getting-started/1.1-introduction.md",
  "1-getting-started/1.2-installation.md",
  "1-getting-started/1.3-interface.md",
  "1-getting-started/1.4-quickstart.md",
  "1-getting-started/1.5-settings.md",
  "2-agent-tools/2.1-install.md",
  "2-agent-tools/2.2-update-diagnose.md",
  "3-providers/3.1-add.md",
  "3-providers/3.2-switch.md",
  "3-providers/3.3-edit.md",
  "3-providers/3.4-sort-duplicate.md",
  "3-providers/3.5-usage-query.md",
  "3-providers/3.6-claude-desktop.md",
  "4-extensions/4.1-mcp.md",
  "4-extensions/4.2-prompts.md",
  "4-extensions/4.3-skills.md",
  "4-extensions/4.4-sessions.md",
  "4-extensions/4.5-workspace.md",
  "4-extensions/4.6-workbuddy.md",
  "5-proxy/5.1-service.md",
  "5-proxy/5.2-routing.md",
  "5-proxy/5.3-failover.md",
  "5-proxy/5.4-usage.md",
  "5-proxy/5.5-model-test.md",
  "6-faq/6.1-config-files.md",
  "6-faq/6.2-questions.md",
  "6-faq/6.3-deeplink.md",
  "6-faq/6.4-env-conflict.md",
] as const;

const VISUAL_DELIVERABLES = [
  "docs/fyagent/audits/user-manual-screenshots.md",
  "docs/fyagent/marketing/visual-asset-plan.md",
  "docs/fyagent/marketing/prompts/README.md",
  "docs/fyagent/marketing/visual-direction-sample-v1.md",
  "docs/fyagent/marketing/visual-direction-sample-v2.md",
  "docs/fyagent/marketing/visual-direction-sample-v3.md",
  "docs/fyagent/marketing/vibekey-reference-audit.md",
  "docs/release-notes/README.md",
] as const;

function read(relative: string): string {
  return fs
    .readFileSync(path.join(ROOT, relative), "utf8")
    .replace(/\r\n/g, "\n");
}

function markdownFilesUnder(relativeDirectory: string): string[] {
  const files: string[] = [];
  const pending = [relativeDirectory];
  while (pending.length > 0) {
    const current = pending.pop();
    if (current === undefined) break;
    for (const entry of fs.readdirSync(path.join(ROOT, current), {
      withFileTypes: true,
    })) {
      const relative = path.posix.join(current, entry.name);
      if (entry.isDirectory()) {
        pending.push(relative);
      } else if (entry.isFile() && relative.endsWith(".md")) {
        files.push(relative);
      }
    }
  }
  return files.sort();
}

function currentAuthorityMarkdownFiles(): string[] {
  return [
    ...new Set([
      ...markdownFilesUnder("docs/fyagent/development"),
      ...markdownFilesUnder("docs/user-manual"),
      ...markdownFilesUnder(".trellis/spec"),
      ".agents/skills/fyagent-trellis/SKILL.md",
      "CONTRIBUTING.md",
      "README.md",
      "README_JA.md",
      "README_ZH.md",
      "flatpak/README.md",
    ]),
  ].sort();
}

function currentPublicRepositoryFiles(): string[] {
  return [
    ...new Set([
      ...CURRENT_PUBLIC_REPOSITORY_FILES,
      ...markdownFilesUnder("docs/user-manual"),
      ...markdownFilesUnder("docs/release-notes").filter(
        (file) => !file.startsWith(HISTORICAL_RELEASE_NOTE_PREFIX),
      ),
    ]),
  ].sort();
}

function markdownTargets(source: string): string[] {
  return [...source.matchAll(/\[[^\]]*\]\(([^)]+)\)/g)].map(
    (match) => match[1],
  );
}

function operationalTextFiles(): string[] {
  return execFileSync(
    "git",
    ["ls-files", "--cached", "--others", "--exclude-standard", "-z"],
    { cwd: ROOT, encoding: "utf8" },
  )
    .split("\0")
    .filter(
      (file) =>
        file.length > 0 &&
        !file.startsWith(".format-files-") &&
        !file.startsWith(HISTORICAL_TRELLIS_ARCHIVE_PREFIX) &&
        fs.existsSync(path.join(ROOT, file)) &&
        !fs.readFileSync(path.join(ROOT, file)).includes(0),
    );
}

describe("current FyAgent documentation authority", () => {
  it("removes versioned design packages and keeps one responsibility owner", () => {
    expect(fs.existsSync(path.join(ROOT, "docs/fyagent/dev"))).toBe(false);
    expect(markdownFilesUnder("docs/fyagent/development")).toEqual(
      [...CURRENT_DEVELOPMENT_DOCS].sort(),
    );
    for (const file of [
      ...CURRENT_DEVELOPMENT_DOCS,
      ...CURRENT_BACKEND_OWNERS,
    ]) {
      expect(fs.statSync(path.join(ROOT, file)).isFile(), file).toBe(true);
    }
    for (const file of RETIRED_BACKEND_OWNERS) {
      expect(fs.existsSync(path.join(ROOT, file)), file).toBe(false);
    }
  });

  it("keeps current authority free of old package and fixed-release routing", () => {
    for (const file of currentAuthorityMarkdownFiles()) {
      const source = read(file);
      expect(source, file).not.toContain("docs/fyagent/dev/");
      expect(source, file).not.toMatch(/\bv?0\.3\.0\b/);
      expect(source, file).not.toMatch(/\bv3\.16\.0\b/);
      expect(source, file).not.toContain("windows-release-boundary.md");
      expect(source, file).not.toContain("fyagent-v1-0-1-config-domains.md");
      expect(source, file).not.toContain("NongHua123/cc-switch");
      expect(source, file).not.toContain("解除锁定");
    }
    for (const file of markdownFilesUnder("docs/user-manual")) {
      expect(read(file), file).not.toMatch(/\bv3\.\d+(?:\.\d+)?\b/);
    }
  });

  it("preserves protocol, schema, third-party API, and toolchain versions", () => {
    expect(read(".trellis/spec/backend/deeplink-import-security.md")).toContain(
      "fyagent://v1/import",
    );
    const releaseOwners = `${read(
      ".trellis/spec/backend/fyagent-version-contract.md",
    )}\n${read(".trellis/spec/backend/github-release-workflow.md")}`;
    for (const schema of [
      "fyagent-download-manifest/v2",
      "fyagent-platform-build/v1",
      "fyagent-build-metadata/v1",
    ]) {
      expect(releaseOwners).toContain(schema);
    }
    expect(read(".trellis/spec/backend/workbuddy-configuration.md")).toContain(
      "/v1",
    );
    expect(read(".trellis/spec/backend/workbuddy-configuration.md")).toContain(
      "get_workbuddy_model_ids() -> WorkBuddyModelIdsResult",
    );
    const windowsRuntime = read(
      ".trellis/spec/backend/windows-runtime-security.md",
    );
    expect(windowsRuntime).toContain("canonical_sid");
    expect(windowsRuntime).not.toContain("canonical_user_sid");
    const environment = read(
      ".trellis/spec/backend/development-environment.md",
    );
    expect(environment).toContain("Node.js 24.19.0");
    expect(environment).toContain("Rust 1.97.1");
  });

  it("keeps localized installation guidance aligned with the release surface", () => {
    for (const file of [
      ...PUBLIC_READMES,
      ...LOCALIZED_INSTALLATION_GUIDES.map((guide) => guide.file),
    ]) {
      const source = read(file);
      for (const installer of INSTALLER_NAME_TEMPLATES) {
        expect(source, `${file} -> ${installer}`).toContain(installer);
      }
      expect(source, file).toContain("NSIS");
      expect(source, file).toMatch(/\bad-hoc\b/iu);
      expect(source, file).not.toMatch(
        /FyAgent-X\.Y\.Z-Windows(?:-(?:x64|arm64))?\.msi/i,
      );
      expect(source, file).not.toContain("FyAgent-X.Y.Z-Windows-Portable.zip");
    }
    for (const file of PUBLIC_READMES) {
      const source = read(file);
      for (const releaseEvidence of [
        "Developer ID",
        "NotSigned",
        "signing-status.json",
      ]) {
        expect(source, `${file} -> ${releaseEvidence}`).toContain(
          releaseEvidence,
        );
      }
    }
    for (const { file, trustPatterns } of LOCALIZED_INSTALLATION_GUIDES) {
      const source = read(file);
      expect(source, file).not.toContain("FyAgent-X.Y.Z-Linux-*");
      for (const trustPattern of trustPatterns) {
        expect(source, `${file} -> ${trustPattern.source}`).toMatch(
          trustPattern,
        );
      }
    }
  });

  it("keeps the six-chapter manual and visual evidence plan closed", () => {
    expect(fs.existsSync(path.join(ROOT, "README_DE.md"))).toBe(false);
    for (const file of PUBLIC_READMES) {
      expect(read(file), file).not.toContain("assets/screenshots/");
    }

    for (const language of MANUAL_LANGUAGES) {
      const prefix = `docs/user-manual/${language}/`;
      const chapters = markdownFilesUnder(prefix.slice(0, -1))
        .filter((file) => file !== `${prefix}README.md`)
        .map((file) => file.slice(prefix.length));
      expect(chapters, language).toEqual([...EXPECTED_MANUAL_CHAPTERS]);
      for (const retiredDirectory of [
        "2-providers",
        "3-extensions",
        "4-proxy",
        "5-faq",
      ]) {
        expect(
          fs.existsSync(path.join(ROOT, prefix, retiredDirectory)),
          `${language}/${retiredDirectory}`,
        ).toBe(false);
      }
    }

    const shotCards = markdownFilesUnder("docs/user-manual/assets/shot-cards");
    expect(shotCards).toHaveLength(16);
    expect(shotCards).toContain(
      "docs/user-manual/assets/shot-cards/001-main-overview.md",
    );
    expect(shotCards).toContain(
      "docs/user-manual/assets/shot-cards/015-failover-queue.md",
    );

    const imageDirectory = path.join(ROOT, "docs/user-manual/assets");
    const imageNames = fs
      .readdirSync(imageDirectory, { withFileTypes: true })
      .filter((entry) => entry.isFile() && entry.name.endsWith(".png"))
      .map((entry) => entry.name)
      .sort();
    expect(imageNames).toHaveLength(40);

    const imageReferences = MANUAL_LANGUAGES.flatMap((language) =>
      markdownFilesUnder(`docs/user-manual/${language}`).flatMap((file) =>
        [...read(file).matchAll(/\.\.\/\.\.\/assets\/([^\s)]+\.png)/g)].map(
          (match) => match[1],
        ),
      ),
    );
    expect(imageReferences).toHaveLength(84);
    expect([...new Set(imageReferences)].sort()).toEqual(imageNames);

    const audit = read("docs/fyagent/audits/user-manual-screenshots.md");
    for (const imageName of imageNames) {
      expect(audit, imageName).toContain(`\`${imageName}\``);
    }
    for (const file of VISUAL_DELIVERABLES) {
      expect(fs.statSync(path.join(ROOT, file)).isFile(), file).toBe(true);
    }
    expect(
      read("docs/fyagent/marketing/visual-direction-sample-v1.md"),
    ).toContain("status: superseded");
    expect(
      read("docs/fyagent/marketing/visual-direction-sample-v2.md"),
    ).toContain("status: superseded");
    expect(
      read("docs/fyagent/marketing/visual-direction-sample-v3.md"),
    ).toContain("status: concept_candidate");
  });

  it("keeps every local link in current authority resolvable", () => {
    for (const file of [
      ...new Set([...currentAuthorityMarkdownFiles(), ...VISUAL_DELIVERABLES]),
    ].sort()) {
      const source = read(file);
      for (const rawTarget of markdownTargets(source)) {
        if (/^(?:https?:|mailto:|#)/.test(rawTarget)) continue;
        const withoutAnchor = rawTarget.split("#", 1)[0];
        if (withoutAnchor.length === 0) continue;
        const decoded = decodeURIComponent(withoutAnchor.replace(/^<|>$/g, ""));
        const target = path.resolve(
          path.dirname(path.join(ROOT, file)),
          decoded,
        );
        expect(fs.existsSync(target), `${file} -> ${rawTarget}`).toBe(true);
      }
    }
  });

  it("keeps current public links independent of retired external authorities", () => {
    for (const file of operationalTextFiles()) {
      expect(read(file), file).not.toContain(EXTERNAL_PLAN_MARKER);
    }
    for (const file of currentPublicRepositoryFiles()) {
      expect(read(file), file).not.toContain(LEGACY_REPOSITORY_SLUG);
    }
  });
});
