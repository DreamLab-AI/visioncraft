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

if __name__=='__main__': unittest.main()
