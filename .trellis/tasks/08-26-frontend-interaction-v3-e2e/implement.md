# Frontend Interaction V3 Implementation Plan

## 闭环清单

任务只有同时满足以下条件才可标记完成：

- [ ] 11 个批准状态均已实现并有运行态截图。
- [ ] 真实 capability matrix 生效，无假写入或假成功。
- [ ] 全部必需检查、构建和本地桌面包 UAT fresh pass。
- [ ] Windows 原生验证有独立 fresh evidence。
- [ ] 缺陷表中无未解释的 Blocking/Major。
- [ ] 飞书阶段与最终消息均有 message ID、readback 和真实附件。
- [ ] 未触碰 main，未 push/merge/release。

## Phase 0｜规划与边界冻结（M0）

1. 完成 PRD、技术设计、实施计划、研究记录和 Trellis context。
2. 将设计集状态从 review pending 更新为 implementation approved。
3. 更新冲突的 frontend spec，旧条款标记 superseded。
4. 获取一次人类确认：批准本计划，以及 Grok/Gemini 的真实可用模型替代方案。
5. 批准后才运行 `task.py start`，把任务变为 `in_progress`。
6. 向飞书群汇报 M0，附上下文、分支、边界和下一步，保存 message ID/readback。

目标时间：00:45。当前状态：`planning_ready_pending_approval`。

## Phase 1｜环境与壳层（M1）

1. 在指定 worktree 执行 `mise run bootstrap`，随后做最小 env/system preflight。
2. 实现 typed navigation tree、SideNavigation、TopBar 收敛和 shell layout。
3. 保留六 route 与 `PersistentPrimaryOutlet` keep-alive。
4. 实现 active/expanded/focus/keyboard/responsive，并先完成组件/浏览器测试。
5. 用 Agent Install Readiness queries 实现 `/agents` 扫描目录骨架与 scanning/success/empty/error/unknown 状态；不新增 native scan/cancel 协议。

目标时间：03:00。里程碑证据：导航与扫描交互测试、两张运行截图。

## Phase 2｜Agent 四段选配与 11 页整合（M2）

1. 构建单 Agent configure shell、四段页签、返回和管理跳转。
2. 接入 Skills/MCP 现有 assignment owners 与 readback。
3. 建立模型/提示词 capability matrix；模型采用只读 projection + 既有管理入口，已有 direct owner 才允许委托写入；提示词只接 `PromptAppId` 支持集合，不支持目标真实降级。
4. 按 07–11 原型重排现有 Models/Skills/MCP/Prompts/Memory 页，不重写业务核心。
5. 覆盖 loading/empty/success/error/disabled/selected/destructive 状态。
6. Gemini 做逐页视觉/交互审查；Grok 做组件/后端边界审查与复杂度挑战；Codex 修复与整合。

目标时间：06:30。里程碑证据：11 状态可达、关键动作回读、页面截图集。

## Phase 3｜质量门禁与本地打包（M3）

按失败最早原则执行并修复：

```text
mise run lint:v2
mise run typecheck:v2
mise run test:v2
mise run test:v2:browser
mise run build:renderer
mise run typecheck
mise run format:check
mise run test:unit
mise run check
```

若触碰 Rust/backend，再增加：

```text
mise run rust:fmt:check
mise run rust:check
mise run rust:clippy
mise run rust:test
```

然后生成本地桌面包。签名、公证和公开分发保持 excluded。

目标时间：08:30。里程碑证据：命令、exit code、构建产物位置与 hash（不含 secret）。

## Phase 4｜本地 packaged-app UAT（M4）

1. 从构建产物启动应用，不以 dev server 替代。
2. 按 11 个原型逐页验收并保存截图。
3. 覆盖导航、展开、返回、搜索、选择/启停、保存、错误、unknown 和重试；不验收不存在的取消成功态。
4. 缺陷按 Blocking/Major/Minor 记录，Codex 负责修复和 fresh rerun。
5. 运行态稳定后才生成最终证据包，避免证据被后续改动失效。

目标时间：10:00。里程碑证据：UAT ledger、截图和缺陷关闭记录。

## Phase 5｜Windows 原生验证与交付（M5）

1. 将候选产物发送到已授权的 Windows 执行路径；传输成功不算运行成功。
2. 在 Windows 原生环境验证安装/启动或当前任务约定的可运行包、六 route、11 状态与关键平台路径。
3. 保存 Windows-native receipt、截图/日志、失败路径和环境版本。
4. Codex 核验所有 Agent 产物、任务卡状态、证据层级和 stale 结论。
5. 在飞书群发送最终详细报告与图片，message ID + readback 后才声明群内交付完成。

目标时间：12:00。若 Windows 环境/权限不可达，只能报告真实 blocker，不能用 macOS 证据替代。

## A-to-A 责任与模型

| 参与者 | 固定责任 | 模型与 reasoning | 成本边界 |
|---|---|---|---|
| Codex | 最终 owner、线程拆分、整合、测试、缺陷闭环、证据与群汇报 | `gpt-5.6-sol / max`，已批准 | 最高档；所有新 Codex 任务固定使用 |
| Gemini | 视觉、交互、逐页一致性与状态审查；可提交其责任文件 | `antigravity/gemini-3.7-flash-high`，真实 probe 已通过 | 最高可用 thinking，外部模型成本 |
| Grok | 后端/组件/研究、能力边界、复杂度与测试必要性挑战 | 请求 `grok-4.7/max` 不可用；执行用 `vibekey/grok-4.6/high` | 当前官方/本地最高可用档，明确保留替代缺口 |

所有交付返回：`结论 / 变更 / 证据 / 边界 / 下一步`。Codex 必须命令级核验，不接受仅凭完成声明。

## 已创建的 Codex 独立线程

- `FyAgent v3 前端差距盘点`：只读审计目标/现状差距与 capability matrix。
- `FyAgent v3 测试打包预检`：只读检查 bootstrap、质量命令、打包和系统依赖。
- `FyAgent v3 实现简化与风险审查`：只读挑战抽象、测试和交付顺序。

三条线程均固定 `gpt-5.6-sol / max`，不会自行修改主实现 worktree；Codex 汇总结论后再拆写入责任。

## 停止条件

仅以下情况允许暂停：

- 人类尚未批准本规划或外部模型替代；
- 必需权限、凭证或 Windows 执行环境不可达；
- 新鲜验证出现同一硬阻塞且安全替代路径已穷尽；
- 动作将越过 push/merge/release/production 授权边界。
