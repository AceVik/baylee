"""KL-gated action-head PPO; a failed final gate exports the unchanged source."""
import argparse
from contextlib import ExitStack
import json
from pathlib import Path
import numpy as np
import torch
import dataset4 as D
from value_calibration4.checkpoint import file_digest, load_source, publish
from value_calibration4.model_identity import source_identity
from .forward import actor_outputs, batch_rows
from .admission import select_learner_histories, check_audit
from .optimization import (POLICY_PREFIXES, accept_final_state, behavior_kl, plan_groups, select_policy_parameters,
                           surrogate_sum, transactional_step)


def main():
    with ExitStack() as resources:
        _main(resources)


def _main(resources):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--model", "--init", dest="model", required=True)
    parser.add_argument("--rollout-data", required=True)
    parser.add_argument("--admission", required=True, help="Rust-certified learner history admission")
    parser.add_argument("--out", required=True)
    parser.add_argument("--epochs", type=int, default=2)
    parser.add_argument("--max-steps", type=int, default=256)
    parser.add_argument("--lr", type=float, default=2e-5)
    parser.add_argument("--target-kl", type=float, default=.02)
    parser.add_argument("--clip-eps", type=float, default=.2)
    parser.add_argument("--min-policy-decisions", type=int, default=128)
    parser.add_argument("--batch-size", type=int, default=8)
    parser.add_argument("--unroll", type=int, default=16)
    parser.add_argument("--board-chunk", type=int, default=4)
    parser.add_argument("--seed", type=int, default=31431)
    parser.add_argument("--device", choices=("cpu", "cuda"), default="cpu")
    args = parser.parse_args()
    if min(args.epochs, args.max_steps, args.batch_size, args.unroll) < 1 or args.min_policy_decisions < 128 or args.lr <= 0 or args.target_kl <= 0 or not 0 < args.clip_eps < 1:
        parser.error("Invalid PPO configuration")
    output = Path(args.out).expanduser().resolve()
    if output.exists() or output.is_symlink():
        raise FileExistsError("Preserving existing output; use a new directory")
    torch.set_num_threads(4)
    torch.manual_seed(args.seed)
    source_path, saved, net = load_source(args.model)
    source_hash = file_digest(source_path)
    device = torch.device(args.device)
    net.to(device)
    from recurrent_ppo4.checkpoint import native_verify_admission
    native_verify_admission(Path(args.rollout_data).expanduser(), source_path, Path(args.admission).expanduser())
    dataset = D.TrajectoryDataset4(Path(args.rollout_data).expanduser(), split="train", unroll_len=args.unroll,
                                   encoder_version=saved['encoder_version'], max_shards=0)
    resources.callback(dataset.store.close)
    if dataset.structure.digest != net.rules.digest:
        raise ValueError("Rollout and model use different public rule catalogs")
    admission = select_learner_histories(dataset, args.admission, source_hash)
    from .targets import prepare_policy_targets
    audit = prepare_policy_targets(net, dataset, device, batch_size=args.batch_size, board_chunk=args.board_chunk)
    resources.callback(dataset._policy_targets_directory.cleanup)
    check_audit(admission, audit)
    parameters = select_policy_parameters(net)
    optimizer = torch.optim.AdamW(parameters, lr=args.lr, weight_decay=0)
    records, scheduling = [], []
    accepted, accepted_decisions, reason = 0, 0, "epochs_complete"
    def outputs(indices):
        return actor_outputs(net, batch_rows(dataset, indices, device), args.board_chunk)
    def loss_sum(indices):
        lp, old, adv = outputs(indices)
        return surrogate_sum(lp, old, adv, args.clip_eps)
    def kl_sum(indices):
        lp, old, _ = outputs(indices)
        return behavior_kl(lp, old)
    for epoch in range(args.epochs):
        groups, stats = plan_groups(dataset.ppo_cache["actor_mask"], args.seed + epoch,
                                    args.min_policy_decisions, args.batch_size)
        scheduling.append(dict(epoch=epoch+1, **stats))
        for microbatches, count in groups:
            record = transactional_step(parameters, optimizer, microbatches, count, loss_sum, kl_sum, args.target_kl)
            records.append(dict(epoch=epoch+1, **record))
            print("[policy-step] " + json.dumps(records[-1]), flush=True)
            if not record["accepted"]:
                reason = "post_step_rejected"
                break
            accepted += 1
            accepted_decisions += count
            if accepted >= args.max_steps:
                reason = "max_steps"
                break
        if reason != "epochs_complete":
            break
    if not records:
        reason = "insufficient_policy_decisions; discarded_undersized_groups"
    # A group-local check cannot certify drift on other states: audit all actor rows.
    all_segments = np.flatnonzero(np.asarray(dataset.ppo_cache["actor_mask"]).any(-1))
    total_decisions = int(np.asarray(dataset.ppo_cache["actor_mask"]).sum())
    try:
        with torch.no_grad():
            final_kl = sum(float(kl_sum(tuple(map(int, all_segments[start:start+args.batch_size]))))
                           for start in range(0, len(all_segments), args.batch_size)) / max(total_decisions, 1)
    except FloatingPointError:
        final_kl = float("inf")
    attempted_accepted = accepted
    final_accepted = accept_final_state(net, saved["state_dict"], final_kl, args.target_kl)
    if not final_accepted:
        accepted, accepted_decisions = 0, 0
        reason = "rollout_wide_kl_rejected; published_unchanged_source"
    state = {key: value.detach().cpu().clone() for key, value in net.state_dict().items()}
    for key, value in saved["state_dict"].items():
        if not key.startswith(POLICY_PREFIXES) and not torch.equal(value.cpu(), state[key]):
            raise AssertionError(f"Frozen tensor changed: {key}")
    if file_digest(source_path) != source_hash:
        raise RuntimeError("Source checkpoint changed during training")
    native_verify_admission(Path(args.rollout_data).expanduser(), source_path, Path(args.admission).expanduser())
    provenance = dict(kind="isolated_action_head_ppo", source=str(source_path.resolve()),
                      source_sha256=source_hash, dataset_fingerprint=dataset.fingerprint,
                      admission=admission,
                      seed=args.seed, gamma=1, gae_lambda=1, promoted=False,
                      trainable_prefixes=list(POLICY_PREFIXES), accepted_steps=accepted,
                      accepted_decisions=accepted_decisions, reason=reason,
                      critic_and_memory="all shared/value tensors verified bit-identical",
                      optimizer_and_memory="new experiment; no stale optimizer or H exported")
    payload = {**source_identity(saved), "structure": saved["structure"],
               "state_dict": state, "provenance": provenance}
    report = dict(provenance=provenance, settings=vars(args), audit=audit, updates=records,
                  scheduling=scheduling, attempted_accepted_steps=attempted_accepted,
                  rollout_wide_behavior_kl=final_kl if np.isfinite(final_kl) else None,
                  checkpoint_behavior_kl=final_kl if final_accepted else None,
                  checkpoint_is_unchanged_source=accepted == 0,
                  fallback_source_audited_max_logprob_error=audit["max_logprob_error"])
    publish(output, payload, report)
    print(json.dumps(dict(output=str(output), **provenance), indent=2))
