# 公证 fixture 耗时修复

范围仅 `tests/releaseWorkflow.test.ts` 的 `runMacNotarization`，没有修改生产脚本、其他测试 helper、timeout 或任何断言，没有提交。

## 定位

- root 的 `control-evidence/release-timeout-recheck.log` 显示 timeout 场景 5170 ms，超过 Vitest 默认 5000 ms；两个文件其余 86 项通过。
- 修改前独立运行该场景通过，但该测试仍耗时约 1720 ms。将同一 helper 放入独立 Node 驱动测得约 1542 ms，说明不只是 Vitest 调度。
- 对实际子脚本启用只读 shell trace，并由 Node 在 stderr 到达时计时：新建 security 脚本开始于 10 ms，ditto 开始于 577 ms，xcrun submit 开始于 1095 ms，随后 Python 解析开始于 1597 ms；同一 xcrun 第二次调用约 4 ms，两次真实 Python 解析各约 30 ms。主要开销集中在三个新建可执行文件的首次启动。未把它归因于未经验证的特定 macOS 安全组件。

## 修复

测试 wrapper 中定义并 `export -f` 既有 security/ditto/xcrun 模拟命令。生产脚本仍以独立 Bash 执行，继承这三个模拟命令；ditto/xcrun 使用子 shell 函数以保留独立命令的 exit 和 set -e 行为。移除三个临时 executable 文件及其 PATH 覆盖。

保留真实生产脚本的 app ZIP 提交、poller、Python JSON 解析、Accepted 复核、staple 顺序和 DMG 后续提交；不是改成字符串断言或模拟整个公证流程。五个用例与全部断言保持原样，默认 5 秒及 helper 的进程上限均未提高。

## 验证

- `mise exec -- pnpm test:unit tests/releaseWorkflow.test.ts -t 'waits for app acceptance|stops before app staple|does not staple a denied final DMG'`：5/5 通过，75 个无关用例跳过；全部五项测试合计 788 ms，整个 Vitest 运行 1.42 秒。相比修复前单个 timeout 场景 1.72 秒，启动开销显著下降。
- 定向 Prettier check 和 `git diff --check` 通过。
- 单用例诊断最初尝试 canonical `mise run test:unit ... -t ...` 被 runner 的选项过滤拒绝，未执行测试；随后使用受管理 runtime 中的只读 Vitest 名称过滤。没有修改 runner 限制。
- 没有重跑全库或制造并发负载。该结果证明这五项现有行为断言通过、首启开销已消除，不承诺任意主机负载下绝不超时。此前其他 helper 的慢用例不在本次改动范围。
