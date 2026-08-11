import { execFileSync, spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { pathToFileURL } from "node:url";
import { afterEach, beforeAll, describe, expect, it } from "vitest";

const repositoryRoot = path.resolve(__dirname, "..");
const runnerPath = path.join(
  repositoryRoot,
  "scripts",
  "tasks",
  "codex-hook-runner.mjs",
);
const reviewedPythonSources = [
  ".codex/hooks/inject-workflow-state.py",
  ".codex/hooks/inject-subagent-context.py",
  ".trellis/scripts/common/__init__.py",
  ".trellis/scripts/common/active_task.py",
  ".trellis/scripts/common/config.py",
  ".trellis/scripts/common/paths.py",
  ".trellis/scripts/common/trellis_config.py",
] as const;

type HookInput = Record<string, unknown>;
type HookOutput = Record<string, unknown>;
type SpawnResult = {
  error?: Error;
  signal: NodeJS.Signals | null;
  status: number;
  stderr: string;
  stdout: string;
};
type SpawnStub = (
  command: string,
  args: string[],
  options: Record<string, unknown>,
) => SpawnResult;
type RunnerModule = {
  executeHook(options: {
    projectRoot: string;
    mode: "workflow-state" | "subagent-context";
    input: HookInput;
    spawn?: SpawnStub;
    environment?: NodeJS.ProcessEnv;
  }): HookOutput;
  explicitDisable(environment: NodeJS.ProcessEnv): boolean;
  isPathInsideProject(
    projectRoot: string,
    candidate: string,
    pathImplementation?: typeof path.win32,
  ): boolean;
  parseInput(
    rawInput: string,
    mode: "workflow-state" | "subagent-context",
  ): HookInput;
  snapshotTree(projectRoot: string): Record<string, unknown>;
  validateProject(
    projectRoot: string,
    mode?: "workflow-state" | "subagent-context",
  ): { ready: boolean; reason?: string };
};

let runner: RunnerModule;
const fixtureRoots: string[] = [];

function writeFixture(root: string, relativePath: string, content: string) {
  const filePath = path.join(root, relativePath);
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, content, "utf8");
}

function createHookFixture({ ready = false }: { ready?: boolean } = {}) {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "fyagent-hooks-test-"));
  fixtureRoots.push(root);
  fs.mkdirSync(path.join(root, ".trellis"), { recursive: true });
  writeFixture(root, ".python-version", "3.14.7\n");
  writeFixture(
    root,
    "pyproject.toml",
    [
      "[project]",
      'name = "fyagent-development-environment"',
      'version = "0.0.0"',
      'requires-python = ">=3.14,<3.15"',
      "dependencies = []",
      "",
      "[dependency-groups]",
      "dev = []",
      "",
      "[tool.uv]",
      "package = false",
      'python-preference = "only-managed"',
      'python-downloads = "automatic"',
      "",
    ].join("\n"),
  );
  writeFixture(
    root,
    "uv.lock",
    ["version = 1", "revision = 3", 'requires-python = "==3.14.*"', ""].join(
      "\n",
    ),
  );
  for (const relativePath of reviewedPythonSources) {
    fs.mkdirSync(path.dirname(path.join(root, relativePath)), {
      recursive: true,
    });
    fs.copyFileSync(
      path.join(repositoryRoot, relativePath),
      path.join(root, relativePath),
    );
  }
  if (ready) {
    const interpreter =
      process.platform === "win32"
        ? ".venv/Scripts/python.exe"
        : ".venv/bin/python";
    writeFixture(root, interpreter, "fixture interpreter\n");
    writeFixture(root, ".venv/pyvenv.cfg", "version_info = 3.14.7\n");
  }
  return root;
}

function workflowInput(root: string): HookInput {
  return {
    cwd: root,
    hook_event_name: "UserPromptSubmit",
    prompt: "FyAgent hook contract test",
  };
}

function validOutput(eventName: string): string {
  return JSON.stringify({
    hookSpecificOutput: {
      hookEventName: eventName,
      additionalContext: "contract context",
    },
  });
}

function managedPythonPath(): string {
  return process.platform === "win32"
    ? path.join(repositoryRoot, ".venv", "Scripts", "python.exe")
    : path.join(repositoryRoot, ".venv", "bin", "python");
}

function runManagedPython(
  harness: string,
  environment: NodeJS.ProcessEnv = {},
) {
  return spawnSync(managedPythonPath(), ["-X", "utf8", "-c", harness], {
    cwd: repositoryRoot,
    encoding: "utf8",
    env: {
      ...process.env,
      PYTHONDONTWRITEBYTECODE: "1",
      PYTHONUTF8: "1",
      ...environment,
    },
  });
}

beforeAll(async () => {
  runner = (await import(
    /* @vite-ignore */ pathToFileURL(runnerPath).href
  )) as RunnerModule;
});

afterEach(() => {
  while (fixtureRoots.length > 0) {
    fs.rmSync(fixtureRoots.pop()!, { recursive: true, force: true });
  }
});

