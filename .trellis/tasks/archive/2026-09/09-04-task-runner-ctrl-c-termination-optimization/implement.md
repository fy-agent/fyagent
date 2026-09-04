# 任务运行器 Ctrl+C 终止优化执行计划

## 执行清单

- [x] 1. 在 `scripts/release/build-macos-privileged-helper.sh` 中添加 `trap 'exit 130' INT TERM`
- [x] 2. 在 `scripts/tasks/lib.mjs` 中实现 `signalExitCode`，重构 `runForeground` 支持二次 Ctrl+C 强退与退出码 130
- [x] 3. 在 `scripts/tasks/lib.mjs` 中优化 `run()` 的 `result.signal` 中断退出处理
- [x] 4. 在 `tests/miseTaskContract.test.ts` 中补充针对 `signalExitCode`、`runForeground` 信号处理与退出码映射的测试用例
- [x] 5. 执行验证命令：
  - `node --throw-deprecation ./node_modules/vitest/vitest.mjs run tests/miseTaskContract.test.ts`
  - `mise run check:contracts`
  - `mise run test:unit`
- [x] 6. 更新规范文档 `.trellis/spec/backend/task-runner-contract.md`
- [x] 7. 代码审查与提交

## 验证命令

```bash
# 运行契约与单元测试
pnpm test:unit tests/miseTaskContract.test.ts
mise run check:contracts
```

## 回滚命令

```bash
git restore scripts/release/build-macos-privileged-helper.sh scripts/tasks/lib.mjs tests/miseTaskContract.test.ts
```
