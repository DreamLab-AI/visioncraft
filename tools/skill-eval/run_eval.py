#!/usr/bin/env python3
"""Replicate a frozen skill comparison; print aggregates, never private cases."""
from __future__ import annotations
import argparse
from concurrent.futures import ThreadPoolExecutor
import json
from pathlib import Path
import subprocess
from dataloader import load_dataset
from rollout import run_batch, sha256, write_json

BASE_FILES = [
    'skills/build-with-quality/SKILL.md',
    'skills/build-with-quality/EDD-PROTOCOL.md',
    'skills/build-with-quality/DEBUGGING-PROTOCOL.md',
    'skills/build-with-quality/references/methodologies.md',
    'skills/build-with-quality/references/review-evidence.md',
    'skills/build-with-quality/references/quality-gates-and-workflow.md',
]
CANDIDATE_FILES = [
    'skills/visioncraft/SKILL.md',
    'skills/visioncraft/references/edd.md',
    'skills/visioncraft/references/debugging.md',
    'skills/visioncraft/references/methodologies.md',
    'skills/visioncraft/references/review-evidence.md',
    'skills/visioncraft/references/quality-gates.md',
]

def git(repo: Path, *args: str) -> bytes:
    return subprocess.check_output(['git', '-C', str(repo), *args])

def freeze_context(repo: Path, revision: str, paths: list[str], destination: Path) -> tuple[str, dict]:
    commit = git(repo, 'rev-parse', revision + '^{commit}').decode().strip()
    parts, files = [], []
    for path in paths:
        raw = git(repo, 'show', f'{commit}:{path}')
        parts.append(f'\n===== {path} =====\n' + raw.decode())
        files.append({'path': path, 'sha256': sha256(raw), 'bytes': len(raw)})
    context = ''.join(parts)
    manifest = {'revision': commit, 'files': files, 'context_sha256': sha256(context.encode()),
                'context_bytes': len(context.encode()), 'context_chars': len(context),
                'context_words': len(context.split()), 'selection': 'Six predeclared equivalent method resources; no task-specific selection or truncation.'}
    if destination.exists():
        assert destination.read_text() == context, 'frozen context changed'
    else:
        destination.write_text(context)
    return context, manifest

def aggregate(out: Path) -> dict:
    groups = {}
    for path in sorted((out / 'runs').glob('*/meta.json')):
        meta = json.loads(path.read_text())
        arm, split, rep = path.parent.name.split('__')
        groups.setdefault((arm, split), []).append(meta)
    rows = []
    for (arm, split), metas in sorted(groups.items()):
        complete = [m for m in metas if m['status'] == 'completed']
        rows.append({'arm': arm, 'split': split, 'attempts': len(metas), 'completed': len(complete),
            'evaluations': sum(m['item_count'] for m in complete),
            'hard_acc': sum(m['hard_acc'] for m in complete) / len(complete) if complete else None,
            'soft_acc': sum(m['soft_acc'] for m in complete) / len(complete) if complete else None,
            'hard_replicates': [m['hard_acc'] for m in complete],
            'soft_replicates': [m['soft_acc'] for m in complete],
            'failures': [m['failure'] for m in metas if m['failure']]})
    result = {'groups': rows, 'target_calls': sum(r['attempts'] for r in rows),
              'no_candidate_edits_during_comparison': True,
              'test_prompts_or_answers_exposed_in_report': False}
    pilot = next((r for r in rows if r['arm'] == 'baseline' and r['split'] == 'train'), None)
    result['pilot_saturated'] = bool(pilot and pilot['completed'] and pilot['hard_acc'] in (0, 1))
    write_json(out / 'aggregate.json', result)
    return result

def main() -> None:
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--dataset', type=Path, required=True)
    p.add_argument('--baseline-repo', type=Path, required=True)
    p.add_argument('--baseline-ref', required=True)
    p.add_argument('--candidate-repo', type=Path, required=True)
    p.add_argument('--candidate-ref', required=True)
    p.add_argument('--out', type=Path, required=True)
    p.add_argument('--phase', choices=['pilot', 'validation', 'test', 'summary'], required=True)
    args = p.parse_args()
    args.out = args.out.resolve()
    args.out.mkdir(parents=True, exist_ok=True)
    data = load_dataset(args.dataset)
    assert all(sum(i['split'] == s for i in data['items']) == 4 for s in ('train', 'val', 'test'))
    contexts, manifest = {}, {'dataset_sha256': sha256(args.dataset.read_bytes()), 'arms': {}}
    for arm, repo, ref, files in [('baseline', args.baseline_repo, args.baseline_ref, BASE_FILES),
                                  ('candidate', args.candidate_repo, args.candidate_ref, CANDIDATE_FILES)]:
        contexts[arm], manifest['arms'][arm] = freeze_context(repo, ref, files, args.out / f'{arm}-context.txt')
    mp = args.out / 'context-manifest.json'
    if mp.exists():
        assert json.loads(mp.read_text()) == manifest, 'dataset/context manifest changed'
    else:
        write_json(mp, manifest)
    (args.out / 'runs').mkdir(exist_ok=True)
    phases = {'pilot': [('baseline', 'train', 1)], 'validation': [('candidate', 'val', 1)],
              'test': [(arm, 'test', rep) for rep in range(1, 4) for arm in ('baseline', 'candidate')],
              'summary': []}
    jobs = phases[args.phase]
    if args.phase in ('validation', 'test'):
        pilot = args.out / 'runs/baseline__train__1/meta.json'
        assert pilot.exists() and json.loads(pilot.read_text())['status'] == 'completed', 'pilot unavailable; no target fallback'
    if args.phase == 'test':
        val = args.out / 'runs/candidate__val__1/meta.json'
        assert val.exists() and json.loads(val.read_text())['status'] == 'completed', 'validation unavailable; stop before test'
    # The external caller runs phases serially. Reservations count failures too;
    # no retry or model fallback occurs, and no public command reads the answers.
    reservations = args.out / 'call-reservations.json'
    used = json.loads(reservations.read_text()) if reservations.exists() else []
    assert len(used) + len(jobs) <= 10, 'ten-call budget exhausted'
    labels = [f'{a}__{s}__{r}' for a, s, r in jobs]
    assert not set(labels) & set(used), 'refusing to rerun a reserved batch'
    write_json(reservations, used + labels)
    def execute(job):
        arm, split, rep = job
        items = [i for i in data['items'] if i['split'] == split]
        meta = run_batch(data, items, contexts[arm], args.out / 'runs' / f'{arm}__{split}__{rep}')
        return {'arm': arm, 'split': split, 'rep': rep, **{k:meta[k] for k in ('status','failure','models_observed','hard_acc','soft_acc','elapsed_s')}}
    with ThreadPoolExecutor(max_workers=2) as pool:
        for result in pool.map(execute, jobs):
            print(json.dumps(result), flush=True)
    print(json.dumps(aggregate(args.out)), flush=True)

if __name__ == '__main__':
    main()