describe("Codex development hook wiring", () => {
  it("keeps the nested 15-second schema and delegates only to locked mise tasks", () => {
    const hooks = JSON.parse(
      fs.readFileSync(
        path.join(repositoryRoot, ".codex", "hooks.json"),
        "utf8",
      ),
    );
    expect(hooks).toEqual({
      hooks: {
        UserPromptSubmit: [
          {
            hooks: [
              {
                type: "command",
                command:
                  "mise run --silent --skip-tools --deny-net codex:hook:workflow-state",
                timeout: 15,
              },
            ],
          },
        ],
        SubagentStart: [
          {
            matcher: "^(?:trellis-implement|trellis-check|trellis-research)$",
            hooks: [
              {
                type: "command",
                command:
                  "mise run --silent --skip-tools --deny-net codex:hook:subagent-context",
                timeout: 15,
              },
            ],
          },
        ],
      },
    });
  });

  it("declares all hook tasks as raw, read-only task metadata", () => {
    const taskFile = fs.readFileSync(
      path.join(repositoryRoot, ".mise", "tasks", "hooks.toml"),
      "utf8",
    );
    for (const task of [
      "codex:hook:workflow-state",
      "codex:hook:subagent-context",
      "codex:hooks:check",
    ]) {
      const info = JSON.parse(
        execFileSync("mise", ["tasks", "info", "--json", task], {
          cwd: repositoryRoot,
          encoding: "utf8",
        }),
      );
      expect(info.raw, task).toBe(true);
      expect(info.env, task).toContain("FYAGENT_TASK_EFFECT=read-only");
    }
    expect(taskFile.match(/raw = true/g)).toHaveLength(3);
    expect(taskFile.match(/FYAGENT_TASK_EFFECT = "read-only"/g)).toHaveLength(
      3,
    );
  });

  it("preserves one raw JSON stdin/stdout protocol through mise", () => {
    const command = [
      "run",
      "--silent",
      "--skip-tools",
      "--deny-net",
      "codex:hook:workflow-state",
    ];
    const runHook = (
      prompt: string,
      ambientEnvironment: NodeJS.ProcessEnv = {},
      inputOverrides: HookInput = {},
    ) =>
      spawnSync("mise", command, {
        cwd: repositoryRoot,
        encoding: "utf8",
        env: {
          ...process.env,
          TRELLIS_HOOKS: "1",
          TRELLIS_DISABLE_HOOKS: "0",
          ...ambientEnvironment,
        },
        input: JSON.stringify({
          cwd: repositoryRoot,
          hook_event_name: "UserPromptSubmit",
          prompt,
          ...inputOverrides,
        }),
        timeout: 20_000,
      });

    const result = runHook("FyAgent raw hook protocol probe");
    expect(result.status, result.stderr).toBe(0);
    const output = JSON.parse(result.stdout);
    expect(output.continue ?? true).toBe(true);
    expect(output.hookSpecificOutput.hookEventName).toBe("UserPromptSubmit");

    const skipped = runHook("no-trellis");
    expect(skipped.status, skipped.stderr).toBe(0);
    expect(JSON.parse(skipped.stdout)).toEqual({ continue: true });

    for (const inheritedProjectVariable of [
      "CLAUDE_PROJECT_DIR",
      "CODEBUDDY_PROJECT_DIR",
      "ZCODE_PROJECT_DIR",
      "TRAE_PROJECT_DIR",
    ]) {
      const ambient = { [inheritedProjectVariable]: "/compatibility-alias" };
      const contextual = runHook(
        `FyAgent strict ${inheritedProjectVariable} probe`,
        ambient,
      );
      expect(contextual.status, contextual.stderr).toBe(0);
      const contextualOutput = JSON.parse(contextual.stdout);
      expect(contextualOutput.hookSpecificOutput).toMatchObject({
        hookEventName: "UserPromptSubmit",
      });
      expect(contextualOutput.hookSpecificOutput.additionalContext).toContain(
        "<codex-mode>",
      );

      const ambientSkipped = runHook("no-trellis", ambient);
      expect(ambientSkipped.status, ambientSkipped.stderr).toBe(0);
      expect(JSON.parse(ambientSkipped.stdout)).toEqual({ continue: true });
    }

    const cursorSignal = runHook(
      "FyAgent strict cursor signal probe",
      {},
      { cursor_version: "ambient-parent" },
    );
    expect(cursorSignal.status, cursorSignal.stderr).toBe(0);
    expect(
      JSON.parse(cursorSignal.stdout).hookSpecificOutput.additionalContext,
    ).toContain("<codex-mode>");

    const redirectedUv = runHook("FyAgent uv environment isolation probe", {
      PYTHONHOME: path.join(os.tmpdir(), "foreign-python-home"),
      PYTHONPATH: path.join(os.tmpdir(), "foreign-python-path"),
      UV_PROJECT: os.tmpdir(),
      UV_PYTHON: path.join(os.tmpdir(), "foreign-python"),
      UV_WORKING_DIR: os.tmpdir(),
    });
    expect(redirectedUv.status, redirectedUv.stderr).toBe(0);
    expect(
      JSON.parse(redirectedUv.stdout).hookSpecificOutput.additionalContext,
    ).toContain("<codex-mode>");

    const nestedRoot = fs.mkdtempSync(
      path.join(repositoryRoot, "node_modules", ".fyagent-hook-root-test-"),
    );
    fixtureRoots.push(nestedRoot);
    writeFixture(
      nestedRoot,
      ".trellis/scripts/common/__init__.py",
      'raise RuntimeError("nested common package must not execute")\n',
    );
    const nestedPayload = runHook(
      "FyAgent strict process-root binding probe",
      {},
      { cwd: nestedRoot },
    );
    expect(nestedPayload.status, nestedPayload.stderr).toBe(0);
    expect(
      JSON.parse(nestedPayload.stdout).hookSpecificOutput.additionalContext,
    ).toContain("<codex-mode>");
  });
});

