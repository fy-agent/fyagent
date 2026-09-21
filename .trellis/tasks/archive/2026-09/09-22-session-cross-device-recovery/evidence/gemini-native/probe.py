import http.server,threading,subprocess,json,uuid,os
from pathlib import Path
R=Path(__file__).parent
captured=[]
class Handler(http.server.BaseHTTPRequestHandler):
 def do_POST(self):
  raw=self.rfile.read(int(self.headers.get('Content-Length','0')))
  try: body=json.loads(raw)
  except: body={'unparsed':raw.decode(errors='replace')}
  captured.append({'path':self.path,'body':body})
  if ':countTokens' in self.path:
   self.send_response(200);self.send_header('Content-Type','application/json');self.end_headers();self.wfile.write(b'{"totalTokens":100}')
  else:
   self.send_response(400);self.send_header('Content-Type','application/json');self.end_headers();self.wfile.write(b'{"error":{"code":400,"message":"synthetic capture complete; no inference","status":"INVALID_ARGUMENT"}}')
 def do_CONNECT(self):self.send_error(403)
 def log_message(self,*args):pass
server=http.server.ThreadingHTTPServer(('127.0.0.1',0),Handler)
threading.Thread(target=server.serve_forever,daemon=True).start()
env=json.loads((R/'env.json').read_text());env.update(GEMINI_API_KEY='synthetic-not-a-key',GOOGLE_GEMINI_BASE_URL=f'http://127.0.0.1:{server.server_port}',GEMINI_CLI_NO_RELAUNCH='true')
settings=Path(env['GEMINI_CLI_HOME'])/'.gemini/settings.json';settings.parent.mkdir(exist_ok=True)
settings.write_text(json.dumps({'security':{'auth':{'selectedType':'gemini-api-key'}},'model':{'name':'gemini-2.5-flash'},'privacy':{'usageStatisticsEnabled':False},'general':{'disableAutoUpdate':True},'telemetry':{'enabled':False}}))
fixture=json.loads((R/'fixture.json').read_text());fixture['summary']='Synthetic recovery';(R/'fixture-cli.json').write_text(json.dumps(fixture,ensure_ascii=False))
def run(label,args):
 p=subprocess.run(['rtk','proxy','/opt/homebrew/bin/gemini','--skip-trust',*args],env=env,cwd=R/'project',capture_output=True,text=True,timeout=40)
 (R/(label+'.stdout')).write_text(p.stdout);(R/(label+'.stderr')).write_text(p.stderr)
 print(label,p.returncode,p.stdout[:1200],p.stderr[:1500],flush=True)
 return {'exit_code':p.returncode,'stdout':p.stdout,'stderr':p.stderr}
results={}
try:
 results['import_list']=run('cli-import-list',['--session-file',str(R/'fixture-cli.json'),'--list-sessions'])
 files=sorted((Path(env['GEMINI_CLI_HOME'])/'.gemini/tmp').rglob('session-*.jsonl'),key=lambda p:p.stat().st_mtime)
 imported=json.loads(files[-1].read_text().splitlines()[0]);sid=imported['sessionId'];results['session_id']=sid;results['native_path']=str(files[-1])
 results['restart_list']=run('cli-restart-list',['--list-sessions'])
 start=len(captured)
 results['continue']=run('cli-continue',['--resume',sid,'--prompt','Synthetic next turn. Keep KITE-73.','--output-format','stream-json','--model','gemini-2.5-flash'])
 results['next_turn_capture_count']=len(captured)-start
except Exception as e:results['probe_error']=repr(e)
results['captured_requests']=captured
(R/'cli-results.json').write_text(json.dumps(results,ensure_ascii=False,indent=2))
server.shutdown()
