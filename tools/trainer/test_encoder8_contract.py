"""CPU checks of explicit E8 admission and unchanged archived E7 behavior."""
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import torch
import model4 as M4
import model5 as M5
from encoder_contract import check_initial_encoder
from dataset4 import TrajectoryDataset4
from trajectory_store4 import TrajectoryStore4
from training4.cli import parse_args
from training4.bc_mixture import BCMixture
from export_onnx4 import load_export_checkpoint

FOREST={'version':1,'dsl_source_sha256':'synthetic','tags':['Unknown'],
        'nodes':[{'tag':0,'bytes':[],'children':[]}],'roots':{}}

class Encoder8Contract(unittest.TestCase):
    def test_cli_explicit_and_default_without_global_mutation(self):
        base=['train5.py','--data','unused','--out','unused']
        with patch('sys.argv',base):
            self.assertEqual(parse_args('v5')[0].encoder_version,7)
        with patch('sys.argv',base+['--encoder-version','8','--init','old','--encoder-warmstart-from7']):
            self.assertEqual(parse_args('v5')[2]['encoder_version'],8)
        self.assertEqual((M4.ENCODER_VERSION,M5.ENCODER_VERSION),(7,7))
        for flags in [['--encoder-version','8','--relational-revision','1'],
                      ['--encoder-version','8','--resume','old','--encoder-warmstart-from7']]:
            with patch('sys.argv',base+flags),self.assertRaises(SystemExit):
                parse_args('v5')

    def test_explicit_weights_only_transition(self):
        with self.assertRaises(ValueError):check_initial_encoder(7,8)
        proof=check_initial_encoder(7,8,True)
        self.assertEqual(proof['kind'],'new_experiment_weights_only')
        self.assertEqual(proof['source_encoder_version'],7)
        self.assertIsNone(check_initial_encoder(8,8))
        self.assertIsNone(check_initial_encoder(6,7))
        with self.assertRaises(ValueError):check_initial_encoder(8,7)
        with self.assertRaises(ValueError):check_initial_encoder(8,8,True)

    def test_dataset_and_mixture_mismatch_before_cache(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory);(root/'meta.json').write_text('{"encoder_version":7}')
            with self.assertRaisesRegex(ValueError,'encoder_version=8'):
                TrajectoryDataset4(root,encoder_version=8)
            self.assertFalse((root/'.trajectory-cache-v2').exists())
        class Child:
            def __init__(self,v):self.encoder_version=v
        with self.assertRaisesRegex(ValueError,'encoder versions differ'):
            BCMixture(Child(8),Child(7),Child(8),8)

    def test_cache_fingerprint_binds_encoder(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory);(root/'meta.json').write_text('{"encoder_version":7}')
            old=TrajectoryStore4(root,'train',[]);fingerprint,path=old.fingerprint,old.data_path;old.close()
            (root/'meta.json').write_text('{"encoder_version":8}')
            new=TrajectoryStore4(root,'train',[])
            self.assertNotEqual(fingerprint,new.fingerprint);self.assertNotEqual(path,new.data_path);new.close()

    def test_export_loader_preserves_encoder_identity(self):
        torch.set_num_threads(1)
        cfg=M5.Config5(relational_revision=2,d_model=16,structure_width=16,n_heads=2,n_layers=1,d_ff=32)
        net=M5.PlayerNet5(cfg);net.attach_structure(FOREST)
        base=dict(model_version=5,trajectory_version=M5.TRAJECTORY_VERSION,sampling_version=M5.SAMPLING_VERSION,
                  config=cfg.to_dict(),structure=FOREST,architecture=M5.architecture_for_config(cfg),state_dict=net.state_dict())
        with tempfile.TemporaryDirectory() as directory:
            path=Path(directory)/'net.pt'
            for encoder in (7,8):
                torch.save(dict(base,encoder_version=encoder),path)
                self.assertEqual(load_export_checkpoint(path)[0]['encoder_version'],encoder)
            torch.save(dict(base,encoder_version=8,architecture=M5.ARCHITECTURES[1],
                            config={**cfg.to_dict(),'relational_revision':1}),path)
            with self.assertRaisesRegex(ValueError,'relational revision2'):load_export_checkpoint(path)

    def test_full_cpu_export_emits_checkpoint_encoder8(self):
        import json
        from export_onnx4 import main
        torch.set_num_threads(1)
        cfg=M5.Config5(relational_revision=2,d_model=16,structure_width=16,n_heads=2,n_layers=1,d_ff=32)
        net=M5.PlayerNet5(cfg);net.attach_structure(FOREST)
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory)
            torch.save(dict(model_version=5,encoder_version=8,trajectory_version=M5.TRAJECTORY_VERSION,
                sampling_version=M5.SAMPLING_VERSION,config=cfg.to_dict(),structure=FOREST,
                architecture=M5.architecture_for_config(cfg),state_dict=net.state_dict()),root/'net.pt')
            with patch('sys.argv',['export_onnx4.py','--model',directory,'--check','0']):main()
            meta=json.loads((root/'model.onnx.json').read_text())
            self.assertEqual((meta['model_version'],meta['encoder_version']),(5,8))

    def test_encoder8_companions_cannot_be_relabelled_from7(self):
        import hashlib,json
        from test_shadow_dataset4 import fixture as shadow_fixture
        from test_llm_dataset4 import fixture as llm_fixture
        from trajectory_store4 import validate_bc_dataset as validate_shadow_dataset
        from trajectory_store4 import validate_bc_dataset as validate_llm_dataset
        for fixture,validator,key in [(shadow_fixture,validate_shadow_dataset,'shadow_bc_label_sha256'),
                                      (llm_fixture,validate_llm_dataset,'llm_bc_label_sha256')]:
            with tempfile.TemporaryDirectory() as directory:
                root=Path(directory);result=fixture(root);meta,path,rows=result[:3]
                meta['encoder_version']=8
                with self.assertRaises(ValueError):validator(root,meta)
                rows[0]['encoder_version']=8
                path.write_text(''.join(json.dumps(row)+'\n' for row in rows))
                meta[key][str(path.relative_to(root))]=hashlib.sha256(path.read_bytes()).hexdigest()
                validator(root,meta)
                meta['encoder_version']=7
                with self.assertRaises(ValueError):validator(root,meta)
