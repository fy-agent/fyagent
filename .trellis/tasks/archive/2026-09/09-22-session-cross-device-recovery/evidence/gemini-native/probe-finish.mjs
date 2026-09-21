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
