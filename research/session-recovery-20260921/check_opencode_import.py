"""Isolated native OpenCode import/export of a synthetic text-only session."""
import json
import os
from pathlib import Path
import subprocess
import tempfile
import uuid

OUTPUT = Path(__file__).with_name('opencode-synthetic-result.json')
CLI = '/Users/serendipity/.opencode/bin/opencode'
with tempfile.TemporaryDirectory(prefix='fyagent-opencode-session-') as folder:
    root = Path(folder)
    workspace = root/'workspace'
    workspace.mkdir()
    env = {key: value for key, value in os.environ.items()
           if key in {'PATH', 'HOME', 'USER', 'LOGNAME', 'LANG', 'TMPDIR', 'TERM'}}
    for key, sub in [('XDG_DATA_HOME','data'), ('XDG_CONFIG_HOME','config'),
                     ('XDG_CACHE_HOME','cache'), ('XDG_STATE_HOME','state'),
                     ('OPENCODE_TEST_HOME','testhome')]:
        env[key] = str(root/sub)
        (root/sub).mkdir()
    env['OPENCODE_DISABLE_MODELS_FETCH'] = 'true'
    env['OPENCODE_DISABLE_AUTOUPDATE'] = 'true'
    sid = 'ses_' + uuid.uuid4().hex
    timestamp = 1789980000000
    source_directory = 'C:\\synthetic\\old-workspace'
    tokens = {'input':0,'output':0,'reasoning':0,'cache':{'read':0,'write':0}}
    messages = []
    texts = [('user','My fictional codeword is maple-73.'),
             ('assistant','I will use maple-73 in this conversation.'),
             ('user','Repeat my codeword.'), ('assistant','maple-73')]
    last_user = None
    for i, (role, text) in enumerate(texts):
        mid = f'msg_{i:02d}_' + uuid.uuid4().hex
        info = {'id':mid,'sessionID':sid,'role':role,'time':{'created':timestamp+i},'agent':'build'}
        if role == 'user':
            last_user = mid
            info['model'] = {'providerID':'synthetic','modelID':'synthetic-model'}
        else:
            info.update(parentID=last_user,modelID='synthetic-model',providerID='synthetic',
                        mode='build',path={'cwd':source_directory,'root':source_directory},
                        cost=0,tokens=tokens,finish='stop')
            info['time']['completed'] = timestamp+i
        part = {'id':'prt_'+uuid.uuid4().hex,'sessionID':sid,'messageID':mid,'type':'text','text':text}
        messages.append({'info':info,'parts':[part]})
    fixture = {'info': {'id':sid,'slug':'synthetic-recovery','projectID':'global',
                        'directory':source_directory,'title':'Synthetic Session Recovery',
                        'version':'1.18.30','time':{'created':timestamp,'updated':timestamp+4}},
               'messages':messages}
    fixture_path = root/'session.json'
    fixture_path.write_text(json.dumps(fixture))
    imported = subprocess.run([CLI,'import','--pure',str(fixture_path)],cwd=workspace,env=env,
                              capture_output=True,text=True,timeout=90)
    result = {'cli_version':'1.18.30','scope':'synthetic final-only native JSON import/export, isolated XDG stores',
              'import_exit_code':imported.returncode,'import_stdout':imported.stdout[-2000:],
              'import_stderr':imported.stderr[-2000:],'model_calls':False,'original_user_sessions_accessed':False}
    if imported.returncode == 0:
        exported = subprocess.run([CLI,'export','--pure',sid],cwd=workspace,env=env,
                                  capture_output=True,text=True,timeout=90)
        result.update(export_exit_code=exported.returncode,export_stderr=exported.stderr[-1000:])
        if exported.returncode == 0:
            data = json.loads(exported.stdout)
            projection = [(m['info']['role'], '\n'.join(p['text'] for p in m['parts'] if p['type']=='text')) for m in data['messages']]
            assert projection == texts, projection
            assert all(p['type']=='text' for m in data['messages'] for p in m['parts'])
            result.update(result='passed',recovered_message_count=len(projection),only_text_parts=True,
                          directory_remapped_to_import_cwd=Path(data['info']['directory']).resolve()==workspace.resolve(),
                          assistant_path_still_contains_source_cwd=any(m['info'].get('path',{}).get('cwd')==source_directory for m in data['messages']))
    result.setdefault('result','failed')
    result['not_proven']=['Native interactive resume with a model response','Mac to Windows transfer','Other OpenCode versions']
    OUTPUT.write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n')
    print(json.dumps(result,ensure_ascii=False,indent=2))
