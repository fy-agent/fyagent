# 历史文本（工作站路径已匿名化）：evidence/codex-native/capture.py

此文件为历史源码或运行证据，经工作站用户目录语义匿名化后以 Markdown 保存，不是运行入口。归档过程未执行其中内容。**本提交不再声称代码块与原稿字节相同。** 原稿可从来源提交 `6e6a3b3870a047874354f8f685b163622325331b` 的 `.trellis/tasks/archive/2026-09/09-22-session-cross-device-recovery/evidence/codex-native/capture.py` 恢复。

原路径、原稿与提交稿的哈希及恢复来源见[归档路径映射](research/archive-path-map.json)。

- 原稿字节数：3925
- 原稿 SHA-256：`03f35ce68ff0b89d98c9a603def695ad06f5104827ba507c3834b1dd8c9dcac1`
- 匿名化载荷字节数：3925
- 匿名化载荷 SHA-256：`03f35ce68ff0b89d98c9a603def695ad06f5104827ba507c3834b1dd8c9dcac1`

下方载荷的准确字节边界见路径映射；若载荷没有末尾换行，围栏前仅补展示换行。

```python
import json,os,subprocess,queue,threading,time
from pathlib import Path
base=Path(__file__).resolve().parent
home=base/'isolated-home';home.mkdir(exist_ok=True)
workspace=base/'workspace';workspace.mkdir(exist_ok=True)
log=[]
from http.server import BaseHTTPRequestHandler,ThreadingHTTPServer
captured=[]
class Handler(BaseHTTPRequestHandler):
 def do_POST(self):
  data=json.loads(self.rfile.read(int(self.headers.get('Content-Length','0'))))
  captured.append(data)
  self.send_response(400);self.send_header('Content-Type','application/json');self.end_headers()
  self.wfile.write(b'{"error":{"message":"Intentional synthetic capture; no inference performed"}}')
 def log_message(self,*args):pass
server=ThreadingHTTPServer(('127.0.0.1',0),Handler)
threading.Thread(target=server.serve_forever,daemon=True).start()
(home/'config.toml').write_text('model_provider = "probe"\nmodel = "gpt-6-astra"\n[model_providers.probe]\nname = "Local synthetic request capture"\nbase_url = "http://127.0.0.1:'+str(server.server_port)+'/v1"\nwire_api = "responses"\nrequires_openai_auth = false\nrequest_max_retries = 0\nstream_max_retries = 0\n')
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
result={'synthetic_only':True,'model_called':False};r=None
try:
 tid=json.loads((base/'result.json').read_text())['thread_id'];r=RPC()
 result['resume']=r.call('thread/resume',{'threadId':tid,'cwd':str(workspace),'approvalPolicy':'never','sandbox':'read-only','modelProvider':'probe','model':'gpt-6-astra'})
 result['turn_start']=r.call('turn/start',{'threadId':tid,'input':[{'type':'text','text':'我们之前约定的编号与颜色是什么？','text_elements':[]}]})
 deadline=time.time()+20
 while time.time()<deadline and not captured:time.sleep(.1)
 expected=[('user','合成档案：纸鹤的编号是 KITE-728。'),('assistant','已记录纸鹤编号 KITE-728。'),('user','约定颜色为琥珀色。'),('assistant','约定颜色：琥珀色。')]
 found=[]
 for item in captured[0].get('input',[]) if captured else []:
  for content in item.get('content',[]):
   text=content.get('text','')
   if (item.get('role'),text) in expected:found.append((item['role'],text))
 result['captured_count']=len(captured);result['history_role_order_exact']=found==expected;result['status']='completed';assert found==expected
except Exception as e:result.update(status='failed',error=str(e))
finally:
 if r:r.close()
 server.shutdown()
 (base/'capture-result.json').write_text(json.dumps(result,ensure_ascii=False,indent=2));(base/'capture-wire.json').write_text(json.dumps(captured,ensure_ascii=False,indent=2))
 print(json.dumps({k:v for k,v in result.items() if k not in ['resume','turn_start']},ensure_ascii=False))
```
