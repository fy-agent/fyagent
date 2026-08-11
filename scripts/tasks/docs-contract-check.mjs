#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { ROOT, fail, isMain } from "./lib.mjs";
import { generateTaskDocs } from "./task-docs.mjs";
import { loadTaskDefinitions } from "./task-contract-check.mjs";

const GENERATED_DOC = "docs/fyagent/development/mise-tasks.md";
export const OPERATIONAL_TRELLIS_DOCUMENTS = Object.freeze([
  ".agents/skills/fyagent-trellis/SKILL.md",
]);
const MANUAL_SETUP_DOCUMENTS = new Set([
  ".agents/skills/fyagent-trellis/SKILL.md",
]);
export const NEW_CHECKOUT_GATE_MARKERS = Object.freeze({
  start: "<!-- fyagent:new-checkout-environment-gate:start -->",
  end: "<!-- fyagent:new-checkout-environment-gate:end -->",
});
const MISE_CLI = "mi" + "se";
const TRUST_ACTION = "tr" + "ust";
const MANUAL_SETUP_COMMAND_PARTS = Object.freeze([
  [MISE_CLI, TRUST_ACTION],
  [MISE_CLI, "run", "bootstrap"],
  [MISE_CLI, "run", "system:check"],
]);
const WHOLE_DOCUMENT_FORBIDDEN = Object.freeze([
  [/\bmise(?:\.exe)?\s+exec\s+--(?:\s|`|$)/i, "legacy mise exec command"],
  [/(?:^|[^A-Za-z0-9:-])\/finish-work\b/, "noncanonical /finish-work command"],
]);
const MISE_RUN_BOOLEAN_LONG_OPTIONS = new Set([
  "--affected",
  "--affected-explain",
  "--affected-json",
  "--continue-on-error",
  "--force",
  "--dry-run",
  "--quiet",
  "--raw",
  "--silent",
  "--deny-all",
  "--deny-env",
  "--deny-net",
  "--deny-read",
  "--deny-write",
  "--fresh-env",
  "--no-cache",
  "--no-deps",
  "--no-timings",
  "--skip-deps",
  "--skip-tools",
  "--task-cache-explain",
  "--task-cache-explain-json",
  "--task-cache-stats",
]);
const MISE_RUN_VALUE_LONG_OPTIONS = new Set([
  "--affected-base",
  "--affected-head",
  "--cd",
  "--jobs",
  "--output",
  "--shell",
  "--tool",
  "--allow-env",
  "--allow-net",
  "--allow-read",
  "--allow-write",
  "--task-cache",
  "--timeout",
]);
const MISE_RUN_BOOLEAN_SHORT_OPTIONS = new Set(["c", "f", "n", "q", "r", "S"]);
const MISE_RUN_VALUE_SHORT_OPTIONS = new Set(["C", "j", "o", "s", "t"]);
const PYTHON_OPTIONS_WITH_VALUES = new Set([
  "-X",
  "-W",
  "--check-hash-based-pycs",
]);
const UV_GLOBAL_OPTIONS_WITH_VALUES = new Set([
  "--allow-insecure-host",
  "--cache-dir",
  "--color",
  "--config-file",
  "--directory",
  "--project",
  "--python",
  "-p",
]);
const UV_RUN_OPTIONS_WITH_VALUES = new Set([
  "--extra",
  "--no-extra",
  "--group",
  "--no-group",
  "--only-group",
  "--no-editable-package",
  "--env-file",
  "--with",
  "-w",
  "--with-editable",
  "--with-requirements",
  "--package",
  "--python-platform",
  "--index",
  "--default-index",
  "--index-url",
  "-i",
  "--extra-index-url",
  "--find-links",
  "-f",
  "--index-strategy",
  "--keyring-provider",
  "--upgrade-package",
  "-P",
  "--upgrade-group",
  "--resolution",
  "--prerelease",
  "--fork-strategy",
  "--exclude-newer",
  "--exclude-newer-package",
  "--no-sources-package",
  "--reinstall-package",
  "--link-mode",
  "--config-setting",
  "-C",
  "--config-settings-package",
  "--no-build-isolation-package",
  "--no-build-package",
  "--no-binary-package",
  "--cache-dir",
  "--refresh-package",
  "--python",
  "-p",
  ...UV_GLOBAL_OPTIONS_WITH_VALUES,
]);
const MAX_COMMAND_SUBSTITUTION_DEPTH = 3;
const COMMAND_WRAPPERS = Object.freeze({
  command: Object.freeze({}),
  exec: Object.freeze({}),
  nice: Object.freeze({ optionsWithValues: new Set(["-n", "--adjustment"]) }),
  nohup: Object.freeze({}),
  sudo: Object.freeze({
    optionsWithValues: new Set([
      "-D",
      "--chdir",
      "-g",
      "--group",
      "-u",
      "--user",
    ]),
  }),
  time: Object.freeze({}),
});
const LEGACY_ENTRYPOINT_HANDOFF = new Set();

function walk(relativeRoot) {
  const absoluteRoot = path.join(ROOT, relativeRoot);
  if (!fs.existsSync(absoluteRoot)) return [];
  const files = [];
  const visit = (absolute) => {
    for (const entry of fs.readdirSync(absolute, { withFileTypes: true })) {
      const child = path.join(absolute, entry.name);
      if (entry.isDirectory()) visit(child);
      else if (entry.isFile() && entry.name.endsWith(".md")) {
        files.push(path.relative(ROOT, child).split(path.sep).join("/"));
      }
    }
  };
  if (fs.statSync(absoluteRoot).isFile()) return [relativeRoot];
  visit(absoluteRoot);
  return files;
}

export function isOperationalTrellisDocument(file) {
  return OPERATIONAL_TRELLIS_DOCUMENTS.includes(file);
}

function countOccurrences(source, needle) {
  let count = 0;
  let offset = 0;
  while ((offset = source.indexOf(needle, offset)) !== -1) {
    count += 1;
    offset += needle.length;
  }
  return count;
}

function extractNewCheckoutGate(file, source) {
  const { start, end } = NEW_CHECKOUT_GATE_MARKERS;
  if (
    countOccurrences(source, start) !== 1 ||
    countOccurrences(source, end) !== 1
  ) {
    throw new Error(
      `${file} must contain exactly one bounded new-checkout gate`,
    );
  }
  const startIndex = source.indexOf(start);
  const endIndex = source.indexOf(end);
  if (endIndex <= startIndex + start.length) {
    throw new Error(`${file} has an invalid new-checkout gate boundary`);
  }
  return source.slice(startIndex + start.length, endIndex);
}

function fencedCodeBlocks(source) {
  const blocks = [];
  const pattern =
    /(?:^|\n)[ \t]*(`{3,}|~{3,})([^\n]*)\n([\s\S]*?)\n[ \t]*\1[ \t]*(?=\n|$)/g;
  for (const match of source.replace(/\r\n/g, "\n").matchAll(pattern)) {
    blocks.push({ info: match[2].trim(), body: match[3] });
  }
  return blocks;
}

function hasAffirmativeCheckoutScope(source) {
  const pattern =
    /\b(?:(?:for|on|in|when|before|after)\s+)?(?:every|each|a|any)\s+(?:new|fresh)\s+(?:repository\s+)?checkout\b/gi;
  for (const match of source.matchAll(pattern)) {
    const prefix = source.slice(Math.max(0, match.index - 48), match.index);
    if (!/\b(?:not|never|no|without|except|unless)\b[^.!?\n]*$/i.test(prefix)) {
      return true;
    }
  }
  return false;
}

function commandStrings() {
  return MANUAL_SETUP_COMMAND_PARTS.map((parts) => parts.join(" "));
}

function validateManualSetupGuidance(file, source) {
  const block = extractNewCheckoutGate(file, source);
  if (!hasAffirmativeCheckoutScope(block)) {
    throw new Error(`${file} must scope the gate to a new or fresh checkout`);
  }

  const prose = block
    .replace(/`{3,}[\s\S]*?`{3,}/g, " ")
    .replace(/~{3,}[\s\S]*?~{3,}/g, " ")
    .replace(/`[^`\n]*`/g, " code ");
  const review = /\bhuman\s+developer\b([^.!?]{0,320})\breview\b/i.exec(prose);
  if (!review || /\b(?:not|never|without)\b/i.test(review[1])) {
    throw new Error(`${file} must assign explicit review to a human developer`);
  }

  const manualRun =
    /\b(?:human\s+)?developer\b([^.!?]{0,320})\bmanually\b([^.!?]{0,100})\brun(?:s|ning)?\b/i.exec(
      prose,
    );
  if (
    !manualRun ||
    /\b(?:not|never|without)\b/i.test(`${manualRun[1]} ${manualRun[2]}`)
  ) {
    throw new Error(
      `${file} must assign manual setup execution to the developer`,
    );
  }

  const blocks = fencedCodeBlocks(block);
  const commands = commandStrings();
  if (
    blocks.length !== 1 ||
    blocks[0].body.replace(/\r\n/g, "\n").trim() !== commands.join("\n")
  ) {
    throw new Error(
      `${file} must contain one exact ordered setup command fence: ${commands.join(
        ", ",
      )}`,
    );
  }

  const sentences = block
    .replace(/\r\n/g, "\n")
    .split(/(?<=[.!?])\s+/)
    .map((sentence) => sentence.replace(/\s+/g, " "));
  const noAutomaticSetup = sentences.some(
    (sentence) =>
      /\bskills?\b/i.test(sentence) &&
      /\bhooks?\b/i.test(sentence) &&
      /\brepository\s+tasks?\b/i.test(sentence) &&
      /\b(?:must\s+not|do\s+not|never)\b/i.test(sentence) &&
      /\bautomatically\b/i.test(sentence) &&
      sentence.includes(commands[0]) &&
      sentence.includes(commands[1]),
  );
  if (!noAutomaticSetup) {
    throw new Error(
      `${file} must tie the no-automatic rule to skills, hooks, repository tasks, trust, and bootstrap`,
    );
  }
}

function splitContinuation(value) {
  const trimmed = value.trimEnd();
  const marker = trimmed.at(-1);
  if (!new Set(["\\", "`", "^"]).has(marker)) {
    return { continued: false, value: trimmed };
  }
  if (marker === "`" && (trimmed.match(/`/g)?.length ?? 0) > 1) {
    return { continued: false, value: trimmed };
  }
  return { continued: true, value: trimmed.slice(0, -1).trimEnd() };
}

function stripDisplayPrefixes(value) {
  let rest = value.trim();
  let marked = false;
  for (;;) {
    const quote = rest.match(/^>\s?/);
    if (quote) {
      rest = rest.slice(quote[0].length).trimStart();
      marked = true;
      continue;
    }
    const list = rest.match(/^(?:[-+*]|\d+[.)])\s+/);
    if (list) {
      rest = rest.slice(list[0].length).trimStart();
      marked = true;
      continue;
    }
    break;
  }

  const prompt = rest.match(
    /^(?:\$|#|%|PS>|pwsh>|cmd>|[A-Za-z]:[^>]*>|[^\s]+@[^\s]+(?::[^\s]*)?[$#])\s+/i,
  );
  if (prompt) {
    rest = rest.slice(prompt[0].length).trimStart();
    marked = true;
  }

  const imperative = rest.match(
    /^(?:run|execute)\b(?:\s+(?:this|the)(?:\s+following)?\s+command)?\s*:?\s+(.+)$/i,
  );
  if (imperative) {
    rest = imperative[1].trim();
    marked = true;
  }

  const inline = rest.match(/^(`+)([\s\S]*?)\1[.,;:]?$/);
  if (inline) rest = inline[2].trim();
  if (/^(?:(?:python(?:3(?:\.\d+)*)?|py|uv|grep)(?:\.exe)?)\b\s+/i.test(rest)) {
    marked = true;
  }
  return { marked, value: rest };
}

function addLogicalCandidate(candidates, state, physicalLine, acceptPlain) {
  const displayed = stripDisplayPrefixes(physicalLine);
  if (!state.pending && !acceptPlain && !displayed.marked) return;
  const continuation = splitContinuation(displayed.value);
  state.pending = state.pending
    ? `${state.pending} ${continuation.value}`.trim()
    : continuation.value;
  if (!continuation.continued) {
    if (state.pending) candidates.add(state.pending);
    state.pending = "";
  }
}

function flushLogicalCandidate(candidates, state) {
  if (state.pending) candidates.add(state.pending);
  state.pending = "";
}

function isFenceClose(line, fence) {
  const trimmed = line.trim();
  return (
    trimmed.length >= fence.length &&
    [...trimmed].every((character) => character === fence.character)
  );
}

export function extractMarkdownCommandCandidates(source) {
  const candidates = new Set();
  const outside = { pending: "" };
  const inside = { pending: "" };
  let fence = null;

  for (const line of source.replace(/\r\n/g, "\n").split("\n")) {
    if (fence) {
      if (isFenceClose(line, fence)) {
        flushLogicalCandidate(candidates, inside);
        fence = null;
      } else {
        addLogicalCandidate(candidates, inside, line, true);
      }
      continue;
    }

    const opening = line.match(/^\s*(`{3,}|~{3,})[^\n]*$/);
    if (opening) {
      flushLogicalCandidate(candidates, outside);
      fence = {
        character: opening[1][0],
        length: opening[1].length,
      };
      continue;
    }

    for (const inline of line.matchAll(/(?<!`)(`+)([^`\n]+?)\1(?!`)/g)) {
      candidates.add(inline[2].trim());
    }
    addLogicalCandidate(candidates, outside, line, false);
  }

  flushLogicalCandidate(candidates, outside);
  flushLogicalCandidate(candidates, inside);
  return [...candidates].filter(Boolean);
}

function tokenizeCommand(source) {
  const tokens = [];
  let current = "";
  let quote = null;
  const push = () => {
    if (current) tokens.push(current);
    current = "";
  };

  for (let index = 0; index < source.length; index += 1) {
    const character = source[index];
    if (quote) {
      if (character === quote) {
        quote = null;
      } else if (
        character === "\\" &&
        quote === '"' &&
        ["\\", '"'].includes(source[index + 1])
      ) {
        current += source[index + 1];
        index += 1;
      } else {
        current += character;
      }
      continue;
    }
    if (character === '"' || character === "'") {
      quote = character;
    } else if (/\s/.test(character)) {
      push();
    } else if (character === "\\" && /[\s"']/.test(source[index + 1] ?? "")) {
      current += source[index + 1];
      index += 1;
    } else {
      current += character;
    }
  }
  push();
  return tokens;
}

function shellCommandSegments(source, depth = 0) {
  const segments = [];
  let current = "";
  let quote = null;
  const push = () => {
    const segment = current.trim();
    if (segment) segments.push(segment);
    current = "";
  };

  for (let index = 0; index < source.length; index += 1) {
    const character = source[index];
    if (quote) {
      if (character === quote) {
        quote = null;
      } else if (
        character === "$" &&
        quote === '"' &&
        source[index + 1] === "("
      ) {
        if (depth >= MAX_COMMAND_SUBSTITUTION_DEPTH) {
          throw new Error(
            "command substitution nesting exceeds the parser limit",
          );
        }
        const substitution = findCommandSubstitutionEnd(source, index + 2);
        if (substitution !== null) {
          segments.push(
            ...shellCommandSegments(
              source.slice(index + 2, substitution),
              depth + 1,
            ),
          );
          current += source.slice(index, substitution + 1);
          index = substitution;
        } else {
          current += character;
        }
      } else if (character === "\\" && quote === '"') {
        current += character;
        if (source[index + 1] !== undefined) {
          current += source[index + 1];
          index += 1;
        }
      } else {
        current += character;
      }
      continue;
    }

    if (character === "\\") {
      current += character;
      if (source[index + 1] !== undefined) {
        current += source[index + 1];
        index += 1;
      }
      continue;
    }
    if (character === '"' || character === "'") {
      quote = character;
      current += character;
      continue;
    }
    if (character === "$" && source[index + 1] === "(") {
      if (depth >= MAX_COMMAND_SUBSTITUTION_DEPTH) {
        throw new Error(
          "command substitution nesting exceeds the parser limit",
        );
      }
      const substitution = findCommandSubstitutionEnd(source, index + 2);
      if (substitution !== null) {
        segments.push(
          ...shellCommandSegments(
            source.slice(index + 2, substitution),
            depth + 1,
          ),
        );
        current += source.slice(index, substitution + 1);
        index = substitution;
        continue;
      }
    }
    if (character === ";" || character === "|") {
      if (character === "|" && ["|", "&"].includes(source[index + 1])) {
        index += 1;
      }
      push();
      continue;
    }
    if (character === "&") {
      push();
      if (source[index + 1] === "&") index += 1;
      continue;
    }
    current += character;
  }
  push();
  return segments;
}

function findCommandSubstitutionEnd(source, startIndex) {
  let nested = 1;
  let quote = null;
  for (let index = startIndex; index < source.length; index += 1) {
    const character = source[index];
    if (quote) {
      if (character === quote) quote = null;
      else if (character === "\\" && quote === '"') index += 1;
      continue;
    }
    if (character === "\\") {
      index += 1;
      continue;
    }
    if (character === '"' || character === "'") {
      quote = character;
    } else if (character === "(") {
      nested += 1;
    } else if (character === ")") {
      nested -= 1;
      if (nested === 0) return index;
    }
  }
  return null;
}

function commandBasename(token) {
  return token
    .replace(/^[({[]+/, "")
    .replace(/[),;\]}]+$/, "")
    .replaceAll("\\", "/")
    .split("/")
    .at(-1);
}

function isEnvironmentAssignment(token) {
  return /^[A-Za-z_][A-Za-z0-9_]*=/.test(token);
}

function unwrapCommand(tokens) {
  let index = 0;

  for (;;) {
    while (isEnvironmentAssignment(tokens[index] ?? "")) index += 1;
    const program = commandBasename(tokens[index] ?? "").toLowerCase();
    if (/^env(?:\.exe)?$/.test(program)) {
      index += 1;
      while (index < tokens.length) {
        const token = tokens[index];
        if (token === "--") {
          index += 1;
          break;
        }
        if (isEnvironmentAssignment(token)) {
          index += 1;
          continue;
        }
        if (
          token === "-C" ||
          token === "--chdir" ||
          token === "-u" ||
          token === "--unset"
        ) {
          index += 2;
          continue;
        }
        if (token.startsWith("-")) {
          index += 1;
          continue;
        }
        break;
      }
      continue;
    }

    const wrapper = COMMAND_WRAPPERS[program];
    if (!wrapper) return tokens.slice(index);
    index += 1;
    while (index < tokens.length) {
      const token = tokens[index];
      if (token === "--") {
        index += 1;
        break;
      }
      if (!token.startsWith("-")) break;
      const option = token.split("=", 1)[0];
      index += 1;
      if (wrapper.optionsWithValues?.has(option) && !token.includes("=")) {
        index += 1;
      }
    }
  }
}

function isPythonLauncher(token) {
  return /^(?:python(?:3(?:\.\d+)*)?|py)(?:\.exe)?$/i.test(
    commandBasename(token),
  );
}

function cleanOperand(token) {
  return token.replace(/^[({[]+/, "").replace(/[),;\]}:!?]+$/, "");
}

function isTrellisPythonScript(token) {
  const normalized = cleanOperand(token)
    .replaceAll("\\", "/")
    .replace(/^(?:\.\/)+/, "");
  return /^\.trellis\/scripts\/(?:[^/\s]+\/)*[^/\s]+\.py$/i.test(normalized);
}

function firstPythonScriptOperand(tokens, startIndex) {
  for (let index = startIndex; index < tokens.length; index += 1) {
    const token = tokens[index];
    if (token === "--") return tokens[index + 1] ?? null;
    if (token === "-c" || token === "-m" || /^-(?:c|m).+/.test(token)) {
      return null;
    }
    if (PYTHON_OPTIONS_WITH_VALUES.has(token)) {
      index += 1;
      continue;
    }
    if (token.startsWith("-")) continue;
    return token;
  }
  return null;
}

function skipOptions(tokens, startIndex, optionsWithValues) {
  let index = startIndex;
  for (; index < tokens.length; index += 1) {
    const token = tokens[index];
    if (token === "--") return index + 1;
    if (!token.startsWith("-") || token === "-") return index;
    const option = token.split("=", 1)[0];
    if (optionsWithValues.has(option) && !token.includes("=")) index += 1;
  }
  return index;
}

function firstUvRunCommand(tokens) {
  const runIndex = skipOptions(tokens, 1, UV_GLOBAL_OPTIONS_WITH_VALUES);
  if (tokens[runIndex]?.toLowerCase() !== "run") return null;
  const commandIndex = skipOptions(
    tokens,
    runIndex + 1,
    UV_RUN_OPTIONS_WITH_VALUES,
  );
  return commandIndex < tokens.length ? commandIndex : null;
}

function hasRecursiveGrepOption(tokens) {
  for (const token of tokens.slice(1)) {
    if (token === "--") return false;
    if (token === "--recursive" || token.startsWith("--recursive=")) {
      return true;
    }
    if (/^-[^-]*[rR]/.test(token)) return true;
  }
  return false;
}

function validateCommandCandidates(file, source) {
  for (const candidate of extractMarkdownCommandCandidates(source)) {
    for (const segment of shellCommandSegments(candidate)) {
      const tokens = unwrapCommand(tokenizeCommand(segment));
      if (tokens.length === 0) continue;
      const program = commandBasename(tokens[0]).toLowerCase();

      if (isPythonLauncher(tokens[0])) {
        const operand = firstPythonScriptOperand(tokens, 1);
        if (operand && isTrellisPythonScript(operand)) {
          throw new Error(
            `direct Python/py Trellis script command found in ${file}`,
          );
        }
      }

      if (/^uv(?:\.exe)?$/i.test(program)) {
        const commandIndex = firstUvRunCommand(tokens);
        if (commandIndex !== null) {
          const command = tokens[commandIndex];
          const script = isPythonLauncher(command)
            ? firstPythonScriptOperand(tokens, commandIndex + 1)
            : command;
          if (script && isTrellisPythonScript(script)) {
            throw new Error(
              `direct uv Trellis script command found in ${file}`,
            );
          }
        }
      }

      if (/^grep(?:\.exe)?$/i.test(program) && hasRecursiveGrepOption(tokens)) {
        throw new Error(`recursive grep command found in ${file}; use rg`);
      }
    }
  }
}

function continuedSourceLines(source) {
  const lines = [];
  let pending = "";
  for (const physicalLine of source.replace(/\r\n/g, "\n").split("\n")) {
    const continuation = splitContinuation(physicalLine);
    pending = pending
      ? `${pending} ${continuation.value.trimStart()}`
      : continuation.value;
    if (!continuation.continued) {
      lines.push(pending);
      pending = "";
    }
  }
  if (pending) lines.push(pending);
  return lines;
}

function consumeMiseRunOption(file, tokens, index) {
  const token = tokens[index];
  if (token.startsWith("--")) {
    const [option, inlineValue] = token.split("=", 2);
    if (MISE_RUN_BOOLEAN_LONG_OPTIONS.has(option)) {
      if (inlineValue !== undefined) {
        throw new Error(`${file} has invalid mise run option: ${token}`);
      }
      return index + 1;
    }
    if (MISE_RUN_VALUE_LONG_OPTIONS.has(option)) {
      if (inlineValue !== undefined) {
        if (!inlineValue) {
          throw new Error(
            `${file} has missing mise run option value: ${option}`,
          );
        }
        return index + 1;
      }
      if (
        tokens[index + 1] === undefined ||
        tokens[index + 1].startsWith("--")
      ) {
        throw new Error(`${file} has missing mise run option value: ${option}`);
      }
      return index + 2;
    }
    throw new Error(`${file} has unknown mise run option: ${token}`);
  }

  const short = token.slice(1);
  if (
    [...short].every((option) => MISE_RUN_BOOLEAN_SHORT_OPTIONS.has(option))
  ) {
    return index + 1;
  }
  const valueOption = short[0];
  if (MISE_RUN_VALUE_SHORT_OPTIONS.has(valueOption)) {
    if (short.length > 1) return index + 1;
    if (tokens[index + 1] === undefined || tokens[index + 1].startsWith("--")) {
      throw new Error(
        `${file} has missing mise run option value: -${valueOption}`,
      );
    }
    return index + 2;
  }
  throw new Error(`${file} has unknown mise run option: ${token}`);
}

function cleanTaskReference(token) {
  const cleaned = token.replace(/^[`*(\[]+/, "").replace(/[`*),;\].!?]+$/, "");
  if (cleaned === "<task>") return cleaned;
  if (!cleaned || /[\s`'"<>|&;()[\]{}]/.test(cleaned)) {
    return null;
  }
  return cleaned;
}

function parseMiseTaskReference(file, reference) {
  const tokens = tokenizeCommand(reference);
  let index = 2;
  while (index < tokens.length && tokens[index].startsWith("-")) {
    if (tokens[index] === "--") {
      index += 1;
      break;
    }
    index = consumeMiseRunOption(file, tokens, index);
  }
  const name = cleanTaskReference(tokens[index] ?? "");
  if (!name) throw new Error(`${file} has an invalid mise run task reference`);
  return name;
}

export function validateMiseTaskReferences(file, source, tasks) {
  for (const line of continuedSourceLines(source)) {
    const pattern = /\bmise(?:\.exe)?\s+run\b/gi;
    for (const match of line.matchAll(pattern)) {
      let reference = line.slice(match.index);
      const inlineEnd = reference.indexOf("`");
      if (inlineEnd !== -1) reference = reference.slice(0, inlineEnd);
      const name = parseMiseTaskReference(file, reference);
      if (name === "<task>") continue;
      if (!Object.hasOwn(tasks, name)) {
        throw new Error(`${file} references unknown mise task: ${name}`);
      }
    }
  }
}

export function validateOperationalTrellisDocument(file, source, tasks) {
  for (const [pattern, label] of WHOLE_DOCUMENT_FORBIDDEN) {
    if (pattern.test(source)) {
      throw new Error(`${label} found in ${file}`);
    }
  }

  validateCommandCandidates(file, source);
  validateMiseTaskReferences(file, source, tasks);

  if (MANUAL_SETUP_DOCUMENTS.has(file)) {
    validateManualSetupGuidance(file, source);
  }
}

export function validateDocsContract() {
  const generatedPath = path.join(ROOT, GENERATED_DOC);
  if (!fs.existsSync(generatedPath))
    throw new Error(`Missing ${GENERATED_DOC}`);
  const committed = fs
    .readFileSync(generatedPath, "utf8")
    .replace(/\r\n/g, "\n");
  if (committed !== generateTaskDocs()) {
    throw new Error("Generated task documentation is stale");
  }

  const tasks = loadTaskDefinitions();
  validateMiseTaskReferences(GENERATED_DOC, committed, tasks);

  for (const file of OPERATIONAL_TRELLIS_DOCUMENTS) {
    const absolute = path.join(ROOT, file);
    if (!fs.existsSync(absolute)) {
      throw new Error(`Missing operational Trellis document: ${file}`);
    }
    const source = fs.readFileSync(absolute, "utf8");
    validateOperationalTrellisDocument(file, source, tasks);
  }

  const activeDocs = [
    ...[
      "README.md",
      "README_ZH.md",
      "README_JA.md",
      "README_DE.md",
      "CONTRIBUTING.md",
    ],
    ...walk(".github").filter((file) => file.endsWith(".md")),
    ...walk(".trellis/spec/backend"),
    ...walk(".trellis/spec/frontend"),
    ...walk("docs/fyagent/development"),
  ];
  const legacy = [];
  for (const file of [...new Set(activeDocs)].sort()) {
    const source = fs.readFileSync(path.join(ROOT, file), "utf8");
    if (!source.includes("mise exec --")) continue;
    if (!LEGACY_ENTRYPOINT_HANDOFF.has(file)) {
      throw new Error(
        `Untracked legacy mise exec entrypoint in active document: ${file}`,
      );
    }
    legacy.push(file);
  }
  return {
    ok: true,
    generated: GENERATED_DOC,
    operationalTrellisDocuments: [...OPERATIONAL_TRELLIS_DOCUMENTS],
    legacyEntrypointHandoff: legacy,
    handoffOwner: null,
  };
}

if (isMain(import.meta.url)) {
  try {
    console.log(JSON.stringify(validateDocsContract(), null, 2));
  } catch (error) {
    fail(error);
  }
}
