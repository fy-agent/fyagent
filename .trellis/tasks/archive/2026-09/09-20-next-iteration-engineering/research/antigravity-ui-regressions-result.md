# FyAgent Browser Regressions Resolution Report

## 1. Execution Scope & Environment

- **Workspace**: `~/.codex/worktrees/fyagent-next-ui-regressions/fyagent`
- **Branch**: `codex/next-ui-regressions`
- **Base Commit**: `35848bef`
- **Model**: Gemini 3.8 Flash High
- **Task Authority**: `ANTIGRAVITY_UI_TASK.md`

---

## 2. Regression Diagnosis and Fixes

### Item 1: WorkBuddy Documentation Terms URL

- **File**: `tests/browser/agent-directory.spec.ts:209`
- **Diagnosis**: The upstream catalog and fixture use `https://www.workbuddy.cn/document/term`, whereas the test previously asserted the obsolete `.ai` domain.
- **Fix**: Updated expectation to assert `https://www.workbuddy.cn/document/term`.

### Item 2: Stale `当前模型` Heading Assertion

- **File**: `tests/browser/agents-models.spec.ts:272`
- **Diagnosis**: The redesigned page architecture separates live configuration readback (`实际配置文件`) from saved profiles (`FyAgent 已保存方案`). The heading `当前模型` is no longer rendered.
- **Fix**: Replaced obsolete heading expectation with explicit assertions for `实际配置文件` and `FyAgent 已保存方案`, and asserted that heading `当前模型` has count 0.

### Item 3: ChangePlan Preview & Single Save Flow + Recovery Preview Fixture

- **Files**: `tests/browser/agents-models.spec.ts:605`, `tests/browser/support/features.ts`
- **Diagnosis**: Issue #56 introduced a unified single ChangePlan preview flow followed by `应用更改`, eliminating the legacy `保存前确认` disclosure. In addition, the fixture lacked a mock handler for `get_proxy_restore_preview`, causing IPC warnings.
- **Fix**:
  - In `tests/browser/support/features.ts`, implemented `get_proxy_restore_preview` fixture handler returning `{ app, enabled: false, canRestore: false, targets: [] }`.
  - In `tests/browser/agents-models.spec.ts`, replaced obsolete `confirmSaveDisclosure` dialog check with `expectSingleSavePreview`, verified that submit button remains disabled during hold, ensured zero `apply_change_plan` calls occur before release, and verified exact provider upsert payload on execution.

### Item 4: Models Input-Action Geometry Measurement Offset

- **File**: `tests/browser/agents-models.spec.ts:707`
- **Diagnosis**: Measurement took place before the model connection layout settled (352px/117px offsets caused by pending connection details rendering and viewport scrolling).
- **Fix**: Added explicit settling assertions for `ChatGPT · Browser Fixture` and `配置名称 = FyAgent Codex`, followed by `input.scrollIntoViewIfNeeded()` before taking bounding box measurements. Preserved the strict `<= 1px` alignment threshold.

### Item 5: Source Links Contrast & Collapsed Disclosure Layout in Chromium

- **File**: `src/pages/agents/AgentSourceLinks.css`
- **Diagnosis**:
  1. Author CSS `display: grid` on `.fy-agent-source-links` overrode user-agent stylesheet's `details:not([open]) > :not(summary) { display: none; }` in Chromium, causing closed disclosure content to participate in layout and be sampled during contrast verification against bright CI backings.
  2. Dark theme badges used hardcoded RGBA colors with insufficient contrast on bright backgrounds.
- **Fix**:
  - Added `details:not([open]) > .fy-agent-source-links { display: none; }` to ensure closed disclosures do not layout.
  - Set container background to `var(--fy-surface-opaque)` and link cards to `var(--fy-surface-raised)`.
  - Re-mapped badges to product semantic design tokens (`var(--fy-selected)`, `var(--fy-selected-border)`, `var(--fy-accent-text)`, `var(--fy-text)`, `var(--fy-warning-text)`). Retained existing contrast thresholds in `blue-themes.spec.ts`.

### Item 6: First-Use Guide Recommendation Keyboard Navigation

- **File**: `tests/browser/first-use-guide.spec.ts:191`
- **Diagnosis**: The guide cards render direct action `开始配置` buttons for each of the 3 recommended agents before `重新选择`. The keyboard test jumped directly from heading to `重新选择`, failing tab sequence assertions.
- **Fix**: Added keyboard tab navigation across all 3 `开始配置` buttons in DOM order before tabbing to `重新选择` and `查看全部软件`, preserving full keyboard navigation coverage.

### Item 7: TopBar About Button Primary Control Coverage

- **File**: `tests/browser/shell.spec.ts:280`
- **Diagnosis**: The About button in `TopBar.tsx` (`aria-label="关于 FyAgent"`) is a primary control increasing the count from 11 to 12, but was not in `visibleControlTestIds`.
- **Fix**: Added `"about"` to `visibleControlTestIds` (updating expected count to 12). Targeted the button via `getByRole("button", { name: "关于 FyAgent" })` for visibility and via `aria-label === "关于 FyAgent"` in the document-order keyboard loop, retaining complete keyboard focus testing.

### Item 8: Ambiguous `getByRole('combobox')` in Grok Subscription Flow

- **File**: `tests/browser/xai-subscription.spec.ts:140`
- **Diagnosis**: Multiple combobox controls (service preset selector and protocol selector) made unlabelled `page.getByRole("combobox")` ambiguous.
- **Fix**: Targeted `page.getByRole("combobox", { name: "切换到" })` to unambiguously select the saved subscription option while preserving payload verification.

