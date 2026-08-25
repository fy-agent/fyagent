import fs from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";

const ROOT = path.resolve(__dirname, "..");
const source = fs
  .readFileSync(path.join(ROOT, ".github", "workflows", "ci.yml"), "utf8")
  .replace(/\r\n/g, "\n");

const REQUIRED_JOBS = [
  "contracts",
  "frontend",
  "desktop-acceptance-contract",
  "backend-windows",
  "windows-native-contracts",
  "backend-macos",
] as const;

const DEPENDENCY_JOBS = [
  "commit-convention",
  "changes",
  ...REQUIRED_JOBS,
] as const;

const LOCAL_MISE_TESTS = [
  "tests/developmentEnvironment.test.ts",
  "tests/miseTaskContract.test.ts",
  "tests/systemCheck.test.ts",
  "tests/taskDocs.test.ts",
] as const;

const ACTION_PINS = new Map([
  ["actions/checkout", ["3d3c42e5aac5ba805825da76410c181273ba90b1", "v7.0.1"]],
  [
    "actions/setup-node",
    ["820762786026740c76f36085b0efc47a31fe5020", "v7.0.0"],
  ],
  [
    "pnpm/action-setup",
    ["0977fd99725f1db4007ccb2928dbb4e90d06cc86", "v6.0.10"],
  ],
  [
    "actions-rust-lang/setup-rust-toolchain",
    ["166cdcfd11aee3cb47222f9ddb555ce30ddb9659", "v1.17.0"],
  ],
  [
    "astral-sh/setup-uv",
    ["c771a70e6277c0a99b617c7a806ffedaca235ff9", "v9.0.0"],
  ],
  ["actions/cache", ["55cc8345863c7cc4c66a329aec7e433d2d1c52a9", "v6.1.0"]],
]);

function jobBlock(id: string): string {
  const jobsStart = source.indexOf("\njobs:\n");
  const header = `\n  ${id}:\n`;
  const start = source.indexOf(header, jobsStart);
  expect(start, id).toBeGreaterThan(-1);
  const bodyStart = start + header.length;
  const next = source.slice(bodyStart).search(/^  [a-z][a-z0-9-]*:\s*$/m);
  if (next < 0) return source.slice(start + 1);
  return source.slice(start + 1, bodyStart + next);
}

function namedStepBlock(jobId: string, stepName: string): string {
  const job = jobBlock(jobId);
  const marker = `      - name: ${stepName}\n`;
  const start = job.indexOf(marker);
  expect(start, `${jobId}/${stepName}`).toBeGreaterThan(-1);
  const bodyStart = start + marker.length;
  const next = job.slice(bodyStart).search(/^      - name:/m);
  if (next < 0) return job.slice(start);
  return job.slice(start, bodyStart + next);
}

function actionSteps(action: string): string[] {
  const escaped = action.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const expression = new RegExp(
    `^      - name: [^\\n]+\\n(?:        [^\\n]+\\n)*?        uses: ${escaped}@[a-f0-9]{40}[^\\n]*\\n(?:        [^\\n]+\\n|          [^\\n]+\\n)*`,
    "gm",
  );
  return source.match(expression) ?? [];
}

