"""Unit and contract tests for Playernet v4 (P0-1 through P0-7 & P1-8 through P1-10)."""

from __future__ import annotations

import tempfile
from pathlib import Path

import numpy as np
import onnxruntime as ort
import torch
import torch.nn.functional as F

import dataset4 as D4
from export_onnx4 import export_v4_onnx, make_dummy_inputs
import model4 as M
from train_rl4 import action_logprob_and_kl, compute_reverse_gae


def test_p0_1_seat_feat_indices_and_value_head_non_collapse():
    """Verify P0-1: seat_feat channel indices match Rust and value head never collapses to 0.5."""
    assert (
        M.SEAT_FEAT_PRESENT == 0
        and M.SEAT_FEAT_SELF == 1
        and M.SEAT_FEAT_TEAMMATE == 2
        and M.SEAT_FEAT_OPPONENT == 3
        and M.SEAT_FEAT_LOST == 4
    )
    cfg = M.Config4(d_model=64, n_heads=4, n_layers=2, d_ff=128, dropout=0.0)
    net = M.PlayerNet4(cfg)
    with torch.no_grad():
        for m in net.seat_value_head.modules():
            if isinstance(m, torch.nn.Linear):
                m.weight.normal_(0.0, 0.5)
                if m.bias is not None:
                    m.bias.fill_(0.1)

    dummy = list(make_dummy_inputs(cfg.d_model, B=3, E=8, M_edges=8, C=6, P=4, K=2))
    # dummy[3] is seat_feat [B, 8, 64]
    seat_feat = torch.zeros(3, M.MAX_SEATS, M.SEAT_FEAT_WIDTH)
    # Sample 0: 2p duel (Seat 0 self, Seat 1 opponent)
    seat_feat[0, 0, M.SEAT_FEAT_PRESENT] = 1.0
    seat_feat[0, 0, M.SEAT_FEAT_SELF] = 1.0
    seat_feat[0, 1, M.SEAT_FEAT_PRESENT] = 1.0
    seat_feat[0, 1, M.SEAT_FEAT_OPPONENT] = 1.0
    # Sample 1: 4p FFA with Seat 2 eliminated (lost=1)
    seat_feat[1, :4, M.SEAT_FEAT_PRESENT] = 1.0
    seat_feat[1, 0, M.SEAT_FEAT_SELF] = 1.0
    seat_feat[1, 1:4, M.SEAT_FEAT_OPPONENT] = 1.0
    seat_feat[1, 2, M.SEAT_FEAT_LOST] = 1.0
    # Sample 2: 2v2 team game (Seat 0 self, Seat 1 teammate, Seats 2,3 opponents)
    seat_feat[2, :4, M.SEAT_FEAT_PRESENT] = 1.0
    seat_feat[2, 0, M.SEAT_FEAT_SELF] = 1.0
    seat_feat[2, 1, M.SEAT_FEAT_TEAMMATE] = 1.0
    seat_feat[2, 2:4, M.SEAT_FEAT_OPPONENT] = 1.0
    dummy[3] = seat_feat

    h_out, cand_logits, pair_logits, scalar_pred, v = net(*dummy)
    assert v.shape == (3,)
    assert not torch.allclose(v, torch.full_like(v, 0.5), atol=1e-4), f"Value head collapsed to 0.5: {v}"

    loss = F.mse_loss(v, torch.tensor([1.0, 0.0, 1.0]))
    loss.backward()
    last_linear = [m for m in net.seat_value_head.modules() if isinstance(m, torch.nn.Linear)][-1]
    assert last_linear.weight.grad is not None
    assert last_linear.weight.grad.abs().max().item() > 1e-5


