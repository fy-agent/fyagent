#!/usr/bin/env node

import { createHash, X509Certificate } from "node:crypto";
import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { gunzipSync, gzipSync } from "node:zlib";
import { assertWindowsBundleVersion } from "./release-contract.mjs";

export const TAURI_NSIS_UPSTREAM = Object.freeze({
  tag: "tauri-cli-v2.8.1",
  commit: "662b39adb33d1d26f0de213e5a04fc4116fd0683",
  sha256: "fe22026f68bdb3292fab376756035496ce0a35e3d580e06ebaa6a28295916eb3",
});

const DEFAULT_ROOT = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "..",
  "..",
);

function contract(condition, message) {
  if (!condition) {
    throw new Error(`Windows NSIS contract violation: ${message}`);
  }
}

const GZIP_OS_OFFSET = 9;
const GZIP_OS_UNKNOWN = 255;

export function canonicalizeGzipHeader(compressed) {
  const canonical = Buffer.from(compressed);
  contract(
    canonical.length > GZIP_OS_OFFSET &&
      canonical[0] === 0x1f &&
      canonical[1] === 0x8b &&
      canonical[2] === 0x08,
    "canonical gzip input must contain a complete deflate header",
  );
  // RFC 1952 makes the OS byte descriptive only. Node/zlib writes a
  // host-specific value, so freeze it to "unknown" before byte comparison.
  canonical[GZIP_OS_OFFSET] = GZIP_OS_UNKNOWN;
  return canonical;
}

export function gzipDeterministically(payload) {
  return canonicalizeGzipHeader(gzipSync(payload, { level: 9 }));
}

function readJson(filePath, label) {
  let source;
  try {
    source = fs.readFileSync(filePath, "utf8");
  } catch (error) {
    throw new Error(`Unable to read ${label} at ${filePath}: ${error.message}`);
  }

  try {
    return JSON.parse(source);
  } catch (error) {
    throw new Error(`${label} is not valid JSON: ${error.message}`);
  }
}

function normalizedLines(source) {
  return source.replaceAll("\r\n", "\n").replaceAll("\r", "\n").split("\n");
}

const nativePowerShellValidatedLoaders = new Set();

function assertPowerShell51LoaderContract(loader, loaderPath, chunkCount) {
  contract(
    loader.includes(
      "$c=[byte[]][Convert]::FromBase64String($e);$m=[IO.MemoryStream]::new($c)",
    ),
    "WebView2 loader must use the PowerShell 5.1-safe byte-array MemoryStream constructor",
  );
  contract(
    !/\[IO\.MemoryStream\]::new\(\s*,/u.test(loader),
    "WebView2 loader must not use a unary-comma method argument rejected by PowerShell 5.1",
  );
  contract(
    loader.includes(
      "[IO.Compression.GZipStream]::new($m,[IO.Compression.CompressionMode]0)",
    ) && !/\[IO\.Compression\.GZipStream\]::new\(\$m,\s*0\)/u.test(loader),
    "WebView2 loader must use a typed PowerShell 5.1-safe GZip decompression mode",
  );
  const callOperators = loader.match(/&/gu) ?? [];
  contract(
    !/(?:\bNew-Object\b|\bImport-Module\b|\bInvoke-Expression\b|\biex\b)/iu.test(
      loader,
    ) &&
      callOperators.length === 1 &&
      loader.includes("&([ScriptBlock]::Create("),
    "WebView2 loader must not use module-resolved or unconstrained indirect commands",
  );
  switch (process.platform) {
    case "darwin":
      return;
    case "win32":
      break;
    default:
      throw new Error(
        `Unsupported release verification host: ${process.platform}`,
      );
  }

  const loaderDigest = createHash("sha256").update(loader).digest("hex");
  if (nativePowerShellValidatedLoaders.has(loaderDigest)) return;
  const systemRoot = process.env.SystemRoot;
  contract(
    typeof systemRoot === "string" && path.isAbsolute(systemRoot),
    "native Windows validation requires an absolute SystemRoot",
  );
  const windowsPowerShell = path.join(
    systemRoot,
    "System32",
    "WindowsPowerShell",
    "v1.0",
    "powershell.exe",
  );
  contract(
    fs.statSync(windowsPowerShell).isFile(),
    "native Windows validation requires system Windows PowerShell 5.1",
  );
  const parseScript = [
    "$ErrorActionPreference='Stop'",
    "if ($PSVersionTable.PSVersion.Major -ne 5 -or $PSVersionTable.PSVersion.Minor -ne 1) { exit 95 }",
    "$tokens=$null",
    "$errors=$null",
    "[Management.Automation.Language.Parser]::ParseFile($env:FYAGENT_PS51_LOADER_PATH,[ref]$tokens,[ref]$errors)|Out-Null",
    "if ($errors.Count -ne 0) { [Console]::Error.Write($errors[0].Message); exit 96 }",
  ].join(";");
  const parseResult = spawnSync(
    windowsPowerShell,
    ["-NoLogo", "-NoProfile", "-NonInteractive", "-Command", parseScript],
    {
      encoding: "utf8",
      env: { ...process.env, FYAGENT_PS51_LOADER_PATH: loaderPath },
      stdio: ["ignore", "pipe", "pipe"],
      windowsHide: true,
    },
  );
  contract(
    !parseResult.error && parseResult.status === 0,
    `system Windows PowerShell 5.1 rejected the WebView2 loader AST (${parseResult.stderr.trim() || parseResult.status})`,
  );

  const controlledSource =
    "if ($PSVersionTable.PSVersion.Major -ne 5 -or $PSVersionTable.PSVersion.Minor -ne 1) { exit 95 };return 'FYAGENT_PS51_LOADER_OK'";
  const controlledPayload = gzipDeterministically(
    Buffer.from(controlledSource, "utf16le"),
  ).toString("base64");
  const executionEnvironment = { ...process.env };
  for (const name of Object.keys(executionEnvironment)) {
    if (/^FY_WV2_\d+$/u.test(name)) delete executionEnvironment[name];
  }
  for (let index = 0; index < chunkCount; index += 1) {
    executionEnvironment[`FY_WV2_${index}`] =
      index === 0 ? controlledPayload : "";
  }
  const encodedLoader = Buffer.from(loader, "utf16le").toString("base64");
  const executionResult = spawnSync(
    windowsPowerShell,
    [
      "-NoLogo",
      "-NoProfile",
      "-NonInteractive",
      "-ExecutionPolicy",
      "Bypass",
      "-EncodedCommand",
      encodedLoader,
    ],
    {
      encoding: "utf8",
      env: executionEnvironment,
      stdio: ["ignore", "pipe", "pipe"],
      windowsHide: true,
    },
  );
  contract(
    !executionResult.error &&
      executionResult.status === 0 &&
      executionResult.stdout.trim() === "FYAGENT_PS51_LOADER_OK",
    `system Windows PowerShell 5.1 could not execute the controlled WebView2 loader fixture (${executionResult.stderr.trim() || executionResult.status})`,
  );
  nativePowerShellValidatedLoaders.add(loaderDigest);
}

// NSIS supports line comments, trailing semicolons, and C-style block comments,
// while comment markers inside quoted SDDL/command strings are data. Security
// contracts use only the executable projection so comments cannot satisfy a gate.
export function stripNsisComments(source) {
  let insideBlockComment = false;
  return normalizedLines(source)
    .map((line) => {
      let quote = null;
      let executable = "";
      for (let index = 0; index < line.length; index += 1) {
        const character = line[index];
        if (insideBlockComment) {
          if (character === "*" && line[index + 1] === "/") {
            insideBlockComment = false;
            index += 1;
          }
          continue;
        }
        if (quote !== null) {
          executable += character;
          if (character === quote && line[index - 1] !== "$") {
            quote = null;
          }
          continue;
        }
        if (character === '"' || character === "'" || character === "`") {
          quote = character;
          executable += character;
          continue;
        }
        if (character === "/" && line[index + 1] === "*") {
          insideBlockComment = true;
          index += 1;
          continue;
        }
        if (character === ";") {
          break;
        }
        if (character === "#" && executable.trim() === "") {
          break;
        }
        executable += character;
      }
      return executable;
    })
    .join("\n");
}

function sectionName(declaration) {
  const remainder = declaration.replace(/^Section(?:\s+\/o)?\s+/, "").trim();
  const quoted = remainder.match(/^"([^"]+)"/);
  return quoted ? quoted[1] : remainder.split(/\s+/u, 1)[0];
}

export function parseNsisBlocks(source) {
  const lines = normalizedLines(source);
  const blocks = [];

  for (let index = 0; index < lines.length; index += 1) {
    const trimmed = lines[index].trim();
    const functionMatch = trimmed.match(/^Function\s+([^\s;]+)\s*$/u);
    const sectionMatch = trimmed.match(/^Section(?:\s+\/o)?\s+.+$/u);
    if (!functionMatch && !sectionMatch) {
      continue;
    }

    const kind = functionMatch ? "function" : "section";
    const endToken = functionMatch ? "FunctionEnd" : "SectionEnd";
    const end = lines.findIndex(
      (line, candidate) => candidate > index && line.trim() === endToken,
    );
    contract(end > index, `${kind} at line ${index + 1} has no ${endToken}`);

    blocks.push({
      kind,
      name: functionMatch ? functionMatch[1] : sectionName(trimmed),
      startLine: index + 1,
      endLine: end + 1,
      body: lines.slice(index + 1, end).join("\n"),
    });
    index = end;
  }

  return blocks;
}

function namedBlock(blocks, kind, name) {
  const matches = blocks.filter(
    (block) => block.kind === kind && block.name === name,
  );
  contract(matches.length === 1, `expected exactly one ${kind} ${name}`);
  return matches[0];
}

function assertOrdered(source, tokens, label) {
  let cursor = -1;
  for (const token of tokens) {
    const index = source.indexOf(token, cursor + 1);
    contract(index >= 0, `${label} is missing ${token}`);
    contract(index > cursor, `${label} has ${token} out of order`);
    cursor = index;
  }
}

function nonEmptyTrimmedLines(source) {
  return source
    .split(/\r?\n/u)
    .map((line) => line.trim())
    .filter(Boolean);
}

function assertNormalizedNsisDigest(source, expectedSha256, label) {
  const digest = createHash("sha256")
    .update(nonEmptyTrimmedLines(source).join("\n"))
    .digest("hex");
  contract(
    digest === expectedSha256,
    `${label} must retain its exact resource, branch, label, and fallthrough control flow`,
  );
}

