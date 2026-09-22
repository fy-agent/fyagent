# 历史文本（工作站路径已匿名化）：evidence/browser-final-review/motion-regression.log

此文件为历史源码或运行证据，经工作站用户目录语义匿名化后以 Markdown 保存，不是运行入口。归档过程未执行其中内容。**本提交不再声称代码块与原稿字节相同。** 该日志先前未被 Git 跟踪，原稿不能从来源提交恢复；原始字节单独保留在仓库外的本地 archive-original-logs 备份中，不纳入 PR。

原路径、原稿与提交稿的哈希及恢复来源见[归档路径映射](research/archive-path-map.json)。

- 原稿字节数：10560
- 原稿 SHA-256：`ad45773ff9e2593e8a44904e742ad89ab76115350360d6b97e85e2d3a9f41b8c`
- 匿名化载荷字节数：10559
- 匿名化载荷 SHA-256：`84e39c4f2f057f8ff934c59f46a43681860dc8caa2668af84495d67fdab28b4f`

下方载荷的准确字节边界见路径映射；若载荷没有末尾换行，围栏前仅补展示换行。

```text
[test:browser] $ pnpm test:browser tests/browser/state-motion.spec.ts

> fyagent@ test:browser /Users/<username>/Documents/Codex/2026-09-22/new-chat/work/fyagent-session
> playwright test --config config/playwright.performance.config.ts --grep 'production boots' && playwright test --config config/playwright.config.ts tests/browser/state-motion.spec.ts

[WebServer] (node:56473) Warning: The 'NO_COLOR' env is ignored due to the 'FORCE_COLOR' env being set.
[WebServer] (Use `pnpm --trace-warnings ...` to show where the warning was created)
[WebServer] (node:56491) Warning: The 'NO_COLOR' env is ignored due to the 'FORCE_COLOR' env being set.
[WebServer] (Use `node --trace-warnings ...` to show where the warning was created)
[WebServer] (node:56507) Warning: The 'NO_COLOR' env is ignored due to the 'FORCE_COLOR' env being set.
[WebServer] (Use `pnpm --trace-warnings ...` to show where the warning was created)
[WebServer] (node:56512) Warning: The 'NO_COLOR' env is ignored due to the 'FORCE_COLOR' env being set.
[WebServer] (Use `node --trace-warnings ...` to show where the warning was created)

Running 3 tests using 1 worker

(node:56526) Warning: The 'NO_COLOR' env is ignored due to the 'FORCE_COLOR' env being set.
(Use `node --trace-warnings ...` to show where the warning was created)
  ✓  1 tests/browser/navigation-performance.spec.ts:17:1 › production boots all nine primary routes without initialization errors (1.2s)
  ✓  2 tests/browser/navigation-performance.spec.ts:32:1 › production boots the first-use guide on demand and persists completion (347ms)
  ✓  3 tests/browser/presentation-performance.spec.ts:4:1 › production boots with unit-correct presentation timing (1.8s)

  3 passed (7.3s)
[WebServer] (node:56578) Warning: The 'NO_COLOR' env is ignored due to the 'FORCE_COLOR' env being set.
[WebServer] (Use `pnpm --trace-warnings ...` to show where the warning was created)
[WebServer] (node:56592) Warning: The 'NO_COLOR' env is ignored due to the 'FORCE_COLOR' env being set.
[WebServer] (Use `node --trace-warnings ...` to show where the warning was created)

Running 25 tests using 5 workers

(node:56600) Warning: The 'NO_COLOR' env is ignored due to the 'FORCE_COLOR' env being set.
(Use `node --trace-warnings ...` to show where the warning was created)
(node:56603) Warning: The 'NO_COLOR' env is ignored due to the 'FORCE_COLOR' env being set.
(Use `node --trace-warnings ...` to show where the warning was created)
(node:56602) Warning: The 'NO_COLOR' env is ignored due to the 'FORCE_COLOR' env being set.
(Use `node --trace-warnings ...` to show where the warning was created)
(node:56604) Warning: The 'NO_COLOR' env is ignored due to the 'FORCE_COLOR' env being set.
(Use `node --trace-warnings ...` to show where the warning was created)
(node:56601) Warning: The 'NO_COLOR' env is ignored due to the 'FORCE_COLOR' env being set.
(Use `node --trace-warnings ...` to show where the warning was created)
  ✓   5 [chromium-900x600] › tests/browser/state-motion.spec.ts:205:1 › revisiting a page keeps its lens size while real tab changes still interpolate (3.1s)
(node:56634) Warning: The 'NO_COLOR' env is ignored due to the 'FORCE_COLOR' env being set.
(Use `node --trace-warnings ...` to show where the warning was created)
  ✓   3 [chromium-900x600] › tests/browser/state-motion.spec.ts:273:1 › explicit steps remain continuous without ResizeObserver and reverse from the intermediate size (4.8s)
  ✓   4 [chromium-900x600] › tests/browser/state-motion.spec.ts:151:1 › a resizing step can close, reopen, resize and adopt reduced motion without a stuck modal (5.2s)
  ✓   1 [chromium-900x600] › tests/browser/state-motion.spec.ts:100:1 › login steps resize one real dialog through intermediate frames without scaling forms (5.4s)
(node:56639) Warning: The 'NO_COLOR' env is ignored due to the 'FORCE_COLOR' env being set.
(Use `node --trace-warnings ...` to show where the warning was created)
(node:56640) Warning: The 'NO_COLOR' env is ignored due to the 'FORCE_COLOR' env being set.
(Use `node --trace-warnings ...` to show where the warning was created)
(node:56641) Warning: The 'NO_COLOR' env is ignored due to the 'FORCE_COLOR' env being set.
(Use `node --trace-warnings ...` to show where the warning was created)
  ✓   2 [chromium-900x600] › tests/browser/state-motion.spec.ts:336:1 › model ID disclosure reuses the shared collapse and closes from the populated height (6.2s)
(node:56654) Warning: The 'NO_COLOR' env is ignored due to the 'FORCE_COLOR' env being set.
(Use `node --trace-warnings ...` to show where the warning was created)
  ✓   8 [chromium-1152x640] › tests/browser/state-motion.spec.ts:205:1 › revisiting a page keeps its lens size while real tab changes still interpolate (2.6s)
(node:56661) Warning: The 'NO_COLOR' env is ignored due to the 'FORCE_COLOR' env being set.
(Use `node --trace-warnings ...` to show where the warning was created)
  ✓   6 [chromium-1152x640] › tests/browser/state-motion.spec.ts:100:1 › login steps resize one real dialog through intermediate frames without scaling forms (5.1s)
  ✓  10 [chromium-1152x640] › tests/browser/state-motion.spec.ts:336:1 › model ID disclosure reuses the shared collapse and closes from the populated height (2.0s)
(node:56663) Warning: The 'NO_COLOR' env is ignored due to the 'FORCE_COLOR' env being set.
(Use `node --trace-warnings ...` to show where the warning was created)
  ✓   7 [chromium-1152x640] › tests/browser/state-motion.spec.ts:151:1 › a resizing step can close, reopen, resize and adopt reduced motion without a stuck modal (4.3s)
(node:56667) Warning: The 'NO_COLOR' env is ignored due to the 'FORCE_COLOR' env being set.
(Use `node --trace-warnings ...` to show where the warning was created)
  ✓   9 [chromium-1152x640] › tests/browser/state-motion.spec.ts:273:1 › explicit steps remain continuous without ResizeObserver and reverse from the intermediate size (4.3s)
(node:56670) Warning: The 'NO_COLOR' env is ignored due to the 'FORCE_COLOR' env being set.
(Use `node --trace-warnings ...` to show where the warning was created)
(node:56676) Warning: The 'NO_COLOR' env is ignored due to the 'FORCE_COLOR' env being set.
(Use `node --trace-warnings ...` to show where the warning was created)
  ✓  13 [chromium-1232x700] › tests/browser/state-motion.spec.ts:205:1 › revisiting a page keeps its lens size while real tab changes still interpolate (2.5s)
(node:56688) Warning: The 'NO_COLOR' env is ignored due to the 'FORCE_COLOR' env being set.
(Use `node --trace-warnings ...` to show where the warning was created)
  ✓  11 [chromium-1232x700] › tests/browser/state-motion.spec.ts:100:1 › login steps resize one real dialog through intermediate frames without scaling forms (4.7s)
  ✓  15 [chromium-1232x700] › tests/browser/state-motion.spec.ts:336:1 › model ID disclosure reuses the shared collapse and closes from the populated height (2.0s)
  ✓  12 [chromium-1232x700] › tests/browser/state-motion.spec.ts:151:1 › a resizing step can close, reopen, resize and adopt reduced motion without a stuck modal (4.0s)
(node:56694) Warning: The 'NO_COLOR' env is ignored due to the 'FORCE_COLOR' env being set.
(Use `node --trace-warnings ...` to show where the warning was created)
(node:56700) Warning: The 'NO_COLOR' env is ignored due to the 'FORCE_COLOR' env being set.
(Use `node --trace-warnings ...` to show where the warning was created)
(node:56697) Warning: The 'NO_COLOR' env is ignored due to the 'FORCE_COLOR' env being set.
(Use `node --trace-warnings ...` to show where the warning was created)
  ✓  14 [chromium-1232x700] › tests/browser/state-motion.spec.ts:273:1 › explicit steps remain continuous without ResizeObserver and reverse from the intermediate size (4.1s)
(node:56719) Warning: The 'NO_COLOR' env is ignored due to the 'FORCE_COLOR' env being set.
(Use `node --trace-warnings ...` to show where the warning was created)
  ✓  19 [chromium-1440x900] › tests/browser/state-motion.spec.ts:205:1 › revisiting a page keeps its lens size while real tab changes still interpolate (2.8s)
  ✓  16 [chromium-1440x900] › tests/browser/state-motion.spec.ts:100:1 › login steps resize one real dialog through intermediate frames without scaling forms (4.3s)
(node:56727) Warning: The 'NO_COLOR' env is ignored due to the 'FORCE_COLOR' env being set.
(Use `node --trace-warnings ...` to show where the warning was created)
(node:56728) Warning: The 'NO_COLOR' env is ignored due to the 'FORCE_COLOR' env being set.
(Use `node --trace-warnings ...` to show where the warning was created)
  ✓  20 [chromium-1440x900] › tests/browser/state-motion.spec.ts:336:1 › model ID disclosure reuses the shared collapse and closes from the populated height (2.0s)
  ✓  17 [chromium-1440x900] › tests/browser/state-motion.spec.ts:151:1 › a resizing step can close, reopen, resize and adopt reduced motion without a stuck modal (4.1s)
(node:56743) Warning: The 'NO_COLOR' env is ignored due to the 'FORCE_COLOR' env being set.
(Use `node --trace-warnings ...` to show where the warning was created)
  ✓  18 [chromium-1440x900] › tests/browser/state-motion.spec.ts:273:1 › explicit steps remain continuous without ResizeObserver and reverse from the intermediate size (4.4s)
(node:56744) Warning: The 'NO_COLOR' env is ignored due to the 'FORCE_COLOR' env being set.
(Use `node --trace-warnings ...` to show where the warning was created)
(node:56758) Warning: The 'NO_COLOR' env is ignored due to the 'FORCE_COLOR' env being set.
(Use `node --trace-warnings ...` to show where the warning was created)
  ✓  23 [webkit-1232x700] › tests/browser/state-motion.spec.ts:205:1 › revisiting a page keeps its lens size while real tab changes still interpolate (3.3s)
  ✓  22 [webkit-1232x700] › tests/browser/state-motion.spec.ts:151:1 › a resizing step can close, reopen, resize and adopt reduced motion without a stuck modal (4.2s)
  ✓  25 [webkit-1232x700] › tests/browser/state-motion.spec.ts:336:1 › model ID disclosure reuses the shared collapse and closes from the populated height (3.2s)
  ✓  21 [webkit-1232x700] › tests/browser/state-motion.spec.ts:100:1 › login steps resize one real dialog through intermediate frames without scaling forms (4.6s)
  ✓  24 [webkit-1232x700] › tests/browser/state-motion.spec.ts:273:1 › explicit steps remain continuous without ResizeObserver and reverse from the intermediate size (5.0s)

  25 passed (26.4s)
```
