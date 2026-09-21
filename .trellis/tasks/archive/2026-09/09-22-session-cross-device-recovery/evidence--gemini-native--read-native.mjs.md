# 历史文本（工作站路径已匿名化）：evidence/gemini-native/read-native.mjs

此文件为历史源码或运行证据，经工作站用户目录语义匿名化后以 Markdown 保存，不是运行入口。归档过程未执行其中内容。**本提交不再声称代码块与原稿字节相同。** 原稿可从来源提交 `6e6a3b3870a047874354f8f685b163622325331b` 的 `.trellis/tasks/archive/2026-09/09-22-session-cross-device-recovery/evidence/gemini-native/read-native.mjs` 恢复。

原路径、原稿与提交稿的哈希及恢复来源见[归档路径映射](research/archive-path-map.json)。

- 原稿字节数：900
- 原稿 SHA-256：`dce3087116530d7cd5f0a557b98477580b350456d78c1a768c5219fe74159a08`
- 匿名化载荷字节数：900
- 匿名化载荷 SHA-256：`dce3087116530d7cd5f0a557b98477580b350456d78c1a768c5219fe74159a08`

下方载荷的准确字节边界见路径映射；若载荷没有末尾换行，围栏前仅补展示换行。

```javascript
import fs from 'node:fs/promises';
import { resolveSessionId } from '/opt/homebrew/Cellar/gemini-cli/0.46.0/libexec/lib/node_modules/@google/gemini-cli/bundle/gemini-YXO2QQ66.js';
import { loadConversationRecord, convertSessionToClientHistory } from '/opt/homebrew/Cellar/gemini-cli/0.46.0/libexec/lib/node_modules/@google/gemini-cli/bundle/chunk-RCJSF5RP.js';
const input=JSON.parse(await fs.readFile(process.argv[2],'utf8'));
let result;
if(input.op==='import') result=await resolveSessionId(undefined,undefined,input.path);
if(input.op==='resume') result=await resolveSessionId(input.sessionId,undefined,undefined);
if(input.op==='read') result={conversation:await loadConversationRecord(input.path)};
const conversation=result.resumedSessionData?.conversation ?? result.conversation;
result.clientHistory=convertSessionToClientHistory(conversation.messages);
console.log(JSON.stringify(result));
```
