> 归档说明：本文的时间与结论保留；具体工作站用户目录已替换为语义占位符。历史路径映射、原稿及提交稿哈希见[归档路径映射](research/archive-path-map.json)。可解析的相对链接已调整。

# QA final report

## 本轮实际结果

| 验证 | 结果 |
|---|---|
| `rtk mise run test:unit tests/session-migration` | 7 files、45 tests 全部通过 |
| `rtk mise run typecheck` | 通过 |
| `rtk mise run test:browser -- tests/browser/session-migration.spec.ts` | production boot 3/3、迁移 12/12 通过 |
| 迁移 + shell 定向 browser | 31/36；4 个测试定位器已修并在上述复测转绿，剩余 1 个生产布局失败 |
| 完整 browser | 未运行：按约定，定向 shell 尚未全绿 |
| Rust rework 复跑 | 未运行：无 `backend-rework-exit.json` / migration `test-hooks` |
| Windows 实机 | 按授权略过，标记未验证 |

## 唯一新增生产阻塞

`tests/browser/shell.spec.ts` 在 900×600 下发现主导航底部溢出：控件底边为 605px，合同上限为 601px。1152×640、1232×700、1440×900 通过；导航数量、路由顺序和键盘顺序断言均已按真实 Sessions 入口更新并通过。QA 未放宽断言，生产布局不在本 writer 范围。

## 已验证的修复

- disabled/unknown local probe 下恢复按钮保持 disabled。
- 用户自报“已手动续聊”和 workspace 选择均不能提升恢复能力。
- 同正文不同来源保持两个 origin/snapshot；连续与未完成 user 顺序保留。
- 旧 `/memory` 完整编辑入口在四个视口仍可达。
- requestId、重复点击、关闭重开隔离、多 provider snapshot 过滤、非成功状态、批量导出冻结集合等组件行为共 45 项单元测试通过。

## 边界

没有使用真实会话、凭据、真实 provider 写入或付费推理。47 条 case 未被宣称整体通过；command/receipt/native 故障注入、Rust rework、跨系统和真实 provider 缺口继续保留在 `qa-case-map.md`。