describe("automatic CI workflow", () => {
  it("routes six conditional domain jobs through one always-present gate", () => {
    const jobsSection = source.slice(source.indexOf("\njobs:\n"));
    const jobIds = [...jobsSection.matchAll(/^  ([a-z][a-z0-9-]*):\s*$/gm)].map(
      (match) => match[1],
    );
    expect(jobIds).toEqual([
      "commit-convention",
      "changes",
      ...REQUIRED_JOBS,
      "required",
    ]);

    const commitConvention = jobBlock("commit-convention");
    expect(commitConvention).toContain("name: Commit Convention");
    expect(commitConvention).toContain("timeout-minutes: 5");
    expect(commitConvention).toContain("fetch-depth: 0");
    expect(commitConvention).toContain(
      "node scripts/ci/verify-commit-messages.mjs",
    );
    expect(commitConvention).toContain("PR_TITLE:");
    expect(commitConvention).not.toContain("pnpm install");

    const changes = jobBlock("changes");
    expect(changes).toContain("needs: commit-convention");
    expect(changes).toContain("name: Classify Changes");
    expect(changes).toContain("fetch-depth: 0");
    expect(changes).toContain('--summary-file "$GITHUB_STEP_SUMMARY"');
    expect(changes).toContain(
      'node scripts/ci/classify-changes.mjs \\\n            --base "$base_sha" --head "$head_sha" --json',
    );
    expect(changes).toContain("PR_BASE_SHA:");
    expect(changes).toContain("MERGE_GROUP_BASE_SHA:");
    expect(changes).not.toContain("PUSH_BASE_SHA:");
    expect(changes).toContain("CURRENT_SHA: ${{ github.sha }}");
    expect(changes).toContain("event_force_full=true");
    expect(changes).toContain(".domains |= with_entries(.value = true)");
    expect(changes).toContain("### Final CI plan");
    expect(changes).toContain("Event-forced Full CI");
    expect(changes).toContain(
      "workflow_dispatch` is the explicit Full CI diagnostic path",
    );
    expect(changes).not.toContain('if [ "$EVENT_NAME" = push ] &&');
    expect(changes).not.toContain(
      "Push before SHA is not a commit in this clone",
    );
    expect(changes).toContain("workflow_dispatch)");
    expect(changes).not.toContain("push)");

    expect(jobBlock("contracts")).toContain(
      "if: ${{ !cancelled() && (needs.changes.result == 'failure' || (needs.changes.result == 'success' && (needs.changes.outputs.contracts == 'true' || needs.changes.outputs.docs_spec == 'true'))) }}",
    );
    expect(jobBlock("frontend")).toContain(
      "if: ${{ !cancelled() && (needs.changes.result == 'failure' || (needs.changes.result == 'success' && needs.changes.outputs.frontend == 'true')) }}",
    );
    expect(jobBlock("desktop-acceptance-contract")).toContain(
      "if: ${{ !cancelled() && (needs.changes.result == 'failure' || (needs.changes.result == 'success' && needs.changes.outputs.desktop == 'true')) }}",
    );
    expect(jobBlock("backend-windows")).toContain(
      "if: ${{ !cancelled() && (needs.changes.result == 'failure' || (needs.changes.result == 'success' && (needs.changes.outputs.backend == 'true' || needs.changes.outputs.windows_native == 'true'))) }}",
    );
    expect(jobBlock("windows-native-contracts")).toContain(
      "if: ${{ !cancelled() && (needs.changes.result == 'failure' || (needs.changes.result == 'success' && needs.changes.outputs.windows_native == 'true')) }}",
    );
    expect(jobBlock("backend-macos")).toContain(
      "if: ${{ !cancelled() && (needs.changes.result == 'failure' || (needs.changes.result == 'success' && needs.changes.outputs.backend == 'true')) }}",
    );

    const required = jobBlock("required");
    expect(required).toContain("name: CI / Required");
    expect(required).toContain("if: always()");
    expect(required).toContain("runs-on: macos-15");
    expect(required).toContain("REQUIRED_RESULTS: ${{ toJSON(needs) }}");
    expect(required).toContain("node scripts/ci/required-gate.mjs");
    expect(required).toContain("actions: read");
    expect(required).toContain(
      "actions/runs/$GITHUB_RUN_ID/attempts/$GITHUB_RUN_ATTEMPT/jobs",
    );
    expect(required).toContain(
      "CI_CLASSIFICATION_PLAN: ${{ needs.changes.outputs.plan }}",
    );
    expect(required).toContain("--jobs-json");
    for (const id of DEPENDENCY_JOBS) {
      expect(required).toContain(`      - ${id}`);
    }
  });

  it("uses only explicit approved runners and read-only workflow permissions", () => {
    expect(source).toContain("permissions:\n  contents: read");
    expect(source).not.toMatch(/^\s+[a-z][a-z-]*:\s+write\s*$/m);
    expect(source).not.toContain("secrets.");
    expect(source).not.toMatch(/runs-on:\s*[^\n]*-latest/);
    expect(source).not.toMatch(/(?:^|\s)paths(?:-ignore)?:/m);

    expect(jobBlock("contracts")).toContain("runs-on: macos-15");
    expect(jobBlock("frontend")).toContain("runs-on: macos-15");
    expect(jobBlock("desktop-acceptance-contract")).toContain(
      "runs-on: macos-15",
    );
    expect(jobBlock("changes")).toContain("runs-on: macos-15");
    expect(jobBlock("commit-convention")).toContain("runs-on: macos-15");
    expect(jobBlock("backend-windows")).toContain("runs-on: windows-2025");
    expect(jobBlock("windows-native-contracts")).toContain(
      "runs-on: ${{ matrix.runner }}",
    );
    expect(jobBlock("backend-macos")).toContain("runs-on: macos-15");
    expect(jobBlock("required")).toContain("runs-on: macos-15");
    expect(source).not.toContain(["ubu", "ntu-"].join(""));
  });

  it("pins every third-party Action to an reviewed full commit", () => {
    const uses = [...source.matchAll(/^\s+uses:\s+(.+)$/gm)].map(
      (match) => match[1],
    );
    expect(uses.length).toBeGreaterThan(0);
    const parsedUses = uses.map((reference) => {
      const parsed = /^([^@\s]+)@([a-f0-9]{40})\s+#\s+(\S+)$/.exec(reference);
      expect(parsed, reference).not.toBeNull();
      return parsed!;
    });
    for (const [, action, sha, version] of parsedUses) {
      expect(ACTION_PINS.has(action), action).toBe(true);
      expect([sha, version], action).toEqual(ACTION_PINS.get(action));
      expect(sha).toMatch(/^[a-f0-9]{40}$/);
    }
    for (const action of ACTION_PINS.keys()) {
      expect(
        parsedUses.some((match) => match[1] === action),
        action,
      ).toBe(true);
    }

    const checkoutSteps = actionSteps("actions/checkout");
    expect(checkoutSteps).toHaveLength(9);
    for (const step of checkoutSteps) {
      expect(step).toContain("persist-credentials: false");
    }
  });

  it("consumes standard version files without mise or duplicate literals", () => {
    expect(source).not.toMatch(/\bmise\s+(?:run|exec|install|trust)\b/);
    expect(source).not.toContain("24.19.0");
    expect(source).not.toContain("10.12.3");
    expect(source).not.toContain("1.97.1");
    expect(source).not.toContain("0.12.2");
    expect(source).not.toContain("3.14.7");

    const nodeSteps = actionSteps("actions/setup-node");
    expect(nodeSteps).toHaveLength(8);
    for (const step of nodeSteps) {
      expect(step).toContain("node-version-file: .node-version");
      expect(step).not.toMatch(/^\s+node-version:/m);
    }

    const pnpmSteps = actionSteps("pnpm/action-setup");
    expect(pnpmSteps).toHaveLength(5);
    for (const step of pnpmSteps) {
      expect(step).toContain("run_install: false");
      expect(step).not.toMatch(/^\s+version:/m);
    }

    const rustSteps = actionSteps("actions-rust-lang/setup-rust-toolchain");
    expect(rustSteps).toHaveLength(4);
    for (const step of rustSteps) {
      expect(step).toContain("cache: false");
      expect(step).toContain('rustflags: ""');
      expect(step).not.toMatch(/^\s+(?:toolchain|components?|targets?):/m);
    }

    const cargoCacheJobs = [
      "backend-windows",
      "windows-native-contracts",
      "backend-macos",
    ] as const;
    expect(source.match(/uses: actions\/cache@/g)).toHaveLength(
      cargoCacheJobs.length,
    );
    for (const jobId of cargoCacheJobs) {
      const step = namedStepBlock(jobId, "Cache Cargo registry");
      expect(step).toContain(
        "uses: actions/cache@55cc8345863c7cc4c66a329aec7e433d2d1c52a9",
      );
      expect(step).toContain("~/.cargo/registry");
      expect(step).toContain("~/.cargo/git");
      expect(step).toContain("hashFiles('src-tauri/Cargo.lock')");
      expect(step).not.toContain("src-tauri/target");
    }
    expect(source).not.toContain("RUSTC_WRAPPER");
    expect(source).not.toContain("sccache");

    const uvSteps = actionSteps("astral-sh/setup-uv");
    expect(uvSteps).toHaveLength(3);
    for (const step of uvSteps) {
      expect(step).toContain(
        "version: ${{ steps.toolchain-facts.outputs.uv-version }}",
      );
      expect(step).toContain("enable-cache: false");
    }
    expect(
      uvSteps.filter((step) =>
        step.includes(
          "python-version: ${{ steps.toolchain-facts.outputs.python-version }}",
        ),
      ),
    ).toHaveLength(2);
    expect(uvSteps).toContainEqual(
      expect.stringContaining(
        "python-version: cpython-${{ steps.toolchain-facts.outputs.python-version }}-windows-${{ matrix.python_architecture }}-none",
      ),
    );
  });

  it("prepares the locked Python environment before the full unit suite", () => {
    const contracts = jobBlock("contracts");
    const frontend = jobBlock("frontend");
    for (const block of [contracts, frontend]) {
      expect(block).toContain(
        "run: node scripts/ci/verify-toolchain.mjs --emit-github-output",
      );
      expect(block).toContain("run: uv sync --locked");
      expect(block.indexOf("run: uv sync --locked")).toBeLessThan(
        block.indexOf("node scripts/tasks/release-check.mjs --ci") >= 0
          ? block.indexOf("node scripts/tasks/release-check.mjs --ci")
          : block.indexOf("pnpm test:unit"),
      );
    }
    expect(frontend).toContain("pnpm test:unit");
    expect(frontend).not.toContain("NODE_OPTIONS:");
  });

  it("keeps GitHub Actions independent from the local mise runtime", () => {
    const contracts = jobBlock("contracts");
    const frontend = jobBlock("frontend");
    const releaseCheck = fs.readFileSync(
      path.join(ROOT, "scripts", "tasks", "release-check.mjs"),
      "utf8",
    );

    expect(contracts).not.toContain("scripts/trellis/");
    expect(contracts).toContain("node scripts/tasks/release-check.mjs --ci");
    expect(frontend).toContain("pnpm test:unit");
    expect(frontend).not.toContain("run: pnpm test:unit");
    for (const test of LOCAL_MISE_TESTS) {
      expect(frontend, test).toContain(`--exclude ${test}`);
      expect(releaseCheck, test).toContain(`\"${test}\"`);
    }
    expect(frontend).not.toContain("tests/developmentHooks.test.ts");
    expect(releaseCheck).toContain("if (!ciMode)");
    expect(releaseCheck).toContain('"tests/formatFiles.test.ts"');
    expect(releaseCheck).toContain('"tests/taskAtomicWriter.test.ts"');
    expect(releaseCheck).toContain(
      '["scripts/tasks/task-contract-check.mjs"],',
    );
    expect(releaseCheck).toContain("for (const [id, command, args] of plan)");
    expect(releaseCheck).toContain("throw new AggregateError(");
  });

  it("runs locked Rust checks on Windows and macOS", () => {
    for (const id of ["backend-windows", "backend-macos"]) {
      const block = jobBlock(id);
      expect(block).toContain(
        "node scripts/ci/verify-toolchain.mjs --tools node,pnpm,rust",
      );
      expect(block).toContain(
        "cargo check --workspace --all-targets --keep-going --locked --manifest-path src-tauri/Cargo.toml",
      );
      expect(block).toContain(
        "cargo clippy --workspace --all-targets --keep-going --locked --manifest-path src-tauri/Cargo.toml -- -D warnings",
      );
      expect(block).toContain(
        "cargo test --workspace --features fyagent/test-hooks --locked --manifest-path src-tauri/Cargo.toml --no-fail-fast",
      );
      expect(namedStepBlock(id, "Run Rust tests")).toContain(
        'RUST_TEST_THREADS: "1"',
      );
      expect(namedStepBlock(id, "Check Rust workspace")).not.toContain(
        "test-hooks",
      );
      expect(namedStepBlock(id, "Run Clippy")).not.toContain("test-hooks");
    }
    expect(jobBlock("backend-windows")).toContain(
      "FYAGENT_WINDOWS_MANIFEST: test",
    );
    const windows = jobBlock("backend-windows");
    expect(windows).toContain(`- name: Prepare Windows user helper sidecar
        id: prepare-helper
        if: \${{ !cancelled() && steps.checkout.outcome == 'success' }}
        env:
          TAURI_ENV_TARGET_TRIPLE: x86_64-pc-windows-msvc
          TAURI_ENV_DEBUG: "true"
        run: node scripts/prepare-windows-user-helper.mjs`);
    expect(windows.indexOf("Prepare Windows user helper sidecar")).toBeLessThan(
      windows.indexOf("Check Rust workspace"),
    );
    const firstWindowsCargo = windows.search(/^        run: cargo\s/mu);
    expect(firstWindowsCargo).toBeGreaterThan(-1);
    expect(windows.indexOf("Prepare Windows user helper sidecar")).toBeLessThan(
      firstWindowsCargo,
    );
    expect(windows).not.toContain("pnpm install");
    const secretRefHil = namedStepBlock(
      "backend-windows",
      "Exercise Credential Manager SecretRef CRUD",
    );
    expect(secretRefHil).toContain('FYAGENT_NATIVE_SECRET_TEST: "1"');
    expect(secretRefHil).toContain(
      "--test secret_service_contract native_os_backend_crud_readback -- --ignored --exact --nocapture --test-threads=1",
    );
    expect(secretRefHil).toContain(
      'if ($exitCode -ne 0 -or $joined -notmatch "test result: ok\\. 1 passed; 0 failed")',
    );
    expect(
      source.match(
        /^        run: node scripts\/prepare-windows-user-helper\.mjs$/gm,
      ),
    ).toHaveLength(2);
    expect(jobBlock("backend-macos")).toContain(
      "cargo fmt --all --check --manifest-path src-tauri/Cargo.toml",
    );
  });

  it("collects every CI diagnostic before each job restores its failure", () => {
    const collectedSteps = {
      "commit-convention": ["Setup Node.js", "Verify commit messages"],
      changes: [
        "Setup Node.js",
        "Classify explicit base and head commits",
        "Verify supported platform surface",
      ],
      contracts: [
        "Setup pnpm",
        "Setup Node.js",
        "Resolve locked toolchain facts",
        "Setup uv and managed Python",
        "Setup Rust",
        "Synchronize locked Python environment",
        "Install frontend dependencies",
        "Verify active toolchains",
        "Run repository and release contracts",
      ],
      frontend: [
        "Setup pnpm",
        "Setup Node.js",
        "Resolve locked toolchain facts",
        "Setup uv and managed Python",
        "Synchronize locked Python environment",
        "Install frontend dependencies",
        "Verify active toolchains",
        "Verify application version contract",
        "TypeScript type check",
        "Check formatting",
        "Unit tests",
        "Locale key parity",
      ],
      "desktop-acceptance-contract": [
        "Setup pnpm",
        "Setup Node.js",
        "Install frontend dependencies",
        "Verify active toolchains",
        "Run mock-only desktop acceptance tests",
        "Verify mock-only desktop acceptance contract",
        "Verify native Fetch through MSW and Tauri mock",
        "Verify visual baseline capture policy",
      ],
      "backend-windows": [
        "Setup pnpm",
        "Setup Node.js",
        "Setup Rust",
        "Cache Cargo registry",
        "Verify active toolchains",
        "Create frontend dist placeholder",
        "Prepare Windows user helper sidecar",
        "Check Rust workspace",
        "Run Clippy",
        "Run Rust tests",
        "Exercise Credential Manager SecretRef CRUD",
      ],
      "windows-native-contracts": [
        "Setup Node.js",
        "Setup Rust",
        "Cache Cargo registry",
        "Resolve locked toolchain facts",
        "Setup uv and managed Python",
        "Synchronize locked Python environment",
        "Verify managed Windows toolchains",
        "Create frontend dist placeholder",
        "Verify native Rust architecture",
        "Prepare Windows user helper sidecar",
        "Exercise explicit-SID Main package inventory",
        "Verify managed Python architecture",
      ],
      "backend-macos": [
        "Setup pnpm",
        "Setup Node.js",
        "Setup Rust",
        "Cache Cargo registry",
        "Verify active toolchains",
        "Create frontend dist placeholder",
        "Check Rust formatting",
        "Check Rust workspace",
        "Run Clippy",
        "Run Rust tests",
      ],
      required: [
        "Read current run-attempt job conclusions",
        "Evaluate required dependency results",
      ],
    } as const;

    for (const [jobId, stepNames] of Object.entries(collectedSteps)) {
      const actualNames = [
        ...jobBlock(jobId).matchAll(/^      - name: (.+)$/gm),
      ].map((match) => match[1]);
      expect(actualNames[0], jobId).toMatch(/^Checkout(?: |$)/u);
      expect(actualNames.slice(1), jobId).toEqual([
        ...stepNames,
        "Evaluate collected diagnostics",
      ]);
      const checkout = namedStepBlock(jobId, actualNames[0]);
      expect(checkout).toContain("id: checkout");
      expect(checkout).not.toContain("continue-on-error: true");

      const stepIds = stepNames.map((stepName) => {
        const step = namedStepBlock(jobId, stepName);
        expect(step).toContain(
          "if: ${{ !cancelled() && steps.checkout.outcome == 'success' }}",
        );
        expect(step).not.toContain("continue-on-error: true");
        const id = /^        id: ([a-z][a-z0-9-]*)$/m.exec(step)?.[1];
        expect(id, `${jobId}/${stepName}`).toBeDefined();
        return id!;
      });

      const evaluator = namedStepBlock(jobId, "Evaluate collected diagnostics");
      expect(evaluator).toContain(
        "if: ${{ !cancelled() && steps.checkout.outcome == 'success' }}",
      );
      expect(evaluator).not.toContain("continue-on-error: true");
      expect(evaluator).toContain(`CI_REQUIRED_STEPS: ${stepIds.join(",")}`);
      expect(evaluator).toContain("CI_STEP_RESULTS: ${{ toJSON(steps) }}");
      expect(evaluator).toContain(
        "run: node scripts/ci/evaluate-step-outcomes.mjs",
      );
    }
  });

  it("runs the current-tree platform surface gate in every CI plan", () => {
    const changes = jobBlock("changes");
    const platformStep = namedStepBlock(
      "changes",
      "Verify supported platform surface",
    );

    expect(platformStep).toContain(
      "run: node scripts/tasks/supported-platform-check.mjs",
    );
    expect(platformStep).not.toContain("exclude-active-task");
    expect(changes.indexOf("Verify supported platform surface")).toBeLessThan(
      changes.indexOf("Evaluate collected diagnostics"),
    );
  });

  it("runs managed Python and the explicit-SID package smoke on native Windows x64 and ARM64", () => {
    const block = jobBlock("windows-native-contracts");
    expect(block).toContain(
      "name: Windows Native Contracts (${{ matrix.architecture }})",
    );
    expect(block).toContain("timeout-minutes: 45");
    expect(block).toContain("fail-fast: false");
    expect(block).toContain(`matrix:
        include:
          - runner: windows-2025
            architecture: X64
            rust_host: x86_64-pc-windows-msvc
            python_architecture: x86_64
            python_platform: win-amd64
          - runner: windows-11-arm
            architecture: ARM64
            rust_host: aarch64-pc-windows-msvc
            python_architecture: aarch64
            python_platform: win-arm64
    runs-on: \${{ matrix.runner }}`);
    expect(block).toContain('UV_MANAGED_PYTHON: "true"');
    expect(block).toContain("shell: pwsh");
    expect(block).toContain("node-version-file: .node-version");
    expect(block).toContain(
      "run: node scripts/ci/verify-toolchain.mjs --emit-github-output",
    );
    expect(block).toContain(
      "version: ${{ steps.toolchain-facts.outputs.uv-version }}",
    );
    expect(block).toContain(
      "python-version: cpython-${{ steps.toolchain-facts.outputs.python-version }}-windows-${{ matrix.python_architecture }}-none",
    );
    expect(block).toContain("enable-cache: false");
    expect(block).toContain("run: uv sync --locked --managed-python");
    expect(block).toContain(
      "run: node scripts/ci/verify-toolchain.mjs --tools node,uv,python,rust",
    );
    expect(block).not.toContain(".trellis/scripts/task.py");
    expect(block).toContain(
      'python -c "import sysconfig; print(sysconfig.get_platform())"',
    );
    expect(block).toContain(
      "$pythonPlatform -ne '${{ matrix.python_platform }}'",
    );
    expect(block).not.toContain("Trellis task listing");
    expect(block).toContain("actions-rust-lang/setup-rust-toolchain@");
    expect(block).toContain(
      "$env:RUNNER_ARCH -cne '${{ matrix.architecture }}'",
    );
    expect(block).toContain("$rustHost -cne '${{ matrix.rust_host }}'");
    expect(block).toContain("rustc -vV did not report exactly one host triple");
    expect(block).toContain(
      "run: New-Item -ItemType Directory -Force dist | Out-Null",
    );
    expect(block).toContain(`- name: Prepare Windows user helper sidecar
        id: prepare-helper
        if: \${{ !cancelled() && steps.checkout.outcome == 'success' }}
        env:
          TAURI_ENV_TARGET_TRIPLE: \${{ matrix.rust_host }}
          TAURI_ENV_DEBUG: "true"
        run: node scripts/prepare-windows-user-helper.mjs`);
    expect(block).toContain(
      "codex_desktop::platform::windows::deployment::tests::native_explicit_sid_main_query_smoke",
    );
    expect(block.indexOf("Verify native Rust architecture")).toBeLessThan(
      block.indexOf("Prepare Windows user helper sidecar"),
    );
    expect(block.indexOf("Prepare Windows user helper sidecar")).toBeLessThan(
      block.indexOf("Exercise explicit-SID Main package inventory"),
    );
    expect(block).toContain(
      "cargo test --target '${{ matrix.rust_host }}' --lib --locked --manifest-path src-tauri/Cargo.toml $testName -- --exact",
    );
    expect(block).not.toContain("test-hooks");
    expect(block).toContain(
      'if ($exitCode -ne 0 -or $joined -notmatch "test result: ok\\. 1 passed; 0 failed")',
    );
    expect(block).not.toContain("windowsInstallerQuery.integration.ps1");
    expect(block.match(/^      - name:/gm)).toHaveLength(14);
    expect(block.match(/^        uses:/gm)).toHaveLength(5);
    expect(block).not.toMatch(/\b(?:npm|npx|pnpm|yarn|bun)\b|bundle|signing/i);
  });

  it("runs the focused pending-deprecation native Fetch probe", () => {
    const desktop = jobBlock("desktop-acceptance-contract");
    expect(desktop).toContain("run: pnpm test:native-fetch");
    expect(source).not.toContain("NODE_NO_WARNINGS");
    expect(source).not.toContain("--no-warnings");
    expect(source).not.toContain("--no-deprecation");
    expect(source).not.toContain("--disable-warning=DEP0040");
  });
});