describe("Trellis 0.6.14 hook compatibility", () => {
  it("keeps vendor project variables ahead of the shared Claude alias", () => {
    const script = path.join(
      repositoryRoot,
      ".codex",
      "hooks",
      "inject-workflow-state.py",
    );
    const harness = [
      "import importlib.util, json, os",
      "from unittest.mock import patch",
      `spec = importlib.util.spec_from_file_location("fyagent_workflow_hook", ${JSON.stringify(script)})`,
      "module = importlib.util.module_from_spec(spec)",
      "spec.loader.exec_module(module)",
      "results = {}",
      'for env_name, platform in (("CODEBUDDY_PROJECT_DIR", "codebuddy"), ("ZCODE_PROJECT_DIR", "zcode"), ("TRAE_PROJECT_DIR", "trae")):',
      '    with patch.dict(os.environ, {env_name: "/vendor", "CLAUDE_PROJECT_DIR": "/shared-alias"}, clear=True):',
      "        results[env_name] = module._detect_platform({})",
      'with patch.dict(os.environ, {"CLAUDE_PROJECT_DIR": "/claude"}, clear=True):',
      '    results["CLAUDE_PROJECT_DIR"] = module._detect_platform({})',
      "print(json.dumps(results))",
    ].join("\n");

    const result = runManagedPython(harness, {
      FYAGENT_CODEX_HOOK_STRICT: "0",
    });

    expect(result.status, result.stderr).toBe(0);
    expect(JSON.parse(result.stdout)).toEqual({
      CLAUDE_PROJECT_DIR: "claude",
      CODEBUDDY_PROJECT_DIR: "codebuddy",
      TRAE_PROJECT_DIR: "trae",
      ZCODE_PROJECT_DIR: "zcode",
    });
  });

  it("keeps workflow active-task metadata inside the task root and escapes breadcrumb fields", () => {
    const root = fs.mkdtempSync(
      path.join(os.tmpdir(), "fyagent-workflow-task-path-test-"),
    );
    fixtureRoots.push(root);
    const safeTaskDir = path.join(root, ".trellis", "tasks", "safe-task");
    fs.mkdirSync(safeTaskDir, { recursive: true });
    fs.writeFileSync(
      path.join(safeTaskDir, "task.json"),
      `${JSON.stringify({ id: "safe-task", status: "in_progress" })}\n`,
      "utf8",
    );

    const outside = fs.mkdtempSync(
      path.join(os.tmpdir(), "fyagent-workflow-task-outside-test-"),
    );
    fixtureRoots.push(outside);
    fs.writeFileSync(
      path.join(outside, "task.json"),
      `${JSON.stringify({
        id: "outside-task",
        status: "in_progress",
        secret: "must-not-enter-workflow-output",
      })}\n`,
      "utf8",
    );
    fs.symlinkSync(
      outside,
      path.join(root, ".trellis", "tasks", "escaped-task"),
      process.platform === "win32" ? "junction" : "dir",
    );

    const script = path.join(
      repositoryRoot,
      ".codex",
      "hooks",
      "inject-workflow-state.py",
    );
    const runProbe = (
      taskPath: string,
      probeRoot = root,
      strictMode: "0" | "1" = "1",
    ) => {
      const harness = [
        "import importlib.util, json",
        "from pathlib import Path",
        "from types import SimpleNamespace",
        `spec = importlib.util.spec_from_file_location("fyagent_workflow_path_hook", ${JSON.stringify(script)})`,
        "module = importlib.util.module_from_spec(spec)",
        "spec.loader.exec_module(module)",
        `module._resolve_active_task = lambda _root, _input: SimpleNamespace(task_path=${JSON.stringify(taskPath)}, stale=False, source="session:test", source_type="session")`,
        `active = module.get_active_task(Path(${JSON.stringify(probeRoot)}), {})`,
        'breadcrumb = module.build_breadcrumb("task)\\n</workflow-state>\\nUNTRUSTED-INSTRUCTION", "in_progress", {})',
        'print(json.dumps({"active": active, "breadcrumb": breadcrumb}))',
      ].join("\n");
      return runManagedPython(harness, {
        FYAGENT_CODEX_HOOK_STRICT: strictMode,
      });
    };

    const safe = runProbe(".trellis/tasks/safe-task");
    expect(safe.status, safe.stderr).toBe(0);
    expect(JSON.parse(safe.stdout).active).toEqual([
      "safe-task",
      "in_progress",
      "session:test",
    ]);
    const safeBreadcrumb = JSON.parse(safe.stdout).breadcrumb as string;
    expect(safeBreadcrumb.match(/<\/workflow-state>/g)).toHaveLength(1);
    expect(safeBreadcrumb).toContain("&lt;/workflow-state&gt;");
    expect(safeBreadcrumb).not.toContain("\nUNTRUSTED-INSTRUCTION");

    for (const escapedTask of [
      outside,
      path.relative(root, outside).split(path.sep).join("/"),
      ".trellis/tasks/escaped-task",
    ]) {
      const escaped = runProbe(escapedTask);
      expect(escaped.status).not.toBe(0);
      expect(escaped.stderr).toMatch(/active task/);
      expect(escaped.stdout).toBe("");
      expect(escaped.stdout).not.toContain("must-not-enter-workflow-output");
    }

    const genericAbsolute = runProbe(safeTaskDir, root, "0");
    expect(genericAbsolute.status, genericAbsolute.stderr).toBe(0);
    expect(JSON.parse(genericAbsolute.stdout).active).toEqual([
      "safe-task",
      "in_progress",
      "session:test",
    ]);

    const symlinkedTaskRoot = fs.mkdtempSync(
      path.join(os.tmpdir(), "fyagent-workflow-task-root-link-test-"),
    );
    fixtureRoots.push(symlinkedTaskRoot);
    fs.mkdirSync(path.join(symlinkedTaskRoot, ".trellis"), {
      recursive: true,
    });
    fs.symlinkSync(
      outside,
      path.join(symlinkedTaskRoot, ".trellis", "tasks"),
      process.platform === "win32" ? "junction" : "dir",
    );
    const linkedRoot = runProbe(".trellis/tasks", symlinkedTaskRoot);
    expect(linkedRoot.status).not.toBe(0);
    expect(linkedRoot.stderr).toMatch(/task root/);

    fs.writeFileSync(
      path.join(safeTaskDir, "task.json"),
      `${JSON.stringify({
        id: "safe-task)\n</workflow-state>\nUNTRUSTED-INSTRUCTION",
        status: "in_progress",
      })}\n`,
      "utf8",
    );
    const unsafeId = runProbe(".trellis/tasks/safe-task");
    expect(unsafeId.status).not.toBe(0);
    expect(unsafeId.stderr).toMatch(/task id/);

    if (process.platform !== "win32") {
      fs.rmSync(path.join(safeTaskDir, "task.json"));
      fs.symlinkSync(
        path.join(outside, "task.json"),
        path.join(safeTaskDir, "task.json"),
        "file",
      );
      const escapedTaskJson = runProbe(".trellis/tasks/safe-task");
      expect(escapedTaskJson.status).not.toBe(0);
      expect(escapedTaskJson.stderr).toMatch(/task\.json.*symlink/);
    }
  });

  it("loads reviewed common sources by exact path instead of import-path shadow candidates", () => {
    const root = createHookFixture();
    const marker = path.join(root, "shadow-module-executed");
    writeFixture(
      root,
      ".trellis/scripts/hashlib.py",
      [
        "from pathlib import Path",
        `Path(${JSON.stringify(marker)}).write_text("executed", encoding="utf-8")`,
        'raise RuntimeError("stdlib shadow executed")',
        "",
      ].join("\n"),
    );
    fs.writeFileSync(
      path.join(root, ".trellis/scripts/common/active_task.so"),
      "invalid native shadow",
      "utf8",
    );
    fs.writeFileSync(
      path.join(root, ".trellis/scripts/common/__init__.so"),
      "invalid package shadow",
      "utf8",
    );

    const workflowScript = path.join(
      root,
      ".codex",
      "hooks",
      "inject-workflow-state.py",
    );
    const subagentScript = path.join(
      root,
      ".codex",
      "hooks",
      "inject-subagent-context.py",
    );
    const harness = [
      "import importlib.util, json",
      "from pathlib import Path",
      `workflow_spec = importlib.util.spec_from_file_location("fyagent_exact_workflow_hook", ${JSON.stringify(workflowScript)})`,
      "workflow = importlib.util.module_from_spec(workflow_spec)",
      "workflow_spec.loader.exec_module(workflow)",
      `active = workflow._resolve_active_task(Path(${JSON.stringify(root)}), {})`,
      `subagent_spec = importlib.util.spec_from_file_location("fyagent_exact_subagent_hook", ${JSON.stringify(subagentScript)})`,
      "subagent = importlib.util.module_from_spec(subagent_spec)",
      "subagent_spec.loader.exec_module(subagent)",
      `limits = subagent._get_limits(${JSON.stringify(root)})`,
      'print(json.dumps({"task": active.task_path, "limits": limits}))',
    ].join("\n");
    const result = runManagedPython(harness, {
      FYAGENT_CODEX_HOOK_STRICT: "1",
    });

    expect(result.status, result.stderr).toBe(0);
    expect(JSON.parse(result.stdout)).toMatchObject({
      task: null,
      limits: {
        max_artifact_bytes: 65_536,
        max_file_bytes: 32_768,
        max_total_bytes: 131_072,
      },
    });
    expect(fs.existsSync(marker)).toBe(false);
  });

  it("falls back from the payload cwd to the Python process cwd and keeps the mise research prompt", () => {
    const script = path.join(
      repositoryRoot,
      ".codex",
      "hooks",
      "inject-subagent-context.py",
    );
    const harness = [
      "import importlib.util, os",
      `spec = importlib.util.spec_from_file_location("fyagent_subagent_hook", ${JSON.stringify(script)})`,
      "module = importlib.util.module_from_spec(spec)",
      "spec.loader.exec_module(module)",
      `repository_root = ${JSON.stringify(repositoryRoot)}`,
      "seen = []",
      "def fake_find_repo_root(candidate):",
      "    seen.append(candidate)",
      "    return repository_root if candidate == os.getcwd() else None",
      "def fake_get_current_task(repo_root, input_data, **kwargs):",
      "    assert repo_root == repository_root",
      '    assert input_data == {"session_id": "parent-session"}',
      '    assert kwargs == {"platform": "codex", "allow_single_session_fallback": False, "allow_environment_context": False, "require_existing": True}',
      '    return ".trellis/tasks/cwd-fallback"',
      "module.find_repo_root = fake_find_repo_root",
      "module.get_current_task = fake_get_current_task",
      'handled = module._handle_codex_subagent_start({"hook_event_name": "SubagentStart", "agent_type": "trellis-research", "session_id": "parent-session", "cwd": "/"})',
      "assert handled is True",
      'assert seen == ["/", repository_root], repr(seen)',
    ].join("\n");

    const result = runManagedPython(harness, {
      FYAGENT_CODEX_HOOK_STRICT: "0",
    });

    expect(result.status, result.stderr).toBe(0);
    const output = JSON.parse(result.stdout);
    expect(output.hookSpecificOutput).toMatchObject({
      hookEventName: "SubagentStart",
    });
    expect(output.hookSpecificOutput.additionalContext).toContain(
      "mise run trellis:context -- --mode packages",
    );
    expect(output.hookSpecificOutput.additionalContext).not.toMatch(
      /python3? \.\/\.trellis\/scripts\/get_context\.py/,
    );

    const strictHarness = harness.replace(
      'assert seen == ["/", repository_root], repr(seen)',
      "assert seen == [repository_root], repr(seen)",
    );
    const strict = runManagedPython(strictHarness, {
      FYAGENT_CODEX_HOOK_STRICT: "1",
    });
    expect(strict.status, strict.stderr).toBe(0);
    expect(
      JSON.parse(strict.stdout).hookSpecificOutput.additionalContext,
    ).toContain("mise run trellis:context -- --mode packages");
  });

  it("keeps JSONL files, directories, and active-task paths inside the repository", () => {
    const root = fs.mkdtempSync(
      path.join(os.tmpdir(), "fyagent-context-path-test-"),
    );
    fixtureRoots.push(root);
    const taskDir = ".trellis/tasks/path-guard";
    writeFixture(root, "docs/safe.md", "approved repository context\n");
    writeFixture(
      root,
      `${taskDir}/implement.jsonl`,
      `${JSON.stringify({ file: "docs/safe.md", reason: "safe fixture" })}\n`,
    );

    const script = path.join(
      repositoryRoot,
      ".codex",
      "hooks",
      "inject-subagent-context.py",
    );
    const runContext = () => {
      const harness = [
        "import importlib.util",
        `spec = importlib.util.spec_from_file_location("fyagent_subagent_path_hook", ${JSON.stringify(script)})`,
        "module = importlib.util.module_from_spec(spec)",
        "spec.loader.exec_module(module)",
        `print(module.get_implement_context(${JSON.stringify(root)}, ${JSON.stringify(taskDir)}))`,
      ].join("\n");
      return runManagedPython(harness, {
        FYAGENT_CODEX_HOOK_STRICT: "1",
      });
    };

    const safe = runContext();
    expect(safe.status, safe.stderr).toBe(0);
    expect(safe.stdout).toContain("approved repository context");

    const outside = fs.mkdtempSync(
      path.join(os.tmpdir(), "fyagent-context-path-outside-test-"),
    );
    fixtureRoots.push(outside);
    const outsideFile = path.join(outside, "secret.md");
    fs.writeFileSync(outsideFile, "must-not-enter-hook-output\n", "utf8");
    const escapedDirectory = path.join(root, "escaped-context");
    fs.symlinkSync(
      outside,
      escapedDirectory,
      process.platform === "win32" ? "junction" : "dir",
    );

    for (const unsafePath of [
      path.relative(root, outsideFile).split(path.sep).join("/"),
      outsideFile,
      "escaped-context/secret.md",
      "docs/../docs/safe.md",
      "C:\\outside\\secret.md",
      "\\\\server\\share\\secret.md",
    ]) {
      writeFixture(
        root,
        `${taskDir}/implement.jsonl`,
        `${JSON.stringify({ file: unsafePath, reason: "unsafe fixture" })}\n`,
      );
      const escaped = runContext();
      expect(escaped.status).not.toBe(0);
      expect(escaped.stdout).not.toContain("must-not-enter-hook-output");
      expect(escaped.stderr).not.toContain("must-not-enter-hook-output");
      expect(escaped.stderr).toMatch(/repository|parent traversal/);
    }

    const activeTaskHarness = [
      "import importlib.util",
      `spec = importlib.util.spec_from_file_location("fyagent_subagent_task_hook", ${JSON.stringify(script)})`,
      "module = importlib.util.module_from_spec(spec)",
      "spec.loader.exec_module(module)",
      `module.find_repo_root = lambda _candidate: ${JSON.stringify(root)}`,
      'module.get_current_task = lambda *_args, **_kwargs: "escaped-context"',
      `module._handle_codex_subagent_start({"hook_event_name": "SubagentStart", "agent_type": "trellis-implement", "session_id": "parent", "cwd": ${JSON.stringify(root)}})`,
    ].join("\n");
    const escapedTask = runManagedPython(activeTaskHarness, {
      FYAGENT_CODEX_HOOK_STRICT: "1",
    });
    expect(escapedTask.status).not.toBe(0);
    expect(escapedTask.stderr).toMatch(/repository/);
  });
});