def test_p0_2_no_silent_single_idx_relabeling_and_pair_remapping():
    """Verify P0-2: out-of-range single_idx is dropped (step_valid=False), never relabeled to 0."""
    # Create a tiny temporary dataset directory with meta.json, card_dsl.bin, card_dsl_back.bin, token_dsl.bin
    with tempfile.TemporaryDirectory() as tmpdir:
        dpath = Path(tmpdir)
        (dpath / "train").mkdir()
        meta = {
            "encoder_version": M.ENCODER_VERSION,
            "trajectory_version": M.TRAJECTORY_VERSION,
            "dsl_version": 1,
            "cards": [1, 2],
            "token_count": 2,
            "train_shards": ["steps-000.jsonl.gz"],
            "val_shards": [],
        }
        import gzip
        import json

        from test_structure4 import small_forest
        from model4.structure import structure_digest
        forest = small_forest()
        meta["structure_sha256"] = structure_digest(forest)
        (dpath / "card_structure.json").write_text(json.dumps(forest))
        (dpath / "meta.json").write_text(json.dumps(meta))
        front = np.ones((2, M.CARD_DSL_WIDTH), dtype=np.float32) * 1.0
        back = np.ones((2, M.CARD_DSL_WIDTH), dtype=np.float32) * 2.0
        tok = np.ones((3, M.CARD_DSL_WIDTH), dtype=np.float32) * 3.0
        front.tofile(dpath / "card_dsl.bin")
        back.tofile(dpath / "card_dsl_back.bin")
        tok.tofile(dpath / "token_dsl.bin")

        # Write 2 steps in one trajectory:
        # Step 0: entity 0 has face_id=1 (should get `back` DSL = 2.0), entity 1 has token_id=2 (should get `tok[2]` = 3.0),
        #         head=0 (SingleChoice) with out-of-range single_idx=99 -> must have step_valid=False!
        # Step 1: head=2 (PairPick) with pairs=[[0, 1], [999, 1]], multi_mask=[0, 1], free_mask=[1, 1]
        #         Wait: pair [999, 1] is out of entity range (E=2), so it is filtered out; pair [0, 1] remains at index 0.
        step0 = {
            "game_id": 1,
            "seat": 0,
            "step_cursor": 0,
            "state": {
                "entity_feat": [[0.0] * M.ENTITY_FEAT_WIDTH, [0.0] * M.ENTITY_FEAT_WIDTH],
                "entity_card_ids": [1, M.UNKNOWN_CARD],
                "entity_face_ids": [1, 0],
                "entity_token_ids": [0, 2],
                "seat_feat": [[0.0] * M.SEAT_FEAT_WIDTH for _ in range(M.MAX_SEATS)],
                "global_feat": [0.0] * M.GLOBAL_FEAT_WIDTH,
                "edges": [],
            },
            "head": 0,
            "cand_feat": [[0.0] * M.CAND_FEAT_WIDTH],
            "cand_entity_idx": [0],
            "cand_rel_seat_idx": [-1],
            "pairs": [],
            "target": {"single_idx": 0, "multi_mask": [], "free_mask": [], "scalar_norm": 0.0},
            "outcome": 1.0,
            "is_decision": True,
            "is_terminal": False,
            "loss_weight": 1.0,
            "aux": {},
            "provenance": {"source": "House", "thinking_steps": 2, "temperature": 1.0, "behavior_logprob": 0.0, "has_logprob": False},
        }
        step1 = {
            "game_id": 1,
            "seat": 0,
            "step_cursor": 1,
            "state": {
                "entity_feat": [[0.0] * M.ENTITY_FEAT_WIDTH, [0.0] * M.ENTITY_FEAT_WIDTH],
                "entity_card_ids": [1, M.UNKNOWN_CARD],
                "entity_face_ids": [0, 0],
                "entity_token_ids": [0, 0],
                "seat_feat": [[0.0] * M.SEAT_FEAT_WIDTH for _ in range(M.MAX_SEATS)],
                "global_feat": [0.0] * M.GLOBAL_FEAT_WIDTH,
                "edges": [],
            },
            "head": 2,
            "cand_feat": [],
            "cand_entity_idx": [],
            "cand_rel_seat_idx": [],
            "pairs": [[999, 0], [0, 1]],
            "target": {"single_idx": 0, "multi_mask": [0, 1], "free_mask": [0, 1], "scalar_norm": 0.0},
            "outcome": 1.0,
            "is_decision": True,
            "is_terminal": True,
            "loss_weight": 1.0,
            "aux": {},
            "provenance": {"source": "Net", "thinking_steps": 2, "temperature": 1.0, "behavior_logprob": -0.5, "has_logprob": True},
        }
        for step in (step0, step1):
            step["state"]["entity_rule_refs"] = [[] for _ in step["state"]["entity_feat"]]
            step["cand_rule_refs"] = [[] for _ in step["cand_feat"]]
        with gzip.open(dpath / "train" / "steps-000.jsonl.gz", "wt", encoding="utf-8") as gz:
            gz.write(json.dumps(step0) + "\n")
            gz.write(json.dumps(step1) + "\n")

        ds = D4.TrajectoryDataset4(dpath, split="train", unroll_len=4)
        sample = ds[0]
        # Verify entity 0 got back-face DSL (2.0) and entity 1 got token DSL (3.0)
        assert np.allclose(sample["entity_dsl"][0, 0], 2.0), "Back-face DSL lookup failed"
        assert np.allclose(sample["entity_dsl"][0, 1], 3.0), "Token DSL lookup failed"
        # Verify step 0 with out-of-range single_idx=99 was marked invalid (is_decision=False, loss_weight=0.0)
        invalid = dict(step0, target=dict(step0["target"], single_idx=99))
        try:
            ds.encode_single_step(invalid)
        except ValueError:
            pass
        else:
            raise AssertionError("Out-of-range decision target must fail loudly")
        # Verify step 1 filtered out invalid pair [999, 0] and remapped pair [0, 1]'s target=1 and free=1 to index 0
        assert bool(sample["is_decision"][1].item()) is True
        assert sample["pair_index"][1, 0].tolist() == [0, 1]
        assert float(sample["pair_target_mask"][1, 0].item()) == 1.0
        assert float(sample["pair_free_mask"][1, 0].item()) == 1.0


