# 历史文本（工作站路径已匿名化）：evidence/gemini-native/probe-finish.mjs

此文件为历史源码或运行证据，经工作站用户目录语义匿名化后以 Markdown 保存，不是运行入口。归档过程未执行其中内容。**本提交不再声称代码块与原稿字节相同。** 原稿可从来源提交 `6e6a3b3870a047874354f8f685b163622325331b` 的 `.trellis/tasks/archive/2026-09/09-22-session-cross-device-recovery/evidence/gemini-native/probe-finish.mjs` 恢复。

原路径、原稿与提交稿的哈希及恢复来源见[归档路径映射](research/archive-path-map.json)。

- 原稿字节数：1859
- 原稿 SHA-256：`725c9609a2dbdc7ab1c640c7993a271ee17996a2722326625b80b722ef5f179d`
- 匿名化载荷字节数：1859
- 匿名化载荷 SHA-256：`725c9609a2dbdc7ab1c640c7993a271ee17996a2722326625b80b722ef5f179d`

下方载荷的准确字节边界见路径映射；若载荷没有末尾换行，围栏前仅补展示换行。

```javascript
import fs from 'node:fs/promises';
import path from 'node:path';
import { GeminiChat, ChatRecordingService, loadConversationRecord, convertSessionToClientHistory } from '/opt/homebrew/Cellar/gemini-cli/0.46.0/libexec/lib/node_modules/@google/gemini-cli/bundle/chunk-RCJSF5RP.js';
const root=process.argv[2];
const results=[];
for(const finishReason of [undefined,'STOP']) {
 const label=finishReason??'missing';
 const dir=path.join(root,'finish-'+label);await fs.mkdir(dir,{recursive:true});
 const config={getProjectRoot:()=>root,storage:{getProjectTempDir:()=>dir},isContextManagementEnabled:()=>false,getHookSystem:()=>undefined};
 const context={config,promptId:`synthetic-${label}`};
 const recorder=new ChatRecordingService(context);await recorder.initialize();
 const chat=Object.create(GeminiChat.prototype);
 chat.context=context;chat.chatRecordingService=recorder;chat.agentHistory={push(){}};
 const chunk={candidates:[{content:{role:'model',parts:[{text:'SYNTHETIC_FINAL_LIKE'}]},...(finishReason?{finishReason}:{})}]};
 let error;
 try {for await(const _ of chat.processStreamResponse('synthetic-target',(async function*(){yield chunk})(),undefined)) {}}
 catch(e){error={name:e.name,message:e.message};}
 const raw=await loadConversationRecord(recorder.getConversationFilePath());
 results.push({label,error,recorded_messages:raw.messages});
}
const prefixCases=['/literal path','?literal question','<session_context>literal text','<hook_context>literal text','ordinary text'];
const prefixResult=prefixCases.map((text,i)=>({input:text,history:convertSessionToClientHistory([{id:`u${i}`,type:'user',timestamp:'2026-09-22T00:00:00Z',content:[{text}]}])}));
const out={finish_results:results,prefix_results:prefixResult};await fs.writeFile(path.join(root,'finish-result.json'),JSON.stringify(out,null,2));console.log(JSON.stringify(out,null,2));
```
