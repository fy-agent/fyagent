"""Exact installed-provider probes; synthetic homes and loopback HTTP only.

Run: rtk proxy python3 work/native-writer-staging/verify_native_writers.py
The HTTP endpoint captures a request then rejects it; no inference is performed.
"""
import http.server
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import threading
import time
import uuid

STAGING = Path(__file__).resolve().parent
REPO = next((p for p in STAGING.parents if (p / 'src-tauri/Cargo.toml').is_file()),
            STAGING.parent / 'fyagent-session')
NATIVE = REPO / 'src-tauri/src/session_manager/migrate/native'
ROOT = STAGING / ('run-' + uuid.uuid4().hex[:10])
ROOT.mkdir()
OPENCODE = '/Users/serendipity/.opencode/bin/opencode'
HERMES = '/Users/serendipity/.local/bin/hermes'
HERMES_SOURCE = '/Users/serendipity/.hermes/hermes-agent'
PYTHON = HERMES_SOURCE + '/venv/bin/python'
GEMINI = '/opt/homebrew/bin/gemini'
CAPTURE = []


class Handler(http.server.BaseHTTPRequestHandler):
    def do_POST(self):
        body = json.loads(self.rfile.read(int(self.headers.get('Content-Length', 0))))
        CAPTURE.append({'path': self.path, 'body': body})
        is_count = ':countTokens' in self.path
        self.send_response(200 if is_count else 400)
        self.send_header('Content-Type', 'application/json')
        self.end_headers()
        if is_count:
            self.wfile.write(b'{"totalTokens":100}')
        else:
            self.wfile.write(b'{"error":{"code":400,"message":"synthetic capture complete; no inference","status":"INVALID_ARGUMENT","type":"invalid_request_error"}}')

    def log_message(self, *_args):
        pass


SERVER = http.server.ThreadingHTTPServer(('127.0.0.1', 0), Handler)
threading.Thread(target=SERVER.serve_forever, daemon=True).start()
BASE = f'http://127.0.0.1:{SERVER.server_port}'


def environment(root):
    env = {k: os.environ[k] for k in ('PATH', 'LANG') if k in os.environ}
    for key, folder in [('HOME', 'home'), ('TMPDIR', 'tmp'), ('XDG_CONFIG_HOME', 'config'),
                        ('XDG_DATA_HOME', 'data'), ('XDG_CACHE_HOME', 'cache'), ('XDG_STATE_HOME', 'state')]:
        path = root / folder
        path.mkdir(parents=True, exist_ok=True)
        env[key] = str(path)
    env.update(HTTP_PROXY='http://127.0.0.1:1', HTTPS_PROXY='http://127.0.0.1:1',
               ALL_PROXY='http://127.0.0.1:1', NO_PROXY='127.0.0.1,localhost',
               NO_COLOR='1', TERM='dumb', PYTHONDONTWRITEBYTECODE='1')
    return env


def run(args, env, cwd, timeout=40):
    try:
        result = subprocess.run(['rtk', 'proxy', *args], env=env, cwd=cwd,
                                text=True, capture_output=True, timeout=timeout)
        return {'exit_code': result.returncode, 'stdout': result.stdout, 'stderr': result.stderr[-8000:]}
    except subprocess.TimeoutExpired as error:
        return {'timeout': True, 'stdout': str(error.stdout), 'stderr': str(error.stderr)[-8000:]}


def embedded(file, constant):
    source = (NATIVE / file).read_text()
    return re.search(r'const ' + constant + r': &str = r#"(.*?)"#;', source, re.S).group(1)


def checkpoint(provider, result):
    (ROOT / (provider + '.json')).write_text(json.dumps(result, ensure_ascii=False, indent=2))
    print(provider, 'saved', flush=True)


def captures_since(start):
    output = []
    for item in CAPTURE[start:]:
        body = item['body']
        projected = {'model': body.get('model')}
        if 'messages' in body:
            projected['messages'] = [m for m in body['messages'] if m.get('role') in ('user', 'assistant')]
        if 'contents' in body:
            projected['contents'] = body['contents']
        if 'input' in body:
            projected['input'] = [m for m in body['input'] if m.get('role') in ('user', 'assistant')]
        output.append({'path': item['path'], 'body': projected})
    return output


MESSAGES = [{'role': role, 'text': text} for role, text in [
    ('user', '  Unanswered one\r\n'), ('user', 'Keep KITE-73 中文'),
    ('assistant', 'One final\r\n  '), ('assistant', 'Second final C:\\foo'),
    ('user', 'Trailing unanswered')]]