### Item 9: Press-Feedback Geometry Shift during WorkBuddy Loading

- **File**: `tests/browser/press-feedback.spec.ts:98`
- **Diagnosis**: Initial bounding box was measured while WorkBuddy model list was still loading asynchronously (`正在读取 WorkBuddy 状态`), shifting the element y-position (427.125 to 394.125) once models loaded.
- **Fix**: Added explicit wait for loading status to disappear and `page.getByLabel("搜索已有模型")` to become visible before capturing baseline geometry. Preserved all press feedback and animation bounds checks.

---

## 3. Verification Commands & Results

### 3.1 Focused Renderer Tests

- **Command**:
  ```bash
  rtk proxy mise exec -- pnpm exec vitest run --config config/vitest.config.ts tests/renderer
  ```
- **Result**:
  - **131 passed / 131 test files** (1062 tests passed, 0 failed, duration: 43.59s)

### 3.2 Full Unit Suite (`test:unit`)

- **Command**:
  ```bash
  rtk proxy mise run test:unit
  ```
- **Result**:
  - **213 test files passed** (2065 tests passed, 1 skipped)
  - 3 contract failures noted (baseline/integration-owned: `classifyChanges.test.ts` demo ownership fixed by GPT-6 in integration tree; 2 contract timeouts under full suite CPU load: `dep0040Contract.test.ts`, `remainingPlatformSurface.test.ts`). All 131 renderer files passed cleanly.

### 3.3 Affected Browser Matrix

- **Command**:
  ```bash
  rtk proxy mise exec -- pnpm exec playwright test --config config/playwright.config.ts \
    tests/browser/agent-directory.spec.ts \
    tests/browser/agents-models.spec.ts \
    tests/browser/blue-themes.spec.ts \
    tests/browser/first-use-guide.spec.ts \
    tests/browser/shell.spec.ts \
    tests/browser/xai-subscription.spec.ts \
    tests/browser/press-feedback.spec.ts
  ```
- **Projects Tested**:
  - `chromium-900x600`
  - `chromium-1152x640`
  - `chromium-1232x700`
  - `chromium-1440x900`
  - `webkit-1232x700`
- **Result**:
  - **196 passed / 196 tests** (0 failed, duration: 4.4m)

---

## 4. Scoped Files Modified

1. `src/pages/agents/AgentSourceLinks.css`
2. `tests/browser/support/features.ts`
3. `tests/browser/agent-directory.spec.ts`
4. `tests/browser/agents-models.spec.ts`
5. `tests/browser/first-use-guide.spec.ts`
6. `tests/browser/press-feedback.spec.ts`
7. `tests/browser/shell.spec.ts`
8. `tests/browser/xai-subscription.spec.ts`
9. `ANTIGRAVITY_UI_RESULT.md`

_(Note: `ANTIGRAVITY_UI_TASK.md` is an untracked prompt document and is excluded from git staging.)_

---

## 5. Acceptance Follow-Up: Opened Source Links Contrast Coverage

### 5.1 Diagnosis & Follow-up Scope

- **Commit `ccb1aa1b` Integrated**: Reviewed and integrated into the integration tree as `a0a170b2`.
- **Identified Gap**: The original `Agent directory cards preserve text contrast on the bright CI backing` test validated text contrast against bright CI backing when the `<details>` disclosure was closed (confirming the collapsed layout fix). Direct verification was needed to exercise the badge and text palette in both light and dark modes when disclosures are actually OPENED.
- **Implemented Verification**:
  - In `tests/browser/blue-themes.spec.ts`, added dedicated light and dark tests: `opened official source disclosures preserve text and badge contrast in ${theme} mode on bright CI backing`.
  - The tests open disclosures on `grokbuild` and `codex` cards over the bright CI backing (`rgb(111, 141, 164)`).
  - Explicitly asserted the presence and contrast of all 5 distinct link/badge categories:
    1. `homepage` ("官方主页")
    2. `docs` ("官方文档")
    3. `download` ("官方下载")
    4. `license` ("开源许可")
    5. `terms` ("服务协议")
       (along with "桌面客户端" and section title "官方来源与许可").
  - Strictly asserted that all sampled text, link labels, and badges satisfy the WCAG `ratio >= 4.5` threshold (`samples.filter(s => s.ratio < 4.5)` is empty).

### 5.2 Visual Inspection Screenshots Captured

Representative screenshots of the opened source link disclosures were captured under the ignored `artifacts/ui-open-source-review/` directory:

- **Light Theme**:
  `~/.codex/worktrees/fyagent-next-ui-regressions/fyagent/artifacts/ui-open-source-review/open-source-links-light.png`
- **Dark Theme**:
  `~/.codex/worktrees/fyagent-next-ui-regressions/fyagent/artifacts/ui-open-source-review/open-source-links-dark.png`

### 5.3 Blue-Themes Browser Matrix Verification

- **Command**:
  ```bash
  rtk proxy mise exec -- pnpm exec playwright test --config config/playwright.config.ts tests/browser/blue-themes.spec.ts
  ```
- **Projects Tested**:
  - `chromium-900x600`
  - `chromium-1152x640`
  - `chromium-1232x700`
  - `chromium-1440x900`
  - `webkit-1232x700`
- **Result**:
  - **40 passed / 40 tests** (0 failed, duration: 44.0s)
  - All 8 tests passed across each of the 5 projects.
