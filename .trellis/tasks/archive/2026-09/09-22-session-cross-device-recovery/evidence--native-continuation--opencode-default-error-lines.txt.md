# 历史文本（工作站路径已匿名化）：evidence/native-continuation/opencode-default-error-lines.txt

此文件为历史源码或运行证据，经工作站用户目录语义匿名化后以 Markdown 保存，不是运行入口。归档过程未执行其中内容。**本提交不再声称代码块与原稿字节相同。** 原稿可从来源提交 `6e6a3b3870a047874354f8f685b163622325331b` 的 `.trellis/tasks/archive/2026-09/09-22-session-cross-device-recovery/evidence/native-continuation/opencode-default-error-lines.txt` 恢复。

原路径、原稿与提交稿的哈希及恢复来源见[归档路径映射](research/archive-path-map.json)。

- 原稿字节数：1351
- 原稿 SHA-256：`bd8ad845f24f8c671c67107828be1203bdb7f00a5dc39857af5e0a114542274f`
- 匿名化载荷字节数：1351
- 匿名化载荷 SHA-256：`bd8ad845f24f8c671c67107828be1203bdb7f00a5dc39857af5e0a114542274f`

下方载荷的准确字节边界见路径映射；若载荷没有末尾换行，围栏前仅补展示换行。

```text
timestamp=2026-09-21T18:46:31.966Z level=ERROR run=5309cfa6 message="share subscriber failed" type=message.updated cause="Cause([Fail(ProviderModelNotFoundError: Model not found: fyagent-migration/fyagent-migration.)])"
timestamp=2026-09-21T18:46:31.974Z level=ERROR run=5309cfa6 message=failed ref=err_1a26b763 error="ProviderModelNotFoundError: Model not found: fyagent-migration/fyagent-migration." cause="ProviderModelNotFoundError: Model not found: fyagent-migration/fyagent-migration.\n    at <anonymous> (/$bunfs/root/chunk-3yjk1w4m.js:439:94093)\n    at SessionPrompt.getModel (/$bunfs/root/chunk-jsvjjq7j.js:1142:11505)\n    at SessionPrompt.getModel (definition) (/$bunfs/root/chunk-jsvjjq7j.js:1142:908)\n    at SessionPrompt.run (/$bunfs/root/chunk-jsvjjq7j.js:1142:15339)\n    at SessionPrompt.run (definition) (/$bunfs/root/chunk-jsvjjq7j.js:1142:10503)\n    at SessionRunState.ensureRunning (/$bunfs/root/chunk-jsvjjq7j.js:1142:15308)\n    at SessionRunState.ensureRunning (definition) (/$bunfs/root/chunk-jsvjjq7j.js:2:8188)\n    at SessionPrompt.loop (/$bunfs/root/chunk-jsvjjq7j.js:1142:10225)\n    at SessionPrompt.loop (definition) (/$bunfs/root/chunk-jsvjjq7j.js:1142:15244)\n    at SessionPrompt.prompt (/$bunfs/root/chunk-mjh4q85p.js:4:13297)\n    at SessionPrompt.prompt (definition) (/$bunfs/root/chunk-jsvjjq7j.js:1142:9844)"
```
