# 历史文本（工作站路径已匿名化）：evidence/native-continuation/create-only/verify_create_only.py

此文件为历史源码或运行证据，经工作站用户目录语义匿名化后以 Markdown 保存，不是运行入口。归档过程未执行其中内容。**本提交不再声称代码块与原稿字节相同。** 原稿可从来源提交 `6e6a3b3870a047874354f8f685b163622325331b` 的 `.trellis/tasks/archive/2026-09/09-22-session-cross-device-recovery/evidence/native-continuation/create-only/verify_create_only.py` 恢复。

原路径、原稿与提交稿的哈希及恢复来源见[归档路径映射](research/archive-path-map.json)。

- 原稿字节数：6755
- 原稿 SHA-256：`f8bed46241f6935e30c3cf01e682dafc1ea9c46dac486d385db6124c4032b87b`
- 匿名化载荷字节数：6754
- 匿名化载荷 SHA-256：`7b5e59d2138aa496d2a5c0002daccdc6296303960fd3c2f6f47188c90082938d`

下方载荷的准确字节边界见路径映射；若载荷没有末尾换行，围栏前仅补展示换行。

```python
"""Actual Rust writer -> official native export -> loopback-only continuation.
The ignored Rust test is built by canonical mise rust:test first. No inference.
"""
import http.server, json, os, subprocess, threading, uuid, hashlib, sys
from pathlib import Path
ROOT = Path(__file__).parent / ('create-only-' + uuid.uuid4().hex[:10])
ROOT.mkdir()
REPO = ROOT.parent.parent / 'fyagent-session'
CLI = '/Users/<username>/.opencode/bin/opencode'
CAPTURES = []
class Handler(http.server.BaseHTTPRequestHandler):
    def do_POST(self):
        body = json.loads(self.rfile.read(int(self.headers.get('Content-Length', 0))))
        CAPTURES.append({'path':self.path,'body':body})
        self.send_response(400)
        self.send_header('Content-Type','application/json')
        self.end_headers()
        self.wfile.write(b'{"error":{"type":"invalid_request_error","message":"loopback capture only; no inference"}}')
    def log_message(self,*args): pass
server = http.server.ThreadingHTTPServer(('127.0.0.1',0), Handler)
threading.Thread(target=server.serve_forever,daemon=True).start()
env={k:os.environ[k] for k in ['PATH','LANG'] if k in os.environ}
for key,folder in [('HOME','home'),('TMPDIR','tmp'),('XDG_CONFIG_HOME','config'),('XDG_DATA_HOME','data'),('XDG_CACHE_HOME','cache'),('XDG_STATE_HOME','state')]:
    path=ROOT/folder;path.mkdir();env[key]=str(path)
env.update(FYAGENT_TEST_HOME=env['HOME'],OPENCODE_TEST_HOME=env['HOME'],FYAGENT_OPENCODE_PROBE_ROOT=str(ROOT),
    OPENCODE_DISABLE_MODELS_FETCH='true',OPENCODE_DISABLE_AUTOUPDATE='true',OPENCODE_DISABLE_CLAUDE_CODE='true',
    HTTP_PROXY='http://127.0.0.1:1',HTTPS_PROXY='http://127.0.0.1:1',ALL_PROXY='http://127.0.0.1:1',NO_PROXY='127.0.0.1,localhost',
    NO_COLOR='1',TERM='dumb',GIT_CONFIG_GLOBAL='/dev/null',GIT_CONFIG_NOSYSTEM='1')
config=ROOT/'home/.config/opencode/opencode.json';config.parent.mkdir(parents=True)
config.write_text(json.dumps({'model':'probe/local','small_model':'probe/local','enabled_providers':['probe'],
    'provider':{'probe':{'npm':'@ai-sdk/openai-compatible','name':'Loopback only','options':{'baseURL':f'http://127.0.0.1:{server.server_port}/v1','apiKey':'synthetic-not-a-key'},
    'models':{'local':{'name':'Synthetic capture','limit':{'context':100000,'output':1024}}}}}}))
env.update(OPENCODE_CONFIG=str(config),OPENCODE_CONFIG_DIR=str(config.parent),OPENCODE_DB=str(ROOT/'data/opencode.db'))
binpath=ROOT/'home/.local/bin';binpath.mkdir(parents=True);(binpath/'opencode').symlink_to(CLI)
project=ROOT/'project';project.mkdir()
bootstrap=ROOT/'bootstrap';bootstrap.mkdir()
def run(args,cwd=project,timeout=60):
    result=subprocess.run(['rtk','proxy',*map(str,args)],cwd=cwd,env=env,text=True,capture_output=True,timeout=timeout)
    return {'exit_code':result.returncode,'stdout':result.stdout,'stderr':result.stderr[-10000:]}
def must(args,cwd=project):
    result=run(args,cwd);assert result['exit_code']==0,result;return result
must(['git','init','-q'])
(project/'README.md').write_text('Synthetic workspace. No real history.\n')
must(['git','add','README.md'])
must(['git','-c','user.name=Synthetic Probe','-c','user.email=probe@example.invalid','-c','core.hooksPath=/dev/null','commit','-qm','Synthetic isolated workspace'])
version=must([CLI,'--version'],bootstrap);assert version['stdout'].strip()=='1.18.30'
# Bootstrap only a non-Git store, leaving this project's native row absent.
must([CLI,'debug','config','--pure'],bootstrap)
# debug config may defer opening storage; db path initializes the same official database.
if not Path(env['OPENCODE_DB']).exists():
    must([CLI,'db','path'],bootstrap)
executables=[p for p in (REPO/'src-tauri/target/aarch64-apple-darwin/debug/deps').glob('fyagent_lib-*') if p.is_file() and os.access(p,os.X_OK) and not p.suffix]
if not executables:
    executables=[p for p in (REPO/'target/aarch64-apple-darwin/debug/deps').glob('fyagent_lib-*') if p.is_file() and os.access(p,os.X_OK) and not p.suffix]
assert executables,'canonical test executable missing'
executable=max(executables,key=lambda p:p.stat().st_mtime)
test=run([executable,'--ignored','--exact','session_manager::migrate::native::opencode::tests::isolated_actual_writer_create_only_probe','--nocapture'],timeout=180)
(ROOT/'rust-test.json').write_text(json.dumps(test,ensure_ascii=False,indent=2))
print('ROOT',ROOT,flush=True);print(test['stdout'],test['stderr'],flush=True)
assert test['exit_code']==0,test
result=json.loads((ROOT/'rust-writer-result.json').read_text())
# A fresh official process opens the target store; this is the writer's actual artifact.
exported=must([CLI,'export','--pure',result['native_id']])
body=json.loads(exported['stdout']);(ROOT/'official-export.json').write_text(json.dumps(body,ensure_ascii=False,indent=2))
expected=[('user' if m['kind']=='user_text' else 'assistant',m['text']) for m in result['messages']]
observed=[(m['info']['role'],m['parts'][0]['text']) for m in body['messages']]
# Rust uses serde camelCase kinds; identify user enum explicitly below.
expected=[('user' if m['kind'] in ('user_text','userText') else 'assistant',m['text']) for m in result['messages']]
assert observed==expected,(observed,expected)
continuation=run([CLI,'run','--pure','--format','json','--session',result['native_id'],'Synthetic next turn; keep KITE-73.'],timeout=60)
(ROOT/'continuation.json').write_text(json.dumps(continuation,ensure_ascii=False,indent=2))
assert CAPTURES,'no next request captured'
projected=[]
for item in CAPTURES:
    messages=[m for m in item['body'].get('messages',[]) if m.get('role') in ('user','assistant')]
    projected.append({'path':item['path'],'model':item['body'].get('model'),'messages':messages})
(ROOT/'captured-requests.json').write_text(json.dumps(projected,ensure_ascii=False,indent=2))
def text_content(m):
    content=m.get('content','')
    return content if isinstance(content,str) else ''.join(p.get('text','') for p in content if p.get('type')=='text')
assert any([(m['role'],text_content(m)) for m in r['messages']][:len(expected)]==expected for r in projected),projected
assert all(r['model']=='local' for r in projected)
remaining=list((ROOT/'tmp').glob('fyagent-opencode-*'));assert not remaining,remaining
summary={**result,'cli_version':'1.18.30','rust_test_executable':str(executable),'rust_test':'passed',
    'official_export_new_process':'exact message roles/order/text', 'bare_resume_target_model':'probe/local',
    'mock':'loopback request captured and HTTP 400 rejected; no generated answer',
    'temp_store_cleanup':True,'test_real_history_or_credentials':False,'request_count':len(projected)}
(ROOT/'summary.json').write_text(json.dumps(summary,ensure_ascii=False,indent=2))
server.shutdown();print(json.dumps(summary,ensure_ascii=False,indent=2))
```
