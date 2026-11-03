"""Private scenario loader; never print reference answers or held-out prompts."""
from __future__ import annotations
import json
from pathlib import Path

DIMENSIONS = ('gate', 'next_action', 'deployment', 'independence')
ENUMS = {
    'gate': ('pass', 'blocked', 'incomplete', 'invalid'),
    'next_action': ('execute', 'fix_setup', 'fix_code', 'review_evidence', 'update_docs', 'ship_source', 'request_required_input'),
    'deployment': ('observed', 'unobserved', 'contradicted'),
    'independence': ('satisfied', 'degraded', 'not_required', 'unknown'),
}

def load_dataset(path: Path) -> dict:
    data = json.loads(path.read_text())
    assert isinstance(data['rubric'], dict)
    ids = set()
    for item in data['items']:
        assert isinstance(item['id'], str) and item['id'] and item['id'] not in ids
        assert all(c.isalnum() or c in '-_' for c in item['id'])
        ids.add(item['id'])
        assert item['split'] in ('train', 'val', 'test')
        assert isinstance(item['prompt'], str) and item['prompt']
        for name in DIMENSIONS:
            values = item['accepted'][name]
            assert isinstance(values, list) and values
            assert all(value in ENUMS[name] for value in values)
    return data

def public_batch(data: dict, items: list[dict]) -> dict:
    """Expose only task prose and the curator's shared dimension definitions."""
    return {
        'rubric': {name: data['rubric'][name] for name in DIMENSIONS},
        'allowed_values': ENUMS,
        'items': [{'id': item['id'], 'prompt': item['prompt']} for item in items],
    }