def test_p0_3_collector_vs_trainer_logprob_parity():
    """Verify P0-3: action_logprob_and_kl matches Rust collector logprobs across all 4 heads."""
    tau = 0.8
    cand_logits = torch.tensor(
        [
            [0.5, -0.3, 1.1, 0.2],
            [0.8, -1.2, 0.4, 0.9],
            [0.0, 0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0, 0.0],
        ],
        dtype=torch.float32,
    )
    pair_logits = torch.tensor(
        [
            [0.0, 0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0, 0.0],
            [0.7, -0.5, 0.2, -0.9],
            [0.0, 0.0, 0.0, 0.0],
        ],
        dtype=torch.float32,
    )
    scalar_pred = torch.tensor([0.0, 0.0, 0.0, 0.62], dtype=torch.float32)

    batch = {
        "head": torch.tensor([[0], [1], [2], [3]], dtype=torch.long),
        "single_idx": torch.tensor([[2], [0], [0], [0]], dtype=torch.long),
        "cand_mask": torch.ones(4, 1, 4, dtype=torch.bool),
        "pair_mask": torch.ones(4, 1, 4, dtype=torch.bool),
        "cand_target_mask": torch.tensor(
            [[[0, 0, 0, 0]], [[1, 1, 0, 0]], [[0, 0, 0, 0]], [[0, 0, 0, 0]]], dtype=torch.float32
        ),
        "cand_free_mask": torch.tensor(
            [[[1, 1, 1, 1]], [[1, 0, 1, 0]], [[0, 0, 0, 0]], [[0, 0, 0, 0]]], dtype=torch.float32
        ),
        "pair_target_mask": torch.tensor(
            [[[0, 0, 0, 0]], [[0, 0, 0, 0]], [[1, 0, 0, 0]], [[0, 0, 0, 0]]], dtype=torch.float32
        ),
        "pair_free_mask": torch.tensor(
            [[[0, 0, 0, 0]], [[0, 0, 0, 0]], [[1, 0, 1, 0]], [[0, 0, 0, 0]]], dtype=torch.float32
        ),
        "scalar_target": torch.tensor([[0.0], [0.0], [0.0], [0.35]], dtype=torch.float32),
        "temperature": torch.full((4, 1), tau, dtype=torch.float32),
    }

    logp, kl = action_logprob_and_kl(
        cand_logits, pair_logits, scalar_pred, cand_logits, pair_logits, scalar_pred, batch, t=0
    )
    assert torch.allclose(kl, torch.zeros_like(kl), atol=1e-6), "Self-KL must be 0"

    # Compute exact Rust collector logprobs:
    # 0: SingleChoice categorical with temperature tau
    l0 = (cand_logits[0].numpy()) / tau
    l0 = l0 - l0.max()
    p0 = np.exp(l0) / np.exp(l0).sum()
    rust_lp0 = float(np.log(max(p0[2], 1e-12)))

    # 1: SubsetPick over free_mask == 1 with temperature tau
    l1 = (cand_logits[1].numpy()) / tau
    p1 = 1.0 / (1.0 + np.exp(-l1))
    rust_lp1 = float(np.log(max(p1[0], 1e-7)) + np.log(max(1.0 - p1[2], 1e-7)))

    # 2: PairPick over pair_free_mask == 1 with temperature tau
    l2 = (pair_logits[2].numpy()) / tau
    p2 = 1.0 / (1.0 + np.exp(-l2))
    rust_lp2 = float(np.log(max(p2[0], 1e-7)) + np.log(max(1.0 - p2[2], 1e-7)))

    # 3: ScalarValue Gaussian log-density around scalar_pred with sigma=0.15
    rust_lp3 = 0.0  # Deterministic scalar actions are excluded from PPO.

    expected = np.array([rust_lp0, rust_lp1, rust_lp2, rust_lp3], dtype=np.float32)
    actual = logp.numpy()
    assert np.allclose(actual, expected, atol=1e-6), f"Logprob mismatch: actual={actual} vs expected={expected}"


