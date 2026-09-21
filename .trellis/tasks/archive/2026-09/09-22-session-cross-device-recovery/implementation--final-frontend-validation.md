> 归档说明：本文的时间与结论保留；具体工作站用户目录已替换为语义占位符。历史路径映射、原稿及提交稿哈希见[归档路径映射](research/archive-path-map.json)。可解析的相对链接已调整。

# Final frontend validation

Final related UI source is frozen. Canonical `mise run typecheck`, `lint`, and `format:check` all exited 0.

Canonical `mise run test` exited 0 after the research ownership correction: **223 files; 2,134 passed / 1 pre-existing skipped**. Its desktop mock leaf passed **7/7**, and visual manifest preflight passed without updating baselines. The raw log and actual subprocess exit receipt remain outside the repository in `work/executor-logs/final-frontend/`.

Browser coverage: initial complete production run passed boot **3/3** and **666/667** renderer tests. The sole animation failure reproduced against unchanged baseline animation code because model data had not loaded; the test now waits for the existing model and still requires intermediate motion samples plus exact start/end heights. The complete affected animation file then passed **25/25**. After final Session status copy/state changes, production boot **3/3** and Session + shell regressions **36/36** passed. Eight final fixture screenshots across 900×600 and 1440×900 were inspected; no overflow or page errors. No unrelated browser rerun is inferred or required from these separate runs.

These are renderer/browser fixtures and mock IPC checks, not real desktop/native-user acceptance. Native writer evidence and the waived Windows real-machine acceptance are documented separately.
