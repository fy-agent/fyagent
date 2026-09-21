# 历史文本（工作站路径已匿名化）：evidence/browser-final-review/counterexample.log

此文件为历史源码或运行证据，经工作站用户目录语义匿名化后以 Markdown 保存，不是运行入口。归档过程未执行其中内容。**本提交不再声称代码块与原稿字节相同。** 该日志先前未被 Git 跟踪，原稿不能从来源提交恢复；原始字节单独保留在仓库外的本地 archive-original-logs 备份中，不纳入 PR。

原路径、原稿与提交稿的哈希及恢复来源见[归档路径映射](research/archive-path-map.json)。

- 原稿字节数：1097
- 原稿 SHA-256：`efbfbfb830e26689720006747bc3a4539fde383e885a39cdb486fa0dd7a634b3`
- 匿名化载荷字节数：1097
- 匿名化载荷 SHA-256：`efbfbfb830e26689720006747bc3a4539fde383e885a39cdb486fa0dd7a634b3`

下方载荷的准确字节边界见路径映射；若载荷没有末尾换行，围栏前仅补展示换行。

```text
[WebServer] (node:56044) Warning: The 'NO_COLOR' env is ignored due to the 'FORCE_COLOR' env being set.
[WebServer] (Use `pnpm --trace-warnings ...` to show where the warning was created)
[WebServer] (node:56057) Warning: The 'NO_COLOR' env is ignored due to the 'FORCE_COLOR' env being set.
[WebServer] (Use `node --trace-warnings ...` to show where the warning was created)

Running 1 test using 1 worker

(node:56065) Warning: The 'NO_COLOR' env is ignored due to the 'FORCE_COLOR' env being set.
(Use `node --trace-warnings ...` to show where the warning was created)
PLACEHOLDER_BEFORE_RELEASE {"height":23.59375,"scrollHeight":24,"text":"还没有找到已配置的模型 ID"}
DELAYED_CONTENT_COUNTEREXAMPLE {"before":{"height":23.59375,"scrollHeight":24,"text":"还没有找到已配置的模型 ID"},"after":{"height":124.984375,"scrollHeight":125,"text":"搜索已有模型existing1existing-model"}}
  ✓  1 [chromium-1440x900] › ../browser-final-review/diagnostic.spec.ts:390:1 › controlled delayed IDs distinguish placeholder height from populated height (1.9s)

  1 passed (3.2s)
```
