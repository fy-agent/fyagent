import { execFileSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";
import { expectedInstallerNames } from "../scripts/release/release-contract.mjs";

const ROOT = path.resolve(__dirname, "..");
const EXTERNAL_PLAN_MARKER = ["fyagent", "modernization", "plan"].join("-");
const LEGACY_REPOSITORY_SLUG = ["NongHua123", "fyagent"].join("/");
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
] as const;

const RETIRED_BACKEND_OWNERS = [
  ".trellis/spec/backend/fyagent-v1-0-1-config-domains.md",
  ".trellis/spec/backend/windows-release-boundary.md",
] as const;

const LOCALIZED_INSTALLATION_GUIDES = [
  {
    file: "docs/user-manual/en/1-getting-started/1.2-installation.md",
    trustPatterns: [
      /signed with an Apple Developer ID/,
      /notarized by Apple/,
      /Open Anyway/,
      /Do not disable Gatekeeper/,
      /remove the file's quarantine attribute/,
    ],
  },
  {
    file: "docs/user-manual/ja/1-getting-started/1.2-installation.md",
    trustPatterns: [
      /Apple Developer ID で署名され/,
      /Apple の公証を受けています/,
      /このまま開く[\s\S]{0,40}Open Anyway/,
      /Gatekeeper を無効にしたり/,
      /quarantine[\s\S]{0,40}削除したりしないで/,
    ],
  },
  {
    file: "docs/user-manual/zh/1-getting-started/1.2-installation.md",
    trustPatterns: [
      /Apple\s+Developer ID\s+签名/,
      /Apple 公证/,
      /仍要打开[\s\S]{0,40}Open\s+Anyway/,
      /不要关闭 Gatekeeper/,
      /不要移除[\s\S]{0,40}quarantine/,
    ],
  },
] as const;

const PUBLIC_READMES = ["README.md", "README_EN.md", "README_JA.md"] as const;

const CURRENT_PUBLIC_REPOSITORY_FILES = [
  ...PUBLIC_READMES,
  "CONTRIBUTING.md",
  "SECURITY.md",
  "SUPPORT.md",
  ".github/DISCUSSION_TEMPLATE/ideas.yml",
  ".github/DISCUSSION_TEMPLATE/q-a.yml",
  ".github/DISCUSSION_TEMPLATE/show-and-tell.yml",
  ".github/ISSUE_TEMPLATE/bug_report.yml",
  ".github/ISSUE_TEMPLATE/config.yml",
  ".github/ISSUE_TEMPLATE/doc_issue.yml",
  ".github/ISSUE_TEMPLATE/feature_request.yml",
] as const;

const INSTALLER_NAME_TEMPLATES = expectedInstallerNames("1.2.3").map((name) =>
  name.replace("1.2.3", "X.Y.Z"),
);

const WINDOWS_CODEX_DESKTOP_DOC =
  "docs/fyagent/development/windows/codex-desktop.md";
const WINDOWS_INSTALLER_DOC = "docs/fyagent/development/windows/installer.md";
const VALIDATION_DOC = "docs/fyagent/development/validation.md";
const CODEX_INSTALLER_SPEC = ".trellis/spec/backend/codex-desktop-installer.md";
const PACKAGE_BRIDGE_ROOT =
  "FyAgent.PackageBridge-{96F39D37-0F42-486F-8C86-3631C12171C5}";
const LEGACY_MIGRATION_VERSION_COUNTS = new Map<string, number>([
  ["docs/fyagent/development/windows/installer.md", 1],
  [".trellis/spec/backend/windows-installer.md", 4],
]);

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
      "CONTRIBUTING.md",
      "README.md",
      "README_EN.md",
      "README_JA.md",
    ]),
  ].sort();
}

