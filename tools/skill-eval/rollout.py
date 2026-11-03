"""Bounded, no-tool Claude rollout and deterministic SkillOpt-shaped scoring.

This is an external evaluation utility, not part of VisionCraft's runtime.
Reference answers stay in the caller's private dataset and scorer.
"""
from __future__ import annotations
import hashlib
import json
import os
from pathlib import Path
import signal
import subprocess
import time
from dataloader import DIMENSIONS, ENUMS, public_batch

TARGET_MODEL = 'claude-haiku-4-5-20251001'

def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()

def write_json(path: Path, value: object) -> None:
    path.write_text(json.dumps(value, indent=2, ensure_ascii=False) + '\n')

def prompt_for(data: dict, items: list[dict], skill: str) -> str:
    task = public_batch(data, items)
    return (
        'Apply the supplied engineering guidance to each independent scenario. '
        'Do not use tools or read files. Use only facts stated in each scenario; '
        'do not assume extra requirements or successful executions. '
        'Return one JSON object with key answers, an array containing exactly '
        'one object per item: id, gate, next_action, deployment, independence. '
        'Use the provided exact enum values. Do not include explanations.\n\n'
        '<engineering-guidance>\n' + skill + '\n</engineering-guidance>\n\n'
        + json.dumps(task, ensure_ascii=False)
    )

def parse_predictions(text: str, items: list[dict]) -> dict:
    value = text.strip()
    if value.startswith('```json') and value.endswith('```'):
        value = value[len('```json'):-3].strip()
    elif value.startswith('```') and value.endswith('```'):
        value = value[3:-3].strip()
    parsed = json.loads(value)
    assert set(parsed) == {'answers'} and isinstance(parsed['answers'], list)
    expected = {item['id'] for item in items}
    answers = {}
    for row in parsed['answers']:
        assert set(row) == {'id', *DIMENSIONS}
        assert row['id'] in expected and row['id'] not in answers
        assert all(row[name] in ENUMS[name] for name in DIMENSIONS)
        answers[row['id']] = row
    assert set(answers) == expected
    return answers

def score_item(item: dict, answer: dict | None, failure: str = '') -> dict:
    dimensions = {name: bool(answer and answer[name] in item['accepted'][name]) for name in DIMENSIONS}
    return {
        'id': item['id'], 'hard': int(all(dimensions.values())),
        'soft': sum(dimensions.values()) / len(DIMENSIONS),
        'dimensions': dimensions, 'fail_reason': failure,
        'task_description': item['prompt'], 'n_turns': 1 if answer else 0,
    }

def run_batch(data: dict, items: list[dict], skill: str, out_dir: Path,
              model: str = TARGET_MODEL, timeout: int = 180) -> dict:
    """Run one four-case batch; retain private raw output and per-item trajectories."""
    assert model == TARGET_MODEL and 1 <= timeout <= 180 and len(items) == 4
    out_dir.mkdir(parents=True, exist_ok=False)
    sandbox = out_dir / 'empty-cwd'
    sandbox.mkdir()
    prompt = prompt_for(data, items, skill)
    (out_dir / 'prompt-private.txt').write_text(prompt)
    args = ['claude', '-p', '--model', model, '--tools', '', '--strict-mcp-config',
            '--mcp-config', '{"mcpServers":{}}', '--setting-sources', '',
            '--no-session-persistence', '--output-format', 'json', '--max-turns', '2']
    started = time.time()
    process = subprocess.Popen(args, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                               stderr=subprocess.PIPE, text=True, cwd=sandbox,
                               start_new_session=True)
    timed_out = False
    try:
        stdout, stderr = process.communicate(prompt, timeout=timeout)
    except subprocess.TimeoutExpired:
        timed_out = True
        os.killpg(process.pid, signal.SIGTERM)
        try:
            stdout, stderr = process.communicate(timeout=3)
        except subprocess.TimeoutExpired:
            os.killpg(process.pid, signal.SIGKILL)
            stdout, stderr = process.communicate(timeout=3)
    (out_dir / 'raw-private.json').write_text(stdout)
    (out_dir / 'stderr-private.log').write_text(stderr)
    outer, answers, failure = {}, {}, ''
    if timed_out:
        failure = 'target_timeout'
    elif process.returncode != 0:
        failure = 'target_exit_error'
    else:
        try:
            outer = json.loads(stdout)
            if outer.get('is_error') is True:
                failure = 'target_reported_error'
            else:
                observed = list(outer.get('modelUsage', {}))
                if not observed or any(name != model for name in observed):
                    failure = 'target_model_unverified'
                else:
                    answers = parse_predictions(outer['result'], items)
        except (ValueError, KeyError, TypeError, AssertionError):
            failure = 'invalid_target_response'
    results = []
    for item in items:
        answer = answers.get(item['id'])
        score = score_item(item, answer, failure)
        results.append(score)
        pred = out_dir / 'predictions' / item['id']
        pred.mkdir(parents=True)
        write_json(pred / 'conversation.json', [
            {'role': 'user', 'content': prompt_for(data, [item], skill)},
            {'role': 'assistant', 'content': json.dumps(answer) if answer else ''},
            {'role': 'system', 'content': json.dumps({k: score[k] for k in ('hard', 'soft', 'dimensions', 'fail_reason')})},
        ])
        write_json(pred / 'prediction.json', answer)
    # Results contain task prose for SkillOpt compatibility; keep them private.
    write_json(out_dir / 'results-private.json', results)
    meta = {
        'status': 'unavailable' if failure else 'completed', 'failure': failure,
        'model_requested': model, 'models_observed': list(outer.get('modelUsage', {})),
        'argv': args, 'cwd_policy': 'fresh empty directory; tools/MCP/settings disabled; managed global instructions may still apply',
        'elapsed_s': round(time.time() - started, 3), 'rc': process.returncode,
        'prompt_sha256': sha256(prompt.encode()), 'prompt_bytes': len(prompt.encode()),
        'raw_output_sha256': sha256(stdout.encode()), 'item_count': len(items),
        'hard_acc': sum(row['hard'] for row in results) / len(results),
        'soft_acc': sum(row['soft'] for row in results) / len(results),
        'dimension_acc': {name: sum(row['dimensions'][name] for row in results) / len(results) for name in DIMENSIONS},
        'usage': outer.get('usage'), 'model_usage': outer.get('modelUsage'),
        'reported_cost_usd': outer.get('total_cost_usd'),
        'count_boundary': 'Unavailable/invalid calls are not evidence of task performance; their scores are retained only for diagnosis.',
    }
    write_json(out_dir / 'meta.json', meta)
    return meta
