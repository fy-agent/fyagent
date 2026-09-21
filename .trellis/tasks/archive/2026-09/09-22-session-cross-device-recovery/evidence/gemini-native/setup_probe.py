"""Prepare only synthetic storage alongside this script; never use real Gemini home."""
from pathlib import Path
import os,json,uuid
r=Path(__file__).parent.resolve()
env={k:v for k,v in os.environ.items() if k in ['PATH','LANG','TERM']}
for key,name in [('HOME','home'),('GEMINI_CLI_HOME','gemini'),('XDG_CONFIG_HOME','config'),('XDG_DATA_HOME','data'),('XDG_CACHE_HOME','cache'),('TMPDIR','tmp')]:
 p=r/name;p.mkdir(exist_ok=True);env[key]=str(p)
(r/'project').mkdir(exist_ok=True)
env.update(GEMINI_CLI_NO_RELAUNCH='true',NO_COLOR='1',HTTP_PROXY='http://127.0.0.1:1',HTTPS_PROXY='http://127.0.0.1:1',ALL_PROXY='http://127.0.0.1:1',NO_PROXY='127.0.0.1,localhost')
(r/'env.json').write_text(json.dumps(env,indent=2))
now='2026-09-22T00:00:00.000Z'
texts=[('user','  Unanswered one\r\n'),('user','Keep KITE-73 中文'),('gemini','One final\r\n  '),('gemini','Second final C:\\foo'),('user','Trailing unanswered')]
x={'sessionId':str(uuid.uuid4()),'projectHash':'synthetic-source','startTime':now,'lastUpdated':now,'messages':[{'id':str(uuid.uuid4()),'type':role,'timestamp':now,'content':[{'text':text}]} for role,text in texts]}
(r/'fixture.json').write_text(json.dumps(x,ensure_ascii=False))
