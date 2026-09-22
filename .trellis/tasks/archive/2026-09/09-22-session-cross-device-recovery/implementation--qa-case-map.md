> 归档说明：本文的时间与结论保留；具体工作站用户目录已替换为语义占位符。历史路径映射、原稿及提交稿哈希见[归档路径映射](research/archive-path-map.json)。可解析的相对链接已调整。

# 47 条 QA 用例到自动化 / 实机缺口映射

更新口径：只记录实际可执行入口与本轮真实结果；“局部通过”不等于整条 case 通过。

缩写：

- `F`：`tests/session-migration/fixture-contract.test.ts`
- `S`：`tests/session-migration/schema.test.ts`
- `P`：`tests/session-migration/tauri-port.test.ts`
- `U`：`tests/session-migration/ui-state.test.tsx`
- `I`：`tests/session-migration/import-dialog-behavior.test.tsx`
- `G`：`tests/session-migration/page-behavior.test.tsx`
- `B`：`tests/browser/session-migration.spec.ts`
- `R`：`src-tauri/tests/session_migration_model.rs`

| Case | 本轮自动化入口与结果 | 整条 case 状态 / 缺口 |
|---|---|---|
| QA-FID-001 | `S` 正文逐码点/CRLF/路径通过；`R` 已执行生产 Codex extractor→serializer→parser 并逐文比较；`B` 四视口通过 | blocked：尚缺 Tauri command 级 exporter 文件输出与重复导出 digest 对照 |
| QA-FID-002 | `F` 六类 LEAK 哨兵 fixture 合同通过；`R` 生产 Codex extractor 的六类黑名单与 omitted counts 通过 | blocked：尚未扫描 command 实际落盘文件原始 UTF-8 字节 |
| QA-FID-003 | `S` 正文路径与代码保真通过；`B` 正文路径与泄漏哨兵四视口通过 | blocked：尚缺生产 exporter envelope 与 native cwd 读回 |
| QA-FID-004 | `R` 对生产 Codex extractor 执行 missing-phase fixture，返回 `finalAnswerIndeterminate`；未知版本返回 `extractionRuleVersionMismatch` | blocked：尚缺 command 级“不产生包/临时文件”和脱敏 detail 断言 |
| QA-FID-005 | `S` 通过真实缺答复可导出；`R` 的 A/B/C 生产 extractor 断言已写，首次执行被 QA 临时文件竞争中断，隔离修复后待后端恢复编译再复跑 | blocked：Rust 最终结果、网络/模型调用计数和 command 级落盘尚无证据 |
| QA-FID-006 | `S` 七层闭集字段通过；`R` 尾随第二文档与重复 key 拒绝通过 | blocked：尚缺落盘包 rolling-hash、大小同阶和 archive/polyglot 扫描 |
| QA-FID-007 | `F` 源顺序 fixture 通过；`R` 已写 A→B→final B→C 和 open `[0,3]` 精确断言但最终复跑被后端编译阻断；`B` 四视口通过 | blocked：Rust 最终结果与 native adapter 读回未执行 |
| QA-PKG-001 | `R` 生产 parser 已拒绝重复 key 与尾随文档 | blocked：尚缺截断 corpus 以及 capability/native/receipt 调用均为 0 的 spy |
| QA-PKG-002 | `S` envelope 到 omitted 七层 excess-field 全通过；`P` native reply excess-field 通过 | blocked：后端 package parser 与零 native/receipt 调用尚未执行 |
| QA-PKG-003 | `S` v2 门控通过；`R` 正文篡改后重算 digest 拒绝通过 | blocked：尚缺 v0/随机版本、精确 detail 与 provider/native 调用为 0 |
| QA-PKG-004 | `R` 仅冻结模型常量 | not_run：128 MiB/200/4000/4 MiB/32 MiB/depth 64 的 limit±1 尚未执行 |
| QA-PKG-005 | `P` 证明 renderer request 无 command/argv 且拒绝 extra/overwrite | blocked：身份恶意字符、越界文件写和 spawn argv 监控尚未执行 |
| QA-IMP-001 | `U` 可区分 nativeWritten 与读回成功 | not_run：fake native write 成功 + readback 空的后端编排尚不可注入 |
| QA-IMP-002 | 无端到端 receipt/native writer 入口 | not_run |
| QA-IMP-003 | `P` 支持显式 saveAsNewCopy 请求形状 | not_run：R1→R2 冲突、旧映射不变与双 native ID 未执行 |
| QA-IMP-004 | 无分叉 provider fixture | not_run |
| QA-IMP-005 | `I` 通过：超时重试复用 requestId、改变另存绑定生成新 ID、pending 双击只调用一次且禁止关闭；`P` payload 稳定 | blocked：后端同 request writer=1、新 request writer+1 尚未执行 |
| QA-IMP-006 | 无 SQLite UPDATE 故障注入可调用面 | not_run |
| QA-IMP-007 | 无进程 kill / crash-point harness | not_run |
| QA-IMP-008 | `I` 的 unknown/disabled probe 门控与 `U` 用户自报正交通过；`B` 明确返回 `writeSupported=false`，四视口均验证恢复按钮 disabled，用户自报后仍 disabled | blocked：后端 native writer 调用=0 尚未证 |
| QA-IMP-009 | `S` 正文源路径保真；`B` 含目录选择后能力不提升断言 | blocked：目标目录不存在/有效目录 native metadata 读回尚未执行 |
| QA-IMP-010 | `P` closed request 不携 command/argv | not_run：包 secret scan、目标登录/模型 spy 尚未执行 |
| QA-IMP-011 | 无隔离 provider 预置会话 harness | not_run |
| QA-IMP-012 | `B` 四视口验证完整 `/memory` 入口仍可达 | blocked：MEMORY.md/USER.md/每日文件 hash+权限+mtime 审计尚未执行 |
| QA-ID-001 | `R` 已执行同正文相同 contentDigest、不同 origin 得到不同 snapshot；`B` 同正文双来源四视口通过 | blocked：默认 slot 与 receipt 两个独立 mapping 尚缺 |
| QA-ID-002 | 后端 identity 随机来源逻辑存在 | not_run：同包复制稳定与独立未知来源不折叠尚缺外部可调用测试面 |
| QA-ID-003 | `R` 空 assistant 与缺 assistant 可独立表示；后端 identity 单元入口覆盖 digest 不碰撞 | blocked：待统一 Rust 运行与默认 slot 断言 |
| QA-ID-004 | 无 target re-export + receipt invalidation harness | not_run |
| QA-REC-001 | 无公开 receipt DAO 并发/故障注入入口 | not_run |
| QA-REC-002 | 无 device A/B 普通备份恢复 harness | not_run |
| QA-REC-003 | `I/G/U` 通过：pending/reconciliation/ambiguous/failed 不显示绿色成功，Page ambiguous 不发成功 toast；`R` 状态禁止重写 | blocked：先写后非零/超时/断连的 counted writer 尚不可注入 |
| QA-REC-004 | `U` needsReconciliation 不提供重试动作 | blocked：0/2 候选 reconciler 与 writer=0 尚不可注入 |
| QA-REC-005 | `R` 仅正面 Failed 状态允许后续幂等动作 | blocked：authoritativeNoEffect 控制组/实验组尚不可注入 |
| QA-XOS-001 | 无 Windows 目标机 | blocked：需真实/明确 VM Windows、provider G0-G7、Mac→Windows 证据 |
| QA-XOS-002 | 无 Windows 源机 | blocked：需 Windows→Mac 独立证据 |
| QA-E2E-001 | `U` 阶段文案区分 | blocked：需真实 provider 打开、完全退出重启、两次原生读回 |
| QA-E2E-002 | 禁止本轮调用真实模型 | blocked：需显式有配额测试环境、完整真实 response；本轮不调用 |
| QA-E2E-003 | `U` 不把 nativeWritten/用户自报显示为 reply verified | blocked：需模拟端点 request capture 与真实端点分次执行 |
| QA-PROV-001 | 合成 Codex fixture 已落盘；生产 Codex extractor 单元入口存在 | blocked：原生 history 查询/UI 仍为空，真实模型与跨 OS 未验证 |
| QA-PROV-002 | 无安装版 OpenCode 隔离 XDG 运行 | not_run |
| QA-PROV-003 | 无安装版 Hermes 隔离 DB 运行 | not_run |
| QA-PROV-004 | 无安装版 Gemini 隔离项目运行 | not_run |
| QA-PROV-005 | 无安装版 Claude 隔离 config 运行 | not_run |
| QA-PROV-006 | `S/P/I/U` 使用后端一致的 `grokbuild` ID，能表达版本不支持且不能被用户提升 | blocked：未运行 1.0.34 实际 help/version 与 store diff |
| QA-PROV-007 | 无隔离 OpenClaw Gateway/store | not_run |
| QA-UI-001 | `I/G/U` 组件与页面行为通过；`B` 定向复测 12/12 通过；shell 路由数量和键盘顺序断言已精确更新并通过 | failed：900×600 下新增 Sessions 后最后一个主导航控件底边 605px，超过 601px 合同上限 |
| QA-ID-005 | `P` 同一 request payload 重放稳定；后端 identity 单元入口不使用包字节 hash | blocked：排版/exportedAt 变异后的 snapshot/slot 与 writer 次数尚未执行 |

## 当前自动化执行汇总

- `rtk mise run test:unit tests/session-migration`：7 files、45 tests 全部通过，包含真实 Import dialog 与 Sessions page 行为，不只是 fixture/常量。
- `rtk mise run typecheck`：通过。
- `B`：定位器按当前可访问名称修正后，production boot 3/3、迁移 12/12 通过。迁移 + shell 定向主套件为 31/36；4 个迁移定位失败已修并复测转绿，唯一剩余生产失败为 900×600 shell 底部溢出。由于定向未全绿，未启动完整 browser。
- `R` 基础合同最后一次干净结果为 7/7；扩展到生产 Codex extractor 后首次为 10/11，唯一失败是 QA harness 共用临时身份文件的并发竞争，已改为每用例独立目录。修复后的复跑被并行后端中间态编译错误阻断，因此不得把扩展集记录为全通过。
- 当前无 `backend-rework-exit.json` 或迁移 `test-hooks` 窄 re-export，遵循约束未反复编译 Rust，也未迁移 path-include harness。
- 47 条整体验收没有被声称全部通过；command、receipt/native 故障注入、跨系统和真实 provider 层仍按逐项缺口保留。
- 无真实用户会话、凭据、付费推理、真实 provider 写入或发布操作。
