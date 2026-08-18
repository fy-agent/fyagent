# Issue #35 可消费主线 — 执行

## Gate

未获本规划摘要批准前不改产品代码、不 `task.py start`。

## Checklist

1. 在 `secret` 模块内实现 RFC 8785 omit-digest hasher。三条前缀常量。单测：稳定、字段漂移、无 `sha256:`。
2. `validate_repr` 三份投影都重算比对。夹具里 `"cd" * 32` 作废。
3. `command_map` 候选 mint 与 apply readiness 改用 hasher。apply 成功路径保留 `SecretApplyPlanProjection`。
4. staged 至少一条独立 hasher + 互解码负例。不接线 `resume_staged_import_cutover`。
5. `rtk mise run rust:test -- secret_`。失败停。

## Validation

```bash
rtk mise run rust:test -- secret_
```

另可用 focused 过滤 digest 测试名。证据：命令输出行数与失败为 0。

## Risky files

- `src-tauri/src/secret/types.rs` — 只加 hasher / validate，不改字段。
- `src-tauri/src/secret/command_map.rs` — 只改 digest 来源与 plan 丢弃。
- 禁止：`Page.tsx`、surfaces、#55 产品、`phase1-visual-*`。

## Rollback

`git checkout --` 上述两文件。HEAD 仍停在批准前的提交，直到用户明确要求 commit。