export function assertInstallPathPolicyContract(
  source,
  repoOwnedIncludeSources = [],
) {
  const blocks = parseNsisBlocks(source);
  const executableSource = stripNsisComments(source);
  const executableRepoOwnedIncludes = repoOwnedIncludeSources.map((include) =>
    stripNsisComments(include),
  );
  const executableClosure = [
    executableSource,
    ...executableRepoOwnedIncludes,
  ].join("\n");

  for (const forbidden of [
    "FyAgentValidateFinalInstallDir",
    "FyAgentValidateInstallDirPageLeave",
    "-FyAgentInstallDirGate",
    "fyagentInvalidInstallDir",
    "FYAGENT_DRIVE_FIXED",
    "GetFullPathNameW",
    "GetFinalPathNameByHandleW",
    "GetVolumePathNameW",
    "GetDriveTypeW",
  ]) {
    contract(
      !executableClosure.includes(forbidden),
      `installer must not reintroduce custom installation-path restriction ${forbidden}`,
    );
  }
  for (const executableInclude of executableRepoOwnedIncludes) {
    const installDirUse = executableInclude
      .split("\n")
      .map((line) => line.trim())
      .find((line) => /\$INSTDIR\b/iu.test(line));
    contract(
      installDirUse === undefined,
      `repo-owned NSIS include/hook must not inspect or rewrite $INSTDIR: ${installDirUse}`,
    );
  }
  const directoryPage = "!insertmacro MUI_PAGE_DIRECTORY";
  const directoryPageIndex = executableSource.indexOf(directoryPage);
  contract(
    directoryPageIndex >= 0 &&
      executableSource.indexOf(directoryPage, directoryPageIndex + 1) < 0,
    "installer must retain exactly one standard NSIS directory page",
  );
  const precedingPageIndex = Math.max(
    executableSource.lastIndexOf(
      "!insertmacro MUI_PAGE_",
      directoryPageIndex - 1,
    ),
    executableSource.lastIndexOf(
      "!insertmacro MUI_UNPAGE_",
      directoryPageIndex - 1,
    ),
  );
  const directoryPageDeclaration = executableSource.slice(
    precedingPageIndex < 0 ? 0 : precedingPageIndex,
    directoryPageIndex + directoryPage.length,
  );
  contract(
    !directoryPageDeclaration.includes("MUI_PAGE_CUSTOMFUNCTION_LEAVE"),
    "directory page must not bind a custom leave-time path gate",
  );
  contract(
    directoryPageDeclaration.includes(
      "!define MUI_PAGE_CUSTOMFUNCTION_PRE SkipIfPassive",
    ),
    "standard NSIS directory page must retain its passive-mode pre callback",
  );

  for (const initName of [".onInit", "un.onInit"]) {
    const init = stripNsisComments(
      namedBlock(blocks, "function", initName).body,
    );
    contract(
      init.includes("SetRegView 64"),
      `${initName} must select the native 64-bit registry view`,
    );
  }

  const sections = blocks.filter((block) => block.kind === "section");
  contract(
    JSON.stringify(sections.map((block) => block.name)) ===
      JSON.stringify(["EarlyChecks", "WebView2", "Install", "Uninstall"]),
    "installer sections must not add a custom installation-path gate",
  );

  const webviewIndex = sections.findIndex((block) => block.name === "WebView2");
  const installIndex = sections.findIndex((block) => block.name === "Install");
  contract(webviewIndex > 0, "WebView2 section must follow EarlyChecks");
  contract(installIndex > webviewIndex, "Install section must follow WebView2");
  contract(
    stripNsisComments(namedBlock(blocks, "section", "Install").body).includes(
      "SetOutPath $INSTDIR",
    ),
    "Install section must select the user-chosen output path",
  );

  const installDirLines = executableSource
    .split(/\r?\n/u)
    .map((line) => line.trim())
    .filter((line) => line.includes("$INSTDIR"));
  const allowedInstallDirLinePatterns = [
    /^\$\{OrIf\} \$\{FileExists\} "\$INSTDIR\\\$\{MAINBINARYNAME\}\.exe"$/u,
    /^nsis_tauri_utils::RunAsUser "\$INSTDIR\\\$\{MAINBINARYNAME\}\.exe" (?:""|"\$R0")$/u,
    /^\$\{If\} \$INSTDIR == "\$\{PLACEHOLDER_INSTALL_DIR\}"$/u,
    /^StrCpy \$INSTDIR (?:"\$(?:PROGRAMFILES64|PROGRAMFILES|LOCALAPPDATA)\\\$\{PRODUCTNAME\}"|\$4)$/u,
    /^StrCpy \$INSTDIR \$LegacyWixInstallDir$/u,
    /^SetOutPath \$INSTDIR$/u,
    /^CreateDirectory "\$INSTDIR\\\\\{\{this\}\}"$/u,
    /^!insertmacro FyAgentOpenCleanupAnchorDirectory "\$INSTDIR\\cache" \$\{Label\}_cache \$5 \$9$/u,
    /^FindFirst \$R0 \$R1 "\$INSTDIR\\cache\\codex-installer\\\*"$/u,
    /^!insertmacro APP_ASSOCIATE .+ "\$INSTDIR\\\$\{MAINBINARYNAME\}\.exe,0" .+ "\$INSTDIR\\\$\{MAINBINARYNAME\}\.exe \$\\"%1\$\\""$/u,
    /^WriteRegStr SHCTX .+\$INSTDIR.*$/u,
    /^WriteUninstaller "\$INSTDIR\\uninstall\.exe"$/u,
    /^Delete "\$INSTDIR\\.+"$/u,
    /^\$\{GetSize\} "\$INSTDIR" "\/M=uninstall\.exe \/S=0K \/G=0" \$0 \$1 \$2$/u,
    /^\$\{If\} \$R7 == "\$\\"\$INSTDIR\\\$\{MAINBINARYNAME\}\.exe\$\\" \$\\"%1\$\\""$/u,
    /^RMDir "\$INSTDIR\\cache(?:\\codex-installer(?:\\\$R1)?)?"$/u,
    /^RMDir(?: \/(?:REBOOTOK|r))? "\$INSTDIR(?:\\\\\{\{this\}\})?"$/u,
    /^!insertmacro (?:IsShortcutTarget|SetShortcutTarget) .+ "\$INSTDIR\\(?:\$OldMainBinaryName|\$\{MAINBINARYNAME\}\.exe)"$/u,
    /^CreateShortcut .+ "\$INSTDIR\\\$\{MAINBINARYNAME\}\.exe"$/u,
  ];
  for (const line of installDirLines) {
    contract(
      allowedInstallDirLinePatterns.some((pattern) => pattern.test(line)),
      `installer must not use $INSTDIR for custom path admission: ${line}`,
    );
  }
  contract(
    installDirLines.filter((line) => line === "SetOutPath $INSTDIR").length ===
      1,
    "installer must select the user-chosen output path exactly once",
  );
  contract(
    installDirLines.filter((line) => line.startsWith("StrCpy $INSTDIR "))
      .length === 7,
    "installer must not rewrite the user-chosen path outside default/maintenance restoration",
  );

  const restorePreviousInstallLocation = stripNsisComments(
    namedBlock(blocks, "function", "RestorePreviousInstallLocation").body,
  )
    .split("\n")
    .map((line) => line.trim())
    .filter(Boolean);
  contract(
    JSON.stringify(restorePreviousInstallLocation) ===
      JSON.stringify([
        'ReadRegStr $4 SHCTX "${MANUPRODUCTKEY}" ""',
        '${If} $4 != ""',
        "StrCpy $INSTDIR $4",
        '${ElseIf} $LegacyWixInstallDir != ""',
        "StrCpy $INSTDIR $LegacyWixInstallDir",
        "${EndIf}",
      ]),
    "RestorePreviousInstallLocation must prefer the current NSIS path, fall back to the captured v0.3.0 MSI path, and copy either value verbatim",
  );

  const reinstall = stripNsisComments(
    namedBlock(blocks, "function", "PageLeaveReinstall").body,
  );
  contract(
    reinstall.includes("ExecWait '$R1' $0"),
    "maintenance flow must invoke the existing NSIS uninstaller",
  );
  contract(
    !/(?:GetDriveTypeW|FyAgentValidateFinalInstallDir|-FyAgentInstallDirGate)/u.test(
      reinstall,
    ),
    "maintenance flow must not reintroduce the retired path restriction",
  );
  const registryInstallPathAliasUses = reinstall
    .split("\n")
    .map((line) => line.trim())
    .filter((line) => /\$4\b/u.test(line));
  contract(
    JSON.stringify(registryInstallPathAliasUses) ===
      JSON.stringify([
        'ReadRegStr $4 SHCTX "${MANUPRODUCTKEY}" ""',
        'StrCpy $R1 "$R1 _?=$4"',
      ]),
    "maintenance registry install-path alias $4 may only be passed to the existing uninstaller",
  );
}

function assertCanonicalIconContract(source, repoOwnedIncludeSources) {
  const sources = [
    { label: "template", source },
    ...repoOwnedIncludeSources.map((include, index) => ({
      label: `repo-owned include ${index + 1}`,
      source: include,
    })),
  ];
  const directives = [];

  for (const candidate of sources) {
    for (const line of stripNsisComments(candidate.source).split("\n")) {
      const match = line.match(/^\s*!(define|undef)\b(.*)$/iu);
      if (!match) continue;
      const tokens = match[2].trim().split(/\s+/u);
      while (tokens[0]?.startsWith("/")) tokens.shift();
      const symbol = tokens[0]?.toUpperCase();
      if (symbol !== "MUI_ICON" && symbol !== "MUI_UNICON") continue;
      directives.push({
        kind: match[1].toLowerCase(),
        label: candidate.label,
        line: line.trim(),
        symbol,
      });
    }
  }

  for (const symbol of ["MUI_ICON", "MUI_UNICON"]) {
    const matches = directives.filter(
      (directive) => directive.symbol === symbol,
    );
    contract(
      matches.length === 1 &&
        matches[0].kind === "define" &&
        matches[0].label === "template" &&
        matches[0].line === `!define ${symbol} "\${INSTALLERICON}"`,
      `installer and uninstaller must each have exactly one canonical FyAgent icon definition across the repo-owned NSIS closure (${symbol})`,
    );
  }
}

// Reviewed line-start runtime macro inventory for the installer template and
// its repository-owned hook. A new entry needs source and include-order review
// because NSIS define expansion can otherwise construct a compiler directive.
const REPO_OWNED_NSIS_LINE_START_MACROS = new Set([
  "AndIf",
  "Else",
  "ElseIf",
  "EndIf",
  "GetOptions",
  "GetSize",
  "If",
  "IfNot",
  "IfThen",
  "NSD_CreateLabel",
  "NSD_CreateRadioButton",
  "NSD_GetState",
  "NSD_OnClick",
  "NSD_SetFocus",
  "OrIf",
  "VersionCompare",
]);
const REPO_OWNED_NSIS_LINE_START_MACROS_UPPER = new Set(
  [...REPO_OWNED_NSIS_LINE_START_MACROS].map((name) => name.toUpperCase()),
);

