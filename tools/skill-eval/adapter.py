"""Thin external batch adapter compatible with SkillOpt's result/trajectory contract."""
from pathlib import Path
from rollout import run_batch

def run_visioncraft_batch(*, data, items, skill_content, out_dir, **kwargs):
    """Return per-item id/hard/soft/task_description/fail_reason/n_turns rows."""
    import json
    path = Path(out_dir)
    run_batch(data, list(items), skill_content, path, **kwargs)
    return json.loads((path / 'results-private.json').read_text())
