# 历史文本（工作站路径已匿名化）：evidence/native-continuation/probe_opencode.py

此文件为历史源码或运行证据，经工作站用户目录语义匿名化后以 Markdown 保存，不是运行入口。归档过程未执行其中内容。**本提交不再声称代码块与原稿字节相同。** 原稿可从来源提交 `6e6a3b3870a047874354f8f685b163622325331b` 的 `.trellis/tasks/archive/2026-09/09-22-session-cross-device-recovery/evidence/native-continuation/probe_opencode.py` 恢复。

原路径、原稿与提交稿的哈希及恢复来源见[归档路径映射](research/archive-path-map.json)。

- 原稿字节数：4609
- 原稿 SHA-256：`532db79e482fdd6c9ebd779d544fd783c5cbfaf0529d1f1333ed97f94f0983b2`
- 匿名化载荷字节数：4608
- 匿名化载荷 SHA-256：`e9eee5cc923150ff33fca38f719607b6a4677439f7983f5717e64173e0782522`

下方载荷的准确字节边界见路径映射；若载荷没有末尾换行，围栏前仅补展示换行。

```python
import http.server, threading, subprocess, json, os, uuid, time
from pathlib import Path

ROOT=Path(__file__).parent
CLI='/Users/<username>/.opencode/bin/opencode'
captured=[]
class Handler(http.server.BaseHTTPRequestHandler):
    def do_POST(self):
        body=json.loads(self.rfile.read(int(self.headers['Content-Length'])))
        captured.append({'path':self.path,'body':body})
        self.send_response(400);self.send_header('Content-Type','application/json');self.end_headers()
        self.wfile.write(b'{"error":{"message":"synthetic capture complete; no inference","type":"invalid_request_error"}}')
    def log_message(self,*args):pass

server=http.server.ThreadingHTTPServer(('127.0.0.1',0),Handler)
threading.Thread(target=server.serve_forever,daemon=True).start()
texts=[('user','  Unanswered one\r\n'),('user','Keep KITE-73 中文'),('assistant','One final\r\n  '),('assistant','Second final C:\\foo'),('user','Trailing unanswered')]
results=[]
for label,explicit in [('default',False),('explicit-local',True),('target-local-header',False)]:
    root=ROOT/('opencode-'+label);root.mkdir(exist_ok=True)
    env=json.loads((ROOT/'env.json').read_text())
    for k,d in [('HOME','home'),('XDG_DATA_HOME','data'),('XDG_CONFIG_HOME','config'),('XDG_CACHE_HOME','cache'),('XDG_STATE_HOME','state'),('OPENCODE_TEST_HOME','home')]:
        p=root/d;p.mkdir(exist_ok=True);env[k]=str(p)
    env.update(HTTP_PROXY='http://127.0.0.1:1',HTTPS_PROXY='http://127.0.0.1:1',ALL_PROXY='http://127.0.0.1:1',NO_PROXY='127.0.0.1,localhost')
    cfg={'model':'probe/local','small_model':'probe/local','enabled_providers':['probe'],'provider':{'probe':{'npm':'@ai-sdk/openai-compatible','name':'Loopback only','options':{'baseURL':f'http://127.0.0.1:{server.server_port}/v1','apiKey':'synthetic-not-a-key'},'models':{'local':{'name':'Synthetic Capture','limit':{'context':100000,'output':1024}}}}}}
    config=root/'opencode.json';config.write_text(json.dumps(cfg));env['OPENCODE_CONFIG']=str(config)
    sid='ses_'+uuid.uuid4().hex; messages=[];parent=None
    for i,(role,body) in enumerate(texts):
        mid=f'msg_{i:04d}_'+uuid.uuid4().hex
        info={'id':mid,'sessionID':sid,'role':role,'time':{'created':1789980000000+i},'agent':'build'}
        if role=='user':
            parent=mid;info['model']={'providerID':'fyagent-migration','modelID':'fyagent-migration'}
        else:
            info.update(parentID=parent,modelID='fyagent-migration',providerID='fyagent-migration',mode='build',path={'cwd':str(root),'root':str(root)},cost=0,tokens={'input':0,'output':0,'reasoning':0,'cache':{'read':0,'write':0}},finish='stop')
            info['time']['completed']=1789980000000+i
        messages.append({'info':info,'parts':[{'id':'prt_'+uuid.uuid4().hex,'sessionID':sid,'messageID':mid,'type':'text','text':body}]})
    fixture={'info':{'id':sid,'slug':'synthetic-recovery','projectID':'global','directory':str(root),'title':'Synthetic Session Recovery','version':'1.18.30','time':{'created':1789980000000,'updated':1789980000005}},'messages':messages}
    if label=='target-local-header': fixture['info']['model']={'providerID':'probe','id':'local'}
    path=root/'fixture.json';path.write_text(json.dumps(fixture))
    def run(args):
        p=subprocess.run(['rtk','proxy',CLI,*args],env=env,cwd=root,text=True,capture_output=True,timeout=50)
        return {'exit_code':p.returncode,'stdout':p.stdout,'stderr':p.stderr[-10000:]}
    result={'label':label,'session_id':sid,'input':texts,'import':run(['import','--pure',str(path)])}
    result['export']=run(['export','--pure',sid])
    args=['run','--pure','--format','json','--session',sid]
    if explicit:args+=['--model','probe/local']
    args+=['Synthetic next turn. Return only KITE-73.']
    start=len(captured)
    try:result['continue']=run(args)
    except subprocess.TimeoutExpired as e:result['continue']={'timeout':True,'stdout':str(e.stdout),'stderr':str(e.stderr)}
    result['captured_requests']=[{'path':x['path'],'body':{'model':x['body'].get('model'),'messages':[m for m in x['body'].get('messages',[]) if m.get('role') in ('user','assistant')]},'note':'Target runtime system prompt and tool schemas omitted from evidence'} for x in captured[start:]]
    (ROOT/('opencode-'+label+'-result.json')).write_text(json.dumps(result,ensure_ascii=False,indent=2))
    results.append({'label':label,'import_exit':result['import']['exit_code'],'export_exit':result['export']['exit_code'],'continue':result['continue'],'requests':len(result['captured_requests'])})
server.shutdown()
print(json.dumps(results,ensure_ascii=False,indent=2))
```
