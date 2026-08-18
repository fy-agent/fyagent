# Issue #35 可消费主线 — 设计

## Architecture

单 owner：`#35 module`。只补投影 digest 计算与 mint 回填。不新建 admission 模块。

已有 `src-tauri/src/proxy/json_canonical.rs` 的 key-sort canonical 不是 RFC 8785 权威。digest 必须按 `secret-contract-v1.md`：完整投影省略 `projectionDigest`，RFC 8785 序列化，加 operation 前缀，SHA-256 小写 hex。

## Multi-dimensional array

行 = 线。列 = 层。格 = owner / 本卡是否动。

| 线 \ 层 | 类型形 | digest | 命令 mint | native 体 | 下游消费 |
| --- | --- | --- | --- | --- | --- |
| 候选激活 | 已有。本卡不动字段 | **本卡做** | `list_secret_candidates` 回填 | 不动 `todo!` | #55 本卡不做 |
| live apply | 已有。无 `candidateId` | **本卡做** | `check_secret_apply_readiness` 保留 plan | 不动 `resolve_for_apply` | #41 本卡不做 |
| staged import | 已有 | **本卡做** + 单元夹具 | 不接线生产 resume | 不发明 admission | main 本卡不做 |

并行禁令：同一时刻只有一个 writer 改 `types.rs` / `command_map.rs`。阵列是验收矩阵，不是三人同时改。

后续阵列（本卡关闭后另开，不在本卡冲）：

| 线 \ 层 | journal/resume | Keychain | #55 准入 | #41 lease | 双机 UAT |
| --- | --- | --- | --- | --- | --- |
| 三线 | 缺字段 | 生产 unavailable | 模块不存在 | 两段未拆 | Windows 六条 |

## Data flow

1. store 行 → 已有 constructor 填 Repr，`projection_digest` 先占位或空计算。
2. `hash_projection(operation, repr_without_digest) -> SecretProjectionDigest`。
3. 写入 Repr → `validate_repr`。校验必须重算并比对。
4. 命令返回含该 digest 的投影 / readiness。

## Compatibility

- 不改 D2 字段集。不改 surfaces 文件。
- #55 现网仍无投影类型。本卡关闭后 #55 仍要自己接线。本卡只保证字节与 digest 可核。
- 禁止把 `binding_set_cas.digest` 或 change-plan `sha256:` digest 当作投影 digest。

## Rollback

只改 secret 模块 digest/mint。失败回退 `4c393721`。不碰 `Page.tsx`。不 add visual 文件。
