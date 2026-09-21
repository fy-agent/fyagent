import os,json,uuid,subprocess
from pathlib import Path
ROOT=Path(__file__).parent
os.environ['HERMES_HOME']=str(ROOT/'hermes-sdk')
Path(os.environ['HERMES_HOME']).mkdir(exist_ok=True)
from hermes_state import SessionDB
texts=[('user',' First\r\n\n'),('user','Second'),('assistant',' Third  '),('assistant','Fourth'),('user','Unanswered')]
sid='fyagent_'+uuid.uuid4().hex
db=SessionDB()
db.create_session(sid,source='cli',cwd=str(ROOT))
for role,text in texts:db.append_message(sid,role,text)
model,display=db.get_resume_conversations(sid)
db.close()
env=json.loads((ROOT/'env.json').read_text());env['HERMES_HOME']=os.environ['HERMES_HOME']
p=subprocess.run(['rtk','proxy','/Users/serendipity/.local/bin/hermes','sessions','export','-','--session-id',sid],env=env,cwd=ROOT,text=True,capture_output=True,timeout=30)
data=json.loads(p.stdout)
result={'scope':'official installed Python SessionDB create/append and resume projection + separate official CLI export; no inference','session_id':sid,'input':texts,'export_exit':p.returncode,'export_projection':[(m['role'],m['content']) for m in data['messages']],'resume_model_projection':[(m['role'],m['content']) for m in model],'resume_display_projection':[(m['role'],m['content']) for m in display],'model':data.get('model'),'model_config':data.get('model_config')}
result['export_exact']=result['export_projection']==texts;result['resume_display_exact']=result['resume_display_projection']==texts;result['resume_model_exact']=result['resume_model_projection']==texts
(ROOT/'hermes-sdk-result.json').write_text(json.dumps(result,ensure_ascii=False,indent=2))
print(json.dumps(result,ensure_ascii=False,indent=2))