def probe_opencode():
    root = ROOT / 'opencode'
    env = environment(root)
    env.update(OPENCODE_TEST_HOME=env['HOME'], OPENCODE_DISABLE_MODELS_FETCH='true', OPENCODE_DISABLE_AUTOUPDATE='true')
    config = root / 'config/opencode/opencode.json'
    config.parent.mkdir(parents=True, exist_ok=True)
    config.write_text(json.dumps({'model': 'probe/local', 'small_model': 'probe/local', 'enabled_providers': ['probe'],
        'provider': {'probe': {'npm': '@ai-sdk/openai-compatible', 'name': 'Loopback only',
            'options': {'baseURL': BASE + '/v1', 'apiKey': 'synthetic-not-a-key'},
            'models': {'local': {'name': 'Synthetic Capture', 'limit': {'context': 100000, 'output': 1024}}}}}}))
    env['OPENCODE_CONFIG'] = str(config)
    env['OPENCODE_CONFIG_DIR'] = str(config.parent)
    env['OPENCODE_DB'] = str(root / 'data/opencode/opencode.db')
    Path(env['OPENCODE_DB']).parent.mkdir(parents=True, exist_ok=True)
    config_result = run([OPENCODE, 'debug', 'config', '--pure'], env, root)
    model = json.loads(config_result['stdout'])['model']
    assert model == 'probe/local'
    sid, now, parent = 'ses_' + uuid.uuid4().hex, int(time.time() * 1000), ''
    messages = []
    for index, message in enumerate(MESSAGES):
        mid = f'msg_{index:010}_' + uuid.uuid4().hex
        info = {'id': mid, 'sessionID': sid, 'role': message['role'], 'time': {'created': now + index}}
        if message['role'] == 'user':
            parent = mid
            info.update(agent='build', model={'providerID': 'probe', 'modelID': 'local'})
        else:
            info.update(parentID=parent, providerID='probe', modelID='local', mode='build', agent='build',
                path={'cwd': str(root), 'root': str(root)}, cost=0,
                tokens={'input': 0, 'output': 0, 'reasoning': 0, 'cache': {'read': 0, 'write': 0}}, finish='stop')
            info['time']['completed'] = now + index
        messages.append({'info': info, 'parts': [{'id': 'prt_' + uuid.uuid4().hex, 'sessionID': sid,
            'messageID': mid, 'type': 'text', 'text': message['text']}]})
    fixture = {'info': {'id': sid, 'slug': 'restored-' + sid[4:12], 'projectID': 'global',
        'directory': str(root), 'title': 'Synthetic recovery', 'version': '1.18.30',
        'time': {'created': now, 'updated': now}, 'model': {'providerID': 'probe', 'id': 'local'}}, 'messages': messages}
    path = root / 'fixture.json'
    path.write_text(json.dumps(fixture, ensure_ascii=False))
    result = {'version': run([OPENCODE, '--version'], env, root), 'id': sid,
        'target_model': model, 'import': run([OPENCODE, 'import', '--pure', str(path)], env, root)}
    exported = run([OPENCODE, 'export', '--pure', sid], env, root)
    result['readback'] = exported
    checkpoint('opencode', result)
    assert [(m['info']['role'], m['parts'][0]['text']) for m in json.loads(exported['stdout'])['messages']] == [(m['role'], m['text']) for m in MESSAGES]
    start = len(CAPTURE)
    result['continue'] = run([OPENCODE, 'run', '--pure', '--format', 'json', '--session', sid, 'Synthetic next turn; keep KITE-73.'], env, root)
    result['captured_requests'] = captures_since(start)
    checkpoint('opencode', result)


