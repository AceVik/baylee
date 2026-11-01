import tempfile
from pathlib import Path
import unittest
import torch
import model4 as M
from training4.package_state import load_package_state


class PackageStateTests(unittest.TestCase):
    def test_preserve_adam_steps_but_never_return_old_hidden_lanes(self):
        torch.manual_seed(51)
        config = M.Config4(d_model=16, n_heads=2, n_layers=1, d_ff=32, structure_width=8)
        old = M.PlayerNet4(config)
        optimizer = torch.optim.AdamW(old.parameters(), lr=1e-4)
        old.rules_proj.weight.square().sum().backward(); optimizer.step()
        saved = dict(config=config.to_dict(), structure=old.rules.data, state_dict=old.state_dict(),
                     optimizer=optimizer.state_dict(), step_num=123, dataset_fingerprint="previous-package",
                     encoder_version=M.ENCODER_VERSION, model_version=M.MODEL_VERSION,
                     trajectory_version=M.TRAJECTORY_VERSION, sampling_version=M.SAMPLING_VERSION,
                     h=torch.ones(8, 9, 16), next_batch=88)
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory)/"last.pt"; torch.save(saved,path)
            new = M.PlayerNet4(config); new_optimizer = torch.optim.AdamW(new.parameters(),lr=2e-5)
            provenance = load_package_state(path,new,new_optimizer,torch.device("cpu"))
            torch.testing.assert_close(new.rules_proj.weight,old.rules_proj.weight)
            self.assertEqual(new_optimizer.state[new.rules_proj.weight]["step"].item(),1)
            self.assertEqual(new_optimizer.param_groups[0]["lr"],2e-5)
            self.assertNotIn("h",provenance); self.assertNotIn("next_batch",provenance)
            self.assertEqual(provenance["parent_step_num"],123)
            saved["structure"] = {**saved["structure"], "dsl_source_sha256":"changed"}
            torch.save(saved,path)
            with self.assertRaisesRegex(ValueError,"catalog differs"):
                load_package_state(path,new,new_optimizer,torch.device("cpu"))


if __name__ == "__main__": unittest.main()
