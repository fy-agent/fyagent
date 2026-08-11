import { spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { afterAll, describe, expect, it } from "vitest";
import {
  EXPECTED_INSTALLERS_BY_TARGET,
  EXPECTED_TARGETS,
  expectedInstallerNames,
} from "../scripts/release/release-contract.mjs";
// @ts-expect-error The release workflow executes this dependency-free helper directly.
import * as pinnedInputsModule from "../scripts/release/pin-release-build-inputs.mjs";

const ROOT = path.resolve(__dirname, "..");
const RELEASE_WORKFLOW = path.join(ROOT, ".github", "workflows", "release.yml");
const CI_WORKFLOW = path.join(ROOT, ".github", "workflows", "ci.yml");
const CARGO_TOML = path.join(ROOT, "src-tauri", "Cargo.toml");
const TAURI_CONFIG = path.join(ROOT, "src-tauri", "tauri.conf.json");
const RELEASE_NOTES_DIR = path.join(ROOT, "docs", "release-notes");
const BUILD_RS = path.join(ROOT, "src-tauri", "build.rs");
const TEST_MANIFEST = path.join(
  ROOT,
  "src-tauri",
  "windows",
  "fyagent-test.manifest",
);
const RELEASE_MANIFEST = path.join(
  ROOT,
  "src-tauri",
  "windows",
  "fyagent-release.manifest",
);
const TAURI_WINDOWS_CONFIG = path.join(
  ROOT,
  "src-tauri",
  "tauri.windows.conf.json",
);
const NSIS_TEMPLATE = path.join(ROOT, "src-tauri", "nsis", "installer.nsi");
const NSIS_CONTRACT = path.join(
  ROOT,
  "scripts",
  "release",
  "verify-windows-nsis-contract.mjs",
);
const NSIS_LIFECYCLE = path.join(
  ROOT,
  "scripts",
  "release",
  "verify-windows-nsis-lifecycle.ps1",
);
const WINDOWS_MANIFEST_VERIFIER = path.join(
  ROOT,
  "scripts",
  "release",
  "verify-windows-release-manifest.ps1",
);
const WINDOWS_SIGNING = path.join(
  ROOT,
  "scripts",
  "release",
  "windows-signing.mjs",
);
const WINDOWS_SIGNING_EVIDENCE = path.join(
  ROOT,
  "scripts",
  "release",
  "windows-signing-evidence.ps1",
);
const MACOS_ADHOC_VERIFIER = path.join(
  ROOT,
  "scripts",
  "release",
  "verify-macos-adhoc-app.sh",
);
const PLATFORM_METADATA_WRITER = path.join(
  ROOT,
  "scripts",
  "release",
  "write-platform-metadata.mjs",
);
const RELEASE_CONTRACT = path.join(
  ROOT,
  "scripts",
  "release",
  "release-contract.mjs",
);
const RELEASE_CONTRACT_TYPES = path.join(
  ROOT,
  "scripts",
  "release",
  "release-contract.d.mts",
);
const AUTO_LAUNCH = path.join(ROOT, "src-tauri", "src", "auto_launch.rs");
const LIB_RS = path.join(ROOT, "src-tauri", "src", "lib.rs");
const temporaryRoots: string[] = [];

const createTrustedBuildInputs =
  pinnedInputsModule.createTrustedBuildInputs as (options: {
    inputRoot: string;
    outputRoot: string;
    version: string;
    sourceSha: string;
  }) => Promise<unknown>;
const verifyTrustedBuildInputs =
  pinnedInputsModule.verifyTrustedBuildInputs as (options: {
    root: string;
    version: string;
    sourceSha: string;
  }) => Promise<unknown>;

function read(file: string): string {
  return fs.readFileSync(file, "utf8").replace(/\r\n/g, "\n");
}

function workflowJobBlock(source: string, job: string, nextJob: string) {
  const start = source.indexOf(`\n  ${job}:\n`);
  const end = source.indexOf(`\n  ${nextJob}:\n`);
  expect(start, job).toBeGreaterThanOrEqual(0);
  expect(end, nextJob).toBeGreaterThan(start);
  return source.slice(start, end);
}

function namedStepBlock(source: string, name: string) {
  const start = source.indexOf(`\n      - name: ${name}\n`);
  expect(start, name).toBeGreaterThanOrEqual(0);
  const end = source.indexOf("\n      - name:", start + 1);
  return source.slice(start, end < 0 ? source.length : end);
}

function expectExactLine(source: string, line: string) {
  expect(
    source.split(/\r?\n/).filter((candidate) => candidate === line),
  ).toEqual([line]);
}

const EXPECTED_RELEASE_JOB_IDS = [
  "eligibility",
  "build-windows",
  "prove-windows-preflight",
  "sign-windows-formal",
  "seal-windows-formal",
  "build-linux",
  "build-macos",
  "pin-release-build-inputs",
  "verify-assets",
  "attest",
  "publish",
] as const;

const ATTEST_JOB_IF_LINE = "    if: ${{ !cancelled() }}";
const PUBLISH_JOB_IF_LINE =
  "    if: ${{ !cancelled() && github.event_name == 'push' && needs.eligibility.result == 'success' && needs.eligibility.outputs.release_mode == 'formal' && needs.attest.result == 'success' }}";
const ATTEST_PREREQUISITE_STEP = `      - name: Require successful attestation prerequisites
        shell: bash
        env:
          ELIGIBILITY_RESULT: \${{ needs.eligibility.result }}
          VERIFY_ASSETS_RESULT: \${{ needs['verify-assets'].result }}
        run: |
          set -euo pipefail
          if [ "$ELIGIBILITY_RESULT" != "success" ] || [ "$VERIFY_ASSETS_RESULT" != "success" ]; then
            echo "Attestation prerequisites were not successful: eligibility=$ELIGIBILITY_RESULT verify-assets=$VERIFY_ASSETS_RESULT" >&2
            exit 1
          fi`;
const RAW_WINDOWS_SETUP_ICON_GATE_STEP = `- name: Verify raw Windows setup embeds the canonical FyAgent icon
        shell: pwsh
        run: |
          $ErrorActionPreference = 'Stop'
          node scripts/release/verify-windows-setup-icon.mjs \`
            $env:FYAGENT_WINDOWS_RAW_ASSET \`
            src-tauri/icons/icon.ico
          if ($LASTEXITCODE -ne 0) {
            throw "Raw Windows setup icon verification failed with exit code $LASTEXITCODE"
          }`;
const SEALED_WINDOWS_SETUP_ICON_GATE_LINES = [
  '          node scripts/release/verify-windows-setup-icon.mjs "installers/FyAgent-$APP_VERSION-Windows-x64-setup.exe" src-tauri/icons/icon.ico',
  '          node scripts/release/verify-windows-setup-icon.mjs "installers/FyAgent-$APP_VERSION-Windows-arm64-setup.exe" src-tauri/icons/icon.ico',
] as const;

type TailJobResult = "failure" | "skipped" | "success";

type ReleaseTailGateInput = {
  cancelled: boolean;
  eligibilityResult: TailJobResult;
  eventName: "push" | "workflow_dispatch";
  mode: "formal" | "preflight";
  verifyAssetsResult: TailJobResult;
};

function releaseTailGateOutcome(input: ReleaseTailGateInput) {
  const attestRuns = !input.cancelled;
  const attestResult: TailJobResult = !attestRuns
    ? "skipped"
    : input.eligibilityResult === "success" &&
        input.verifyAssetsResult === "success"
      ? "success"
      : "failure";
  const publishRuns =
    !input.cancelled &&
    input.eventName === "push" &&
    input.eligibilityResult === "success" &&
    input.mode === "formal" &&
    attestResult === "success";

  return { attestResult, attestRuns, publishRuns };
}

function assertReleaseTailStatusGates(workflow: string) {
  const attest = workflowJobBlock(workflow, "attest", "publish");
  const publish = workflow.slice(workflow.indexOf("\n  publish:\n"));

  const exactLineCount = (block: string, line: string) =>
    block.split("\n").filter((candidate) => candidate === line).length;
  if (exactLineCount(attest, ATTEST_JOB_IF_LINE) !== 1) {
    throw new Error("attest must have exactly one explicit !cancelled() gate");
  }
  if (exactLineCount(publish, PUBLISH_JOB_IF_LINE) !== 1) {
    throw new Error(
      "publish must bind !cancelled(), formal push, eligibility success, and attestation success",
    );
  }

  const stepsIndex = attest.indexOf("\n    steps:\n");
  const firstStepIndex = attest.indexOf("\n      - name:", stepsIndex);
  const prerequisiteIndex = attest.indexOf(
    "\n      - name: Require successful attestation prerequisites\n",
    stepsIndex,
  );
  if (stepsIndex < 0 || firstStepIndex !== prerequisiteIndex) {
    throw new Error(
      "attest must fail closed on direct needs in its first step",
    );
  }
  if (!attest.includes(ATTEST_PREREQUISITE_STEP)) {
    throw new Error(
      "attest prerequisite step must require eligibility and verify-assets success",
    );
  }
}

function assertWindowsSetupIconGates(workflow: string) {
  const windowsBuild = workflowJobBlock(
    workflow,
    "build-windows",
    "prove-windows-preflight",
  );
  const rawIconGate = namedStepBlock(
    windowsBuild,
    "Verify raw Windows setup embeds the canonical FyAgent icon",
  );
  if (rawIconGate.trim() !== RAW_WINDOWS_SETUP_ICON_GATE_STEP) {
    throw new Error(
      "each raw Windows setup must pass the exact fail-closed canonical PE icon gate",
    );
  }

  const verify = workflowJobBlock(workflow, "verify-assets", "attest");
  const aggregate = namedStepBlock(
    verify,
    "Verify exact ten and generate three machine-readable subjects",
  );
  const aggregateLines = aggregate.split("\n");
  if (
    (aggregate.match(/verify-windows-setup-icon\.mjs/gu) ?? []).length !== 2 ||
    aggregateLines.filter((line) => line === "          set -euo pipefail")
      .length !== 1 ||
    SEALED_WINDOWS_SETUP_ICON_GATE_LINES.some(
      (requiredLine) =>
        aggregateLines.filter((line) => line === requiredLine).length !== 1,
    ) ||
    /(?:\|\||;)\s*(?:true|:)\b|\bset\s+\+(?:e|o\s+errexit)\b/iu.test(aggregate)
  ) {
    throw new Error(
      "both sealed Windows setups must pass exact fail-closed canonical PE icon gates before attestation",
    );
  }
  if (
    (workflow.match(/scripts\/release\/verify-windows-setup-icon\.mjs/gu) ?? [])
      .length !== 3
  ) {
    throw new Error(
      "Release must invoke the Windows setup PE icon verifier exactly three times",
    );
  }
}

function releaseWorkflowJobIds(workflow: string): string[] {
  const jobsStart = workflow.indexOf("\njobs:\n");
  if (jobsStart < 0) {
    throw new Error("release workflow has no jobs mapping");
  }

  // GitHub job IDs may start with a letter or underscore and may otherwise
  // contain alphanumeric characters, hyphens, and underscores. YAML permits
  // those keys in plain, single-quoted, or double-quoted form. Fail closed on
  // any other direct jobs key syntax instead of silently omitting an escaped
  // or inline YAML spelling from the topology comparison.
  const jobIds: string[] = [];
  for (const line of workflow.slice(jobsStart).split("\n")) {
    if (!/^  \S/u.test(line) || /^  #/u.test(line)) continue;
    const match =
      /^  (?:(?<plain>[A-Za-z_][A-Za-z0-9_-]*)|'(?<single>[A-Za-z_][A-Za-z0-9_-]*)'|"(?<double>[A-Za-z_][A-Za-z0-9_-]*)"):$/u.exec(
        line,
      );
    if (!match) {
      throw new Error(`unsupported Release job key syntax: ${line.trim()}`);
    }
    jobIds.push(
      String(
        match.groups?.plain ?? match.groups?.single ?? match.groups?.double,
      ),
    );
  }
  return jobIds;
}

function releaseWorkflowRunScripts(workflow: string): string[] {
  const lines = workflow.split("\n");
  const scripts: string[] = [];
  const jobsIndex = lines.indexOf("jobs:");
  if (jobsIndex < 0) {
    throw new Error("release workflow has no jobs mapping");
  }
  const allowedInlineScripts = new Set([
    "pnpm install --frozen-lockfile",
    "node scripts/release/verify-windows-nsis-contract.mjs",
    "pnpm tauri build --bundles appimage,deb,rpm --verbose",
    "pnpm tauri build --target universal-apple-darwin --bundles app",
    'node scripts/release/verify-release-files.mjs subjects verified-subjects "$APP_VERSION"',
  ]);

  for (let index = 0; index < lines.length; index += 1) {
    if (
      index > jobsIndex &&
      /^ {6}-\s/u.test(lines[index]) &&
      !/^ {6}- name:\s+\S/u.test(lines[index])
    ) {
      throw new Error(
        `unsupported Release step sequence item syntax: ${lines[index].trim()}`,
      );
    }

    const directMapping = /^ {8}\S/u.test(lines[index]);
    const canonicalDirectMapping =
      /^ {8}(?:[A-Za-z_][A-Za-z0-9_-]*|'[A-Za-z_][A-Za-z0-9_-]*'|"[A-Za-z_][A-Za-z0-9_-]*")\s*:/u.test(
        lines[index],
      );
    if (
      directMapping &&
      lines[index].includes(":") &&
      !canonicalDirectMapping
    ) {
      throw new Error(
        `unsupported Release direct mapping key syntax: ${lines[index].trim()}`,
      );
    }

    const run = /^(\s*)(?:run|'run'|"run")\s*:\s*(.*)$/u.exec(lines[index]);
    if (!run) continue;

    const scalar = run[2];
    const blockScalar =
      /^[|>](?:(?:[+-][1-9]?)|(?:[1-9][+-]?))?(?:[ \t]+#.*)?[ \t]*$/u.test(
        scalar,
      );
    if (!blockScalar) {
      if (/^[|>]/u.test(scalar)) {
        throw new Error(
          `unsupported Release run block scalar syntax: ${lines[index].trim()}`,
        );
      }
      const inlineScript = scalar.trim();
      if (!allowedInlineScripts.has(inlineScript)) {
        throw new Error(
          `unsupported Release inline run scalar: ${lines[index].trim()}`,
        );
      }
      scripts.push(inlineScript);
      continue;
    }

    const indentation = run[1].length;
    const scriptLines: string[] = [];
    for (index += 1; index < lines.length; index += 1) {
      const line = lines[index];
      if (
        line.trim() !== "" &&
        line.length - line.trimStart().length <= indentation
      ) {
        index -= 1;
        break;
      }
      scriptLines.push(line);
    }
    scripts.push(scriptLines.join("\n"));
  }

  return scripts;
}

function assertReleaseWorkflowDoesNotExecuteInstallers(workflow: string) {
  const jobIds = releaseWorkflowJobIds(workflow);
  if (JSON.stringify(jobIds) !== JSON.stringify(EXPECTED_RELEASE_JOB_IDS)) {
    throw new Error(`unexpected Release job topology: ${jobIds.join(", ")}`);
  }

  const allowedPowerShellCallOperators = [
    /^\s*\$evidenceJson\s*=\s*&\s+\.\/scripts\/release\/windows-signing-evidence\.ps1\s+`\s*$/u,
    /^\s*&\s+node\s+@commonArguments\s+--mode\s+unsigned\s*$/u,
    /^\s*&\s+node\s+@commonArguments\s+`\s*$/u,
  ];
  const forbiddenRunScriptLaunches = [
    {
      name: "Windows lifecycle diagnostic",
      pattern: /verify-windows-nsis-lifecycle\.ps1/iu,
    },
    {
      name: "Start-Process",
      pattern: /(^\s*|[;|]\s*)Start-Process\b/iu,
    },
    {
      name: "cmd command shell",
      pattern: /^\s*(?:&\s*)?cmd(?:\.exe)?\s+\/c\b/iu,
    },
    {
      name: "dynamic variable command",
      pattern:
        /^\s*\$(?:env:)?[A-Za-z_][A-Za-z0-9_]*\s+(?:["']?\/S\b|["']?\/D=|["']?_\?=)/iu,
    },
    {
      name: "direct executable command",
      pattern: /^\s*(?:["'][^"'\r\n]+\.exe["']|[^\s#"']+\.exe)(?:\s|$)/iu,
    },
  ];

  for (const script of releaseWorkflowRunScripts(workflow)) {
    for (const line of script.split("\n")) {
      for (const { name, pattern } of forbiddenRunScriptLaunches) {
        if (pattern.test(line)) {
          throw new Error(
            `Release workflow must not execute installers via ${name}: ${line.trim()}`,
          );
        }
      }

      if (
        /(^|[=\s])&\s+/u.test(line) &&
        !allowedPowerShellCallOperators.some((pattern) => pattern.test(line))
      ) {
        throw new Error(
          `Release workflow contains a non-allowlisted PowerShell call operator: ${line.trim()}`,
        );
      }
    }
  }

  if (/verify-windows-nsis-lifecycle\.ps1/iu.test(workflow)) {
    throw new Error(
      "Release workflow must not reference the Windows lifecycle diagnostic",
    );
  }
}

function createBuildInputFixture(version: string) {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "fyagent-release-pin-"));
  temporaryRoots.push(root);
  const installerNames = expectedInstallerNames(version);
  const artifacts: Array<
    | { name: string; files: readonly number[] }
    | { name: string; metadata: string }
  > = [
    ...(["windows-x64", "windows-arm64"] as const).map((target) => ({
      name: `raw-${target}`,
      files: EXPECTED_INSTALLERS_BY_TARGET[target],
    })),
    ...(["macos-universal", "linux-x64", "linux-arm64"] as const).map(
      (target) => ({
        name: `installers-${target}`,
        files: EXPECTED_INSTALLERS_BY_TARGET[target],
      }),
    ),
    ...EXPECTED_TARGETS.map(({ targetGroup }) => ({
      name: `metadata-${targetGroup}`,
      metadata: `${targetGroup}.json`,
    })),
  ];
  for (const artifact of artifacts) {
    const artifactRoot = path.join(root, artifact.name);
    fs.mkdirSync(artifactRoot);
    if ("files" in artifact) {
      for (const index of artifact.files) {
        const name = installerNames[index];
        fs.writeFileSync(
          path.join(artifactRoot, name),
          `fixture:${artifact.name}:${name}`,
        );
      }
    } else {
      fs.writeFileSync(
        path.join(artifactRoot, artifact.metadata),
        `${JSON.stringify({ artifact: artifact.name })}\n`,
      );
    }
  }
  return root;
}

function runMacAdhocVerifier(mode: string) {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "fyagent-macos-adhoc-"));
  temporaryRoots.push(root);
  const binRoot = path.join(root, "bin");
  const appPath = path.join(root, "FyAgent.app");
  const callLog = path.join(root, "codesign.log");
  fs.mkdirSync(binRoot);
  fs.mkdirSync(appPath);
  fs.writeFileSync(
    path.join(binRoot, "codesign"),
    `#!/usr/bin/env bash
set -euo pipefail
printf '%s\\n' "$*" >> "$FYAGENT_FAKE_CODESIGN_LOG"
if [ "$1" = '--display' ]; then
  printf '%s\\n' \\
    'Executable=FyAgent' \\
    'Identifier=com.fyagent.desktop' \\
    "CodeDirectory v=20400 size=1 flags=$([ "$FYAGENT_FAKE_MODE" = linker ] && printf '0x20002(adhoc,linker-signed)' || printf '0x2(adhoc)') hashes=1+0 location=embedded" \\
    'CMSDigest=0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef' \\
    'CMSDigestType=2' \\
    'Signature=adhoc' \\
    "$([ "$FYAGENT_FAKE_MODE" = team ] && printf 'TeamIdentifier=ABCDE12345' || printf 'TeamIdentifier=not set')" \\
    "$([ "$FYAGENT_FAKE_MODE" = unsealed ] && printf 'Sealed Resources=none' || printf 'Sealed Resources version=2 rules=13 files=4')"
  [ "$FYAGENT_FAKE_MODE" = authority ] && printf '%s\\n' 'Authority=Developer ID Application: Example'
  [ "$FYAGENT_FAKE_MODE" = timestamp ] && printf '%s\\n' 'Timestamp=10 Aug 2026 at 00:00:00'
  exit 0
fi
if [ "$1" = '--verify' ]; then
  [ "$FYAGENT_FAKE_MODE" != verify-fail ]
  exit
fi
exit 2
`,
    { mode: 0o755 },
  );
  fs.writeFileSync(
    path.join(binRoot, "xcrun"),
    `#!/usr/bin/env bash
[ "$FYAGENT_FAKE_MODE" = stapled ]
`,
    { mode: 0o755 },
  );
  const result = spawnSync("bash", [MACOS_ADHOC_VERIFIER, appPath], {
    encoding: "utf8",
    env: {
      ...process.env,
      FYAGENT_FAKE_CODESIGN_LOG: callLog,
      FYAGENT_FAKE_MODE: mode,
      PATH: `${binRoot}:${process.env.PATH ?? ""}`,
    },
  });
  return {
    ...result,
    calls: fs.existsSync(callLog) ? read(callLog).trim().split("\n") : [],
  };
}

