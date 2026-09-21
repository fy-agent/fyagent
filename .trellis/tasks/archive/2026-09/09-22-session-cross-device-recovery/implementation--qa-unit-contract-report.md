> 归档说明：本文的时间与结论保留；具体工作站用户目录已替换为语义占位符。历史路径映射、原稿及提交稿哈希见[归档路径映射](research/archive-path-map.json)。可解析的相对链接已调整。

# QA unit contract report

## 范围

本轮仅修改 Session 前端生产 CSS 与本报告。未修改 renderer 共享合同测试、Tauri `lib.rs`、permissions、其他 Rust、shell widget，也未运行 Rust 全套测试。

## 初始复现

命令：

`rtk mise run test:unit tests/renderer/shared/designTokens.test.ts tests/renderer/platform/tauriAclContract.test.ts tests/renderer/app/userFacingCopy.test.ts`

结果：3 files failed；6 tests 中 4 failed、2 passed。

1. `designTokens.test.ts`
   - `src/pages/sessions/page.css` 有 11 处硬编码 `border-radius`。
   - 同文件有 8 处硬编码 transition 时长。
2. `tauriAclContract.test.ts`
   - renderer literal invoke 实际 175，测试硬编码期望 159。
3. `userFacingCopy.test.ts`
   - 实际页面族新增 `sessions`，测试仍期望九个旧页面族。
   - “does not expose reviewed implementation narration” 独立用例通过；没有 Session 禁用文案命中。

## 修改

`src/pages/sessions/page.css`

- 小型 badge、提示、code/kbd 标签与页签统一使用 `--fy-radius-compact`。
- 所有硬编码 `120ms` / `140ms` 改为 `--fy-motion-control`。
- transition easing 改为 `--fy-motion-ease`，并把 `all` 收窄为实际变化的颜色、背景、边框和阴影属性。
- 未改恢复状态、target 打开 guard、attempt/provider binding、空恢复结果分类或 shell 布局。

## 验证结果

| 命令 | 结果 |
|---|---|
| `rtk mise run format:files -- src/pages/sessions/page.css` | pass；文件格式无额外变化 |
| `rtk mise run test:unit tests/renderer/shared/designTokens.test.ts` | pass：1 file，2/2 tests |
| `rtk mise run test:unit tests/session-migration` | pass：7 files，45/45 tests |
| 三合同最终复测（同初始命令） | partial：design tokens 2/2 pass；总计 4 pass、2 fail |

## IPC 精确核对

只读集合审计结果：

- renderer literal commands：175
- `generate_handler!` registered commands：430
- main capability allowed commands：430
- renderer 未注册：`[]`
- renderer 未授权：`[]`
- registered 未授权：`[]`
- allowed 未注册：`[]`

因此当前失败不是 migration IPC 漏注册，也不需要 Opus 修改 `lib.rs` / permissions；唯一不匹配是共享测试的精确数量仍为 159。迁移命令已出现在真实 handler 和 `allow-session-migration` ACL 中。

## 剩余阻塞与 release ownership

1. `tests/renderer/platform/tauriAclContract.test.ts`
   - owner：平台 surface / root。
   - 动作：审核本分支新增 literal invoke 后，将预期命令集合基线同步到 175；保留四个集合双向相等检查，不放宽注册或 ACL 断言。
2. `tests/renderer/app/userFacingCopy.test.ts`
   - owner：共享 copy contract / root。
   - 动作：把 `sessions` 纳入受审页面族并同步“nine route families”名称。禁用实现叙述用例当前已通过，不需要为了测试改写 Session 产品文案。
3. Rust migration test hooks
   - owner：Opus/backend。
   - 当前仍在实现；QA 未轮询、未替换 path-include harness、未编辑任何 Rust 注册文件，也未运行整套 Rust。

在以上两个共享合同基线同步前，不能宣称 full unit 全绿；Session 自有生产修复和迁移单元回归已通过。
