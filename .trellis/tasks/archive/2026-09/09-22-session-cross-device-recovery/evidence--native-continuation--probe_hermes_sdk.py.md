# 历史文本（工作站路径已匿名化）：evidence/native-continuation/probe_hermes_sdk.py

此文件为历史源码或运行证据，经工作站用户目录语义匿名化后以 Markdown 保存，不是运行入口。归档过程未执行其中内容。**本提交不再声称代码块与原稿字节相同。** 原稿可从来源提交 `6e6a3b3870a047874354f8f685b163622325331b` 的 `.trellis/tasks/archive/2026-09/09-22-session-cross-device-recovery/evidence/native-continuation/probe_hermes_sdk.py` 恢复。

原路径、原稿与提交稿的哈希及恢复来源见[归档路径映射](research/archive-path-map.json)。

- 原稿字节数：1662
- 原稿 SHA-256：`e52ee5132e624903ed02dc981a9fc65dbd01bdb7c064fb26d5e7e46f42516114`
- 匿名化载荷字节数：1661
- 匿名化载荷 SHA-256：`efd2807f792343a3d316c822d239f9d478bcb71da18c767b2f11de81e7c3a468`

下方载荷的准确字节边界见路径映射；若载荷没有末尾换行，围栏前仅补展示换行。

```python
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
p=subprocess.run(['rtk','proxy','/Users/<username>/.local/bin/hermes','sessions','export','-','--session-id',sid],env=env,cwd=ROOT,text=True,capture_output=True,timeout=30)
data=json.loads(p.stdout)
result={'scope':'official installed Python SessionDB create/append and resume projection + separate official CLI export; no inference','session_id':sid,'input':texts,'export_exit':p.returncode,'export_projection':[(m['role'],m['content']) for m in data['messages']],'resume_model_projection':[(m['role'],m['content']) for m in model],'resume_display_projection':[(m['role'],m['content']) for m in display],'model':data.get('model'),'model_config':data.get('model_config')}
result['export_exact']=result['export_projection']==texts;result['resume_display_exact']=result['resume_display_projection']==texts;result['resume_model_exact']=result['resume_model_projection']==texts
(ROOT/'hermes-sdk-result.json').write_text(json.dumps(result,ensure_ascii=False,indent=2))
print(json.dumps(result,ensure_ascii=False,indent=2))
```
