# V2 体验、发布与工程卫生

## Goal

一次性交付用户列出的八项：去掉 Codex 一键安装的安装包校验、对齐 Skill/MCP 页头、补齐 Agent 目录介绍、让正式发布不必再等一轮完整 CI 且 tag 可重试、删掉文档契约屎山测试、Claude Code 显式 `/v1` 门禁、安全范围内清洗仓库膨胀、以及模型页配置前连通性测试（Qoder/TRAE 除外）。

## Requirements

1. Codex 一键安装不再对安装包做 FyAgent 侧校验（含上游 checksum/签名准入，以及会把整包再读一遍的本地 SHA-256 复验）。后续一键安装不得继承这套校验。原生 OS 安装器与“安装后是否存在可运行结果”保留。
2. Skill 与 MCP 页头：已安装/发现与右上角两个按钮同一行。Skill 不得让页签独占整行。Skill 切到发现时仍保留「检查更新」「更多」。
3. Agent 目录除 Codex 安装器外，六个详情页必须有基于官方资料的实质性中文介绍；不渲染 catalog `description`，不出现「使用说明」。
4. 正式发布以 tag 指向的 commit 为权威；不再要求 live `main` HEAD 全程冻结、不再要求同 SHA 的 `main` push CI 先绿、允许在尚未发布 GitHub Release 时移动/重打同一版本 tag。不恢复巨型 `target/` 缓存；允许 lockfile 键控的 Cargo registry 缓存与 Release 侧 pnpm cache。吸收 main 上已验证的「只公证一次 DMG」。
5. 删除以 Markdown/spec 字符串包含为中心的冗余契约测试；保留真正执行行为或生成物 byte 比较的单一检查器。
6. Claude Code 服务地址若路径显式含 `v1` 段，警告完整端点会变成 `/v1/v1/XXXX`；未显式输入则不提示。警告不阻断保存。
7. 扫描并停止追踪无用大文件（安全范围：`.gitignore` + `git rm --cached`，不改写历史）。测试以准确为准，不为堆数量。
8. V2 模型页在配置保存前可对草稿 URL 做 reachability 连通测试；Qoder、TRAE 不提供该按钮。复用既有 `stream_check` 探测语义（任意 HTTP 响应即可达，不发真实模型请求）。

## Acceptance Criteria

- [x] Codex Desktop 一键安装路径不再对下载包做整包 SHA 复验/上游准入；安装仍能走到原生安装器并确认存在结果
- [x] Skill/MCP 页头：页签与两个主按钮同一行；Skill 发现页仍有「检查更新」「更多」
- [x] 六个非 Codex Agent 详情有多段介绍，且测试仍禁止 catalog description / 使用说明
- [x] 正式发布资格不再绑定 live main HEAD + 同 SHA push CI；未发布 Release 时允许重打 tag；Release/CI 不缓存 `src-tauri/target`
- [x] `currentDocsContract` 类文档字符串测试已删除或大幅收缩；`docs-contract-check` / `task-docs check` 仍是文档生成物的单一权威
- [x] Claude 输入 `https://example.com/v1` 出现 `/v1/v1` 警告；`https://example.com` 不出现
- [x] `.tmp/` 被 gitignore；不再为文档模板增加契约测试
- [x] WorkBuddy / Codex / Claude / Grok Build / OpenCode 模型页可在保存前测连通；Qoder/TRAE 没有该按钮

## Notes

父任务只做集成验收与 spec 回写。实现落在四个子任务。