afterAll(() => {
  for (const root of temporaryRoots) {
    fs.rmSync(root, { force: true, recursive: true });
  }
});

describe("FyAgent release workflow", () => {
  const source = read(RELEASE_WORKFLOW);
  const platformMetadataWriter = read(PLATFORM_METADATA_WRITER);
  const releaseContract = read(RELEASE_CONTRACT);
  const releaseContractTypes = read(RELEASE_CONTRACT_TYPES);
  const windowsManifestVerifier = read(WINDOWS_MANIFEST_VERIFIER);

  it("ships non-empty English notes for the canonical release version", () => {
    const canonicalVersion = read(CARGO_TOML).match(
      /\[workspace\.package\]\s+version = "([0-9]+\.[0-9]+\.[0-9]+)"/u,
    )?.[1];
    expect(canonicalVersion).toBeTruthy();
    const notes = read(
      path.join(RELEASE_NOTES_DIR, `v${canonicalVersion}-en.md`),
    );

    expect(notes).toContain(`# FyAgent v${canonicalVersion}`);
    for (const installer of expectedInstallerNames(canonicalVersion!)) {
      expect(notes, installer).toContain(installer);
    }
    for (const evidence of [
      "download-manifest.json",
      "build-metadata.json",
      "signing-status.json",
      "artifact-attestation.sigstore.json",
    ]) {
      expect(notes, evidence).toContain(evidence);
    }
    expect(notes).toContain("14 attachments total");
    expect(notes).toContain("13 subjects");
    expect(notes).toContain("dev/laiyongjie");
    expect(notes).toContain("NotSigned");
    expect(notes).toMatch(/Developer\s+ID/u);
    expect(notes).toContain("not notarized");
    expect(notes).toMatch(/universal app is ad-hoc signed/iu);
    expect(notes).toContain("DMG container is unsigned");
    for (const retiredInstaller of [
      `FyAgent-${canonicalVersion}-Windows.msi`,
      `FyAgent-${canonicalVersion}-Windows-arm64.msi`,
    ]) {
      expect(notes, retiredInstaller).not.toContain(retiredInstaller);
    }
    expect(notes).not.toMatch(/FyAgent-[^\s`]*Windows[^\s`]*\.msi/iu);
  });

  it("pins every pre-signer build input by exact file identity", async () => {
    const version = "12.34.56";
    const sourceSha = "0123456789abcdef0123456789abcdef01234567";
    const inputRoot = createBuildInputFixture(version);
    const outputRoot = path.join(
      inputRoot,
      "..",
      `${path.basename(inputRoot)}-trusted`,
    );
    temporaryRoots.push(outputRoot);

    await createTrustedBuildInputs({
      inputRoot,
      outputRoot,
      version,
      sourceSha,
    });
    await expect(
      verifyTrustedBuildInputs({ root: outputRoot, version, sourceSha }),
    ).resolves.toBeTruthy();

    await expect(
      verifyTrustedBuildInputs({
        root: outputRoot,
        version,
        sourceSha: "1123456789abcdef0123456789abcdef01234567",
      }),
    ).rejects.toThrow(/manifest does not exactly bind/u);

    const rawPath = path.join(
      outputRoot,
      "raw-windows-x64",
      expectedInstallerNames(version)[2],
    );
    const rawBytes = fs.readFileSync(rawPath);
    fs.appendFileSync(rawPath, "tampered");
    await expect(
      verifyTrustedBuildInputs({ root: outputRoot, version, sourceSha }),
    ).rejects.toThrow(/manifest does not exactly bind/u);
    fs.writeFileSync(rawPath, rawBytes);

    const metadataPath = path.join(
      outputRoot,
      "metadata-linux-x64",
      "linux-x64.json",
    );
    const metadataBytes = fs.readFileSync(metadataPath);
    fs.rmSync(metadataPath);
    await expect(
      verifyTrustedBuildInputs({ root: outputRoot, version, sourceSha }),
    ).rejects.toThrow(/must contain exactly 1 files/u);
    fs.writeFileSync(metadataPath, metadataBytes);

    const unknownPath = path.join(outputRoot, "unknown.txt");
    fs.writeFileSync(unknownPath, "unknown");
    await expect(
      verifyTrustedBuildInputs({ root: outputRoot, version, sourceSha }),
    ).rejects.toThrow(/Unexpected trusted build input entry/u);
  });

  it("supports an immutable dev preflight and stable tag candidates without publishing dispatches", () => {
    const trigger = source.slice(0, source.indexOf("\npermissions:"));
    expect(trigger).toContain('      - "v*.*.*"');
    expect(trigger).not.toContain('      - "v*"');
    expect(trigger).not.toMatch(/^\s+- ["']v\d+\.\d+\.\d+["']\s*$/mu);
    expect(trigger).toContain("workflow_dispatch:");
    expect(trigger).toContain("source_sha:");
    expect(trigger).toContain("dev/laiyongjie HEAD SHA");
    expect(trigger).toContain("required: true");
    expect(source).toContain("release_mode='preflight'");
    expect(source).toContain("release_mode='formal'");
    expect(source).toContain(PUBLISH_JOB_IF_LINE);
    expect(source).not.toContain(
      "if: github.event_name == 'workflow_dispatch' && needs.eligibility.outputs.release_mode == 'formal'",
    );
    expect(source).not.toContain("gh release create");
    expect(source).toContain("draft:true,prerelease:false");
    expect(source).toContain("draft:false,prerelease:false");
  });

  it("makes attestation and formal publication status propagation explicit and fail-closed", () => {
    expect(() => assertReleaseTailStatusGates(source)).not.toThrow();

    const mutations = [
      {
        name: "implicit attest success propagation",
        workflow: source.replace(
          ATTEST_JOB_IF_LINE,
          "    if: ${{ success() }}",
        ),
      },
      {
        name: "missing verify-assets direct-needs result",
        workflow: source.replace(
          "          VERIFY_ASSETS_RESULT: ${{ needs['verify-assets'].result }}",
          '          VERIFY_ASSETS_RESULT: "success"',
        ),
      },
      {
        name: "publish without an explicit status function",
        workflow: source.replace(
          PUBLISH_JOB_IF_LINE,
          PUBLISH_JOB_IF_LINE.replace("!cancelled() && ", ""),
        ),
      },
      {
        name: "publish without eligibility success",
        workflow: source.replace(
          PUBLISH_JOB_IF_LINE,
          PUBLISH_JOB_IF_LINE.replace(
            "needs.eligibility.result == 'success' && ",
            "",
          ),
        ),
      },
      {
        name: "publish without attestation success",
        workflow: source.replace(
          PUBLISH_JOB_IF_LINE,
          PUBLISH_JOB_IF_LINE.replace(
            " && needs.attest.result == 'success'",
            "",
          ),
        ),
      },
    ];

    for (const mutation of mutations) {
      expect(mutation.workflow, mutation.name).not.toBe(source);
      expect(
        () => assertReleaseTailStatusGates(mutation.workflow),
        mutation.name,
      ).toThrow();
    }
  });

  it("enforces the preflight and formal tail-job truth table", () => {
    const truthTable: Array<{
      expected: ReturnType<typeof releaseTailGateOutcome>;
      input: ReleaseTailGateInput;
      name: string;
    }> = [
      {
        name: "successful preflight attests without publishing",
        input: {
          cancelled: false,
          eligibilityResult: "success",
          eventName: "workflow_dispatch",
          mode: "preflight",
          verifyAssetsResult: "success",
        },
        expected: {
          attestResult: "success",
          attestRuns: true,
          publishRuns: false,
        },
      },
      {
        name: "successful formal tag push attests and publishes",
        input: {
          cancelled: false,
          eligibilityResult: "success",
          eventName: "push",
          mode: "formal",
          verifyAssetsResult: "success",
        },
        expected: {
          attestResult: "success",
          attestRuns: true,
          publishRuns: true,
        },
      },
      {
        name: "unexpected skipped preflight assets fail attestation",
        input: {
          cancelled: false,
          eligibilityResult: "success",
          eventName: "workflow_dispatch",
          mode: "preflight",
          verifyAssetsResult: "skipped",
        },
        expected: {
          attestResult: "failure",
          attestRuns: true,
          publishRuns: false,
        },
      },
      {
        name: "failed formal eligibility fails attestation",
        input: {
          cancelled: false,
          eligibilityResult: "failure",
          eventName: "push",
          mode: "formal",
          verifyAssetsResult: "skipped",
        },
        expected: {
          attestResult: "failure",
          attestRuns: true,
          publishRuns: false,
        },
      },
      {
        name: "failed formal assets fail attestation",
        input: {
          cancelled: false,
          eligibilityResult: "success",
          eventName: "push",
          mode: "formal",
          verifyAssetsResult: "failure",
        },
        expected: {
          attestResult: "failure",
          attestRuns: true,
          publishRuns: false,
        },
      },
      {
        name: "dispatch cannot publish even with a formal-mode mutation",
        input: {
          cancelled: false,
          eligibilityResult: "success",
          eventName: "workflow_dispatch",
          mode: "formal",
          verifyAssetsResult: "success",
        },
        expected: {
          attestResult: "success",
          attestRuns: true,
          publishRuns: false,
        },
      },
      {
        name: "push cannot publish in preflight mode",
        input: {
          cancelled: false,
          eligibilityResult: "success",
          eventName: "push",
          mode: "preflight",
          verifyAssetsResult: "success",
        },
        expected: {
          attestResult: "success",
          attestRuns: true,
          publishRuns: false,
        },
      },
      {
        name: "cancellation starts neither tail job",
        input: {
          cancelled: true,
          eligibilityResult: "success",
          eventName: "push",
          mode: "formal",
          verifyAssetsResult: "success",
        },
        expected: {
          attestResult: "skipped",
          attestRuns: false,
          publishRuns: false,
        },
      },
    ];

    for (const row of truthTable) {
      expect(releaseTailGateOutcome(row.input), row.name).toEqual(row.expected);
    }
  });

  it("keeps authorized run observation synchronous and completion-scoped", () => {
    expect(source).toContain(
      "Authorized callers wait synchronously for this whole run to complete",
    );
    expect(source).toContain("read its final state once");
    expect(source).toContain(
      "fetch failed-job logs only after the completed run reports failure",
    );
    for (const forbiddenMonitor of [
      "gh run watch",
      "gh run view",
      "Start-Job",
      "Start-ThreadJob",
      "Start-Process",
      "nohup",
      "disown",
    ]) {
      expect(source).not.toContain(forbiddenMonitor);
    }
  });

  it("pins every third-party Action and every release runner", () => {
    const actionRefs = [...source.matchAll(/uses:\s+([^\s#]+)/g)].map(
      ([, reference]) => reference,
    );
    expect(actionRefs.length).toBeGreaterThan(0);
    for (const reference of actionRefs) {
      expect(reference).toMatch(/^[\w.-]+\/[\w.-]+@[0-9a-f]{40}$/);
    }
    for (const runner of [
      "ubuntu-24.04",
      "ubuntu-24.04-arm",
      "windows-2025",
      "windows-11-arm",
      "macos-15",
    ]) {
      expect(source).toContain(runner);
    }
    expect(source).not.toContain("windows-2022");
    expect(source).not.toMatch(/runs-on:\s*[^\n]*-latest/);
    expect(source).not.toContain("actions/cache");
    expect(source).not.toContain("cache: true");
    expect(source).not.toContain("cache: pnpm");
    expect(source.match(/uses: actions\/checkout@/g)).toHaveLength(
      source.match(/persist-credentials: false/g)?.length ?? 0,
    );
  });

  it("bootstraps native jobs without implicit tools, broad Git trust, or release caches", () => {
    const nativeJobs = [
      {
        block: workflowJobBlock(
          source,
          "build-windows",
          "prove-windows-preflight",
        ),
        rustStep: "Setup Rust",
      },
      {
        block: workflowJobBlock(source, "build-linux", "build-macos"),
        rustStep: "Setup Rust",
      },
      {
        block: workflowJobBlock(source, "build-macos", "verify-assets"),
        rustStep: "Setup Rust with both universal targets",
      },
    ];

    for (const { block, rustStep } of nativeJobs) {
      const nodeIndex = block.indexOf("- name: Setup Node.js");
      const pnpmIndex = block.indexOf("- name: Setup pnpm");
      expect(nodeIndex).toBeGreaterThanOrEqual(0);
      expect(pnpmIndex).toBeGreaterThan(nodeIndex);
      expect(namedStepBlock(block, "Setup Node.js")).toContain(
        "uses: actions/setup-node@",
      );
      const pnpmStep = namedStepBlock(block, "Setup pnpm");
      expectExactLine(pnpmStep, "          run_install: false");
      expectExactLine(pnpmStep, "          cache: false");
      const rustSetupStep = namedStepBlock(block, rustStep);
      expect(rustSetupStep).toContain(
        "uses: actions-rust-lang/setup-rust-toolchain@",
      );
      expectExactLine(rustSetupStep, "          cache: false");
    }

    const linux = nativeJobs[1].block;
    const trustStep = namedStepBlock(
      linux,
      "Trust exact checked-out workspace for container Git",
    );
    expectExactLine(
      trustStep,
      "          git config --global --unset-all safe.directory 2>/dev/null || true",
    );
    expectExactLine(
      trustStep,
      '          git config --global --add safe.directory "$GITHUB_WORKSPACE"',
    );
    expect(source).not.toMatch(/safe\.directory\s+["']?\*["']?/);
  });

  it("uses read-only defaults and isolates attestation and publication writes", () => {
    expect(source).toContain("permissions:\n  contents: read");
    expect(source).not.toContain("environment:");
    expect(source).toContain("artifact-metadata: write");
    expect(source).toContain("attestations: write");
    expect(source).toContain("id-token: write");
    const publish = source.slice(source.indexOf("\n  publish:\n"));
    expect(publish).toContain("contents: write");
    expect(source.slice(0, source.indexOf("\n  publish:\n"))).not.toContain(
      "contents: write",
    );
  });

  it("binds repository, dev HEAD, annotated formal tag, and exact Required CI through the repository-owned verifier", () => {
    const eligibility = source.slice(
      source.indexOf("\n  eligibility:\n"),
      source.indexOf("\n  build-windows:\n"),
    );
    expect(eligibility).toContain("expected_repository='fy-agent/fyagent'");
    expect(eligibility).toContain("expected_repository_id='1313497021'");
    expect(eligibility).toContain("GITHUB_WORKFLOW_REF");
    expect(eligibility).toContain("GITHUB_WORKFLOW_SHA");
    expect(eligibility).toContain("path: candidate-source");
    expect(eligibility).not.toContain("installer-actions");
    expect(eligibility).not.toContain("pnpm install");
    expect(eligibility).toContain("refs/heads/dev/laiyongjie");
    expect(eligibility).toContain('"refs/tags/$GITHUB_REF_NAME"');
    expect(eligibility).toContain('release_tag="v$app_version"');
    expect(eligibility).toContain('check --tag "$release_tag"');
    expect(eligibility).toContain(
      "node scripts/release/verify-dev-release-remote.mjs",
    );
    expect(eligibility).toContain(
      '--evidence "$RUNNER_TEMP/fyagent-release-remote-evidence.json"',
    );
    expect(eligibility).toContain(
      "RELEASE_DISPATCH_SOURCE_SHA: ${{ inputs.source_sha }}",
    );
    expect(eligibility).toContain(
      "GITHUB_WORKFLOW_SHA: ${{ github.workflow_sha }}",
    );
    expect(eligibility).toContain("unset RELEASE_DISPATCH_SOURCE_SHA");
    expect(eligibility).toContain(
      "ci_run_id: ${{ steps.remote.outputs.ci_run_id }}",
    );
    expect(eligibility).toContain(
      "ci_run_attempt: ${{ steps.remote.outputs.ci_run_attempt }}",
    );
    expect(eligibility).toContain("checks: read");
    expect(eligibility).not.toContain("merge-base --is-ancestor");
    expect(eligibility).not.toContain("refs/remotes/origin/main");
    expect(eligibility).not.toContain("branch=main");
    expect(
      namedStepBlock(
        eligibility,
        "Bind remote dev, tag, and successful Required CI evidence",
      ),
    ).not.toContain("\n        if:");
  });

  it("uses native Linux hosts with reviewed Ubuntu child digests", () => {
    expect(source).toContain(
      "image: ${{ matrix.container_image }}@${{ matrix.container_digest }}",
    );
    expect(source).toContain(
      "sha256:0199853f6d6b20b0424f3c5694a72a62764f01e6a771b1eb48a4197848986c7e",
    );
    expect(source).toContain(
      "sha256:a8cdd2158a73d7e5c02aa351fe269f48f57cf710a241db86e9ede371fc150149",
    );
    expect(source.toLowerCase()).not.toContain("qemu");
    expect(source).toContain("Expected exactly one raw AppImage, DEB, and RPM");
    const packageStep = namedStepBlock(
      workflowJobBlock(source, "build-linux", "build-macos"),
      "Build native Linux packages",
    );
    expectExactLine(packageStep, '          APPIMAGE_EXTRACT_AND_RUN: "1"');
    expect(source.match(/APPIMAGE_EXTRACT_AND_RUN:/g)).toHaveLength(1);
    expect(source).not.toContain("SYS_ADMIN");
    expect(source).not.toContain("/dev/fuse");
  });

  it("records source-explicit runner and container metadata", () => {
    const windowsMetadataStep = namedStepBlock(
      workflowJobBlock(source, "build-windows", "prove-windows-preflight"),
      "Record Windows build metadata",
    );
    const linuxMetadataStep = namedStepBlock(
      workflowJobBlock(source, "build-linux", "build-macos"),
      "Record Linux build metadata",
    );
    const macosMetadataStep = namedStepBlock(
      workflowJobBlock(source, "build-macos", "verify-assets"),
      "Record macOS build metadata",
    );
    for (const step of [
      windowsMetadataStep,
      linuxMetadataStep,
      macosMetadataStep,
    ]) {
      expectExactLine(step, "          ACTUAL_RUNNER_OS: ${{ runner.os }}");
      expectExactLine(step, "          ACTUAL_RUNNER_ARCH: ${{ runner.arch }}");
      expect(step).toContain("write-platform-metadata.mjs");
    }
    expectExactLine(
      windowsMetadataStep,
      "          REQUESTED_RUNNER_LABEL: ${{ matrix.runner }}",
    );
    for (const variable of [
      "CONTAINER_IMAGE_REFERENCE",
      "CONTAINER_MANIFEST_DIGEST",
      "ACTUAL_CONTAINER_OS_ID",
      "ACTUAL_CONTAINER_OS_VERSION_ID",
      "ACTUAL_CONTAINER_UNAME_MACHINE",
    ]) {
      expect(linuxMetadataStep).toContain(variable);
      expect(windowsMetadataStep).not.toContain(variable);
      expect(macosMetadataStep).not.toContain(variable);
    }
    for (const ambientVariable of [
      '"RUNNER_OS"',
      '"RUNNER_ARCH"',
      '"ImageOS"',
      '"ImageVersion"',
    ]) {
      expect(platformMetadataWriter).not.toContain(ambientVariable);
    }
    for (const retiredField of ["imageOs", "imageVersion"]) {
      expect(platformMetadataWriter).not.toContain(retiredField);
      expect(releaseContract).not.toContain(retiredField);
      expect(releaseContractTypes).not.toContain(retiredField);
    }
  });

  it("isolates raw NSIS builds from signing credentials and lifecycle execution", () => {
    const windowsBuild = workflowJobBlock(
      source,
      "build-windows",
      "prove-windows-preflight",
    );
    expect(windowsBuild).toContain("runner: windows-2025");
    expect(windowsBuild).toContain("target_group: windows-x64");
    expect(windowsBuild).toContain("rust_target: x86_64-pc-windows-msvc");
    expect(windowsBuild).toContain("runner: windows-11-arm");
    expect(windowsBuild).toContain("target_group: windows-arm64");
    expect(windowsBuild).toContain("rust_target: aarch64-pc-windows-msvc");
    expect(
      windowsBuild.match(/FYAGENT_WINDOWS_MANIFEST: release/g),
    ).toHaveLength(2);
    expect(
      windowsBuild.match(/verify-windows-nsis-contract\.mjs/g),
    ).toHaveLength(2);
    expect(windowsBuild).toContain("verify-windows-release-manifest.ps1");
    expect(windowsManifestVerifier).toContain("Resolve-WindowsSdkManifestTool");
    expect(windowsManifestVerifier).toContain("requireAdministrator");
    expect(windowsManifestVerifier).toContain("0xAA64");
    expect(windowsManifestVerifier).toContain("0x8664");

    const bundle = namedStepBlock(
      windowsBuild,
      "Bundle Windows NSIS setup executable",
    );
    expect(bundle).toContain(
      "pnpm tauri bundle --target '${{ matrix.rust_target }}' --bundles nsis --verbose",
    );
    expect(bundle).toContain("pnpm tauri bundle --bundles nsis --verbose");
    expect(bundle).not.toContain("--config");
    expect(bundle).toContain("$bundleExitCode = $LASTEXITCODE");
    expect(bundle.indexOf("if ($bundleExitCode -ne 0)")).toBeLessThan(
      bundle.indexOf("Get-ChildItem"),
    );
    expect(bundle).toContain("bundle/nsis");
    expect(bundle).toContain(
      "Expected exactly one raw Windows NSIS setup executable",
    );

    const normalize = namedStepBlock(
      windowsBuild,
      "Normalize exact unsigned Windows candidate",
    );
    expect(normalize).toContain(
      '"FyAgent-$env:APP_VERSION-Windows-${{ matrix.architecture }}-setup.exe"',
    );
    expect(normalize).toContain("raw-windows-candidate");
    expect(normalize).toContain("Expected exactly one normalized raw");

    const unsignedProof = namedStepBlock(
      windowsBuild,
      "Prove raw Windows candidate is strictly unsigned",
    );
    expect(unsignedProof).toContain("windows-signing-evidence.ps1");
    expect(unsignedProof).toContain("fyagent-authenticode-evidence/v1");
    expect(unsignedProof).toContain("NotSigned");
    expect(unsignedProof).toContain("PE security directory is not empty");
    expect(windowsBuild).toContain("name: raw-${{ matrix.target_group }}");
    expect(windowsBuild).toContain("name: metadata-${{ matrix.target_group }}");
    expect(windowsBuild).not.toContain("${{ secrets.");
    expect(windowsBuild).not.toMatch(/\bSIGNER_/u);
    expect(windowsBuild).not.toContain("FYAGENT_WINDOWS_SIGN");
    expect(windowsBuild).not.toContain("windows-signing.mjs asset");
    expect(windowsBuild).not.toContain("verify-windows-nsis-lifecycle.ps1");
    expect(windowsBuild).not.toContain(
      "name: installers-${{ matrix.target_group }}",
    );
    expect(windowsBuild).not.toContain(
      "name: signing-${{ matrix.target_group }}",
    );
    expect(windowsBuild).not.toMatch(/\.msi\b/i);
    expect(windowsBuild).not.toMatch(/\bwix\b/i);
    expect(windowsBuild).not.toContain("installer-actions");
  });

  it("proves canonical FyAgent PE icon resources in raw and sealed Windows setups", () => {
    expect(() => assertWindowsSetupIconGates(source)).not.toThrow();

    const mutations = [
      source.replace(
        "node scripts/release/verify-windows-setup-icon.mjs `",
        "node scripts/release/windows-signing.mjs `",
      ),
      source.replace(
        '"installers/FyAgent-$APP_VERSION-Windows-x64-setup.exe"',
        '"installers/FyAgent-$APP_VERSION-Windows-x86-setup.exe"',
      ),
      source.replace(
        "            src-tauri/icons/icon.ico\n          if ($LASTEXITCODE -ne 0)",
        "            src-tauri/icons/32x32.png\n          if ($LASTEXITCODE -ne 0)",
      ),
      source.replace(
        "            src-tauri/icons/icon.ico\n          if ($LASTEXITCODE -ne 0)",
        "            src-tauri/icons/icon.ico\n          $global:LASTEXITCODE = 0\n          if ($LASTEXITCODE -ne 0)",
      ),
      source.replace(
        SEALED_WINDOWS_SETUP_ICON_GATE_LINES[0],
        `${SEALED_WINDOWS_SETUP_ICON_GATE_LINES[0]} || true`,
      ),
    ];
    for (const mutation of mutations) {
      expect(mutation).not.toBe(source);
      expect(() => assertWindowsSetupIconGates(mutation)).toThrow(
        /canonical PE icon gate|exactly three times/u,
      );
    }
  });

  it("seals unsigned preflight assets in a job whose payload has no signer secrets", () => {
    const preflight = workflowJobBlock(
      source,
      "prove-windows-preflight",
      "sign-windows-formal",
    );
    expectExactLine(
      preflight,
      "    if: needs.eligibility.outputs.release_mode == 'preflight' && github.event_name == 'workflow_dispatch'",
    );
    expectExactLine(
      preflight,
      "    needs: [eligibility, pin-release-build-inputs]",
    );
    expect(preflight).toContain("runner: windows-2025");
    expect(preflight).toContain("runner: windows-11-arm");
    expect(preflight).toContain("permissions:\n      contents: read");
    expect(preflight).not.toContain("id-token:");
    expect(preflight).not.toContain("${{ secrets.");
    expect(preflight).not.toContain("SIGNER_ADAPTER");
    expect(preflight).not.toContain("SIGNER_CREDENTIAL");
    expect(preflight).not.toContain("SIGN_EXPECTED_PUBLISHER");
    expect(preflight).not.toContain("SIGN_EXPECTED_CERTIFICATE");
    expect(preflight).not.toContain("verify-windows-nsis-lifecycle.ps1");
    expect(preflight).not.toContain("pnpm install");
    expect(preflight).not.toMatch(/\bcargo\b/iu);
    expect(preflight).not.toContain("pnpm tauri");
    expect(preflight).toContain(
      "artifact-ids: ${{ needs['pin-release-build-inputs'].outputs.artifact_id }}",
    );
    expect(preflight).not.toContain("name: raw-${{ matrix.target_group }}");
    expect(preflight).toContain("pin-release-build-inputs.mjs verify");

    const unsigned = namedStepBlock(
      preflight,
      "Prove unsigned preflight Windows setup executable",
    );
    expectExactLine(
      unsigned,
      "          FYAGENT_WINDOWS_SIGNING_MODE: unsigned",
    );
    expect(unsigned).toContain("windows-signing.mjs asset");
    expect(unsigned).toContain(
      "--output $env:FYAGENT_WINDOWS_SIGNING_FRAGMENT",
    );
    const validation = namedStepBlock(
      preflight,
      "Validate unsigned Windows preflight output before immutable upload",
    );
    expect(validation).toContain("$fragment.mode -cne 'unsigned'");
    expect(validation).toContain(
      "$fragment.asset.signature.status -cne 'NotSigned'",
    );
    expect(validation).toContain(
      "$null -ne $fragment.asset.signature.publisher",
    );
    expect(validation).toContain(
      "$null -ne $fragment.asset.signature.signerCertificate",
    );
    expect(validation).toContain(
      "$null -ne $fragment.asset.signature.timestampCertificate",
    );
    expect(validation).toContain("$fragment.asset.sha256 -cne $assetSha256");
    expect(preflight).toContain("name: installers-${{ matrix.target_group }}");
    expect(preflight).toContain("name: signing-${{ matrix.target_group }}");
    expect(preflight.match(/uses: actions\/upload-artifact@/gu)).toHaveLength(
      2,
    );
  });

  it("limits the secret-bearing formal producer to one untrusted candidate artifact", () => {
    const formal = workflowJobBlock(
      source,
      "sign-windows-formal",
      "seal-windows-formal",
    );
    expectExactLine(
      formal,
      "    if: needs.eligibility.outputs.release_mode == 'formal' && github.event_name == 'push'",
    );
    expectExactLine(
      formal,
      "    needs: [eligibility, pin-release-build-inputs]",
    );
    expect(formal).toContain("runner: windows-2025");
    expect(formal).toContain("runner: windows-11-arm");
    expect(formal).toContain("permissions:\n      contents: read");
    expect(formal).not.toContain("id-token:");
    expect(formal).not.toContain("verify-windows-nsis-lifecycle.ps1");
    expect(formal).not.toContain("pnpm install");
    expect(formal).not.toMatch(/\bcargo\b/iu);
    expect(formal).not.toContain("pnpm tauri");
    expect(formal).toContain(
      "artifact-ids: ${{ needs['pin-release-build-inputs'].outputs.artifact_id }}",
    );
    expect(formal).not.toContain("name: raw-${{ matrix.target_group }}");
    expect(formal).toContain("pin-release-build-inputs.mjs verify");

    const transform = namedStepBlock(
      formal,
      "Produce untrusted formal Windows candidate",
    );
    expect(transform).toContain("windows-signing.mjs transform");
    expect(transform).not.toContain("windows-signing.mjs asset");
    expect(transform).not.toContain("--output");
    expect(transform).toContain(
      "SIGNING_MODE_CONFIG: ${{ vars.FYAGENT_WINDOWS_SIGNING_MODE }}",
    );
    expect(transform).toContain("$hasProviderConfig");
    expect(transform).toContain("$stagingSignerEnvironment");
    expect(transform).toContain("$managedSignerEnvironment");
    expect(transform).toContain(
      "@($managedSignerEnvironment + $stagingSignerEnvironment)",
    );
    expect(transform).toContain("[IO.File]::Delete($adapterPath)");
    expect(transform.slice(transform.indexOf("run: |"))).not.toContain(
      "${{ secrets.",
    );
    expect(formal.match(/\$\{\{ secrets\./gu)).toHaveLength(2);
    expect(formal).toContain(
      "name: formal-candidate-${{ matrix.target_group }}",
    );
    expect(formal).not.toContain("name: installers-${{ matrix.target_group }}");
    expect(formal).not.toContain("name: signing-${{ matrix.target_group }}");
    expect(formal).not.toContain("signing-fragments");
    expect(formal).not.toContain("verify-sealed");
    expect(formal).not.toContain("windows-signing-evidence.ps1");
    expect(formal).not.toContain("verify-windows-nsis-lifecycle.ps1");
    expect(formal.match(/uses: actions\/upload-artifact@/gu)).toHaveLength(1);
    expect(formal.trimEnd()).toMatch(/retention-days: 7$/u);
  });

  it("verifies and seals formal bytes on a fresh no-secret native runner", () => {
    const sealer = workflowJobBlock(
      source,
      "seal-windows-formal",
      "build-linux",
    );
    expectExactLine(
      sealer,
      "    if: needs.eligibility.outputs.release_mode == 'formal' && github.event_name == 'push'",
    );
    expectExactLine(
      sealer,
      "    needs: [eligibility, pin-release-build-inputs, sign-windows-formal]",
    );
    expect(sealer).toContain("runner: windows-2025");
    expect(sealer).toContain("runner: windows-11-arm");
    expect(sealer).toContain("permissions:\n      contents: read");
    expect(sealer).not.toContain("${{ secrets.");
    expect(sealer).not.toContain("SIGNER_ADAPTER");
    expect(sealer).not.toContain("SIGNER_CREDENTIAL");
    expect(sealer).not.toContain("SIGNER_ADAPTER_BASE64_CONFIG");
    expect(sealer).not.toContain("FYAGENT_WINDOWS_SIGNER_ADAPTER");
    expect(sealer).not.toContain("windows-signing.mjs transform");
    expect(sealer).not.toContain("windows-signing.mjs asset");
    expect(sealer).not.toContain("verify-windows-nsis-lifecycle.ps1");
    expect(sealer).not.toContain("pnpm install");
    expect(sealer).not.toMatch(/\bcargo\b/iu);
    expect(sealer).not.toContain("pnpm tauri");
    expect(sealer).toContain(
      "artifact-ids: ${{ needs['pin-release-build-inputs'].outputs.artifact_id }}",
    );
    expect(sealer).not.toContain("name: raw-${{ matrix.target_group }}");
    expect(sealer).toContain("pin-release-build-inputs.mjs verify");
    expect(sealer).toContain(
      "name: formal-candidate-${{ matrix.target_group }}",
    );
    expect(sealer).toContain("name: installers-${{ matrix.target_group }}");
    expect(sealer).toContain("name: signing-${{ matrix.target_group }}");
    expect(sealer.match(/uses: actions\/upload-artifact@/gu)).toHaveLength(2);

    const verification = namedStepBlock(
      sealer,
      "Independently verify and seal formal Windows candidate",
    );
    expect(verification).toContain("windows-signing.mjs");
    expect(verification).toContain("'verify-sealed'");
    expect(verification).toContain("'--raw'");
    expect(verification).toContain("'--candidate'");
    expect(verification).toContain("--mode unsigned");
    expect(verification).toContain("--mode provider");
    expect(verification).toContain("--expected-publisher");
    expect(verification).toContain("--expected-certificate-sha256");
  });

  it("routes successful builds and Windows sealing directly into asset verification without installer execution", () => {
    expect(() =>
      assertReleaseWorkflowDoesNotExecuteInstallers(source),
    ).not.toThrow();
    const verify = workflowJobBlock(source, "verify-assets", "attest");
    expectExactLine(
      verify,
      "    if: ${{ always() && needs.eligibility.result == 'success' && needs['build-windows'].result == 'success' && needs['build-linux'].result == 'success' && needs['build-macos'].result == 'success' && needs['pin-release-build-inputs'].result == 'success' && ((github.event_name == 'workflow_dispatch' && needs.eligibility.outputs.release_mode == 'preflight' && needs['prove-windows-preflight'].result == 'success' && needs['sign-windows-formal'].result == 'skipped' && needs['seal-windows-formal'].result == 'skipped') || (github.event_name == 'push' && needs.eligibility.outputs.release_mode == 'formal' && needs['prove-windows-preflight'].result == 'skipped' && needs['sign-windows-formal'].result == 'success' && needs['seal-windows-formal'].result == 'success')) }}",
    );
    expect(verify).toContain(
      "    needs:\n      [\n        eligibility,\n        build-windows,\n        build-linux,\n        build-macos,\n        pin-release-build-inputs,\n        prove-windows-preflight,\n        sign-windows-formal,\n        seal-windows-formal,\n      ]",
    );
    expect(verify).toContain(
      "artifact-ids: ${{ needs['pin-release-build-inputs'].outputs.artifact_id }}",
    );
    expect(verify).toContain("pattern: installers-windows-*");
    expect(verify).toContain("pattern: signing-*");
    expect(verify).toContain("windows-signing.mjs aggregate");
    expect(verify).not.toContain("runs-on: windows");
  });

  it("parses every legal GitHub Actions job ID shape used by the topology guard", () => {
    expect(
      releaseWorkflowJobIds(`
jobs:
  _Leading_ID:
    runs-on: ubuntu-24.04
  'UpperCase':
    runs-on: ubuntu-24.04
  "lower-hyphen_2":
    runs-on: ubuntu-24.04
`),
    ).toEqual(["_Leading_ID", "UpperCase", "lower-hyphen_2"]);
  });

  it.each(["_Windows_SMOKE", "'_Windows_SMOKE'", '"_Windows_SMOKE"'])(
    "rejects an extra legal Actions job key %s independently of its behavior",
    (jobKey) => {
      const mutated = source.replace(
        "\n  verify-assets:\n",
        `
  ${jobKey}:
    runs-on: ubuntu-24.04
    steps:
      - name: Harmless topology mutation
        run: echo unexpected extra job

  verify-assets:
`,
      );
      expect(mutated).not.toBe(source);
      expect(() =>
        assertReleaseWorkflowDoesNotExecuteInstallers(mutated),
      ).toThrow(/unexpected Release job topology:.*_Windows_SMOKE/u);
    },
  );

  it("fails closed on an escaped YAML spelling of a legal Actions job ID", () => {
    const mutated = source.replace(
      "\n  verify-assets:\n",
      `
  "\\x5fWindows_SMOKE":
    runs-on: ubuntu-24.04
    steps:
      - run: echo unexpected escaped job key

  verify-assets:
`,
    );
    expect(mutated).not.toBe(source);
    expect(() => releaseWorkflowJobIds(mutated)).toThrow(
      /unsupported Release job key syntax/u,
    );
  });

  it("fails closed on an escaped YAML spelling of the run step key", () => {
    const step = `
      - name: Mutated escaped run key
        shell: pwsh
        "\\x72un": '.\\release-assets\\FyAgent-smoke-setup.exe /S'
`;
    const mutated = source.replace(
      "\n      - name: Checkout immutable formal verification boundary\n",
      `${step}\n      - name: Checkout immutable formal verification boundary\n`,
    );
    expect(mutated).not.toBe(source);
    expect(releaseWorkflowJobIds(mutated)).toEqual(EXPECTED_RELEASE_JOB_IDS);
    expect(() =>
      assertReleaseWorkflowDoesNotExecuteInstallers(mutated),
    ).toThrow(/unsupported Release direct mapping key syntax/u);
  });

  it("fails closed on sequence-first run keys and flow-mapping steps", () => {
    const sequenceItems = [
      "      - run: .\\release-assets\\FyAgent-smoke-setup.exe /S",
      "      - { run: .\\release-assets\\FyAgent-smoke-setup.exe /S }",
    ];
    for (const sequenceItem of sequenceItems) {
      const mutated = source.replace(
        "\n      - name: Checkout immutable formal verification boundary\n",
        `\n${sequenceItem}\n\n      - name: Checkout immutable formal verification boundary\n`,
      );
      expect(mutated).not.toBe(source);
      expect(releaseWorkflowJobIds(mutated)).toEqual(EXPECTED_RELEASE_JOB_IDS);
      expect(() =>
        assertReleaseWorkflowDoesNotExecuteInstallers(mutated),
      ).toThrow(/unsupported Release step sequence item syntax/u);
    }
  });

  it.each(["'", '"'])(
    "rejects a %s-quoted inline installer command before shell scanning",
    (quote) => {
      const step = `
      - name: Mutated quoted installer execution
        shell: pwsh
        run: ${quote}.\\release-assets\\FyAgent-smoke-setup.exe /S${quote}
`;
      const mutated = source.replace(
        "\n      - name: Checkout immutable formal verification boundary\n",
        `${step}\n      - name: Checkout immutable formal verification boundary\n`,
      );
      expect(mutated).not.toBe(source);
      expect(releaseWorkflowJobIds(mutated)).toEqual(EXPECTED_RELEASE_JOB_IDS);
      expect(() =>
        assertReleaseWorkflowDoesNotExecuteInstallers(mutated),
      ).toThrow(/unsupported Release inline run scalar/u);
    },
  );

  it.each([
    {
      launch: "lowercase Start-Process under a commented quoted run key",
      runHeader: '"run" : |2- # execution-guard mutation',
      script: "start-process $env:FYAGENT_WINDOWS_FINAL_ASSET /S",
      expectedError: /via Start-Process/u,
    },
    {
      launch: "a variable command",
      script:
        "$candidate = $env:FYAGENT_WINDOWS_FINAL_ASSET\n          & $candidate /S",
      expectedError: /non-allowlisted PowerShell call operator/u,
    },
    {
      launch: "cmd.exe",
      script: 'cmd.exe /c "$env:FYAGENT_WINDOWS_FINAL_ASSET /S"',
      expectedError: /via cmd command shell/u,
    },
    {
      launch: "the PowerShell call operator with an environment variable",
      script: "& $env:FYAGENT_WINDOWS_FINAL_ASSET /S",
      expectedError: /non-allowlisted PowerShell call operator/u,
    },
    {
      launch: "a direct executable path",
      script: ".\\release-assets\\FyAgent-smoke-setup.exe /S",
      expectedError: /via direct executable command/u,
    },
  ])(
    "rejects installer execution in an existing job via $launch",
    ({ runHeader = "run: |", script, expectedError }) => {
      const step = `
      - name: Mutated installer execution
        shell: pwsh
        ${runHeader}
          ${script}
`;
      const mutated = source.replace(
        "\n      - name: Checkout immutable formal verification boundary\n",
        `${step}\n      - name: Checkout immutable formal verification boundary\n`,
      );
      expect(mutated).not.toBe(source);
      expect(releaseWorkflowJobIds(mutated)).toEqual(EXPECTED_RELEASE_JOB_IDS);
      expect(() =>
        assertReleaseWorkflowDoesNotExecuteInstallers(mutated),
      ).toThrow(expectedError);
    },
  );

  it("pins all build outputs before the provider receives an artifact token", () => {
    const pin = workflowJobBlock(
      source,
      "pin-release-build-inputs",
      "verify-assets",
    );
    expectExactLine(
      pin,
      "    needs: [eligibility, build-windows, build-linux, build-macos]",
    );
    expect(pin).toContain(
      "artifact_id: ${{ steps.upload.outputs.artifact-id }}",
    );
    expect(pin).toContain(
      "artifact_digest: ${{ steps.upload.outputs.artifact-digest }}",
    );
    expect(pin).toContain("pattern: raw-windows-*");
    expect(pin).toContain("pattern: installers-*");
    expect(pin).toContain("pattern: metadata-*");
    expect(pin).toContain("pin-release-build-inputs.mjs create");
    expect(pin).toContain("name: trusted-build-inputs");
    expect(pin.match(/uses: actions\/upload-artifact@/gu)).toHaveLength(1);
    expect(pin).not.toContain("${{ secrets.");
    expect(pin).not.toContain("SIGNER_");

    const formal = workflowJobBlock(
      source,
      "sign-windows-formal",
      "seal-windows-formal",
    );
    expect(source.indexOf("\n  pin-release-build-inputs:\n")).toBeLessThan(
      source.indexOf("\n  verify-assets:\n"),
    );
    expect(formal).toContain("pin-release-build-inputs");
  });

  it("aggregates signing evidence and attests thirteen subjects into fourteen attachments", () => {
    const verify = workflowJobBlock(source, "verify-assets", "attest");
    expect(verify).toContain(
      "artifact-ids: ${{ needs['pin-release-build-inputs'].outputs.artifact_id }}",
    );
    expect(verify).toContain("pin-release-build-inputs.mjs verify");
    expect(verify).toContain("pattern: installers-windows-*");
    expect(verify).not.toContain("pattern: metadata-*");
    expect(verify).toContain("pattern: signing-*");
    expect(verify).not.toContain("pattern: raw-*");
    expect(verify).not.toContain("formal-candidate-");
    expect(verify).toContain("signing downloaded-signing signing-fragments");
    expect(verify).toContain("windows-signing.mjs aggregate");
    expect(verify).toContain(
      "--x64-status signing-fragments/windows-signing-x64.json",
    );
    expect(verify).toContain(
      "--arm64-status signing-fragments/windows-signing-arm64.json",
    );
    expect(verify).toContain("--output verified-subjects/signing-status.json");
    expect(verify).toContain("Upload the exact thirteen attestation subjects");

    const attest = workflowJobBlock(source, "attest", "publish");
    expectExactLine(attest, "    needs: [eligibility, verify-assets]");
    expect(attest).toContain(
      "actions/attest@1e69f48acb82d1966a394da916b4c1698aa569d6",
    );
    expect(attest).toContain("subject-path: verified-subjects/*");
    expect(attest).toContain("Recheck the exact thirteen subjects");
    expect(attest).toContain("exact fourteen Release attachments");
    expect(attest).toContain("artifact-attestation.sigstore.json");

    const publish = source.slice(source.indexOf("\n  publish:\n"));
    expectExactLine(publish, "    needs: [eligibility, attest]");
    expect(publish).toContain("fyagent-windows-signing-status/v1");
    expect(publish).toContain("## Windows installer signing status");
    expect(publish).toContain(".signature.status");
    expect(publish).toContain(".signature.publisher");
    expect(publish).toContain(".signature.timestampCertificate");
    expect(publish).toContain(".attestation.bundle");
    expect(publish).toContain(".attestation.subjectName");
    expect(publish).toContain(".attestation.subjectDigest");
    expect(publish).toContain("signing-status.json");
    expect(publish).toContain("length == 14");
    expect(publish).toContain("(.assets | length) == 14");
  });

  it("seals the universal macOS app ad-hoc while keeping the DMG unsigned", () => {
    const macJob = source.slice(
      source.indexOf("\n  build-macos:\n"),
      source.indexOf("\n  pin-release-build-inputs:\n"),
    );
    const macAdhocVerifier = read(MACOS_ADHOC_VERIFIER);
    expect(fs.statSync(MACOS_ADHOC_VERIFIER).mode & 0o111).not.toBe(0);
    const tauriConfig = JSON.parse(read(TAURI_CONFIG)) as {
      bundle?: { macOS?: { signingIdentity?: string } };
    };
    expect(source).toContain("--target universal-apple-darwin --bundles app");
    expect(source).toContain("lipo -archs");
    expect(source).toContain("CFBundleShortVersionString");
    expect(source).toContain("com.fyagent.desktop");
    expect(source).toContain("xcrun stapler validate");
    expect(source).not.toContain("stapler staple");
    expect(source).not.toContain("notarytool");
    expect(source).toContain("hdiutil attach");
    expect(source).toContain("-readonly");
    expect(tauriConfig.bundle?.macOS).not.toHaveProperty("signingIdentity");
    expect(macJob).not.toContain("APPLE_SIGNING_IDENTITY");
    expect(macJob).toContain(
      'codesign --force --sign - --timestamp=none "$app_path"',
    );
    expect(
      macJob.match(/scripts\/release\/verify-macos-adhoc-app\.sh/gu),
    ).toHaveLength(3);
    expect(macAdhocVerifier).toContain("for architecture in arm64 x86_64; do");
    expect(macAdhocVerifier).toContain("Signature=adhoc");
    expect(macAdhocVerifier).toContain("flags=.*adhoc");
    expect(macAdhocVerifier).toContain("TeamIdentifier=not set");
    expect(macAdhocVerifier).toContain("^Sealed Resources version=");
    expect(macAdhocVerifier).toContain(
      "codesign --verify --deep --strict --verbose=4",
    );
    expect(macAdhocVerifier).toContain("xcrun stapler validate");
    expect(macAdhocVerifier).not.toMatch(/codesign\s+--force[^\n]*--deep/gu);
    expect(macAdhocVerifier).toMatch(
      /linker-signed\|\^Authority=\|Developer ID/u,
    );
    expect(macJob).toContain(
      'if dmg_signature="$(codesign -dvvv "$dmg_path" 2>&1)"; then',
    );
    expect(macJob.match(/unexpectedly has a code signature/gu)).toHaveLength(1);
    expect(macJob.match(/code object is not signed at all/gu)).toHaveLength(1);
    expect(macJob).toContain(
      "^Signature=|^Authority=|Developer ID|^TeamIdentifier=|^Timestamp=|^CMSDigest",
    );
    expect(macJob).not.toContain('codesign -dvvv "$dmg_path" 2>&1 || true');
    expect(source).toContain(
      "FyAgent-${APP_VERSION}-Linux-${{ matrix.asset_arch }}.AppImage",
    );
    expect(source).toContain("FyAgent-${APP_VERSION}-macOS.dmg");
  });

  it("executes the ad-hoc verifier for both slices and fails closed on trust drift", () => {
    const accepted = runMacAdhocVerifier("accepted");
    expect(accepted.status, accepted.stderr).toBe(0);
    expect(
      accepted.calls.filter((call) => call.startsWith("--display ")),
    ).toEqual([
      expect.stringContaining("--architecture arm64"),
      expect.stringContaining("--architecture x86_64"),
    ]);
    expect(accepted.calls).toContainEqual(
      expect.stringContaining("--verify --deep --strict"),
    );

    for (const rejected of [
      "authority",
      "linker",
      "stapled",
      "team",
      "timestamp",
      "unsealed",
      "verify-fail",
    ]) {
      const result = runMacAdhocVerifier(rejected);
      expect(result.status, `${rejected}: ${result.stderr}`).not.toBe(0);
    }
  });

  it("publishes once through a verified draft and never auto-deletes failure residue", () => {
    const publish = source.slice(source.indexOf("\n  publish:\n"));
    expect(publish).toContain("releases?per_page=100");
    expect(publish).toContain("draft:true,prerelease:false");
    expect(publish).toContain('all(.state == "uploaded" and .size > 0)');
    expect(publish).toContain("Re-downloaded bytes differ");
    expect(publish).toContain("draft:false,prerelease:false");
    expect(publish).toContain('make_latest:"true"');
    expect(publish).toContain("releases/latest");
    expect(publish).toContain("failure-release-state.json");
    expect(publish).toContain("The publish outcome is unknown");
    expect(publish).toContain("published-confirmed.json");
    expect(publish).toContain(
      'release_notes_path="docs/release-notes/${RELEASE_TAG}-en.md"',
    );
    expect(publish).not.toContain("gh release create");
    expect(publish).not.toContain("--request DELETE");
    expect(publish).not.toContain("gh release delete");
    expect(publish).not.toMatch(/git (?:push --delete|tag -d)/);
  });

  it("rechecks the exact frozen remote eligibility before publication starts and immediately before the final PATCH", () => {
    const publish = source.slice(source.indexOf("\n  publish:\n"));
    expect(publish).toContain(PUBLISH_JOB_IF_LINE);
    expect(publish).toContain(
      "permissions:\n      actions: read\n      checks: read\n      contents: write",
    );
    expect(publish).toContain(
      "GITHUB_WORKFLOW_SHA: ${{ github.workflow_sha }}",
    );
    expect(publish).toContain(
      "Revalidate frozen dev release eligibility at publish start",
    );
    expect(
      publish.match(/node scripts\/release\/verify-dev-release-remote\.mjs/gu),
    ).toHaveLength(2);
    expect(publish.match(/--expected /gu)).toHaveLength(2);
    expect(publish).toContain('--expected "$frozen_eligibility"');
    expect(publish).toContain(
      '--expected "$RUNNER_TEMP/fyagent-frozen-release-eligibility.json"',
    );
    for (const frozenField of [
      "appVersion",
      "releaseTag",
      "sourceSha",
      "workflowSha",
      "ciRunId",
      "ciRunAttempt",
      'mode:"formal"',
    ]) {
      expect(publish).toContain(frozenField);
    }
    const finalRecheck = publish.lastIndexOf(
      "node scripts/release/verify-dev-release-remote.mjs",
    );
    const publishRequest = publish.indexOf(
      'publish_status="$(curl --silent',
      finalRecheck,
    );
    const finalPatch = publish.indexOf("--request PATCH", finalRecheck);
    expect(finalRecheck).toBeGreaterThan(
      publish.indexOf('prepublish_json="$transaction_root/prepublish.json"'),
    );
    expect(publishRequest).toBeGreaterThan(finalRecheck);
    expect(finalPatch).toBeGreaterThan(finalRecheck);
    expect(publish.slice(finalRecheck, publishRequest)).not.toContain(
      "curl --silent",
    );
  });

  it("keeps formal assets free of MSI, WiX, portable, and updater surfaces", () => {
    const normalized = source.toLowerCase();
    expect(source).not.toMatch(
      /(?:verified-subjects|release-attachments)\/latest\.json/i,
    );
    expect(normalized).not.toContain("tauri_signing_private_key");
    expect(normalized).not.toContain("portable");
    expect(source).not.toMatch(/\.msi\b/i);
    expect(source).not.toMatch(/\bwix\b/i);
    expect(source).not.toContain("installer-actions");
  });
});

describe("FyAgent Windows NSIS, elevation, signing, and manual diagnostics", () => {
  const windowsConfig = JSON.parse(read(TAURI_WINDOWS_CONFIG)) as {
    bundle: {
      targets: string[];
      windows: {
        webviewInstallMode: { type: string };
        nsis: {
          template: string;
          installerHooks: string;
          installerIcon: string;
          installMode: string;
          languages: string[];
          displayLanguageSelector: boolean;
          startMenuFolder: string;
        };
      };
    };
  };
  const template = read(NSIS_TEMPLATE);
  const contract = read(NSIS_CONTRACT);
  const lifecycle = read(NSIS_LIFECYCLE);
  const signing = read(WINDOWS_SIGNING);
  const signingEvidence = read(WINDOWS_SIGNING_EVIDENCE);
  const releaseWorkflow = read(RELEASE_WORKFLOW);
  const buildRs = read(BUILD_RS);
  const testManifest = read(TEST_MANIFEST);
  const releaseManifest = read(RELEASE_MANIFEST);
  const ciWorkflow = read(CI_WORKFLOW);
  const cargoToml = read(CARGO_TOML);
  const autoLaunch = read(AUTO_LAUNCH);
  const libRs = read(LIB_RS);

  it("selects normal-privilege tests and an elevated formal application manifest", () => {
    expect(testManifest).toContain(
      '<requestedExecutionLevel level="asInvoker" uiAccess="false" />',
    );
    expect(testManifest).not.toContain("requireAdministrator");
    expect(releaseManifest).toContain(
      '<requestedExecutionLevel level="requireAdministrator" uiAccess="false" />',
    );
    for (const manifest of [testManifest, releaseManifest]) {
      expect(manifest).toContain("Microsoft.Windows.Common-Controls");
      expect(manifest).toContain('version="6.0.0.0"');
    }
    expect(buildRs).toContain("FYAGENT_WINDOWS_MANIFEST");
    expect(buildRs).toContain("WindowsAttributes::new().app_manifest");
    expect(buildRs).toContain("cargo:rustc-cfg=fyagent_windows_release");
    expect(buildRs).toContain("cargo:rustc-link-arg=/MANIFEST:EMBED");
    expect(buildRs).toContain("cargo:rustc-link-arg-bins=/MANIFEST:NO");
    expect(buildRs).not.toContain("cargo:rustc-link-arg-tests=");
    expect(ciWorkflow).toContain("FYAGENT_WINDOWS_MANIFEST: test");
  });

  it("selects only per-machine bilingual NSIS with WebView2 bootstrap download", () => {
    expect(windowsConfig.bundle.targets).toEqual(["nsis"]);
    expect(windowsConfig.bundle.windows.webviewInstallMode).toEqual({
      type: "downloadBootstrapper",
    });
    expect(windowsConfig.bundle.windows.nsis).toEqual({
      template: "nsis/installer.nsi",
      installerHooks: "nsis/webview2-command.nsh",
      installerIcon: "icons/icon.ico",
      installMode: "perMachine",
      languages: ["English", "SimpChinese"],
      displayLanguageSelector: false,
      startMenuFolder: "FyAgent",
    });
    expect(template).toContain("RequestExecutionLevel admin");
    expect(template).toContain("${LANG_ENGLISH}");
    expect(template).toContain("${LANG_SIMPCHINESE}");
    expect(template).toContain('!if "${DISPLAYLANGUAGESELECTOR}" == "true"');
    expect(template).toContain('!define MUI_ICON "${INSTALLERICON}"');
    expect(template).toContain('!define MUI_UNICON "${INSTALLERICON}"');
  });

  it("leaves installation-path selection to the standard NSIS directory flow", () => {
    expect(template).toContain("!insertmacro MUI_PAGE_DIRECTORY");
    expect(template).not.toContain("Function FyAgentValidateFinalInstallDir");
    expect(template).not.toContain("GetDriveTypeW");
    expect(template).not.toContain("FYAGENT_DRIVE_FIXED");
    expect(template).not.toContain("MUI_PAGE_CUSTOMFUNCTION_LEAVE");
    expect(template).not.toContain("Section -FyAgentInstallDirGate");

    const runtime = template.indexOf("Section -FyAgentMachineRuntimeBootstrap");
    const webview = template.indexOf("Section WebView2");
    const setOutPath = template.indexOf("SetOutPath $INSTDIR");
    expect(runtime).toBeGreaterThan(-1);
    expect(webview).toBeGreaterThan(runtime);
    expect(setOutPath).toBeGreaterThan(webview);
  });

  it("pins a hermetic NSIS source verifier to the config and template boundary", () => {
    expect(contract).toContain("tauri.windows.conf.json");
    expect(contract).toContain("nsis/installer.nsi");
    expect(contract).toContain("downloadBootstrapper");
    expect(contract).toContain("perMachine");
    expect(contract).toContain("assertInstallPathPolicyContract");
    expect(contract).toContain(
      "must not reintroduce custom installation-path restriction",
    );
    expect(contract).toContain("WebView2");
    expect(contract).toContain("SetOutPath");
  });

  it("retains the manual native lifecycle diagnostic and derives product architecture from installed fyagent.exe", () => {
    expect(lifecycle).toContain("fyagent.exe");
    expect(lifecycle).toContain("0x8664");
    expect(lifecycle).toContain("0xAA64");
    expect(lifecycle).toContain("$InstallerPath");
    expect(lifecycle).toContain("$Architecture");
    expect(lifecycle).toContain("$AppVersion");
    expect(lifecycle).toContain("DisplayVersion");
    expect(lifecycle).toMatch(/\/D=/);
    expect(lifecycle).not.toContain("relative-path-negative");
    expect(lifecycle).not.toContain("unc-network-negative");
    expect(lifecycle).not.toContain("unsupported-drive-network-negative");
    expect(lifecycle).not.toContain("NativeNetworkDrive");
    expect(lifecycle).toContain("default-install");
    expect(lifecycle).toContain("custom-space-unicode-silent-D");
    expect(lifecycle).toContain(".fyagent");
    expect(lifecycle).toContain("com.fyagent.desktop");
    expect(lifecycle).toContain("uninstall.exe");
    expect(lifecycle).not.toMatch(/installer[^\n]*PE Machine/i);
  });

  it("preserves user state while removing bounded installer-owned runtime state", () => {
    expect(template).toContain(
      'Delete "$COMMONPROGRAMDATA\\FyAgent\\runtime\\business-*.state"',
    );
    expect(template).toContain(
      'Delete "$COMMONPROGRAMDATA\\FyAgent\\runtime\\business-*.lock"',
    );
    expect(template).toContain('RMDir "$COMMONPROGRAMDATA\\FyAgent\\runtime"');
    expect(template).toContain("~/.fyagent data");
    expect(template).not.toMatch(
      /RMDir\s+\/r[^\n]*(?:APPDATA|LOCALAPPDATA|\.fyagent)/i,
    );
    expect(lifecycle).toContain("default-uninstall-user-data-preservation");
    expect(lifecycle).toContain("custom-uninstall-user-data-preservation");
    expect(lifecycle).toContain("User data sentinel was deleted by uninstall");
  });

  it("keeps signing provider-neutral, fail-closed, and independent of launcher architecture", () => {
    expect(signing).toContain("FYAGENT_WINDOWS_SIGNER_ADAPTER");
    expect(releaseWorkflow).toContain(
      "secrets.FYAGENT_WINDOWS_SIGNER_ADAPTER_BASE64",
    );
    expect(releaseWorkflow).toContain(
      "secrets.FYAGENT_WINDOWS_SIGNER_CREDENTIAL",
    );
    expect(releaseWorkflow).not.toContain(
      "vars.FYAGENT_WINDOWS_SIGNER_ADAPTER",
    );
    expect(releaseWorkflow).toContain("[IO.FileMode]::CreateNew");
    expect(releaseWorkflow).toContain("[IO.FileShare]::None");
    expect(releaseWorkflow).toContain(
      "[Environment]::SetEnvironmentVariable($name, $null, 'Process')",
    );
    expect(releaseWorkflow).toContain("[Array]::Clear($providerConfig");
    expect(signing).toContain("FYAGENT_WINDOWS_SIGNING_MODE");
    expect(signing).toContain("FYAGENT_WINDOWS_SIGN_EXPECTED_PUBLISHER");
    expect(signing).toContain(
      "FYAGENT_WINDOWS_SIGN_EXPECTED_CERTIFICATE_SHA256",
    );
    expect(signing).toContain("Windows signer configuration is partial");
    expect(signing).toContain("SUPPORTED_LAUNCHER_PE_MACHINES");
    expect(signing).toContain("0x014c");
    expect(signing).toContain("assertAuthenticodeOnlyMutation");
    expect(signing).toContain('if (command === "asset")');
    expect(signing).toContain('if (command === "transform")');
    expect(signing).toContain('if (command === "verify-sealed")');
    expect(signing).toContain('if (command === "aggregate")');
    expect(signing).not.toContain("PE_MACHINE = Object.freeze({ x64");
    expect(signingEvidence).toContain("Get-AuthenticodeSignature");
    expect(signingEvidence).toContain("TimeStamperCertificate");
    expect(signingEvidence).toContain(
      "$PSModuleAutoLoadingPreference = 'None'",
    );
    expect(signingEvidence).toContain(
      "Microsoft.PowerShell.Security\\Get-AuthenticodeSignature",
    );
    expect(signingEvidence).not.toMatch(
      /^\s*\$signature\s*=\s*Get-AuthenticodeSignature/mu,
    );
  });

  it("removes the MSI/WiX helper, query, verifier, and fixture implementation", () => {
    for (const retired of [
      path.join(ROOT, "src-tauri", "installer-actions"),
      path.join(ROOT, "src-tauri", "wix"),
      path.join(ROOT, "scripts", "release", "WindowsInstallerQuery.psm1"),
      path.join(ROOT, "scripts", "release", "verify-windows-msi.ps1"),
      path.join(ROOT, "scripts", "release", "verify-windows-msi-structure.ps1"),
      path.join(ROOT, "scripts", "release", "verify-windows-unsigned.ps1"),
      path.join(ROOT, "tests", "windowsInstallerQuery.integration.ps1"),
      path.join(ROOT, "tests", "windowsInstallerQueryContract.test.ts"),
      path.join(ROOT, "tests", "fixtures", "windows-installer-query"),
    ]) {
      expect(fs.existsSync(retired), retired).toBe(false);
    }
    expect(cargoToml).not.toContain("installer-actions");
    expect(cargoToml).not.toContain("wix");
  });

  it("disables Windows autolaunch without widening cleanup ownership", () => {
    expect(autoLaunch).toContain(
      'const WINDOWS_AUTO_LAUNCH_VALUE: &str = "FyAgent";',
    );
    expect(autoLaunch).toContain("clear_windows_auto_launch_entry");
    expect(autoLaunch).toContain("enforce_platform_auto_launch_policy");
    expect(autoLaunch).not.toContain(
      'WINDOWS_AUTO_LAUNCH_VALUE: &str = "CC Switch"',
    );
    const cleanupIndex = libRs.indexOf(
      "auto_launch::enforce_platform_auto_launch_policy()",
    );
    const builderIndex = libRs.indexOf(
      "let builder = tauri::Builder::default();",
    );
    expect(cleanupIndex).toBeGreaterThan(-1);
    expect(builderIndex).toBeGreaterThan(cleanupIndex);
  });
});