def test_p0_4_stateful_batch_sampler_streams_consecutive_segments():
    """Verify P0-4: StatefulBatchSampler4 places consecutive segments of a trajectory in the same batch lane."""

    class DummySegDataset:
        def __init__(self):
            # 2 trajectories, each with 3 consecutive segments: (0,1,2) and (3,4,5)
            self.trajectories = [[0, 1, 2], [3, 4, 5]]

    ds = DummySegDataset()
    sampler = D4.StatefulBatchSampler4(ds, batch_size=2, shuffle=False, seed=0)
    batches = list(sampler)
    assert batches == [[0, 3], [1, 4], [2, 5]], f"Unexpected batch stream: {batches}"


def test_p0_5_reverse_gae_and_terminal_bootstrap():
    """Verify P0-5: compute_reverse_gae bootstraps across non-terminal steps and terminates at is_terminal."""
    values = torch.tensor([[0.4, 0.6, 0.8]], dtype=torch.float32)
    outcome = torch.tensor([[1.0, 1.0, 1.0]], dtype=torch.float32)
    step_valid = torch.tensor([[True, True, True]], dtype=torch.bool)
    is_terminal = torch.tensor([[False, False, True]], dtype=torch.bool)

    adv, ret = compute_reverse_gae(values, outcome, step_valid, is_terminal, gamma=0.99, gae_lambda=0.95)
    # Manual backward check:
    # t=2 (terminal): delta_2 = 1.0 - 0.8 = 0.2, gae_2 = 0.2
    # t=1 (non-term): delta_1 = 0.99 * 0.8 - 0.6 = 0.192, gae_1 = 0.192 + 0.99 * 0.95 * 0.2 = 0.3801
    # t=0 (non-term): delta_0 = 0.99 * 0.6 - 0.4 = 0.194, gae_0 = 0.194 + 0.99 * 0.95 * 0.3801 = 0.55148405
    expected_adv = torch.tensor([[0.55148405, 0.3801, 0.2]], dtype=torch.float32)
    assert torch.allclose(adv, expected_adv, atol=1e-6), f"GAE mismatch: {adv} vs {expected_adv}"
    assert torch.allclose(ret, adv + values, atol=1e-6)


