# 移除 Codex 安装校验并补齐模型连通与 Claude v1 门禁

## Goal

Codex 一键安装不再做安装包校验；Claude Code 对显式 v1 路径给出警告；V2 模型页在保存前可测连通（Qoder/TRAE 除外）。

## Requirements

1. 去掉下载后整包 SHA-256 复验（`revalidate_artifact`、`VerifiedFilePin` 全文件 `verify_reader`）。不要与上游 checksum/签名比较。保留 OS 安装器、受保护 job 目录、安装后存在性检查。下载过程中的流式哈希若不再被消费可一并删除。
2. Claude 服务地址 pathname 含独立 `v1` 段时，警告：最终 Claude 需要访问的完整端点将会是 `/v1/v1/XXXX`，请确认是否需要添加 v1，通常路径一般为 `/v1/XXXX`。hostname `v1.example.com` 不算。不阻断保存。仅 Claude。
3. 新增草稿 URL 连通 IPC，复用 `probe_reachability`：GET base URL，任意 HTTP 响应即可达。挂到 WorkBuddy、Claude、Codex、Grok Build、OpenCode 配置表单，保存前可点。Qoder/TRAE 不挂。

## Acceptance Criteria

- [x] 安装路径不再对同一文件做多次全量 SHA 复验
- [x] Claude 显式 `/v1` 有警告，无 v1 无警告
- [x] 五个模型面板可在草稿 URL 上测连通；Qoder/TRAE 没有按钮
