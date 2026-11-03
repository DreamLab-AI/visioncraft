#!/usr/bin/env python3
"""Inert Deepsec 2.3.10 protocol double; never calls an LLM or a network."""
import hashlib, json, pathlib, sys, time
args=sys.argv[1:]
def opt(name,default=None):
 return args[args.index(name)+1] if name in args else default
workspace=pathlib.Path.cwd()
root=workspace.parent
casefile=root/'.fixture-case'
case=casefile.read_text().strip() if casefile.exists() else 'pass'
if args==['--version']:
 print('9.0.0' if case=='unsupported' else '2.3.10');sys.exit(0)
project=opt('--project-id',root.name)
data=workspace/'data'/project
cmd=args[0]
if cmd=='scan':
 target=data/'files'/'input.py.json';target.parent.mkdir(parents=True,exist_ok=True)
 target.write_text(json.dumps({'filePath':'input.py','status':'pending'}));sys.exit(0)
if cmd=='process':
 if case=='timeout':time.sleep(30)
 if case=='runtime':sys.exit(1)
 if case=='no-evidence':sys.exit(0)
 paths=pathlib.Path(opt('--files-from')).read_text().splitlines() if '--files-from' in args else ['input.py']
 (data/'runs').mkdir(parents=True,exist_ok=True)
 run_id='fixture-run'
 run={'runId':run_id,'projectId':project,'rootPath':str(root),'type':'process','phase':'done','completedAt':'2026-11-03T00:00:00Z','stats':{'filesProcessed':len(paths)}}
 if case=='incomplete':run['phase']='running'
 (data/'runs'/f'{run_id}.json').write_text(json.dumps(run))
 for path in paths:
  entry={'runId':run_id,'model':'fixture-model','agentType':'fixture-agent','refusal':{'refused':False,'skipped':[]},'usage':{'outputTokens':42}}
  if case=='missing-skipped':del entry['refusal']['skipped']
  if case=='null-skipped':entry['refusal']['skipped']=None
  if case=='scalar-skipped':entry['refusal']['skipped']='unknown'
  if case=='refused':entry['refusal']['refused']=True
  if case=='partial':entry['refusal']['skipped']=[{'filePath':path,'reason':'fixture'}]
  if case=='unknown-refusal':del entry['refusal']
  if case=='zero-output':entry['usage']['outputTokens']=0
  if case=='missing-model':del entry['model']
  record={'filePath':path,'projectId':project,'fileHash':hashlib.sha256((root/path).read_bytes()).hexdigest(),'status':'analyzed','analysisHistory':[entry]}
  if case=='stale':record['analysisHistory'][0]['runId']='old-run'
  if case=='wrong-hash':record['fileHash']='0'*64
  target=data/'files'/f'{path}.json';target.parent.mkdir(parents=True,exist_ok=True);target.write_text(json.dumps(record))
 if case=='drift':(root/'input.py').write_text('changed during review\n')
 if case in ['block','low']:
  pathlib.Path(opt('--comment-out')).write_text('Fresh finding\n');sys.exit(1)
 sys.exit(0)
if cmd=='export':
 if case=='export-failure':sys.exit(2)
 output='[]'
 if case=='malformed':output='{}'
 if case=='duplicate-key':output='[{"severity":"HIGH","severity":"LOW"}]'
 if case=='block':output='[{"severity":"HIGH","title":"fixture"}]'
 if case=='low':output='[{"severity":"LOW","title":"fixture"}]'
 pathlib.Path(opt('--out')).write_text(output);sys.exit(0)
sys.exit(99)