function currentPublicRepositoryFiles(): string[] {
  return [
    ...new Set([
      ...CURRENT_PUBLIC_REPOSITORY_FILES,
      ...markdownFilesUnder("docs/user-manual"),
      ...markdownFilesUnder("docs/release-notes"),
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
        !file.startsWith(".trellis/") &&
        !file.startsWith(".agents/") &&
        !file.startsWith(".codex/") &&
        !file.startsWith(".format-files-") &&
        !file.startsWith(HISTORICAL_TRELLIS_ARCHIVE_PREFIX) &&
        file !== "AGENTS.md" &&
        fs.existsSync(path.join(ROOT, file)) &&
        fs.statSync(path.join(ROOT, file)).isFile() &&
        !fs.readFileSync(path.join(ROOT, file)).includes(0),
    );
}

function trackedMarkdownFiles(): string[] {
  return execFileSync("git", ["ls-files", "--cached", "-z"], {
    cwd: ROOT,
    encoding: "utf8",
  })
    .split("\0")
    .filter((file) => file.toLowerCase().endsWith(".md"))
    .sort();
}

function hasConcreteWindowsProfilePath(source: string): boolean {
  const windowsProfileRoot =
    /(?:^|[^\p{L}\p{N}_])(?:[A-Za-z]:[\\/]+Users[\\/]+)/giu;
  for (const match of source.matchAll(windowsProfileRoot)) {
    const profileStart = (match.index ?? 0) + match[0].length;
    const remainder = source.slice(profileStart);
    if (!remainder.startsWith("<")) return true;
    const placeholderEnd = remainder.indexOf(">");
    const firstSeparator = remainder.search(/[\\/\r\n]/u);
    if (
      placeholderEnd <= 1 ||
      (firstSeparator >= 0 && placeholderEnd > firstSeparator) ||
      /[<>]/u.test(remainder.slice(1, placeholderEnd))
    ) {
      return true;
    }
  }
  return false;
}

describe("current FyAgent documentation authority", () => {
  it("keeps tracked Markdown free of concrete Windows profile paths", () => {
    const acceptedExamples = [
      String.raw`C:\Users\<username>\.codex`,
      String.raw`C:\Users\<用户名>\AppData\Roaming`,
      String.raw`C:\Users\<ユーザー名>\.fyagent`,
      String.raw`C:\Demo\screenshots`,
      String.raw`C:\ProgramData\FyAgent`,
      String.raw`C:\Program Files\FyAgent`,
      String.raw`D:\FyAgent-Acceptance`,
      String.raw`%USERPROFILE%\.fyagent`,
      "~/.fyagent",
    ];
    for (const example of acceptedExamples) {
      expect(
        hasConcreteWindowsProfilePath(example),
        "approved path example",
      ).toBe(false);
    }
    expect(
      hasConcreteWindowsProfilePath(
        String.raw`C:\Users\concrete-profile\.codex`,
      ),
      "concrete profile fixture",
    ).toBe(true);

    const files = trackedMarkdownFiles();
    expect(files.length).toBeGreaterThan(0);
    for (const file of files) {
      expect(hasConcreteWindowsProfilePath(read(file)), file).toBe(false);
    }
  });

  it("keeps README architecture, onboarding, and evidence semantics aligned", () => {
    for (const file of PUBLIC_READMES) {
      const source = read(file);
      const normalized = source.replace(/\s+/gu, " ");
      expect(source, `${file} -> architecture`).toMatch(
        /React\s*(?:\+|\/)\s*Vite/iu,
      );
      expect(source, `${file} -> IPC`).toMatch(/Tauri IPC/iu);
      expect(source, `${file} -> Rust`).toMatch(/Rust/iu);
      expect(source, `${file} -> SQLite`).toMatch(/SQLite/iu);
      expect(normalized, `${file} -> target-tool configuration`).toMatch(
        /(?:configuration writes[^.。]{0,40}target AI tools|target AI tools?[^.。]{0,40}configuration|目标 AI 工具[^.。]{0,40}配置|対象 AI ツール[^.。]{0,40}設定)/iu,
      );
      expect(normalized, `${file} -> local proxy`).toMatch(
        /(?:local proxy|本地代理|ローカルプロキシ)/iu,
      );
      expect(source, `${file} -> maintained development docs`).toContain(
        "docs/fyagent/development/README.md",
      );
      expect(normalized, `${file} -> WorkBuddy boundary`).toMatch(
        /(?:WorkBuddy[^.。\n]{0,120}(?:separate|independent|独立)|(?:separate|independent|独立)[^.。\n]{0,120}WorkBuddy)/iu,
      );

      expect(source, `${file} -> mise version`).toContain("mise >= 2026.8.6");
      const onboarding = [
        "mise trust",
        "mise run bootstrap",
        "mise run system:check",
        "mise run dev",
      ];
      const positions = onboarding.map((command) => source.indexOf(command));
      expect(
        positions.every((position) => position >= 0),
        file,
      ).toBe(true);
      expect(positions).toEqual(
        [...positions].sort((left, right) => left - right),
      );

      expect(source, `${file} -> current-host gate`).toContain(
        "mise run check",
      );
      expect(source, `${file} -> remote gate`).toContain("CI / Required");
      expect(normalized, `${file} -> formal release evidence`).toMatch(
        /(?:formal|正式(?:な)?)\s*Release/iu,
      );
      expect(source, `${file} -> HIL limit`).toMatch(/HIL/iu);
    }
  });

  it("keeps contribution topology and CODEOWNERS enforcement factual", () => {
    const contributing = read("CONTRIBUTING.md");
    const normalized = contributing.replace(/\s+/gu, " ");
    expect(
      contributing.match(/fy-agent\/fyagent/gu)?.length ?? 0,
    ).toBeGreaterThanOrEqual(2);
    expect(normalized).toMatch(/maintainer[^.]{0,160}origin/iu);
    expect(normalized).toMatch(/维护者[^。]{0,160}origin/u);
    expect(normalized).toMatch(/personal fork[^.]{0,160}origin/iu);
    expect(normalized).toMatch(/个人[^。]{0,80}fork[^。]{0,80}origin/iu);
    expect(normalized).toMatch(/canonical[^.]{0,160}fetch/iu);
    expect(normalized).toMatch(/CC Switch[^.]{0,160}fetch-only/iu);
    expect(
      contributing.match(/CI \/ Required/gu)?.length ?? 0,
    ).toBeGreaterThanOrEqual(2);
    expect(contributing.match(/squash/giu)?.length ?? 0).toBeGreaterThanOrEqual(
      2,
    );

    const codeowners = read(".github/CODEOWNERS");
    expect(codeowners).toMatch(/^\*\s+@\S+/mu);
    expect(codeowners).toMatch(/advisory/iu);
    expect(codeowners).toMatch(/branch protection/iu);
    expect(codeowners).toMatch(/Code Owner review/iu);
  });

  it("keeps the approved GitHub brand and discussion surface", () => {
    for (const file of PUBLIC_READMES) {
      const source = read(file);
      expect(source, file).toContain(
        'src="assets/brand/github/for-you-gate.svg"',
      );
      expect(source, file).toContain("discussions/categories/q-a");
      expect(source, file).not.toContain('src="assets/fyagent.png"');
    }
    expect(
      fs.existsSync(path.join(ROOT, ".github/ISSUE_TEMPLATE/question.yml")),
    ).toBe(false);
    for (const file of [
      ".github/DISCUSSION_TEMPLATE/ideas.yml",
      ".github/DISCUSSION_TEMPLATE/q-a.yml",
      ".github/DISCUSSION_TEMPLATE/show-and-tell.yml",
    ]) {
      expect(read(file), file).toContain("body:");
    }
    const expectedDescription =
      "Personal desktop control center for AI Workers and Agents";
    expect(JSON.parse(read("package.json")).description).toBe(
      expectedDescription,
    );
    expect(read("src-tauri/Cargo.toml")).toContain(
      `description = "${expectedDescription}"`,
    );
    const preview = fs.readFileSync(
      path.join(ROOT, "assets/brand/github/fyagent-social-preview.png"),
    );
    expect(preview.subarray(0, 8)).toEqual(
      Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]),
    );
    expect(preview.readUInt32BE(16)).toBe(1280);
    expect(preview.readUInt32BE(20)).toBe(640);
  });

  it("removes versioned design packages and keeps one responsibility owner", () => {
    expect(fs.existsSync(path.join(ROOT, "docs/fyagent/dev"))).toBe(false);
    const cargoVersion = read("src-tauri/Cargo.toml").match(
      /\[workspace\.package\]\s+version = "([0-9]+\.[0-9]+\.[0-9]+)"/u,
    )?.[1];
    expect(cargoVersion).toBeTruthy();
    const releaseNotes = markdownFilesUnder("docs/release-notes");
    const allowedCurrentNotes = new Set(
      ["en", "zh", "ja"].map(
        (language) => `docs/release-notes/v${cargoVersion}-${language}.md`,
      ),
    );
    const formalEnglishNote = `docs/release-notes/v${cargoVersion}-en.md`;
    expect(allowedCurrentNotes.has(formalEnglishNote)).toBe(true);
    expect(read(".github/workflows/release.yml")).toContain(
      "docs/release-notes/${RELEASE_TAG}-en.md",
    );
    expect(releaseNotes).toContain("docs/release-notes/README.md");
    for (const note of releaseNotes.filter(
      (file) => file !== "docs/release-notes/README.md",
    )) {
      expect(allowedCurrentNotes.has(note), note).toBe(true);
      expect(read(note).trim(), note).not.toBe("");
      expect(read("docs/release-notes/README.md"), note).toContain(
        `(${path.basename(note)})`,
      );
    }
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

  it("keeps private local evidence identifiers out of public documentation", () => {
    const localUserRoot = ["C:", "Users", ""].join("\\");
    const privateFingerprints = [
      [
        "9C54280E",
        "B1EB700800AB2022CEF32C392690ECB301D21DC7BBCB07A2BDE9F0C1",
      ].join(""),
      [
        "BD5DB121",
        "CEE6ACAEB0F3D706E2054ED7B54B492878E4BBAB85AD260F82B8FB86",
      ].join(""),
    ];
    for (const file of [
      "docs/fyagent/audits/vibekey-to-fyagent-capability-gap.md",
      "docs/fyagent/marketing/vibekey-reference-audit.md",
      "docs/fyagent/marketing/visual-direction-sample-v2.md",
    ]) {
      const source = read(file);
      expect(source, file).not.toContain(localUserRoot);
      for (const fingerprint of privateFingerprints) {
        expect(source, file).not.toContain(fingerprint);
      }
    }
    expect(
      read("docs/fyagent/marketing/visual-direction-sample-v2.md"),
    ).not.toContain(["Chat", "GPT built-in"].join(""));
  });

  it("documents the protected ProgramData A1 package bridge without an HTTP or NSIS fallback", () => {
    const codexDesktop = read(WINDOWS_CODEX_DESKTOP_DOC);
    const installer = read(WINDOWS_INSTALLER_DOC);
    const installerSpec = read(CODEX_INSTALLER_SPEC);
    const bridgeAuthority = `${codexDesktop}\n${installerSpec}`;

    for (const fixedBoundary of [
      "FOLDERID_ProgramData",
      PACKAGE_BRIDGE_ROOT,
      "installer.msix",
      "FYABRIDG",
      "UrlCreateFromPathW",
      "PathCreateFromUrlW",
      "AddPackageByUriAsync",
    ]) {
      expect(bridgeAuthority, fixedBoundary).toContain(fixedBoundary);
    }

    expect(bridgeAuthority).toMatch(
      /Hello[\s\S]+?bridge control[\s\S]+?Started[\s\S]+?admission[\s\S]+?progress[\s\S]+?(?:success|error)/iu,
    );
    expect(bridgeAuthority).toMatch(/protected(?:[- ]DACL|[^.\n]{0,24}ACL)/iu);
    expect(bridgeAuthority).toMatch(
      /(?:no|without)[^\n.]{0,100}HTTP[^\n.]{0,100}fallback/iu,
    );
    expect(bridgeAuthority).toMatch(
      /A1[\s\S]+?Windows 10[\s\S]+?Windows 11[\s\S]+?x64[\s\S]+?ARM64/iu,
    );
    expect(bridgeAuthority).toMatch(
      /(?:minimum supported Windows version|Windows support floor)[^\n.]{0,120}(?:does not change|unchanged|not raised)/iu,
    );
    expect(bridgeAuthority).toMatch(
      /A2[\s\S]+?future independent native validation[\s\S]+?explicit[\s\S]+?decision/iu,
    );
    expect(bridgeAuthority).toMatch(
      /A2[\s\S]+?(?:never a runtime fallback|runtime[^.]{0,120}never selects A2)/iu,
    );

    for (const [file, source] of [
      [WINDOWS_CODEX_DESKTOP_DOC, codexDesktop],
      [CODEX_INSTALLER_SPEC, installerSpec],
      [VALIDATION_DOC, read(VALIDATION_DOC)],
    ] as const) {
      const normalized = source.replace(/\s+/gu, " ");
      expect(normalized, file).toMatch(
        /(?:does not run|runs no|do not run)[^.]{0,100}HIL[^.]{0,120}(?:local|locally)[^.]{0,120}(?:Actions|GitHub Actions)/iu,
      );
      expect(normalized, file).toMatch(
        /static contract[^.]{0,160}Windows-target compilation checks[^.]{0,120}(?:code\/security )?review/iu,
      );
      expect(normalized, file).toMatch(
        /Windows 10(?:\/11|[^.]{0,30}Windows 11)/iu,
      );
      expect(normalized, file).toMatch(/x64\/ARM64/iu);
      expect(normalized, file).toMatch(
        /(?:Bob-elevated\/Alice|elevated-Bob\/(?:standard-)?Alice)/iu,
      );
      for (const [label, pattern] of [
        ["PackageManager", /PackageManager/iu],
        ["file URI", /file[- ]URI/iu],
        ["ACL", /ACL/iu],
        ["cleanup", /cleanup/iu],
      ] as const) {
        expect(normalized, `${file} -> ${label}`).toMatch(pattern);
      }
      expect(normalized, file).toMatch(/explicit,? unverified residual risk/iu);
      expect(normalized, file).toMatch(
        /(?:must not|cannot|prohibit|Do not treat)[^.]{0,160}native[- ]compatibility[^.]{0,160}native[- ]runtime/iu,
      );
    }

    for (const [file, source] of [
      [WINDOWS_CODEX_DESKTOP_DOC, codexDesktop],
      [CODEX_INSTALLER_SPEC, installerSpec],
    ] as const) {
      const normalized = source.replace(/\s+/gu, " ");
      expect(normalized, file).toMatch(
        /Before admission[^.]{0,320}structured error[^.]{0,120}PackageManager has not run/iu,
      );
      expect(normalized, file).toMatch(
        /After admission[^.]{0,420}invalid progress[^.]{0,100}terminal[^.]{0,160}(?:duplicate|extra data)[^.]{0,160}protocol\/transport[^.]{0,160}timeout[^.]{0,160}unclean close[^.]{0,200}(?:best-effort cancellation|best-effort cancel)[^.]{0,200}permanent process-lifetime quarantine/iu,
      );
      expect(normalized, file).toContain("Job remains `Installing`");
      expect(normalized, file).toMatch(
        /no terminal result is published to the renderer/iu,
      );
      expect(normalized, file).toMatch(
        /Only an authenticated valid terminal status[^.]{0,160}matching valid terminal frame[^.]{0,120}clean pipe close[^.]{0,80}(?:permit|cleanup)/iu,
      );
    }

    for (const retiredPositiveContract of [
      "FYAHHTTP",
      "exclusive numeric-loopback source",
      "one-operation HTTP source",
      "HTTP/1.1 `HEAD`/`GET`",
      "WinSock",
      "SO_EXCLUSIVEADDRUSE",
      "http://127.0.0.1",
    ]) {
      expect(bridgeAuthority, retiredPositiveContract).not.toContain(
        retiredPositiveContract,
      );
    }

    expect(installer).toContain(PACKAGE_BRIDGE_ROOT);
    expect(installer).toMatch(
      /NSIS[^\n.]{0,160}(?:does not|never)[^\n.]{0,120}(?:own|enumerate|repair|remove)[^\n.]{0,120}(?:PackageBridge|package bridge)/iu,
    );
    expect(installer).toMatch(
      /(?:application|bridge module)[^\n.]{0,120}(?:owns|owns both)[^\n.]{0,120}(?:cleanup|orphan)/iu,
    );
    expect(installer).toMatch(/%ProgramData%\\FyAgent\\runtime/iu);
    expect(installer).toMatch(
      /(?:separate|distinct|independent)[^\n.]{0,120}(?:PackageBridge|package bridge)[^\n.]{0,160}(?:retired|legacy)[^\n.]{0,80}runtime/iu,
    );
  });

  it("keeps current authority free of old package and fixed-release routing", () => {
    for (const file of currentAuthorityMarkdownFiles()) {
      const source = read(file);
      expect(source, file).not.toContain("docs/fyagent/dev/");
      expect(source.match(/\bv?0\.3\.0\b/gu) ?? [], file).toHaveLength(
        LEGACY_MIGRATION_VERSION_COUNTS.get(file) ?? 0,
      );
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
      "fyagent-download-manifest/v3",
      "fyagent-platform-build/v2",
      "fyagent-build-metadata/v2",
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
      expect(source, file).toMatch(/Developer ID/u);
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
      for (const trustPattern of trustPatterns) {
        expect(source, `${file} -> ${trustPattern.source}`).toMatch(
          trustPattern,
        );
      }
    }
  });

  it("uses Simplified Chinese as the repository homepage and keeps language links explicit", () => {
    const chinese = read("README.md");
    const english = read("README_EN.md");
    const japanese = read("README_JA.md");

    expect(chinese).toContain("<strong>For You Agent</strong>");
    expect(chinese).toContain('href="README_EN.md">English</a>');
    expect(english).toContain('href="README.md">简体中文</a>');
    expect(japanese).toContain('href="README_EN.md">English</a>');
    expect(japanese).toContain('href="README.md">简体中文</a>');
    expect(fs.existsSync(path.join(ROOT, "README_ZH.md"))).toBe(false);
  });

  it("keeps the six-chapter manual and visual evidence plan closed", () => {
    expect(fs.existsSync(path.join(ROOT, "README_DE.md"))).toBe(false);
    const readmeScreenshots = [
      "assets/screenshots/main-zh-1.png",
      "assets/screenshots/main-zh-2.png",
      "assets/screenshots/main-zh-3.png",
    ] as const;
    const retiredReadmeScreenshots = [
      "assets/screenshots/add-en.png",
      "assets/screenshots/add-ja.png",
      "assets/screenshots/add-zh.png",
      "assets/screenshots/main-en.png",
      "assets/screenshots/main-ja.png",
      "assets/screenshots/main-zh.png",
      "assets/screenshots/main-zh-4.png",
    ] as const;
    for (const image of readmeScreenshots) {
      expect(fs.statSync(path.join(ROOT, image)).isFile(), image).toBe(true);
    }
    for (const file of PUBLIC_READMES) {
      const source = read(file);
      for (const image of readmeScreenshots) {
        expect(source, `${file} -> ${image}`).toContain(image);
      }
      for (const image of retiredReadmeScreenshots) {
        expect(source, `${file} -> ${image}`).not.toContain(image);
      }
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