// This is deliberately narrower than a general NSIS tokenizer. Current
// repository-owned declarations use only bare /options and literal names, so
// quoted, escaped, empty, or dynamically constructed declaration tokens fail
// closed before they can redefine an inventoried line-start runtime macro.
function parseRepoOwnedNsisDeclaration(line) {
  const declaration = line.match(/^\s*!(define|macro)\b(.*)$/iu);
  if (!declaration) return null;

  const kind = declaration[1].toLowerCase();
  let remainder = declaration[2].trim();
  while (true) {
    const token = remainder.match(/^\S+/u)?.[0] ?? "";
    const unsafeToken = token === "" || /["'`$]/u.test(token);
    if (unsafeToken) return { name: token, unsafe: true };

    remainder = remainder.slice(token.length).trimStart();
    if (kind === "define" && token.startsWith("/")) continue;
    return { name: token, unsafe: false };
  }
}

function assertWarning6000PackagingContract(source, repoOwnedIncludeSources) {
  const canonicalDirective = "!pragma warning error 6000";
  const executableTemplateLines = stripNsisComments(source)
    .split("\n")
    .map((line) => line.trim())
    .filter(Boolean);
  const canonicalOpening = [
    "Unicode true",
    canonicalDirective,
    "ManifestDPIAware true",
  ];
  contract(
    canonicalOpening.every(
      (line, index) => executableTemplateLines[index] === line,
    ),
    "warning 6000 protection must be the canonical top-level template directive",
  );

  const sources = [
    { label: "template", source },
    ...repoOwnedIncludeSources.map((include, index) => ({
      label: `repo-owned include ${index + 1}`,
      source: include,
    })),
  ];
  const directives = [];
  const dynamicDirectiveNames = [];
  const unreviewedLineStartMacros = [];
  const unsafeOrProtectedMacroDeclarations = [];

  for (const candidate of sources) {
    for (const line of stripNsisComments(candidate.source).split("\n")) {
      if (/^\s*!\$\{/u.test(line)) {
        dynamicDirectiveNames.push({
          label: candidate.label,
          line: line.trim(),
        });
      }
      const lineStartMacro = line.match(/^\s*\$\{([^}]+)\}/u);
      if (
        lineStartMacro &&
        !REPO_OWNED_NSIS_LINE_START_MACROS.has(lineStartMacro[1])
      ) {
        unreviewedLineStartMacros.push({
          label: candidate.label,
          line: line.trim(),
        });
      }
      const declaration = parseRepoOwnedNsisDeclaration(line);
      if (declaration) {
        if (
          declaration.unsafe ||
          REPO_OWNED_NSIS_LINE_START_MACROS_UPPER.has(
            declaration.name.toUpperCase(),
          )
        ) {
          unsafeOrProtectedMacroDeclarations.push({
            label: candidate.label,
            line: line.trim(),
          });
        }
      }
      if (!/^\s*!pragma(?:\s|$)/iu.test(line)) continue;
      directives.push({ label: candidate.label, line: line.trim() });
    }
  }

  contract(
    dynamicDirectiveNames.length === 0,
    "dynamic NSIS preprocessor directive names are forbidden across the repo-owned executable closure",
  );
  contract(
    unreviewedLineStartMacros.length === 0,
    "only reviewed runtime macros may appear at line start across the repo-owned executable closure",
  );
  contract(
    unsafeOrProtectedMacroDeclarations.length === 0,
    "repo-owned declarations must use literal unquoted names and must not redefine reviewed line-start runtime macro names",
  );
  contract(
    directives.length === 1 &&
      directives[0].label === "template" &&
      directives[0].line === canonicalDirective,
    "NSIS warning 6000 must remain an error across the repo-owned executable closure",
  );

  const hookInclude = '!include "{{installer_hooks}}"';
  const hookIncludeIndices = executableTemplateLines.flatMap((line, index) =>
    line === hookInclude ? [index] : [],
  );
  let preprocessorDepth = 0;
  let hookIncludeDepth = null;
  for (const [index, line] of executableTemplateLines.entries()) {
    if (/^!(?:endif|macroend)\b/iu.test(line)) preprocessorDepth -= 1;
    if (index === hookIncludeIndices[0]) hookIncludeDepth = preprocessorDepth;
    if (/^!(?:if|ifdef|ifndef|ifmacrodef|ifmacrondef|macro)\b/iu.test(line)) {
      preprocessorDepth += 1;
    }
  }
  contract(
    hookIncludeIndices.length === 1 &&
      hookIncludeIndices[0] > 1 &&
      hookIncludeDepth === 0,
    "repo-owned installer hook include must appear exactly once at top level after warning 6000 protection",
  );
}

function assertProcessStopGateContract(source, blocks) {
  const executableSource = stripNsisComments(source);
  const gateMatch = executableSource.match(
    /!macro FyAgentRequireProcessStopped ExecutableName DisplayName Label([\s\S]*?)!macroend/u,
  );
  contract(
    gateMatch !== null &&
      (
        executableSource.match(
          /^\s*!macro\s+FyAgentRequireProcessStopped\b/gmu,
        ) ?? []
      ).length === 1,
    "process stop gate must have exactly one local macro definition",
  );
  const gateDefinitionIndex = executableSource.indexOf(
    "!macro FyAgentRequireProcessStopped ExecutableName DisplayName Label",
  );
  const firstGateInvocationIndex = executableSource.indexOf(
    "!insertmacro FyAgentRequireProcessStopped ",
  );
  contract(
    gateDefinitionIndex >= 0 &&
      firstGateInvocationIndex >= 0 &&
      gateDefinitionIndex < firstGateInvocationIndex,
    "process stop gate macro must be defined before its first invocation",
  );
  const expectedGateLines = [
    "fyagent_${Label}_process_retry:",
    '!if "${INSTALLMODE}" == "currentUser"',
    'nsis_tauri_utils::FindProcessCurrentUser "${ExecutableName}"',
    "!else",
    'nsis_tauri_utils::FindProcess "${ExecutableName}"',
    "!endif",
    "Pop $R0",
    "${If} $R0 = 0",
    "IfSilent fyagent_${Label}_process_silent fyagent_${Label}_process_interactive",
    "fyagent_${Label}_process_interactive:",
    "${If} $PassiveMode = 1",
    "Goto fyagent_${Label}_process_silent",
    "${EndIf}",
    'MessageBox MB_ICONEXCLAMATION|MB_RETRYCANCEL "Close ${DisplayName} normally before continuing. Choose Retry after it has exited." IDRETRY fyagent_${Label}_process_retry IDCANCEL fyagent_${Label}_process_cancel',
    "fyagent_${Label}_process_cancel:",
    'Abort "${DisplayName} is still running. No installer changes were made."',
    "fyagent_${Label}_process_silent:",
    'Abort "${DisplayName} is running. Close it normally, then run setup again."',
    "${EndIf}",
  ];
  contract(
    JSON.stringify(nonEmptyTrimmedLines(gateMatch?.[1] ?? "")) ===
      JSON.stringify(expectedGateLines),
    "process stop gate must retain exact find-only, retry/cancel, passive/silent Abort control flow",
  );
  contract(
    !/(?:CheckIfAppIsRunning|KillProcess(?:CurrentUser)?|TerminateProcess|\btaskkill(?:\.exe)?\b)/iu.test(
      executableSource,
    ),
    "installer lifecycle must never force-terminate the main or helper process",
  );

  const expectedInvocations = [
    '!insertmacro FyAgentRequireProcessStopped "${MAINBINARYNAME}.exe" "${PRODUCTNAME}" maintenance_main',
    '!insertmacro FyAgentRequireProcessStopped "fyagent-user-helper.exe" "${PRODUCTNAME} user helper" maintenance_helper',
    '!insertmacro FyAgentRequireProcessStopped "${MAINBINARYNAME}.exe" "${PRODUCTNAME}" early_main',
    '!insertmacro FyAgentRequireProcessStopped "fyagent-user-helper.exe" "${PRODUCTNAME} user helper" early_helper',
    '!insertmacro FyAgentRequireProcessStopped "${MAINBINARYNAME}.exe" "${PRODUCTNAME}" install_main',
    '!insertmacro FyAgentRequireProcessStopped "fyagent-user-helper.exe" "${PRODUCTNAME} user helper" install_helper',
    '!insertmacro FyAgentRequireProcessStopped "${MAINBINARYNAME}.exe" "${PRODUCTNAME}" uninstall_main',
    '!insertmacro FyAgentRequireProcessStopped "fyagent-user-helper.exe" "${PRODUCTNAME} user helper" uninstall_helper',
  ];
  const invocations = nonEmptyTrimmedLines(executableSource).filter((line) =>
    line.startsWith("!insertmacro FyAgentRequireProcessStopped "),
  );
  contract(
    JSON.stringify(invocations) === JSON.stringify(expectedInvocations),
    "main and fixed helper process gates must appear in maintenance, pre-WebView2 early checks, install, and uninstall paths",
  );

  const maintenance = stripNsisComments(
    namedBlock(blocks, "function", "PageLeaveReinstall").body,
  );
  assertOrdered(
    maintenance,
    [
      "reinst_uninstall:",
      expectedInvocations[0],
      expectedInvocations[1],
      "HideWindow",
      "ClearErrors",
      "ExecWait '$R1' $0",
    ],
    "maintenance uninstall process gate",
  );

  const early = stripNsisComments(
    namedBlock(blocks, "section", "EarlyChecks").body,
  );
  assertOrdered(
    early,
    [
      expectedInvocations[2],
      expectedInvocations[3],
      "Call FyAgentMigrateLegacyWixInstall",
    ],
    "pre-WebView2 process gate before MSI migration",
  );

  const install = stripNsisComments(
    namedBlock(blocks, "section", "Install").body,
  );
  assertOrdered(
    install,
    [
      expectedInvocations[4],
      expectedInvocations[5],
      "SetOutPath $INSTDIR",
      "!insertmacro NSIS_HOOK_PREINSTALL",
      "!insertmacro FyAgentCleanupLegacyMachineRuntime install_legacy_runtime",
      'File "${MAINBINARYSRCPATH}"',
    ],
    "install process recheck before every install payload mutation",
  );

  const uninstall = stripNsisComments(
    namedBlock(blocks, "section", "Uninstall").body,
  );
  assertOrdered(
    uninstall,
    [
      expectedInvocations[6],
      expectedInvocations[7],
      "!insertmacro NSIS_HOOK_PREUNINSTALL",
      "!insertmacro FyAgentCleanupLegacyMachineRuntime uninstall_legacy_runtime",
      'Delete "$INSTDIR\\${MAINBINARYNAME}.exe"',
    ],
    "uninstall process gate before hooks, cleanup, and owned payload deletion",
  );
}

function assertLegacyWixMigrationContract(source, blocks) {
  const executableSource = stripNsisComments(source);
  const executableLines = nonEmptyTrimmedLines(executableSource);
  for (const definition of [
    '!define FYAGENT_LEGACY_WIX_REGISTRY_KEY "Software\\fyagent\\FyAgent"',
    "!define FYAGENT_INSTALLSTATE_UNKNOWN -1",
    "!define FYAGENT_MSI_SUCCESS 0",
    "!define FYAGENT_MSI_UNKNOWN_PRODUCT 1605",
    "!define FYAGENT_MSI_PRODUCT_UNINSTALLED 1614",
    "!define FYAGENT_MSI_REBOOT_REQUIRED 3010",
    '!define FYAGENT_LEGACY_WIX_PRODUCT_CODE "{D50D8CE2-B49A-41DE-839D-6574FB69ADC1}"',
    '!define FYAGENT_LEGACY_WIX_PRODUCT_CODE "{78F69296-A73D-40CA-A2BA-11D117AA2C9B}"',
  ]) {
    contract(
      executableLines.filter((line) => line === definition).length === 1,
      `v0.3.0 MSI migration must pin ${definition}`,
    );
  }
  assertOrdered(
    executableSource,
    [
      '!if "${ARCH}" == "x64"',
      '!define FYAGENT_LEGACY_WIX_PRODUCT_CODE "{D50D8CE2-B49A-41DE-839D-6574FB69ADC1}"',
      '!else if "${ARCH}" == "arm64"',
      '!define FYAGENT_LEGACY_WIX_PRODUCT_CODE "{78F69296-A73D-40CA-A2BA-11D117AA2C9B}"',
      "!else",
      '!error "FyAgent\'s frozen v0.3.0 MSI migration supports only x64 and arm64"',
      "!endif",
    ],
    "architecture-specific frozen v0.3.0 MSI ProductCode selection",
  );

  const init = stripNsisComments(
    namedBlock(blocks, "function", ".onInit").body,
  );
  assertOrdered(
    init,
    [
      "SetRegView 64",
      "!insertmacro SetContext",
      'StrCpy $LegacyWixInstallDir ""',
      'ReadRegStr $LegacyWixInstallDir HKLM "${FYAGENT_LEGACY_WIX_REGISTRY_KEY}" "InstallDir"',
      "ClearErrors",
      '${If} $INSTDIR == "${PLACEHOLDER_INSTALL_DIR}"',
      "Call RestorePreviousInstallLocation",
    ],
    "v0.3.0 MSI install-path capture",
  );
  contract(
    (
      init.match(
        /^\s*ReadRegStr\s+\$LegacyWixInstallDir\s+HKLM\s+"\$\{FYAGENT_LEGACY_WIX_REGISTRY_KEY\}"\s+"InstallDir"\s*$/gmu,
      ) ?? []
    ).length === 1 &&
      init.includes(
        '    Call RestorePreviousInstallLocation\n  ${EndIf}\n\n\n  !if "${INSTALLMODE}" == "both"',
      ),
    "v0.3.0 MSI path must be captured once and used only as a placeholder fallback so explicit /D= remains authoritative",
  );

  const migration = stripNsisComments(
    namedBlock(blocks, "function", "FyAgentMigrateLegacyWixInstall").body,
  );
  const expectedMigrationLines = [
    'System::Call \'"$SYSDIR\\msi.dll"::MsiQueryProductStateW(w "${FYAGENT_LEGACY_WIX_PRODUCT_CODE}") i .r0\'',
    "${If} $0 == ${FYAGENT_INSTALLSTATE_UNKNOWN}",
    "Goto fyagent_legacy_wix_migration_accepted",
    "${EndIf}",
    "ClearErrors",
    "ExecWait '\"$SYSDIR\\msiexec.exe\" /x ${FYAGENT_LEGACY_WIX_PRODUCT_CODE} /qn /norestart' $0",
    "${If} ${Errors}",
    'MessageBox MB_ICONSTOP|MB_OK "FyAgent Setup could not start Windows Installer to remove the previous FyAgent version. No files were changed."',
    'Abort "Close other installers and run FyAgent Setup again."',
    "${EndIf}",
    "${If} $0 == ${FYAGENT_MSI_SUCCESS}",
    "${OrIf} $0 == ${FYAGENT_MSI_UNKNOWN_PRODUCT}",
    "${OrIf} $0 == ${FYAGENT_MSI_PRODUCT_UNINSTALLED}",
    "Goto fyagent_legacy_wix_migration_accepted",
    "${EndIf}",
    "${If} $0 == ${FYAGENT_MSI_REBOOT_REQUIRED}",
    'MessageBox MB_ICONSTOP|MB_OK "The previous FyAgent version requires a Windows restart to finish uninstalling. Restart Windows, then run FyAgent Setup again. No new files were installed."',
    'Abort "Restart Windows before installing FyAgent."',
    "${EndIf}",
    'MessageBox MB_ICONSTOP|MB_OK "FyAgent Setup could not remove the previous FyAgent version (Windows Installer code $0). No new files were installed."',
    'Abort "Resolve the previous uninstall error, then run FyAgent Setup again."',
    "fyagent_legacy_wix_migration_accepted:",
    'DeleteRegValue HKLM "${FYAGENT_LEGACY_WIX_REGISTRY_KEY}" "InstallDir"',
    "ClearErrors",
  ];
  contract(
    JSON.stringify(nonEmptyTrimmedLines(migration)) ===
      JSON.stringify(expectedMigrationLines),
    "v0.3.0 MSI migration must retain exact query, fixed uninstall, accepted-code, fail-closed, and marker cleanup control flow",
  );
  contract(
    (migration.match(/MsiQueryProductStateW/gu) ?? []).length === 1 &&
      (migration.match(/msiexec\.exe/giu) ?? []).length === 1 &&
      migration.includes(
        'System::Call \'"$SYSDIR\\msi.dll"::MsiQueryProductStateW',
      ) &&
      !/(?:^|[\s'])msi::MsiQueryProductStateW/imu.test(migration) &&
      !/(?:UninstallString|ReadRegStr|EnumReg|LegacyWixInstallDir|ExecShell|nsExec|PowerShell|cmd\.exe)/iu.test(
        migration,
      ),
    "v0.3.0 MSI migration may query only the System-directory MSI library and uninstall only the frozen ProductCode without registry enumeration or a dynamic command",
  );
  contract(
    JSON.stringify(
      executableLines.filter((line) => /msiexec(?:\.exe)?/iu.test(line)),
    ) ===
      JSON.stringify([
        "ExecWait '\"$SYSDIR\\msiexec.exe\" /x ${FYAGENT_LEGACY_WIX_PRODUCT_CODE} /qn /norestart' $0",
      ]) &&
      (executableSource.match(/MsiQueryProductStateW/gu) ?? []).length === 1 &&
      (executableSource.match(/D50D8CE2-B49A-41DE-839D-6574FB69ADC1/gu) ?? [])
        .length === 1 &&
      (executableSource.match(/78F69296-A73D-40CA-A2BA-11D117AA2C9B/gu) ?? [])
        .length === 1,
    "v0.3.0 MSI migration must be the only msiexec surface and may reference each frozen ProductCode exactly once",
  );

  const early = stripNsisComments(
    namedBlock(blocks, "section", "EarlyChecks").body,
  );
  const install = stripNsisComments(
    namedBlock(blocks, "section", "Install").body,
  );
  assertOrdered(
    executableSource,
    [
      "Section EarlyChecks",
      '!insertmacro FyAgentRequireProcessStopped "${MAINBINARYNAME}.exe" "${PRODUCTNAME}" early_main',
      '!insertmacro FyAgentRequireProcessStopped "fyagent-user-helper.exe" "${PRODUCTNAME} user helper" early_helper',
      "Call FyAgentMigrateLegacyWixInstall",
      "Section WebView2",
      "Section Install",
      '!insertmacro FyAgentRequireProcessStopped "${MAINBINARYNAME}.exe" "${PRODUCTNAME}" install_main',
      '!insertmacro FyAgentRequireProcessStopped "fyagent-user-helper.exe" "${PRODUCTNAME} user helper" install_helper',
      "SetOutPath $INSTDIR",
      "!insertmacro NSIS_HOOK_PREINSTALL",
      "!insertmacro FyAgentCleanupLegacyMachineRuntime install_legacy_runtime",
      "!insertmacro FyAgentCleanupKnownCodexInstallerStaging install_codex_staging",
      'File "${MAINBINARYSRCPATH}"',
      'WriteRegStr SHCTX "${MANUPRODUCTKEY}" "" $INSTDIR',
    ],
    "fail-closed v0.3.0 MSI migration before new NSIS payload mutation",
  );
  contract(
    (early.match(/^\s*Call\s+FyAgentMigrateLegacyWixInstall\s*$/gmu) ?? [])
      .length === 1 &&
      (
        executableSource.match(
          /^\s*Call\s+FyAgentMigrateLegacyWixInstall\s*$/gmu,
        ) ?? []
      ).length === 1 &&
      !install.includes("Call FyAgentMigrateLegacyWixInstall"),
    "v0.3.0 MSI migration must run exactly once in pre-WebView2 early checks",
  );
}

function assertLegacyRuntimeCleanupContract(source, blocks) {
  const executableSource = stripNsisComments(source);
  contract(
    !/\$COMMONAPPDATA\b/iu.test(executableSource),
    "legacy runtime cleanup must use the NSIS $COMMONPROGRAMDATA variable, not the unknown $COMMONAPPDATA token",
  );
  const runtimeRootAliases = [
    ...executableSource.matchAll(/\$[A-Z][A-Z0-9_]*\\FyAgent\b/giu),
  ].map((match) => match[0].toUpperCase());
  contract(
    runtimeRootAliases.length > 0 &&
      runtimeRootAliases.every(
        (alias) => alias === "$COMMONPROGRAMDATA\\FYAGENT",
      ),
    "legacy runtime paths must resolve through the exact NSIS $COMMONPROGRAMDATA variable",
  );

  for (const [name, nextToken] of [
    [".onInit", "Call RestorePreviousInstallLocation"],
    ["un.onInit", "!insertmacro MUI_UNGETLANGUAGE"],
  ]) {
    const init = stripNsisComments(namedBlock(blocks, "function", name).body);
    contract(
      (init.match(/^\s*!insertmacro\s+SetContext\s*$/gimu) ?? []).length === 1,
      `${name} must initialize the per-machine shell and registry context exactly once`,
    );
    assertOrdered(
      init,
      ["!insertmacro SetContext", nextToken],
      `${name} per-machine context initialization`,
    );
  }

  const anchorMatch = executableSource.match(
    /!macro FyAgentOpenCleanupAnchorDirectory Path Label OutputHandle ValidFlag([\s\S]*?)!macroend/u,
  );
  contract(anchorMatch, "missing fixed no-follow cleanup anchor macro");
  const anchorOpen = anchorMatch[1];
  assertNormalizedNsisDigest(
    anchorOpen,
    "138480de81d8eb65521fd5bcacb0e3959847d08d4f6b2d86d3af51c65a1e7941",
    "cleanup anchor",
  );
  const pinnedAnchorOpen =
    "System::Call 'kernel32::CreateFileW(w \"${Path}\", i ${FYAGENT_DELETE}|${FYAGENT_FILE_READ_ATTRIBUTES}, i ${FYAGENT_FILE_SHARE_READ}, p 0, i ${FYAGENT_OPEN_EXISTING}, i ${FYAGENT_FILE_FLAG_BACKUP_SEMANTICS}|${FYAGENT_FILE_FLAG_OPEN_REPARSE_POINT}, p 0) p .r8'";
  contract(
    nonEmptyTrimmedLines(anchorOpen).filter((line) => line === pinnedAnchorOpen)
      .length === 1,
    "cleanup anchor must issue exactly one pinned no-follow CreateFileW open",
  );
  assertOrdered(
    anchorOpen,
    [
      "StrCpy ${OutputHandle} 0",
      "StrCpy ${ValidFlag} 0",
      pinnedAnchorOpen,
      "GetFileInformationByHandle(p r8, p r6)",
      "FYAGENT_FILE_ATTRIBUTE_DIRECTORY",
      "FYAGENT_FILE_ATTRIBUTE_REPARSE_POINT",
      "StrCpy ${OutputHandle} $8",
      "StrCpy ${ValidFlag} 1",
      "Goto fyagent_${Label}_done",
      "fyagent_${Label}_close:",
      "CloseHandle(p r8)",
      "fyagent_${Label}_done:",
    ],
    "fixed cleanup anchor validation",
  );
  contract(
    !/(?:NtCreateFile|SetFileInformationByHandle|GetSecurityInfo|SetSecurityInfo|SetKernelObjectSecurity|SetNamedSecurityInfo|ConvertStringSecurityDescriptor|CreateDirectoryW|(?:^|\s)(?:Delete|DeleteFileW|RemoveDirectoryW|RMDir|Quit|Abort|SetErrorLevel|Return)(?:\s|$))/imu.test(
      anchorOpen,
    ),
    "cleanup anchor validation must remain no-follow, non-provisioning, and free of path mutation or early exits",
  );

  const relativeDirectoryMatch = executableSource.match(
    /!macro FyAgentOpenDirectoryRelativeToHandle ParentSystemRegister ParentHandle RelativeName Label OutputHandle ValidFlag([\s\S]*?)!macroend/u,
  );
  contract(
    relativeDirectoryMatch,
    "missing parent-handle-relative cleanup directory macro",
  );
  const relativeDirectoryOpen = relativeDirectoryMatch[1];
  assertNormalizedNsisDigest(
    relativeDirectoryOpen,
    "150dc8b3ae57b1362fe4bb999d004d9023a64f09142363e88305a7b2537511f1",
    "relative cleanup directory",
  );
  const relativeDirectoryObjectAttributes =
    "System::Call '*(&l4, p ${ParentSystemRegister}, p r7, i ${FYAGENT_OBJ_CASE_INSENSITIVE}|${FYAGENT_OBJ_DONT_REPARSE}, p 0, p 0, &l.R3) p .r4'";
  const relativeDirectoryNtOpen =
    "System::Call 'ntdll::NtCreateFile(*p .r8, i ${FYAGENT_DELETE}|${FYAGENT_FILE_READ_ATTRIBUTES}, p r4, p r0, p 0, i 0, i ${FYAGENT_FILE_SHARE_READ}, i ${FYAGENT_FILE_OPEN}, i ${FYAGENT_FILE_DIRECTORY_FILE}|${FYAGENT_FILE_FLAG_OPEN_REPARSE_POINT}, p 0, i 0) i .r2'";
  for (const required of [
    'StrLen $R2 "${RelativeName}"',
    "IntOp $R2 $R2 * 2",
    "IntOp $R5 $R2 + 2",
    "System::Call '*(&w${NSIS_MAX_STRLEN} \"${RelativeName}\") p .r6'",
    "System::Call '*(&i2 R2, &i2 R5, p r6, &l.R3) p .r7'",
    relativeDirectoryObjectAttributes,
    "${If} $R7 != ${ParentHandle}",
    "System::Call '*(p 0, p 0, &l.R3) p .r0'",
    relativeDirectoryNtOpen,
    "GetFileInformationByHandle(p r8, p r6)",
    "FYAGENT_FILE_ATTRIBUTE_DIRECTORY",
    "FYAGENT_FILE_ATTRIBUTE_REPARSE_POINT",
    "StrCpy ${OutputHandle} $8",
    "StrCpy ${ValidFlag} 1",
    "CloseHandle(p r8)",
  ]) {
    contract(
      relativeDirectoryOpen.includes(required),
      `relative cleanup directory open is missing ${required}`,
    );
  }
  contract(
    (relativeDirectoryOpen.match(/ntdll::NtCreateFile/gu) ?? []).length === 1 &&
      (relativeDirectoryOpen.match(/CloseHandle\(p r8\)/gu) ?? []).length === 1,
    "relative cleanup directory must issue one native open and close every rejected handle",
  );
  assertOrdered(
    relativeDirectoryOpen,
    [
      'StrLen $R2 "${RelativeName}"',
      "System::Call '*(&i2 R2, &i2 R5, p r6, &l.R3) p .r7'",
      relativeDirectoryObjectAttributes,
      "IntOp $R4 $4 + ${FYAGENT_OBJECT_ATTRIBUTES_ROOT_DIRECTORY_OFFSET}",
      "${If} $R7 != ${ParentHandle}",
      "System::Call '*(p 0, p 0, &l.R3) p .r0'",
      relativeDirectoryNtOpen,
      "fyagent_${Label}_directory_native_buffers_done:",
      "System::Free $0",
      "System::Free $4",
      "System::Free $7",
      "System::Free $6",
      "${If} $2 <> 0",
      "GetFileInformationByHandle(p r8, p r6)",
      "FYAGENT_FILE_ATTRIBUTE_DIRECTORY",
      "FYAGENT_FILE_ATTRIBUTE_REPARSE_POINT",
      "StrCpy ${OutputHandle} $8",
      "StrCpy ${ValidFlag} 1",
      "Goto fyagent_${Label}_directory_done",
      "fyagent_${Label}_directory_close:",
      "CloseHandle(p r8)",
      "fyagent_${Label}_directory_done:",
    ],
    "parent-handle-relative cleanup directory open",
  );
  contract(
    !/(?:CreateFileW|\$INSTDIR|\$COMMONPROGRAMDATA|(?:^|\s)(?:Delete|DeleteFileW|RemoveDirectoryW|RMDir|MoveFile\w*|Quit|Abort|SetErrorLevel|Return)(?:\s|$))/imu.test(
      relativeDirectoryOpen,
    ),
    "relative cleanup directory open must never reparse a full path or use a path mutation or early exit",
  );

  const relativeLeafMatch = executableSource.match(
    /!macro FyAgentDeleteRegularFileRelativeToHandle ParentSystemRegister ParentHandle LeafName Label([\s\S]*?)!macroend/u,
  );
  contract(
    relativeLeafMatch,
    "missing parent-handle-relative cleanup leaf macro",
  );
  const relativeLeafDeletion = relativeLeafMatch[1];
  assertNormalizedNsisDigest(
    relativeLeafDeletion,
    "704e1a4cda50877f06b8601a67907627a8fe10cadc9c52647a59ba17ab9a91cb",
    "relative cleanup leaf",
  );
  const relativeLeafObjectAttributes =
    "System::Call '*(&l4, p ${ParentSystemRegister}, p r7, i ${FYAGENT_OBJ_CASE_INSENSITIVE}|${FYAGENT_OBJ_DONT_REPARSE}, p 0, p 0, &l.R3) p .r4'";
  const relativeLeafNtOpen =
    "System::Call 'ntdll::NtCreateFile(*p .r8, i ${FYAGENT_DELETE}|${FYAGENT_FILE_READ_ATTRIBUTES}, p r4, p r0, p 0, i 0, i ${FYAGENT_FILE_SHARE_READ}, i ${FYAGENT_FILE_OPEN}, i ${FYAGENT_FILE_NON_DIRECTORY_FILE}|${FYAGENT_FILE_FLAG_OPEN_REPARSE_POINT}, p 0, i 0) i .r2'";
  const sameHandleDisposition =
    "System::Call 'kernel32::SetFileInformationByHandle(p r8, i ${FYAGENT_FILE_DISPOSITION_INFO_CLASS}, p r6, i ${FYAGENT_FILE_DISPOSITION_INFO_SIZE}) i .r7'";
  for (const required of [
    'StrLen $R2 "${LeafName}"',
    "IntOp $R2 $R2 * 2",
    "IntOp $R5 $R2 + 2",
    "System::Call '*(&w${NSIS_MAX_STRLEN} \"${LeafName}\") p .r6'",
    "System::Call '*(&i2 R2, &i2 R5, p r6, &l.R3) p .r7'",
    relativeLeafObjectAttributes,
    "${If} $R7 != ${ParentHandle}",
    relativeLeafNtOpen,
    "GetFileInformationByHandle(p r8, p r6)",
    "FYAGENT_FILE_ATTRIBUTE_DIRECTORY",
    "FYAGENT_FILE_ATTRIBUTE_REPARSE_POINT",
    sameHandleDisposition,
    "CloseHandle(p r8)",
  ]) {
    contract(
      relativeLeafDeletion.includes(required),
      `relative cleanup leaf deletion is missing ${required}`,
    );
  }
  contract(
    (relativeLeafDeletion.match(/ntdll::NtCreateFile/gu) ?? []).length === 1 &&
      (relativeLeafDeletion.match(/SetFileInformationByHandle/gu) ?? [])
        .length === 1 &&
      (relativeLeafDeletion.match(/CloseHandle\(p r8\)/gu) ?? []).length === 1,
    "relative cleanup leaf must open, mark, and close exactly one leaf handle",
  );
  assertOrdered(
    relativeLeafDeletion,
    [
      'StrLen $R2 "${LeafName}"',
      "System::Call '*(&i2 R2, &i2 R5, p r6, &l.R3) p .r7'",
      relativeLeafObjectAttributes,
      "${If} $R7 != ${ParentHandle}",
      relativeLeafNtOpen,
      "fyagent_${Label}_leaf_native_buffers_done:",
      "System::Free $0",
      "System::Free $4",
      "System::Free $7",
      "System::Free $6",
      "${If} $2 <> 0",
      "GetFileInformationByHandle(p r8, p r6)",
      "FYAGENT_FILE_ATTRIBUTE_DIRECTORY",
      "FYAGENT_FILE_ATTRIBUTE_REPARSE_POINT",
      sameHandleDisposition,
      "fyagent_${Label}_leaf_close:",
      "CloseHandle(p r8)",
      "fyagent_${Label}_leaf_done:",
    ],
    "parent-handle-relative cleanup leaf deletion",
  );
  contract(
    !/(?:CreateFileW|\$INSTDIR|\$COMMONPROGRAMDATA|(?:^|\s)(?:Delete|DeleteFileW|RemoveDirectoryW|RMDir|MoveFile\w*|Quit|Abort|SetErrorLevel|Return)(?:\s|$))/imu.test(
      relativeLeafDeletion,
    ),
    "relative cleanup leaf deletion must never reparse a full path or use a path mutation or early exit",
  );

  const directoryDispositionMatch = executableSource.match(
    /!macro FyAgentMarkEmptyDirectoryForDeletion HandleSystemRegister Label([\s\S]*?)!macroend/u,
  );
  contract(
    directoryDispositionMatch,
    "missing same-handle empty-directory disposition macro",
  );
  const directoryDisposition = directoryDispositionMatch[1];
  assertNormalizedNsisDigest(
    directoryDisposition,
    "7d8106019a83900191e96c1bae74f840f7f88a35aa34ba342c9096dcc628ea42",
    "same-handle empty-directory disposition",
  );
  const sameDirectoryHandleDisposition =
    "System::Call 'kernel32::SetFileInformationByHandle(p ${HandleSystemRegister}, i ${FYAGENT_FILE_DISPOSITION_INFO_CLASS}, p r6, i ${FYAGENT_FILE_DISPOSITION_INFO_SIZE}) i .r7'";
  assertOrdered(
    directoryDisposition,
    [
      "GetFileInformationByHandle(p ${HandleSystemRegister}, p r6)",
      "FYAGENT_FILE_ATTRIBUTE_DIRECTORY",
      "FYAGENT_FILE_ATTRIBUTE_REPARSE_POINT",
      "System::Call '*$6(&i1 1)'",
      sameDirectoryHandleDisposition,
      "System::Free $6",
      "fyagent_${Label}_directory_disposition_done:",
    ],
    "same-handle empty-directory disposition",
  );
  contract(
    (directoryDisposition.match(/SetFileInformationByHandle/gu) ?? [])
      .length === 1 &&
      !/(?:CreateFileW|NtCreateFile|CloseHandle|\$INSTDIR|\$COMMONPROGRAMDATA|(?:^|\s)(?:Delete|DeleteFileW|RemoveDirectoryW|RMDir|MoveFile\w*|Quit|Abort|SetErrorLevel|Return)(?:\s|$))/imu.test(
        directoryDisposition,
      ),
    "empty-directory cleanup must use only the caller-owned validated handle and leave closure to the caller",
  );

  const legacyNameMatch = executableSource.match(
    /!macro FyAgentValidateLegacyRuntimeName Value Label ValidFlag([\s\S]*?)!macroend/u,
  );
  contract(legacyNameMatch, "missing strict legacy runtime filename validator");
  contract(
    JSON.stringify(nonEmptyTrimmedLines(legacyNameMatch[1])) ===
      JSON.stringify([
        "StrCpy ${ValidFlag} 0",
        'StrLen $R3 "${Value}"',
        "${If} $R3 < 14",
        "Goto fyagent_${Label}_legacy_name_done",
        "${EndIf}",
        'StrCpy $R4 "${Value}" 9',
        'StrCmp $R4 "business-" 0 fyagent_${Label}_legacy_name_done',
        'StrCpy $R4 "${Value}" 5 -5',
        'StrCmp $R4 ".lock" fyagent_${Label}_legacy_name_valid',
        "${If} $R3 < 15",
        "Goto fyagent_${Label}_legacy_name_done",
        "${EndIf}",
        'StrCpy $R4 "${Value}" 6 -6',
        'StrCmp $R4 ".state" 0 fyagent_${Label}_legacy_name_done',
        "fyagent_${Label}_legacy_name_valid:",
        "StrCpy ${ValidFlag} 1",
        "fyagent_${Label}_legacy_name_done:",
      ]),
    "legacy runtime filename validation must admit only complete lowercase business-*.state and business-*.lock direct-child names",
  );

  const cleanupMatch = executableSource.match(
    /!macro FyAgentCleanupLegacyMachineRuntime Label([\s\S]*?)!macroend/u,
  );
  contract(cleanupMatch, "missing bounded legacy runtime cleanup macro");
  const cleanup = cleanupMatch[1];
  const cleanupLines = nonEmptyTrimmedLines(cleanup);
  const expectedCleanupLines = [
    "ClearErrors",
    '!insertmacro FyAgentOpenCleanupAnchorDirectory "$COMMONPROGRAMDATA\\FyAgent" ${Label}_parent $5 $9',
    "${If} $9 <> 1",
    "Goto fyagent_${Label}_done",
    "${EndIf}",
    '!insertmacro FyAgentOpenDirectoryRelativeToHandle r5 $5 "runtime" ${Label}_runtime $3 $2',
    "${If} $2 <> 1",
    "Goto fyagent_${Label}_close_parent",
    "${EndIf}",
    "ClearErrors",
    'FindFirst $R0 $R1 "$COMMONPROGRAMDATA\\FyAgent\\runtime\\*"',
    "IfErrors fyagent_${Label}_close_runtime",
    "fyagent_${Label}_legacy_entry:",
    'StrCmp $R1 "." fyagent_${Label}_legacy_next',
    'StrCmp $R1 ".." fyagent_${Label}_legacy_next',
    '!insertmacro FyAgentValidateLegacyRuntimeName "$R1" ${Label}_legacy_entry $R5',
    "${If} $R5 == 1",
    '!insertmacro FyAgentDeleteRegularFileRelativeToHandle r3 $3 "$R1" ${Label}_legacy_file',
    "${EndIf}",
    "fyagent_${Label}_legacy_next:",
    "ClearErrors",
    "FindNext $R0 $R1",
    "IfErrors fyagent_${Label}_legacy_close_find",
    "Goto fyagent_${Label}_legacy_entry",
    "fyagent_${Label}_legacy_close_find:",
    "FindClose $R0",
    "fyagent_${Label}_close_runtime:",
    "!insertmacro FyAgentMarkEmptyDirectoryForDeletion r3 ${Label}_legacy_runtime",
    "System::Call 'kernel32::CloseHandle(p r3) i .r4'",
    "fyagent_${Label}_close_parent:",
    "!insertmacro FyAgentMarkEmptyDirectoryForDeletion r5 ${Label}_legacy_parent",
    "System::Call 'kernel32::CloseHandle(p r5) i .r4'",
    "fyagent_${Label}_done:",
    "ClearErrors",
  ];
  contract(
    JSON.stringify(cleanupLines) === JSON.stringify(expectedCleanupLines),
    "legacy cleanup must retain its exact handle-relative branch, label, fallthrough, closure, and final ClearErrors control flow",
  );
  const legacyPathLines = nonEmptyTrimmedLines(executableSource).filter(
    (line) => line.includes("$COMMONPROGRAMDATA\\FyAgent"),
  );
  contract(
    JSON.stringify(legacyPathLines) ===
      JSON.stringify([
        '!insertmacro FyAgentOpenCleanupAnchorDirectory "$COMMONPROGRAMDATA\\FyAgent" ${Label}_parent $5 $9',
        'FindFirst $R0 $R1 "$COMMONPROGRAMDATA\\FyAgent\\runtime\\*"',
      ]),
    "legacy ProgramData full paths may only anchor the parent and enumerate candidate direct-child names",
  );
  contract(
    (cleanup.match(/\*/gu) ?? []).length === 1 &&
      (cleanup.match(/^\s*FindFirst\b/gmu) ?? []).length === 1 &&
      (cleanup.match(/^\s*FindNext\b/gmu) ?? []).length === 1 &&
      (cleanup.match(/^\s*FindClose\b/gmu) ?? []).length === 1,
    "legacy cleanup must enumerate the fixed runtime directory exactly once",
  );
  contract(
    !/(?:Quit|Abort|SetErrorLevel|Return|CreateDirectoryW|ConvertStringSecurityDescriptor|GetSecurityInfo|SetSecurityInfo|SetKernelObjectSecurity|SetNamedSecurityInfo|icacls|CreateFileW|NtCreateFile|RMDir|^\s*Delete\b|DeleteFileW|RemoveDirectoryW|MoveFile\w*)/imu.test(
      cleanup,
    ),
    "legacy cleanup must remain handle-relative, best-effort, non-provisioning, non-recursive, and free of path mutation or early exits",
  );
  contract(
    !/(?:FyAgentProvisionMachineRuntime|FyAgentMachineRuntimeBootstrap|FYAGENT_RUNTIME_ROOT_SDDL|FyAgentRuntimeProvision|FyAgentCreateTrustedRuntimeDirectory|FyAgentMarkRuntimeDirectoryForDeletion)/u.test(
      executableSource,
    ),
    "retired machine-runtime provisioning contract remains executable",
  );

  const install = stripNsisComments(
    namedBlock(blocks, "section", "Install").body,
  );
  assertOrdered(
    install,
    [
      '!insertmacro FyAgentRequireProcessStopped "${MAINBINARYNAME}.exe" "${PRODUCTNAME}" install_main',
      '!insertmacro FyAgentRequireProcessStopped "fyagent-user-helper.exe" "${PRODUCTNAME} user helper" install_helper',
      "!insertmacro FyAgentCleanupLegacyMachineRuntime install_legacy_runtime",
      'File "${MAINBINARYSRCPATH}"',
    ],
    "best-effort install-time legacy cleanup",
  );
  contract(
    (
      executableSource.match(
        /^\s*!insertmacro\s+FyAgentCleanupLegacyMachineRuntime\s+/gimu,
      ) ?? []
    ).length === 2,
    "legacy runtime cleanup must run exactly once for install and uninstall",
  );
}

function assertKnownStagingCleanupContract(source, blocks) {
  const executableSource = stripNsisComments(source);
  const executableLines = nonEmptyTrimmedLines(executableSource);

  for (const definition of [
    "!define FYAGENT_DELETE 0x00010000",
    "!define FYAGENT_FILE_READ_ATTRIBUTES 0x80",
    "!define FYAGENT_FILE_SHARE_READ 0x1",
    "!define FYAGENT_FILE_FLAG_OPEN_REPARSE_POINT 0x00200000",
    "!define FYAGENT_FILE_DISPOSITION_INFO_CLASS 4",
    "!define FYAGENT_FILE_DISPOSITION_INFO_SIZE 1",
    "!define FYAGENT_OBJ_CASE_INSENSITIVE 0x40",
    "!define FYAGENT_OBJ_DONT_REPARSE 0x1000",
    "!define FYAGENT_FILE_OPEN 0x1",
    "!define FYAGENT_FILE_DIRECTORY_FILE 0x1",
    "!define FYAGENT_FILE_NON_DIRECTORY_FILE 0x40",
    "!define FYAGENT_NSIS_SYSTEM_POINTER_SIZE 4",
    "!define FYAGENT_UNICODE_STRING_SIZE 8",
    "!define FYAGENT_UNICODE_STRING_BUFFER_OFFSET 4",
    "!define FYAGENT_OBJECT_ATTRIBUTES_SIZE 24",
    "!define FYAGENT_OBJECT_ATTRIBUTES_ROOT_DIRECTORY_OFFSET 4",
    "!define FYAGENT_IO_STATUS_BLOCK_SIZE 8",
  ]) {
    contract(
      executableLines.filter((line) => line === definition).length === 1,
      `staging leaf deletion must pin ${definition}`,
    );
  }
  const leafMatch = executableSource.match(
    /!macro FyAgentDeleteRegularFileRelativeToHandle ParentSystemRegister ParentHandle LeafName Label([\s\S]*?)!macroend/u,
  );
  contract(leafMatch, "missing parent-handle-relative cleanup leaf macro");
  const leafDeletion = leafMatch[1];
  const leafDeletionLines = nonEmptyTrimmedLines(leafDeletion);
  const expectedLeafDeletionLines = [
    "StrCpy $8 0",
    "StrCpy $6 0",
    "StrCpy $7 0",
    "StrCpy $4 0",
    "StrCpy $0 0",
    "StrCpy $2 -1",
    'StrLen $R2 "${LeafName}"',
    "${If} $R2 == 0",
    "Goto fyagent_${Label}_leaf_done",
    "${EndIf}",
    "IntOp $R2 $R2 * 2",
    "IntOp $R5 $R2 + 2",
    "System::Call '*(&w${NSIS_MAX_STRLEN} \"${LeafName}\") p .r6'",
    "${If} $6 == 0",
    "Goto fyagent_${Label}_leaf_native_buffers_done",
    "${EndIf}",
    "System::Call '*(&i2 R2, &i2 R5, p r6, &l.R3) p .r7'",
    "${If} $7 == 0",
    "${OrIf} $R3 <> ${FYAGENT_UNICODE_STRING_SIZE}",
    "Goto fyagent_${Label}_leaf_native_buffers_done",
    "${EndIf}",
    "IntOp $R4 $7 + ${FYAGENT_UNICODE_STRING_BUFFER_OFFSET}",
    "System::Call '*$R4(p .R7)'",
    "${If} $R7 != $6",
    "Goto fyagent_${Label}_leaf_native_buffers_done",
    "${EndIf}",
    "System::Call '*(&l4, p ${ParentSystemRegister}, p r7, i ${FYAGENT_OBJ_CASE_INSENSITIVE}|${FYAGENT_OBJ_DONT_REPARSE}, p 0, p 0, &l.R3) p .r4'",
    "${If} $4 == 0",
    "${OrIf} $R3 <> ${FYAGENT_OBJECT_ATTRIBUTES_SIZE}",
    "Goto fyagent_${Label}_leaf_native_buffers_done",
    "${EndIf}",
    "IntOp $R4 $4 + ${FYAGENT_OBJECT_ATTRIBUTES_ROOT_DIRECTORY_OFFSET}",
    "System::Call '*$R4(p .R7)'",
    "${If} $R7 != ${ParentHandle}",
    "Goto fyagent_${Label}_leaf_native_buffers_done",
    "${EndIf}",
    "System::Call '*(p 0, p 0, &l.R3) p .r0'",
    "${If} $0 == 0",
    "${OrIf} $R3 <> ${FYAGENT_IO_STATUS_BLOCK_SIZE}",
    "Goto fyagent_${Label}_leaf_native_buffers_done",
    "${EndIf}",
    "System::Call 'ntdll::NtCreateFile(*p .r8, i ${FYAGENT_DELETE}|${FYAGENT_FILE_READ_ATTRIBUTES}, p r4, p r0, p 0, i 0, i ${FYAGENT_FILE_SHARE_READ}, i ${FYAGENT_FILE_OPEN}, i ${FYAGENT_FILE_NON_DIRECTORY_FILE}|${FYAGENT_FILE_FLAG_OPEN_REPARSE_POINT}, p 0, i 0) i .r2'",
    "fyagent_${Label}_leaf_native_buffers_done:",
    "${If} $0 <> 0",
    "System::Free $0",
    "${EndIf}",
    "${If} $4 <> 0",
    "System::Free $4",
    "${EndIf}",
    "${If} $7 <> 0",
    "System::Free $7",
    "${EndIf}",
    "${If} $6 <> 0",
    "System::Free $6",
    "${EndIf}",
    "${If} $2 <> 0",
    "Goto fyagent_${Label}_leaf_done",
    "${EndIf}",
    "${If} $8 == ${FYAGENT_INVALID_HANDLE_VALUE}",
    "${OrIf} $8 == 0",
    "Goto fyagent_${Label}_leaf_done",
    "${EndIf}",
    "System::Alloc ${FYAGENT_BY_HANDLE_FILE_INFORMATION_SIZE}",
    "Pop $6",
    "${If} $6 == 0",
    "Goto fyagent_${Label}_leaf_close",
    "${EndIf}",
    "System::Call 'kernel32::GetFileInformationByHandle(p r8, p r6) i .r7'",
    "${If} $7 == 0",
    "System::Free $6",
    "Goto fyagent_${Label}_leaf_close",
    "${EndIf}",
    "System::Call '*$6(i .r0)'",
    "System::Free $6",
    "IntOp $4 $0 & ${FYAGENT_FILE_ATTRIBUTE_DIRECTORY}",
    "${If} $4 <> 0",
    "Goto fyagent_${Label}_leaf_close",
    "${EndIf}",
    "IntOp $4 $0 & ${FYAGENT_FILE_ATTRIBUTE_REPARSE_POINT}",
    "${If} $4 <> 0",
    "Goto fyagent_${Label}_leaf_close",
    "${EndIf}",
    "System::Alloc ${FYAGENT_FILE_DISPOSITION_INFO_SIZE}",
    "Pop $6",
    "${If} $6 == 0",
    "Goto fyagent_${Label}_leaf_close",
    "${EndIf}",
    "System::Call '*$6(&i1 1)'",
    "System::Call 'kernel32::SetFileInformationByHandle(p r8, i ${FYAGENT_FILE_DISPOSITION_INFO_CLASS}, p r6, i ${FYAGENT_FILE_DISPOSITION_INFO_SIZE}) i .r7'",
    "System::Free $6",
    "fyagent_${Label}_leaf_close:",
    "System::Call 'kernel32::CloseHandle(p r8) i .r4'",
    "fyagent_${Label}_leaf_done:",
  ];
  contract(
    JSON.stringify(leafDeletionLines) ===
      JSON.stringify(expectedLeafDeletionLines),
    "cleanup leaf deletion must keep exact parent-handle-relative open, no-follow validation, resource cleanup, and same-handle disposition control flow",
  );
  contract(
    !/(?:CreateFileW|\$INSTDIR|\$COMMONPROGRAMDATA|(?:^|\s)(?:Delete|DeleteFileW|RemoveDirectoryW|MoveFile\w*|Quit|Abort|SetErrorLevel|Return)(?:\s|$))/imu.test(
      leafDeletion,
    ),
    "cleanup leaf deletion must not reparse a full path or use a path delete, rename, or early exit",
  );

  const uuidMatch = executableSource.match(
    /!macro FyAgentValidateCanonicalUuid Value Label ValidFlag([\s\S]*?)!macroend/u,
  );
  contract(uuidMatch, "missing canonical staging UUID validator");
  const uuidValidator = uuidMatch[1];
  const hyphenOffsets = [
    ...uuidValidator.matchAll(/^\s*StrCpy \$R4 "\$\{Value\}" 1 (\d+)\s*$/gmu),
  ].map((match) => match[1]);
  contract(
    JSON.stringify(hyphenOffsets) === JSON.stringify(["8", "13", "18", "23"]),
    "staging UUID validation must pin the four canonical hyphen offsets",
  );
  for (const offset of hyphenOffsets) {
    assertOrdered(
      uuidValidator,
      [
        `StrCpy $R4 "\${Value}" 1 ${offset}`,
        'StrCmp $R4 "-" 0 fyagent_${Label}_uuid_done',
      ],
      `staging UUID hyphen ${offset}`,
    );
  }
  const admittedCharacters = [
    ...uuidValidator.matchAll(
      /^\s*StrCmp \$R4 "([0-9A-Za-z])" fyagent_\$\{Label\}_uuid_next\s*$/gmu,
    ),
  ].map((match) => match[1]);
  contract(
    JSON.stringify(admittedCharacters) ===
      JSON.stringify([..."0123456789abcdef"]),
    "staging UUID validation must admit lowercase hexadecimal characters only",
  );
  assertOrdered(
    uuidValidator,
    [
      "StrCpy ${ValidFlag} 0",
      'StrLen $R3 "${Value}"',
      "StrCmp $R3 36 0 fyagent_${Label}_uuid_done",
      "StrCpy $R2 0",
      "fyagent_${Label}_uuid_loop:",
      'StrCpy $R4 "${Value}" 1 $R2',
      'StrCmp $R4 "0" fyagent_${Label}_uuid_next',
      'StrCmp $R4 "f" fyagent_${Label}_uuid_next',
      "Goto fyagent_${Label}_uuid_done",
      "fyagent_${Label}_uuid_next:",
      "IntOp $R2 $R2 + 1",
      "Goto fyagent_${Label}_uuid_loop",
      "fyagent_${Label}_uuid_done:",
    ],
    "canonical staging UUID validation",
  );

  const cleanupMatch = executableSource.match(
    /!macro FyAgentCleanupKnownCodexInstallerStaging Label([\s\S]*?)!macroend/u,
  );
  contract(cleanupMatch, "missing known-only Codex installer staging cleanup");
  const cleanup = cleanupMatch[1];
  const cleanupLines = nonEmptyTrimmedLines(cleanup);
  const expectedCleanupLines = [
    "ClearErrors",
    '!insertmacro FyAgentOpenCleanupAnchorDirectory "$INSTDIR\\cache" ${Label}_cache $5 $9',
    "${If} $9 <> 1",
    "Goto fyagent_${Label}_staging_done",
    "${EndIf}",
    '!insertmacro FyAgentOpenDirectoryRelativeToHandle r5 $5 "codex-installer" ${Label}_staging $3 $2',
    "${If} $2 <> 1",
    "Goto fyagent_${Label}_staging_close_cache",
    "${EndIf}",
    "ClearErrors",
    'FindFirst $R0 $R1 "$INSTDIR\\cache\\codex-installer\\*"',
    "IfErrors fyagent_${Label}_staging_close_root",
    "fyagent_${Label}_staging_entry:",
    'StrCmp $R1 "." fyagent_${Label}_staging_next',
    'StrCmp $R1 ".." fyagent_${Label}_staging_next',
    '!insertmacro FyAgentValidateCanonicalUuid "$R1" ${Label}_staging_entry $R5',
    "${If} $R5 == 1",
    '!insertmacro FyAgentOpenDirectoryRelativeToHandle r3 $3 "$R1" ${Label}_staging_child $1 $R6',
    "${If} $R6 == 1",
    '!insertmacro FyAgentDeleteRegularFileRelativeToHandle r1 $1 "installer.msix" ${Label}_staging_msix',
    '!insertmacro FyAgentDeleteRegularFileRelativeToHandle r1 $1 "installer.msix.part" ${Label}_staging_part',
    "!insertmacro FyAgentMarkEmptyDirectoryForDeletion r1 ${Label}_staging_child",
    "System::Call 'kernel32::CloseHandle(p r1) i .r4'",
    "${EndIf}",
    "${EndIf}",
    "fyagent_${Label}_staging_next:",
    "ClearErrors",
    "FindNext $R0 $R1",
    "IfErrors fyagent_${Label}_staging_close_find",
    "Goto fyagent_${Label}_staging_entry",
    "fyagent_${Label}_staging_close_find:",
    "FindClose $R0",
    "fyagent_${Label}_staging_close_root:",
    "!insertmacro FyAgentMarkEmptyDirectoryForDeletion r3 ${Label}_staging_root",
    "System::Call 'kernel32::CloseHandle(p r3) i .r4'",
    "fyagent_${Label}_staging_close_cache:",
    "!insertmacro FyAgentMarkEmptyDirectoryForDeletion r5 ${Label}_staging_cache",
    "System::Call 'kernel32::CloseHandle(p r5) i .r4'",
    "fyagent_${Label}_staging_done:",
    "ClearErrors",
  ];
  contract(
    JSON.stringify(cleanupLines) === JSON.stringify(expectedCleanupLines),
    "known-only staging cleanup must retain its exact branch, label, fallthrough, and final ClearErrors control flow",
  );
  contract(
    !/^(?:Quit|Abort|SetErrorLevel|Return)\b/imu.test(cleanup),
    "known-only staging cleanup must not contain an early exit",
  );

  const stagingPathLines = executableLines.filter((line) =>
    line.includes("$INSTDIR\\cache"),
  );
  contract(
    JSON.stringify(stagingPathLines) ===
      JSON.stringify([
        '!insertmacro FyAgentOpenCleanupAnchorDirectory "$INSTDIR\\cache" ${Label}_cache $5 $9',
        'FindFirst $R0 $R1 "$INSTDIR\\cache\\codex-installer\\*"',
      ]),
    "staging full paths may only anchor cache and enumerate candidate direct-child names",
  );
  assertOrdered(
    cleanup,
    [
      "ClearErrors",
      '!insertmacro FyAgentOpenCleanupAnchorDirectory "$INSTDIR\\cache" ${Label}_cache $5 $9',
      '!insertmacro FyAgentOpenDirectoryRelativeToHandle r5 $5 "codex-installer" ${Label}_staging $3 $2',
      'FindFirst $R0 $R1 "$INSTDIR\\cache\\codex-installer\\*"',
      'StrCmp $R1 "." fyagent_${Label}_staging_next',
      'StrCmp $R1 ".." fyagent_${Label}_staging_next',
      '!insertmacro FyAgentValidateCanonicalUuid "$R1" ${Label}_staging_entry $R5',
      '!insertmacro FyAgentOpenDirectoryRelativeToHandle r3 $3 "$R1" ${Label}_staging_child $1 $R6',
      '!insertmacro FyAgentDeleteRegularFileRelativeToHandle r1 $1 "installer.msix" ${Label}_staging_msix',
      '!insertmacro FyAgentDeleteRegularFileRelativeToHandle r1 $1 "installer.msix.part" ${Label}_staging_part',
      "!insertmacro FyAgentMarkEmptyDirectoryForDeletion r1 ${Label}_staging_child",
      "CloseHandle(p r1)",
      "FindNext $R0 $R1",
      "FindClose $R0",
      "!insertmacro FyAgentMarkEmptyDirectoryForDeletion r3 ${Label}_staging_root",
      "CloseHandle(p r3)",
      "!insertmacro FyAgentMarkEmptyDirectoryForDeletion r5 ${Label}_staging_cache",
      "CloseHandle(p r5)",
      "ClearErrors",
    ],
    "known-only Codex installer staging cleanup",
  );
  const deleteLines = cleanup
    .split("\n")
    .map((line) => line.trim())
    .filter((line) => line.startsWith("Delete "));
  contract(
    deleteLines.length === 0,
    "staging cleanup must never delete a leaf by path after validation",
  );
  contract(
    !/^\s*RMDir\b/imu.test(cleanup),
    "staging cleanup must retire empty directories only through their already-held handles",
  );
  contract(
    (cleanup.match(/\*/gu) ?? []).length === 1 &&
      (cleanup.match(/^\s*FindFirst\b/gmu) ?? []).length === 1 &&
      (cleanup.match(/^\s*FindNext\b/gmu) ?? []).length === 1 &&
      (cleanup.match(/^\s*FindClose\b/gmu) ?? []).length === 1,
    "staging cleanup must enumerate only the fixed root's direct children",
  );
  contract(
    !/(?:Quit|Abort|SetErrorLevel|Return|CreateDirectory|CreateFileW|NtCreateFile|RMDir|^\s*Delete\b|DeleteFileW|RemoveDirectoryW|MoveFile\w*)/imu.test(
      cleanup,
    ),
    "staging cleanup must remain handle-bound, best-effort, non-recursive, and free of early exits",
  );

  const install = stripNsisComments(
    namedBlock(blocks, "section", "Install").body,
  );
  const uninstall = stripNsisComments(
    namedBlock(blocks, "section", "Uninstall").body,
  );
  assertOrdered(
    install,
    [
      "!insertmacro FyAgentCleanupLegacyMachineRuntime install_legacy_runtime",
      "!insertmacro FyAgentCleanupKnownCodexInstallerStaging install_codex_staging",
      'File "${MAINBINARYSRCPATH}"',
    ],
    "best-effort install-time staging cleanup",
  );
  assertOrdered(
    uninstall,
    [
      "!insertmacro FyAgentCleanupLegacyMachineRuntime uninstall_legacy_runtime",
      "!insertmacro FyAgentCleanupKnownCodexInstallerStaging uninstall_codex_staging",
      'Delete "$INSTDIR\\${MAINBINARYNAME}.exe"',
    ],
    "best-effort uninstall-time staging cleanup",
  );
  contract(
    (
      executableSource.match(
        /^\s*!insertmacro\s+FyAgentCleanupKnownCodexInstallerStaging\s+/gimu,
      ) ?? []
    ).length === 2,
    "known-only staging cleanup must run exactly once for install and uninstall",
  );
}

function assertUninstallOwnershipContract(source, blocks) {
  const executableSource = stripNsisComments(source);
  const install = stripNsisComments(
    namedBlock(blocks, "section", "Install").body,
  );
  const uninstall = stripNsisComments(
    namedBlock(blocks, "section", "Uninstall").body,
  );
  assertOrdered(
    uninstall,
    [
      '!insertmacro FyAgentRequireProcessStopped "${MAINBINARYNAME}.exe" "${PRODUCTNAME}" uninstall_main',
      '!insertmacro FyAgentRequireProcessStopped "fyagent-user-helper.exe" "${PRODUCTNAME} user helper" uninstall_helper',
      "!insertmacro FyAgentCleanupLegacyMachineRuntime uninstall_legacy_runtime",
      'Delete "$INSTDIR\\${MAINBINARYNAME}.exe"',
    ],
    "best-effort uninstall-time legacy cleanup",
  );
  contract(
    !/RMDir\s+\/r(?:\s+\/REBOOTOK)?\s+"?\$INSTDIR/iu.test(executableSource),
    "uninstaller must never recursively delete a caller-selected $INSTDIR",
  );
  contract(
    !/RMDir\s+\/r\s+"?\$COMMONPROGRAMDATA\\FyAgent/iu.test(executableSource),
    "ProgramData cleanup must delete only known runtime files and empty directories",
  );
  contract(
    !/(?:DeleteAppData|RmDir\s+\/r\s+"?\$(?:APPDATA|LOCALAPPDATA))/iu.test(
      executableSource,
    ),
    "uninstaller must not offer or perform recursive user-data deletion",
  );
  contract(
    uninstall.includes('DeleteRegKey SHCTX "${MANUPRODUCTKEY}"'),
    "uninstaller must remove the installer-owned install-location marker",
  );
  contract(
    (executableSource.match(/^\s*\{\{#each binaries\}\}\s*$/gmu) ?? [])
      .length === 2 &&
      install.includes('File /a "/oname={{this}}" "{{no-escape @key}}"') &&
      uninstall.includes('Delete "$INSTDIR\\\\{{this}}"'),
    "installer and uninstaller must package and remove the configured helper binary",
  );
  for (const required of [
    'Delete "$INSTDIR\\${MAINBINARYNAME}.exe"',
    'Delete "$INSTDIR\\uninstall.exe"',
    'Delete "$SMPROGRAMS\\$AppStartMenuFolder\\${PRODUCTNAME}.lnk"',
    'Delete "$SMPROGRAMS\\${PRODUCTNAME}.lnk"',
    'Delete "$DESKTOP\\${PRODUCTNAME}.lnk"',
    'DeleteRegKey SHCTX "Software\\Classes\\\\{{protocol}}"',
  ]) {
    contract(
      uninstall.includes(required),
      `uninstaller is missing known owned payload/registration cleanup: ${required}`,
    );
  }
}

function readWorkspaceVersion(cargoManifestPath) {
  const source = fs.readFileSync(cargoManifestPath, "utf8");
  const workspacePackage = source.match(
    /^\[workspace\.package\]\s*$([\s\S]*?)(?=^\[|(?![\s\S]))/mu,
  );
  contract(workspacePackage, "Cargo manifest is missing [workspace.package]");
  const version = workspacePackage[1].match(
    /^version\s*=\s*"([^"]+)"\s*(?:#.*)?$/mu,
  );
  contract(
    version,
    "Cargo [workspace.package] is missing its canonical version",
  );
  return version[1];
}

function assertWebView2CommandContract({
  source,
  include,
  loader,
  loaderPath,
  template,
  blocks,
  fakeRootPem,
  fakeLeafPem,
}) {
  const executableTemplate = stripNsisComments(template);
  const chunkCountMatch = include.match(
    /^!define FYAGENT_WEBVIEW2_COMMAND_CHUNK_COUNT (\d+)$/mu,
  );
  contract(chunkCountMatch, "WebView2 include is missing its chunk count");
  const chunkCount = Number.parseInt(chunkCountMatch[1], 10);
  contract(
    chunkCount > 0 && chunkCount < 100,
    "WebView2 chunk count is invalid",
  );

  const chunks = [
    ...include.matchAll(
      /^!define FYAGENT_WEBVIEW2_COMMAND_(\d{2}) "([A-Za-z0-9+/=]+)"$/gmu,
    ),
  ];
  contract(
    chunks.length === chunkCount,
    "WebView2 include chunk count drifted",
  );
  for (let index = 0; index < chunks.length; index += 1) {
    contract(
      chunks[index][1] === String(index).padStart(2, "0"),
      "WebView2 include chunks must be contiguous and ordered",
    );
    contract(
      chunks[index][2].length <= 768,
      "WebView2 include chunk exceeds the reviewed NSIS string bound",
    );
  }
  const encodedPayload = chunks.map((match) => match[2]).join("");
  contract(
    encodedPayload.length <= 8192,
    `WebView2 encoded payload exceeds its environment budget (${encodedPayload.length})`,
  );
  const compressedSource = Buffer.from(encodedPayload, "base64");
  contract(
    compressedSource[GZIP_OS_OFFSET] === GZIP_OS_UNKNOWN,
    "WebView2 payload gzip header must use the canonical unknown OS",
  );
  const expectedCompressedSource = gzipDeterministically(
    Buffer.from(source, "utf16le"),
  );
  contract(
    compressedSource.equals(expectedCompressedSource),
    "WebView2 payload is not the deterministic level-9 gzip of its repo-owned source",
  );
  contract(
    gunzipSync(compressedSource).toString("utf16le") === source,
    "WebView2 compressed command does not decode to its repo-owned source",
  );

  const loaderMatch = include.match(
    /^!define FYAGENT_WEBVIEW2_LOADER_BASE64 "([A-Za-z0-9+/=]+)"$/mu,
  );
  contract(loaderMatch, "WebView2 include is missing its encoded loader");
  const decodedLoader = Buffer.from(loaderMatch[1], "base64").toString(
    "utf16le",
  );
  contract(
    decodedLoader === loader,
    "WebView2 encoded loader does not byte-match its repo-owned source",
  );
  contract(
    loader.includes(`foreach($i in 0..${chunkCount - 1})`) &&
      loader.includes("[IO.Compression.GZipStream]::new") &&
      loader.includes("[ScriptBlock]::Create") &&
      !/(?:\biex\b|\|\s*%)/iu.test(loader),
    "WebView2 loader must read only the fixed chunk set without module commands",
  );
  assertPowerShell51LoaderContract(loader, loaderPath, chunkCount);

  const setNames = [
    ...include.matchAll(
      /SetEnvironmentVariableW\(w "(FY_WV2_\d+)", w "\$\{FYAGENT_WEBVIEW2_COMMAND_\d{2}\}"\)/gu,
    ),
  ].map((match) => match[1]);
  const clearNames = [
    ...include.matchAll(/SetEnvironmentVariableW\(w "(FY_WV2_\d+)", p 0\)/gu),
  ].map((match) => match[1]);
  const expectedNames = Array.from(
    { length: chunkCount },
    (_, index) => `FY_WV2_${index}`,
  );
  contract(
    JSON.stringify(setNames) === JSON.stringify(expectedNames) &&
      JSON.stringify(clearNames) === JSON.stringify(expectedNames),
    "WebView2 chunk environment must be written and cleared exactly once",
  );

  assertOrdered(
    source,
    [
      '$env:PSModulePath = "$PSHOME\\Modules"',
      "$PSModuleAutoLoadingPreference = 'None'",
      "$PSHOME\\Modules\\Microsoft.PowerShell.Management\\Microsoft.PowerShell.Management.psd1",
      "$PSHOME\\Modules\\Microsoft.PowerShell.Security\\Microsoft.PowerShell.Security.psd1",
      "Microsoft.PowerShell.Core\\Set-StrictMode",
    ],
    "trusted PowerShell module initialization",
  );
  contract(
    (source.match(/\$env:/gu) ?? []).length === 1,
    "production WebView2 semantics must not be activated or overridden by environment",
  );
  contract(
    !/^\s*(?:\$[A-Za-z][A-Za-z0-9]*\s*=\s*)?(?:Get-Item|Get-Acl|Get-AuthenticodeSignature|Start-Process|Remove-Item|Import-Module|ForEach-Object|Select-Object|Where-Object)\b/mu.test(
      source,
    ),
    "elevated WebView2 helper contains an unqualified module command",
  );
  for (const required of [
    "Microsoft.PowerShell.Management\\Get-Item",
    "Microsoft.PowerShell.Management\\Get-Acl",
    "Microsoft.PowerShell.Security\\Get-AuthenticodeSignature",
    "O=Microsoft Corporation",
    "1.3.6.1.5.5.7.3.3",
    "CB97E8E85E8E9321FB2646E9574EFD17669B3B0581D24262AC7C8A227433A244",
    "50E824592CAA59C7DB9615D676738C7E4EEE522622440C4C2152D0668D68C6D9",
    "[Security.Cryptography.X509Certificates.X509Chain]::new($true)",
    "[Security.Cryptography.X509Certificates.X509RevocationMode]::Online",
    "[Security.Cryptography.X509Certificates.X509RevocationFlag]::EntireChain",
    "Get-RsaSubjectPublicKeyInfoSha256",
    "[IO.FileShare]::None",
    "[IO.FileShare]::Read",
    "[Net.Http.HttpClientHandler]::new()",
    "$httpHandler.MaxAutomaticRedirections = 5",
    "$httpClient.Timeout = [TimeSpan]::FromMinutes(2)",
    "[Threading.CancellationTokenSource]::new",
    "$cancellation.Token",
    "$responseStream.ReadAsync(",
    "$response.RequestMessage.RequestUri.Scheme",
    "[Uri]::UriSchemeHttps",
    "$maximumBootstrapperBytes = 64MB",
    "[Diagnostics.ProcessStartInfo]::new()",
    "$startInfo.UseShellExecute = $false",
    "$process.WaitForExit()",
    "Microsoft.PowerShell.Management\\Remove-Item",
  ]) {
    contract(
      source.includes(required),
      `WebView2 helper is missing ${required}`,
    );
  }
  assertOrdered(
    source,
    [
      "[Environment+SpecialFolder]::CommonApplicationData",
      "[IO.Path]::IsPathRooted($programDataRoot)",
      "Join-Path $programDataRoot \"FyAgent-WebView2-$([Guid]::NewGuid().ToString('N'))\"",
      "$directorySecurity.SetSecurityDescriptorSddlForm($strictDirectorySddl)",
      "$stage.Create($directorySecurity)",
      "Assert-StrictSecurity -Path $stagePath",
    ],
    "ephemeral WebView2 staging",
  );
  contract(
    !source.includes("$programDataParent") &&
      !/Join-Path\s+\$programDataRoot\s+['"]FyAgent['"]/u.test(source),
    "WebView2 staging must not depend on or recreate the retired ProgramData FyAgent runtime parent",
  );
  contract(
    !/Remove-Item[\s\S]{0,160}-Recurse/iu.test(source),
    "WebView2 cleanup must not recurse",
  );
  contract(
    !/(?:\$TEMP|\$PLUGINSDIR|GetEnvironmentVariable\([^)]*(?:URL|PUBLISH|ARG|MODE))/iu.test(
      source,
    ),
    "WebView2 production policy must not use user-controlled paths or semantic overrides",
  );
  contract(
    /GetAsync\([\s\S]*?\$cancellation\.Token[\s\S]*?ReadAsync\([\s\S]*?\$cancellation\.Token/u.test(
      source,
    ),
    "one hard cancellation token must bound both WebView2 headers and body reads",
  );

  const fakeRoot = new X509Certificate(fakeRootPem);
  const fakeLeaf = new X509Certificate(fakeLeafPem);
  contract(
    fakeRoot.ca,
    "fake CurrentUser root fixture must be a CA certificate",
  );
  contract(
    fakeLeaf.subject.includes("O=Microsoft Corporation") &&
      fakeLeaf.verify(fakeRoot.publicKey) &&
      fakeLeaf.keyUsage?.includes("1.3.6.1.5.5.7.3.3"),
    "fake CurrentUser fixture must be a valid O=Microsoft Corporation leaf chain",
  );
  const fakeRootSpkiSha256 = createHash("sha256")
    .update(fakeRoot.publicKey.export({ type: "spki", format: "der" }))
    .digest("hex")
    .toUpperCase();
  contract(
    !source.includes(fakeRootSpkiSha256),
    "fake CurrentUser root SPKI must not be admitted by the production PCA allow-list",
  );
  assertOrdered(
    source,
    [
      "[IO.FileShare]::Read",
      "Assert-MicrosoftAuthenticode -Path $bootstrapperPath",
      "[Diagnostics.ProcessStartInfo]::new()",
      "$process.WaitForExit()",
      "$reader.Dispose()",
    ],
    "pinned WebView2 signature and execution",
  );

  const webview = stripNsisComments(
    namedBlock(blocks, "section", "WebView2").body,
  );
  assertOrdered(
    webview,
    [
      "FyAgentSetWebView2CommandEnvironment",
      '"$SYSDIR\\WindowsPowerShell\\v1.0\\powershell.exe"',
      "-NoProfile",
      "-NonInteractive",
      "-EncodedCommand ${FYAGENT_WEBVIEW2_LOADER_BASE64}",
      "FyAgentClearWebView2CommandEnvironment",
    ],
    "secure WebView2 invocation",
  );
  contract(
    !/(?:NSISdl::download|\$TEMP|\$PLUGINSDIR)/iu.test(webview),
    "WebView2 section must not stage or execute from a user-writable path",
  );
  const commandLine = webview
    .split("\n")
    .find(
      (line) =>
        line.includes("powershell.exe") && line.includes("-EncodedCommand"),
    );
  contract(commandLine, "WebView2 PowerShell command line is missing");
  const canonicalExpandedCommand = commandLine
    .replace("${FYAGENT_WEBVIEW2_LOADER_BASE64}", loaderMatch[1])
    .replace("$SYSDIR", "C:\\Windows\\System32");
  contract(
    canonicalExpandedCommand.length < 1024 &&
      canonicalExpandedCommand.length < 32767,
    `WebView2 EncodedCommand is too long (${canonicalExpandedCommand.length} UTF-16 code units)`,
  );
  contract(
    executableTemplate.includes(
      '!if "${INSTALLWEBVIEW2MODE}" != "downloadBootstrapper"',
    ),
    "custom template must fail compilation for a non-downloadBootstrapper mode",
  );
}

function powerShellExecutableProjection(source) {
  return normalizedLines(source.replace(/<#[\s\S]*?#>/gu, ""))
    .filter((line) => !line.trimStart().startsWith("#"))
    .join("\n")
    .replace(/`\n\s*/gu, " ");
}

function assertResolvedInstallerLifecycleCalls(source) {
  const expectedCalls = new Map([
    ["default-install", { arguments: "@('/S')", shouldSucceed: "$true" }],
    [
      "preexisting-runtime-extra-ace-negative",
      {
        arguments: "@('/S', \"/D=$customInstallDir\")",
        shouldSucceed: "$false",
      },
    ],
    [
      "preexisting-runtime-unknown-content-negative",
      {
        arguments: "@('/S', \"/D=$customInstallDir\")",
        shouldSucceed: "$false",
      },
    ],
    [
      "preexisting-runtime-no-delete-share-negative",
      {
        arguments: "@('/S', \"/D=$customInstallDir\")",
        shouldSucceed: "$false",
      },
    ],
    [
      "custom-space-unicode-silent-D",
      {
        arguments: "@('/S', \"/D=$customInstallDir\")",
        shouldSucceed: "$true",
      },
    ],
  ]);
  const executable = powerShellExecutableProjection(source);
  const functionDefinitions = [
    ...executable.matchAll(/^function Invoke-NsisProcess \{$/gimu),
  ];
  contract(
    functionDefinitions.length === 1,
    "manual lifecycle must define Invoke-NsisProcess exactly once",
  );
  const uninstallHelperCalls = [
    ...executable.matchAll(
      /\[void\]\(\s*Invoke-NsisProcess\s+-FilePath\s+\$copiedUninstaller\s+-Arguments\s+@\(\s*'\/S'\s*,\s*"_\?=\$InstallDirectory"\s*\)\s+-ShouldSucceed\s+\$true\s+-CaseName\s+\$CaseName\s+-WorkingDirectory\s+\$WorkingDirectory\s+-ArgumentKind\s+Uninstall\s+-TimeoutMilliseconds\s+\$TimeoutMilliseconds\s*\)/giu,
    ),
  ];
  contract(
    uninstallHelperCalls.length === 1,
    "manual lifecycle must retain exactly one approved case-local uninstaller Invoke-NsisProcess call",
  );

  const mainLifecycleStart = executable.indexOf("$cleanupAuthorized = $true");
  const mainLifecycleEnd = executable.indexOf(
    'Write-Host "Windows NSIS native lifecycle verified for $Architecture."',
    mainLifecycleStart,
  );
  contract(
    mainLifecycleStart >= 0 && mainLifecycleEnd > mainLifecycleStart,
    "manual lifecycle setup invocation boundary is missing",
  );
  const mainLifecycle = executable.slice(mainLifecycleStart, mainLifecycleEnd);
  const resolvedInstallerReferences = [
    ...mainLifecycle.matchAll(/-FilePath\s+\$resolvedInstaller\b/giu),
  ];
  const resolvedInstallerCalls = [
    ...mainLifecycle.matchAll(
      /\[void\]\(\s*Invoke-NsisProcess\s+-FilePath\s+\$resolvedInstaller\s+-Arguments\s+(?<arguments>@\([^)]*\))\s+-ShouldSucceed\s+(?<shouldSucceed>\$(?:true|false))\s+-CaseName\s+'(?<caseName>[^']+)'\s+-WorkingDirectory\s+\$testRoot\s*\)/giu,
    ),
  ];
  contract(
    resolvedInstallerReferences.length === expectedCalls.size,
    "manual lifecycle resolved-installer invocation set drifted",
  );
  contract(
    resolvedInstallerCalls.length === resolvedInstallerReferences.length,
    "manual lifecycle resolved-installer invocations must use literal arguments, case name, expected outcome, and the test working directory",
  );

  const observedCases = new Set();
  for (const match of resolvedInstallerCalls) {
    contract(
      match?.groups,
      "manual lifecycle resolved-installer invocations must use literal arguments, case name, expected outcome, and the test working directory",
    );
    const { arguments: argumentsValue, caseName, shouldSucceed } = match.groups;
    const expected = expectedCalls.get(caseName);
    contract(
      expected !== undefined && !observedCases.has(caseName),
      `manual lifecycle contains an unexpected or duplicate resolved-installer case ${caseName}`,
    );
    contract(
      argumentsValue.replace(/\s+/gu, "") ===
        expected.arguments.replace(/\s+/gu, "") &&
        shouldSucceed === expected.shouldSucceed,
      `manual lifecycle resolved-installer case ${caseName} has unexpected arguments or outcome`,
    );
    observedCases.add(caseName);
  }
  contract(
    observedCases.size === expectedCalls.size,
    "manual lifecycle is missing an approved resolved-installer case",
  );

  const allInvokeNsisProcessReferences = [
    ...executable.matchAll(/\bInvoke-NsisProcess\b/giu),
  ];
  contract(
    allInvokeNsisProcessReferences.length ===
      functionDefinitions.length +
        uninstallHelperCalls.length +
        resolvedInstallerCalls.length,
    "manual lifecycle Invoke-NsisProcess invocation set drifted",
  );
}

export function assertLifecycleContract(source) {
  for (const required of [
    "[string]$InstallerPath",
    "[string]$Architecture",
    "[string]$AppVersion",
    "Get-PeMachine",
    "0x8664",
    "0xAA64",
    "DisplayVersion",
    "RegistryView]::Registry64",
    "CASE: default-install",
    "CASE: preexisting-runtime-extra-ace-negative",
    "CASE: preexisting-runtime-unknown-content-negative",
    "CASE: preexisting-runtime-no-delete-share-negative",
    "CASE: trusted-legacy-runtime-rebuild",
    "CASE: custom-space-unicode-silent-D",
    "CASE: webview2-signed-space-unicode-verify",
    "CASE: webview2-current-user-fake-root-negative",
    "https://go.microsoft.com/fwlink/p/?LinkId=2124703",
    "StoreName]::Root",
    "StoreName]::TrustedPublisher",
    "StoreLocation]::CurrentUser",
    "Get-AuthenticodeSignature -LiteralPath $unsignedPe",
    "SignatureStatus]::Valid",
    "$publisherStore.Remove($leafPublic)",
    "$rootStore.Remove($rootPublic)",
    "$cleanupFailures",
    "$cleanupAuthorized = $false",
    "$cleanupAuthorized = $true",
    "$sentinelParentsCreatedByTest",
    "User data sentinel was deleted by uninstall",
  ]) {
    contract(
      source.includes(required),
      `native lifecycle is missing ${required}`,
    );
  }
  for (const retired of [
    "CASE: relative-path-negative",
    "CASE: unc-network-negative",
    "CASE: access-denied-ancestor-negative",
    "CASE: reparse-network-negative",
    "CASE: unsupported-drive-network-negative",
    "CASE: reparse-unsupported-drive-network-negative",
    "FyAgent.NsisLifecycle.NativeNetworkDrive",
    "Invoke-RequiredUnsupportedDriveAcceptance",
    "SmbShare\\New-SmbShare",
    "Assert-RejectedInstallLeftNoMachineWrites",
    "before its final path was admitted",
  ]) {
    contract(
      !source.includes(retired),
      `manual lifecycle must not enforce the retired installation-path restriction ${retired}`,
    );
  }
  assertResolvedInstallerLifecycleCalls(source);
  const mainLifecycleStart = source.indexOf("$cleanupAuthorized = $true");
  contract(
    mainLifecycleStart >= 0,
    "native lifecycle never reaches an authorized clean-run execution",
  );
  const mainLifecycle = source.slice(mainLifecycleStart);

  assertOrdered(
    mainLifecycle,
    [
      "CASE: webview2-signed-space-unicode-verify",
      "Save-OfficialWebView2BootstrapperFixture -DestinationPath $signedCandidate",
      "Invoke-WebView2SignatureVerification",
      "webview2-signed-space-unicode-verify",
      "CASE: webview2-current-user-fake-root-negative",
      "Invoke-FakeCurrentUserRootAttackFixture",
    ],
    "native WebView2 live trust fixtures",
  );

  const fakeRootFixtureStart = source.indexOf(
    "function Invoke-FakeCurrentUserRootAttackFixture",
  );
  const fakeRootFixtureEnd = source.indexOf("\n$runId =", fakeRootFixtureStart);
  contract(
    fakeRootFixtureStart >= 0 && fakeRootFixtureEnd > fakeRootFixtureStart,
    "native lifecycle fake-root attack fixture is missing or unterminated",
  );
  const fakeRootFixture = source.slice(
    fakeRootFixtureStart,
    fakeRootFixtureEnd,
  );
  assertOrdered(
    fakeRootFixture,
    [
      "$defaultSignature = Get-AuthenticodeSignature -LiteralPath $unsignedPe",
      "SignatureStatus]::Valid",
      "Invoke-WebView2SignatureVerification",
      "webview2-current-user-fake-root-negative",
    ],
    "native CurrentUser fake-root trust attack fixture",
  );
  contract(
    /if \(\$null -ne \$publisherStore[\s\S]*?try \{[\s\S]*?\$publisherStore\.Remove\(\$leafPublic\)[\s\S]*?\} catch \{[\s\S]*?Fake TrustedPublisher cleanup failed[\s\S]*?\}\s*\}\s*if \(\$null -ne \$rootStore[\s\S]*?try \{[\s\S]*?\$rootStore\.Remove\(\$rootPublic\)[\s\S]*?\} catch \{[\s\S]*?Fake CurrentUser root cleanup failed/u.test(
      fakeRootFixture,
    ),
    "native fake-root cleanup must independently remove both CurrentUser certificates",
  );
  contract(
    /function Assert-FailedRuntimeBootstrapLeftNoInstallWrites[\s\S]*?Test-Path -LiteralPath \$CandidateInstallDirectory/u.test(
      source,
    ),
    "rejected machine-runtime cases must prove the final directory was never created",
  );
  contract(
    !source.includes("Remove-Item -LiteralPath $userProfileFyagentDirectory"),
    "native lifecycle must never delete a pre-existing real user-data parent",
  );
  contract(
    /finally \{[\s\S]*?if \(\$cleanupAuthorized\) \{[\s\S]*?Invoke-BestEffortNsisUninstall/u.test(
      source,
    ),
    "destructive lifecycle cleanup must remain behind clean-run authorization",
  );
}

function assertConfigContract(baseConfig, windowsConfig) {
  contract(
    baseConfig?.bundle?.windows === undefined,
    "base Tauri config must not retain a Windows WiX/MSI surface",
  );
  contract(
    JSON.stringify(windowsConfig?.bundle?.targets) === JSON.stringify(["nsis"]),
    "Windows override must bundle exactly NSIS",
  );
  contract(
    JSON.stringify(windowsConfig?.bundle?.externalBin) ===
      JSON.stringify(["binaries/fyagent-user-helper"]),
    "Windows bundle must package exactly the fixed current-user helper binary",
  );
  const windows = windowsConfig?.bundle?.windows;
  contract(
    windows && typeof windows === "object",
    "Windows bundle config is missing",
  );
  contract(
    windows.wix === undefined,
    "Windows override must not configure WiX",
  );
  contract(
    JSON.stringify(windows.webviewInstallMode) ===
      JSON.stringify({ type: "downloadBootstrapper" }),
    "WebView2 mode must be downloadBootstrapper",
  );

  const nsis = windows.nsis;
  contract(
    nsis?.template === "nsis/installer.nsi",
    "custom NSIS template path drifted",
  );
  contract(
    nsis?.installerHooks === "nsis/webview2-command.nsh",
    "secure WebView2 encoded-command include path drifted",
  );
  contract(
    nsis?.installerIcon === "icons/icon.ico",
    "NSIS installer icon must use the canonical FyAgent icon",
  );
  contract(
    nsis?.installMode === "perMachine",
    "NSIS installMode must be perMachine",
  );
  contract(
    JSON.stringify(nsis?.languages) ===
      JSON.stringify(["English", "SimpChinese"]),
    "NSIS languages must be English and SimpChinese",
  );
  contract(
    nsis?.displayLanguageSelector === false,
    "installer language must follow the OS without a selector",
  );
}

export function verifyWindowsNsisContract(options = {}) {
  const baseConfigPath = path.resolve(
    options.baseConfigPath ??
      path.join(DEFAULT_ROOT, "src-tauri", "tauri.conf.json"),
  );
  const windowsConfigPath = path.resolve(
    options.windowsConfigPath ??
      path.join(DEFAULT_ROOT, "src-tauri", "tauri.windows.conf.json"),
  );
  const templatePath = path.resolve(
    options.templatePath ??
      path.join(DEFAULT_ROOT, "src-tauri", "nsis", "installer.nsi"),
  );
  const cargoManifestPath = path.resolve(
    options.cargoManifestPath ??
      path.join(DEFAULT_ROOT, "src-tauri", "Cargo.toml"),
  );
  const webviewSourcePath = path.resolve(
    options.webviewSourcePath ??
      path.join(
        DEFAULT_ROOT,
        "src-tauri",
        "nsis",
        "install-webview2-bootstrapper.ps1",
      ),
  );
  const webviewLoaderPath = path.resolve(
    options.webviewLoaderPath ??
      path.join(
        DEFAULT_ROOT,
        "src-tauri",
        "nsis",
        "load-encoded-webview2-command.ps1",
      ),
  );
  const webviewIncludePath = path.resolve(
    options.webviewIncludePath ??
      path.join(DEFAULT_ROOT, "src-tauri", "nsis", "webview2-command.nsh"),
  );
  const lifecyclePath =
    options.lifecyclePath === undefined
      ? null
      : path.resolve(options.lifecyclePath);
  const fakeRootCertificatePath = path.resolve(
    options.fakeRootCertificatePath ??
      path.join(
        DEFAULT_ROOT,
        "tests",
        "fixtures",
        "windows-nsis",
        "fake-current-user-root.pem",
      ),
  );
  const fakeLeafCertificatePath = path.resolve(
    options.fakeLeafCertificatePath ??
      path.join(
        DEFAULT_ROOT,
        "tests",
        "fixtures",
        "windows-nsis",
        "fake-microsoft-code-signing-leaf.pem",
      ),
  );

  const baseConfig = readJson(baseConfigPath, "base Tauri config");
  const windowsConfig = readJson(windowsConfigPath, "Windows Tauri config");
  const source = fs.readFileSync(templatePath, "utf8");
  const executableSource = stripNsisComments(source);
  const webviewSource = fs.readFileSync(webviewSourcePath, "utf8");
  const webviewLoader = fs.readFileSync(webviewLoaderPath, "utf8");
  const webviewInclude = fs.readFileSync(webviewIncludePath, "utf8");
  const lifecycleSource =
    lifecyclePath === null ? null : fs.readFileSync(lifecyclePath, "utf8");
  const fakeRootPem = fs.readFileSync(fakeRootCertificatePath, "utf8");
  const fakeLeafPem = fs.readFileSync(fakeLeafCertificatePath, "utf8");
  const blocks = parseNsisBlocks(source);

  assertConfigContract(baseConfig, windowsConfig);
  const workspaceVersion = readWorkspaceVersion(cargoManifestPath);
  try {
    assertWindowsBundleVersion(workspaceVersion);
  } catch (error) {
    contract(
      false,
      `canonical Cargo version cannot be bundled by NSIS: ${
        error instanceof Error ? error.message : String(error)
      }`,
    );
  }
  for (const [label, value] of Object.entries(TAURI_NSIS_UPSTREAM)) {
    contract(
      source.includes(value),
      `template provenance is missing upstream ${label}`,
    );
  }
  contract(
    executableSource.includes("RequestExecutionLevel admin"),
    "per-machine template must require administrator execution",
  );
  assertCanonicalIconContract(source, [webviewInclude]);
  assertWarning6000PackagingContract(source, [webviewInclude]);
  contract(
    executableSource.includes(
      'StrCpy $INSTDIR "$PROGRAMFILES64\\${PRODUCTNAME}"',
    ),
    "64-bit default install path must remain Program Files",
  );
  contract(
    !/(?:WixMode|wix_loop|Uninstall previous WiX installation)/iu.test(
      executableSource,
    ),
    "unbounded retired MSI/WiX migration logic remains executable",
  );

  assertInstallPathPolicyContract(source, [webviewInclude]);
  assertProcessStopGateContract(source, blocks);
  assertLegacyWixMigrationContract(source, blocks);
  assertLegacyRuntimeCleanupContract(source, blocks);
  assertKnownStagingCleanupContract(source, blocks);
  assertUninstallOwnershipContract(source, blocks);
  assertWebView2CommandContract({
    source: webviewSource,
    include: webviewInclude,
    loader: webviewLoader,
    loaderPath: webviewLoaderPath,
    template: source,
    blocks,
    fakeRootPem,
    fakeLeafPem,
  });
  if (lifecycleSource !== null) {
    assertLifecycleContract(lifecycleSource);
  }

  return Object.freeze({
    templatePath,
    baseConfigPath,
    windowsConfigPath,
    cargoManifestPath,
    lifecyclePath,
    workspaceVersion,
    upstream: TAURI_NSIS_UPSTREAM,
    sectionOrder: blocks
      .filter((block) => block.kind === "section")
      .map((block) => block.name),
  });
}

function parseArguments(argv) {
  const options = {};
  const keys = new Map([
    ["--base-config", "baseConfigPath"],
    ["--windows-config", "windowsConfigPath"],
    ["--template", "templatePath"],
    ["--cargo-manifest", "cargoManifestPath"],
    ["--webview-source", "webviewSourcePath"],
    ["--webview-loader", "webviewLoaderPath"],
    ["--webview-include", "webviewIncludePath"],
    ["--lifecycle", "lifecyclePath"],
    ["--fake-root-certificate", "fakeRootCertificatePath"],
    ["--fake-leaf-certificate", "fakeLeafCertificatePath"],
  ]);
  for (let index = 0; index < argv.length; index += 1) {
    const property = keys.get(argv[index]);
    contract(property, `unknown argument ${argv[index]}`);
    contract(index + 1 < argv.length, `${argv[index]} requires a path`);
    options[property] = argv[index + 1];
    index += 1;
  }
  return options;
}

const invokedPath = process.argv[1] ? path.resolve(process.argv[1]) : null;
if (invokedPath === fileURLToPath(import.meta.url)) {
  try {
    const result = verifyWindowsNsisContract(
      parseArguments(process.argv.slice(2)),
    );
    process.stdout.write(
      `Windows NSIS contract verified (${result.upstream.tag}; sections: ${result.sectionOrder.join(
        ", ",
      )})\n`,
    );
  } catch (error) {
    process.stderr.write(
      `${error instanceof Error ? error.message : String(error)}\n`,
    );
    process.exitCode = 1;
  }
}