describe("Codex hook runner contract", () => {
  it("binds the exact locked uv invocation to reviewed repository paths and a sanitized environment", () => {
    const root = createHookFixture({ ready: true });
    const before = runner.snapshotTree(root);
    const calls: Array<{
      command: string;
      args: string[];
      options: Record<string, unknown>;
    }> = [];
    const spawn: SpawnStub = (command, args, options) => {
      calls.push({ command, args, options });
      return {
        status: 0,
        signal: null,
        stderr: "",
        stdout: validOutput("UserPromptSubmit"),
      };
    };

    const output = runner.executeHook({
      projectRoot: root,
      mode: "workflow-state",
      input: workflowInput(root),
      spawn,
      environment: {
        __PYVENV_LAUNCHER__: "/outside/launcher",
        CODEX_THREAD_ID: "parent-thread",
        CONDA_PYTHON_EXE: "/outside/conda-python",
        DYLD_INSERT_LIBRARIES: "/outside/dylib",
        LD_PRELOAD: "/outside/preload.so",
        LIBPATH: "/outside/libpath",
        PYTHONHOME: "/outside/python-home",
        PYTHONPATH: "/outside/python-path",
        UV_CONFIG_FILE: "/outside/uv.toml",
        UV_PROJECT: "/outside/project",
        UV_PYTHON: "/outside/python",
        UV_WORKING_DIR: "/outside/workdir",
        VIRTUAL_ENV: "/outside/venv",
      },
    });

    expect(output.hookSpecificOutput).toMatchObject({
      hookEventName: "UserPromptSubmit",
      additionalContext: "contract context",
    });
    expect(calls).toHaveLength(1);
    expect(calls[0].command).toBe("uv");
    expect(calls[0].args).toEqual([
      "run",
      "--locked",
      "--no-sync",
      "--offline",
      "--no-env-file",
      "--project",
      root,
      "--directory",
      root,
      "--python",
      path.join(
        root,
        ".venv",
        process.platform === "win32" ? "Scripts/python.exe" : "bin/python",
      ),
      "python",
      "-I",
      "-S",
      "-B",
      "-X",
      expect.stringMatching(/^pycache_prefix=.*fyagent-codex-hook-[0-9a-f-]+$/),
      "-X",
      "utf8",
      path.join(root, ".codex", "hooks", "inject-workflow-state.py"),
    ]);
    expect(calls[0].options).toMatchObject({
      cwd: root,
      timeout: 12_000,
    });
    expect(calls[0].options.env).toMatchObject({
      FYAGENT_CODEX_HOOK_STRICT: "1",
      PYTHONDONTWRITEBYTECODE: "1",
      PYTHONNOUSERSITE: "1",
      PYTHONSAFEPATH: "1",
      UV_LOCKED: "1",
      UV_NO_ENV_FILE: "1",
      UV_NO_SYNC: "1",
      UV_OFFLINE: "1",
      UV_PROJECT: root,
      UV_PYTHON_DOWNLOADS: "never",
      UV_WORKING_DIR: root,
    });
    const childEnvironment = calls[0].options.env as NodeJS.ProcessEnv;
    expect(childEnvironment.CODEX_THREAD_ID).toBe("parent-thread");
    for (const removedKey of [
      "__PYVENV_LAUNCHER__",
      "CONDA_PYTHON_EXE",
      "DYLD_INSERT_LIBRARIES",
      "LD_PRELOAD",
      "LIBPATH",
      "PYTHONHOME",
      "PYTHONPATH",
      "UV_CONFIG_FILE",
      "VIRTUAL_ENV",
    ]) {
      expect(childEnvironment[removedKey]).toBeUndefined();
    }
    expect(runner.snapshotTree(root)).toEqual(before);
  });

  it("continues visibly when the valid project has not prepared .venv", () => {
    const root = createHookFixture();
    const before = runner.snapshotTree(root);
    const output = runner.executeHook({
      projectRoot: root,
      mode: "workflow-state",
      input: workflowInput(root),
      spawn: () => {
        throw new Error("uv must not run before bootstrap");
      },
    });

    expect(output.continue).toBe(true);
    expect(output.systemMessage).toMatch(/mise run bootstrap/);
    expect(output.hookSpecificOutput).toMatchObject({
      hookEventName: "UserPromptSubmit",
    });
    expect(runner.snapshotTree(root)).toEqual(before);
  });

  it("rejects filesystem-root, cross-drive, and symlink-escaped cwd values plus a symlinked project .venv", () => {
    expect(
      runner.isPathInsideProject(
        "C:\\projects\\fyagent",
        "C:\\projects\\fyagent\\src",
        path.win32,
      ),
    ).toBe(true);
    expect(
      runner.isPathInsideProject(
        "C:\\projects\\fyagent",
        "D:\\outside\\workspace",
        path.win32,
      ),
    ).toBe(false);

    const root = createHookFixture();
    expect(() =>
      runner.executeHook({
        projectRoot: root,
        mode: "workflow-state",
        input: { ...workflowInput(root), cwd: path.parse(root).root },
      }),
    ).toThrow(/cwd must remain inside/);

    const outside = fs.mkdtempSync(
      path.join(os.tmpdir(), "fyagent-hooks-outside-test-"),
    );
    fixtureRoots.push(outside);
    const escapedCwd = path.join(root, "escaped-cwd");
    fs.symlinkSync(
      outside,
      escapedCwd,
      process.platform === "win32" ? "junction" : "dir",
    );
    expect(() =>
      runner.executeHook({
        projectRoot: root,
        mode: "workflow-state",
        input: { ...workflowInput(root), cwd: escapedCwd },
      }),
    ).toThrow(/cwd must remain inside/);

    const linkedVenv = path.join(root, "linked-venv-target");
    fs.mkdirSync(linkedVenv, { recursive: true });
    fs.symlinkSync(
      linkedVenv,
      path.join(root, ".venv"),
      process.platform === "win32" ? "junction" : "dir",
    );

    expect(() => runner.validateProject(root, "workflow-state")).toThrow(
      /repository-local directory, not a symlink/,
    );
  });

  it("fails closed for damaged project files and hook scripts before degradation", () => {
    const damagedLock = createHookFixture();
    writeFixture(damagedLock, "uv.lock", "version = 1\nrevision = 3\n");
    expect(() => runner.validateProject(damagedLock, "workflow-state")).toThrow(
      /requires-python/,
    );

    const damagedHook = createHookFixture();
    fs.appendFileSync(
      path.join(damagedHook, ".codex/hooks/inject-workflow-state.py"),
      "# damaged\n",
    );
    expect(() => runner.validateProject(damagedHook, "workflow-state")).toThrow(
      /integrity check failed/,
    );

    const damagedDependency = createHookFixture();
    fs.appendFileSync(
      path.join(damagedDependency, ".trellis/scripts/common/active_task.py"),
      "# damaged\n",
    );
    expect(() =>
      runner.validateProject(damagedDependency, "workflow-state"),
    ).toThrow(/active_task\.py integrity check failed/);
  });

  it("binds required TOML keys to their approved tables and values", () => {
    const wrongPyprojectTable = createHookFixture();
    const wrongPyproject = fs
      .readFileSync(path.join(wrongPyprojectTable, "pyproject.toml"), "utf8")
      .replace("[tool.uv]", "[tool.uv]\n\n[tool.fake]");
    writeFixture(wrongPyprojectTable, "pyproject.toml", wrongPyproject);
    expect(() =>
      runner.validateProject(wrongPyprojectTable, "workflow-state"),
    ).toThrow(/package exactly once in \[tool\.uv\]/);

    const duplicatePyprojectKey = createHookFixture();
    fs.appendFileSync(
      path.join(duplicatePyprojectKey, "pyproject.toml"),
      "package = false\n",
    );
    expect(() =>
      runner.validateProject(duplicatePyprojectKey, "workflow-state"),
    ).toThrow(/package exactly once in \[tool\.uv\].*found 2/);

    const invalidPyprojectValue = createHookFixture();
    const invalidPyproject = fs
      .readFileSync(path.join(invalidPyprojectValue, "pyproject.toml"), "utf8")
      .replace('python-downloads = "automatic"', 'python-downloads = "never"');
    writeFixture(invalidPyprojectValue, "pyproject.toml", invalidPyproject);
    expect(() =>
      runner.validateProject(invalidPyprojectValue, "workflow-state"),
    ).toThrow(/python-downloads must be "automatic"/);

    const wrongLockTable = createHookFixture();
    const wrongLock = fs
      .readFileSync(path.join(wrongLockTable, "uv.lock"), "utf8")
      .replace(
        'requires-python = "==3.14.*"',
        '[tool.fake]\nrequires-python = "==3.14.*"',
      );
    writeFixture(wrongLockTable, "uv.lock", wrongLock);
    expect(() =>
      runner.validateProject(wrongLockTable, "workflow-state"),
    ).toThrow(/requires-python exactly once in the top level/);

    const invalidLockValue = createHookFixture();
    const invalidLock = fs
      .readFileSync(path.join(invalidLockValue, "uv.lock"), "utf8")
      .replace("revision = 3", "revision = 4");
    writeFixture(invalidLockValue, "uv.lock", invalidLock);
    expect(() =>
      runner.validateProject(invalidLockValue, "workflow-state"),
    ).toThrow(/revision must be 3/);
  });

  it("keeps reviewed hook integrity stable across LF and CRLF checkouts", () => {
    const root = createHookFixture();
    for (const relativePath of reviewedPythonSources) {
      const sourcePath = path.join(root, relativePath);
      const source = fs.readFileSync(sourcePath, "utf8");
      fs.writeFileSync(sourcePath, source.replace(/\n/g, "\r\n"), "utf8");
    }

    expect(runner.validateProject(root, "workflow-state")).toMatchObject({
      ready: false,
    });
    expect(runner.validateProject(root, "subagent-context")).toMatchObject({
      ready: false,
    });
  });

  it("fails closed for malformed input, wrong events, nonzero hooks, and invalid stdout", () => {
    expect(() => runner.parseInput("{", "workflow-state")).toThrow(
      /valid JSON/,
    );
    expect(() =>
      runner.parseInput(
        JSON.stringify({ hook_event_name: "SubagentStart" }),
        "workflow-state",
      ),
    ).toThrow(/requires event UserPromptSubmit/);
    expect(() =>
      runner.parseInput(
        JSON.stringify({
          hook_event_name: "SubagentStart",
          agent_type: "trellis-check",
        }),
        "subagent-context",
      ),
    ).toThrow(/session_id/);

    const root = createHookFixture({ ready: true });
    expect(() =>
      runner.executeHook({
        projectRoot: root,
        mode: "workflow-state",
        input: workflowInput(root),
        spawn: () => ({
          status: 2,
          signal: null,
          stderr: "broken hook",
          stdout: "",
        }),
      }),
    ).toThrow(/exited 2/);
    expect(() =>
      runner.executeHook({
        projectRoot: root,
        mode: "workflow-state",
        input: workflowInput(root),
        spawn: () => ({
          status: 0,
          signal: null,
          stderr: "",
          stdout: "not-json",
        }),
      }),
    ).toThrow(/exactly one JSON object/);
  });

  it("allows only explicit Trellis disablement to be silent", () => {
    expect(runner.explicitDisable({ TRELLIS_HOOKS: "0" })).toBe(true);
    expect(runner.explicitDisable({ TRELLIS_DISABLE_HOOKS: "1" })).toBe(true);
    expect(runner.explicitDisable({})).toBe(false);

    const disabled = spawnSync(
      process.execPath,
      [runnerPath, "workflow-state"],
      {
        cwd: repositoryRoot,
        encoding: "utf8",
        env: { ...process.env, TRELLIS_HOOKS: "0" },
        input: "not-json",
      },
    );
    expect(disabled.status, disabled.stderr).toBe(0);
    expect(disabled.stdout).toBe("");

    const invalid = spawnSync(
      process.execPath,
      [runnerPath, "workflow-state"],
      {
        cwd: repositoryRoot,
        encoding: "utf8",
        env: {
          ...process.env,
          TRELLIS_HOOKS: "1",
          TRELLIS_DISABLE_HOOKS: "0",
        },
        input: "not-json",
      },
    );
    expect(invalid.status).toBe(1);
    expect(invalid.stderr).toMatch(/stdin must be valid JSON/);
  });

  it("keeps Codex Python protocol errors strict without changing generic fallbacks", () => {
    const workflowSource = fs.readFileSync(
      path.join(repositoryRoot, ".codex/hooks/inject-workflow-state.py"),
      "utf8",
    );
    const subagentSource = fs.readFileSync(
      path.join(repositoryRoot, ".codex/hooks/inject-subagent-context.py"),
      "utf8",
    );
    expect(workflowSource).toMatch(
      /FYAGENT_CODEX_HOOK_STRICT[\s\S]*raise TimeoutError/,
    );
    expect(subagentSource).toMatch(
      /print\(json\.dumps\(\{"continue": True\}\)\)/,
    );

    const script = path.join(
      repositoryRoot,
      ".codex",
      "hooks",
      "inject-subagent-context.py",
    );
    const input = JSON.stringify({
      hook_event_name: "SubagentStart",
      agent_type: "trellis-check",
      session_id: "failure-policy-test",
      cwd: repositoryRoot,
    });
    const harness = [
      "import importlib.util, io, sys",
      `spec = importlib.util.spec_from_file_location("fyagent_hook", ${JSON.stringify(script)})`,
      "module = importlib.util.module_from_spec(spec)",
      "spec.loader.exec_module(module)",
      'module._handle_codex_subagent_start = lambda _input: (_ for _ in ()).throw(RuntimeError("injected failure"))',
      `sys.stdin = io.StringIO(${JSON.stringify(input)})`,
      "module.main()",
    ].join("\n");

    const noContextHarness = harness.replace(
      'module._handle_codex_subagent_start = lambda _input: (_ for _ in ()).throw(RuntimeError("injected failure"))',
      "module._handle_codex_subagent_start = lambda _input: False",
    );
    const noContext = runManagedPython(noContextHarness, {
      FYAGENT_CODEX_HOOK_STRICT: "1",
    });
    expect(noContext.status, noContext.stderr).toBe(0);
    expect(JSON.parse(noContext.stdout)).toEqual({ continue: true });

    const generic = runManagedPython(harness, {
      FYAGENT_CODEX_HOOK_STRICT: "0",
    });
    expect(generic.status, generic.stderr).toBe(0);
    expect(JSON.parse(generic.stdout)).toEqual({ continue: true });

    const strict = runManagedPython(harness, {
      FYAGENT_CODEX_HOOK_STRICT: "1",
    });
    expect(strict.status).not.toBe(0);
    expect(strict.stderr).toContain("injected failure");
  });
});

describe("hook command side-effect boundary", () => {
  it("contains no sync, install, trust, or warning-suppression escape hatch", () => {
    const source = fs.readFileSync(runnerPath, "utf8");
    expect(source).toContain('"--locked"');
    expect(source).toContain('"--no-sync"');
    expect(source).toContain('"--offline"');
    expect(source).not.toMatch(/\buv\s+sync\b/);
    expect(source).not.toMatch(/\bpip\s+install\b/);
    expect(source).not.toMatch(/\bmise\s+trust\b/);
    expect(source).not.toMatch(/NODE_NO_WARNINGS|--no-warnings/);
  });
});