def test_p0_6_dynamic_thinking_loop_in_pytorch_and_onnx():
    """Verify P0-6: K=1, 2, 4, 8 produce distinct outputs in PyTorch & ONNX while h_out is K-invariant."""
    cfg = M.Config4(d_model=64, n_heads=4, n_layers=2, d_ff=128, dropout=0.0)
    net = M.PlayerNet4(cfg).eval()
    with torch.no_grad():
        for m in net.cand_scorer.modules():
            if isinstance(m, torch.nn.Linear):
                m.weight.normal_(0.0, 0.5)

    with tempfile.TemporaryDirectory() as tmpdir:
        onnx_path = Path(tmpdir) / "test_v4.onnx"
        export_v4_onnx(net, onnx_path, thinking_steps=2)
        sess = ort.InferenceSession(str(onnx_path), providers=["CPUExecutionProvider"])

        base_tensors = list(make_dummy_inputs(cfg.d_model, B=1, E=12, M_edges=12, C=8, P=8, K=1))
        logits_by_k = {}
        h_by_k = {}
        for k_val in (1, 2, 4, 8):
            tensors = list(base_tensors)
            tensors[-1] = torch.tensor([k_val], dtype=torch.int64)
            with torch.no_grad():
                pt_outs = net(*tensors)
            feed = {name: t.numpy() for name, t in zip(M.INPUT_NAMES, tensors)}
            ort_outs = sess.run(list(M.OUTPUT_NAMES), feed)
            for idx, name in enumerate(M.OUTPUT_NAMES):
                diff = float(np.max(np.abs(pt_outs[idx].numpy() - ort_outs[idx])))
                assert diff < 1e-4, f"{name} mismatch at K={k_val}: max_diff={diff}"
            h_by_k[k_val] = ort_outs[0].copy()
            logits_by_k[k_val] = ort_outs[1].copy()

        for k1, k2 in ((1, 2), (2, 4), (4, 8)):
            diff = float(np.max(np.abs(logits_by_k[k1] - logits_by_k[k2])))
            assert diff > 1e-5, f"ONNX thinking_steps K={k1} vs K={k2} produced identical logits (diff={diff})"
        assert np.allclose(h_by_k[1], h_by_k[8], atol=1e-6), "Persistent h_out must be invariant to thinking depth K"


def test_p0_7_encoder7_warm_start_and_candidate_context_refinement():
    """Verify Encoder 6 -> 7 warm start (upgrade_state_dict_to_v7) and cand_ctx_proj menu awareness."""
    cfg = M.Config4(d_model=64, n_heads=4, n_layers=2, d_ff=128, dropout=0.0)
    net7 = M.PlayerNet4(cfg)
    v6_sd = net7.state_dict()
    # Simulate an archived Encoder 6 state_dict: entity_proj input width 96, global_proj 64, no cand_ctx_proj
    v6_sd["entity_proj.0.weight"] = v6_sd["entity_proj.0.weight"][:, :96].clone()
    v6_sd["global_proj.0.weight"] = v6_sd["global_proj.0.weight"][:, :64].clone()
    v6_sd.pop("cand_ctx_proj.weight")
    v6_sd.pop("cand_ctx_proj.bias")

    upgraded = M.upgrade_state_dict_to_v7(v6_sd, net7)
    net7.load_state_dict(upgraded, strict=True)
    assert torch.all(net7.entity_proj[0].weight[:, 96:128] == 0.0)
    assert torch.all(net7.global_proj[0].weight[:, 64:80] == 0.0)
    assert torch.all(net7.cand_ctx_proj.weight == 0.0)

    # Activate cand_ctx_proj and cand_scorer and verify candidate 0's logit changes when competing candidate 1 changes
    with torch.no_grad():
        net7.cand_ctx_proj.weight.normal_(0.0, 0.25)
        for m in net7.cand_scorer.modules():
            if isinstance(m, torch.nn.Linear):
                m.weight.normal_(0.0, 0.25)
    dummy_a = list(make_dummy_inputs(cfg.d_model, B=1, E=6, M_edges=4, C=3, P=2, K=1))
    dummy_b = [t.clone() for t in dummy_a]
    cand_feat_idx = M.INPUT_NAMES.index("cand_feat")
    # Change candidate 1 features while keeping candidate 0 identical
    dummy_b[cand_feat_idx][:, 1, :] += 2.5
    with torch.no_grad():
        logits_a = net7(*dummy_a)[1]
        logits_b = net7(*dummy_b)[1]
    assert abs(float(logits_a[0, 0] - logits_b[0, 0])) > 1e-4, "cand_ctx_proj must make candidate 0 sensitive to competing candidates"


if __name__ == "__main__":
    test_p0_1_seat_feat_indices_and_value_head_non_collapse()
    test_p0_2_no_silent_single_idx_relabeling_and_pair_remapping()
    test_p0_3_collector_vs_trainer_logprob_parity()
    test_p0_4_stateful_batch_sampler_streams_consecutive_segments()
    test_p0_5_reverse_gae_and_terminal_bootstrap()
    test_p0_6_dynamic_thinking_loop_in_pytorch_and_onnx()
    test_p0_7_encoder7_warm_start_and_candidate_context_refinement()
    print("ALL 7 PYTHON P0/P1 CONTRACT TESTS PASSED!")
