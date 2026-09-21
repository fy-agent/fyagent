# 历史文本（工作站路径已匿名化）：evidence/native-continuation/hermes-import-help.txt

此文件为历史源码或运行证据，经工作站用户目录语义匿名化后以 Markdown 保存，不是运行入口。归档过程未执行其中内容。**本提交不再声称代码块与原稿字节相同。** 原稿可从来源提交 `6e6a3b3870a047874354f8f685b163622325331b` 的 `.trellis/tasks/archive/2026-09/09-22-session-cross-device-recovery/evidence/native-continuation/hermes-import-help.txt` 恢复。

原路径、原稿与提交稿的哈希及恢复来源见[归档路径映射](research/archive-path-map.json)。

- 原稿字节数：589
- 原稿 SHA-256：`686a0a138671f7396d5a11c5bcb4a5bcecd53b4d7843bca0a71c157d240e41b7`
- 匿名化载荷字节数：589
- 匿名化载荷 SHA-256：`686a0a138671f7396d5a11c5bcb4a5bcecd53b4d7843bca0a71c157d240e41b7`

下方载荷的准确字节边界见路径映射；若载荷没有末尾换行，围栏前仅补展示换行。

```text
usage: hermes sessions import [-h] [--from {claude,codex}] [path]

Pull a conversation started in Claude Code (~/.claude/projects) or Codex CLI
(~/.codex/sessions) into the Hermes session store so it can be resumed with
'hermes --resume <id>'. The foreign files are only read, never modified.

positional arguments:
  path                  Path to a specific session JSONL file (skips the
                        picker)

options:
  -h, --help            show this help message and exit
  --from {claude,codex}
                        Which tool to import from (default: pick across both)
```
