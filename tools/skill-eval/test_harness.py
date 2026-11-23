"""Synthetic controls for leakage boundaries, parsing and four-dimension scores."""
import json
from pathlib import Path
import tempfile
import unittest
from dataloader import DIMENSIONS, load_dataset, public_batch
from rollout import parse_predictions, prompt_for, score_item


def fixture():
    answer = {'gate':'pass','next_action':'execute','deployment':'unobserved','independence':'not_required'}
    item = {'id':'synthetic-1','split':'train','prompt':'Synthetic task, no private evaluation item.',
            'accepted':{k:[v] for k,v in answer.items()},'answer_rationale':'SECRET_REFERENCE_MARKER'}
    data = {'rubric':{k:{'definition':'Shared enum definition'} for k in DIMENSIONS},'items':[item]}
    return data,item,{'id':item['id'],**answer}

class HarnessTest(unittest.TestCase):
    def test_answers_never_enter_target_prompt(self):
        data,item,_=fixture()
        public=public_batch(data,[item])
        self.assertEqual(set(public['items'][0]),{'id','prompt'})
        rendered=prompt_for(data,[item],'Frozen guidance')
        self.assertNotIn('SECRET_REFERENCE_MARKER',rendered)
        self.assertNotIn('"accepted"',rendered)
        self.assertNotIn('answer_rationale',rendered)
    def test_exact_response_and_multiple_accepted_values(self):
        _,item,answer=fixture()
        item['accepted']['next_action'].append('review_evidence')
        answer['next_action']='review_evidence'
        parsed=parse_predictions(json.dumps({'answers':[answer]}),[item])
        self.assertEqual(score_item(item,parsed[item['id']])['hard'],1)
    def test_soft_fraction_and_missing_response(self):
        _,item,answer=fixture()
        answer['deployment']='observed'
        score=score_item(item,answer)
        self.assertEqual((score['hard'],score['soft']),(0,0.75))
        self.assertEqual(score_item(item,None,'unavailable')['soft'],0)
    def test_duplicate_unknown_and_missing_rows_rejected(self):
        _,item,answer=fixture()
        for rows in ([answer,answer],[],[{**answer,'extra':'pass'}],[{**answer,'gate':'green'}]):
            with self.assertRaises(AssertionError):
                parse_predictions(json.dumps({'answers':rows}),[item])
    def test_loader_validates_and_does_not_print(self):
        data,_,_=fixture()
        with tempfile.TemporaryDirectory() as d:
            p=Path(d)/'synthetic.json';p.write_text(json.dumps(data))
            self.assertEqual(load_dataset(p),data)
    def test_fenced_json_response_is_parsed(self):
        _,item,answer=fixture()
        body=json.dumps({'answers':[answer]})
        for fenced in ('```json\n'+body+'\n```','```\n'+body+'\n```'):
            self.assertEqual(parse_predictions(fenced,[item])[item['id']],answer)
    def test_loader_rejects_invalid_split_id_prompt_and_accepted(self):
        data,_,_=fixture()
        mutations=[lambda i:i.update(split='holdout'),lambda i:i.update(id='bad id'),
                   lambda i:i.update(prompt=''),lambda i:i['accepted'].update(gate=[]),
                   lambda i:i['accepted'].update(gate=['green'])]
        with tempfile.TemporaryDirectory() as d:
            p=Path(d)/'synthetic.json'
            for mutate in mutations:
                item=json.loads(json.dumps(data['items'][0]));mutate(item)
                p.write_text(json.dumps({**data,'items':[item]}))
                with self.assertRaises(AssertionError):
                    load_dataset(p)
            p.write_text(json.dumps({**data,'items':[data['items'][0],dict(data['items'][0])]}))
            with self.assertRaises(AssertionError):
                load_dataset(p)
    def test_loader_rejects_rubric_missing_a_dimension(self):
        data,_,_=fixture()
        with tempfile.TemporaryDirectory() as d:
            p=Path(d)/'synthetic.json'
            for name in DIMENSIONS:
                truncated={k:v for k,v in data['rubric'].items() if k!=name}
                p.write_text(json.dumps({**data,'rubric':truncated}))
                with self.assertRaises(AssertionError):
                    load_dataset(p)
    def test_aggregate_averages_completed_replicates_only(self):
        from run_eval import aggregate
        with tempfile.TemporaryDirectory() as d:
            out=Path(d);runs=out/'runs'
            metas={'baseline__train__1':(1.0,1.0,''),
                   'candidate__test__1':(0.5,0.75,''),
                   'candidate__test__2':(1.0,1.0,''),
                   'candidate__test__3':(0.0,0.0,'target_timeout')}
            for name,(hard,soft,failure) in metas.items():
                meta={'status':'completed' if not failure else 'unavailable','hard_acc':hard,
                      'soft_acc':soft,'failure':failure,'item_count':4}
                (runs/name).mkdir(parents=True);(runs/name/'meta.json').write_text(json.dumps(meta))
            result=aggregate(out)
            self.assertEqual([(r['arm'],r['split'],r['attempts'],r['completed']) for r in result['groups']],
                             [('baseline','train',1,1),('candidate','test',3,2)])
            test=next(r for r in result['groups'] if r['split']=='test')
            self.assertEqual((test['hard_acc'],test['soft_acc'],test['failures']),(0.75,0.875,['target_timeout']))
            self.assertEqual(test['hard_replicates'],[0.5,1.0])
            self.assertTrue(result['pilot_saturated'])

if __name__=='__main__': unittest.main()
