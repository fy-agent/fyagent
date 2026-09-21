# 归档结构与工作站路径规范化

本次仅整理本任务归档目录，以符合既有归档结构与工作站路径隐私契约；未修改产品代码或检查器，未执行历史脚本。来源快照提交为 `6e6a3b3870a047874354f8f685b163622325331b`。

## 原稿与提交稿

保留全部 138 份原文件的去向和原始 SHA-256。133 份原稿可从来源提交及记录的原路径恢复；5 份原先被忽略的测试/诊断日志从未进入该提交，其原始字节已单独备份到仓库外的本地 `work/archive-original-logs/`，不纳入 PR。不得把这 5 份日志声称为 Git 来源。

提交材料中的具体用户目录均改为语义占位符。24 份历史脚本、HTML、日志及文本保存在不可执行的代码围栏中，并分别记录原稿 SHA-256、匿名化载荷 SHA-256、提交文档 SHA-256 与对应字节数。**先前“task.json/JSON/围栏载荷在提交中保持原字节”的说法已 superseded；本次提交稿不再声称原样字节。** 原稿哈希和提交稿哈希是不同证据。

嵌套 Markdown 已平铺，JSON 已迁入直属 research 目录。task.json 的 ID、状态、分支、提交与 PR 字段语义，以及各报告的时间、结论和先后关系保持不变。7 个相对链接已修复，8 个历史绝对路径链接作为匿名化的出处保留。完整对应关系见[归档路径映射](research/archive-path-map.json)。

## 交付入口

- [交付说明](implementation--delivery-readiness.md)
- [证据索引](implementation--evidence-index.md)
- [最终 Rust 验证](implementation--final-rust-validation.md)
- [最终前端验证](implementation--final-frontend-validation.md)

## 验证

结构整理后，平台检查通过 3,141 文件。首次归档后契约检查有 663 passed / 1 failed / 1 existing skipped，唯一失败是具体工作站路径；该失败触发本次明确授权的语义匿名化，没有放宽或绕过检查。

匿名化后，`mise run supported-platform:check` 再次通过 3,141 文件；不带任何临时排除的 `mise run check:contracts` 最终 exit 0，35 个契约测试文件全部通过（664 passed / 1 existing skipped），native-fetch 4/4 通过。133 份 Git 原稿、5 份仓库外原始日志、138 份匿名化提交文档和 24 份匿名化围栏载荷均已按映射中的独立字节数与 SHA-256 核验。只暂存本归档目录，没有产品源码改动、检查器变更、提交或推送；交由协调者运行最终全量检查并提交。
