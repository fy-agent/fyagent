# R5 最终修复复核

对象：control 固定提交 `35253aaf`，相对 `b52df5ce`。只核对原审查 P1 是否闭合及此次新增的直接问题；不评审并行的 `d6020b4d`，不触发真实 Apple 公证、签名或全库测试。

## 结论

**原 P1 已在源码与本地行为测试层闭合；未发现新的直接阻断。** `35253aaf` 可作为 R5 的唯一集成实现。原 `reliability-stage2-8118a163.md` 的 P1 发现对旧提交仍成立，其整改状态由本报告更新为已修复。

## 闭合证据

- `.github/workflows/release.yml:1314-1325` 顺序现在为 app 公证 → app staple → 源 app 完整验证 → 计算主程序摘要 → 从同一 `$APP_PATH` 制作 DMG → DMG 签名、公证及验证。没有在 app staple 之后再次签名 app。
- `scripts/release/macos-developer-id.sh:359-367` 新增 `notarize_app`，把已经签名的 app 用 `ditto -c -k --keepParent` 归档到既有私有 `$STATE_DIR`，调用原来的 `submit_for_notarization`。该函数继续复用真实 submission ID 及原 Accepted 轮询门槛；未 Accepted 不返回成功，因此不再出现原“尚无票据先 staple”的依赖断裂。
- 成功后删除临时 ZIP；失败走原工作流 always teardown 清理签名状态目录。没有把签名凭据或私有中间归档加入发布产物。单次等待预算 `9000` 秒显式作用于 app 与 DMG 两次公证，没有改掉轮询的拒绝/超时失败语义。
- 工作流 `:1328-1336` 保留只读挂载、挂载 app 的版本/摘要核对及完整 verifier。传入目标明确为 `$mount_point/FyAgent.app`；完整 verifier 仍要求 `codesign --verify --deep --strict` 和 app ticket，不会误验源 app。

## 测试是否覆盖真实依赖

新增 `runMacNotarization` 调用的是实际 `macos-developer-id.sh`，外部 `xcrun` / `ditto` / `security` 使用临时替身。替身只有在对应 app/DMG 获得 Accepted 后才允许 staple；测试断言实际日志顺序，而非仅搜索源码字符串。

- 成功路径经历 app In Progress → Accepted → staple，之后才出现 package-ticketed-app，再经历 DMG In Progress → Accepted → staple。
- app Invalid、missing-id、timeout 均不能到达 app staple 或打包；DMG 被拒绝时不能 staple DMG。
- 过时的“禁止 notarize-app”静态约束已修正。现有 app/DMG verifier 的执行型 trust-drift 负例仍保留，工作流另有固定路径和顺序断言。
- 本 fixture 的打包步骤是 ticket 哨兵，不是真实 `hdiutil` 镜像；它直接证明本次修复的公证/票据依赖，不能代替真实挂载/Apple 验证。这一边界不妨碍原 P1 在本阶段闭合。

已读取最终日志 `~/fyagent/tmp/fde-workstreams-20260919/control-evidence/r5-release-tests-final.log`：`tests/releaseWorkflow.test.ts` **54 / 54 通过**，包含上述五项新增行为用例及既有真实 verifier 脚本的替身执行用例。日志记录 runner 为 **Vitest 3.2.7**；这是该 R5 阶段的证据，不能当作随后集成 Vitest 4 后统一版本的运行证据。本次未重复执行。

## 保留边界

本轮无新的必须修项。尚无正式 Apple 服务返回、实际 Developer ID 构建及挂载产物回读，因此只报告“发布流程源码修复与本地行为检查通过”，不报告已发布或真实票据已验收。root 后续统一检查沿用既定集成版本与依赖版本，无需再合入第二份并行 R5 实现。

产品路径均相对于 `~/.codex/worktrees/fyagent-fde-control/fyagent` 的固定提交 `35253aaf`。
