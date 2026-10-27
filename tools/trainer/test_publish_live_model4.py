"""Confirmation veto is explicit, conservative and never mutates a live pin."""
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import publish_live_model4 as P


class PublishConfirmation(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.model = self.root / 'candidate'
        self.model.mkdir()
        (self.model / 'net.pt').write_bytes(b'frozen')
        (self.model / 'model.onnx.json').write_text(json.dumps({'graphs':{'0':'model.k0.onnx','1':'model.k1.onnx'}}))
        (self.model / 'model.k0.onnx').write_bytes(b'passive')
        (self.model / 'model.k1.onnx').write_bytes(b'decision')
        (self.model / 'model.onnx').write_bytes(b'decision')
        self.model_inputs = {str(path):P.digest(path) for path in self.model.iterdir() if path.name != 'model.onnx'}
        self.output = self.root / 'selected-model.json'
        (self.root / 'progress.json').write_text(json.dumps({'best_development_candidate':str(self.model)}))
        self.current = dict(model=str(self.model), score=.2, benchmark_id='same', sha256={'net.pt':P.digest(self.model/'net.pt')})

    def report(self):
        return json.loads(P.confirmation_status_path(self.output).read_text())

    def test_already_selected_failed_confirmation_is_reported_before_fast_path(self):
        self.output.write_text(json.dumps(self.current))
        original = self.output.read_bytes()
        with patch.object(P, 'confirmation_assessment', return_value=dict(status='nonpositive_confirmation', veto=True)), patch.object(P, 'manifest') as manifest:
            self.assertFalse(P.publish(self.root, self.output))
            manifest.assert_not_called()
        self.assertEqual(self.output.read_bytes(), original)
        self.assertTrue(self.report()['review_required'])
        self.assertEqual(self.report()['decision'], 'confirmation_veto_existing_selection_requires_review')

    def test_vetoed_candidate_never_creates_selection(self):
        with patch.object(P, 'confirmation_assessment', return_value=dict(status='nonpositive_confirmation', veto=True)):
            self.assertFalse(P.publish(self.root, self.output))
        self.assertFalse(self.output.exists())
        self.assertTrue(self.report()['selection_unchanged'])

    def test_confirmation_of_other_model_does_not_block_new_candidate(self):
        self.current['model'] = str(self.root/'old')
        self.output.write_text(json.dumps(self.current))
        candidate = {**self.current, 'model':str(self.model), 'score':.3}
        def assessment(_experiment, model):
            return dict(status='not_this_candidate', veto=False) if str(model)==str(self.model) else dict(status='nonpositive_confirmation',veto=True)
        with patch.object(P, 'confirmation_assessment', side_effect=assessment), patch.object(P, 'manifest', return_value=candidate):
            self.assertTrue(P.publish(self.root, self.output))
        self.assertEqual(json.loads(self.output.read_text())['model'], str(self.model))
        self.assertFalse(self.report()['review_required'])

    def confirmation_fixture(self):
        selected=dict(model=str(self.model), model_sha256=P.digest(self.model/'net.pt'), selected_before_confirmation=True)
        (self.root/'confirmation-selection.json').write_text(json.dumps(selected))
        for shape in ('duels','commander'):
            for label in ('selected','initial'):
                path=self.root/'jobs'/f'eval-confirmation-{label}-{shape}'/'job.json'
                path.parent.mkdir(parents=True)
                path.write_text(json.dumps(dict(status='complete')))
        return selected

    def test_raw_verified_confirmation_mean_not_significance_drives_veto(self):
        selected=self.confirmation_fixture()
        def evaluation(_experiment,label,shape):
            return ({'model':str(self.model)}, {}, {'path':label+'-'+shape}, {'inputs':dict(self.model_inputs)})
        def comparison(*_args, **_kwargs):
            return {'paired_uncertainty':{'paired_utility_difference':-.01},'outcomes':{'better':0,'worse':1}}
        with patch.object(P,'checked_evaluation',side_effect=evaluation) as checked, patch.object(P,'compare',side_effect=comparison):
            result=P.confirmation_assessment(self.root,self.model)
        self.assertEqual(checked.call_count,8)
        self.assertTrue(result['veto'])
        self.assertAlmostEqual(result['mean_paired_utility_difference'],-.01)
        self.assertIn('no significance threshold',result['rule'])

    def test_pending_unrelated_and_changed_checkpoint(self):
        self.assertEqual(P.confirmation_assessment(self.root,self.model)['status'],'pending')
        self.confirmation_fixture()
        self.assertEqual(P.confirmation_assessment(self.root,self.root/'other')['status'],'not_this_candidate')
        (self.model/'net.pt').write_bytes(b'changed')
        with self.assertRaisesRegex(ValueError,'checkpoint changed'):
            P.confirmation_assessment(self.root,self.model)

    def test_zero_and_positive_confirmation_mean(self):
        selected=self.confirmation_fixture()
        evaluation=({'model':str(self.model)},{},{},{'inputs':dict(self.model_inputs)})
        for delta,veto in [(0.,True),(.001,False)]:
            with self.subTest(delta=delta), patch.object(P,'checked_evaluation',return_value=evaluation), patch.object(P,'compare',return_value={'paired_uncertainty':{'paired_utility_difference':delta},'outcomes':{}}):
                self.assertEqual(P.confirmation_assessment(self.root,self.model)['veto'],veto)


    def test_graph_mutated_after_evaluation_is_rejected_without_checkpoint_change(self):
        self.confirmation_fixture()
        value={'model':str(self.model),'thinking_steps':1}
        job={'inputs':dict(self.model_inputs)}
        P.verified_model_inputs(value,job)
        unchanged=P.digest(self.model/'net.pt')
        (self.model/'model.k0.onnx').write_bytes(b'changed passive graph')
        self.assertEqual(P.digest(self.model/'net.pt'),unchanged)
        with self.assertRaisesRegex(ValueError,'input changed: model.k0.onnx'):
            P.verified_model_inputs(value,job)

    def test_confirmation_rejects_changed_graph_and_wrong_initial_reference(self):
        self.confirmation_fixture()
        evaluation=({'model':str(self.model)},{},{},{'inputs':dict(self.model_inputs)})
        (self.model/'model.k1.onnx').write_bytes(b'changed after comparison')
        with patch.object(P,'checked_evaluation',return_value=evaluation), self.assertRaisesRegex(ValueError,'input changed'):
            P.confirmation_assessment(self.root,self.model)
        def mismatched(_experiment,label,_shape):
            return ({'model':str(self.root/'wrong-reference') if label=='confirmation-initial' else str(self.model)}, {}, {}, {'inputs':dict(self.model_inputs)})
        with patch.object(P,'checked_evaluation',side_effect=mismatched), self.assertRaisesRegex(ValueError,'reference differs'):
            P.confirmation_assessment(self.root,self.model)

if __name__=='__main__':unittest.main()