def probe_hermes():
    root = ROOT / 'hermes'
    env = environment(root)
    home = root / 'hermes-home'
    home.mkdir()
    env.update(HERMES_HOME=str(home), HERMES_DISABLE_UPDATE_CHECK='1',
        OPENAI_BASE_URL=BASE + '/v1', OPENAI_API_KEY='synthetic-not-a-key')
    sid = 'fyagent_' + uuid.uuid4().hex
    request = {'op': 'preflight', 'id': sid, 'db_path': str(home / 'state.db'),
        'workspace': str(root), 'title': 'Synthetic recovery', 'messages': [
            {'role': 'user', 'text': 'KITE-73 中文 C:\\temp\\x'},
            {'role': 'assistant', 'text': 'answer\r\nsecond line'},
            {'role': 'user', 'text': 'second question'},
            {'role': 'assistant', 'text': 'second final 中文'}]}
    body = embedded('hermes.rs', 'SDK_BRIDGE')
    def call(op, messages=None):
        data = {**request, 'op': op}
        if messages is not None:
            data['messages'] = messages
        file = root / 'request.json'
        file.write_text(json.dumps(data, ensure_ascii=False))
        return run([PYTHON, '-I', '-B', '-c', body, HERMES_SOURCE, str(file)], env, root)
    result = {'id': sid, 'preflight': call('preflight'), 'write': call('write'), 'repeat_write': call('write')}
    result['readback'] = run([HERMES, 'sessions', 'export', '-', '--session-id', sid, '--format', 'jsonl'], env, root)
    exported = json.loads(result['readback']['stdout'])
    assert [(m['role'], m['content']) for m in exported['messages']] == [(m['role'], m['text']) for m in request['messages']]
    assert exported['messages'][1]['finish_reason'] == 'stop'
    result['reject_trim'] = call('preflight', [{'role': 'user', 'text': '  Q'}, {'role': 'assistant', 'text': 'A'}])
    result['reject_merge'] = call('preflight', MESSAGES)
    result['reject_unanswered'] = call('preflight', request['messages'][:-1])
    start = len(CAPTURE)
    result['continue'] = run([HERMES, 'chat', '--resume', sid, '-q', 'Synthetic next turn; keep KITE-73.',
        '--provider', 'openai-api', '--model', 'local-capture', '--safe-mode', '--no-restore-cwd',
        '--in', str(root), '--max-turns', '1', '--run-budget', '15', '--quiet'], env, root, 35)
    result['captured_requests'] = captures_since(start)
    checkpoint('hermes', result)


def probe_gemini():
    root = ROOT / 'gemini'
    env = environment(root)
    env.update(GEMINI_CLI_HOME=env['HOME'], GEMINI_CLI_NO_RELAUNCH='true',
        GEMINI_API_KEY='synthetic-not-a-key', GOOGLE_GEMINI_BASE_URL=BASE)
    config = Path(env['GEMINI_CLI_HOME']) / '.gemini'
    config.mkdir()
    (config / 'tmp').mkdir()
    (config / 'settings.json').write_text(json.dumps({'security': {'auth': {'selectedType': 'gemini-api-key'}},
        'model': {'name': 'gemini-2.5-flash'}, 'privacy': {'usageStatisticsEnabled': False},
        'general': {'disableAutoUpdate': True}, 'telemetry': {'enabled': False}}))
    project = root / 'project'
    project.mkdir()
    sid = str(uuid.uuid4())
    request = {'op': 'preflight', 'id': sid, 'workspace': str(project), 'title': 'Synthetic recovery', 'messages': MESSAGES}
    body = embedded('gemini.rs', 'NATIVE_BRIDGE')
    def call(op, messages=None):
        data = {**request, 'op': op}
        if messages is not None:
            data['messages'] = messages
        file = root / 'request.json'
        file.write_text(json.dumps(data, ensure_ascii=False))
        return run(['node', '--input-type=module', '--eval', body, GEMINI, str(file)], env, project)
    result = {'id': sid, 'preflight': call('preflight'), 'write': call('write'),
              'readback': call('read'), 'restart_readback': call('read'), 'repeat_write': call('write')}
    assert json.loads(result['readback']['stdout'])['messages'] == MESSAGES
    result['reject_prefix'] = call('preflight', [{'role': 'user', 'text': '?literal question'}])
    start = len(CAPTURE)
    result['continue'] = run([GEMINI, '--skip-trust', '--resume', sid, '--prompt',
        'Synthetic next turn; keep KITE-73.', '--output-format', 'stream-json', '--model', 'gemini-2.5-flash'], env, project)
    result['captured_requests'] = captures_since(start)
    checkpoint('gemini', result)


try:
    for probe in (probe_opencode, probe_hermes, probe_gemini):
        if len(sys.argv) > 1 and probe.__name__ != 'probe_' + sys.argv[1]:
            continue
        try:
            probe()
        except Exception as error:
            checkpoint(probe.__name__, {'error': repr(error)})
finally:
    SERVER.shutdown()
print(str(ROOT), flush=True)
