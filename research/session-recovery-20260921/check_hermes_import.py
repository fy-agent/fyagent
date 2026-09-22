"""Synthetic-only import/replay check against installed Hermes; no model calls."""
import hashlib
import json
import os
from pathlib import Path
import sys
import tempfile

SOURCE = Path('/Users/serendipity/.hermes/hermes-agent')
OUTPUT = Path(__file__).with_name('hermes-synthetic-result.json')

def message(role, text, channel=None):
    payload = {'type': 'message', 'role': role,
               'content': [{'type': 'input_text' if role == 'user' else 'output_text', 'text': text}]}
    if channel:
        payload['phase'] = channel
    return {'type': 'response_item', 'payload': payload}

with tempfile.TemporaryDirectory(prefix='fyagent-session-recovery-') as folder:
    root = Path(folder)
    os.environ['HERMES_HOME'] = str(root / 'hermes')
    (root / 'hermes').mkdir()
    sys.path.insert(0, str(SOURCE))
    from hermes_cli.foreign_sessions import parse_codex_session, import_foreign_session
    from hermes_state import SessionDB

    header = {'type': 'session_meta', 'payload': {'id': 'synthetic-fyagent-session', 'cwd': str(root / 'workspace')}}
    clean = [header, message('user', 'My fictional codeword is maple-73.'),
             message('assistant', 'I will use maple-73 in this conversation.', 'final'),
             message('user', 'Repeat my codeword.'),
             message('assistant', 'maple-73', 'final')]
    mixed = clean[:2] + [message('assistant', 'Checking now.', 'commentary'),
            {'type': 'response_item', 'payload': {'type': 'function_call', 'name': 'fake_tool', 'call_id': 'fake-1', 'arguments': '{}'}},
            {'type': 'response_item', 'payload': {'type': 'function_call_output', 'call_id': 'fake-1', 'output': 'TOOL_OUTPUT_MUST_NOT_BE_KEPT'}}] + clean[2:]
    clean_path, mixed_path = root/'clean.jsonl', root/'mixed.jsonl'
    for path, rows in [(clean_path, clean), (mixed_path, mixed)]:
        path.write_text(''.join(json.dumps(row)+'\n' for row in rows))
    before = hashlib.sha256(clean_path.read_bytes()).hexdigest()
    db = SessionDB(db_path=root/'hermes'/'state.db')
    try:
        sid = import_foreign_session('codex', clean_path, db=db)
        recovered = db.get_messages_as_conversation(sid, repair_alternation=True)
        projection = [{'role': m['role'], 'content': m['content']} for m in recovered]
        expected = [{'role': x['payload']['role'], 'content': x['payload']['content'][0]['text']} for x in clean[1:]]
        assert projection == expected, projection
        assert before == hashlib.sha256(clean_path.read_bytes()).hexdigest()
        db.append_message(sid, 'user', 'This is an offline next turn.')
        continued = db.get_messages_as_conversation(sid, repair_alternation=True)
        assert len(continued) == 5
        mixed_turns = parse_codex_session(mixed_path)['turns']
        mixed_text = '\n'.join(x['content'] for x in mixed_turns)
        assert 'TOOL_OUTPUT_MUST_NOT_BE_KEPT' not in mixed_text
        assert '[ran tool: fake_tool]' in mixed_text
        assert 'Checking now.' in mixed_text
        result = {
            'result': 'passed',
            'scope': 'Synthetic Codex-format final-only transcript -> installed Hermes native SessionDB -> conversation replay -> offline user append',
            'original_user_data_accessed': False,
            'network_or_model_calls': False,
            'source_unchanged': True,
            'recovered_message_count': len(projection),
            'after_offline_append_count': len(continued),
            'native_importer_final_only': False,
            'native_importer_retains_commentary_and_tool_name_marker': True,
            'isolated_storage_removed_after_test': True,
            'not_proven': ['Model reply after resume', 'Mac to Windows transfer', 'Codex native resume', 'All software/version compatibility']
        }
        OUTPUT.write_text(json.dumps(result, indent=2)+'\n')
        print(json.dumps(result, indent=2))
    finally:
        db.close()
