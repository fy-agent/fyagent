# 历史文本（工作站路径已匿名化）：evidence/codex-native/probe.py

此文件为历史源码或运行证据，经工作站用户目录语义匿名化后以 Markdown 保存，不是运行入口。归档过程未执行其中内容。**本提交不再声称代码块与原稿字节相同。** 原稿可从来源提交 `6e6a3b3870a047874354f8f685b163622325331b` 的 `.trellis/tasks/archive/2026-09/09-22-session-cross-device-recovery/evidence/codex-native/probe.py` 恢复。

原路径、原稿与提交稿的哈希及恢复来源见[归档路径映射](research/archive-path-map.json)。

- 原稿字节数：4024
- 原稿 SHA-256：`5fc79c972265665480886b676a86a3be0c001cf036d1d0d0a4f2915db792e65b`
- 匿名化载荷字节数：4024
- 匿名化载荷 SHA-256：`5fc79c972265665480886b676a86a3be0c001cf036d1d0d0a4f2915db792e65b`

下方载荷的准确字节边界见路径映射；若载荷没有末尾换行，围栏前仅补展示换行。

```python
import json,os,subprocess,queue,threading,time
from pathlib import Path
base=Path(__file__).resolve().parent
home=base/'isolated-home';home.mkdir(exist_ok=True)
workspace=base/'workspace';workspace.mkdir(exist_ok=True)
log=[]
class RPC:
 def __init__(self):
  env=os.environ.copy();env['CODEX_HOME']=str(home)
  self.err=open(base/'server-stderr.log','a')
  self.p=subprocess.Popen(['codex','app-server','--stdio'],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=self.err,text=True,env=env,cwd=workspace)
  self.q=queue.Queue();self.i=0
  def read():
   for line in self.p.stdout:
    try:self.q.put(json.loads(line))
    except:pass
  threading.Thread(target=read,daemon=True).start()
  self.call('initialize',{'clientInfo':{'name':'fyagent_synthetic_probe','version':'0.1'},'capabilities':{'experimentalApi':True}})
  self.send({'method':'initialized','params':{}})
 def send(self,msg):self.p.stdin.write(json.dumps(msg)+'\n');self.p.stdin.flush()
 def call(self,method,params):
  self.i+=1;rid=self.i;self.send({'id':rid,'method':method,'params':params});end=time.time()+35
  while time.time()<end:
   try:r=self.q.get(timeout=1)
   except queue.Empty:continue
   log.append({'method_called':method,'response':r})
   if r.get('id')==rid:
    if 'error' in r:raise RuntimeError(json.dumps(r))
    return r['result']
  raise TimeoutError(method)
 def close(self):
  self.p.terminate()
  try:self.p.wait(timeout=5)
  except subprocess.TimeoutExpired:self.p.kill();self.p.wait()
  self.err.close()
import uuid, datetime
now=datetime.datetime.now(datetime.timezone.utc);stamp=now.isoformat().replace("+00:00","Z");tid=str(uuid.uuid4())
root=home/'sessions'/now.strftime('%Y/%m/%d');root.mkdir(parents=True,exist_ok=True)
path=root/f"rollout-{now.strftime('%Y-%m-%dT%H-%M-%S')}-{tid}.jsonl"
records=[]
def row(kind,payload):records.append({'timestamp':stamp,'type':kind,'payload':payload})
row('session_meta',{'session_id':tid,'id':tid,'timestamp':stamp,'cwd':str(workspace),'originator':'fyagent','cli_version':'0.154.0','source':'cli','history_mode':'legacy'})
messages=[('user','合成档案：纸鹤的编号是 KITE-728。'),('assistant','已记录纸鹤编号 KITE-728。'),('user','约定颜色为琥珀色。'),('assistant','约定颜色：琥珀色。')]
turn=None
for index,(role,body) in enumerate(messages):
 if role=='user':
  if turn:row('event_msg',{'type':'task_complete','turn_id':turn,'last_agent_message':None})
  turn=f'fyagent-import-{index}'
  row('event_msg',{'type':'task_started','turn_id':turn,'model_context_window':None,'collaboration_mode_kind':'default'})
  row('event_msg',{'type':'user_message','message':body,'images':[],'local_images':[]})
 else:row('event_msg',{'type':'agent_message','message':body,'phase':'final_answer'})
 item={'type':'message','role':role,'content':[{'type':'input_text' if role=='user' else 'output_text','text':body}], 'internal_chat_message_metadata_passthrough':{'turn_id':turn,'content_item_kinds':['user.text' if role=='user' else 'assistant.final_answer']}}
 if role=='assistant':item['phase']='final_answer'
 row('response_item',item)
row('event_msg',{'type':'task_complete','turn_id':turn,'last_agent_message':messages[-1][1]})
path.write_text(''.join(json.dumps(x,ensure_ascii=False)+'\n' for x in records))
result={'thread_id':tid,'native_file':str(path),'synthetic_only':True,'model_called':False};r=None
try:
 r=RPC();result['read']=r.call('thread/read',{'threadId':tid,'includeTurns':True})
 result['resume']=r.call('thread/resume',{'threadId':tid,'cwd':str(workspace),'approvalPolicy':'never','sandbox':'read-only'})
 r.close();r=RPC();result['restart_read']=r.call('thread/read',{'threadId':tid,'includeTurns':True});result['status']='completed'
except Exception as e:result.update(status='failed',error=str(e))
finally:
 if r:r.close()
 (base/'result.json').write_text(json.dumps(result,ensure_ascii=False,indent=2));(base/'rpc-log.json').write_text(json.dumps(log,ensure_ascii=False,indent=2))
 print(json.dumps(result,ensure_ascii=False)[:16000])
```
