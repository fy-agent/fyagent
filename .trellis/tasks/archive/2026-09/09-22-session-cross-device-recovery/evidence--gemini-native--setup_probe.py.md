# 历史文本（工作站路径已匿名化）：evidence/gemini-native/setup_probe.py

此文件为历史源码或运行证据，经工作站用户目录语义匿名化后以 Markdown 保存，不是运行入口。归档过程未执行其中内容。**本提交不再声称代码块与原稿字节相同。** 原稿可从来源提交 `6e6a3b3870a047874354f8f685b163622325331b` 的 `.trellis/tasks/archive/2026-09/09-22-session-cross-device-recovery/evidence/gemini-native/setup_probe.py` 恢复。

原路径、原稿与提交稿的哈希及恢复来源见[归档路径映射](research/archive-path-map.json)。

- 原稿字节数：1194
- 原稿 SHA-256：`507c78417bf41b32784078603f08846290748f99d6861b38c402a262cbe03da5`
- 匿名化载荷字节数：1194
- 匿名化载荷 SHA-256：`507c78417bf41b32784078603f08846290748f99d6861b38c402a262cbe03da5`

下方载荷的准确字节边界见路径映射；若载荷没有末尾换行，围栏前仅补展示换行。

```python
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
```
